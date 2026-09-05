//! A primitive's associated items — the constants `f64::EPSILON` and
//! `u32::MAX`, and the functions `i64::from_be_bytes` and `u8::from_str_radix`.
//!
//! For: Rust puts these on the type, and the port writes the type as a
//! JavaScript primitive, which has no members of its own. Left alone, the path
//! came out as written — `f64.EPSILON`, `u32.MAX` — and `f64` and `u32` are
//! names the emitted file never declares, so the line threw a `ReferenceError`.
//! Worse for the float ones: nothing typed the expression, so a method call on
//! it fell through the number translations too, and
//! `storage-indexeddb/planner_integration.ts:35` wrote
//! `f64.EPSILON.max(Math.abs(v) * f64.EPSILON)` — an undeclared name and a
//! `.max()` a JavaScript number has not got.
//!
//! One table, read twice: `written` gives the emitted expression and `type_of`
//! gives the type it has, so the value and the calls on it agree.

use super::Prim;

/// The TypeScript expression a `<prim>::<CONST>` path is, where the port has
/// one.
pub fn written(prim: Prim, konst: &str) -> Option<String> {
    if matches!(prim, Prim::F32 | Prim::F64) {
        // A `f32` constant is written at `f64` precision, because the port has
        // one float type: `f32::EPSILON` really is a different number, and
        // where a body compares against it the difference is the port's own
        // float mapping, reported at the type rather than here.
        return Some(
            match konst {
                "EPSILON" => "Number.EPSILON",
                "INFINITY" => "Infinity",
                "NEG_INFINITY" => "-Infinity",
                "NAN" => "NaN",
                "MAX" => "Number.MAX_VALUE",
                "MIN" => "-Number.MAX_VALUE",
                "MIN_POSITIVE" => "Number.MIN_VALUE",
                _ => return None,
            }
            .to_string(),
        );
    }
    // An integer width's MIN and MAX are the numbers `Prim::range` already
    // holds — the one table R13 put the widths in — so the two cannot disagree
    // about what a `usize` holds.
    let (low, high) = prim.range()?;
    // A width the port holds in a `bigint` writes its constants as `bigint`
    // literals: `1n + 1` throws rather than adding, so a `u64::MAX` written as a
    // `number` would be a `TypeError` the first time it met one.
    let suffix = match prim {
        Prim::U64 | Prim::I64 | Prim::U128 | Prim::I128 => "n",
        _ => "",
    };
    match konst {
        "MIN" => Some(format!("{}{}", low, suffix)),
        "MAX" => Some(format!("{}{}", high, suffix)),
        _ => None,
    }
}

/// The type such a path has: the primitive it is written on.
pub fn type_of_path(segments: &[String]) -> Option<Prim> {
    let (prim, konst) = parse(segments)?;
    written(prim, &konst).map(|_| prim)
}

/// The primitive and the constant a two-segment path names, where it names one.
///
/// A CONSTANT, not a function: `i64::from_be_bytes` and `u32::from_str_radix`
/// are also two-segment paths on a primitive, and reading them as constants
/// turned every call of one into a refusal. Rust decides by resolution; the port
/// decides by the naming convention rustc warns on either side of, which is the
/// same convention fixpass5's §3.5 reads a `const` pattern by.
pub fn parse(segments: &[String]) -> Option<(Prim, String)> {
    let (prim, item) = split(segments)?;
    is_screaming(&item).then_some((prim, item))
}

/// The primitive and the item name a two-segment path on a primitive names.
fn split(segments: &[String]) -> Option<(Prim, String)> {
    let [prim, item] = segments else { return None };
    Some((Prim::from_rust_name(prim)?, item.clone()))
}

/// The naming convention rustc warns on either side of, which is how the port
/// tells a constant from a function on a primitive.
fn is_screaming(item: &str) -> bool {
    item.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// The written constant a path names, or the reason the port has none.
///
/// `None` where the path does not name a primitive's constant at all.
pub fn written_or_reason(segments: &[String]) -> Option<Result<String, String>> {
    let (prim, item) = split(segments)?;
    // An associated FUNCTION — `i64::from_be_bytes`, `u8::from_str_radix` — is
    // the same gap as a constant the port has no spelling for, and it was the
    // one half left writing the path out: `i64.fromBeBytes(..)` names something
    // the emitted file never declares, and nothing said so.
    if !is_screaming(&item) {
        return Some(Err(format!(
            "`{}::{}` is a function Rust puts on a primitive type, and the port writes that \
             type as a JavaScript primitive, which has no members and no spelling for this one",
            prim.rust_name(),
            item
        )));
    }
    let konst = item;
    Some(match written(prim, &konst) {
        Some(text) => Ok(text),
        None => Err(format!(
            "`{}::{}` is a constant Rust puts on a primitive type, and the port writes that \
             type as a JavaScript primitive, which has no members and no spelling for this one",
            prim.rust_name(),
            konst
        )),
    })
}
