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

/// Translate a block of statements to TS
pub fn translate_block(block: &syn::Block) -> String {
    translate_block_with_self(block, "Self")
}

/// Translate a block with a known self type name for Self resolution
pub fn translate_block_with_self(block: &syn::Block, self_type: &str) -> String {
    SELF_TYPE.with(|cell| cell.replace(self_type.to_string()));
    let mut out = String::new();
    let stmts = &block.stmts;

    // Collect local bindings for drop insertion
    let mut locals: Vec<(String, String)> = Vec::new(); // (var_name, type_hint)
    for stmt in stmts {
        if let syn::Stmt::Local(local) = stmt {
            collect_local_bindings(&local.pat, &mut locals);
        }
    }

    // Determine which locals are consumed (returned, passed as args, stored as fields)
    let mut consumed_vars = std::collections::HashSet::new();
    for (i, stmt) in stmts.iter().enumerate() {
        let is_last = i == stmts.len() - 1;
        if is_last {
            // Last expression without semicolon = implicit return, its vars are consumed
            if let syn::Stmt::Expr(expr, None) = stmt {
                collect_direct_vars(expr, &mut consumed_vars);
            }
        }
        collect_consumed_vars_in_stmt(stmt, &mut consumed_vars);
    }

    for (i, stmt) in stmts.iter().enumerate() {
        let is_last = i == stmts.len() - 1;
        if is_last {
            if let syn::Stmt::Expr(expr, None) = stmt {
                // Before the return, drop all locals that aren't being returned
                let drops = generate_drops(&locals, &consumed_vars);
                if !drops.is_empty() {
                    out.push_str(&drops);
                }
                out.push_str(&control_flow::translate_expr_in_return_position(expr));
                out.push('\n');
            } else {
                // Last statement with semicolon — drop everything after it
                out.push_str(&translate_stmt(stmt));
                let drops = generate_drops(&locals, &consumed_vars);
                if !drops.is_empty() {
                    out.push_str(&drops);
                }
            }
        } else {
            out.push_str(&translate_stmt(stmt));
        }
    }
    out
}

/// Collect variable names from a let binding pattern
fn collect_local_bindings(pat: &syn::Pat, locals: &mut Vec<(String, String)>) {
    match pat {
        syn::Pat::Ident(ident) => {
            let name = name_map::to_camel_case(&ident.ident.to_string());
            // We don't have reliable type info, so store empty type hint
            // The drop will be emitted for all non-primitive-looking bindings
            locals.push((name, String::new()));
        }
        syn::Pat::Tuple(tuple) => {
            for elem in &tuple.elems {
                collect_local_bindings(elem, locals);
            }
        }
        syn::Pat::Type(t) => {
            collect_local_bindings(&t.pat, locals);
        }
        _ => {}
    }
}

/// Collect variables consumed in a statement (passed, stored, returned)
fn collect_consumed_vars_in_stmt(stmt: &syn::Stmt, vars: &mut std::collections::HashSet<String>) {
    match stmt {
        syn::Stmt::Expr(expr, _) => collect_consumed_in_expr(expr, vars),
        syn::Stmt::Local(local) => {
            // RHS of let binding — vars used in init are consumed
            if let Some(init) = &local.init {
                // BUT: only collect vars passed to functions/constructors, not simple reads
                collect_consumed_in_expr(&init.expr, vars);
            }
        }
        _ => {}
    }
}

/// Collect variables consumed by an expression
/// "Consumed" means: passed as an owned argument, stored as a field, or returned
fn collect_consumed_in_expr(expr: &syn::Expr, vars: &mut std::collections::HashSet<String>) {
    match expr {
        // Function calls — arguments are consumed (moved to callee)
        syn::Expr::Call(call) => {
            for arg in &call.args {
                collect_direct_vars(arg, vars);
            }
            // Also recurse into the function expression itself
            collect_consumed_in_expr(&call.func, vars);
        }
        // Method calls — some args are consumed, receiver is borrowed (not consumed)
        syn::Expr::MethodCall(call) => {
            let method = call.method.to_string();
            // Methods that consume their arguments (store/move them)
            if matches!(method.as_str(), "insert" | "push" | "set" | "splice" | "extend") {
                for arg in &call.args {
                    collect_direct_vars(arg, vars);
                }
            }
            // Recurse into args for nested consumption patterns
            for arg in &call.args {
                collect_consumed_in_expr(arg, vars);
            }
            // Receiver is generally borrowed, but recurse to find nested consumption
            // (e.g., ids.into_iter().collect() — into_iter consumes ids)
            collect_consumed_in_expr(&call.receiver, vars);
            // Direct consumption: .into() / .into_iter() consume the receiver
            if matches!(method.as_str(), "into" | "into_iter" | "into_inner") {
                collect_direct_vars(&call.receiver, vars);
            }
        }
        // Assignment — RHS is consumed
        syn::Expr::Assign(assign) => {
            collect_direct_vars(&assign.right, vars);
        }
        // Struct construction — field values are consumed
        syn::Expr::Struct(s) => {
            for field in &s.fields {
                collect_direct_vars(&field.expr, vars);
            }
        }
        // Return — value is consumed
        syn::Expr::Return(ret) => {
            if let Some(expr) = &ret.expr {
                collect_direct_vars(expr, vars);
            }
        }
        // If/else — recurse into branches
        syn::Expr::If(if_expr) => {
            collect_consumed_in_expr(&if_expr.cond, vars);
            for stmt in &if_expr.then_branch.stmts {
                collect_consumed_vars_in_stmt(stmt, vars);
            }
            if let Some((_, else_expr)) = &if_expr.else_branch {
                collect_consumed_in_expr(else_expr, vars);
            }
        }
        // Match — recurse into arms
        syn::Expr::Match(match_expr) => {
            for arm in &match_expr.arms {
                collect_consumed_in_expr(&arm.body, vars);
            }
        }
        // Block — recurse into statements
        syn::Expr::Block(block) => {
            for stmt in &block.block.stmts {
                collect_consumed_vars_in_stmt(stmt, vars);
            }
        }
        // For loop — recurse
        syn::Expr::ForLoop(for_loop) => {
            for stmt in &for_loop.body.stmts {
                collect_consumed_vars_in_stmt(stmt, vars);
            }
        }
        _ => {}
    }
}

/// Collect direct variable references (not nested in sub-expressions)
/// Only collects top-level Path expressions (variable names)
fn collect_direct_vars(expr: &syn::Expr, vars: &mut std::collections::HashSet<String>) {
    match expr {
        syn::Expr::Path(path) => {
            if let Some(seg) = path.path.segments.last() {
                let name = seg.ident.to_string();
                // Skip self, Self, true, false, None
                if !matches!(name.as_str(), "self" | "Self" | "true" | "false" | "None") {
                    vars.insert(name_map::to_camel_case(&name));
                }
            }
        }
        syn::Expr::Reference(r) => {
            // &x — x is borrowed, not consumed. BUT in our model we're conservative.
            // Actually, &x means borrow — the caller keeps ownership. Don't mark as consumed.
        }
        syn::Expr::Tuple(tuple) => {
            for elem in &tuple.elems {
                collect_direct_vars(elem, vars);
            }
        }
        _ => {
            // Complex expression — recurse to find consumed vars
            collect_consumed_in_expr(expr, vars);
        }
    }
}

/// Generate .drop() calls for locals not consumed (not stored/passed/returned)
fn generate_drops(
    locals: &[(String, String)],
    consumed_vars: &std::collections::HashSet<String>,
) -> String {
    let mut out = String::new();

    // Drop in reverse order (mirrors Rust's drop order)
    for (name, _type_hint) in locals.iter().rev() {
        if consumed_vars.contains(name) {
            continue; // Moved to another owner — don't drop
        }
        out.push_str(&format!("{}.drop();\n", name));
    }
    out
}

thread_local! {
    static SELF_TYPE: std::cell::RefCell<String> = std::cell::RefCell::new("Self".to_string());
}

pub fn get_self_type() -> String {
    SELF_TYPE.with(|cell| cell.borrow().clone())
}

/// Translate a single statement
fn translate_stmt(stmt: &syn::Stmt) -> String {
    match stmt {
        syn::Stmt::Local(local) => translate_local(local),
        syn::Stmt::Expr(expr, semi) => {
            let ts = translate_expr(expr);
            if semi.is_some() {
                format!("{};\n", ts)
            } else {
                // Last expression in block (implicit return)
                format!("{}\n", ts)
            }
        }
        syn::Stmt::Item(_) => String::new(), // Items in blocks are rare, skip for now
        syn::Stmt::Macro(macro_stmt) => {
            let ts = translate_macro(&macro_stmt.mac);
            if macro_stmt.semi_token.is_some() {
                format!("{};\n", ts)
            } else {
                format!("{}\n", ts)
            }
        }
    }
}

/// Translate a let binding
fn translate_local(local: &syn::Local) -> String {
    let pat = translate_pat(&local.pat);

    if let Some(init) = &local.init {
        let expr = translate_expr(&init.expr);

        // Check for `let ... else` pattern
        if let Some((_, diverge)) = &init.diverge {
            // let Some(x) = expr else { return/panic };
            // This is complex — for now emit as-is with a comment
            return format!("/* let-else */ const {} = {};\n", pat, expr);
        }

        // Determine const vs let — use let if the pattern is mut
        let keyword = if is_mut_binding(&local.pat) { "let" } else { "const" };
        format!("{} {} = {};\n", keyword, pat, expr)
    } else {
        format!("let {};\n", pat)
    }
}

/// Translate a pattern (let binding LHS)
pub fn translate_pat(pat: &syn::Pat) -> String {
    match pat {
        syn::Pat::Ident(ident) => {
            name_map::to_camel_case(&ident.ident.to_string())
        }
        syn::Pat::Tuple(tuple) => {
            let parts: Vec<String> = tuple.elems.iter().map(translate_pat).collect();
            format!("[{}]", parts.join(", "))
        }
        syn::Pat::TupleStruct(ts) => {
            // Pattern like Some(x) or Ok(x)
            let parts: Vec<String> = ts.elems.iter().map(translate_pat).collect();
            parts.join(", ")
        }
        syn::Pat::Struct(s) => {
            let fields: Vec<String> = s.fields.iter().map(|f| {
                let member = match &f.member {
                    syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                    syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                };
                let pat = translate_pat(&f.pat);
                if member == pat {
                    member
                } else {
                    format!("{}: {}", member, pat)
                }
            }).collect();
            format!("{{ {} }}", fields.join(", "))
        }
        syn::Pat::Wild(_) => "_".to_string(),
        syn::Pat::Lit(_) => "/* pat literal */".to_string(),
        syn::Pat::Path(path) => translate_path(&path.path),
        syn::Pat::Reference(r) => translate_pat(&r.pat),
        syn::Pat::Type(t) => translate_pat(&t.pat),
        syn::Pat::Or(or_pat) => {
            // Pattern1 | Pattern2 — used in match arms
            let parts: Vec<String> = or_pat.cases.iter().map(translate_pat).collect();
            parts.join(" | ")
        }
        syn::Pat::Slice(slice) => {
            let parts: Vec<String> = slice.elems.iter().map(translate_pat).collect();
            format!("[{}]", parts.join(", "))
        }
        syn::Pat::Rest(_) => "...".to_string(),
        _ => "/* unknown pat */".to_string(),
    }
}

/// Check if a binding pattern is mutable
fn is_mut_binding(pat: &syn::Pat) -> bool {
    if let syn::Pat::Ident(ident) = pat {
        ident.mutability.is_some()
    } else {
        false
    }
}

/// Translate a Rust expression to TS
pub fn translate_expr(expr: &syn::Expr) -> String {
    match expr {
        syn::Expr::Lit(lit) => translate_lit(&lit.lit),

        syn::Expr::Path(path) => translate_path(&path.path),

        syn::Expr::Field(field) => {
            let base = translate_expr(&field.base);
            let member = match &field.member {
                syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                syn::Member::Unnamed(idx) => format!("_{}", idx.index),
            };
            // self.field → this.field
            if base == "self" {
                format!("this.{}", member)
            } else {
                format!("{}.{}", base, member)
            }
        }

        syn::Expr::MethodCall(call) => {
            let receiver = translate_expr(&call.receiver);
            let method = name_map::map_fn_name(&call.method.to_string());
            let args: Vec<String> = call.args.iter().map(|a| translate_expr(a)).collect();

            // Special method translations
            match method.as_str() {
                // Result/Option methods
                "unwrap" | "expect" => receiver,
                "unwrapOr" if args.len() == 1 => format!("{} ?? {}", receiver, args[0]),
                "unwrapOrElse" if args.len() == 1 => format!("{} ?? ({})()", receiver, args[0]),
                "unwrapOrDefault" => format!("{} ?? default()", receiver),
                "isOk" | "isSome" => format!("{} != null", receiver),
                "isErr" | "isNone" => format!("{} == null", receiver),
                "ok" => receiver, // .ok() → just the value
                "mapErr" if args.len() == 1 => receiver, // .map_err() → error handling is via throw

                // Collection methods
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

                // Iterator methods
                "map" if args.len() == 1 => format!("{}.map({})", receiver, args[0]),
                "filter" if args.len() == 1 => format!("{}.filter({})", receiver, args[0]),
                "any" if args.len() == 1 => format!("{}.some({})", receiver, args[0]),
                "all" if args.len() == 1 => format!("{}.every({})", receiver, args[0]),
                "find" if args.len() == 1 => format!("{}.find({})", receiver, args[0]),
                "position" if args.len() == 1 => format!("{}.findIndex({})", receiver, args[0]),
                "enumerate" => format!("{}.entries()", receiver),
                "collect" => receiver, // .collect() → no-op in TS
                "iter" | "intoIter" => receiver, // .iter() → just the collection
                "cloned" => format!("[...{}]", receiver), // .cloned() on iter → spread copy

                // Conversion methods
                "clone" => format!("{}.clone()", receiver),
                "toOwned" => format!("{}.clone()", receiver),
                "toString" | "toStr" => format!("{}.toString()", receiver),
                "into" => receiver, // .into() → implicit conversion
                "from" => receiver,
                "asRef" => receiver, // .as_ref() → just the value
                "asMut" => receiver,

                // Vec-specific
                "insert" if args.len() == 2 => {
                    format!("{}.splice({}, 0, {})", receiver, args[0], args[1])
                }
                "remove" if args.len() == 1 => {
                    format!("{}.splice({}, 1)[0]", receiver, args[0])
                }
                "extend" if args.len() == 1 => {
                    format!("{}.push(...{})", receiver, args[0])
                }
                "clear" => format!("{}.length = 0", receiver),
                "truncate" if args.len() == 1 => format!("{}.length = {}", receiver, args[0]),
                "drain" => format!("{}.splice(0)", receiver),

                // Map-specific
                "insertMap" if args.len() == 2 => format!("{}.set({}, {})", receiver, args[0], args[1]),
                "entry" => format!("/* {}.entry({}) */", receiver, args.join(", ")),

                // String methods
                "startsWith" if args.len() == 1 => format!("{}.startsWith({})", receiver, args[0]),
                "endsWith" if args.len() == 1 => format!("{}.endsWith({})", receiver, args[0]),
                "trim" => format!("{}.trim()", receiver),
                "splitStr" if args.len() == 1 => format!("{}.split({})", receiver, args[0]),
                "replacen" | "replace" => format!("{}.replace({})", receiver, args.join(", ")),

                // Comparison
                "cmp" | "partialCmp" if args.len() == 1 => format!("{}.compareTo({})", receiver, args[0]),
                "eq" if args.len() == 1 => format!("{}.equals({})", receiver, args[0]),

                // Binary search
                "binarySearch" if args.len() == 1 => format!("{}.binarySearch({})", receiver, args[0]),

                _ => format!("{}.{}({})", receiver, method, args.join(", ")),
            }
        }

        syn::Expr::Call(call) => {
            let func = translate_expr(&call.func);
            let args: Vec<String> = call.args.iter().map(|a| translate_expr(a)).collect();

            // Special function translations
            match func.as_str() {
                "Self" => format!("new {}({})", get_self_type(), args.join(", ")),
                "Ok" | "Some" => {
                    // Ok(x) / Some(x) → x (Result/Option unwrapping)
                    if args.len() == 1 { args[0].clone() } else { args.join(", ") }
                }
                "Err" => format!("throw new Error({})", args.join(", ")),
                "None" => "null".to_string(),
                "Vec.new" | "Vec::new" => "[]".to_string(),
                "HashMap.new" | "HashMap::new" | "BTreeMap.new" | "BTreeMap::new" => "new Map()".to_string(),
                "HashSet.new" | "HashSet::new" | "BTreeSet.new" | "BTreeSet::new" => "new Set()".to_string(),
                "String.new" | "String::new" => "''".to_string(),
                _ if func.ends_with(".new") || func.ends_with("::new") => {
                    let type_name = func.trim_end_matches(".new").trim_end_matches("::new");
                    format!("new {}({})", type_name, args.join(", "))
                }
                _ if func.starts_with("Self.") || func.starts_with("Self::") => {
                    let method = func.split('.').last().unwrap_or(&func);
                    let method = func.split("::").last().unwrap_or(method);
                    format!("{}.{}({})", get_self_type(), method, args.join(", "))
                }
                _ => {
                    // If func is PascalCase (a type name), treat as constructor
                    if func.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                        && !func.contains('.')
                        && !matches!(func.as_str(), "Ok" | "Some" | "Err" | "None" | "Self")
                    {
                        format!("new {}({})", func, args.join(", "))
                    } else {
                        format!("{}({})", func, args.join(", "))
                    }
                }
            }
        }

        syn::Expr::Binary(bin) => {
            let left = translate_expr(&bin.left);
            let right = translate_expr(&bin.right);
            let op = translate_binop(&bin.op);
            format!("{} {} {}", left, op, right)
        }

        syn::Expr::Unary(unary) => {
            let expr = translate_expr(&unary.expr);
            match &unary.op {
                syn::UnOp::Not(_) => format!("!{}", expr),
                syn::UnOp::Neg(_) => format!("-{}", expr),
                syn::UnOp::Deref(_) => expr, // *x → x in TS
                _ => format!("/* unknown unary op */ {}", expr),
            }
        }

        syn::Expr::If(if_expr) => control_flow::translate_if(if_expr),

        syn::Expr::Block(block) => {
            // If block has a single expression, unwrap it (avoid unnecessary {})
            if block.block.stmts.len() == 1 {
                if let syn::Stmt::Expr(expr, None) = &block.block.stmts[0] {
                    return translate_expr(expr);
                }
            }
            let body = translate_block(&block.block);
            format!("{{\n{}}}", indent(&body))
        }

        syn::Expr::Return(ret) => {
            if let Some(expr) = &ret.expr {
                format!("return {}", translate_expr(expr))
            } else {
                "return".to_string()
            }
        }

        syn::Expr::Match(me) => match_expr::translate_match(me),

        syn::Expr::Closure(closure) => {
            let params: Vec<String> = closure.inputs.iter().map(translate_pat).collect();
            let body = translate_expr(&closure.body);
            if params.len() == 1 {
                format!("({}) => {}", params[0], body)
            } else {
                format!("({}) => {}", params.join(", "), body)
            }
        }

        syn::Expr::ForLoop(for_loop) => {
            let pat = translate_pat(&for_loop.pat);
            let iter = translate_expr(&for_loop.expr);
            let body = translate_block(&for_loop.body);
            format!("for (const {} of {}) {{\n{}}}", pat, iter, indent(&body))
        }

        syn::Expr::While(while_loop) => {
            let cond = translate_expr(&while_loop.cond);
            let body = translate_block(&while_loop.body);
            format!("while ({}) {{\n{}}}", cond, indent(&body))
        }

        syn::Expr::Loop(loop_expr) => {
            let body = translate_block(&loop_expr.body);
            format!("while (true) {{\n{}}}", indent(&body))
        }

        syn::Expr::Break(brk) => {
            if let Some(expr) = &brk.expr {
                format!("break /* {} */", translate_expr(expr))
            } else {
                "break".to_string()
            }
        }

        syn::Expr::Continue(_) => "continue".to_string(),

        syn::Expr::Assign(assign) => {
            let left = translate_expr(&assign.left);
            let right = translate_expr(&assign.right);
            format!("{} = {}", left, right)
        }

        syn::Expr::Index(idx) => {
            let base = translate_expr(&idx.expr);
            let index = translate_expr(&idx.index);
            format!("{}[{}]", base, index)
        }

        syn::Expr::Reference(reference) => {
            // &x → x (references are implicit in TS)
            translate_expr(&reference.expr)
        }

        syn::Expr::Paren(paren) => {
            format!("({})", translate_expr(&paren.expr))
        }

        syn::Expr::Tuple(tuple) => {
            let parts: Vec<String> = tuple.elems.iter().map(|e| translate_expr(e)).collect();
            format!("[{}]", parts.join(", "))
        }

        syn::Expr::Array(arr) => {
            let items: Vec<String> = arr.elems.iter().map(|e| translate_expr(e)).collect();
            format!("[{}]", items.join(", "))
        }

        syn::Expr::Struct(s) => {
            let name = translate_path(&s.path);
            let fields: Vec<String> = s.fields.iter().map(|f| {
                let member = match &f.member {
                    syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                    syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                };
                let value = translate_expr(&f.expr);
                if member == value {
                    member
                } else {
                    format!("{}: {}", member, value)
                }
            }).collect();
            format!("new {}({{ {} }})", name, fields.join(", "))
        }

        syn::Expr::Try(try_expr) => {
            // expr? → expr (errors throw in TS)
            translate_expr(&try_expr.expr)
        }

        syn::Expr::Await(await_expr) => {
            format!("await {}", translate_expr(&await_expr.base))
        }

        syn::Expr::Range(range) => {
            // Ranges don't have a direct TS equivalent
            let from = range.start.as_ref().map(|e| translate_expr(e)).unwrap_or_default();
            let to = range.end.as_ref().map(|e| translate_expr(e)).unwrap_or_default();
            format!("/* range {}..{} */", from, to)
        }

        syn::Expr::Cast(cast) => {
            let expr = translate_expr(&cast.expr);
            let ty = name_map::map_type(&cast.ty);
            format!("{} as {}", expr, ty)
        }

        syn::Expr::Macro(mac) => translate_macro(&mac.mac),

        syn::Expr::Unsafe(unsafe_block) => {
            // unsafe { ... } → emit contents with warning (candidate for provided impl)
            let body = translate_block(&unsafe_block.block).trim().to_string();
            format!("/* unsafe — consider provided impl */ {}", body)
        }

        syn::Expr::Async(async_block) => {
            let body = translate_block(&async_block.block);
            format!("(async () => {{\n{}}})()", indent(&body))
        }

        syn::Expr::Let(let_expr) => {
            // Standalone let expression (used in if-let conditions)
            let pat = translate_pat(&let_expr.pat);
            let expr = translate_expr(&let_expr.expr);
            format!("/* let {} = {} */", pat, expr)
        }

        syn::Expr::Repeat(repeat) => {
            // [expr; N] — array repeat
            let expr = translate_expr(&repeat.expr);
            let len = translate_expr(&repeat.len);
            format!("Array({}).fill({})", len, expr)
        }

        _ => {
            // Fallback for unhandled expression types
            format!("/* TODO: unhandled expr */")
        }
    }
}

/// Translate a literal
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

/// Translate a path expression (variable, type, enum variant)
fn translate_path(path: &syn::Path) -> String {
    let segments: Vec<String> = path.segments.iter().map(|seg| {
        let name = seg.ident.to_string();
        match name.as_str() {
            "self" => "this".to_string(),
            "Self" => "Self".to_string(),
            "None" => "null".to_string(),
            "true" | "false" => name,
            "Ok" | "Some" | "Err" => name, // Handled at call site
            // Skip crate-level path segments
            "std" | "core" | "alloc" | "crate" | "super" => name,
            _ => {
                if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    name
                } else {
                    name_map::to_camel_case(&name)
                }
            }
        }
    }).collect();

    // Clean up path: "crate.foo.Bar" → "Bar", "std.collections.HashMap" → "Map"
    let joined = segments.join(".");
    match joined.as_str() {
        s if s.contains("std.") || s.contains("core.") => {
            // Extract last segment
            segments.last().cloned().unwrap_or(joined)
        }
        s if s.starts_with("crate.") => {
            // crate.module.Type → Type (import handles the path)
            segments.last().cloned().unwrap_or(joined)
        }
        _ => joined,
    }
}

/// Translate a binary operator
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

// If/if-let/return-position → control_flow module
// Macros → macros module

fn translate_macro(mac: &syn::Macro) -> String {
    macros::translate_macro(mac)
}

/// Indent each line of a string by 2 spaces
pub fn indent(s: &str) -> String {
    s.lines()
        .map(|line| if line.is_empty() { String::new() } else { format!("  {}", line) })
        .collect::<Vec<_>>()
        .join("\n")
        + if s.ends_with('\n') { "\n" } else { "" }
}

