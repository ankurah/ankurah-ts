//! Macro translation — Rust macros → TS expressions

pub mod format_call;
pub mod format_emit;
pub mod format_spec;
pub mod items;
pub mod logging;
mod select;

use crate::body::BodyTranslator;
pub use select::select_futures;
use select::translate_select;

/// Translate a macro invocation to TS.
///
/// The arguments are Rust expressions written in the body the macro sits in, so
/// they are translated through *that* body's translator: a fresh one knows
/// nothing of the closure parameters and inferred types around it, and emitted
/// `${[...].map(..)}` — a spread of a name it could not see — where the source
/// had a closure parameter.
pub fn translate_macro(mac: &syn::Macro, t: &BodyTranslator) -> String {
    let name = mac.path.segments.last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default();
    let at = syn::spanned::Spanned::span(mac);

    // `tracing::warn!` and a bare `warn!` behind a `use tracing::warn` are one
    // macro, so the leaf name decides. Nothing else in the corpus answers to
    // these five names.
    if let Some(level) = logging::level(&name) {
        return logging::call(level, &mac.tokens, t, at);
    }

    match name.as_str() {
        "vec" => {
            // A `Vec<u8>` is a `Uint8Array` in the port, so the position the
            // literal stands in decides which of the two is written.
            let want = t.expectation_at(at);
            // `vec![v; n]` is the repeat form: n copies of one value, not a
            // list. It has no comma to split on, so it used to fall through to
            // "not expressions the engine could read" and the token stream went
            // out verbatim — `[0u8 ; array . length () as usize]`, which is not
            // TypeScript.
            if let Some(written) = repeated(&mac.tokens, t, at, want.as_ref()) {
                return written;
            }
            if let Ok(args) = parse_exprs_from_tokens(&mac.tokens) {
                // Each element is written for the type the SEQUENCE holds.
                // `let values: Vec<Expr> = vec![true.into(), 95.5.into()]` says
                // what each `.into()` converts to, and only the annotation says
                // it: read without the element type, every one of them had "no
                // expected type here" and stood as the value it was, so the
                // vector held a boolean and a number where `Expr`s were
                // declared (`ankql`'s `test_populate_mixed_types`).
                let element = want.as_ref().and_then(|want| t.element_of(want));
                let translated: Vec<String> = args
                    .iter()
                    .map(|e| t.expecting(e, element.as_ref(), || t.moved_value(e)))
                    .collect();
                t.sequence_literal(translated, want.as_ref())
            } else {
                t.fallback(
                    at,
                    "the contents of this `vec!` are not expressions the engine could read, \
                     so they are written through unchanged",
                );
                format!("[{}]", mac.tokens)
            }
        }
        // `rusqlite::params![a, b, c]` is a parameter list, and the provided
        // driver takes one as an array. The macro builds a `[&dyn ToSql; N]`,
        // which erases to exactly the values it was handed.
        "params" => match parse_exprs_from_tokens(&mac.tokens) {
            Ok(args) => {
                let written: Vec<String> = args.iter().map(|e| t.moved_value(e)).collect();
                format!("[{}]", written.join(", "))
            }
            Err(_) => {
                t.fallback(
                    at,
                    "the contents of this `params!` are not expressions the engine could read",
                );
                format!("[{}]", mac.tokens)
            }
        },
        "format" => formatted(&mac.tokens, t, at, &name),
        "println" | "eprintln" | "print" | "eprint" => {
            format!("console.log({})", formatted(&mac.tokens, t, at, &name))
        }
        "dbg" => format!("console.log({})", mac.tokens),
        // `write!(f, "..")` puts its text into the formatter the caller handed
        // it, and the port's `Display` returns that text, so the formatter is
        // dropped and the text is the value.
        "write" | "writeln" => written_text(&name, &mac.tokens, t, at),
        "panic" | "unreachable" => {
            format!("throw new Error({})", formatted(&mac.tokens, t, at, &name))
        }
        // The condition is Rust, so it is translated as Rust. Printing the
        // token stream put the source back out verbatim — `event_ids . contains
        // (& 7)` — which is neither TypeScript nor what the assertion meant.
        "assert" | "debug_assert" => match parse_exprs_from_tokens(&mac.tokens) {
            Ok(args) if !args.is_empty() => {
                let condition = t.expr_value(&args[0]);
                // `assert!(c, "..", x)` carries its own message; without one the
                // failure says only that the assertion failed, as Rust's does.
                let message = if args.len() > 1 {
                    let tail = args[1..].iter().map(|e| quote::quote!(#e));
                    formatted(&quote::quote!(#(#tail),*), t, at, &name)
                } else {
                    "'assertion failed'".to_string()
                };
                format!("if (!({})) throw new Error({})", condition, message)
            }
            _ => {
                t.fallback(
                    at,
                    format!(
                        "the condition of this `{}!` is not an expression the engine could read, \
                         so the assertion is emitted as a comment and never runs",
                        name
                    ),
                );
                format!("undefined /* {}!({}) */", name, mac.tokens)
            }
        },
        // Rust compares two values of one type, so whichever side the engine
        // can type says what the other one is. That is what writes
        // `assert_eq!(bytes, [1, 2, 3])` as a `Uint8Array` on both sides
        // instead of a `Uint8Array` against a JavaScript array.
        "assert_eq" => compare(mac, t, "assert_eq", "expect({}).toEqual({})"),
        "assert_ne" => compare(mac, t, "assert_ne", "expect({}).not.toEqual({})"),
        // A macro cannot be a function, so the arms stay with the emitter and
        // only the arbitration goes to the runtime.
        "select" => translate_select(&mac.tokens, t),
        "todo" => "throw new Error('TODO')".to_string(),
        "unimplemented" => "throw new Error('unimplemented')".to_string(),
        "matches" => items::matches_macro(&mac.tokens, t).unwrap_or_else(|| {
            t.fallback(
                at,
                "the pattern of this `matches!` is not one the engine could read, so the test is \
                 written as `false`",
            );
            format!("false /* matches!({}) */", mac.tokens)
        }),
        "anyhow" => items::anyhow_macro(&mac.tokens, t, at),
        // `bail!(..)` is `return Err(anyhow!(..))`. The corpus writes none, so
        // nothing here exercises it; the hook exists because the alternative to
        // a wrong lowering is no lowering at all, and this one is anyhow's own.
        "bail" => format!(
            "return Result.Err({})",
            items::anyhow_macro(&mac.tokens, t, at)
        ),
        "stringify" => items::stringify_macro(&mac.tokens),
        "hashmap" => items::hashmap_macro(&mac.tokens, t).unwrap_or_else(|| {
            t.fallback(
                at,
                "the entries of this `hashmap!` are not `key => value` pairs the engine could \
                 read, so the map is empty",
            );
            format!("new HashMap() /* hashmap!({}) */", mac.tokens)
        }),
        "json" => items::json_macro(&mac.tokens, t, at),
        "cfg" | "include_str" | "include_bytes" | "env" | "option_env" | "concat" => {
            items::unimplemented_macro(&name, &mac.tokens, t, at)
        }
        // A macro with no hook is emitted as a comment, so everything it was
        // going to do does not happen. That is a gap in the port whether or not
        // it was handed a value to release, which is why this reports at every
        // one and not only where something leaks.
        _ => {
            t.fallback(
                at,
                format!(
                    "`{}!` has no hook, so it is emitted as a comment and what it stood for does \
                     not run",
                    name
                ),
            );
            t.report_unsupported_macro(mac, &name);
            format!("undefined /* {}!({}) */", name, mac.tokens)
        }
    }
}

/// A formatting macro's text, or its format string alone where the arguments
/// could not be read.
fn formatted(
    tokens: &TokenStream,
    t: &BodyTranslator,
    at: proc_macro2::Span,
    name: &str,
) -> String {
    let Some(written) = format_call::written(tokens) else {
        t.fallback(
            at,
            format!(
                "this `{}!` does not begin with a format string the engine could read, so its \
                 text is the tokens as written",
                name
            ),
        );
        return format_emit::quoted(&tokens.to_string());
    };
    format_call::format_string(&written, t, at)
        .unwrap_or_else(|| format_emit::quoted(&written.fmt.value()))
}

/// `write!(f, "..", ..)` and `writeln!`: the formatter the first argument names
/// is what the port's `toString` returns, so it is dropped and the text stands.
fn written_text(
    name: &str,
    tokens: &TokenStream,
    t: &BodyTranslator,
    at: proc_macro2::Span,
) -> String {
    struct Rest {
        rest: TokenStream,
    }
    impl Parse for Rest {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let _formatter: Expr = input.parse()?;
            input.parse::<Token![,]>()?;
            Ok(Rest { rest: input.parse()? })
        }
    }
    let Ok(Rest { rest }) = syn::parse2::<Rest>(tokens.clone()) else {
        t.fallback(
            at,
            format!(
                "this `{}!` is not a formatter followed by a format string, so its text is the \
                 tokens as written",
                name
            ),
        );
        return format_emit::quoted(&tokens.to_string());
    };
    let text = formatted(&rest, t, at, name);
    if name == "writeln" {
        // `writeln!` is `write!` with a newline after it.
        return format!("{} + '\\n'", text);
    }
    text
}

// ── Token-based macro parsing (avoids string round-trip) ────────────

use syn::parse::{Parse, ParseStream};
use syn::{Expr, Token};
use proc_macro2::TokenStream;

/// Parse comma-separated expressions from a TokenStream
/// The two sides of an equality assertion, each typed by the other.
///
/// Rust already knows the two are the same type; the engine has to be told
/// which one, and it takes whichever side it can read on its own. The written
/// form takes two placeholders, the left side then the right.
fn compare(mac: &syn::Macro, t: &BodyTranslator, name: &str, written: &str) -> String {
    let Ok(args) = parse_exprs_from_tokens(&mac.tokens) else {
        t.fallback(
            syn::spanned::Spanned::span(mac),
            format!(
                "the operands of this `{}!` are not expressions the engine could read, so the \
                 assertion is emitted as a comment",
                name
            ),
        );
        return format!("undefined /* {}!({}) */", name, mac.tokens);
    };
    if args.len() < 2 {
        t.fallback(
            syn::spanned::Spanned::span(mac),
            format!(
                "`{}!` is written with fewer than two operands, so there is nothing to \
                 compare and the assertion is emitted as a comment",
                name
            ),
        );
        return format!("undefined /* {}!({}) */", name, mac.tokens);
    }
    let left_ty = t.quietly(|| t.resolve_expr_type(&args[0])).ok();
    let right_ty = t.quietly(|| t.resolve_expr_type(&args[1])).ok();
    // An operand that builds a value builds it for the length of the assertion
    // and no longer: Rust drops what a statement produced at the statement's
    // end, and `assert_eq!(id, T::from_json(&s).unwrap())` produces one.
    let left = operand(t, &args[0], right_ty.as_ref());
    let right = operand(t, &args[1], left_ty.as_ref());
    written.replacen("{}", &left, 1).replacen("{}", &right, 1)
}

/// One side of an equality assertion: translated with the other side's type
/// wanted of it, and released at the end of the assertion where it built a
/// value of its own.
///
/// Rust drops what a statement produced when the statement ends, and an
/// assertion is a statement: `assert_eq!(id, T::from_json(&s).unwrap())` builds
/// a second `T` that nothing else ever holds.
fn operand(t: &BodyTranslator, expr: &Expr, want: Option<&crate::ty::Ty>) -> String {
    t.expecting(expr, want, || {
        let written = t.expr_value(expr);
        if crate::body::is_place(expr) {
            return written;
        }
        let Some(tc) = &t.types else { return written };
        let Ok(ty) = t.quietly(|| t.resolve_expr_type(expr)) else {
            return written;
        };
        let drops = crate::ownership::drops_of(&tc.borrow().probe(), &ty);
        if drops.is_droppable() {
            t.hoist_temporary(written, drops)
        } else {
            written
        }
    })
}

/// The elements a `vec![..]` holds, for the engine to type them.
///
/// `vec![a; n]` repeats one element, and `vec![a, b]` lists them; either way
/// the first expression is the one whose type the whole `Vec` takes. Tokens
/// that do not parse as expressions hand back nothing, and the caller says so
/// rather than guessing.
pub fn vec_macro_elements(mac: &syn::Macro) -> Vec<Expr> {
    if let Ok(repeat) = syn::parse2::<syn::ExprRepeat>(mac.tokens.clone()) {
        return vec![(*repeat.expr).clone()];
    }
    parse_exprs_from_tokens(&mac.tokens).unwrap_or_default()
}

fn parse_exprs_from_tokens(tokens: &TokenStream) -> Result<Vec<Expr>, syn::Error> {
    struct ExprList(Vec<Expr>);
    impl Parse for ExprList {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut exprs = Vec::new();
            while !input.is_empty() {
                exprs.push(input.parse()?);
                if input.peek(Token![,]) { input.parse::<Token![,]>()?; }
            }
            Ok(ExprList(exprs))
        }
    }
    syn::parse2::<ExprList>(tokens.clone()).map(|el| el.0)
}

/// The text with exactly one newline at its end, so a block body that already
/// ends in one does not open a blank line inside the arm.
fn ensure_newline(text: &str) -> String {
    format!("{}\n", text.trim_end())
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    /// Two channels to race and a counter to write into, which is the smallest
    /// crate a `select!` can be written against.
    const PRELUDE: &str = "\
use tokio::sync::mpsc;\n\
";

    fn body(rust: &str, method: &str) -> String {
        let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", PRELUDE, rust))]);
        fixture.translated_method("lib.rs", method)
    }

    fn diagnostics(rust: &str, method: &str) -> Vec<String> {
        let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", PRELUDE, rust))]);
        fixture.translated_method("lib.rs", method);
        fixture.messages()
    }

    const BOUND: &str = "\
pub async fn f(mut left: mpsc::Receiver<u32>, mut right: mpsc::Receiver<u32>) -> u32 {\n\
    let value = tokio::select! {\n\
        _ = left.recv() => 1,\n\
        _ = right.recv() => 2,\n\
    };\n\
    value\n\
}\n";

    const BREAKS: &str = "\
pub async fn f(mut left: mpsc::Receiver<u32>, mut right: mpsc::Receiver<u32>) -> u32 {\n\
    let mut seen = 0u32;\n\
    loop {\n\
        tokio::select! {\n\
            _ = left.recv() => { break; }\n\
            _ = right.recv() => { seen += 1; }\n\
        }\n\
    }\n\
    seen\n\
}\n";

    #[test]
    fn a_select_that_something_binds_is_written_as_one_expression() {
        let ts = body(BOUND, "f");
        assert!(
            ts.contains("const value = await (async () => {"),
            "the select has to produce the value the `let` binds, and a run of statements \
             cannot stand where an initialiser goes:\n{}",
            ts
        );
        assert!(
            ts.contains("return 1;") && ts.contains("return 2;"),
            "each arm has to hand its value back out of the arrow function:\n{}",
            ts
        );
    }

    #[test]
    fn every_branch_future_is_released_however_the_select_is_left() {
        let ts = body(BOUND, "f");
        let released = ts
            .lines()
            .find(|line| line.contains("dropOwned"))
            .unwrap_or_else(|| panic!("no branch release in:\n{}", ts));
        assert!(
            released.contains(".promise"),
            "the release is of the branch futures:\n{}",
            ts
        );
        let raced = ts.find("await select(").unwrap_or_else(|| panic!("nothing races in:\n{}", ts));
        let at = ts.find("dropOwned").unwrap();
        assert!(
            ts[raced..at].contains("} finally {"),
            "the release has to be in the `finally` of the `try` the arms run in, so a losing \
             branch is cancelled whether the select returned or threw:\n{}",
            ts
        );
    }

    #[test]
    fn an_arm_that_breaks_the_loop_keeps_the_statement_form() {
        let ts = body(BREAKS, "f");
        assert!(
            !ts.contains("async () =>"),
            "a `break` inside an arrow function leaves the arrow function, not the loop the \
             source wrote it against:\n{}",
            ts
        );
        assert!(
            ts.contains("break;"),
            "the break has to reach the emitted loop:\n{}",
            ts
        );
    }

    #[test]
    fn taking_the_statement_form_is_reported_rather_than_taken_in_silence() {
        let said = diagnostics(BREAKS, "f");
        assert!(
            said.iter().any(|m| m.contains("`select!`") && m.contains("breaks out of the loop")),
            "the two forms are not the same lowering, so choosing the one that produces no \
             value has to be recorded: {:?}",
            said
        );
    }

    #[test]
    fn a_select_that_produces_a_value_reports_nothing_of_its_own() {
        let said = diagnostics(BOUND, "f");
        assert!(
            !said.iter().any(|m| m.contains("`select!` is lowered to the runtime's arbiter")),
            "this select is carried whole, so nothing about its lowering is given up: {:?}",
            said
        );
    }
}

/// `vec![value; count]` — the repeat form.
///
/// `Array(n).fill(v)` puts the same value in every slot, which is what Rust
/// does for a `Copy` element and is not what it does for anything else: Rust
/// clones. A non-primitive element is therefore reported rather than written,
/// because sharing one object across every slot is a different program.
fn repeated(
    tokens: &proc_macro2::TokenStream,
    t: &crate::body::BodyTranslator,
    at: proc_macro2::Span,
    want: Option<&crate::ty::Ty>,
) -> Option<String> {
    let text = tokens.to_string();
    let (value, count) = text.split_once(';')?;
    let value: syn::Expr = syn::parse_str(value.trim()).ok()?;
    let count: syn::Expr = syn::parse_str(count.trim()).ok()?;
    let element = t.moved_value(&value);
    let n = t.expr(&count);
    let bytes = matches!(&value, syn::Expr::Lit(l) if matches!(&l.lit, syn::Lit::Int(_)))
        && want.is_some_and(|w| t.expects_bytes(w));
    if bytes {
        // A zero-filled `Uint8Array` is what `new Uint8Array(n)` already is.
        return Some(if element == "0" {
            format!("new Uint8Array({})", n)
        } else {
            format!("new Uint8Array({}).fill({})", n, element)
        });
    }
    if !matches!(&value, syn::Expr::Lit(_) | syn::Expr::Path(_)) {
        t.fallback(
            at,
            "this `vec![v; n]` repeats a value the port would share rather than clone, \
             so the copies would be one object",
        );
    }
    Some(format!("Array({}).fill({})", n, element))
}
