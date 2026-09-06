//! Which identifiers an emitted file WRITES — the question every import list
//! is really asking.
//!
//! For: a file has to import exactly the names it writes and does not declare.
//! The import lists used to answer that by splitting the rendered text on
//! non-word characters, which reads the inside of a string literal as if it
//! were code. The `collect` refusal names the iterator types it could not
//! build — "`collect` into `Collect<FilterMap<TopKStream<Iter<IntoIter>>, Fut,
//! F>, C>` is a `FromIterator` the port has no construction for" — so
//! `storage-indexeddb/collection.ts` imported `Iter`, `SortedStream` and
//! `TopKStream` from `@ankurah/core`, which exports none of the three, and the
//! module failed to load. The parse gate could not see it: it builds each file
//! with `--external '*'`, which makes every specifier somebody else's problem.
//!
//! The same answer settles the other direction. A `use` the source wrote for a
//! name the emission never writes — `use ankurah_signals::Peek;` inside a body
//! whose lowering does not reach it, `use ankql::ast::OrderDirection;` inside a
//! test module — used to be imported anyway, because the cross-crate list read
//! the `use` statements and asked nothing about the file.
//!
//! So the scan lexes rather than splits: a string literal, a template
//! literal's text and a comment contribute nothing; a template literal's
//! `${..}` holes are code and do contribute; a numeric literal is a number and
//! not the name `xFF`; and a name after a `.` is a member read, which the file
//! imports nothing for.
//!
//! One shape is deliberately not modelled: a regular-expression literal, whose
//! body would be lexed as code. The emitter writes none — every `/…/` in the
//! validation copy is in a hand-written test — and reading one would only add
//! a name, never drop one.

use std::collections::BTreeSet;

/// Every identifier this emitted TypeScript writes as a name of its own.
pub(crate) fn written_names(text: &str) -> BTreeSet<String> {
    let source: Vec<char> = text.chars().collect();
    let mut out = BTreeSet::new();
    scan(&source, &mut out);
    out
}

fn scan(source: &[char], out: &mut BTreeSet<String>) {
    let mut at = 0usize;
    // The last character that was CODE, so that a name after a `.` is read as
    // the member it is. Whitespace does not move it: `value\n  .clone()` is one
    // member read.
    let mut previous: Option<char> = None;
    while at < source.len() {
        let c = source[at];
        if c.is_whitespace() {
            at += 1;
            continue;
        }
        if c == '/' && source.get(at + 1) == Some(&'/') {
            while at < source.len() && source[at] != '\n' {
                at += 1;
            }
            continue;
        }
        if c == '/' && source.get(at + 1) == Some(&'*') {
            at += 2;
            while at + 1 < source.len() && !(source[at] == '*' && source[at + 1] == '/') {
                at += 1;
            }
            at = (at + 2).min(source.len());
            continue;
        }
        if c == '\'' || c == '"' {
            at = past_quoted(source, at, c);
            previous = Some(c);
            continue;
        }
        if c == '`' {
            at = past_template(source, at, out);
            previous = Some('`');
            continue;
        }
        if c.is_ascii_digit() {
            at = past_number(source, at);
            previous = Some('0');
            continue;
        }
        if is_name_start(c) {
            let from = at;
            while at < source.len() && is_name_part(source[at]) {
                at += 1;
            }
            if previous != Some('.') {
                out.insert(source[from..at].iter().collect());
            }
            previous = Some('x');
            continue;
        }
        previous = Some(c);
        at += 1;
    }
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_name_part(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Where the `'` or `"` string that opens at `at` ends, one past its closing
/// quote.
fn past_quoted(source: &[char], at: usize, quote: char) -> usize {
    let mut i = at + 1;
    while i < source.len() {
        match source[i] {
            '\\' => i += 2,
            c if c == quote => return i + 1,
            _ => i += 1,
        }
    }
    source.len()
}

/// Where the template literal that opens at `at` ends, collecting the names
/// its `${..}` holes write — those are code, and the text around them is not.
fn past_template(source: &[char], at: usize, out: &mut BTreeSet<String>) -> usize {
    let mut i = at + 1;
    while i < source.len() {
        match source[i] {
            '\\' => i += 2,
            '`' => return i + 1,
            '$' if source.get(i + 1) == Some(&'{') => {
                let from = i + 2;
                let to = past_hole(source, from);
                scan(&source[from..to], out);
                i = (to + 1).min(source.len());
            }
            _ => i += 1,
        }
    }
    source.len()
}

/// Where the `${` hole that opens at `from` closes — its own braces, strings
/// and nested templates counted, so an object literal inside one does not end
/// it early.
fn past_hole(source: &[char], from: usize) -> usize {
    let mut depth = 0usize;
    let mut i = from;
    while i < source.len() {
        match source[i] {
            '{' => {
                depth += 1;
                i += 1;
            }
            '}' if depth == 0 => return i,
            '}' => {
                depth -= 1;
                i += 1;
            }
            '\'' | '"' => i = past_quoted(source, i, source[i]),
            // The nested template's own names are collected when `scan` reads
            // the hole; here it only has to be stepped over.
            '`' => i = past_template(source, i, &mut BTreeSet::new()),
            _ => i += 1,
        }
    }
    source.len()
}

/// Where the numeric literal starting at `at` ends. Without this `0xFF` reads
/// as the name `xFF` and `1e5` as `e5`.
fn past_number(source: &[char], at: usize) -> usize {
    let mut i = at;
    if source[i] == '0' && matches!(source.get(i + 1), Some('x' | 'X' | 'b' | 'B' | 'o' | 'O')) {
        i += 2;
    }
    while i < source.len() {
        let c = source[i];
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            i += 1;
        } else if (c == '+' || c == '-')
            && matches!(i.checked_sub(1).and_then(|p| source.get(p)), Some('e' | 'E'))
        {
            i += 1;
        } else {
            break;
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::written_names;

    fn names(text: &str) -> Vec<String> {
        written_names(text).into_iter().collect()
    }

    #[test]
    fn a_name_inside_a_string_literal_is_not_a_name_the_file_writes() {
        // K1: the `collect` refusal names the iterator types it could not
        // build, and storage-indexeddb imported three of them from a package
        // that exports none.
        let emitted = "return await unsupported('`collect` into \
                       `Collect<FilterMap<TopKStream<Iter<IntoIter>>, Fut, F>, C>` is a \
                       `FromIterator` the port has no construction for');";
        let written = names(emitted);
        for absent in ["Iter", "SortedStream", "TopKStream", "Collect", "FilterMap"] {
            assert!(!written.contains(&absent.to_string()), "{absent} came out of a string literal: {written:?}");
        }
        assert!(written.contains(&"unsupported".to_string()));
    }

    #[test]
    fn a_double_quoted_string_and_an_escaped_quote_are_both_skipped() {
        let written = names(r#"const m = "a Value \" Iter"; const n = 'it\'s Peek'; use(Real);"#);
        assert!(!written.contains(&"Iter".to_string()), "{written:?}");
        assert!(!written.contains(&"Peek".to_string()), "{written:?}");
        assert!(written.contains(&"Real".to_string()), "{written:?}");
    }

    #[test]
    fn a_template_literal_writes_only_what_its_holes_hold() {
        let written = names("`Entity ${entity.id} of Kind ${Kind.of(x)}`");
        // `Entity` and the word `Kind` before the hole are TEXT; the `Kind`
        // inside a hole is code, and that is the one that comes back.
        assert!(!written.contains(&"Entity".to_string()), "{written:?}");
        assert!(written.contains(&"entity".to_string()), "{written:?}");
        assert!(written.contains(&"Kind".to_string()), "{written:?}");
        assert!(written.contains(&"x".to_string()), "{written:?}");
        // `id` and `of` are member reads, which import nothing.
        assert!(!written.contains(&"id".to_string()), "{written:?}");
        assert!(!written.contains(&"of".to_string()), "{written:?}");
    }

    #[test]
    fn a_hole_holding_an_object_literal_does_not_end_the_template_early() {
        let written = names("`a ${ f({ k: Inner }) } b Outer`");
        assert!(written.contains(&"Inner".to_string()), "{written:?}");
        assert!(!written.contains(&"Outer".to_string()), "{written:?}");
    }

    #[test]
    fn a_comment_writes_nothing() {
        let written = names("// a Peek of the Value\nconst a = 1; /* Iter */ const b = Real;");
        assert!(!written.contains(&"Peek".to_string()), "{written:?}");
        assert!(!written.contains(&"Iter".to_string()), "{written:?}");
        assert!(written.contains(&"Real".to_string()), "{written:?}");
    }

    #[test]
    fn a_member_read_is_not_a_name_to_import() {
        let written = names("tokio.mpsc.channel(); value\n  .clone(); a?.Peek;");
        assert!(written.contains(&"tokio".to_string()), "{written:?}");
        assert!(!written.contains(&"mpsc".to_string()), "{written:?}");
        assert!(!written.contains(&"clone".to_string()), "{written:?}");
        assert!(!written.contains(&"Peek".to_string()), "{written:?}");
    }

    #[test]
    fn a_numeric_literal_is_a_number_and_not_a_name() {
        let written = names("const a = 0xFF; const b = 1e5; const c = 12n; use(Real);");
        assert!(!written.contains(&"xFF".to_string()), "{written:?}");
        assert!(!written.contains(&"e5".to_string()), "{written:?}");
        assert!(!written.contains(&"n".to_string()), "{written:?}");
        assert!(written.contains(&"Real".to_string()), "{written:?}");
    }

    #[test]
    fn a_whole_name_and_not_a_substring() {
        // The old scan matched `Mutex` inside `AsyncMutex` and `Ref` inside
        // `RefCell`, which imported std's for a file that only wrote tokio's.
        let written = names("let m: AsyncMutex<void>; let r: RefCell<number>;");
        assert!(written.contains(&"AsyncMutex".to_string()), "{written:?}");
        assert!(!written.contains(&"Mutex".to_string()), "{written:?}");
        assert!(written.contains(&"RefCell".to_string()), "{written:?}");
        assert!(!written.contains(&"Ref".to_string()), "{written:?}");
    }
}
