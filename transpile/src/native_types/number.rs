//! Rust's atomics as the plain values the port writes them as.
//!
//! ONE rule, and every part of the engine holds it: an atomic IS the value it
//! holds. `AtomicUsize` and its numeric peers are a `number`, `AtomicBool` is a
//! `boolean` (`name_map::system_shapes`), `new` is the argument, and `load`,
//! `store`, `fetch_add` and `compare_exchange` are reads and writes of the
//! place. A single-threaded host has nothing for an `Ordering` to say, so the
//! argument is dropped.
//!
//! Leaving `AtomicBool` out of the constructor list left `AtomicBool.new(false)`
//! standing in six emitted places — core's `resultset.ts`, `node.ts`,
//! `context.ts` and `transaction.ts` — where nothing declares an `AtomicBool`:
//! a `ReferenceError` on the line that builds the object.

use super::MethodTranslation;

/// Every atomic whose value the port writes plainly, spelled as the corpus
/// writes the type.
///
/// This list and `name_map`'s are the same list: an atomic the type mapping
/// does not know is one whose constructor must not be lowered either, or the
/// emitted code builds a plain value and then declares it as a class nothing
/// exports.
const ATOMICS: [&str; 4] = ["AtomicBool", "AtomicU32", "AtomicU64", "AtomicUsize"];

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

/// The four explicit families Rust offers for saying what should happen on
/// overflow, as the free helper each one is in `@ankurah/base`.
///
/// A number has no `wrappingAdd` method, so `x.wrapping_add(1)` was a
/// `TypeError` at the call. The helper takes the WIDTH, because that is what
/// decides the answer, and only the resolved receiver type carries it: `u8` and
/// `usize` are both `number` here.
fn explicit_family(method: &str) -> Option<&'static str> {
    Some(match method {
        "wrapping_add" => "wrappingAdd",
        "wrapping_sub" => "wrappingSub",
        "wrapping_mul" => "wrappingMul",
        "saturating_add" => "saturatingAdd",
        "saturating_sub" => "saturatingSub",
        "saturating_mul" => "saturatingMul",
        "checked_add" => "checkedAddOption",
        "checked_sub" => "checkedSubOption",
        "checked_mul" => "checkedMulOption",
        "overflowing_add" => "overflowingAdd",
        "overflowing_sub" => "overflowingSub",
        "overflowing_mul" => "overflowingMul",
        _ => return None,
    })
}

pub fn translate(
    receiver: &str,
    method: &str,
    args: &[String],
    width: Option<crate::ty::Prim>,
) -> MethodTranslation {
    if let (Some(helper), 1) = (explicit_family(method), args.len()) {
        return match width.filter(|prim| prim.range().is_some()) {
            Some(prim) => MethodTranslation::Expr(format!(
                "{}({}, {}, '{}')",
                helper,
                receiver,
                args[0],
                prim.rust_name()
            )),
            // Without the width the helper cannot answer: `wrapping_add` on a
            // `u8` and on a `u32` are two different results.
            None => MethodTranslation::Refused {
                message: format!(
                    "`{}` needs the integer width, and the engine could not resolve the \
                     receiver's type",
                    method
                ),
                fallback: Box::new(MethodTranslation::Passthrough),
            },
        };
    }
    let result = match method {
        // Load — just the value (strip Ordering arg)
        "load" => receiver.to_string(),

        // Store — assignment (strip Ordering arg)
        "store" if args.len() >= 1 => format!("{} = {}", receiver, args[0]),

        // Fetch-add — returns old value, increments
        "fetch_add" if args.len() >= 1 => format!(
            "(() => {{ const _v = {}; {} += {}; return _v; }})()",
            receiver, receiver, args[0]
        ),

        // Fetch-sub
        "fetch_sub" if args.len() >= 1 => format!(
            "(() => {{ const _v = {}; {} -= {}; return _v; }})()",
            receiver, receiver, args[0]
        ),

        // Compare-and-swap
        "compare_exchange" if args.len() >= 2 => format!(
            "(() => {{ if ({} === {}) {{ {} = {}; return true; }} return false; }})()",
            receiver, args[0], receiver, args[1]
        ),

        // `Ord::cmp` and `PartialOrd::partial_cmp` on a number: the ordering the
        // port writes as `-1 | 0 | 1`. A primitive has no `compareTo` method
        // for the call to land on, so the comparison is written out — through
        // an arrow so that each side is evaluated once, whatever expression it
        // is. `Math.sign(a - b)` is not the same answer: it throws on a
        // `bigint` and answers `NaN` where a float is `NaN`, which Rust's
        // `partial_cmp` answers `None` for.
        "cmp" | "partial_cmp" | "total_cmp" if args.len() == 1 => format!(
            "(($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)({}, {})",
            receiver, args[0]
        ),
        // The float methods JavaScript spells differently, or not at all.
        // `n.fract()` stood in three emitted files as written, and no number
        // has such a method: ankql's `conversion.ts`, core's `value/wasm.ts`
        // and storage-indexeddb's `planner_integration.ts` each raised on the
        // line that asks whether a JSON number is a whole one.
        "fract" if args.is_empty() => format!("({} - Math.trunc({}))", receiver, receiver),
        "trunc" if args.is_empty() => format!("Math.trunc({})", receiver),
        "floor" if args.is_empty() => format!("Math.floor({})", receiver),
        "ceil" if args.is_empty() => format!("Math.ceil({})", receiver),
        "round" if args.is_empty() => format!("Math.round({})", receiver),
        "abs" if args.is_empty() => format!("Math.abs({})", receiver),
        "sqrt" if args.is_empty() => format!("Math.sqrt({})", receiver),
        "signum" if args.is_empty() => format!("Math.sign({})", receiver),
        "is_nan" if args.is_empty() => format!("Number.isNaN({})", receiver),
        "is_finite" if args.is_empty() => format!("Number.isFinite({})", receiver),
        "is_infinite" if args.is_empty() => {
            format!("(!Number.isFinite({}) && !Number.isNaN({}))", receiver, receiver)
        }
        "powi" | "powf" if args.len() == 1 => format!("({} ** {})", receiver, args[0]),
        "min" if args.len() == 1 => format!("Math.min({}, {})", receiver, args[0]),
        "max" if args.len() == 1 => format!("Math.max({}, {})", receiver, args[0]),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expr(receiver: &str, method: &str, args: &[&str]) -> String {
        widened(receiver, method, args, None)
    }

    /// The same, with the receiver's resolved width — which is what the
    /// explicit arithmetic families need and nothing else reads.
    fn widened(
        receiver: &str,
        method: &str,
        args: &[&str],
        width: Option<crate::ty::Prim>,
    ) -> String {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        match translate(receiver, method, &args, width) {
            MethodTranslation::Expr(ts) => ts,
            _ => panic!("`{method}` has no expression translation"),
        }
    }

    /// #7: a number has no `wrappingAdd` method, so `x.wrapping_add(1)` was a
    /// `TypeError` at the call. The helper takes the WIDTH, which only the
    /// resolved receiver type carries: `u8` and `usize` are both `number`.
    #[test]
    fn the_explicit_families_go_through_the_free_helpers() {
        use crate::ty::Prim;
        assert_eq!(widened("x", "wrapping_add", &["1"], Some(Prim::U8)), "wrappingAdd(x, 1, 'u8')");
        assert_eq!(
            widened("x", "saturating_sub", &["1"], Some(Prim::Usize)),
            "saturatingSub(x, 1, 'usize')"
        );
        assert_eq!(
            widened("x", "checked_mul", &["2"], Some(Prim::I32)),
            "checkedMulOption(x, 2, 'i32')"
        );
        assert_eq!(
            widened("x", "overflowing_add", &["y"], Some(Prim::U64)),
            "overflowingAdd(x, y, 'u64')"
        );
    }

    /// Without the width the helper cannot answer, so the call is refused
    /// rather than written with a guessed one.
    #[test]
    fn an_unresolved_width_refuses_the_family() {
        let args = vec!["1".to_string()];
        assert!(matches!(
            translate("x", "wrapping_add", &args, None),
            MethodTranslation::Refused { .. }
        ));
    }

    /// Rust's `f64::fract` is the part after the point, which JavaScript has no
    /// method for: `n.fract()` stood in three emitted files as written and
    /// raised on the line that asks whether a JSON number is a whole one.
    #[test]
    fn the_float_methods_javascript_spells_differently() {
        assert_eq!(expr("n", "fract", &[]), "(n - Math.trunc(n))");
        assert_eq!(expr("n", "trunc", &[]), "Math.trunc(n)");
        assert_eq!(expr("n", "abs", &[]), "Math.abs(n)");
        assert_eq!(expr("n", "is_nan", &[]), "Number.isNaN(n)");
        assert_eq!(expr("n", "powi", &["2"]), "(n ** 2)");
        assert_eq!(expr("n", "min", &["m"]), "Math.min(n, m)");
    }
}
