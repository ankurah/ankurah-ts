//! AtomicUsize/AtomicU32 → number method translations
//!
//! Rust atomics map to plain numbers in single-threaded JS.
//! Ordering arguments are stripped (no JS equivalent).

use super::MethodTranslation;

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

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
