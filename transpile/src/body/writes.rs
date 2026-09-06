//! Which expressions are a `write!` — the macro a `Display` body composes its
//! string out of.
//!
//! A formatter's `write!` APPENDS, in all four forms it is written in: with and
//! without `?`, with and without a semicolon, and inside every arm of a match
//! that is itself the body's tail. Reading only one form left everything the
//! body had written before it thrown away.

use super::single_block_expr;

/// Check if an expression is a write!/writeln! macro call
pub(crate) fn is_write_macro(expr: &syn::Expr) -> bool {
    if let syn::Expr::Macro(mac) = expr {
        let name = mac.mac.path.segments.last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        matches!(name.as_str(), "write" | "writeln")
    } else {
        false
    }
}

/// Does this formatter body COMPOSE, or does every path through it write once?
///
/// `fn fmt(&self, f: &mut Formatter) -> fmt::Result { write!(f, "{}", self.0) }`
/// composes nothing: the one write IS the string, and so is each arm of a
/// `match self { .. }` whose arms each write once. Those need no accumulator,
/// and the method stays the expression it always was. A body that writes twice
/// in sequence needs one, and it is the only thing that does.
pub fn writes_once_at_the_tail(block: &syn::Block) -> bool {
    matches!(block.stmts.as_slice(), [syn::Stmt::Expr(expr, None)] if writes_once(expr))
}

pub(crate) fn writes_once(expr: &syn::Expr) -> bool {
    match expr {
        _ if as_write_macro(expr).is_some() => true,
        syn::Expr::Match(m) => m.arms.iter().all(|arm| writes_once(&arm.body)),
        syn::Expr::If(if_expr) => {
            single_block_expr(&if_expr.then_branch).is_some_and(writes_once)
                && if_expr.else_branch.as_ref().is_some_and(|(_, e)| writes_once(e))
        }
        syn::Expr::Block(block) => writes_once_at_the_tail(&block.block),
        syn::Expr::Paren(p) => writes_once(&p.expr),
        _ => false,
    }
}

/// Is this macro a `write!` or a `writeln!`?
pub(crate) fn is_write_macro_path(mac: &syn::Macro) -> bool {
    let name = mac.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
    matches!(name.as_str(), "write" | "writeln")
}

/// The `write!`/`writeln!` an expression is, through the `?` it may carry.
pub(crate) fn as_write_macro(expr: &syn::Expr) -> Option<&syn::Macro> {
    match expr {
        syn::Expr::Try(try_expr) => as_write_macro(&try_expr.expr),
        syn::Expr::Paren(p) => as_write_macro(&p.expr),
        _ if is_write_macro(expr) => extract_macro(expr),
        _ => None,
    }
}

/// The write a formatter statement performs, and whether the statement LEAVES
/// the formatter having performed it.
///
/// `write!(f, "..")` appends and carries on; `return write!(f, "..")` appends
/// and then answers what the formatter has composed. The second was read as an
/// ordinary `return`, so the string it wrote became the whole answer and
/// everything written before it was discarded: `Display for Size` answered
/// `'big)'` where Rust answers `Size(big)`.
pub(crate) fn formatter_write(expr: &syn::Expr) -> Option<(&syn::Macro, bool)> {
    match expr {
        syn::Expr::Return(ret) => {
            let value = ret.expr.as_deref()?;
            as_write_macro(value).map(|mac| (mac, true))
        }
        _ => as_write_macro(expr).map(|mac| (mac, false)),
    }
}

/// Extract the Macro from an expression (for write! detection)
pub(crate) fn extract_macro(expr: &syn::Expr) -> Option<&syn::Macro> {
    if let syn::Expr::Macro(mac) = expr {
        Some(&mac.mac)
    } else {
        None
    }
}

/// Check if a match expression has arms that are write! macro calls (Display pattern)
pub(crate) fn is_match_with_write_arms(expr: &syn::Expr) -> bool {
    if let syn::Expr::Match(m) = expr {
        m.arms.iter().any(|arm| {
            matches!(&*arm.body, syn::Expr::Try(t) if is_write_macro(&t.expr))
        })
    } else {
        false
    }
}
