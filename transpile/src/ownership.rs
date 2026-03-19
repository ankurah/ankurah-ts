//! Ownership tracking — determines which variables are consumed (moved)
//! and generates .drop() calls for block-scoped values that go out of scope.
//!
//! Mirrors Rust's ownership semantics: variables that are not moved
//! (stored as field, passed as arg, returned) get .drop() at block exit.

use std::collections::HashSet;
use crate::name_map;

/// Collect variable names from a let binding pattern
pub fn collect_local_bindings(pat: &syn::Pat, locals: &mut Vec<(String, String)>) {
    match pat {
        syn::Pat::Ident(ident) => {
            let name = name_map::to_camel_case(&ident.ident.to_string());
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
pub fn collect_consumed_vars_in_stmt(stmt: &syn::Stmt, vars: &mut HashSet<String>) {
    match stmt {
        syn::Stmt::Expr(expr, _) => collect_consumed_in_expr(expr, vars),
        syn::Stmt::Local(local) => {
            if let Some(init) = &local.init {
                collect_consumed_in_expr(&init.expr, vars);
            }
        }
        _ => {}
    }
}

/// Collect variables consumed by an expression
/// "Consumed" means: passed as an owned argument, stored as a field, or returned
fn collect_consumed_in_expr(expr: &syn::Expr, vars: &mut HashSet<String>) {
    match expr {
        // Function calls — arguments are consumed (moved to callee)
        syn::Expr::Call(call) => {
            for arg in &call.args {
                collect_direct_vars(arg, vars);
            }
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
            for arg in &call.args {
                collect_consumed_in_expr(arg, vars);
            }
            // Recurse into receiver chain for nested consumption
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
        // Block — recurse
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

/// Collect direct variable references (top-level Path expressions)
pub fn collect_direct_vars(expr: &syn::Expr, vars: &mut HashSet<String>) {
    match expr {
        syn::Expr::Path(path) => {
            if let Some(seg) = path.path.segments.last() {
                let name = seg.ident.to_string();
                if !matches!(name.as_str(), "self" | "Self" | "true" | "false" | "None") {
                    vars.insert(name_map::to_camel_case(&name));
                }
            }
        }
        syn::Expr::Reference(_) => {
            // &x — borrowed, not consumed
        }
        syn::Expr::Tuple(tuple) => {
            for elem in &tuple.elems {
                collect_direct_vars(elem, vars);
            }
        }
        _ => {
            collect_consumed_in_expr(expr, vars);
        }
    }
}

/// Generate .drop() calls for all locals at end of block, in reverse order.
/// Mirrors Rust: all locals get drop glue. Moved values are idempotent no-ops.
/// Variables in `skip_vars` are NOT dropped (they are the implicit return value).
pub fn generate_drops(
    locals: &[(String, String)],
    returned_vars: &HashSet<String>,
) -> String {
    let mut out = String::new();
    for (name, ty) in locals.iter().rev() {
        // Don't drop the return value — it's being moved to the caller
        if returned_vars.contains(name) {
            continue;
        }
        // Don't drop primitives, arrays, or other non-AkObject types
        if is_non_droppable(ty) {
            continue;
        }
        out.push_str(&format!("{}.drop();\n", name));
    }
    out
}

/// Types that should never get .drop() calls
fn is_non_droppable(ty: &str) -> bool {
    let base = ty.trim_end_matches(" | null");
    matches!(base, "string" | "boolean" | "number" | "bigint | number" | "void" | "never" | "unknown")
        || base.ends_with("[]")
        || base.starts_with('[')  // tuples
        || base == "Uint8Array"
        || base.starts_with("Map<")
        || base.starts_with("Set<")
}
