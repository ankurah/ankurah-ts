//! Control flow translation — if/else, if-let, return position handling

use crate::body::{BodyTranslator, translate_pat, indent};
use crate::name_map;
use crate::match_expr;

/// Translate an if expression (handles if-let patterns)
pub fn translate_if(if_expr: &syn::ExprIf, t: &BodyTranslator) -> String {
    if let syn::Expr::Let(let_expr) = &*if_expr.cond {
        return translate_if_let(let_expr, &if_expr.then_branch, &if_expr.else_branch, None, t);
    }

    // Handle let-chains: if let Some(x) = expr && guard { ... }
    if let syn::Expr::Binary(bin) = &*if_expr.cond {
        if matches!(bin.op, syn::BinOp::And(_)) {
            if let syn::Expr::Let(let_expr) = &*bin.left {
                return translate_if_let(let_expr, &if_expr.then_branch, &if_expr.else_branch, Some(&bin.right), t);
            }
        }
    }

    let cond = t.expr(&if_expr.cond);
    let then_body = t.translate_block(&if_expr.then_branch);

    let else_part = if let Some((_, else_expr)) = &if_expr.else_branch {
        match else_expr.as_ref() {
            syn::Expr::If(else_if) => format!(" else {}", translate_if(else_if, t)),
            syn::Expr::Block(block) => {
                let body = t.translate_block(&block.block);
                format!(" else {{\n{}}}", indent(&body))
            }
            _ => format!(" else {{\n{}}}", indent(&t.expr(else_expr))),
        }
    } else {
        String::new()
    };

    format!("if ({}) {{\n{}}}{}", cond, indent(&then_body), else_part)
}

/// Translate `if let` pattern
fn translate_if_let(
    let_expr: &syn::ExprLet,
    then_branch: &syn::Block,
    else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
    guard: Option<&syn::Expr>,
    t: &BodyTranslator,
) -> String {
    let scrutinee = t.expr(&let_expr.expr);
    // The names the pattern introduces are in scope for the branch it guards,
    // and for the guard expression written after it.
    let scrutinee_ty = t.scrutinee_type(&let_expr.expr);
    let bound = t.enter_pattern(&let_expr.pat, scrutinee_ty.as_ref());
    let then_body = t.translate_block(then_branch);
    let guard_str = guard.map(|g| format!(" && {}", t.expr(g))).unwrap_or_default();
    drop(bound);

    let else_part = if let Some((_, else_expr)) = else_branch {
        match else_expr.as_ref() {
            syn::Expr::If(else_if) => format!(" else {}", translate_if(else_if, t)),
            syn::Expr::Block(block) => {
                let body = t.translate_block(&block.block);
                format!(" else {{\n{}}}", indent(&body))
            }
            _ => format!(" else {{\n{}}}", indent(&t.expr(else_expr))),
        }
    } else {
        String::new()
    };

    match &*let_expr.pat {
        syn::Pat::TupleStruct(ts) => {
            let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            match name.as_str() {
                "Some" => {
                    let var = ts.elems.first().map(translate_pat).unwrap_or_else(|| "v".to_string());
                    if guard_str.is_empty() {
                        format!("if ({} != null) {{\n  const {} = {};\n{}}}{}",
                            scrutinee, var, scrutinee, indent(&then_body), else_part)
                    } else {
                        // Let-chain: bind the variable, then check the guard
                        format!("if ({} != null) {{\n  const {} = {};\n  if ({}) {{\n{}\n  }}\n}}{}",
                            scrutinee, var, scrutinee, guard_str.trim_start_matches(" && "),
                            indent(&indent(&then_body)), else_part)
                    }
                }
                // `if let Ok(v) = r` is a test, not an unwrapping: without it
                // the branch runs whatever `r` turned out to be, and a fallible
                // call becomes an unconditional one.
                //
                // The exception is the lock-guard shim: the port's `read()`
                // yields the guard where Rust yields a `LockResult`, so the `Ok`
                // the source writes has nothing to test and the binding is the
                // guard itself (spec 4.4, deleted when the stubs land).
                "Ok" | "Err" => {
                    let var = ts.elems.first().map(translate_pat).unwrap_or_else(|| "v".to_string());
                    if t.is_lock_guard_expr(&let_expr.expr) {
                        return format!("const {} = {};\n{}", var, scrutinee, then_body);
                    }
                    let (test, take) = if name == "Ok" {
                        ("isOk", "unwrap")
                    } else {
                        ("isErr", "unwrapErr")
                    };
                    format!(
                        "if ({}.{}()) {{\n  const {} = {}.{}();\n{}}}{}",
                        scrutinee, test, var, scrutinee, take, indent(&then_body), else_part
                    )
                }
                _ => {
                    let vars: Vec<String> = ts.elems.iter().map(translate_pat).collect();
                    format!("if ({}.is('{}')) {{\n  const {{ {} }} = {}.value;\n{}}}{}",
                        scrutinee, name, vars.join(", "), scrutinee, indent(&then_body), else_part)
                }
            }
        }
        syn::Pat::Struct(s) => {
            let name = s.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            let fields: Vec<String> = s.fields.iter().map(|f| {
                match &f.member {
                    syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                    syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                }
            }).collect();
            format!("if ({}.is('{}')) {{\n  const {{ {} }} = {}.value;\n{}}}{}",
                scrutinee, name, fields.join(", "), scrutinee, indent(&then_body), else_part)
        }
        _ => {
            let pat = translate_pat(&let_expr.pat);
            format!("if (/* let {} = {} */) {{\n{}}}{}",
                pat, scrutinee, indent(&then_body), else_part)
        }
    }
}

/// Translate expression in return position with pending drops
/// `pending_drops` is emitted before each return statement in every branch
pub fn translate_expr_in_return_position_with(expr: &syn::Expr, t: &BodyTranslator, pending_drops: &str) -> String {
    match expr {
        syn::Expr::If(if_expr) => translate_if_returning_with(if_expr, t, pending_drops),
        syn::Expr::Match(match_expr) => {
            // Drops are not yet threaded into match arms.
            match_expr::translate_match_returning(match_expr, t)
        }
        syn::Expr::Block(block) => {
            if block.block.stmts.len() == 1 {
                if let syn::Stmt::Expr(inner, None) = &block.block.stmts[0] {
                    return translate_expr_in_return_position_with(inner, t, pending_drops);
                }
            }
            let body = t.translate_block(&block.block);
            format!("{{\n{}}}", indent(&body))
        }
        // Loops return () — just emit them as statements
        syn::Expr::ForLoop(_) | syn::Expr::While(_) | syn::Expr::Loop(_) => {
            let ts = t.expr(expr);
            format!("{}\n{}", ts, pending_drops)
        }
        _ => {
            // Leaf case: compute value, emit drops, return
            let ts = t.expr(expr);
            // throw/panic is already a terminator — don't prefix with return
            if ts.starts_with("throw ") {
                format!("{};", ts)
            } else if pending_drops.is_empty() {
                format!("return {};", ts)
            } else {
                format!("const _ret = {};\n{}return _ret;", ts, pending_drops)
            }
        }
    }
}

/// If expression where each branch should return, with pending drops
fn translate_if_returning_with(if_expr: &syn::ExprIf, t: &BodyTranslator, pending_drops: &str) -> String {
    if let syn::Expr::Let(_) = &*if_expr.cond {
        return translate_if(if_expr, t);
    }

    let cond = t.expr(&if_expr.cond);

    let then_body = translate_branch_returning(&if_expr.then_branch, t, pending_drops);

    let else_part = if let Some((_, else_expr)) = &if_expr.else_branch {
        match else_expr.as_ref() {
            syn::Expr::If(else_if) => format!(" else {}", translate_if_returning_with(else_if, t, pending_drops)),
            syn::Expr::Block(block) => {
                let body = translate_branch_returning(&block.block, t, pending_drops);
                format!(" else {{\n{}}}", indent(&body))
            }
            _ => {
                let ts = t.expr(else_expr.as_ref());
                if pending_drops.is_empty() {
                    format!(" else {{\n  return {};\n}}", ts)
                } else {
                    format!(" else {{\n  const _ret = {};\n{}  return _ret;\n}}", ts, indent(pending_drops))
                }
            }
        }
    } else {
        String::new()
    };

    format!("if ({}) {{\n{}}}{}", cond, indent(&then_body), else_part)
}

/// Translate a block that's a branch of an if/else in return position
fn translate_branch_returning(block: &syn::Block, t: &BodyTranslator, pending_drops: &str) -> String {
    if block.stmts.len() == 1 {
        if let Some(syn::Stmt::Expr(expr, None)) = block.stmts.last() {
            return translate_expr_in_return_position_with(expr, t, pending_drops);
        }
    }
    let mut body = t.translate_block(block);
    if !pending_drops.is_empty() {
        if let Some(ret_pos) = body.rfind("return ") {
            body.insert_str(ret_pos, pending_drops);
        } else {
            body.push_str(pending_drops);
        }
    }
    body
}
