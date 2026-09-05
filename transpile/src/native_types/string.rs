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

        // `String::push` and `push_str` grow the string in place. A JavaScript
        // string cannot be grown, so the place is assigned: `buffer.push('\"')`
        // used to be emitted as it stood and threw `buffer.push is not a
        // function` the first time ankql's SQL renderer ran.
        "push" | "push_str" => format!("{} += {}", receiver, args.join(", ")),
        "insert_str" if args.len() == 2 => format!(
            "{r} = {r}.slice(0, {at}) + {s} + {r}.slice({at})",
            r = receiver,
            at = args[0],
            s = args[1]
        ),
        "clear" => format!("{} = ''", receiver),
        "truncate" if args.len() == 1 => {
            format!("{r} = {r}.slice(0, {n})", r = receiver, n = args[0])
        }
        // `as_str`, `as_mut_str` and `to_string` on a `String` are the string.
        "as_str" | "as_mut_str" => receiver.to_string(),

        // The ordering the port writes as `-1 | 0 | 1`. JavaScript compares two
        // strings by UTF-16 code unit and Rust compares two `String`s by byte;
        // the two agree below U+10000 and disagree on an astral character
        // against one in the surrogate range, which spec 7a records.
        "cmp" | "partial_cmp" if args.len() == 1 => format!(
            "(($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)({}, {})",
            receiver, args[0]
        ),
        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
