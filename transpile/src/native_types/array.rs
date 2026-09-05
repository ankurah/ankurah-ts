//! Vec<T> → T[] method and static call translations

use super::MethodTranslation;

/// Translate Vec static/associated function calls
pub fn translate_static(func: &str, _args: &[String]) -> Option<String> {
    match func {
        "Vec::new" | "Vec.new" => Some("[]".to_string()),
        "Vec::with_capacity" | "Vec.withCapacity" => Some("[]".to_string()),
        _ => None,
    }
}

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let result = match method {
        // Properties (not methods in JS)
        "len" => format!("{}.length", receiver),
        "is_empty" => format!("{}.length === 0", receiver),

        // Passthrough — same name in JS
        "push" | "pop" | "reverse" | "join" | "map" | "filter" | "find"
            => return MethodTranslation::Passthrough,

        // `Vec::sort()` orders by `Ord`. JavaScript's argument-less `sort`
        // orders by `String(value)`, so a `Vec<Key>` came out ordered by
        // `[object Object]` — every element equal, and the order whatever the
        // engine's sort happened to leave.
        "sort" | "sort_unstable" if args.is_empty() => {
            format!("{}.sort((a, b) => a.compareTo(b))", receiver)
        }

        // Renamed methods
        "contains" => format!("{}.includes({})", receiver, args.join(", ")),
        "last" => format!("{}.at(-1)", receiver),
        "first" => format!("{}[0]", receiver),
        "sort_by" | "sort_unstable_by" => format!("{}.sort({})", receiver, args.join(", ")),
        // `sort_by_key(f)` orders by what `f` answers, through that type's own
        // ordering.
        "sort_by_key" if args.len() == 1 => format!(
            "{}.sort((a, b) => {{ const f = {}; return f(a).compareTo(f(b)); }})",
            receiver, args[0]
        ),

        // Structural transforms
        "insert" if args.len() == 2 => format!("{}.splice({}, 0, {})", receiver, args[0], args[1]),
        "remove" if args.len() == 1 => format!("{}.splice({}, 1)[0]", receiver, args[0]),
        "extend" if args.len() == 1 => format!("{}.push(...{})", receiver, args[0]),
        "drain" => format!("{}.splice(0)", receiver),
        "clear" => format!("{}.length = 0", receiver),
        "truncate" if args.len() == 1 => format!("{}.length = {}", receiver, args[0]),
        "retain" if args.len() == 1 => {
            // Vec::retain is in-place filter. JS doesn't have this natively.
            // We emit a splice-based approach or just use filter + reassign
            format!("/* TODO: retain */ {}.filter({})", receiver, args[0])
        }
        "split_last" => format!(
            "{}.length > 0 ? [{}.at(-1), {}.slice(0, -1)] : null",
            receiver, receiver, receiver
        ),
        "split_first" => format!(
            "{}.length > 0 ? [{}[0], {}.slice(1)] : null",
            receiver, receiver, receiver
        ),

        // Iterator entry points — these convert to array operations
        // `to_vec` and `to_owned` on a slice COPY it, which is what
        // `Array.prototype.slice()` with no arguments does. A `[usize]` used to
        // be written as a `Uint8Array`, whose `slice` this borrowed; now that
        // only `[u8]` is bytes, an ordinary array owes the same answer, and
        // `parentIds.toVec()` — a method no array has — was what it gave.
        "to_vec" | "to_owned" => format!("{}.slice()", receiver),

        "iter" | "into_iter" => format!("[...{}]", receiver),
        "values" => format!("[...{}]", receiver),

        // Everything else an iterator declares is an array operation, and the
        // table for those is shared with the untyped path rather than copied:
        // a `Cloned<Values<'_, K, V>>` is a JavaScript array, so `collect` and
        // `cloned` mean on it what they mean on any other one.
        _ => match super::iterator::translate(receiver, method, args) {
            Some(result) => result,
            None => return MethodTranslation::Passthrough,
        },
    };
    MethodTranslation::Expr(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `to_vec`/`to_owned` on a slice COPY it. While every numeric slice was
    /// written as a `Uint8Array` these borrowed that type's translation; once
    /// only `[u8]` was bytes, six emitted sites read `xs.toVec()`, which no
    /// JavaScript array has.
    #[test]
    fn to_vec_on_an_array_is_a_copy() {
        for method in ["to_vec", "to_owned"] {
            match translate("xs", method, &[]) {
                MethodTranslation::Expr(ts) => assert_eq!(ts, "xs.slice()"),
                _ => panic!("{method} has no expression translation"),
            }
        }
    }
}
