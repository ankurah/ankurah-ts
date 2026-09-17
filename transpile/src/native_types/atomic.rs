//! `std::sync::atomic` as the plain value the port writes it as.
//!
//! ONE rule, and every part of the engine holds it: an atomic IS the value it
//! holds. `AtomicUsize` and its numeric peers are a `number`, `AtomicBool` is a
//! `boolean` (`name_map::system_shapes`), `new` is the argument, and every
//! other method is a read or a write of that place. A single-threaded host has
//! nothing for an `Ordering` to say, so the argument is dropped.
//!
//! A method the surface DECLARES and this module does not lower resolves at the
//! call and then reaches JavaScript as a method no number has, with no
//! diagnostic. The test at the bottom holds the two lists together.

use super::MethodTranslation;

/// Every atomic whose value the port writes plainly, spelled as the corpus
/// writes the type.
///
/// This list and `name_map`'s are the same list: an atomic the type mapping
/// does not know is one whose constructor must not be lowered either, or the
/// emitted code builds a plain value and then declares it as a class nothing
/// exports.
const ATOMICS: [&str; 4] = ["AtomicBool", "AtomicU32", "AtomicU64", "AtomicUsize"];

/// What the atomic holds, which decides what its read-modify-writes mean:
/// `fetch_and` on a boolean is `&&` and on an integer is `&`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Holds {
    Boolean,
    Integer(Option<crate::ty::Prim>),
}

/// `Atomic*::new(v)` is `v`: the atomic is the value it holds.
pub fn translate_static(func: &str, args: &[String]) -> Option<String> {
    if args.len() != 1 {
        return None;
    }
    let (owner, method) = func
        .rsplit_once("::")
        .or_else(|| func.rsplit_once('.'))?;
    if method != "new" || !ATOMICS.contains(&owner) {
        return None;
    }
    Some(args[0].clone())
}

/// One call on an atomic, written as the read or the write of the place it is.
pub fn translate(
    receiver: &str,
    method: &str,
    args: &[String],
    holds: Holds,
) -> MethodTranslation {
    let width = match holds {
        Holds::Integer(width) => width,
        Holds::Boolean => None,
    };
    let result = match method {
        // Reads of the place: the value, and the value the caller takes with it.
        // `get_mut` hands back a reference Rust writes THROUGH, and the port
        // has no reference to a plain value, so it is refused rather than
        // written as a copy whose writes go nowhere.
        "load" | "into_inner" => receiver.to_string(),
        "get_mut" => {
            return refused(format!(
                "`get_mut` hands back a reference to the value inside this atomic, and the port \
                 writes an atomic as the plain value it holds, which has no reference to hand \
                 out — so a write through `{}` would land on a copy",
                receiver
            ))
        }

        "store" if !args.is_empty() => format!("{} = {}", receiver, args[0]),

        // The value it swapped out, which is what Rust's `swap` answers.
        "swap" if !args.is_empty() => format!(
            "(() => {{ const _v = {r}; {r} = {n}; return _v; }})()",
            r = receiver,
            n = args[0]
        ),

        // Compare-and-swap answers the value it FOUND: `Ok(old)` where the
        // swap happened, `Err(old)` where it did not. Answered as a bare
        // boolean, `is_ok` and `unwrap` ran on something that has neither.
        "compare_exchange" | "compare_exchange_weak" if args.len() >= 2 => {
            let found = format!("(() => {{ const _v = {r}; if (_v === {c})", r = receiver, c = args[0]);
            let swap = format!(" {{ {r} = {n}; return Result.Ok(_v); }}", r = receiver, n = args[1]);
            format!("{found}{swap} return Result.Err(_v); }})()")
        }

        // Rust's atomics WRAP at their width, whatever the build's debug
        // assertions say: `AtomicU32::MAX.fetch_add(1)` stores `0`. A `+=` on a
        // `number` went on counting, so the port and Rust disagreed from the
        // first overflow on.
        "fetch_add" if !args.is_empty() => {
            return wrapping("wrappingAdd", "fetch_add", receiver, &args[0], width)
        }
        "fetch_sub" if !args.is_empty() => {
            return wrapping("wrappingSub", "fetch_sub", receiver, &args[0], width)
        }

        // The bitwise family neither overflows nor wraps, so the old value out
        // and the operator's own answer in is the whole of it. On a boolean the
        // operators are the logical ones Rust's `AtomicBool` names.
        "fetch_and" | "fetch_or" | "fetch_xor" if !args.is_empty() => {
            return bitwise(method, receiver, &args[0], holds)
        }

        // `fetch_max` and `fetch_min` compare, and a comparison reads each
        // operand once whatever width it has.
        "fetch_max" | "fetch_min" if !args.is_empty() => format!(
            "(() => {{ const _v = {r}; if ({n} {op} _v) {r} = {n}; return _v; }})()",
            r = receiver,
            n = args[0],
            op = if method == "fetch_max" { ">" } else { "<" }
        ),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}

/// A read-modify-write that wraps: the old value out, the wrapped new value in.
///
/// The width is the atomic's own. Without one the helper cannot answer, so the
/// operation is refused rather than written with a guessed width — the same
/// rule the explicit overflow families take.
fn wrapping(
    helper: &str,
    method: &str,
    receiver: &str,
    operand: &str,
    width: Option<crate::ty::Prim>,
) -> MethodTranslation {
    match width {
        Some(prim) => MethodTranslation::Expr(format!(
            "(() => {{ const _v = {r}; {r} = {h}({r}, {n}, '{w}'); return _v; }})()",
            r = receiver,
            h = helper,
            n = operand,
            w = crate::operators::primitives::width_name(prim)
        )),
        // The atomics that reach here with no width are `AtomicU64` and
        // `AtomicI64`, whose TypeScript spelling and whose Rust width disagree
        // — see `native_types::atomic_width`.
        None => refused(format!(
            "`{}` on this atomic wraps at a width the port does not write the atomic as, so \
             neither the wrap nor the operand's own type can be written here",
            method
        )),
    }
}

/// `fetch_and`, `fetch_or` and `fetch_xor`: the old value out, the operator's
/// answer in.
///
/// On a `number` the JavaScript operators read the low 32 bits, which is the
/// same rule the port already writes a plain `&` by; on an atomic whose width
/// the port does not hold, the answer would silently lose the rest, so it is
/// refused.
fn bitwise(method: &str, receiver: &str, operand: &str, holds: Holds) -> MethodTranslation {
    let operator = match (method, holds) {
        ("fetch_and", Holds::Boolean) => "&&",
        ("fetch_or", Holds::Boolean) => "||",
        ("fetch_xor", Holds::Boolean) => "!==",
        ("fetch_and", _) => "&",
        ("fetch_or", _) => "|",
        _ => "^",
    };
    if holds == Holds::Integer(None) {
        return refused(format!(
            "`{}` on this atomic reads bits at a width the port does not write the atomic as, \
             so the answer would drop the bits above it",
            method
        ));
    }
    MethodTranslation::Expr(format!(
        "(() => {{ const _v = {r}; {r} = _v {op} {n}; return _v; }})()",
        r = receiver,
        op = operator,
        n = operand
    ))
}

/// A call the port cannot write: the message says what is missing and the hole
/// stands where the call was, so nothing runs a wrong answer.
fn refused(message: String) -> MethodTranslation {
    MethodTranslation::Refused {
        fallback: Box::new(MethodTranslation::Expr(crate::body::hole_text(&message))),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::{translate, Holds, MethodTranslation};

    /// Every method the declared surface writes on an `Atomic*` type, read out
    /// of `std_surface/std/sync/atomic.rs` itself.
    fn declared_methods() -> Vec<String> {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("std_surface/std/sync/atomic.rs"),
        )
        .expect("the declared atomic surface");
        let mut inside_an_atomic = false;
        let mut found = Vec::new();
        for line in source.lines() {
            if let Some(rest) = line.strip_prefix("impl ") {
                inside_an_atomic = rest.starts_with("Atomic");
            }
            if !inside_an_atomic {
                continue;
            }
            let Some(rest) = line.trim().strip_prefix("pub fn ") else {
                continue;
            };
            let Some((name, _)) = rest.split_once('(') else {
                continue;
            };
            if name != "new" && !found.contains(&name.to_string()) {
                found.push(name.to_string());
            }
        }
        found
    }

    /// The surface and the lowering are one list: a method that resolves at the
    /// call and reaches no arm here is written out as a method no number has.
    #[test]
    fn every_declared_atomic_method_is_lowered_or_refused() {
        let arguments = ["$n".to_string(), "$m".to_string(), "$o".to_string(), "$p".to_string()];
        for method in declared_methods() {
            for holds in [Holds::Boolean, Holds::Integer(Some(crate::ty::Prim::U32)), Holds::Integer(None)] {
                let answer = translate("$r", &method, &arguments, holds);
                assert!(
                    !matches!(answer, MethodTranslation::Passthrough),
                    "`{}` is declared on an atomic and {:?} reaches no arm, so it is written as a \
                     method the value does not have",
                    method,
                    holds
                );
            }
        }
    }

    /// A read-modify-write whose width the engine could not resolve emits a
    /// hole, not a guess: `this.c + 1n` on a `number` field puts a `bigint`
    /// beside a `number`, which JavaScript refuses to mix.
    #[test]
    fn a_wrapping_write_without_a_width_is_a_hole() {
        for method in ["fetch_add", "fetch_sub"] {
            let MethodTranslation::Refused { message, fallback } =
                translate("this.c", method, &["1".to_string()], Holds::Integer(None))
            else {
                panic!("`{method}` was not refused without a width")
            };
            assert!(!message.contains("  "), "the message is mangled: {message}");
            let MethodTranslation::Expr(text) = *fallback else {
                panic!("`{method}`'s fallback is not an expression")
            };
            assert!(text.starts_with("unsupported("), "`{method}` still runs: {text}");
        }
    }

    /// The bitwise family answers the old value and stores the operator's own
    /// answer, with the boolean atomics taking the logical operators.
    #[test]
    fn the_bitwise_family_answers_the_old_value() {
        let and = translate("$r", "fetch_and", &["$n".to_string()], Holds::Boolean);
        let MethodTranslation::Expr(text) = and else {
            panic!("`fetch_and` on a boolean atomic is lowered")
        };
        assert_eq!(text, "(() => { const _v = $r; $r = _v && $n; return _v; })()");

        let or = translate("$r", "fetch_or", &["$n".to_string()], Holds::Integer(Some(crate::ty::Prim::Usize)));
        let MethodTranslation::Expr(text) = or else {
            panic!("`fetch_or` on a sized integer atomic is lowered")
        };
        assert_eq!(text, "(() => { const _v = $r; $r = _v | $n; return _v; })()");

        let unsized_or = translate("$r", "fetch_or", &["$n".to_string()], Holds::Integer(None));
        assert!(
            matches!(unsized_or, MethodTranslation::Refused { .. }),
            "an atomic the port holds at no width refuses the bitwise family"
        );
    }
}
