//! Option<T> → T | null method translations
//!
//! Option maps to nullable (T | null) in TS, so "methods" on Option
//! become syntax-level operations, not method calls.
//!
//! Each of these is written as the test it is, and a test reads the value it
//! tests and then reads it AGAIN to hand it on. Rust reads it once. So the
//! receiver is given one name before the test — `Once::bind_receiver` — and the
//! two reads are reads of that name. Without it
//! `subscriptions.remove(id).ok_or(..)` removed the entry, threw it away,
//! removed nothing the second time and answered `Ok(null)`.
//!
//! And each of these is an expression with a ternary or a binary operator at its
//! top, so each is parenthesised: `map_or(0, f) == 0` came out
//! `x != null ? f(x) : 0 === 0`, which JavaScript reads as `x != null ? f(x) :
//! (0 === 0)` — the comparison swallowed by the false branch.

use super::MethodTranslation;

/// One name for a value one of these translations reads twice, or reads out of
/// the order Rust evaluates it in.
///
/// For: only the body translator knows where a declaration can stand, and
/// whether the expression is a PLACE — a name or a field, which reading twice
/// costs nothing and which Rust reads once for that same reason. So the naming
/// is the caller's and the decision to ask for it is here.
pub struct Once<'a> {
    /// Give the receiver a name that stands before the statement it is written
    /// in, and answer that name. A place comes back unchanged.
    pub bind_receiver: &'a dyn Fn(&str) -> String,
    /// The same for an argument Rust evaluates BEFORE it branches — `ok_or`'s
    /// error and `map_or`'s default, which are values and not closures. Nothing
    /// comes back where naming it would change what the program owns: Rust
    /// builds such a value and drops it on the path that does not use it, and
    /// there is no name here to drop it under, so the argument stays in the
    /// branch and the caller reports the difference.
    pub bind_eager: &'a dyn Fn(usize, &str) -> Option<String>,
}

impl Once<'_> {
    /// The `Once` for a caller that has no expressions to ask about — the
    /// ownership pass, which asks only WHETHER a call has a translation.
    pub fn unasked() -> Once<'static> {
        Once { bind_receiver: &|written| written.to_string(), bind_eager: &|_, written| Some(written.to_string()) }
    }
}

pub fn translate(receiver: &str, method: &str, args: &[String], once: &Once<'_>) -> MethodTranslation {
    // The receiver of every form below is read at least twice — the test, and
    // the hand-on — so it is named once here rather than per arm.
    let reads_twice = matches!(
        method,
        "map" | "and_then" | "filter" | "is_some_and" | "ok_or" | "ok_or_else" | "map_or" | "map_or_else"
    );
    let subject = if reads_twice { (once.bind_receiver)(receiver) } else { receiver.to_string() };
    let subject = subject.as_str();
    let result = match method {
        // unwrap/expect/unwrap_or/unwrap_or_else handled in body.rs before dispatch.

        // Null checks
        "is_some" => format!("({} != null)", subject),
        "is_none" => format!("({} == null)", subject),

        // Map — apply function if non-null
        "map" if args.len() == 1 => format!("({} != null ? ({})({}!) : null)", subject, args[0], subject),

        // A JavaScript value is neither borrowed nor owned, so the four
        // `as_` conversions between those states are the value itself. Written
        // through, they named methods no value has: 39 `asRef` and 11 `asMut`
        // calls across the emitted corpus, each a `TypeError`.
        "as_ref" | "as_mut" | "as_deref" | "as_deref_mut" if args.is_empty() => receiver.to_string(),

        // The combinators, each written as the test it is.
        "and_then" if args.len() == 1 => {
            format!("({} != null ? ({})({}!) : null)", subject, args[0], subject)
        }
        "filter" if args.len() == 1 => {
            format!("({} != null && ({})({}!) ? {} : null)", subject, args[0], subject, subject)
        }
        "is_some_and" if args.len() == 1 => {
            format!("({} != null && ({})({}!))", subject, args[0], subject)
        }
        "ok_or" if args.len() == 1 => {
            let error = (once.bind_eager)(0, &args[0]).unwrap_or_else(|| args[0].clone());
            format!("({} != null ? Result.Ok({}!) : Result.Err({}))", subject, subject, error)
        }
        "ok_or_else" if args.len() == 1 => {
            format!("({} != null ? Result.Ok({}!) : Result.Err(({})()))", subject, subject, args[0])
        }
        "map_or" if args.len() == 2 => {
            let default = (once.bind_eager)(0, &args[0]).unwrap_or_else(|| args[0].clone());
            format!("({} != null ? ({})({}!) : {})", subject, args[1], subject, default)
        }
        "map_or_else" if args.len() == 2 => {
            format!("({} != null ? ({})({}!) : ({})())", subject, args[1], subject, args[0])
        }

        // `Option<&T>::cloned` and `::copied` turn a borrow of the payload into
        // an owned one. A JavaScript value is neither, so the nullable is
        // already what they produce. (What `cloned` does to the payload itself
        // — an `Arc` refcount, say — is the same question `iter().cloned()`
        // raises and is answered the same way: it does not.)
        "cloned" | "copied" if args.is_empty() => receiver.to_string(),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A caller that names every receiver `_v0` and every eager argument `_e0`,
    /// so the tests read the shape rather than the numbering.
    fn named() -> Once<'static> {
        Once {
            bind_receiver: &|written| {
                if written.contains('(') { "_v0".to_string() } else { written.to_string() }
            },
            bind_eager: &|_, written| {
                Some(if written.contains('(') { "_e0".to_string() } else { written.to_string() })
            },
        }
    }

    fn expr(receiver: &str, method: &str, args: &[&str]) -> String {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        match translate(receiver, method, &args, &named()) {
            MethodTranslation::Expr(ts) => ts,
            _ => panic!("`{method}` has no expression translation"),
        }
    }

    /// A JavaScript value is neither borrowed nor owned, so the conversions
    /// between those states are the value. Written through, `as_ref` named a
    /// method no value has: 39 `asRef` and 11 `asMut` calls across the emitted
    /// corpus, each a `TypeError` the moment it ran.
    #[test]
    fn the_borrow_conversions_are_the_value() {
        for method in ["as_ref", "as_mut", "as_deref", "as_deref_mut", "cloned", "copied"] {
            assert_eq!(expr("o", method, &[]), "o");
        }
    }

    /// PREMISE CHANGED 2026-09-05 (fixpass4 item 2): the tests this replaces
    /// pinned the receiver written TWICE as the expected text — which is what
    /// made `subscriptions.remove(id).ok_or(..)` remove the entry, discard it,
    /// remove nothing the second time and answer `Ok(null)`, with the removed
    /// entry leaked. Rust reads the receiver once.
    #[test]
    fn a_receiver_that_is_not_a_place_is_read_once() {
        assert_eq!(
            expr("take(id)", "ok_or", &["e"]),
            "(_v0 != null ? Result.Ok(_v0!) : Result.Err(e))"
        );
        assert_eq!(
            expr("take(id)", "map", &["f"]),
            "(_v0 != null ? (f)(_v0!) : null)"
        );
        assert_eq!(
            expr("take(id)", "filter", &["p"]),
            "(_v0 != null && (p)(_v0!) ? _v0 : null)"
        );
    }

    /// A place is read where it stands: reading a name or a field twice has no
    /// effect, which is the same reason Rust may read it once.
    #[test]
    fn a_place_is_read_where_it_stands() {
        assert_eq!(expr("this.limit", "map", &["f"]), "(this.limit != null ? (f)(this.limit!) : null)");
    }

    /// Each of these has a ternary or a binary operator at its top, so each is
    /// parenthesised: `map_or(0, f) == 0` came out `x != null ? f(x) : 0 === 0`,
    /// which JavaScript reads as `x != null ? f(x) : (0 === 0)` — so the
    /// comparison was swallowed by the false branch and the whole thing answered
    /// `true` for every `None`.
    #[test]
    fn every_form_is_parenthesised() {
        for written in [
            expr("o", "is_some", &[]),
            expr("o", "is_none", &[]),
            expr("o", "map", &["f"]),
            expr("o", "and_then", &["f"]),
            expr("o", "filter", &["p"]),
            expr("o", "is_some_and", &["p"]),
            expr("o", "ok_or", &["e"]),
            expr("o", "ok_or_else", &["f"]),
            expr("o", "map_or", &["0", "f"]),
            expr("o", "map_or_else", &["d", "f"]),
        ] {
            assert!(written.starts_with('('), "{}", written);
            assert!(written.ends_with(')'), "{}", written);
        }
    }

    /// `ok_or` and `map_or` take VALUES, which Rust evaluates before it
    /// branches; only the `_else` forms take closures, which it calls inside the
    /// branch. An argument the caller can name is named there.
    #[test]
    fn an_eager_argument_is_evaluated_before_the_branch() {
        assert_eq!(
            expr("o", "ok_or", &["build()"]),
            "(o != null ? Result.Ok(o!) : Result.Err(_e0))"
        );
        assert_eq!(
            expr("o", "map_or", &["build()", "f"]),
            "(o != null ? (f)(o!) : _e0)"
        );
        // The lazy pair keep their closures where the branch calls them.
        assert_eq!(
            expr("o", "ok_or_else", &["build"]),
            "(o != null ? Result.Ok(o!) : Result.Err((build)()))"
        );
        assert_eq!(
            expr("o", "map_or_else", &["d", "f"]),
            "(o != null ? (f)(o!) : (d)())"
        );
    }
}
