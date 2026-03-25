//! Phase 2: Body translation — syn::Expr/Stmt → TS expression/statement text
//!
//! Translates Rust function bodies to TypeScript. Each syn expression variant
//! maps to a TS expression string. The output is deterministic and structural,
//! prioritizing 1:1 correspondence with the Rust source over elegance.

use syn;
use crate::name_map;
use crate::macros;
use crate::match_expr;
use crate::control_flow;
use crate::ownership;
use crate::native_types;

/// Check if an expression is a write!/writeln! macro call
fn is_write_macro(expr: &syn::Expr) -> bool {
    if let syn::Expr::Macro(mac) = expr {
        let name = mac.mac.path.segments.last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        matches!(name.as_str(), "write" | "writeln")
    } else {
        false
    }
}

/// Extract the Macro from an expression (for write! detection)
fn extract_macro(expr: &syn::Expr) -> Option<&syn::Macro> {
    if let syn::Expr::Macro(mac) = expr {
        Some(&mac.mac)
    } else {
        None
    }
}

/// Check if a match expression has arms that are write! macro calls (Display pattern)
fn is_match_with_write_arms(expr: &syn::Expr) -> bool {
    if let syn::Expr::Match(m) = expr {
        m.arms.iter().any(|arm| {
            matches!(&*arm.body, syn::Expr::Try(t) if is_write_macro(&t.expr))
        })
    } else {
        false
    }
}

/// Extract a single expression from a block (for ternary conversion)
fn single_block_expr(block: &syn::Block) -> Option<&syn::Expr> {
    if block.stmts.len() == 1 {
        if let syn::Stmt::Expr(expr, _) = &block.stmts[0] {
            return Some(expr);
        }
    }
    None
}

// ── Public entry points ─────────────────────────────────────────────────

/// Translate a block of statements to TS (default Self type)
pub fn translate_block(block: &syn::Block) -> String {
    BodyTranslator::new("Self").translate_block(block)
}

/// Translate a block with a known self type name for Self resolution
pub fn translate_block_with_self(block: &syn::Block, self_type: &str) -> String {
    BodyTranslator::new(self_type).translate_block(block)
}

/// Translate a single expression (used by match_expr, control_flow, macros modules)
pub fn translate_expr(expr: &syn::Expr) -> String {
    BodyTranslator::new("Self").expr(expr)
}

/// Translate a pattern (used by match_expr, control_flow modules)
pub fn translate_pat(pat: &syn::Pat) -> String {
    BodyTranslator::pat_static(pat)
}

/// Indent each line by 2 spaces
pub fn indent(s: &str) -> String {
    s.lines()
        .map(|line| if line.is_empty() { String::new() } else { format!("  {}", line) })
        .collect::<Vec<_>>()
        .join("\n")
        + if s.ends_with('\n') { "\n" } else { "" }
}

// ── Translator struct ───────────────────────────────────────────────────

pub struct BodyTranslator<'a> {
    pub self_type: &'a str,
    /// Type context for comprehensive expression type resolution.
    /// Wraps ScopeStack + TypeRegistry. None for legacy codepaths
    /// (free function shims, match_expr, control_flow).
    pub types: Option<std::cell::RefCell<crate::type_context::TypeContext<'a>>>,
    /// Inline module names for the current file — path qualifiers
    /// stripped during path resolution (symbols imported from separate .ts file).
    pub inline_module_names: Vec<String>,
}

impl<'a> BodyTranslator<'a> {
    pub fn new(self_type: &'a str) -> Self {
        Self { self_type, types: None, inline_module_names: vec![] }
    }

    pub fn with_registry(self_type: &'a str, registry: &'a crate::resolve::TypeRegistry, module: &str) -> Self {
        let tc = crate::type_context::TypeContext::new(registry, module, self_type);
        Self {
            self_type,
            types: Some(std::cell::RefCell::new(tc)),
            inline_module_names: vec![],
        }
    }

    /// Resolve the type of an expression using the TypeContext.
    fn resolve_expr_type(&self, expr: &syn::Expr) -> Option<crate::resolve::ResolvedType> {
        self.types.as_ref()?.borrow().resolve_expr(expr)
    }

    /// Check if a name is bound in any active scope.
    fn is_in_scope(&self, name: &str) -> bool {
        self.types.as_ref()
            .map(|tc| tc.borrow().resolve_var(name).is_some())
            .unwrap_or(false)
    }

    /// Bind a variable in the current TypeContext scope.
    pub fn bind_var(&self, name: &str, ty: crate::resolve::ResolvedType) {
        if let Some(tc) = &self.types {
            tc.borrow_mut().bind(name, ty);
        }
    }

    /// Push a block scope in the TypeContext.
    pub fn push_block(&self) {
        if let Some(tc) = &self.types {
            tc.borrow_mut().push_block();
        }
    }

    /// Pop scope in the TypeContext.
    pub fn pop_scope(&self) {
        if let Some(tc) = &self.types {
            tc.borrow_mut().pop();
        }
    }

    /// Push a function scope with typed parameters.
    pub fn push_fn_scope(&self, params: Vec<(String, crate::resolve::ResolvedType)>) {
        if let Some(tc) = &self.types {
            tc.borrow_mut().push_fn(params);
        }
    }

    /// Push a closure scope with typed parameters.
    pub fn push_closure_scope(&self, params: Vec<(String, crate::resolve::ResolvedType)>) {
        if let Some(tc) = &self.types {
            tc.borrow_mut().push_closure(params);
        }
    }

    // ── Block translation with ownership tracking ───────────────────

    pub fn translate_block(&self, block: &syn::Block) -> String {
        let mut out = String::new();
        let stmts = &block.stmts;

        // Collect local bindings for drop insertion (with type inference from init)
        let mut locals: Vec<(String, String)> = Vec::new();
        for stmt in stmts {
            if let syn::Stmt::Local(local) = stmt {
                let init_expr = local.init.as_ref().map(|i| &*i.expr);
                ownership::collect_local_bindings(&local.pat, init_expr, &mut locals);
            }
        }

        // Only track variables that are the implicit return value —
        // those should NOT be dropped (they're moved to the caller).
        // Everything else drops at end of scope (idempotent for moved values).
        let mut returned_vars = std::collections::HashSet::new();
        if let Some(syn::Stmt::Expr(expr, None)) = stmts.last() {
            ownership::collect_direct_vars(expr, &mut returned_vars);
        }

        let drops = ownership::generate_drops(&locals, &returned_vars);

        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == stmts.len() - 1;
            if is_last {
                if let syn::Stmt::Expr(expr, None) = stmt {
                    // Implicit return — drops go before return
                        out.push_str(&control_flow::translate_expr_in_return_position_with(expr, self, &drops));
                    out.push('\n');
                } else {
                    out.push_str(&self.stmt(stmt));
                    // Drops after last statement
                    if !drops.is_empty() {
                        out.push_str(&drops);
                    }
                }
            } else {
                out.push_str(&self.stmt(stmt));
            }
        }
        out
    }

    // ── Statement translation ───────────────────────────────────────

    fn stmt(&self, stmt: &syn::Stmt) -> String {
        match stmt {
            syn::Stmt::Local(local) => self.local(local),
            syn::Stmt::Expr(expr, semi) => {
                // Detect standalone `expr?;` — emit Result check
                if semi.is_some() {
                    if let syn::Expr::Try(try_expr) = expr {
                        // Special case: write!(f, ...)?; in Display impls — emit string append
                        if is_write_macro(&try_expr.expr) {
                            let fmt_str = macros::translate_macro(extract_macro(&try_expr.expr).unwrap());
                            return format!("_result += {};\n", fmt_str);
                        }
                        let inner = self.expr(&try_expr.expr);
                        return format!("const _r = {};\nif (_r.isErr()) return _r as any;\n", inner);
                    }
                }
                let ts = self.expr(expr);
                // If a match expression contains write! arms (Display pattern),
                // append the result to _result
                let ts = if is_match_with_write_arms(expr) {
                    format!("_result += {}", ts)
                } else {
                    ts
                };
                if semi.is_some() {
                    format!("{};\n", ts)
                } else {
                    format!("{}\n", ts)
                }
            }
            syn::Stmt::Item(_) => String::new(),
            syn::Stmt::Macro(macro_stmt) => {
                let ts = macros::translate_macro(&macro_stmt.mac);
                if macro_stmt.semi_token.is_some() {
                    format!("{};\n", ts)
                } else {
                    format!("{}\n", ts)
                }
            }
        }
    }

    fn local(&self, local: &syn::Local) -> String {
        let pat = Self::pat_static(&local.pat);

        if let Some(init) = &local.init {
            // Detect `let x = expr?` pattern — emit Result check + early return
            if let syn::Expr::Try(try_expr) = &*init.expr {
                let inner = self.expr(&try_expr.expr);
                let keyword = if is_mut_binding(&local.pat) { "let" } else { "const" };
                return format!(
                    "const _r_{} = {};\nif (_r_{}.isErr()) return _r_{} as any;\n{} {} = _r_{}.unwrap();\n",
                    pat, inner, pat, pat, keyword, pat, pat
                );
            }

            // Check for shadowing BEFORE binding the variable type.
            // Rust allows `let x = x.method()` to shadow — JS doesn't allow
            // redeclaring a closure/function parameter. Use assignment instead.
            let already_in_scope = self.is_in_scope(&pat);

            // Register the local variable's type for downstream resolution.
            if let Some(tc) = &self.types {
                let resolved = tc.borrow().resolve_local_type(local);
                if let Some(ty) = resolved {
                    tc.borrow_mut().bind(&pat, ty);
                }
            }

            let expr = self.expr(&init.expr);

            if let Some((_tok, _diverge)) = &init.diverge {
                return format!("/* let-else */ const {} = {};\n", pat, expr);
            }

            if already_in_scope {
                // Check if the init expression references the same name as a
                // standalone variable (not as a field name in a.b.c).
                // If so, IIFE param threading already provided the value — skip.
                let rust_name = if let syn::Pat::Ident(ident) = &local.pat {
                    ident.ident.to_string()
                } else {
                    pat.clone()
                };
                if references_var(&init.expr, &rust_name) {
                    return String::new();
                }
                return format!("{} = {};\n", pat, expr);
            }
            let keyword = if is_mut_binding(&local.pat) { "let" } else { "const" };
            format!("{} {} = {};\n", keyword, pat, expr)
        } else {
            format!("let {};\n", pat)
        }
    }

    // ── Pattern translation (static — no self_type needed) ──────────

    pub fn pat_static(pat: &syn::Pat) -> String {
        match pat {
            syn::Pat::Ident(ident) => name_map::to_camel_case(&ident.ident.to_string()),
            syn::Pat::Tuple(tuple) => {
                let parts: Vec<String> = tuple.elems.iter().map(Self::pat_static).collect();
                format!("[{}]", parts.join(", "))
            }
            syn::Pat::TupleStruct(ts) => {
                let parts: Vec<String> = ts.elems.iter().map(Self::pat_static).collect();
                parts.join(", ")
            }
            syn::Pat::Struct(s) => {
                let fields: Vec<String> = s.fields.iter().map(|f| {
                    let member = match &f.member {
                        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    let pat = Self::pat_static(&f.pat);
                    if member == pat { member } else { format!("{}: {}", member, pat) }
                }).collect();
                format!("{{ {} }}", fields.join(", "))
            }
            syn::Pat::Wild(_) => "_".to_string(),
            syn::Pat::Lit(_) => "/* pat literal */".to_string(),
            syn::Pat::Path(path) => Self::path_static(&path.path),
            syn::Pat::Reference(r) => Self::pat_static(&r.pat),
            syn::Pat::Type(t) => Self::pat_static(&t.pat),
            syn::Pat::Or(or_pat) => {
                let parts: Vec<String> = or_pat.cases.iter().map(Self::pat_static).collect();
                parts.join(" | ")
            }
            syn::Pat::Slice(slice) => {
                let parts: Vec<String> = slice.elems.iter().map(Self::pat_static).collect();
                format!("[{}]", parts.join(", "))
            }
            syn::Pat::Rest(_) => "...".to_string(),
            _ => "/* unknown pat */".to_string(),
        }
    }

    // ── Expression translation ──────────────────────────────────────

    pub fn expr(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Lit(lit) => translate_lit(&lit.lit),
            syn::Expr::Path(path) => Self::path_static(&path.path),

            syn::Expr::Field(field) => {
                let base = self.expr(&field.base);
                let member = match &field.member {
                    syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                    syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                };
                let base_str = if base == "self" { "this".to_string() } else { base };

                // Type-aware deref insertion for field access
                if let Some(tc) = &self.types {
                    if let Some((accessor, _field_ty)) = tc.borrow().field_deref(&field.base, &member) {
                        return format!("{}.{}.{}", base_str, accessor, member);
                    }
                }

                format!("{}.{}", base_str, member)
            }

            syn::Expr::MethodCall(call) => {
                let receiver = self.expr(&call.receiver);
                let rust_method = call.method.to_string();
                let ts_method = name_map::map_fn_name(&rust_method);

                // ── closure param type resolution ──
                // If a closure arg's param types can be resolved from the call context,
                // register them before translating arguments.
                let mut pushed_closure_scope = false;
                if let Some(tc) = &self.types {
                    if let Some(syn::Expr::Closure(closure)) = call.args.first() {
                        let params = tc.borrow().resolve_closure_param_types(
                            &call.receiver, &rust_method, closure);
                        if !params.is_empty() {
                            self.push_closure_scope(params);
                            pushed_closure_scope = true;
                        }
                    }
                }

                let args: Vec<String> = call.args.iter().map(|a| self.expr(a)).collect();

                if pushed_closure_scope {
                    self.pop_scope();
                }

                // ── unwrap/expect: single decision point ──
                // In TS, only Result has a real .unwrap(). All other types
                // (guards, Option/nullable, etc.) treat it as identity.
                if matches!(rust_method.as_str(), "unwrap" | "expect") {
                    let is_result = self.resolve_expr_type(&call.receiver)
                        .map(|ty| matches!(&ty, crate::resolve::ResolvedType::Named { name, .. } if name == "Result"))
                        .unwrap_or(false);
                    if !is_result {
                        return receiver.to_string();
                    }
                }
                if rust_method == "unwrap_or" && args.len() == 1 {
                    return format!("{} ?? {}", receiver, args[0]);
                }
                if rust_method == "unwrap_or_else" && args.len() == 1 {
                    return format!("{} ?? ({})()", receiver, args[0]);
                }

                // ── deref insertion via TypeContext ──
                if let Some(tc) = &self.types {
                    let tc_ref = tc.borrow();
                    if let Some(accessor) = tc_ref.method_deref(&call.receiver, &rust_method) {
                        let deref_receiver = format!("{}.{}", receiver, accessor);
                        // Dispatch against inner type (after deref)
                        if let Some(receiver_ty) = tc_ref.resolve_expr(&call.receiver) {
                            if let Some(inner) = tc_ref.deref_inner_type(&receiver_ty) {
                                match native_types::translate_method(&inner, &deref_receiver, &rust_method, &args) {
                                    native_types::MethodTranslation::Expr(result) => return result,
                                    native_types::MethodTranslation::Passthrough => {}
                                }
                            }
                        }
                        drop(tc_ref);
                        return self.translate_method_call(&deref_receiver, &rust_method, &ts_method, &args, Some(&call.receiver));
                    }
                }

                self.translate_method_call(&receiver, &rust_method, &ts_method, &args, Some(&call.receiver))
            }

            syn::Expr::Call(call) => {
                let func = self.expr(&call.func);
                let args: Vec<String> = call.args.iter().map(|a| self.expr(a)).collect();
                self.translate_call(&func, &args)
            }

            syn::Expr::Binary(bin) => {
                // Handle deref compound assignment: *guard += value → guard.value += value
                if is_assign_op(&bin.op) {
                    if let syn::Expr::Unary(unary) = &*bin.left {
                        if matches!(unary.op, syn::UnOp::Deref(_)) {
                            let inner = self.expr(&unary.expr);
                            let op = translate_binop(&bin.op);
                            let right = self.expr(&bin.right);
                            // Deref compound-assign via TypeContext
                            if let Some(tc) = &self.types {
                                if let Some(inner_ty) = tc.borrow().resolve_expr(&unary.expr) {
                                    if let Some(accessor) = tc.borrow().deref_accessor(&inner_ty) {
                                        return format!("{}.{} {} {}", inner, accessor, op, right);
                                    }
                                }
                            }
                            // Syntactic *x always means deref-assign; .value is the semantic default
                            return format!("{}.value {} {}", inner, op, right);
                        }
                    }
                }
                format!("{} {} {}", self.expr(&bin.left), translate_binop(&bin.op), self.expr(&bin.right))
            }

            syn::Expr::Unary(unary) => {
                let e = self.expr(&unary.expr);
                match &unary.op {
                    syn::UnOp::Not(_) => {
                        if e.contains("===") || e.contains("!==") || e.contains(">=") || e.contains("<=") {
                            format!("!({})", e)
                        } else {
                            format!("!{}", e)
                        }
                    }
                    syn::UnOp::Neg(_) => format!("-{}", e),
                    syn::UnOp::Deref(_) => e,
                    _ => format!("/* unknown unary op */ {}", e),
                }
            }

            syn::Expr::If(if_expr) => {
                // Try ternary for simple if/else value expressions
                if let Some(ternary) = self.try_ternary(if_expr) {
                    ternary
                } else {
                    control_flow::translate_if(if_expr)
                }
            }

            syn::Expr::Block(block) => {
                if block.block.stmts.len() == 1 {
                    if let syn::Stmt::Expr(expr, None) = &block.block.stmts[0] {
                        return self.expr(expr);
                    }
                }
                // Multi-statement block as expression → IIFE
                // Detect shadowed variables: if a local in the block has the same name
                // as a variable used in its init, thread it as an IIFE parameter
                let mut shadow_params: Vec<(String, String)> = Vec::new();
                for stmt in &block.block.stmts {
                    if let syn::Stmt::Local(local) = stmt {
                        let pat_name = Self::pat_static(&local.pat);
                        if let Some(init) = &local.init {
                            // Check if the init expression references pat_name as a
                            // standalone variable (not as a field name in a.b.c)
                            if references_var(&init.expr, &pat_name) {
                                // This is a shadow pattern — pass as IIFE param
                                let init_ts = self.expr(&init.expr);
                                shadow_params.push((pat_name, init_ts));
                            }
                        }
                    }
                }
                if !shadow_params.is_empty() {
                    // Thread shadowed variables as IIFE parameters.
                    // Push shadow names into scope so local() skips their declarations
                    // (they're already bound as IIFE params).
                    self.push_block();
                    for (name, _) in &shadow_params {
                        // Bind with Unknown type — the IIFE param is the value
                        self.bind_var(name, crate::resolve::ResolvedType::Unknown);
                    }
                    let body = self.translate_block(&block.block);
                    self.pop_scope();
                    let params: Vec<&str> = shadow_params.iter().map(|(n, _)| n.as_str()).collect();
                    let args: Vec<&str> = shadow_params.iter().map(|(_, v)| v.as_str()).collect();
                    format!("(({}) => {{\n{}}})({})", params.join(", "), indent(&body), args.join(", "))
                } else {
                    let body = self.translate_block(&block.block);
                    format!("(() => {{\n{}}})()", indent(&body))
                }
            }

            syn::Expr::Return(ret) => {
                if let Some(expr) = &ret.expr {
                    format!("return {}", self.expr(expr))
                } else {
                    "return".to_string()
                }
            }

            syn::Expr::Match(me) => match_expr::translate_match(me),

            syn::Expr::Closure(closure) => {
                let params: Vec<String> = closure.inputs.iter().map(Self::pat_static).collect();
                // Push closure scope — param types may already be registered
                // by the calling method's closure param resolution
                self.push_block();
                // Check if the body is a block — if so, translate as block with braces
                let result = match &*closure.body {
                    syn::Expr::Block(block) => {
                        let body = self.translate_block(&block.block);
                        format!("({}) => {{\n{}}}", params.join(", "), indent(&body))
                    }
                    _ => {
                        let body = self.expr(&closure.body);
                        // If body starts with { or if/for/while, wrap in braces
                        // (arrow function expression body can't start with these)
                        if body.starts_with("if ") || body.starts_with("for ") || body.starts_with("while ") || body.starts_with('{') {
                            format!("({}) => {{\n  {}\n}}", params.join(", "), body)
                        } else {
                            format!("({}) => {}", params.join(", "), body)
                        }
                    }
                };
                self.pop_scope();
                result
            }

            syn::Expr::ForLoop(for_loop) => {
                let pat = Self::pat_static(&for_loop.pat);
                let iter = self.expr(&for_loop.expr);
                let body = self.translate_block(&for_loop.body);
                format!("for (const {} of {}) {{\n{}}}", pat, iter, indent(&body))
            }

            syn::Expr::While(while_loop) => {
                let cond = self.expr(&while_loop.cond);
                let body = self.translate_block(&while_loop.body);
                format!("while ({}) {{\n{}}}", cond, indent(&body))
            }

            syn::Expr::Loop(loop_expr) => {
                let body = self.translate_block(&loop_expr.body);
                format!("while (true) {{\n{}}}", indent(&body))
            }

            syn::Expr::Break(brk) => {
                if let Some(expr) = &brk.expr {
                    format!("break /* {} */", self.expr(expr))
                } else { "break".to_string() }
            }

            syn::Expr::Continue(_) => "continue".to_string(),

            syn::Expr::Assign(assign) => {
                // Check for deref-assign: *guard = value → guard.value = value
                if let syn::Expr::Unary(unary) = &*assign.left {
                    if matches!(unary.op, syn::UnOp::Deref(_)) {
                        let inner = self.expr(&unary.expr);
                        if let Some(tc) = &self.types {
                            if let Some(inner_ty) = tc.borrow().resolve_expr(&unary.expr) {
                                if let Some(accessor) = tc.borrow().deref_accessor(&inner_ty) {
                                    return format!("{}.{} = {}", inner, accessor, self.expr(&assign.right));
                                }
                            }
                        }
                        // Deref in Rust (*x = y) always means "assign through wrapper."
                        // All current wrapper types use .value — use it as the semantic default.
                        return format!("{}.value = {}", inner, self.expr(&assign.right));
                    }
                }
                format!("{} = {}", self.expr(&assign.left), self.expr(&assign.right))
            }

            syn::Expr::Index(idx) => {
                format!("{}[{}]", self.expr(&idx.expr), self.expr(&idx.index))
            }

            syn::Expr::Reference(reference) => self.expr(&reference.expr),

            syn::Expr::Paren(paren) => format!("({})", self.expr(&paren.expr)),

            syn::Expr::Tuple(tuple) => {
                let parts: Vec<String> = tuple.elems.iter().map(|e| self.expr(e)).collect();
                format!("[{}]", parts.join(", "))
            }

            syn::Expr::Array(arr) => {
                let items: Vec<String> = arr.elems.iter().map(|e| self.expr(e)).collect();
                format!("[{}]", items.join(", "))
            }

            syn::Expr::Struct(s) => {
                let mut name = Self::path_static(&s.path);
                if name == "Self" { name = self.self_type.to_string(); }
                let values: Vec<String> = s.fields.iter().map(|f| {
                    self.expr(&f.expr)
                }).collect();
                format!("new {}({})", name, values.join(", "))
            }

            syn::Expr::Try(try_expr) => {
                // Special case: write!(f, ...)? in expression position — just the format string
                if is_write_macro(&try_expr.expr) {
                    let fmt_str = macros::translate_macro(extract_macro(&try_expr.expr).unwrap());
                    return fmt_str;
                }
                // expr? in expression position — use .unwrap() (caller handles Result propagation)
                // For statement-level ?, see translate_local which emits the full check pattern.
                let inner = self.expr(&try_expr.expr);
                format!("{}.unwrap()", inner)
            }
            syn::Expr::Await(await_expr) => format!("await {}", self.expr(&await_expr.base)),

            syn::Expr::Range(range) => {
                let from = range.start.as_ref().map(|e| self.expr(e)).unwrap_or_default();
                let to = range.end.as_ref().map(|e| self.expr(e)).unwrap_or_default();
                format!("/* range {}..{} */", from, to)
            }

            syn::Expr::Cast(cast) => {
                format!("{} as {}", self.expr(&cast.expr), name_map::map_type(&cast.ty))
            }

            syn::Expr::Macro(mac) => macros::translate_macro(&mac.mac),

            syn::Expr::Unsafe(unsafe_block) => {
                let body = self.translate_block(&unsafe_block.block).trim().to_string();
                format!("/* unsafe — consider provided impl */ {}", body)
            }

            syn::Expr::Async(async_block) => {
                let body = self.translate_block(&async_block.block);
                format!("(async () => {{\n{}}})()", indent(&body))
            }

            syn::Expr::Let(let_expr) => {
                let pat = Self::pat_static(&let_expr.pat);
                let expr = self.expr(&let_expr.expr);
                format!("/* let {} = {} */", pat, expr)
            }

            syn::Expr::Repeat(repeat) => {
                format!("Array({}).fill({})", self.expr(&repeat.len), self.expr(&repeat.expr))
            }

            _ => "/* TODO: unhandled expr */".to_string(),
        }
    }

    /// Try to translate an if/else as a ternary expression.
    /// Returns Some(ternary) if both branches are single expressions.
    fn try_ternary(&self, if_expr: &syn::ExprIf) -> Option<String> {
        // Must not be if-let
        if matches!(&*if_expr.cond, syn::Expr::Let(_)) { return None; }
        // Must have an else branch
        let (_, else_expr) = if_expr.else_branch.as_ref()?;
        // Then branch must be a single expression
        let then_val = single_block_expr(&if_expr.then_branch)?;
        // Else branch must be a single expression (not another if)
        let else_val = match else_expr.as_ref() {
            syn::Expr::Block(block) => single_block_expr(&block.block)?,
            _ => return None,
        };
        let cond = self.expr(&if_expr.cond);
        let then_ts = self.expr(then_val);
        let else_ts = self.expr(else_val);
        Some(format!("{} ? {} : {}", cond, then_ts, else_ts))
    }

    // ── Method call translation ─────────────────────────────────────
    //
    // Dispatches to native_types modules based on resolved receiver type.
    // System types (Arc, RwLock, Result, etc.) pass through — their TS
    // implementations handle the method names directly.

    fn translate_method_call(&self, receiver: &str, rust_method: &str, ts_method: &str, args: &[String], receiver_expr: Option<&syn::Expr>) -> String {
        // Type-aware dispatch via TypeContext
        if let Some(receiver_expr) = receiver_expr {
            if let Some(receiver_ty) = self.resolve_expr_type(receiver_expr) {
                match native_types::translate_method(&receiver_ty, receiver, rust_method, args) {
                    native_types::MethodTranslation::Expr(result) => return result,
                    native_types::MethodTranslation::Passthrough => {
                        return format!("{}.{}({})", receiver, ts_method, args.join(", "));
                    }
                }
            }
        }

        // No type info — try untyped dispatch (iterator methods, conversions)
        match native_types::translate_untyped(receiver, rust_method, args) {
            native_types::MethodTranslation::Expr(result) => result,
            native_types::MethodTranslation::Passthrough => {
                format!("{}.{}({})", receiver, ts_method, args.join(", "))
            }
        }
    }

    // ── Function call translation ───────────────────────────────────
    //
    // Language-level constructs (Self, Ok/Err/Some/None, enum variants,
    // constructor heuristic) stay here. Type-specific translations
    // (Vec::new, HashMap::new, etc.) are in native_types/ modules.

    fn translate_call(&self, func: &str, args: &[String]) -> String {
        // 0. Resolve inline module qualifiers (e.g., stack.track → track)
        // Resolve inline module qualifiers (e.g., stack.track → track).
        // Import generation is handled by codegen.rs scanning the translated bodies.
        for mod_name in &self.inline_module_names {
            let prefix = format!("{}.", mod_name);
            if let Some(stripped) = func.strip_prefix(&prefix) {
                return self.translate_call(stripped, args);
            }
        }

        // 1. Language-level constructs
        match func {
            "Self" => return format!("new {}({})", self.self_type, args.join(", ")),
            "Ok" => return format!("Result.Ok({})", args.join(", ")),
            "Err" => return format!("Result.Err({})", args.join(", ")),
            "Some" if args.len() == 1 => return args[0].clone(),
            "Some" => return args.join(", "),
            "None" => return "null".to_string(),
            _ => {}
        }

        // 2. Native type static calls (Vec::new, HashMap::new, Arc::clone, etc.)
        if let Some(result) = native_types::translate_static_call(func, args) {
            return result;
        }

        // 3. Serde/bincode crate calls
        match func {
            "serde_json.to_string" | "serde_json::to_string" | "serdeJson.toString"
                if args.len() == 1 => return format!("JSON.stringify({})", args[0]),
            "serde_json.from_str" | "serde_json::from_str" | "serdeJson.fromStr"
                if args.len() == 1 => return format!("JSON.parse({})", args[0]),
            "bincode.serialize" | "bincode::serialize" if args.len() == 1 =>
                return format!("(() => {{ const _w = new BincodeWriter(); {}.encode(_w); return _w.finish(); }})()", args[0]),
            "bincode.deserialize" | "bincode::deserialize" if args.len() == 1 =>
                return format!("(() => {{ const _r = new BincodeReader({}); return /* TODO: need type */ _r; }})()", args[0]),
            _ => {}
        }

        // 4. Box::new is transparent
        if matches!(func, "Box.new" | "Box::new") && args.len() == 1 {
            return args[0].clone();
        }

        // 5. Arc static methods → instance methods
        match func {
            "Arc.asPtr" | "Arc::asPtr" | "Arc.as_ptr" | "Arc::as_ptr"
                if args.len() == 1 => return format!("{}.asPtr()", args[0]),
            "Arc.downgrade" | "Arc::downgrade"
                if args.len() == 1 => return format!("{}.downgrade()", args[0]),
            _ => {}
        }

        // 6. Type::new() constructor pattern
        // System/base types (Arc, Mutex, RwLock, RefCell, etc.) use `new Type(args)` because
        // their TS constructors match the Rust ::new() signature directly.
        // Crate-defined types use `Type.new(args)` because the transpiler emits a
        // `static new()` method with custom initialization logic.
        if func.ends_with(".new") || func.ends_with("::new") {
            let type_name = func.trim_end_matches(".new").trim_end_matches("::new");
            let type_name = if type_name == "Self" { self.self_type } else { type_name };
            // System types with public constructors matching ::new() signature
            let use_constructor = matches!(type_name,
                "Mutex" | "RwLock" | "RefCell" | "HashMap" | "BTreeMap"
                | "HashSet" | "BTreeSet" | "Vec" | "RwLockReadGuard" | "RwLockWriteGuard"
                | "MutexGuard" | "Ref" | "RefMut" | "Box" | "ThreadLocal"
            );
            if use_constructor {
                return format!("new {}({})", type_name, args.join(", "));
            }
            // Everything else (crate-defined types + Arc/Weak): use static new()
            return format!("{}.new({})", type_name, args.join(", "));
        }

        // 7. Self::method() → TypeName.method()
        if func.starts_with("Self.") || func.starts_with("Self::") {
            let method = func.split("::").last()
                .or_else(|| func.split('.').last())
                .unwrap_or(func);
            return format!("{}.{}({})", self.self_type, method, args.join(", "));
        }

        // 8. Enum variant constructor: Type.Variant(args) → new Type('Variant', {...})
        if let Some(dot) = func.rfind('.') {
            let type_name = &func[..dot];
            let variant = &func[dot+1..];

            let is_enum_variant = if let Some(tc) = &self.types {
                tc.borrow().is_variant(type_name, variant)
            } else {
                // Fallback: PascalCase heuristic (no type context available)
                type_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && variant.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && !matches!(type_name, "Math" | "JSON" | "Object" | "Array" | "console" | "Promise")
            };

            if is_enum_variant {
                if args.is_empty() {
                    return format!("new {}('{}', {{}})", type_name, variant);
                } else if args.len() == 1 {
                    return format!("new {}('{}', {{ _0: {} }})", type_name, variant, args[0]);
                } else {
                    let fields: Vec<String> = args.iter().enumerate()
                        .map(|(i, a)| format!("_{}: {}", i, a))
                        .collect();
                    return format!("new {}('{}', {{ {} }})", type_name, variant, fields.join(", "));
                }
            }
        }

        // 9. PascalCase function → constructor heuristic
        if func.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && !func.contains('.')
            && !matches!(func, "Ok" | "Some" | "Err" | "None" | "Self")
        {
            return format!("new {}({})", func, args.join(", "));
        }

        // 10. Default: plain function call
        format!("{}({})", func, args.join(", "))
    }

    // ── Path translation (static) ───────────────────────────────────

    fn path_static(path: &syn::Path) -> String {
        let segments: Vec<String> = path.segments.iter().map(|seg| {
            let name = seg.ident.to_string();
            match name.as_str() {
                "self" => "this".to_string(),
                "Self" => "Self".to_string(),
                "None" => "null".to_string(),
                "true" | "false" => name,
                "Ok" | "Some" | "Err" => name,
                "std" | "core" | "alloc" | "crate" | "super" | "marker" => name,
                "PhantomData" => return "undefined /* PhantomData */".to_string(),
                // Ordering::SeqCst etc. — no JS equivalent, stripped by method call handlers
                "Ordering" => return "undefined /* Ordering */".to_string(),
                _ => {
                    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        name
                    } else {
                        name_map::to_camel_case(&name)
                    }
                }
            }
        }).collect();

        // Strip std/core/alloc module prefixes, keep type+method
        let segments: Vec<String> = segments.into_iter()
            .filter(|s| !matches!(s.as_str(), "std" | "core" | "alloc" | "sync" | "collections" | "convert" | "fmt" | "ops" | "iter" | "atomic" | "marker"))
            .collect();
        let joined = segments.join(".");
        match joined.as_str() {
            s if s.starts_with("crate.") => {
                segments.last().cloned().unwrap_or(joined)
            }
            _ => joined,
        }
    }
}

// ── Standalone helpers ──────────────────────────────────────────────────

fn is_mut_binding(pat: &syn::Pat) -> bool {
    if let syn::Pat::Ident(ident) = pat {
        ident.mutability.is_some()
    } else {
        false
    }
}

fn translate_lit(lit: &syn::Lit) -> String {
    match lit {
        syn::Lit::Str(s) => format!("'{}'", s.value().replace('\'', "\\'")),
        syn::Lit::Int(i) => i.base10_digits().to_string(),
        syn::Lit::Float(f) => f.base10_digits().to_string(),
        syn::Lit::Bool(b) => if b.value { "true" } else { "false" }.to_string(),
        syn::Lit::Char(c) => format!("'{}'", c.value()),
        syn::Lit::Byte(b) => format!("{}", b.value()),
        _ => "/* unknown literal */".to_string(),
    }
}

/// Check if an expression references a variable name as a standalone path
/// (not as a field name in `a.field`). Used for shadow detection.
fn references_var(expr: &syn::Expr, name: &str) -> bool {
    match expr {
        syn::Expr::Path(path) => {
            // Standalone variable reference: just the name
            path.path.segments.len() == 1
                && path.path.segments[0].ident == name
        }
        syn::Expr::MethodCall(call) => {
            // Check receiver and args, but NOT the method name
            references_var(&call.receiver, name)
                || call.args.iter().any(|a| references_var(a, name))
        }
        syn::Expr::Call(call) => {
            references_var(&call.func, name)
                || call.args.iter().any(|a| references_var(a, name))
        }
        syn::Expr::Field(field) => {
            // Check the base, but NOT the field name
            references_var(&field.base, name)
        }
        syn::Expr::Binary(bin) => {
            references_var(&bin.left, name) || references_var(&bin.right, name)
        }
        syn::Expr::Unary(unary) => references_var(&unary.expr, name),
        syn::Expr::Reference(r) => references_var(&r.expr, name),
        syn::Expr::Paren(p) => references_var(&p.expr, name),
        syn::Expr::Block(b) => {
            b.block.stmts.iter().any(|s| match s {
                syn::Stmt::Expr(e, _) => references_var(e, name),
                _ => false,
            })
        }
        syn::Expr::Closure(c) => references_var(&c.body, name),
        _ => false,
    }
}

fn is_assign_op(op: &syn::BinOp) -> bool {
    matches!(op, syn::BinOp::AddAssign(_) | syn::BinOp::SubAssign(_) | syn::BinOp::MulAssign(_)
        | syn::BinOp::DivAssign(_) | syn::BinOp::RemAssign(_) | syn::BinOp::BitXorAssign(_)
        | syn::BinOp::BitAndAssign(_) | syn::BinOp::BitOrAssign(_) | syn::BinOp::ShlAssign(_)
        | syn::BinOp::ShrAssign(_))
}

fn translate_binop(op: &syn::BinOp) -> &'static str {
    match op {
        syn::BinOp::Add(_) => "+",
        syn::BinOp::Sub(_) => "-",
        syn::BinOp::Mul(_) => "*",
        syn::BinOp::Div(_) => "/",
        syn::BinOp::Rem(_) => "%",
        syn::BinOp::And(_) => "&&",
        syn::BinOp::Or(_) => "||",
        syn::BinOp::BitXor(_) => "^",
        syn::BinOp::BitAnd(_) => "&",
        syn::BinOp::BitOr(_) => "|",
        syn::BinOp::Shl(_) => "<<",
        syn::BinOp::Shr(_) => ">>",
        syn::BinOp::Eq(_) => "===",
        syn::BinOp::Lt(_) => "<",
        syn::BinOp::Le(_) => "<=",
        syn::BinOp::Ne(_) => "!==",
        syn::BinOp::Ge(_) => ">=",
        syn::BinOp::Gt(_) => ">",
        syn::BinOp::AddAssign(_) => "+=",
        syn::BinOp::SubAssign(_) => "-=",
        syn::BinOp::MulAssign(_) => "*=",
        syn::BinOp::DivAssign(_) => "/=",
        syn::BinOp::RemAssign(_) => "%=",
        syn::BinOp::BitXorAssign(_) => "^=",
        syn::BinOp::BitAndAssign(_) => "&=",
        syn::BinOp::BitOrAssign(_) => "|=",
        syn::BinOp::ShlAssign(_) => "<<=",
        syn::BinOp::ShrAssign(_) => ">>=",
        _ => "/* unknown op */",
    }
}
