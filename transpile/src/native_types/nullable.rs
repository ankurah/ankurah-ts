//! Option<T> → T | null method translations
//!
//! Option maps to nullable (T | null) in TS, so "methods" on Option
//! become syntax-level operations, not method calls.

use super::MethodTranslation;

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let result = match method {
        // unwrap/expect/unwrap_or/unwrap_or_else handled in body.rs before dispatch.

        // Null checks
        "is_some" => format!("{} != null", receiver),
        "is_none" => format!("{} == null", receiver),

        // Map — apply function if non-null
        "map" if args.len() == 1 => format!("{} != null ? ({})({}!) : null", receiver, args[0], receiver),

        // `Option<&T>::cloned` and `::copied` turn a borrow of the payload into
        // an owned one. A JavaScript value is neither, so the nullable is
        // already what they produce. (What `cloned` does to the payload itself
        // — an `Arc` refcount, say — is the same question `iter().cloned()`
        // raises and is answered the same way: it does not.)
        "cloned" | "copied" if args.is_empty() => receiver.to_string(),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
