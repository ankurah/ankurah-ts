//! Type-erased conversion methods — apply to any receiver type.
//!
//! These are Rust trait methods that have no runtime equivalent in TS:
//! .into(), .from(), .as_ref(), .as_mut() — all identity transforms.

pub fn translate(receiver: &str, method: &str, _args: &[String]) -> Option<String> {
    match method {
        // to_owned() — Rust's ToOwned trait, equivalent to clone for owned types
        "to_owned" | "toOwned" => Some(format!("{}.clone()", receiver)),

        // Formatter::alternate() — TS has no alternate formatting flag
        "alternate" => Some("false".to_string()),

        _ => None,
    }
}
