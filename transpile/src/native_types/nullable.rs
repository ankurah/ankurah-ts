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
    /// error and `map_or`'s default, which are values and not closures. The
    /// answer carries the release the OTHER branch owes, because Rust builds
    /// such a value on both paths and drops it on the path that hands it
    /// nowhere. Nothing comes back where the engine could not name what the
    /// argument is: no release can be written for a type nobody could name, so
    /// the argument stays inside the branch and the caller reports it.
    pub bind_eager: &'a dyn Fn(usize, &str) -> Option<Eager>,
    /// The same for a CLOSURE argument the port hands over as a value.
    ///
    /// Rust builds the closure before it enters the method and drops it on the
    /// path that does not call it, and a `move` closure over something
    /// droppable is an `OwnedClosure`, which R10 says is invoked through one
    /// base helper and never called as a function. So it is named once, called
    /// through `invoke`, and released by the branch that does not call it.
    /// Nothing comes back for an ordinary arrow, which owns nothing and which
    /// the port calls where it stands.
    pub bind_closure: &'a dyn Fn(usize, &str) -> Option<Eager>,
}

/// An eager argument the caller has named, and what the branch that hands it
/// nowhere owes it.
///
/// For: `o.ok_or(build())` builds the error in Rust whether or not `o` is
/// `None`, and drops it again where `o` was `Some`. Writing the argument inside
/// the `Err` branch restored neither the evaluation nor the drop; writing it
/// before the branch and releasing it in the other one restores both.
pub struct Eager {
    /// What to write where the argument stood — a hoisted name, or the
    /// argument itself where reading it twice runs nothing.
    pub name: String,
    /// The release the branch that does not hand the value on runs, where the
    /// value owns something. `None` where it owns nothing.
    pub release: Option<String>,
}

impl Once<'_> {
    /// The `Once` for a caller that has no expressions to ask about — the
    /// ownership pass, which asks only WHETHER a call has a translation.
    pub fn unasked() -> Once<'static> {
        Once {
            bind_receiver: &|written| written.to_string(),
            bind_eager: &|_, written| Some(Eager { name: written.to_string(), release: None }),
            bind_closure: &|_, _| None,
        }
    }
}

/// The used branch of a combinator whose eager argument owns something: the
/// release first, then the value the branch hands on.
///
/// The release stands FIRST because Rust drops the argument whether the branch
/// returns or panics, and a release written before the branch's own work runs
/// on both of those paths without a `try`. Nothing the branch does can observe
/// the difference: the argument was moved into the call, so no other name
/// reaches it.
fn releasing(release: &Option<String>, value: String) -> String {
    match release {
        Some(release) => format!("({}, {})", release, value),
        None => value,
    }
}

/// A closure argument, as the port calls it and as the other branch releases it.
struct Callable {
    /// The call, with the argument list already in it.
    call: String,
    /// What the branch that does NOT call it owes.
    release: Option<String>,
}

/// How to call a combinator's closure argument, and what the other branch owes.
///
/// Every one of these takes its closure by value (`FnOnce`), so the call
/// consumes it: `invoke` is the helper for that, and it is what R10 says an
/// `OwnedClosure` is always reached through — calling one as a function is a
/// `TypeError`, and the emitted code did exactly that.
fn callable(once: &Once<'_>, at: usize, written: &str, args: &str) -> Callable {
    match (once.bind_closure)(at, written) {
        Some(bound) => Callable {
            call: match args.is_empty() {
                true => format!("invoke({})", bound.name),
                false => format!("invoke({}, {})", bound.name, args),
            },
            release: bound.release,
        },
        None => Callable { call: format!("({})({})", written, args), release: None },
    }
}

/// Which of these read the receiver more than once — the test, and the hand-on.
///
/// Asked from two places: here, to name it once; and by the body translator,
/// which has to take that name BEFORE it translates the arguments, because Rust
/// evaluates the receiver first and an argument that hoists something of its own
/// would otherwise land ahead of it.
pub fn reads_receiver_twice(method: &str) -> bool {
    matches!(
        method,
        "map" | "and_then" | "filter" | "is_some_and" | "ok_or" | "ok_or_else" | "map_or" | "map_or_else"
    )
}

pub fn translate(receiver: &str, method: &str, args: &[String], once: &Once<'_>) -> MethodTranslation {
    // The receiver of every form below is read at least twice — the test, and
    // the hand-on — so it is named once here rather than per arm.
    let subject =
        if reads_receiver_twice(method) { (once.bind_receiver)(receiver) } else { receiver.to_string() };
    let subject = subject.as_str();
    let result = match method {
        // unwrap/expect/unwrap_or/unwrap_or_else handled in body.rs before dispatch.

        // Null checks
        "is_some" => format!("({} != null)", subject),
        "is_none" => format!("({} == null)", subject),

        // Map — apply function if non-null
        "map" if args.len() == 1 => {
            let f = callable(once, 0, &args[0], &format!("{}!", subject));
            format!("({} != null ? {} : {})", subject, f.call, releasing(&f.release, "null".into()))
        }

        // A JavaScript value is neither borrowed nor owned, so the four
        // `as_` conversions between those states are the value itself. Written
        // through, they named methods no value has: 39 `asRef` and 11 `asMut`
        // calls across the emitted corpus, each a `TypeError`.
        "as_ref" | "as_mut" | "as_deref" | "as_deref_mut" if args.is_empty() => receiver.to_string(),

        // The combinators, each written as the test it is.
        "and_then" if args.len() == 1 => {
            let f = callable(once, 0, &args[0], &format!("{}!", subject));
            format!("({} != null ? {} : {})", subject, f.call, releasing(&f.release, "null".into()))
        }
        "filter" if args.len() == 1 => {
            let p = callable(once, 0, &args[0], &format!("{}!", subject));
            match &p.release {
                // The predicate is built whether or not the receiver is `Some`,
                // and dropped where it is not — so the `&&` form, which does not
                // evaluate its right-hand side at all, has nowhere to put the
                // release and the branch is written out.
                Some(_) => format!(
                    "({} != null ? ({} ? {} : null) : {})",
                    subject,
                    p.call,
                    subject,
                    releasing(&p.release, "null".into())
                ),
                None => format!("({} != null && {} ? {} : null)", subject, p.call, subject),
            }
        }
        "is_some_and" if args.len() == 1 => {
            let f = callable(once, 0, &args[0], &format!("{}!", subject));
            match &f.release {
                Some(_) => format!(
                    "({} != null ? {} : {})",
                    subject,
                    f.call,
                    releasing(&f.release, "false".into())
                ),
                None => format!("({} != null && {})", subject, f.call),
            }
        }
        "ok_or" if args.len() == 1 => {
            let eager = (once.bind_eager)(0, &args[0]);
            let (error, release) = match eager {
                Some(eager) => (eager.name, eager.release),
                None => (args[0].clone(), None),
            };
            let ok = releasing(&release, format!("Result.Ok({}!)", subject));
            format!("({} != null ? {} : Result.Err({}))", subject, ok, error)
        }
        "ok_or_else" if args.len() == 1 => {
            let f = callable(once, 0, &args[0], "");
            let ok = releasing(&f.release, format!("Result.Ok({}!)", subject));
            format!("({} != null ? {} : Result.Err({}))", subject, ok, f.call)
        }
        "map_or" if args.len() == 2 => {
            let eager = (once.bind_eager)(0, &args[0]);
            let (default, release) = match eager {
                Some(eager) => (eager.name, eager.release),
                None => (args[0].clone(), None),
            };
            let f = callable(once, 1, &args[1], &format!("{}!", subject));
            let mapped = releasing(&release, f.call);
            format!("({} != null ? {} : {})", subject, mapped, releasing(&f.release, default))
        }
        "map_or_else" if args.len() == 2 => {
            // Rust evaluates the default closure first — it is the first
            // argument — and builds both before it branches.
            let default = callable(once, 0, &args[0], "");
            let f = callable(once, 1, &args[1], &format!("{}!", subject));
            format!(
                "({} != null ? {} : {})",
                subject,
                releasing(&default.release, f.call),
                releasing(&f.release, default.call)
            )
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
    /// so the tests read the shape rather than the numbering. An eager argument
    /// that owns something is written `owned(..)` here and comes back with the
    /// release the other branch owes.
    fn named() -> Once<'static> {
        Once {
            bind_receiver: &|written| {
                if written.contains('(') { "_v0".to_string() } else { written.to_string() }
            },
            bind_eager: &|_, written| {
                let owns = written.starts_with("owned");
                let name =
                    if written.contains('(') { "_e0".to_string() } else { written.to_string() };
                let release = owns.then(|| format!("{}.drop()", name));
                Some(Eager { name, release })
            },
            // A closure written `owned(..)` in these tests is one the port
            // wraps: named once, invoked through `invoke`, released by the
            // branch that does not invoke it.
            bind_closure: &|_, written| {
                written.starts_with("owned").then(|| Eager {
                    name: "_f0".to_string(),
                    release: Some("dropOwned(_f0)".to_string()),
                })
            },
        }
    }

    /// A caller that could not name what the argument is, so no release can be
    /// written and the argument stays inside its branch.
    fn unnamed() -> Once<'static> {
        Once {
            bind_receiver: &|written| {
                if written.contains('(') { "_v0".to_string() } else { written.to_string() }
            },
            bind_eager: &|_, _| None,
            bind_closure: &|_, _| None,
        }
    }

    fn expr_with(once: &Once<'_>, receiver: &str, method: &str, args: &[&str]) -> String {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        match translate(receiver, method, &args, once) {
            MethodTranslation::Expr(ts) => ts,
            _ => panic!("`{method}` has no expression translation"),
        }
    }

    fn expr(receiver: &str, method: &str, args: &[&str]) -> String {
        expr_with(&named(), receiver, method, args)
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

    /// PREMISE CHANGED 2026-09-05 (fixpass5 item 1): fixpass4's §3.3 left an
    /// eager argument that OWNS something inside its branch, because naming it
    /// restored the evaluation and not the drop. The release is written now:
    /// the branch that hands the value nowhere runs it, which is where Rust
    /// drops it.
    #[test]
    fn an_eager_argument_that_owns_something_is_released_on_the_other_path() {
        assert_eq!(
            expr("o", "ok_or", &["owned()"]),
            "(o != null ? (_e0.drop(), Result.Ok(o!)) : Result.Err(_e0))"
        );
        assert_eq!(
            expr("o", "map_or", &["owned()", "f"]),
            "(o != null ? (_e0.drop(), (f)(o!)) : _e0)"
        );
        // A place is read where it stands, and the move is still a move: the
        // release is written for it too.
        assert_eq!(
            expr("o", "ok_or", &["ownedLocal"]),
            "(o != null ? (ownedLocal.drop(), Result.Ok(o!)) : Result.Err(ownedLocal))"
        );
    }

    /// A caller with nothing to say about the argument leaves it where it
    /// stood: the combinator writes no release it was not handed one for.
    #[test]
    fn an_argument_the_caller_does_not_name_stays_in_its_branch() {
        assert_eq!(
            expr_with(&unnamed(), "o", "ok_or", &["build()"]),
            "(o != null ? Result.Ok(o!) : Result.Err(build()))"
        );
        assert_eq!(
            expr_with(&unnamed(), "o", "map_or", &["build()", "f"]),
            "(o != null ? (f)(o!) : build())"
        );
    }

    /// Y4: a closure the port wraps is invoked through `invoke` — an
    /// `OwnedClosure` called as a function is a `TypeError` — and released by
    /// the branch that does not invoke it, because Rust builds it before it
    /// enters the method and drops it on the path that does not call it.
    #[test]
    fn a_wrapped_closure_is_invoked_and_released_by_the_other_branch() {
        assert_eq!(
            expr("o", "map", &["owned(f)"]),
            "(o != null ? invoke(_f0, o!) : (dropOwned(_f0), null))"
        );
        assert_eq!(
            expr("o", "and_then", &["owned(f)"]),
            "(o != null ? invoke(_f0, o!) : (dropOwned(_f0), null))"
        );
        assert_eq!(
            expr("o", "is_some_and", &["owned(f)"]),
            "(o != null ? invoke(_f0, o!) : (dropOwned(_f0), false))"
        );
        assert_eq!(
            expr("o", "filter", &["owned(p)"]),
            "(o != null ? (invoke(_f0, o!) ? o : null) : (dropOwned(_f0), null))"
        );
        assert_eq!(
            expr("o", "ok_or_else", &["owned(e)"]),
            "(o != null ? (dropOwned(_f0), Result.Ok(o!)) : Result.Err(invoke(_f0)))"
        );
    }

    /// And an ordinary arrow — one that captures nothing with drop glue — is
    /// called where it stands, because there is nothing to release and nothing
    /// R10 refuses.
    #[test]
    fn a_plain_arrow_is_called_where_it_stands() {
        assert_eq!(expr("o", "map", &["(v) => v"]), "(o != null ? ((v) => v)(o!) : null)");
        assert_eq!(expr("o", "is_some_and", &["(v) => v"]), "(o != null && ((v) => v)(o!))");
        assert_eq!(
            expr("o", "filter", &["(v) => v"]),
            "(o != null && ((v) => v)(o!) ? o : null)"
        );
    }
}
