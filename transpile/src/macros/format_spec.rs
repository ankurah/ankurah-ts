//! Rust format strings, read rather than expanded.
//!
//! For: every formatting macro in the corpus — `format!`, `write!`, `panic!`,
//! the assertions, the tracing calls and thiserror's `#[error("..")]` — carries
//! the same little language in its first argument, and the port has to render
//! the same text from it. Reading that language once, here, is what keeps the
//! eight callers from each inventing their own half of it.
//!
//! This file only *reads*. It says which argument each placeholder names, which
//! formatting trait it asks for, and what else the spec demanded; turning that
//! into TypeScript is `format_emit.rs`'s job, because that needs the body's
//! translator and this does not.
//!
//! The grammar is the one in `std::fmt`:
//!
//! ```text
//! format := '{' [ argument ] [ ':' spec ] '}'
//! argument := integer | identifier
//! spec := [[fill] align] [sign] ['#'] ['0'] [width] ['.' precision] type
//! type := '' | '?' | 'x?' | 'X?' | 'x' | 'X' | 'o' | 'b' | 'e' | 'E' | 'p'
//! ```

/// Which argument a placeholder names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgRef {
    /// `{}` — the next argument not yet taken by a `{}`.
    Next,
    /// `{0}` — the argument at this position.
    Positional(usize),
    /// `{name}` — the argument written `name = ..`, or the variable `name`
    /// itself where the macro's argument list does not name it.
    Named(String),
}

/// The formatting trait a placeholder asks for. Rust picks the impl by this
/// letter, and the port has to pick the same rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmtTrait {
    /// `{}` — `Display`.
    Display,
    /// `{:?}` — `Debug`.
    Debug,
    /// `{:x}`, `{:X}`, `{:o}`, `{:b}`, `{:e}`, `{:E}`, `{:p}`, `{:x?}`, `{:X?}`.
    /// The letter travels so the diagnostic can name it.
    Other(&'static str),
}

/// What a placeholder asked for beyond naming its argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub fmt_trait: FmtTrait,
    /// `#` — `{:#?}` is Debug's pretty form and `{:#}` is Display's alternate.
    pub alternate: bool,
    /// `{:>8}`, `{:*^5}` — the fill character and the alignment.
    pub fill_align: Option<(char, char)>,
    /// `{:+}`.
    pub sign: Option<char>,
    /// `{:08}` — zero padding, which also implies a width.
    pub zero_pad: bool,
    /// `{:8}`, `{:width$}`, `{:1$}` — as written.
    pub width: Option<String>,
    /// `{:.3}`, `{:.*}`, `{:.prec$}` — as written, without the dot.
    pub precision: Option<String>,
    /// The spec exactly as written, for the diagnostic that names it.
    pub written: String,
}

impl Spec {
    /// The plain cases: `{}` and `{:?}`, with nothing else asked of them.
    ///
    /// Everything else changes the text Rust produces, and the port either
    /// carries it or says it did not.
    #[cfg(test)]
    pub fn is_plain(&self) -> bool {
        self.fill_align.is_none()
            && self.sign.is_none()
            && !self.zero_pad
            && self.width.is_none()
            && self.precision.is_none()
            && matches!(self.fmt_trait, FmtTrait::Display | FmtTrait::Debug)
    }

    /// What this spec asks for that the port does not carry, named for a reader.
    ///
    /// The list is empty where `is_plain` holds. `alternate` is not in it: the
    /// two traits handle their own alternate form, and a caller that cannot
    /// says so itself with the type in hand.
    pub fn unsupported(&self) -> Vec<String> {
        let mut asked = Vec::new();
        if let Some((fill, align)) = self.fill_align {
            asked.push(format!("alignment `{}{}`", fill, align));
        }
        if let Some(sign) = self.sign {
            asked.push(format!("sign `{}`", sign));
        }
        if self.zero_pad {
            asked.push("zero padding".to_string());
        }
        if let Some(width) = &self.width {
            asked.push(format!("width `{}`", width));
        }
        if let Some(precision) = &self.precision {
            asked.push(format!("precision `.{}`", precision));
        }
        if let FmtTrait::Other(letter) = self.fmt_trait {
            asked.push(format!("the `{}` formatting trait", letter));
        }
        asked
    }
}

/// One piece of a format string: literal text, or a placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// Literal text, with `{{` and `}}` already reduced to `{` and `}`.
    Text(String),
    Arg { which: ArgRef, spec: Spec },
}

/// Read a format string into its pieces.
///
/// A string this cannot read comes back as the reason, and the caller reports
/// that rather than emitting a guess: a misread format string is silently wrong
/// output, which is the one thing worse than no output.
pub fn parse(fmt: &str) -> Result<Vec<Piece>, String> {
    let mut pieces = Vec::new();
    let mut text = String::new();
    let mut chars = fmt.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                text.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                text.push('}');
            }
            '}' => return Err("a `}` that opens nothing".to_string()),
            '{' => {
                if !text.is_empty() {
                    pieces.push(Piece::Text(std::mem::take(&mut text)));
                }
                let mut inner = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == '}' {
                        closed = true;
                        break;
                    }
                    inner.push(c);
                }
                if !closed {
                    return Err("a `{` that nothing closes".to_string());
                }
                let (which, spec) = parse_placeholder(&inner)?;
                pieces.push(Piece::Arg { which, spec });
            }
            _ => text.push(c),
        }
    }
    if !text.is_empty() {
        pieces.push(Piece::Text(text));
    }
    Ok(pieces)
}

/// The inside of one `{..}`: which argument, and what was asked of it.
fn parse_placeholder(inner: &str) -> Result<(ArgRef, Spec), String> {
    let (argument, spec_text) = match inner.find(':') {
        Some(at) => (&inner[..at], &inner[at + 1..]),
        None => (inner, ""),
    };
    let which = if argument.is_empty() {
        ArgRef::Next
    } else if let Ok(index) = argument.parse::<usize>() {
        ArgRef::Positional(index)
    } else if is_identifier(argument) {
        ArgRef::Named(argument.to_string())
    } else {
        return Err(format!("`{{{}}}` names no argument the engine can read", inner));
    };
    Ok((which, parse_spec(spec_text)))
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// The part after the `:`.
///
/// Read left to right in the order `std::fmt` documents, so that a `<` in the
/// fill position is an alignment and not a stray character, and a `0` after the
/// sign is zero padding rather than the first digit of the width.
fn parse_spec(text: &str) -> Spec {
    let written = text.to_string();
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0;

    // [[fill] align] — the two-character form first, so `*^` reads as a fill of
    // `*` and not as an alignment of `^` with `*` left over.
    let mut fill_align = None;
    if chars.len() >= 2 && matches!(chars[1], '<' | '^' | '>') {
        fill_align = Some((chars[0], chars[1]));
        at = 2;
    } else if !chars.is_empty() && matches!(chars[0], '<' | '^' | '>') {
        fill_align = Some((' ', chars[0]));
        at = 1;
    }

    let mut sign = None;
    if matches!(chars.get(at), Some('+') | Some('-')) {
        sign = Some(chars[at]);
        at += 1;
    }

    let mut alternate = false;
    if chars.get(at) == Some(&'#') {
        alternate = true;
        at += 1;
    }

    let mut zero_pad = false;
    if chars.get(at) == Some(&'0') {
        zero_pad = true;
        at += 1;
    }

    // The trailing type is what is left after the width and the precision, so
    // the width is read up to a `.` or up to the type letters.
    let rest: String = chars[at..].iter().collect();
    let (count_part, type_part) = split_type(&rest);

    let (width, precision) = match count_part.find('.') {
        Some(dot) => {
            let w = &count_part[..dot];
            let p = &count_part[dot + 1..];
            (
                (!w.is_empty()).then(|| w.to_string()),
                Some(p.to_string()),
            )
        }
        None => (
            (!count_part.is_empty()).then(|| count_part.to_string()),
            None,
        ),
    };

    let fmt_trait = match type_part {
        "" => FmtTrait::Display,
        "?" => FmtTrait::Debug,
        "x?" => FmtTrait::Other("x?"),
        "X?" => FmtTrait::Other("X?"),
        "x" => FmtTrait::Other("x"),
        "X" => FmtTrait::Other("X"),
        "o" => FmtTrait::Other("o"),
        "b" => FmtTrait::Other("b"),
        "e" => FmtTrait::Other("e"),
        "E" => FmtTrait::Other("E"),
        "p" => FmtTrait::Other("p"),
        _ => FmtTrait::Other("an unknown one"),
    };

    Spec { fmt_trait, alternate, fill_align, sign, zero_pad, width, precision, written }
}

/// Split the tail of a spec into its count part and its trailing type.
///
/// `8.3` has no type, `.3?` ends in Debug, and `width$` ends in nothing — the
/// `$` belongs to the count. Only the letters `std::fmt` lists are types, so a
/// `x` inside `max$` stays where it is.
fn split_type(rest: &str) -> (&str, &str) {
    for candidate in ["x?", "X?"] {
        if let Some(head) = rest.strip_suffix(candidate) {
            return (head, &rest[head.len()..]);
        }
    }
    if let Some(last) = rest.chars().last() {
        if matches!(last, '?' | 'x' | 'X' | 'o' | 'b' | 'e' | 'E' | 'p') {
            // A `$` before it makes it part of a named count (`{:x$}`), not a type.
            let head = &rest[..rest.len() - last.len_utf8()];
            if !head.ends_with('$') {
                return (head, &rest[head.len()..]);
            }
        }
    }
    (rest, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(fmt: &str) -> Vec<Piece> {
        parse(fmt).expect("parses")
    }

    #[test]
    fn plain_text_is_one_piece() {
        assert_eq!(args("Empty expression"), vec![Piece::Text("Empty expression".into())]);
    }

    #[test]
    fn positional_and_named() {
        let pieces = args("Expected {expected}, got {0}");
        assert_eq!(pieces.len(), 4);
        assert!(matches!(&pieces[1], Piece::Arg { which: ArgRef::Named(n), .. } if n == "expected"));
        assert!(matches!(&pieces[3], Piece::Arg { which: ArgRef::Positional(0), .. }));
    }

    #[test]
    fn next_argument_is_distinct_from_position_zero() {
        let pieces = args("{} {}");
        assert!(matches!(&pieces[0], Piece::Arg { which: ArgRef::Next, .. }));
        assert!(matches!(&pieces[2], Piece::Arg { which: ArgRef::Next, .. }));
    }

    #[test]
    fn braces_are_escaped_in_pairs() {
        assert_eq!(args("{{}}"), vec![Piece::Text("{}".into())]);
        assert_eq!(args("a {{ b"), vec![Piece::Text("a { b".into())]);
    }

    #[test]
    fn debug_and_alternate_debug() {
        let Piece::Arg { spec, .. } = &args("{:?}")[0] else { panic!("an argument") };
        assert_eq!(spec.fmt_trait, FmtTrait::Debug);
        assert!(!spec.alternate);
        assert!(spec.is_plain());

        let Piece::Arg { spec, .. } = &args("{:#?}")[0] else { panic!("an argument") };
        assert_eq!(spec.fmt_trait, FmtTrait::Debug);
        assert!(spec.alternate);
    }

    #[test]
    fn named_debug_keeps_its_name() {
        let Piece::Arg { which, spec } = &args("{got:?}")[0] else { panic!("an argument") };
        assert_eq!(*which, ArgRef::Named("got".into()));
        assert_eq!(spec.fmt_trait, FmtTrait::Debug);
    }

    #[test]
    fn width_precision_and_alignment_are_read_and_named() {
        let Piece::Arg { spec, .. } = &args("{:>8}")[0] else { panic!("an argument") };
        assert_eq!(spec.fill_align, Some((' ', '>')));
        assert_eq!(spec.width.as_deref(), Some("8"));
        assert!(!spec.is_plain());
        assert_eq!(spec.unsupported(), vec!["alignment ` >`", "width `8`"]);

        let Piece::Arg { spec, .. } = &args("{:.3}")[0] else { panic!("an argument") };
        assert_eq!(spec.precision.as_deref(), Some("3"));
        assert_eq!(spec.unsupported(), vec!["precision `.3`"]);

        let Piece::Arg { spec, .. } = &args("{:08x}")[0] else { panic!("an argument") };
        assert!(spec.zero_pad);
        assert_eq!(spec.width.as_deref(), Some("8"));
        assert_eq!(spec.fmt_trait, FmtTrait::Other("x"));

        let Piece::Arg { spec, .. } = &args("{:*^5}")[0] else { panic!("an argument") };
        assert_eq!(spec.fill_align, Some(('*', '^')));
        assert_eq!(spec.width.as_deref(), Some("5"));
    }

    #[test]
    fn a_named_width_is_not_a_type_letter() {
        let Piece::Arg { spec, .. } = &args("{:x$}")[0] else { panic!("an argument") };
        assert_eq!(spec.fmt_trait, FmtTrait::Display);
        assert_eq!(spec.width.as_deref(), Some("x$"));
    }

    #[test]
    fn unbalanced_braces_are_refused_rather_than_guessed() {
        assert!(parse("a { b").is_err());
        assert!(parse("a } b").is_err());
    }
}
