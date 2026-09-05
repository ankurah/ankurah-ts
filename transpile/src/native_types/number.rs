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
        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
