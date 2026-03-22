//! HashSet<T>/BTreeSet<T> → Set<T> method translations

use super::MethodTranslation;

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
