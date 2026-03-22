//! Option<T> → T | null method translations
//!
//! Option maps to nullable (T | null) in TS, so "methods" on Option
//! become syntax-level operations, not method calls.

use super::MethodTranslation;

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let result = match method {
        // Unwrapping — identity (the value IS the inner type or null)
        "unwrap" | "expect" => receiver.to_string(),

        // Fallback values
        "unwrap_or" if args.len() == 1 => format!("{} ?? {}", receiver, args[0]),
        "unwrap_or_else" if args.len() == 1 => format!("{} ?? ({})()", receiver, args[0]),

        // Null checks
        "is_some" => format!("{} != null", receiver),
        "is_none" => format!("{} == null", receiver),

        // Map — apply function if non-null
        "map" if args.len() == 1 => format!("{} != null ? ({})({}!) : null", receiver, args[0], receiver),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
