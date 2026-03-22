//! HashSet<T>/BTreeSet<T> → Set<T> method and static call translations

use super::MethodTranslation;

/// Translate HashSet/BTreeSet static/associated function calls
pub fn translate_static(func: &str, _args: &[String]) -> Option<String> {
    match func {
        "HashSet::new" | "HashSet.new" | "BTreeSet::new" | "BTreeSet.new" => Some("new Set()".to_string()),
        _ => None,
    }
}

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let result = match method {
        // Properties
        "len" => format!("{}.size", receiver),
        "is_empty" => format!("{}.size === 0", receiver),

        // Renamed methods
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
