//! Vec<T> → T[] method and static call translations

use super::MethodTranslation;

/// Translate Vec static/associated function calls
pub fn translate_static(func: &str, _args: &[String]) -> Option<String> {
    match func {
        "Vec::new" | "Vec.new" => Some("[]".to_string()),
        "Vec::with_capacity" | "Vec.withCapacity" => Some("[]".to_string()),
        _ => None,
    }
}

/// What an array holds, as the copier needs to know it.
///
/// For: `to_vec` and `to_owned` on a slice CLONE each element — Rust's
/// signature says `T: Clone` — and `slice()` copies the array while leaving both
/// copies holding the same elements. In the port that is two owners for one
/// value: core's `subscription_state.ts`, `node.ts` and proto's `clock.ts` each
/// handed a caller an array whose elements the original still released.
pub struct Element {
    /// The type the port writes the element as.
    pub written: String,
    /// Whether the port has a copy for it — either one it writes out, or the
    /// element's own `clone()`.
    pub has_clone: bool,
}

impl Element {
    /// Nothing is known about the element: the untyped path, and `Uint8Array`'s
    /// fallback, which never reaches the copier.
    pub fn unknown() -> Element {
        Element { written: String::new(), has_clone: false }
    }

    /// What the registry says this element is.
    pub fn of(reg: &crate::registry::TypeRegistry, ty: &crate::ty::Ty) -> Element {
        let written = crate::name_map::map_ty(reg, ty);
        // A copy the port writes out — a spread, a `new Uint8Array`, a `map` —
        // asks nothing of the element. One that is only `e.clone()` does, and
        // the registry is what says whether the element has one.
        let asks_for_clone =
            crate::derives::cloning::clone_of("e", &written) == "e.clone()";
        let has_clone = !asks_for_clone
            || reg
                .system_type(crate::registry::CLONE_PATH)
                .is_some_and(|clone| {
                    crate::registry::Probe::new(reg, reg.crate_root()).implements(ty, clone)
                });
        Element { written, has_clone }
    }
}

/// A copy of an array, element by element — or the reason the port cannot make
/// one.
///
/// `shallow` is what to write where the elements need no copy of their own: a
/// number and a string are copied by being read, so the copy of the array is
/// the whole copy.
pub(crate) fn copy(receiver: &str, element: &Element, shallow: &str) -> Result<String, String> {
    let each = crate::derives::cloning::clone_of("e", &element.written);
    if element.written.is_empty() || each == "e" {
        return Ok(shallow.to_string());
    }
    if !element.has_clone {
        return Err(format!(
            "`{}` has no `clone()` in the port, so the copy would hold what the original still \
             owns",
            element.written
        ));
    }
    Ok(format!("{}.map((e) => {})", receiver, each))
}

pub fn translate(
    receiver: &str,
    method: &str,
    args: &[String],
    element: &Element,
) -> MethodTranslation {
    let result = match method {
        // Properties (not methods in JS)
        "len" => format!("{}.length", receiver),
        "is_empty" => format!("{}.length === 0", receiver),

        // Passthrough — same name in JS
        "push" | "pop" | "reverse" | "join" | "map" | "filter" | "find"
            => return MethodTranslation::Passthrough,

        // `Vec::sort()` orders by `Ord`. JavaScript's argument-less `sort`
        // orders by `String(value)`, so a `Vec<Key>` came out ordered by
        // `[object Object]` — every element equal, and the order whatever the
        // engine's sort happened to leave.
        "sort" | "sort_unstable" if args.is_empty() => {
            format!("{}.sort((a, b) => a.compareTo(b))", receiver)
        }

        // Renamed methods
        "contains" => format!("{}.includes({})", receiver, args.join(", ")),
        "last" => format!("{}.at(-1)", receiver),
        "first" => format!("{}[0]", receiver),
        "sort_by" | "sort_unstable_by" => format!("{}.sort({})", receiver, args.join(", ")),
        // `sort_by_key(f)` orders by what `f` answers, through that type's own
        // ordering.
        "sort_by_key" if args.len() == 1 => format!(
            "{}.sort((a, b) => {{ const f = {}; return f(a).compareTo(f(b)); }})",
            receiver, args[0]
        ),

        // Structural transforms
        "insert" if args.len() == 2 => format!("{}.splice({}, 0, {})", receiver, args[0], args[1]),
        "remove" if args.len() == 1 => format!("{}.splice({}, 1)[0]", receiver, args[0]),
        "extend" if args.len() == 1 => format!("{}.push(...{})", receiver, args[0]),
        "drain" => format!("{}.splice(0)", receiver),
        "clear" => format!("{}.length = 0", receiver),
        "truncate" if args.len() == 1 => format!("{}.length = {}", receiver, args[0]),
        // `Vec::retain` keeps what the predicate accepts, IN PLACE, and DROPS
        // what it rejects. `filter` answers a new array and leaves the original
        // as it was, so the two lines the emitter wrote — a `/* TODO */` comment
        // and a `filter` whose answer nobody read — changed nothing at all.
        //
        // Three things the loop that replaced it still got wrong, all of them
        // about the PREDICATE and about what happens when it throws:
        //
        //   - it was interpolated INSIDE the loop, so a `move` closure was
        //     constructed once per element and an `OwnedClosure` — which is not
        //     callable as a function (R10) — threw a `TypeError` on the first;
        //   - `retain` takes its predicate by value and Rust drops it when the
        //     call ends, however it ends, and nothing released it;
        //   - the array was truncated only on normal completion, so a predicate
        //     that threw left the already-dropped elements still in it and the
        //     kept ones duplicated: a later cascade dropped them twice.
        //
        // So: an IIFE over `(receiver, predicate)`, which is Rust's own
        // evaluation order and evaluates each exactly once; `invokeRef`,
        // because an `FnMut` bound borrows the closure however the parameter is
        // written (fixpass5 §3.3); and a `finally` that moves the tail the loop
        // never reached down over the gap the rejected elements left, cuts the
        // array to what is left, and releases the predicate. That is what
        // Rust's own `BackshiftOnDrop` guard does on an unwind — the element
        // the predicate threw on is counted unprocessed and kept.
        "retain" if args.len() == 1 => format!(
            "((<T,>($xs: T[], $p: Invocable<[T], boolean>) => {{\n\
             \x20 let $at = 0;\n\
             \x20 let $i = 0;\n\
             \x20 try {{\n\
             \x20   for (; $i < $xs.length; $i++) {{\n\
             \x20     if (invokeRef($p, $xs[$i])) {{ $xs[$at++] = $xs[$i]; }} else {{ dropOwned($xs[$i]); }}\n\
             \x20   }}\n\
             \x20 }} finally {{\n\
             \x20   for (; $i < $xs.length; $i++) $xs[$at++] = $xs[$i];\n\
             \x20   $xs.length = $at;\n\
             \x20   dropOwned($p);\n\
             \x20 }}\n\
             }})({}, {}))",
            receiver, args[0]
        ),
        "split_last" => format!(
            "{}.length > 0 ? [{}.at(-1), {}.slice(0, -1)] : null",
            receiver, receiver, receiver
        ),
        "split_first" => format!(
            "{}.length > 0 ? [{}[0], {}.slice(1)] : null",
            receiver, receiver, receiver
        ),

        // `to_vec` and `to_owned` on a slice COPY it, and Rust's own signature
        // says how: `T: Clone`, one clone per element. `slice()` copies the
        // ARRAY and leaves both copies holding the same elements, so the port
        // had two owners for one value and the second drop was a fatal. Each
        // element is copied by its own Clone shape; where that is nothing —
        // a number, a string — the copy of the array is the whole copy, and
        // `slice()` is what it was.
        // `Vec::clone` is `to_vec` under another name — Rust's own signature
        // says so, `T: Clone` and one clone per element — and it fell through
        // to a bare `.clone()` on a JavaScript array, with no diagnostic. Live
        // at `core/reactor.test.ts`, where `self.entities.clone()` answered a
        // method the array has not got.
        "clone" | "to_vec" | "to_owned" if args.is_empty() => {
            match copy(receiver, element, &format!("{}.slice()", receiver)) {
                Ok(written) => written,
                Err(why) => {
                    return MethodTranslation::Refused {
                        message: format!(
                            "`{}` copies a slice, which clones each element, and {}",
                            method, why
                        ),
                        fallback: Box::new(MethodTranslation::Expr(format!("{}.slice()", receiver))),
                    }
                }
            }
        }

        // Iterator entry points — these convert to array operations

        "iter" | "into_iter" => format!("[...{}]", receiver),
        "values" => format!("[...{}]", receiver),

        // Everything else an iterator declares is an array operation, and the
        // table for those is shared with the untyped path rather than copied:
        // a `Cloned<Values<'_, K, V>>` is a JavaScript array, so `collect` and
        // `cloned` mean on it what they mean on any other one.
        _ => match super::iterator::translate(receiver, method, args) {
            Some(result) => result,
            None => return MethodTranslation::Passthrough,
        },
    };
    MethodTranslation::Expr(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holding(written: &str, has_clone: bool) -> Element {
        Element { written: written.to_string(), has_clone }
    }

    fn expr(receiver: &str, method: &str, element: &Element) -> String {
        match translate(receiver, method, &[], element) {
            MethodTranslation::Expr(ts) => ts,
            MethodTranslation::Refused { fallback, .. } => match *fallback {
                MethodTranslation::Expr(ts) => ts,
                _ => panic!("{method} refuses with no fallback expression"),
            },
            _ => panic!("{method} has no expression translation"),
        }
    }

    /// PREMISE CHANGED 2026-09-05 (fixpass4 item 5): what this used to assert is
    /// that `to_vec` is `slice()`, full stop. `slice()` copies the ARRAY and
    /// leaves both copies holding the same elements, and Rust's own signature
    /// says otherwise — `T: Clone`, one clone per element. The port had two
    /// owners for one value: core's `subscription_state.ts` and `node.ts`, and
    /// proto's `clock.ts`.
    #[test]
    fn to_vec_on_an_array_clones_each_element() {
        for method in ["to_vec", "to_owned"] {
            // Nothing inside to copy: the copy of the array is the whole copy.
            assert_eq!(expr("xs", method, &holding("number", true)), "xs.slice()");
            assert_eq!(
                expr("xs", method, &holding("Event", true)),
                "xs.map((e) => e.clone())"
            );
            // The element's own shape decides, at any depth.
            assert_eq!(
                expr("xs", method, &holding("Uint8Array", true)),
                "xs.map((e) => new Uint8Array(e))"
            );
            // With nothing known about the element the copy is what it was.
            assert_eq!(expr("xs", method, &Element::unknown()), "xs.slice()");
        }
    }

    /// An element with no `clone()` is reported rather than shared: the fallback
    /// keeps the shape of the output, and the diagnostic says what it does not
    /// do.
    #[test]
    fn an_element_with_no_clone_is_reported() {
        match translate("xs", "to_vec", &[], &holding("Opaque", false)) {
            MethodTranslation::Refused { message, .. } => {
                assert!(message.contains("has no `clone()`"), "{}", message)
            }
            _ => panic!("an element with no clone is refused"),
        }
    }
}
