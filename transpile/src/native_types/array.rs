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
    /// One element copied, written against the name `e`. Asked of the resolved
    /// type where there is one, because that is what the derive writers ask.
    pub copy_of_e: String,
    /// Whether the port has a copy for it — either one it writes out, or the
    /// element's own `clone()`.
    pub has_clone: bool,
    /// Whether the port writes the element as a JavaScript REFERENCE, so that
    /// a loop variable bound to it and the array slot are the same object.
    /// `iter_mut` turns on it: over a number or a string the loop variable is a
    /// COPY and every write through it is lost.
    pub by_reference: bool,
    /// Whether the ELEMENT is itself an `Option`, which the port writes as
    /// `T | null`. A reader answering `Option<Element>` then has one `null` for
    /// two different answers.
    pub nullable: bool,
}

impl Element {
    /// Nothing is known about the element: the untyped path, and `Uint8Array`'s
    /// fallback, which never reaches the copier.
    pub fn unknown() -> Element {
        Element {
            written: String::new(),
            copy_of_e: "e".to_string(),
            has_clone: false,
            by_reference: false,
            nullable: false,
        }
    }

    /// What the registry says this element is.
    pub fn of(reg: &crate::registry::TypeRegistry, ty: &crate::ty::Ty) -> Element {
        let written = crate::name_map::map_ty(reg, ty);
        let copy_of_e = crate::derives::cloning::clone_within(reg, "e", Some(ty));
        // A copy the port writes out — a spread, a `new Uint8Array`, a `map` —
        // asks nothing of the element. One that is only `e.clone()` does, and
        // the registry is what says whether the element has one.
        let asks_for_clone = copy_of_e == "e.clone()";
        let has_clone = !asks_for_clone
            || reg
                .system_type(crate::registry::CLONE_PATH)
                .is_some_and(|clone| {
                    crate::registry::Probe::new(reg, reg.crate_root()).implements(ty, clone)
                });
        let by_reference = crate::name_map::shape::writes_by_reference(reg, ty);
        let nullable = matches!(
            crate::name_map::shape::js_shape(reg, ty),
            crate::name_map::shape::JsShape::Nullable(_)
        );
        Element { written, copy_of_e, has_clone, by_reference, nullable }
    }
}

/// A copy of an array, element by element — or the reason the port cannot make
/// one.
///
/// `shallow` is what to write where the elements need no copy of their own: a
/// number and a string are copied by being read, so the copy of the array is
/// the whole copy.
pub(crate) fn copy(receiver: &str, element: &Element, shallow: &str) -> Result<String, String> {
    let each = element.copy_of_e.clone();
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

/// The readers whose answer is an `Option` of the ELEMENT, with the arity Rust
/// declares. Each of them has one `null` for two different answers where the
/// element is itself an `Option`.
const ELEMENT_READERS: &[(&str, usize)] = &[
    ("first", 0),
    ("last", 0),
    ("get", 1),
    ("pop", 0),
    ("find", 1),
    ("reduce", 1),
    ("max_by", 1),
    ("min_by", 1),
    ("max_by_key", 1),
    ("min_by_key", 1),
];

pub fn translate(
    receiver: &str,
    method: &str,
    args: &[String],
    element: &Element,
    elements: super::iterator::Elements,
) -> MethodTranslation {
    // E13: `Option<T>` is `T | null` here, so a reader answering
    // `Option<Element>` over a `Vec<Option<T>>` has ONE `null` for two
    // different answers — "there is no element" and "the element is `None`".
    // Rust tells them apart and every caller of `first`/`last`/`get` on such a
    // vector is written expecting that, so the reader is refused rather than
    // flattening the two.
    if element.nullable && ELEMENT_READERS.iter().any(|(n, a)| *n == method && *a == args.len()) {
        let message = format!(
            "`{}` answers an `Option` of the element, and this element is itself an `Option`; \
             the port writes both as `null`, so the answer cannot say whether there is no \
             element or an element that is `None`",
            method
        );
        return MethodTranslation::Refused {
            fallback: Box::new(MethodTranslation::Expr(crate::body::hole_text(&message))),
            message,
        };
    }
    let result = match method {
        // Properties (not methods in JS)
        "len" => format!("{}.length", receiver),
        "is_empty" => format!("{}.length === 0", receiver),

        // Passthrough — same name in JS
        // `find` is NOT here: Rust's answers an `Option` and JavaScript's
        // answers `undefined`, so it goes through the shared table below.
        "push" | "pop" | "reverse" | "join" | "map" | "filter"
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
        // J1: `last` and `first` answer an `Option`, and the sentinel each of
        // their JavaScript spellings answers is `undefined`. Both go through
        // the shared Option-adaptor table below.
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
        // F4/E12: `iter_mut` had no entry at all, so it fell through to the
        // camelCase fallback and emitted `xs.iterMut()`, a method no array
        // declares — `TypeError: v.iterMut is not a function`, live at
        // `core/node.ts` and `core/property/backend/lww.ts`. Rust hands out
        // `&mut T`; the port has no `&mut`, so the loop writes through only
        // because the variable and the slot are the same OBJECT. Over a number,
        // a string or a `bigint` the variable is a copy and the write is lost,
        // so that shape is refused rather than emitted silently. The
        // disposition is BORROWED either way: `iter_mut` takes `&mut self` and
        // the elements stay the caller's.
        "iter_mut" if args.is_empty() => {
            if !element.by_reference {
                let message = format!(
                    "`iter_mut` hands out `&mut {}`, and the port writes that element as a \
                     JavaScript value rather than an object: the loop would bind a COPY and \
                     every write through it would be lost",
                    match element.written.is_empty() {
                        true => "the element".to_string(),
                        false => element.written.clone(),
                    }
                );
                return MethodTranslation::Refused {
                    fallback: Box::new(MethodTranslation::Expr(crate::body::hole_text(&message))),
                    message,
                };
            }
            format!("[...{}]", receiver)
        }

        // Everything else an iterator declares is an array operation, and the
        // table for those is shared with the untyped path rather than copied:
        // a `Cloned<Values<'_, K, V>>` is a JavaScript array, so `collect` and
        // `cloned` mean on it what they mean on any other one.
        _ => match super::iterator::translate(
            receiver,
            method,
            args,
            super::iterator::Receiver::Sequence,
            elements,
        ) {
            Some(result) => result,
            None => return MethodTranslation::Passthrough,
        },
    };
    MethodTranslation::Expr(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// E13: `Option<T>` is `T | null` here, so a reader answering
    /// `Option<Element>` over a vector of `Option`s has ONE `null` for two
    /// different answers, and the caller cannot tell "there is no element" from
    /// "the element is `None`".
    #[test]
    fn a_reader_over_a_nullable_element_is_refused() {
        let f = crate::testing::Fixture::build(&[("lib.rs", "pub struct Item { pub n: u32 }\n")]);
        let nullable = Element::of(&f.reg, &f.ty("lib.rs", "Option<u32>"));
        let plain = Element::of(&f.reg, &f.ty("lib.rs", "u32"));
        assert!(nullable.nullable, "an Option element is nullable");
        assert!(!plain.nullable, "a u32 element is not");
        for (method, args) in [("first", 0usize), ("last", 0), ("get", 1), ("find", 1), ("pop", 0)] {
            let args: Vec<String> = (0..args).map(|n| format!("a{n}")).collect();
            let refused = translate("xs", method, &args, &nullable, super::super::iterator::Elements::Borrowed);
            assert!(
                matches!(refused, MethodTranslation::Refused { .. }),
                "`{}` over a nullable element was written anyway",
                method
            );
            let written = translate("xs", method, &args, &plain, super::super::iterator::Elements::Borrowed);
            assert!(
                !matches!(written, MethodTranslation::Refused { .. }),
                "`{}` over a plain element must be unchanged",
                method
            );
        }
    }

    /// An element of a written Rust type, resolved the way a field of it would
    /// be — which is what `Element::of` is handed in a real run.
    fn holding(rust_ty: &str, has_clone: bool) -> Element {
        let f = crate::testing::Fixture::build(&[(
            "lib.rs",
            "pub struct Event { pub n: u32 }\npub struct Opaque { pub n: u32 }\n",
        )]);
        let ty = f.ty("lib.rs", rust_ty);
        Element { has_clone, ..Element::of(&f.reg, &ty) }
    }

    fn expr(receiver: &str, method: &str, element: &Element) -> String {
        match translate(receiver, method, &[], element, super::super::iterator::Elements::Borrowed) {
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
            assert_eq!(expr("xs", method, &holding("u32", true)), "xs.slice()");
            assert_eq!(
                expr("xs", method, &holding("Event", true)),
                "xs.map((e) => e.clone())"
            );
            // The element's own shape decides, at any depth.
            assert_eq!(
                expr("xs", method, &holding("Vec<u8>", true)),
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
        match translate("xs", "to_vec", &[], &holding("Opaque", false), super::super::iterator::Elements::Borrowed) {
            MethodTranslation::Refused { message, .. } => {
                assert!(message.contains("has no `clone()`"), "{}", message)
            }
            _ => panic!("an element with no clone is refused"),
        }
    }
}
