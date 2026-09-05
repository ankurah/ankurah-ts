//! AtomicUsize/AtomicU32 → number method and static call translations
//!
//! Rust atomics map to plain numbers in single-threaded JS.
//! Ordering arguments are stripped (no JS equivalent).

use super::MethodTranslation;

/// Translate Atomic static/associated function calls
pub fn translate_static(func: &str, args: &[String]) -> Option<String> {
    match func {
        // AtomicUsize::new(val) → val (just a number)
        "AtomicUsize::new" | "AtomicUsize.new" | "AtomicU32::new" | "AtomicU32.new"
            if args.len() == 1 => Some(args[0].clone()),
        _ => None,
    }
}

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
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
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        match translate(receiver, method, &args) {
            MethodTranslation::Expr(ts) => ts,
            _ => panic!("`{method}` has no expression translation"),
        }
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
