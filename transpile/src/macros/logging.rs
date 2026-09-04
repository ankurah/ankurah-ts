//! `tracing::{trace, debug, info, warn, error}!`, emitted as calls.
//!
//! For: a running program's log is how anybody finds out what it did, and the
//! port emitted these 83 sites as inert comments — so the ported program ran
//! silently where the Rust one narrated itself. Each becomes a call on the
//! runtime's `tracing` namespace, carrying the same rendered message.
//!
//! Only the message form is emitted. `tracing` also takes structured fields —
//! `warn!(peer = %id, "lost")` — which record typed values alongside the text;
//! the corpus writes none outside a feature-gated `#[instrument]`, and one that
//! appears later is reported rather than flattened into the message.

use proc_macro2::TokenStream;

use crate::body::BodyTranslator;

/// The five levels, by the name the macro is written with. `tracing::warn!` and
/// a bare `warn!` behind a `use tracing::warn` are the same macro, so the leaf
/// name is what decides.
pub fn level(name: &str) -> Option<&'static str> {
    Some(match name {
        "trace" => "trace",
        "debug" => "debug",
        "info" => "info",
        "warn" => "warn",
        "error" => "error",
        _ => return None,
    })
}

/// One tracing call: the level, and the message its format string renders.
pub fn call(level: &str, tokens: &TokenStream, t: &BodyTranslator, at: proc_macro2::Span) -> String {
    let Some(written) = super::format_call::written(tokens) else {
        // Structured fields, or an argument list that does not start with a
        // format string. Both carry information the message form loses.
        t.fallback(
            at,
            format!(
                "this `{}!` does not begin with a format string — tracing's structured fields are \
                 not carried over — so nothing is logged here",
                level
            ),
        );
        return format!("undefined /* {}!({}) */", level, tokens);
    };
    let message = super::format_call::format_string(&written, t, at)
        .unwrap_or_else(|| super::format_emit::quoted(&written.fmt.value()));
    format!("tracing.{}({})", level, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_five_levels_are_named_and_nothing_else_is() {
        assert_eq!(level("warn"), Some("warn"));
        assert_eq!(level("info"), Some("info"));
        assert_eq!(level("trace"), Some("trace"));
        assert_eq!(level("notice_info"), None);
        assert_eq!(level("println"), None);
    }
}
