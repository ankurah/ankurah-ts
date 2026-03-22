//! String/&str → string method and static call translations

use super::MethodTranslation;

/// Translate String static/associated function calls
pub fn translate_static(func: &str, _args: &[String]) -> Option<String> {
    match func {
        "String::new" | "String.new" => Some("''".to_string()),
        _ => None,
    }
}

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let result = match method {
        // Properties
        "len" => format!("{}.length", receiver),
        "is_empty" => format!("{}.length === 0", receiver),

        // Passthrough — same name in JS
        "trim" | "repeat" | "replace"
            => return MethodTranslation::Passthrough,

        // Renamed (already camelCased by name_map)
        "starts_with" => format!("{}.startsWith({})", receiver, args.join(", ")),
        "ends_with" => format!("{}.endsWith({})", receiver, args.join(", ")),
        "to_lowercase" => format!("{}.toLowerCase()", receiver),
        "to_uppercase" => format!("{}.toUpperCase()", receiver),
        "split" => format!("{}.split({})", receiver, args.join(", ")),
        "contains" => format!("{}.includes({})", receiver, args.join(", ")),
        "chars" => format!("[...{}]", receiver),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
