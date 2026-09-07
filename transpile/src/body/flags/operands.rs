//! What a statement's operands are, for the question of where its move flags
//! stand.
//!
//! A flag says "somebody else owns this now", so nothing that can throw may
//! stand between it and the call it reports. These three answer which operands
//! those are: what Rust evaluates quietly, what the PORT's own text evaluates
//! (the port writes calls the source does not have), and what the port writes
//! as a literal, which builds nothing at all.

/// Can this argument be left where it stands, because evaluating it cannot
/// throw?
///
/// A name is the original answer: reading one cannot fail, and naming it would
/// only add noise. A LITERAL and a CLOSURE are the same — `f(c, false)` and
/// `opt.map(|v| v)` build a value out of nothing, and lifting them wrote
/// `const _b2 = false;` and `const _b6 = (v) => v;` above two live statements.
///
/// N1: a FIELD and an INDEX are quiet only if what they are read OUT of is.
/// This asked `is_place`, which answers "does this name existing storage" and
/// says yes to any `Expr::Field` and any `Expr::Index` without looking at the
/// base or at the index — so `eat(c, maybe().n)` and `eat(c, xs[which()])` set
/// the flag before `maybe()` and `which()` ran, and on their throw path the
/// block released nothing. Reused as "can evaluating this throw", `is_place`
/// was answering a different question.
pub(crate) fn evaluates_quietly(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Lit(_) | syn::Expr::Closure(_) | syn::Expr::Path(_) => true,
        syn::Expr::Paren(p) => evaluates_quietly(&p.expr),
        syn::Expr::Group(g) => evaluates_quietly(&g.expr),
        syn::Expr::Reference(r) => evaluates_quietly(&r.expr),
        syn::Expr::Unary(syn::ExprUnary { op: syn::UnOp::Deref(_), expr, .. }) => {
            evaluates_quietly(expr)
        }
        syn::Expr::Field(field) => evaluates_quietly(&field.base),
        syn::Expr::Index(index) => {
            evaluates_quietly(&index.expr) && evaluates_quietly(&index.index)
        }
        other => crate::body::is_place(other),
    }
}

/// Is the text the port wrote for this operand a JavaScript LITERAL — a value
/// built out of nothing, which cannot throw and needs no name of its own?
pub(crate) fn writes_a_literal(text: &str) -> bool {
    let text = text.trim();
    matches!(text, "[]" | "null" | "undefined" | "true" | "false" | "{}")
        || text.parse::<f64>().is_ok()
}

/// Does the text the port wrote for this operand CALL something?
///
/// For: a place the source reads quietly can still be a call in the emitted
/// output — `self.id` on a value behind a `Deref` is `this.deref().id`, and
/// `deref()` on a value somebody has dropped throws — so U3's "the flag stands
/// below everything that can throw" has to see what the port wrote, not only
/// what Rust wrote.
///
/// A closure and a literal are asked of the expression instead: an arrow's
/// parentheses hold its parameters and its body runs when the callee calls it,
/// and a string literal's parentheses are characters.
pub(crate) fn text_calls(expr: &syn::Expr, text: &str) -> bool {
    match expr {
        syn::Expr::Closure(_) | syn::Expr::Lit(_) => false,
        syn::Expr::Paren(p) => text_calls(&p.expr, text),
        syn::Expr::Group(g) => text_calls(&g.expr, text),
        syn::Expr::Reference(r) => text_calls(&r.expr, text),
        _ => text.contains('('),
    }
}
