//! The macros that are neither formatting nor control flow.
//!
//! For: each of these has one meaning, written down here, so that the emitter
//! writes what the macro means instead of leaving the invocation as a comment.
//! None of them is expanded — each hook reads the macro's own arguments and
//! emits what a hand port would write for them.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Expr, Pat, Token};

use crate::body::BodyTranslator;

/// `matches!(subject, pattern)` and `matches!(subject, pattern if guard)`.
///
/// The port's answer is the same test a `match` arm would have written, which
/// is why this reaches for the translator's own pattern test rather than
/// inventing a second one: an enum is tested on its variant tag, an `Option` on
/// `null`, and a literal by equality, in one place.
pub fn matches_macro(tokens: &TokenStream, t: &BodyTranslator) -> Option<String> {
    struct Written {
        subject: Expr,
        pat: Pat,
        guard: Option<Expr>,
    }
    impl syn::parse::Parse for Written {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let subject: Expr = input.parse()?;
            input.parse::<Token![,]>()?;
            let pat = Pat::parse_multi_with_leading_vert(input)?;
            let guard = if input.peek(Token![if]) {
                input.parse::<Token![if]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            Ok(Written { subject, pat, guard })
        }
    }

    let written = syn::parse2::<Written>(tokens.clone()).ok()?;
    let scrutinee_ty = t.borrowed_scrutinee_type(&written.subject);
    let value = t.expr_value(&written.subject);

    // Rust evaluates the subject once and the test may read it several times, so
    // a subject that is not already a name is read into one.
    let simple = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$' || c == '.');

    let subject = if simple { value.clone() } else { t.fresh_temp() };
    let _bindings = t.enter_pattern(&written.pat, scrutinee_ty.as_ref());
    let (test, bind) = t.pattern_test(&subject, &written.pat);
    let guard = written.guard.as_ref().map(|g| t.expr_value(g));
    drop(_bindings);

    if simple && bind.trim().is_empty() && guard.is_none() {
        return Some(test);
    }
    // The bindings the pattern makes are what the guard reads, so they stand
    // between the test and it, inside a function that has somewhere to put them.
    let mut body = format!("if (!({})) return false;\n", test);
    if !bind.trim().is_empty() {
        body.push_str(&bind);
    }
    body.push_str(&format!("return {};\n", guard.as_deref().unwrap_or("true")));
    Some(format!(
        "(({}) => {{\n{}}})({})",
        subject,
        crate::body::indent(&body),
        value
    ))
}

/// `anyhow!(..)` — the error the runtime's `anyhow::Error` stand-in builds.
///
/// anyhow reads a lone argument two ways: a value that is already an error
/// becomes that error, and anything else becomes a message. The engine decides
/// which by the argument's type, and says so where it cannot.
pub fn anyhow_macro(tokens: &TokenStream, t: &BodyTranslator, at: proc_macro2::Span) -> String {
    // A leading string literal makes this the format form, whatever follows it.
    if let Some(written) = super::format_call::written(tokens) {
        let text = super::format_call::format_string(&written, t, at)
            .unwrap_or_else(|| super::format_emit::quoted(&written.fmt.value()));
        return format!("AnyhowError.msg({})", text);
    }
    let Ok(expr) = syn::parse2::<Expr>(tokens.clone()) else {
        t.fallback(
            at,
            "the argument of this `anyhow!` is not an expression the engine could read, so the \
             error carries the tokens as written",
        );
        return format!("AnyhowError.msg({})", super::format_emit::quoted(&tokens.to_string()));
    };
    let value = t.expr_value(&expr);
    match t.resolve_expr_type(&expr) {
        Ok(ty) if is_text(t, &ty) => format!("AnyhowError.msg({})", value),
        Ok(_) => format!("AnyhowError.from({})", value),
        Err(_) => {
            t.fallback(
                at,
                format!(
                    "`anyhow!({})` is handed a value the engine could not type, and anyhow builds \
                     a message from a string and wraps anything else, so it is written as a message",
                    value
                ),
            );
            format!("AnyhowError.msg({})", value)
        }
    }
}

/// Is this a Rust string, which anyhow turns into a message rather than wrapping?
fn is_text(t: &BodyTranslator, ty: &crate::ty::Ty) -> bool {
    match ty.peel_refs() {
        crate::ty::Ty::Str => true,
        crate::ty::Ty::Named { id, .. } => t
            .registry()
            .is_some_and(|reg| reg.name_of(*id).ends_with("::String")),
        _ => false,
    }
}

/// `stringify!(tokens)` — the token text as a string.
pub fn stringify_macro(tokens: &TokenStream) -> String {
    super::format_emit::quoted(&tokens.to_string())
}

/// `hashmap!{ k => v, .. }` — maplit's map literal, which the port writes as the
/// `Map` a `HashMap` becomes.
pub fn hashmap_macro(tokens: &TokenStream, t: &BodyTranslator) -> Option<String> {
    struct Pair {
        key: Expr,
        value: Expr,
    }
    struct Written(Vec<Pair>);
    impl syn::parse::Parse for Written {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let mut pairs = Vec::new();
            while !input.is_empty() {
                let key: Expr = input.parse()?;
                input.parse::<Token![=>]>()?;
                let value: Expr = input.parse()?;
                pairs.push(Pair { key, value });
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            }
            Ok(Written(pairs))
        }
    }
    let Written(pairs) = syn::parse2::<Written>(tokens.clone()).ok()?;
    let entries: Vec<String> = pairs
        .iter()
        .map(|p| format!("[{}, {}]", t.moved_value(&p.key), t.moved_value(&p.value)))
        .collect();
    Some(format!("HashMap.from([{}])", entries.join(", ")))
}

/// `serde_json::json!(expr)` — a JSON value built from one expression.
///
/// The corpus's five uses outside tests all pass a single scalar, which is
/// already a JSON value in the port. The literal form — `json!({ "a": 1 })` — is
/// a token DSL the hook does not read, and it is reported.
pub fn json_macro(tokens: &TokenStream, t: &BodyTranslator, at: proc_macro2::Span) -> String {
    match syn::parse2::<Expr>(tokens.clone()) {
        Ok(expr) => t.moved_value(&expr),
        Err(_) => {
            t.fallback(
                at,
                "this `json!` is written as a JSON literal rather than as one expression, and the \
                 hook reads the expression form only, so the value is `undefined`",
            );
            format!("undefined /* json!({}) */", tokens)
        }
    }
}

/// `cfg!(..)` and `include_str!(..)`: named, so that a use appearing later is a
/// diagnostic rather than a silent comment. Neither is in the corpus today.
pub fn unimplemented_macro(name: &str, tokens: &TokenStream, t: &BodyTranslator, at: proc_macro2::Span) -> String {
    t.fallback(
        at,
        format!(
            "`{}!` has no hook, and what it stands for is decided at compile time in Rust, so the \
             value is `undefined`",
            name
        ),
    );
    format!("undefined /* {}!({}) */", name, tokens.to_token_stream())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stringify_gives_the_token_text() {
        let tokens: TokenStream = syn::parse_str("Vec<u8>").expect("tokens");
        assert_eq!(stringify_macro(&tokens), "'Vec < u8 >'");
    }
}

/// Does this macro's lowering write a run of STATEMENTS rather than one value?
///
/// `assert!(c)` is `if (!(c)) throw new Error(..)` and `bail!(..)` is
/// `return Result.Err(..)`; putting a `return` in front of either does not
/// parse. Answered from the macro the source named, so that the position
/// asking has the lowering's own answer rather than a guess at the first word
/// of the text it produced.
pub(crate) fn writes_statements(path: &syn::Path) -> bool {
    matches!(leaf(path).as_str(), "assert" | "debug_assert" | "bail")
}

/// Does this macro's lowering never come back — a `throw` on every path?
pub(crate) fn never_comes_back(path: &syn::Path) -> bool {
    matches!(leaf(path).as_str(), "panic" | "unreachable" | "todo" | "unimplemented")
}

fn leaf(path: &syn::Path) -> String {
    path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default()
}
