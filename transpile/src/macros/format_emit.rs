//! Rust format strings rendered as TypeScript text.
//!
//! For: the eight formatting macros and thiserror's `#[error("..")]` all have to
//! produce the same text the Rust program printed, and they differ only in where
//! their arguments come from. This file holds the part that is the same — the
//! walk over the pieces, the escaping, the choice between Display and Debug —
//! and takes the part that differs as an `Operands`.
//!
//! What comes out is one TypeScript expression of type `string`.

use crate::macros::format_spec::{ArgRef, FmtTrait, Piece, Spec};
use crate::ty::Ty;

/// Where a placeholder's argument comes from, and what the engine says it is.
///
/// A macro reads its arguments from the invocation; thiserror reads them from
/// the variant's fields. Both answer the same two questions, so both implement
/// this and the walk below is written once.
pub trait Operands {
    /// The TypeScript expression for this argument, and its Rust type where the
    /// engine could name one. `None` means there is no such argument, and the
    /// caller says so rather than emitting a placeholder for nothing.
    ///
    /// `needs_type` is false for a plain `{}`: TypeScript's string conversion
    /// renders that placeholder whatever the value turns out to be, so asking
    /// the engine would file a diagnostic about a gap that changes no output.
    /// `{:?}` and `{:#}` do depend on the type, and there it is asked for.
    fn operand(&mut self, which: &ArgRef, needs_type: bool) -> Option<(String, Option<Ty>)>;

    /// How this value renders under `Display`. The default is the value itself:
    /// TypeScript's string conversion calls `toString`, which is where the port
    /// puts every `impl Display`.
    fn display(&mut self, expr: &str, _ty: Option<&Ty>, _alternate: bool) -> String {
        expr.to_string()
    }

    /// How this value renders under `Debug`.
    fn debug(&mut self, expr: &str, ty: Option<&Ty>, alternate: bool) -> String;

    /// Say that this format string asked for something the port does not carry.
    /// The text is emitted anyway, without whatever was asked for.
    fn report(&mut self, what: String);
}

/// Render the pieces of a format string as one TypeScript string expression.
pub fn render(pieces: &[Piece], operands: &mut dyn Operands) -> String {
    // A format string with no placeholders is a string literal, which is what a
    // hand port writes and what reads back as the message it is.
    if pieces.iter().all(|p| matches!(p, Piece::Text(_))) {
        let text: String = pieces
            .iter()
            .map(|p| match p {
                Piece::Text(t) => t.as_str(),
                Piece::Arg { .. } => unreachable!("checked just above"),
            })
            .collect();
        return quoted(&text);
    }

    let mut out = String::from("`");
    for piece in pieces {
        match piece {
            Piece::Text(text) => out.push_str(&escape_template(text)),
            Piece::Arg { which, spec } => {
                out.push_str("${");
                out.push_str(&one_argument(which, spec, operands));
                out.push('}');
            }
        }
    }
    out.push('`');
    out
}

/// One placeholder's TypeScript expression.
fn one_argument(which: &ArgRef, spec: &Spec, operands: &mut dyn Operands) -> String {
    let needs_type = matches!(spec.fmt_trait, FmtTrait::Debug) || spec.alternate;
    let Some((expr, ty)) = operands.operand(which, needs_type) else {
        operands.report(format!(
            "the format string names {}, and the call has no such argument, so \
             the placeholder is written as `undefined`",
            name_of(which)
        ));
        return "undefined".to_string();
    };

    let unsupported = spec.unsupported();
    if !unsupported.is_empty() {
        operands.report(format!(
            "the placeholder for {} asks for {}, which the port does not carry, \
             so the value is written without it",
            name_of(which),
            unsupported.join(" and ")
        ));
    }

    match spec.fmt_trait {
        FmtTrait::Debug => operands.debug(&expr, ty.as_ref(), spec.alternate),
        // A trait the port has no rendering for still has to put the value
        // somewhere, and Display is the rendering every value has.
        FmtTrait::Display | FmtTrait::Other(_) => {
            operands.display(&expr, ty.as_ref(), spec.alternate)
        }
    }
}

fn name_of(which: &ArgRef) -> String {
    match which {
        ArgRef::Next => "the next argument".to_string(),
        ArgRef::Positional(index) => format!("argument {}", index),
        ArgRef::Named(name) => format!("`{}`", name),
    }
}

/// Literal text inside a template literal.
///
/// A backtick would end the literal, a `${` would open a substitution, and a
/// backslash would start an escape — all three have to be written as escapes.
/// The line breaks are escaped too: a real one is the same character, but it
/// puts the emitted expression on two lines and the port's output is read.
pub fn escape_template(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push_str("\\\\"),
            '`' => out.push_str("\\`"),
            '$' if chars.peek() == Some(&'{') => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

/// A TypeScript single-quoted string literal holding this text.
pub fn quoted(text: &str) -> String {
    let mut out = String::from("'");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::format_spec::parse;

    /// Arguments given positionally, with no types and no rendering of their
    /// own — enough to test the walk, the escaping and the reports.
    struct Fixed {
        args: Vec<String>,
        next: usize,
        reports: Vec<String>,
    }

    impl Fixed {
        fn new(args: &[&str]) -> Fixed {
            Fixed { args: args.iter().map(|s| s.to_string()).collect(), next: 0, reports: Vec::new() }
        }
    }

    impl Operands for Fixed {
        fn operand(&mut self, which: &ArgRef, _needs_type: bool) -> Option<(String, Option<Ty>)> {
            let at = match which {
                ArgRef::Next => {
                    let at = self.next;
                    self.next += 1;
                    at
                }
                ArgRef::Positional(index) => *index,
                ArgRef::Named(name) => return Some((format!("v.{}", name), None)),
            };
            self.args.get(at).map(|a| (a.clone(), None))
        }
        fn debug(&mut self, expr: &str, _ty: Option<&Ty>, _alternate: bool) -> String {
            format!("dbg({})", expr)
        }
        fn report(&mut self, what: String) {
            self.reports.push(what);
        }
    }

    fn render_with(fmt: &str, args: &[&str]) -> (String, Vec<String>) {
        let pieces = parse(fmt).expect("parses");
        let mut operands = Fixed::new(args);
        let text = render(&pieces, &mut operands);
        (text, operands.reports)
    }

    #[test]
    fn a_string_with_no_placeholders_is_a_string_literal() {
        assert_eq!(render_with("Empty expression", &[]).0, "'Empty expression'");
    }

    #[test]
    fn positional_and_named_placeholders() {
        assert_eq!(
            render_with("Expected {expected}, got {0}", &["v._0"]).0,
            "`Expected ${v.expected}, got ${v._0}`"
        );
    }

    #[test]
    fn the_next_argument_advances() {
        assert_eq!(render_with("{} then {}", &["a", "b"]).0, "`${a} then ${b}`");
    }

    #[test]
    fn debug_goes_through_the_debug_rendering() {
        assert_eq!(render_with("got {0:?}", &["v._0"]).0, "`got ${dbg(v._0)}`");
    }

    #[test]
    fn escaped_braces_survive_as_text() {
        assert_eq!(render_with("{{{0}}}", &["a"]).0, "`{${a}}`");
    }

    #[test]
    fn backticks_and_substitutions_in_the_text_are_escaped() {
        let (text, _) = render_with("invalid variant `{given}` for `{ty}`", &[]);
        assert_eq!(text, "`invalid variant \\`${v.given}\\` for \\`${v.ty}\\``");
        // A `$` the format string ends a text run with is not an escape: what
        // follows it in the emitted literal is the substitution, and `$${a}` is
        // a literal `$` then that substitution.
        assert_eq!(render_with("cost ${0}", &["a"]).0, "`cost $${a}`");
        // A `${` inside the text is one, and would open a substitution.
        assert_eq!(render_with("cost ${{}}", &[]).0, "'cost ${}'");
    }

    #[test]
    fn a_missing_argument_is_reported_rather_than_guessed() {
        let (text, reports) = render_with("{0} and {1}", &["a"]);
        assert_eq!(text, "`${a} and ${undefined}`");
        assert_eq!(reports.len(), 1);
        assert!(reports[0].contains("argument 1"));
    }

    #[test]
    fn width_is_reported_and_the_value_written_without_it() {
        let (text, reports) = render_with("{0:>8}", &["a"]);
        assert_eq!(text, "`${a}`");
        assert_eq!(reports.len(), 1);
        assert!(reports[0].contains("alignment"), "{}", reports[0]);
        assert!(reports[0].contains("width `8`"), "{}", reports[0]);
    }

    #[test]
    fn a_hex_placeholder_is_reported_and_falls_to_display() {
        let (text, reports) = render_with("{0:x}", &["a"]);
        assert_eq!(text, "`${a}`");
        assert!(reports[0].contains("`x` formatting trait"), "{}", reports[0]);
    }

    #[test]
    fn quoting_escapes_what_a_single_quoted_literal_cannot_hold() {
        assert_eq!(quoted("it's\n"), "'it\\'s\\n'");
    }
}
