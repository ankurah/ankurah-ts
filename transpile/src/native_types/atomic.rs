//! `std::sync::atomic` as the plain value the port writes it as.
//!
//! ONE rule, and every part of the engine holds it: an atomic IS the value it
//! holds. `AtomicUsize` and its numeric peers are a `number`, `AtomicBool` is a
//! `boolean` (`name_map::system_shapes`), `new` is the argument, and every
//! other method is a read or a write of that place. A single-threaded host has
//! nothing for an `Ordering` to say, so the argument is dropped.

use super::MethodTranslation;

/// Every atomic whose value the port writes plainly, spelled as the corpus
/// writes the type. This list and `name_map`'s are the same list: an atomic the
/// type mapping does not know must not have its constructor lowered either, or
/// the emitted code builds a plain value and declares it a class nothing exports.
const ATOMICS: [&str; 4] = ["AtomicBool", "AtomicU32", "AtomicU64", "AtomicUsize"];

/// The atomics the declared surface writes that the port has no shape for. A
/// use of one is a hole: `AtomicI64::new(1)` emitted `AtomicI64.new(1n)` and
/// `fetchAdd` on it — a class nothing exports — with nothing said.
const UNWRITTEN: [&str; 1] = ["AtomicI64"];

/// Every method `std::sync::atomic`'s types carry, whether or not the declared
/// surface writes it. A name here that reaches no arm of `translate` is a hole,
/// because the port writes an atomic as a plain value and no plain value has
/// it; a name that is NOT here — `clone` on a `Copy` number — is a method on
/// that plain value, and goes on to `number::translate` as it always did.
const ATOMIC_API: [&str; 23] = [
    "load",
    "store",
    "swap",
    "compare_and_swap",
    "compare_exchange",
    "compare_exchange_weak",
    "fetch_add",
    "fetch_sub",
    "fetch_and",
    "fetch_nand",
    "fetch_not",
    "fetch_or",
    "fetch_xor",
    "fetch_update",
    "fetch_max",
    "fetch_min",
    "into_inner",
    "get_mut",
    "as_ptr",
    "from_ptr",
    "from_mut",
    "get_mut_slice",
    "from_mut_slice",
];

/// Does this written name belong to an atomic the port has no shape for?
pub fn is_unwritten(name: &str) -> bool {
    UNWRITTEN.contains(&name)
}

/// The unwritten atomic a written call path names, where it names one. The path
/// takes either separator, as `translate_static` reads it, and its owner may
/// carry the module it was written through.
pub fn unwritten_owner(func: &str) -> Option<&'static str> {
    let (owner, _) = func.rsplit_once("::").or_else(|| func.rsplit_once('.'))?;
    let name = owner.rsplit("::").next().unwrap_or(owner);
    UNWRITTEN.iter().copied().find(|unwritten| *unwritten == name)
}

/// What a use of such an atomic says, whether it is the constructor or a call
/// on one. The name is the type's, because the sentence is about the type.
pub fn unwritten_message(name: &str) -> String {
    format!(
        "`{}` is declared in the std surface and the port writes no shape for it, so a value of \
         it would be built as a class nothing exports",
        name
    )
}

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
///
/// Rust evaluates the arguments before the call reads the place, and evaluates
/// each of them once, so every arm binds its operands first and reads the place
/// after them: an operand that ticks a counter ticks it exactly as often as
/// Rust does, whichever way a comparison inside the call then goes.
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

        // The place is written and never read, so JavaScript's own order —
        // the place expression, then the value — is already Rust's.
        "store" if !args.is_empty() => format!("{} = {}", receiver, args[0]),

        // The value it swapped out, which is what Rust's `swap` answers.
        "swap" if !args.is_empty() => {
            place_body(receiver, &[("_n", &args[0])], &format!("{} = _n; return _v;", receiver))
        }

        // Compare-and-swap answers the value it FOUND: `Ok(old)` where the
        // swap happened, `Err(old)` where it did not. Answered as a bare
        // boolean, `is_ok` and `unwrap` ran on something that has neither.
        "compare_exchange" | "compare_exchange_weak" if args.len() >= 2 => place_body(
            receiver,
            &[("_want", &args[0]), ("_next", &args[1])],
            &format!(
                "if (_v === _want) {{ {} = _next; return Result.Ok(_v); }} return Result.Err(_v);",
                receiver
            ),
        ),

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

        // `fetch_max` and `fetch_min` compare and then store, and Rust reads
        // the operand once whichever way the comparison goes.
        "fetch_max" | "fetch_min" if !args.is_empty() => place_body(
            receiver,
            &[("_n", &args[0])],
            &format!(
                "if (_n {} _v) {} = _n; return _v;",
                if method == "fetch_max" { ">" } else { "<" },
                receiver
            ),
        ),

        // An atomic's own method that reaches here is one the port does not
        // write, and it reached JavaScript as a method no number has:
        // `fetch_nand` got a "no method" row and `r.fetchNand(..)` beside it.
        _ if ATOMIC_API.contains(&method) => return refused(no_such_method(method)),
        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}

/// What a method no arm above writes says. Named so the totality test can tell
/// this sentence from a refusal an arm made on purpose.
fn no_such_method(method: &str) -> String {
    format!(
        "`{}` is a method on an atomic that the port does not write, and an atomic is a plain \
         value here, which has no such method",
        method
    )
}

/// The shape every read-modify-write takes: each operand into its own `const`
/// in Rust's order, then the place into `_v`, then what the method does with
/// them. The place itself is read and written where it stands, because
/// JavaScript has no reference to hand a plain value's home over by.
fn place_body(receiver: &str, operands: &[(&str, &str)], rest: &str) -> String {
    let mut bound = String::new();
    for (name, text) in operands {
        bound.push_str(&format!("const {} = {}; ", name, text));
    }
    format!("(() => {{ {}const _v = {}; {} }})()", bound, receiver, rest)
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
        Some(prim) => MethodTranslation::Expr(place_body(
            receiver,
            &[("_n", operand)],
            &format!(
                "{r} = {h}(_v, _n, '{w}'); return _v;",
                r = receiver,
                h = helper,
                w = crate::operators::primitives::width_name(prim)
            ),
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
/// answer in. On an atomic whose width the port does not hold, the answer would
/// silently lose the bits above that width, so it is refused.
fn bitwise(method: &str, receiver: &str, operand: &str, holds: Holds) -> MethodTranslation {
    let stored = match holds {
        // Rust's `AtomicBool` names the logical operators. `&&` and `||` skip
        // nothing here, because the operand is already in its own `const`.
        Holds::Boolean => match method {
            "fetch_and" => "_v && _n".to_string(),
            "fetch_or" => "_v || _n".to_string(),
            _ => "_v !== _n".to_string(),
        },
        // On an integer this is the plain operator at the atomic's own width.
        // JavaScript's `|` answers a SIGNED 32-bit number, so a `fetch_or` with
        // the high bit set stored -2147483647 where Rust stores 2147483649.
        Holds::Integer(Some(prim)) => {
            let native = match method {
                "fetch_and" => "&",
                "fetch_or" => "|",
                _ => "^",
            };
            match crate::operators::primitives::bit_operation(native, prim, "_v", "_n") {
                Some(written) => written,
                None => {
                    return refused(format!(
                        "`{}` is not an operation the port writes between two numbers",
                        method
                    ))
                }
            }
        }
        Holds::Integer(None) => {
            return refused(format!(
                "`{}` on this atomic reads bits at a width the port does not write the atomic as, \
                 so the answer would drop the bits above it",
                method
            ))
        }
    };
    MethodTranslation::Expr(place_body(
        receiver,
        &[("_n", operand)],
        &format!("{} = {}; return _v;", receiver, stored),
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

    /// Every type the declared surface writes an `impl Atomic*` block for.
    fn declared_types() -> Vec<String> {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("std_surface/std/sync/atomic.rs"),
        )
        .expect("the declared atomic surface");
        let mut found = Vec::new();
        for line in source.lines() {
            let Some(rest) = line.strip_prefix("impl Atomic") else { continue };
            let name = format!("Atomic{}", rest.split_whitespace().next().unwrap_or_default());
            if !found.contains(&name) {
                found.push(name);
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
            assert!(
                super::ATOMIC_API.contains(&method.as_str()),
                "`{method}` is declared on an atomic and is not in this module's own list of \
                 atomic methods, so a name it does not write would pass through as a method on a \
                 plain number"
            );
            for holds in [Holds::Boolean, Holds::Integer(Some(crate::ty::Prim::U32)), Holds::Integer(None)] {
                let answer = translate("$r", &method, &arguments, holds);
                assert!(
                    !matches!(answer, MethodTranslation::Passthrough),
                    "`{}` is declared on an atomic and {:?} reaches no arm",
                    method,
                    holds
                );
                if let MethodTranslation::Refused { message, .. } = &answer {
                    assert_ne!(
                        *message,
                        super::no_such_method(&method),
                        "`{}` is declared on an atomic and {:?} reaches no arm of its own",
                        method,
                        holds
                    );
                }
            }
        }
    }

    /// And a method that is NOT the atomic's own is a method on the plain value
    /// it is written as: `clone` on a `Copy` number is a live corpus site.
    #[test]
    fn a_method_on_the_plain_value_passes_through() {
        for method in ["clone", "to_string", "cmp"] {
            let answer = translate("$r", method, &[], Holds::Integer(Some(crate::ty::Prim::U32)));
            assert!(
                matches!(answer, MethodTranslation::Passthrough),
                "`{method}` is not an atomic's own method and belongs to the number it holds"
            );
        }
    }

    /// And the TYPE list the same way: an atomic the surface declares is either
    /// one the port writes as a plain value or one every use of is a hole.
    #[test]
    fn every_declared_atomic_type_is_written_or_unwritten() {
        let declared = declared_types();
        assert!(!declared.is_empty(), "the declared surface writes no `impl Atomic*` block");
        for name in &declared {
            assert!(
                super::ATOMICS.contains(&name.as_str()) || super::is_unwritten(name),
                "`{name}` is declared in the std surface and is in neither list here, so a value \
                 of it is emitted as a class nothing exports"
            );
        }
        for name in super::UNWRITTEN {
            assert!(
                declared.iter().any(|d| d == name),
                "`{name}` is refused here and the declared surface does not write it"
            );
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
    /// answer at the atomic's width, with the boolean atomics taking the
    /// logical operators.
    #[test]
    fn the_bitwise_family_answers_the_old_value() {
        let and = translate("$r", "fetch_and", &["$n".to_string()], Holds::Boolean);
        let MethodTranslation::Expr(text) = and else {
            panic!("`fetch_and` on a boolean atomic is lowered")
        };
        assert_eq!(text, "(() => { const _n = $n; const _v = $r; $r = _v && _n; return _v; })()");

        let or = translate("$r", "fetch_or", &["$n".to_string()], Holds::Integer(Some(crate::ty::Prim::Usize)));
        let MethodTranslation::Expr(text) = or else {
            panic!("`fetch_or` on a sized integer atomic is lowered")
        };
        assert_eq!(
            text,
            "(() => { const _n = $n; const _v = $r; $r = ((_v | _n) >>> 0); return _v; })()"
        );

        let unsized_or = translate("$r", "fetch_or", &["$n".to_string()], Holds::Integer(None));
        assert!(
            matches!(unsized_or, MethodTranslation::Refused { .. }),
            "an atomic the port holds at no width refuses the bitwise family"
        );
    }

    /// Every operand is written into a `const` before the place is read, so it
    /// runs exactly once and in Rust's order: a branch that does not store must
    /// still have evaluated what it would have stored.
    #[test]
    fn every_operand_is_read_once_before_the_place() {
        let operands = ["$n".to_string(), "$m".to_string(), "$o".to_string(), "$p".to_string()];
        for method in ["swap", "fetch_add", "fetch_and", "fetch_or", "fetch_xor", "fetch_max", "fetch_min"] {
            let MethodTranslation::Expr(text) =
                translate("$r", method, &operands, Holds::Integer(Some(crate::ty::Prim::U32)))
            else {
                panic!("`{method}` on a sized integer atomic is lowered")
            };
            assert_eq!(text.matches("$n").count(), 1, "`{method}` reads its operand {text}");
            let bound = text.find("$n").expect("the operand is bound");
            let read = text.find("= $r").expect("the place is read");
            assert!(bound < read, "`{method}` reads the place before its operand: {text}");
        }

        let MethodTranslation::Expr(text) =
            translate("$r", "compare_exchange", &operands, Holds::Integer(Some(crate::ty::Prim::U32)))
        else {
            panic!("`compare_exchange` on a sized integer atomic is lowered")
        };
        assert_eq!(
            text,
            "(() => { const _want = $n; const _next = $m; const _v = $r; \
             if (_v === _want) { $r = _next; return Result.Ok(_v); } return Result.Err(_v); })()"
        );
    }
}
