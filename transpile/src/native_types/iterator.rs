//! Iterator trait method translations
//!
//! Rust iterator combinators map to JS array methods.
//! These apply when the receiver is already an array (post .iter()).


pub fn translate(receiver: &str, method: &str, args: &[String]) -> Option<String> {
    let result = match method {
        // Renamed methods
        "any" if args.len() == 1 => format!("{}.some({})", receiver, args[0]),
        "all" if args.len() == 1 => format!("{}.every({})", receiver, args[0]),
        "position" if args.len() == 1 => format!("{}.findIndex({})", receiver, args[0]),
        "enumerate" => format!("{}.entries()", receiver),

        // Aggregation
        "sum" => format!("{}.reduce((a, b) => a + b, 0)", receiver),
        "count" => format!("{}.length", receiver),

        // Identity / no-ops in JS array context
        "collect" => receiver.to_string(),
        "cloned" => format!("[...{}]", receiver),

        // Chaining
        "chain" if args.len() == 1 => format!("[...{}, ...{}]", receiver, args[0]),
        "flatten" => format!("{}.flat()", receiver),

        // Passthrough — same name in JS
        "map" | "filter" | "find" | "flat_map" => return None,

        _ => return None,
    };
    Some(result)
}
