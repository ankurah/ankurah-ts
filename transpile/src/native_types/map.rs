//! HashMap<K,V>/BTreeMap<K,V> → Map<K,V> method translations

use super::MethodTranslation;

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let result = match method {
        // Properties
        "len" => format!("{}.size", receiver),
        "is_empty" => format!("{}.size === 0", receiver),

        // Renamed methods
        "insert" if args.len() == 2 => format!("{}.set({}, {})", receiver, args[0], args[1]),
        "contains_key" => format!("{}.has({})", receiver, args.join(", ")),
        "remove" if args.len() == 1 => format!("{}.delete({})", receiver, args[0]),

        // Passthrough
        "get" | "clear" | "keys" | "values" | "entries"
            => return MethodTranslation::Passthrough,

        // Iterator entry points
        "iter" | "into_iter" => format!("[...{}]", receiver),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
