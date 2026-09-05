//! `std::cmp::Ordering` → the number `-1 | 0 | 1`.
//!
//! For: `Ord` is what a sort, a `min`, a `max` and every tie-break run on, and
//! the port had no spelling for the answer at all — `Ordering::Greater` came
//! out as `undefined /* Ordering */.Greater` and `.then_with(..)` as a method
//! call on a number. The derive already writes `compareTo`, which answers a
//! number, so the number IS the ordering: `-1` before, `0` equal, `1` after,
//! which is also what `Array.prototype.sort` takes and what every emitted
//! `compareTo` returns.
//!
//! A wrapper class would have bought a `.then_with` that reads like Rust's and
//! cost a runtime type, an allocation per comparison, and a conversion at every
//! `sort` boundary. Everything `Ordering` declares is one small expression on a
//! number, so that is what is written.

use super::MethodTranslation;

/// The number a variant of `Ordering` is.
///
/// `std::sync::atomic::Ordering` shares the type's name and none of these
/// variants, so the variant name alone tells them apart.
pub fn variant(name: &str) -> Option<&'static str> {
    match name {
        "Less" => Some("-1"),
        "Equal" => Some("0"),
        "Greater" => Some("1"),
        _ => None,
    }
}

/// Is this the type whose values are those numbers?
pub fn is_ordering(reg: &crate::registry::TypeRegistry, ty: &crate::ty::Ty) -> bool {
    let Some(id) = ty.peel_refs().id() else { return false };
    reg.system_type("std::cmp::Ordering") == Some(id)
}

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let result = match method {
        "is_eq" => format!("{} === 0", receiver),
        "is_ne" => format!("{} !== 0", receiver),
        "is_lt" => format!("{} < 0", receiver),
        "is_le" => format!("{} <= 0", receiver),
        "is_gt" => format!("{} > 0", receiver),
        "is_ge" => format!("{} >= 0", receiver),
        "reverse" => format!("-({})", receiver),
        // `then` and `then_with` keep the first answer unless it is `Equal`.
        // The receiver is read twice, which is why it is read into a name
        // first — a `compareTo` call is not free and may not be pure.
        "then" if args.len() == 1 => {
            format!("(($c) => $c !== 0 ? $c : {})({})", args[0], receiver)
        }
        "then_with" if args.len() == 1 => {
            format!("(($c) => $c !== 0 ? $c : ({})())({})", args[0], receiver)
        }
        // Comparing two orderings is comparing two numbers, and `Ordering` is
        // `Copy`, so a clone is the value.
        "cmp" if args.len() == 1 => {
            format!("Math.sign({} - {})", receiver, args[0])
        }
        "clone" => receiver.to_string(),
        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
