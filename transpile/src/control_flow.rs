//! Control flow translation — if/else, if-let, return position handling

use crate::body::{translate_expr, translate_pat, translate_block, translate_block_with_self, indent, BodyTranslator};
use crate::name_map;
use crate::match_expr;

/// Translate an if expression (handles if-let patterns)
pub fn translate_if(if_expr: &syn::ExprIf) -> String {
    // Check for `if let` pattern
    if let syn::Expr::Let(let_expr) = &*if_expr.cond {
        return translate_if_let(let_expr, &if_expr.then_branch, &if_expr.else_branch);
    }

    let cond = translate_expr(&if_expr.cond);
    let then_body = translate_block(&if_expr.then_branch);

    let else_part = if let Some((_, else_expr)) = &if_expr.else_branch {
        match else_expr.as_ref() {
            syn::Expr::If(else_if) => format!(" else {}", translate_if(else_if)),
            syn::Expr::Block(block) => {
                let body = translate_block(&block.block);
                format!(" else {{\n{}}}", indent(&body))
            }
            _ => format!(" else {{\n{}}}", indent(&translate_expr(else_expr))),
        }
    } else {
        String::new()
    };

    format!("if ({}) {{\n{}}}{}", cond, indent(&then_body), else_part)
}

/// Translate `if let` pattern
/// if let Some(x) = expr { ... } → if (expr != null) { const x = expr; ... }
/// if let Pattern::Variant { fields } = expr { ... } → if (expr.is('Variant')) { ... }
fn translate_if_let(
    let_expr: &syn::ExprLet,
    then_branch: &syn::Block,
    else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
) -> String {
    let scrutinee = translate_expr(&let_expr.expr);
    let then_body = translate_block(then_branch);

    let else_part = if let Some((_, else_expr)) = else_branch {
        match else_expr.as_ref() {
            syn::Expr::If(else_if) => format!(" else {}", translate_if(else_if)),
            syn::Expr::Block(block) => {
                let body = translate_block(&block.block);
                format!(" else {{\n{}}}", indent(&body))
            }
            _ => format!(" else {{\n{}}}", indent(&translate_expr(else_expr))),
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
                    format!("if ({} != null) {{\n  const {} = {};\n{}}}{}",
                        scrutinee, var, scrutinee, indent(&then_body), else_part)
                }
                "Ok" => {
                    let var = ts.elems.first().map(translate_pat).unwrap_or_else(|| "v".to_string());
                    format!("const {} = {};\n{}", var, scrutinee, then_body)
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

/// Translate an expression in return position — adds return where needed
pub fn translate_expr_in_return_position(expr: &syn::Expr) -> String {
    translate_expr_in_return_position_with(expr, &BodyTranslator::new("Self"))
}

/// Translate expression in return position with a specific translator
pub fn translate_expr_in_return_position_with(expr: &syn::Expr, t: &BodyTranslator) -> String {
    match expr {
        syn::Expr::If(if_expr) => translate_if_returning_with(if_expr, t),
        syn::Expr::Match(match_expr) => match_expr::translate_match_returning(match_expr),
        syn::Expr::Block(block) => {
            if block.block.stmts.len() == 1 {
                if let syn::Stmt::Expr(inner, None) = &block.block.stmts[0] {
                    return translate_expr_in_return_position_with(inner, t);
                }
            }
            let body = t.translate_block(&block.block);
            format!("{{\n{}}}", indent(&body))
        }
        _ => {
            let ts = t.expr(expr);
            format!("return {};", ts)
        }
    }
}

/// If expression where each branch should return a value
fn translate_if_returning_with(if_expr: &syn::ExprIf, t: &BodyTranslator) -> String {
    if let syn::Expr::Let(_) = &*if_expr.cond {
        return translate_if(if_expr);
    }

    let cond = t.expr(&if_expr.cond);

    let then_body = if if_expr.then_branch.stmts.len() == 1 {
        if let Some(syn::Stmt::Expr(expr, None)) = if_expr.then_branch.stmts.last() {
            translate_expr_in_return_position_with(expr, t)
        } else {
            t.translate_block(&if_expr.then_branch)
        }
    } else {
        t.translate_block(&if_expr.then_branch)
    };

    let else_part = if let Some((_, else_expr)) = &if_expr.else_branch {
        match else_expr.as_ref() {
            syn::Expr::If(else_if) => format!(" else {}", translate_if_returning_with(else_if, t)),
            syn::Expr::Block(block) => {
                let body = if block.block.stmts.len() == 1 {
                    if let Some(syn::Stmt::Expr(expr, None)) = block.block.stmts.last() {
                        translate_expr_in_return_position_with(expr, t)
                    } else {
                        t.translate_block(&block.block)
                    }
                } else {
                    t.translate_block(&block.block)
                };
                format!(" else {{\n{}}}", indent(&body))
            }
            _ => {
                let ts = t.expr(else_expr.as_ref());
                format!(" else {{\n  return {};\n}}", ts)
            }
        }
    } else {
        String::new()
    };

    format!("if ({}) {{\n{}}}{}", cond, indent(&then_body), else_part)
}
