//! Iterator trait method translations
//!
//! Rust iterator combinators map to JS array methods.
//! These apply when the receiver is already an array (post .iter()).

/// Is the receiver a sequence the engine RESOLVED, or one it could not name?
///
/// The distinction decides which of the Option-returning readers may be
/// written. `position` and `find_map` name nothing but an iterator adaptor, so
/// they are safe on either. `find`, `first`, `last`, `get` and `reduce` are
/// names any type may declare — `ankql`'s `PathExpr::first()` answers a `&str`,
/// `js_sys::Array::get` answers a `JsValue` and not an `Option` at all — so on
/// a receiver the engine could not type, writing them as the slice reader
/// emits a call to the wrong function. They are written only where the
/// receiver's own type says it is a sequence. `copied` is in that group too:
/// `Option::copied` is the identity on a `T | null`, not a spread.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Receiver {
    /// An array, a slice or a byte buffer: the type is known.
    Sequence,
    /// A receiver the engine could not name.
    Unknown,
}

/// Does the terminal OWN the elements it walks?
///
/// F1: Rust's consuming terminals take the sequence's elements — the one they
/// select is the caller's and every other one is dropped where Rust drops it —
/// and the reading helpers do none of that, because a chain built with `iter()`
/// holds borrows. Written with the reading helper, a consuming chain either
/// released the element it had just handed back (where the emitter had also
/// given the sequence a `dropOwned` of its own) or leaked every element it did
/// not hand back (where it had not).
///
/// The answer is not the method's name: `slice::last(&self)` borrows and
/// `Iterator::last(self)` does not. `BodyTranslator::terminal_owns_the_sequence`
/// asks the resolution, and this is what it tells the lowering.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Elements {
    /// The expression owns them: the terminal releases what it does not return.
    Owned,
    /// Somebody else owns them — a borrowed chain, or elements with no drop
    /// glue at all. The reading helpers are right, and are what is written.
    Borrowed,
}

/// The terminals whose OWNED spelling exists, and the helper each is written as
/// there.
///
/// Read beside `OPTION_ADAPTORS`, which is the same list of Rust names with the
/// reading helper. `first` and `get` are not here: `slice::first(&self)` and
/// `slice::get(&self)` borrow whatever they are called on and have no consuming
/// form to write.
const OWNED_TERMINALS: &[(&str, &str)] = &[
    ("position", "iterPositionOwned"),
    ("rposition", "iterRpositionOwned"),
    ("find_map", "iterFindMapOwned"),
    ("max_by", "iterMaxByOwned"),
    ("min_by", "iterMinByOwned"),
    ("max_by_key", "iterMaxByKeyOwned"),
    ("min_by_key", "iterMinByKeyOwned"),
    ("find", "iterFindOwned"),
    ("last", "iterLastOwned"),
    ("reduce", "iterReduceOwned"),
];

/// Is this call one whose lowering TAKES the sequence's elements, so that the
/// block which held them owes them nothing?
///
/// I1's rule: the move scan and the emitter ask one question and get one
/// answer, so they cannot disagree about who releases what. The arity is
/// checked here exactly as `translate` checks it, because a same-named method
/// of some other trait with some other arity is not this one.
pub fn is_owned_terminal(method: &str, arity: usize) -> bool {
    OWNED_TERMINALS.iter().any(|(rust, _)| *rust == method)
        && OPTION_ADAPTORS.iter().any(|(rust, _, n, _)| *rust == method && *n == arity)
}

/// The adaptors Rust answers an `Option` with, and the base helper each is
/// written as.
///
/// J1: JavaScript's own spellings answer a SENTINEL rather than absence, and
/// the port spells absence `null` (R5). `findIndex` answers `-1`, which
/// `!= null` reads as PRESENT — the reactor's `remove` therefore ran
/// `entries.splice(-1, 1)` for a watcher that was not in the list and deleted
/// the last live one instead. `find` and `at` answer `undefined`, which reads
/// as absent by accident but is not the value a declared `T | null` promises.
/// `reduce` with no initial value throws on an empty array rather than
/// answering absence at all.
///
/// Each helper takes the sequence as its first argument, so the receiver is
/// written once: a receiver written twice is evaluated twice.
///
/// The arity is the Rust arity, checked so that a same-named method of some
/// other trait — a `find` that takes two arguments — falls through rather than
/// being written as this one. The last column is whether the name is safe on a
/// receiver the engine could not type.
const OPTION_ADAPTORS: &[(&str, &str, usize, bool)] = &[
    ("position", "iterPosition", 1, true),
    ("rposition", "iterRposition", 1, true),
    ("find_map", "iterFindMap", 1, true),
    ("max_by", "iterMaxBy", 1, true),
    ("min_by", "iterMinBy", 1, true),
    ("max_by_key", "iterMaxByKey", 1, true),
    ("min_by_key", "iterMinByKey", 1, true),
    ("find", "iterFind", 1, false),
    ("last", "iterLast", 0, false),
    ("reduce", "iterReduce", 1, false),
    // Not iterator adaptors, but the same sentinel: `xs[0]` and `xs[i]` past
    // the end are `undefined`, and `slice::first`/`slice::get` answer `Option`.
    ("first", "iterFirst", 0, false),
    ("get", "iterGet", 1, false),
    // `copied` hands back the elements themselves — but only where the
    // receiver IS a sequence. `Option::copied` is the identity on a `T | null`,
    // and `peers.choose(rng).copied()`, whose receiver the engine could not
    // type, would spread an `Option` as if it were one. (`cloned` keeps the
    // spread on either, which is the answer it already had and which is right
    // at all three corpus sites.)
    ("copied", "$spread", 0, false),
];

/// Did the port BUILD this array on the spot, so that nobody else holds it?
///
/// The emitted text is what will run, and these are the shapes the emitter
/// writes for "a new array": a materialised range, a spread, and the helpers
/// that answer a fresh list. `reverse` mutates, so a copy stands in front of it
/// for every other receiver — and copying one of these copies what was built a
/// character earlier.
fn builds_a_fresh_array(receiver: &str) -> bool {
    let text = receiver.trim().trim_start_matches('(').trim_end_matches(')').trim();
    text.starts_with("[...")
        || ["range(", "rangeIncl(", "stepBy(", "iterFilterMap("]
            .iter()
            .any(|helper| text.starts_with(helper))
}

pub fn translate(
    receiver: &str,
    method: &str,
    args: &[String],
    of: Receiver,
    elements: Elements,
) -> Option<String> {
    for (rust, helper, arity, on_unknown) in OPTION_ADAPTORS {
        if *rust == method && args.len() == *arity && (*on_unknown || of == Receiver::Sequence) {
            if *helper == "$spread" {
                return Some(format!("[...{}]", receiver));
            }
            // The one place the ownership mode changes what is written. A
            // receiver the engine could not name has no ownership answer, and
            // is already reported for being untyped.
            let helper = match elements {
                Elements::Owned => OWNED_TERMINALS
                    .iter()
                    .find(|(rust, _)| *rust == method)
                    .map(|(_, owned)| *owned)
                    .unwrap_or(helper),
                Elements::Borrowed => helper,
            };
            let mut written = format!("{}({}", helper, receiver);
            for a in args {
                written.push_str(", ");
                written.push_str(a);
            }
            written.push(')');
            return Some(written);
        }
    }

    let result = match method {
        // Renamed methods
        "any" if args.len() == 1 => format!("{}.some({})", receiver, args[0]),
        "all" if args.len() == 1 => format!("{}.every({})", receiver, args[0]),
        "enumerate" => format!("{}.entries()", receiver),

        // Aggregation
        "sum" => format!("{}.reduce((a, b) => a + b, 0)", receiver),
        "count" => format!("{}.length", receiver),

        // Identity / no-ops in JS array context
        "collect" => receiver.to_string(),
        "cloned" => format!("[...{}]", receiver),
        // `rev()` walks the same sequence backwards and leaves the original
        // alone; `Array.prototype.reverse` mutates, so the copy comes first.
        // Written as its own name it was `xs.rev()`, a method no array has.
        //
        // E17: unless the receiver is an array the port BUILT on this line —
        // `range(0, n)`, a spread, a `filter_map` — which nobody else holds, so
        // there is nothing for `reverse` to mutate out from under. Ten emitted
        // sites copied a range the line above had just allocated.
        "rev" if args.is_empty() => match builds_a_fresh_array(receiver) {
            true => format!("{}.reverse()", receiver),
            false => format!("{}.slice().reverse()", receiver),
        },
        // `step_by(n)` keeps every nth element of the sequence. Written as the
        // camelCase of its Rust name it was `xs.stepBy(..)`, a method no array
        // declares — `(0..10).step_by(2)` had no diagnostic beside it either
        // (E7).
        "step_by" if args.len() == 1 => format!("stepBy({}, {})", receiver, args[0]),

        // `skip(n)`/`take(n)` are the two ends of a slice.
        "skip" if args.len() == 1 => format!("{}.slice({})", receiver, args[0]),
        "take" if args.len() == 1 => format!("{}.slice(0, {})", receiver, args[0]),
        // `filter_map(f)` keeps what the closure answers `Some` for. Written as
        // the camelCase of its Rust name it was `xs.filterMap(..)`, a method no
        // array declares — twelve emitted sites.
        "filter_map" if args.len() == 1 => {
            format!("iterFilterMap({}, {})", receiver, args[0])
        }

        // Chaining
        "chain" if args.len() == 1 => format!("[...{}, ...{}]", receiver, args[0]),
        "flatten" => format!("{}.flat()", receiver),

        // Passthrough — same name in JS
        "map" | "filter" | "flat_map" => return None,

        _ => return None,
    };
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrote(method: &str, args: &[&str]) -> Option<String> {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        translate("xs", method, &args, Receiver::Sequence, Elements::Borrowed)
    }

    fn wrote_owned(method: &str, args: &[&str]) -> Option<String> {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        translate("xs", method, &args, Receiver::Sequence, Elements::Owned)
    }

    fn wrote_untyped(method: &str, args: &[&str]) -> Option<String> {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        translate("xs", method, &args, Receiver::Unknown, Elements::Borrowed)
    }

    /// J1's live case: `entries.iter().position(..)` answered `-1` for a
    /// watcher that had already gone, `-1 != null` read as present, and
    /// `splice(-1, 1)` removed the last LIVE watcher.
    #[test]
    fn position_answers_null_rather_than_minus_one() {
        assert_eq!(wrote("position", &["(id) => id === w"]).unwrap(), "iterPosition(xs, (id) => id === w)");
    }

    #[test]
    fn find_answers_null_rather_than_undefined() {
        assert_eq!(wrote("find", &["p"]).unwrap(), "iterFind(xs, p)");
    }

    /// `find_map` had no entry at all: it fell through to the camelCase
    /// fallback and emitted `xs.findMap(..)`, a method no array declares.
    #[test]
    fn find_map_is_lowered_rather_than_camel_cased() {
        assert_eq!(wrote("find_map", &["f"]).unwrap(), "iterFindMap(xs, f)");
    }

    #[test]
    fn last_answers_null_rather_than_undefined() {
        assert_eq!(wrote("last", &[]).unwrap(), "iterLast(xs)");
    }

    /// `Array.prototype.reduce` with no initial value THROWS on an empty
    /// array, so it could never stand in for the adaptor that answers `None`.
    #[test]
    fn reduce_answers_null_rather_than_throwing() {
        assert_eq!(wrote("reduce", &["f"]).unwrap(), "iterReduce(xs, f)");
    }

    #[test]
    fn the_max_and_min_families_answer_options() {
        assert_eq!(wrote("max_by", &["c"]).unwrap(), "iterMaxBy(xs, c)");
        assert_eq!(wrote("min_by", &["c"]).unwrap(), "iterMinBy(xs, c)");
        assert_eq!(wrote("max_by_key", &["k"]).unwrap(), "iterMaxByKey(xs, k)");
        assert_eq!(wrote("min_by_key", &["k"]).unwrap(), "iterMinByKey(xs, k)");
    }

    /// The receiver is written ONCE: a receiver written twice is evaluated
    /// twice, and the second evaluation of a call with an effect is a defect.
    #[test]
    fn the_receiver_is_written_once() {
        let written =
            translate("takeOne()", "position", &["p".into()], Receiver::Sequence, Elements::Borrowed).unwrap();
        assert_eq!(written.matches("takeOne()").count(), 1, "{}", written);
    }

    /// A same-named method of some other trait, with some other arity, is not
    /// this one and falls through to the receiver's own table.
    #[test]
    fn a_different_arity_is_not_this_adaptor() {
        assert!(wrote("find", &["a", "b"]).is_none());
        assert!(wrote("last", &["a"]).is_none());
    }

    /// A receiver the engine could not type may declare `first`, `get`, `find`,
    /// `last` or `reduce` itself: `ankql`'s `PathExpr::first()` answers a
    /// `&str` and `js_sys::Array::get` answers a `JsValue`. Writing the slice
    /// reader there calls the wrong function, so those names wait for a type.
    #[test]
    fn a_generic_name_on_an_untyped_receiver_is_left_alone() {
        for m in ["find", "first", "get", "last", "reduce", "copied"] {
            let args: &[&str] =
                if matches!(m, "first" | "last" | "copied") { &[] } else { &["a"] };
            assert!(wrote_untyped(m, args).is_none(), "`{}` was claimed on an untyped receiver", m);
            assert!(wrote(m, args).is_some(), "`{}` was not written on a sequence", m);
        }
    }

    /// F1: the ownership mode is what decides which of two helper families is
    /// written, and it is the ONLY thing the mode changes.
    #[test]
    fn an_owned_sequence_gets_the_terminal_that_releases_the_losers() {
        for (method, args, borrowed, owned) in [
            ("find", &["p"][..], "iterFind(xs, p)", "iterFindOwned(xs, p)"),
            ("position", &["p"][..], "iterPosition(xs, p)", "iterPositionOwned(xs, p)"),
            ("rposition", &["p"][..], "iterRposition(xs, p)", "iterRpositionOwned(xs, p)"),
            ("find_map", &["f"][..], "iterFindMap(xs, f)", "iterFindMapOwned(xs, f)"),
            ("reduce", &["f"][..], "iterReduce(xs, f)", "iterReduceOwned(xs, f)"),
            ("max_by", &["c"][..], "iterMaxBy(xs, c)", "iterMaxByOwned(xs, c)"),
            ("min_by", &["c"][..], "iterMinBy(xs, c)", "iterMinByOwned(xs, c)"),
            ("max_by_key", &["k"][..], "iterMaxByKey(xs, k)", "iterMaxByKeyOwned(xs, k)"),
            ("min_by_key", &["k"][..], "iterMinByKey(xs, k)", "iterMinByKeyOwned(xs, k)"),
            ("last", &[][..], "iterLast(xs)", "iterLastOwned(xs)"),
        ] {
            assert_eq!(wrote(method, args).unwrap(), borrowed, "borrowed `{}`", method);
            assert_eq!(wrote_owned(method, args).unwrap(), owned, "owned `{}`", method);
        }
    }

    /// `slice::first` and `slice::get` BORROW whatever they are called on and
    /// have no consuming form, so the mode leaves them alone.
    #[test]
    fn the_slice_readers_have_no_owned_spelling() {
        assert_eq!(wrote_owned("first", &[]).unwrap(), "iterFirst(xs)");
        assert_eq!(wrote_owned("get", &["i"]).unwrap(), "iterGet(xs, i)");
        assert!(!is_owned_terminal("first", 0));
        assert!(!is_owned_terminal("get", 1));
    }

    /// The arity is checked exactly as `translate` checks it, so the move scan
    /// and the lowering agree about which calls are these terminals at all.
    #[test]
    fn the_owned_question_is_asked_with_the_arity() {
        assert!(is_owned_terminal("find", 1));
        assert!(!is_owned_terminal("find", 2));
        assert!(is_owned_terminal("last", 0));
        assert!(!is_owned_terminal("last", 1));
        assert!(!is_owned_terminal("copied", 0));
    }

    /// E17: `reverse` mutates, so a copy stands in front of it — unless the
    /// receiver is an array the port BUILT on the spot, which nobody else
    /// holds. Ten emitted sites copied a range the line above had allocated.
    #[test]
    fn a_freshly_built_array_is_reversed_in_place() {
        assert_eq!(wrote("rev", &[]).unwrap(), "xs.slice().reverse()");
        for fresh in ["range(0, n)", "(range(0, n))", "rangeIncl(0, n)", "[...ys]", "stepBy(xs, 2)"] {
            let written =
                translate(fresh, "rev", &[], Receiver::Sequence, Elements::Borrowed).unwrap();
            assert_eq!(written, format!("{}.reverse()", fresh), "{}", fresh);
        }
        // A name, a field and a call the port did not write stay copied.
        for held in ["xs", "this.steps", "read()"] {
            let written =
                translate(held, "rev", &[], Receiver::Sequence, Elements::Borrowed).unwrap();
            assert_eq!(written, format!("{}.slice().reverse()", held), "{}", held);
        }
    }

    /// The adaptors that name nothing but an iterator are written either way:
    /// `position` always was, and the rest answer an `Option` wherever they
    /// resolve.
    #[test]
    fn an_iterator_only_name_is_written_on_either_receiver() {
        assert_eq!(wrote_untyped("position", &["p"]).unwrap(), "iterPosition(xs, p)");
        assert_eq!(wrote_untyped("find_map", &["f"]).unwrap(), "iterFindMap(xs, f)");
        assert_eq!(wrote_untyped("max_by_key", &["k"]).unwrap(), "iterMaxByKey(xs, k)");
    }
}
