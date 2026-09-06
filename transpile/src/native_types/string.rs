//! String/&str → string method and static call translations

use super::MethodTranslation;

/// Translate String static/associated function calls
pub fn translate_static(func: &str, args: &[String]) -> Option<String> {
    match func {
        "String::new" | "String.new" => Some("''".to_string()),
        // Rust has TWO byte-to-text answers and which one a site takes is the
        // source's choice: `from_utf8` refuses an invalid run and
        // `from_utf8_lossy` substitutes U+FFFD for it. Written from its name
        // this was `String.fromUtf8Lossy(bytes)`, a static the JavaScript
        // `String` has not got — a `TypeError` at four emitted sites in
        // `core/value/index.ts`, each of them writing an arbitrary byte value
        // out as a query literal.
        "String::from_utf8_lossy" | "String.fromUtf8Lossy" if args.len() == 1 => {
            Some(format!("decodeUtf8Lossy({})", args[0]))
        }
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

        // A JavaScript string is a value, so copying one is reading it: Rust's
        // three spellings for "make me an owned `String` from this" are all the
        // receiver itself. Passed through, `name.clone()` called a method a
        // string has not got — live at `core/entity.ts:61`, where the `String`
        // key of a `BTreeMap` was cloned into an insert. The derived clone
        // writer already stops at a primitive; this is the explicit-call path,
        // which resolved `String::clone` and wrote it out.
        "clone" | "to_owned" | "to_string" if args.is_empty() => receiver.to_string(),

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
