//! What a written expression IS, in the words a diagnostic uses, and the small
//! syntactic questions the inference walk asks of one.
//!
//! Split out of `context.rs`, which was over the 600-line rule. Nothing here
//! resolves a type; each answers a question about the syntax alone.

use crate::name_map;


/// The name of an expression form, so a refusal says which one it could not
/// read rather than only that it could not.
pub fn expr_form(expr: &syn::Expr) -> &'static str {
    match expr {
        syn::Expr::Array(_) => "array",
        syn::Expr::Assign(_) => "assignment",
        syn::Expr::Binary(_) => "binary operator",
        syn::Expr::Break(_) => "break",
        syn::Expr::Closure(_) => "closure",
        syn::Expr::Const(_) => "const block",
        syn::Expr::Continue(_) => "continue",
        syn::Expr::ForLoop(_) => "for loop",
        syn::Expr::If(_) => "if",
        syn::Expr::Index(_) => "index",
        syn::Expr::Let(_) => "let condition",
        syn::Expr::Lit(_) => "literal",
        syn::Expr::Loop(_) => "loop",
        syn::Expr::Macro(_) => "macro invocation",
        syn::Expr::Match(_) => "match",
        syn::Expr::Range(_) => "range",
        syn::Expr::Repeat(_) => "array repeat",
        syn::Expr::Return(_) => "return",
        syn::Expr::Unary(_) => "unary operator",
        syn::Expr::Unsafe(_) => "unsafe block",
        syn::Expr::While(_) => "while loop",
        syn::Expr::Yield(_) => "yield",
        syn::Expr::Async(_) => "async block",
        _ => "this",
    }
}

pub fn member_name(member: &syn::Member) -> String {
    match member {
        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
    }
}

/// Is this an integer literal written without a suffix?
///
/// Rust infers such a literal's type from where it stands. In index position it
/// can only be a `usize`, because that and the ranges of it are what
/// `SliceIndex` is implemented for; nothing else is being decided here.
pub(super) fn is_unsuffixed_int(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(int),
            ..
        }) if int.suffix().is_empty()
    )
}

/// The first value a `break` inside this loop's body carries, which is what the
/// loop's own type is.
///
/// A `break` in a loop written INSIDE the body belongs to that loop, and a
/// closure carries its own control flow, so neither is looked into.
pub(super) fn break_value(block: &syn::Block) -> Option<&syn::Expr> {
    fn in_expr(expr: &syn::Expr) -> Option<&syn::Expr> {
        match expr {
            syn::Expr::Break(brk) if brk.label.is_none() => brk.expr.as_deref(),
            syn::Expr::Loop(_)
            | syn::Expr::While(_)
            | syn::Expr::ForLoop(_)
            | syn::Expr::Closure(_)
            | syn::Expr::Async(_) => None,
            syn::Expr::Block(b) => in_block(&b.block),
            syn::Expr::Unsafe(b) => in_block(&b.block),
            syn::Expr::If(if_expr) => in_block(&if_expr.then_branch).or_else(|| {
                if_expr.else_branch.as_ref().and_then(|(_, other)| in_expr(other))
            }),
            syn::Expr::Match(m) => m.arms.iter().find_map(|arm| in_expr(&arm.body)),
            _ => None,
        }
    }
    fn in_block(block: &syn::Block) -> Option<&syn::Expr> {
        block.stmts.iter().find_map(|stmt| match stmt {
            syn::Stmt::Expr(expr, _) => in_expr(expr),
            syn::Stmt::Local(local) => local.init.as_ref().and_then(|init| in_expr(&init.expr)),
            _ => None,
        })
    }
    in_block(block)
}
