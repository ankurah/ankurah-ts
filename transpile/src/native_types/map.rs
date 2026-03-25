//! HashMap<K,V>/BTreeMap<K,V> → Map<K,V> method and static call translations

use super::MethodTranslation;

/// Translate HashMap/BTreeMap static/associated function calls
pub fn translate_static(func: &str, _args: &[String]) -> Option<String> {
    match func {
        "HashMap::new" | "HashMap.new" | "BTreeMap::new" | "BTreeMap.new" => Some("new Map()".to_string()),
        _ => None,
    }
}

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

        // Mutable iterator variants → same as immutable in JS
        "values_mut" => format!("{}.values()", receiver),
        "get_mut" if args.len() == 1 => format!("{}.get({})", receiver, args[0]),

        // retain(|k, v| predicate) → manual delete loop
        "retain" if args.len() == 1 => format!(
            "{{ for (const [_k, _v] of {}) {{ if (!({}(_k, _v))) {}.delete(_k); }} }}",
            receiver, args[0], receiver
        ),

        // Iterator entry points
        "iter" | "into_iter" => format!("[...{}]", receiver),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
