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
    /// Type registry for resolve_field, resolve_method, is_enum.
    /// None for legacy codepaths (free function shims, match_expr, control_flow).
    pub registry: Option<&'a crate::resolve::TypeRegistry>,
    /// Current module path for module-qualified type lookups.
    /// E.g., "broadcast" for broadcast.rs, "signal/memo" for signal/memo.rs.
    pub current_module: Option<&'a str>,
}

impl<'a> BodyTranslator<'a> {
    pub fn new(self_type: &'a str) -> Self {
        Self { self_type, registry: None, current_module: None }
    }

    pub fn with_registry(self_type: &'a str, registry: &'a crate::resolve::TypeRegistry, module: &'a str) -> Self {
        Self { self_type, registry: Some(registry), current_module: Some(module) }
    }

    /// Resolve the type of a receiver expression for type-aware translation.
    /// Returns the ResolvedType if the registry is available and the receiver
    /// can be resolved from struct field types.
    fn resolve_receiver_type(&self, expr: &syn::Expr) -> Option<crate::resolve::ResolvedType> {
        let registry = self.registry?;
        let module = self.current_module.unwrap_or("");
        match expr {
            // self.field → look up field type on self_type's struct
            syn::Expr::Field(field) => {
                if let syn::Expr::Path(path) = &*field.base {
                    if path.path.is_ident("self") {
                        let member = match &field.member {
                            syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                            syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                        };
                        let self_ty = crate::resolve::ResolvedType::Named {
                            name: self.self_type.to_string(),
                            args: vec![],
                        };
                        return registry.resolve_field_in_module(&self_ty, &member, module)
                            .map(|(ty, _deref)| ty);
                    }
                }
                // Nested field access: resolve the base, then look up the field
                let base_ty = self.resolve_receiver_type(&field.base)?;
                let member = match &field.member {
                    syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                    syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                };
                registry.resolve_field_in_module(&base_ty, &member, module).map(|(ty, _)| ty)
            }
            // self → self_type
            syn::Expr::Path(path) if path.path.is_ident("self") => {
                Some(crate::resolve::ResolvedType::Named {
                    name: self.self_type.to_string(),
                    args: vec![],
                })
            }
            // method_call().field → resolve method return type
            syn::Expr::MethodCall(call) => {
                let receiver_ty = self.resolve_receiver_type(&call.receiver)?;
                let rust_method = call.method.to_string();
                registry.resolve_method_in_module(&receiver_ty, &rust_method, module)
            }
            _ => None,
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

            let expr = self.expr(&init.expr);

            if let Some((_tok, _diverge)) = &init.diverge {
                return format!("/* let-else */ const {} = {};\n", pat, expr);
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

                // Type-aware deref insertion: if the base type has deref_field,
                // insert the accessor (e.g., .value for Arc)
                if let Some(registry) = self.registry {
                    let module = self.current_module.unwrap_or("");
                    if let Some(base_ty) = self.resolve_receiver_type(&field.base) {
                        if let Some((ref _field_ty, Some(ref accessor))) = registry.resolve_field_in_module(&base_ty, &member, module) {
                            if !accessor.is_empty() {
                                return format!("{}.{}.{}", base_str, accessor, member);
                            }
                        }
                    }
                }

                format!("{}.{}", base_str, member)
            }

            syn::Expr::MethodCall(call) => {
                let receiver = self.expr(&call.receiver);
                let rust_method = call.method.to_string();
                let method = name_map::map_fn_name(&rust_method);
                let args: Vec<String> = call.args.iter().map(|a| self.expr(a)).collect();

                // Type-aware method dispatch: if we know the receiver type,
                // check if this method requires deref insertion.
                if let Some(registry) = self.registry {
                    if let Some(receiver_ty) = self.resolve_receiver_type(&call.receiver) {
                        // If the method is NOT on the wrapper type itself, we need to deref first
                        if !registry.is_own_method(&receiver_ty, &rust_method) {
                            if let Some(accessor) = registry.deref_field(&receiver_ty) {
                                if !accessor.is_empty() {
                                    // Insert .value (or other accessor) before the method call
                                    let deref_receiver = format!("{}.{}", receiver, accessor);
                                    return self.translate_method_call(&deref_receiver, &method, &args);
                                }
                            }
                        }
                    }
                }

                self.translate_method_call(&receiver, &method, &args)
            }

            syn::Expr::Call(call) => {
                let func = self.expr(&call.func);
                let args: Vec<String> = call.args.iter().map(|a| self.expr(a)).collect();
                self.translate_call(&func, &args)
            }

            syn::Expr::Binary(bin) => {
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
                let body = self.translate_block(&block.block);
                format!("(() => {{\n{}}})()", indent(&body))
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
                let body = self.expr(&closure.body);
                format!("({}) => {}", params.join(", "), body)
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

    fn translate_method_call(&self, receiver: &str, method: &str, args: &[String]) -> String {
        match method {
            // Result/Option
            "unwrap" | "expect" => receiver.to_string(),
            "unwrapOr" if args.len() == 1 => format!("{} ?? {}", receiver, args[0]),
            "unwrapOrElse" if args.len() == 1 => format!("{} ?? ({})()", receiver, args[0]),
            "unwrapOrDefault" => format!("{} ?? default()", receiver),
            "isOk" | "isSome" => format!("{} != null", receiver),
            "isErr" | "isNone" => format!("{} == null", receiver),
            "ok" | "mapErr" => receiver.to_string(),

            // Collections
            "len" if args.is_empty() => format!("{}.length", receiver),
            "isEmpty" if args.is_empty() => format!("{}.length === 0", receiver),
            "push" => format!("{}.push({})", receiver, args.join(", ")),
            "pop" => format!("{}.pop()", receiver),
            "last" => format!("{}.at(-1)", receiver),
            "first" => format!("{}[0]", receiver),
            "get" if args.len() == 1 => format!("{}.get({})", receiver, args[0]),
            "contains" if args.len() == 1 => format!("{}.includes({})", receiver, args[0]),
            "sort" if args.is_empty() => format!("{}.sort()", receiver),
            "sortBy" if args.len() == 1 => format!("{}.sort({})", receiver, args[0]),
            "reverse" => format!("{}.reverse()", receiver),
            "join" if args.len() == 1 => format!("{}.join({})", receiver, args[0]),

            // Iterators
            "map" if args.len() == 1 => format!("{}.map({})", receiver, args[0]),
            "filter" if args.len() == 1 => format!("{}.filter({})", receiver, args[0]),
            "any" if args.len() == 1 => format!("{}.some({})", receiver, args[0]),
            "all" if args.len() == 1 => format!("{}.every({})", receiver, args[0]),
            "find" if args.len() == 1 => format!("{}.find({})", receiver, args[0]),
            "position" if args.len() == 1 => format!("{}.findIndex({})", receiver, args[0]),
            "enumerate" => format!("{}.entries()", receiver),
            "collect" => receiver.to_string(),
            // Spread to array — works for both arrays (copy) and Maps/Sets (entries)
            // Preserves type inference better than Array.from()
            "iter" | "intoIter" | "values" => format!("[...{}]", receiver),
            "cloned" => format!("[...{}]", receiver),
            "sum" => format!("{}.reduce((a, b) => a + b, 0)", receiver),
            "count" if args.is_empty() => format!("{}.length", receiver),
            "flatten" => format!("{}.flat()", receiver),
            "chain" if args.len() == 1 => format!("[...{}, ...{}]", receiver, args[0]),

            // Conversion
            "clone" => format!("{}.clone()", receiver),
            "toOwned" => format!("{}.clone()", receiver),
            "toString" | "toStr" => format!("{}.toString()", receiver),
            "into" | "from" | "asRef" | "asMut" => receiver.to_string(),

            // Vec
            "insert" if args.len() == 2 => format!("{}.splice({}, 0, {})", receiver, args[0], args[1]),
            "remove" if args.len() == 1 => format!("{}.splice({}, 1)[0]", receiver, args[0]),
            "extend" if args.len() == 1 => format!("{}.push(...{})", receiver, args[0]),
            "clear" => format!("{}.length = 0", receiver),
            "truncate" if args.len() == 1 => format!("{}.length = {}", receiver, args[0]),
            "drain" => format!("{}.splice(0)", receiver),

            // Map
            "insertMap" if args.len() == 2 => format!("{}.set({}, {})", receiver, args[0], args[1]),
            "entry" => format!("/* {}.entry({}) */", receiver, args.join(", ")),

            // String
            "startsWith" if args.len() == 1 => format!("{}.startsWith({})", receiver, args[0]),
            "endsWith" if args.len() == 1 => format!("{}.endsWith({})", receiver, args[0]),
            "trim" => format!("{}.trim()", receiver),
            "splitStr" if args.len() == 1 => format!("{}.split({})", receiver, args[0]),
            "replacen" | "replace" => format!("{}.replace({})", receiver, args.join(", ")),

            // Comparison
            "cmp" | "partialCmp" if args.len() == 1 => format!("{}.compareTo({})", receiver, args[0]),
            "eq" if args.len() == 1 => format!("{}.equals({})", receiver, args[0]),
            "binarySearch" if args.len() == 1 => format!("{}.binarySearch({})", receiver, args[0]),

            // Slices
            "splitLast" if args.is_empty() => format!("{}.length > 0 ? [{}.at(-1), {}.slice(0, -1)] : null", receiver, receiver, receiver),
            "splitFirst" if args.is_empty() => format!("{}.length > 0 ? [{}[0], {}.slice(1)] : null", receiver, receiver, receiver),

            // Atomics
            "fetchAdd" if args.len() >= 1 => format!("(() => {{ const _v = {}; {} += {}; return _v; }})()", receiver, receiver, args[0]),
            // Atomics — Ordering args are stripped (no JS equivalent)
            "load" => receiver.to_string(),
            "store" if args.len() >= 1 => format!("{} = {}", receiver, args[0]),

            // Formatter — TS has no alternate formatting, always false
            "alternate" if args.is_empty() => "false".to_string(),

            _ => format!("{}.{}({})", receiver, method, args.join(", ")),
        }
    }

    // ── Function call translation ───────────────────────────────────

    fn translate_call(&self, func: &str, args: &[String]) -> String {
        match func {
            "Self" => format!("new {}({})", self.self_type, args.join(", ")),
            "Ok" => {
                if args.len() == 1 {
                    format!("Result.Ok({})", args[0])
                } else {
                    format!("Result.Ok({})", args.join(", "))
                }
            }
            "Err" => {
                if args.len() == 1 {
                    format!("Result.Err({})", args[0])
                } else {
                    format!("Result.Err({})", args.join(", "))
                }
            }
            "Some" => {
                if args.len() == 1 { args[0].clone() } else { args.join(", ") }
            }
            "None" => "null".to_string(),
            // Serde crate calls
            "serdeJson.toString" | "serde_json::to_string" if args.len() == 1 =>
                format!("JSON.stringify({})", args[0]),
            "serdeJson.fromStr" | "serde_json::from_str" if args.len() == 1 =>
                format!("JSON.parse({})", args[0]),
            "bincode.serialize" | "bincode::serialize" if args.len() == 1 =>
                format!("(() => {{ const _w = new BincodeWriter(); {}.encode(_w); return _w.finish(); }})()", args[0]),
            "bincode.deserialize" | "bincode::deserialize" if args.len() == 1 =>
                format!("(() => {{ const _r = new BincodeReader({}); return /* TODO: need type */ _r; }})()", args[0]),
            // Box is transparent in TS — Box::new(x) → x
            "Box.new" | "Box::new" if args.len() == 1 => args[0].clone(),
            // AtomicUsize/AtomicU32 are just numbers — new() → value
            "AtomicUsize.new" | "AtomicUsize::new" if args.len() == 1 => args[0].clone(),
            "AtomicU32.new" | "AtomicU32::new" if args.len() == 1 => args[0].clone(),
            // Arc::as_ptr(x) → x.asPtr() (static → instance method)
            "Arc.asPtr" | "Arc::asPtr" if args.len() == 1 => format!("{}.asPtr()", args[0]),
            // Arc::downgrade(x) → x.downgrade()
            "Arc.downgrade" | "Arc::downgrade" if args.len() == 1 => format!("{}.downgrade()", args[0]),
            "Vec.new" | "Vec::new" => "[]".to_string(),
            "HashMap.new" | "HashMap::new" | "BTreeMap.new" | "BTreeMap::new" => "new Map()".to_string(),
            "HashSet.new" | "HashSet::new" | "BTreeSet.new" | "BTreeSet::new" => "new Set()".to_string(),
            "String.new" | "String::new" => "''".to_string(),
            // System types with factory .new() methods (private constructors in TS)
            "Arc.new" | "Arc::new" => format!("Arc.new({})", args.join(", ")),
            _ if func.ends_with(".new") || func.ends_with("::new") => {
                let type_name = func.trim_end_matches(".new").trim_end_matches("::new");
                let type_name = if type_name == "Self" { self.self_type } else { type_name };
                format!("new {}({})", type_name, args.join(", "))
            }
            _ if func.starts_with("Self.") || func.starts_with("Self::") => {
                let method = func.split('.').last().unwrap_or(func);
                let method = func.split("::").last().unwrap_or(method);
                format!("{}.{}({})", self.self_type, method, args.join(", "))
            }
            _ => {
                // Enum variant constructor: Type.Variant(args) → new Type('Variant', { _0: arg, ... })
                if let Some(dot) = func.rfind('.') {
                    let type_name = &func[..dot];
                    let variant = &func[dot+1..];

                    // Use registry for definitive enum detection if available
                    let is_enum_variant = if let Some(registry) = self.registry {
                        registry.is_variant(type_name, variant)
                    } else {
                        // Fallback: PascalCase heuristic
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

                if func.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && !func.contains('.')
                    && !matches!(func, "Ok" | "Some" | "Err" | "None" | "Self")
                {
                    format!("new {}({})", func, args.join(", "))
                } else {
                    format!("{}({})", func, args.join(", "))
                }
            }
        }
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
