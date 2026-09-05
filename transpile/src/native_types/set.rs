//! `HashSet<T>` and `BTreeSet<T>` → the runtime's `HashSet`.
//!
//! NOT JavaScript's `Set`, for the reason `map.rs` gives: a `Set` compares by
//! identity, and a set of ids read off the wire then contains nothing.

use super::MethodTranslation;

/// Translate HashSet/BTreeSet static/associated function calls
pub fn translate_static(func: &str, args: &[String]) -> Option<String> {
    match func {
        "HashSet::new" | "HashSet.new" | "BTreeSet::new" | "BTreeSet.new"
        | "HashSet::with_capacity" | "HashSet.withCapacity" => Some("new HashSet()".to_string()),
        "HashSet::default" | "HashSet.default" | "BTreeSet::default" | "BTreeSet.default" => {
            Some("new HashSet()".to_string())
        }
        "HashSet::from" | "HashSet.from" | "BTreeSet::from" | "BTreeSet.from"
            if args.len() == 1 =>
        {
            Some(format!("HashSet.from({})", args[0]))
        }
        _ => None,
    }
}

/// Rust's `insert` answers whether the value was new and `remove` whether it
/// was there; both hand the displaced value to the caller. Where the source
/// reads the answer the emitted call is the runtime's own; where it discards
/// it, the container releases what it took.
pub fn translate_using_result(receiver: &str, method: &str, args: &[String], used: bool) -> MethodTranslation {
    let result = match (method, used) {
        ("insert", true) if args.len() == 1 => format!("{}.insert({})", receiver, args[0]),
        ("remove", true) if args.len() == 1 => format!("{}.remove({})", receiver, args[0]),
        _ => return translate(receiver, method, args),
    };
    MethodTranslation::Expr(result)
}

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let result = match method {
        // Properties
        "len" => format!("{}.size", receiver),
        "is_empty" => format!("{}.size === 0", receiver),

        // Renamed methods
        // The answer is discarded here; a caller that reads it goes through
        // `translate_using_result`.
        "insert" if args.len() == 1 => format!("{}.add({})", receiver, args[0]),
        "contains" => format!("{}.has({})", receiver, args.join(", ")),
        "remove" if args.len() == 1 => format!("{}.delete({})", receiver, args[0]),

        // Passthrough
        "clear" => return MethodTranslation::Passthrough,

        // Iterator entry points
        "iter" | "into_iter" => format!("[...{}]", receiver),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
