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
//! A regular-expression literal is modelled too, because not modelling it does
//! not merely add a name: a quote inside one — `const r = /"/;` — opened a
//! string that swallowed the rest of the file, and every name after it was
//! lost. Whether a `/` opens one is decided the way JavaScript decides it, from
//! what stands before it: after a name, a number or a closing bracket it is
//! division, and everywhere else it opens a literal.

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
        if c == '/' && opens_a_regex(previous) {
            at = past_regex(source, at);
            previous = Some('x');
            continue;
        }
        // `...xs` spreads `xs`, and the run of dots is ONE operator: read a dot
        // at a time, the name after it was a member read and the file imported
        // nothing for it.
        if c == '.' && source.get(at + 1) == Some(&'.') && source.get(at + 2) == Some(&'.') {
            at += 3;
            previous = Some('…');
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
            // A member read imports nothing, and neither does an object
            // literal's KEY: `.match({ Comparison: (v) => .. })` names the
            // ARM, not a type, and `core/type_resolver.ts` imported a
            // `Comparison` from `./lineage` — an unrelated private class — on
            // the strength of one. A ternary's `a ? b : c` is not this: `b`'s
            // predecessor is `?`.
            let is_key = matches!(previous, Some('{') | Some(',')) && next_is_colon(source, at);
            if previous != Some('.') && !is_key {
                out.insert(source[from..at].iter().collect());
            }
            // S9: what a name IS decides what a `/` after it means. A KEYWORD
            // is not a value, so `return /"/;` opens a regular-expression
            // literal where `x /2/ y` is two divisions — and reading every name
            // as a value swallowed the rest of the file from the first `/`
            // after a `return`. The category travels, not the character.
            let word: String = source[from..at].iter().collect();
            previous = Some(match opens_a_regex_after(&word) {
                true => 'k',
                false => 'x',
            });
            continue;
        }
        previous = Some(c);
        at += 1;
    }
}

/// Is the next code character a `:`? What makes a name a key rather than a read.
fn next_is_colon(source: &[char], from: usize) -> bool {
    source[from..].iter().find(|c| !c.is_whitespace()) == Some(&':')
}

/// Can a `/` here open a regular-expression literal? JavaScript decides this
/// from what stands before it: after a name, a number or a closing bracket a
/// `/` is division, and everywhere else it opens a literal. `previous` is the
/// last CODE character, so a string or a template counts as a value.
fn opens_a_regex(previous: Option<char>) -> bool {
    !matches!(previous, Some('x') | Some('0') | Some(')') | Some(']') | Some('}') | Some('\'')
        | Some('"') | Some('`'))
}

/// Is this word a KEYWORD after which a `/` opens a literal rather than
/// dividing? S9: the ones that are followed by an expression.
///
/// The emitter writes no regular-expression literal today, so this half is
/// modelled and the other half — a `/` after a value — is what the corpus
/// exercises. Both are here because the lexer is read by the import gate, and a
/// gate that mis-lexes one file reports the wrong names for it.
fn opens_a_regex_after(word: &str) -> bool {
    matches!(
        word,
        "return"
            | "throw"
            | "case"
            | "yield"
            | "await"
            | "typeof"
            | "instanceof"
            | "in"
            | "of"
            | "new"
            | "delete"
            | "void"
            | "do"
            | "else"
    )
}

/// Where the regular-expression literal that opens at `at` ends, one past its
/// flags. A `/` inside a `[…]` class does not close it, and neither does an
/// escaped one.
fn past_regex(source: &[char], at: usize) -> usize {
    let mut i = at + 1;
    let mut in_class = false;
    while i < source.len() {
        match source[i] {
            '\\' => i += 2,
            '[' => {
                in_class = true;
                i += 1;
            }
            ']' if in_class => {
                in_class = false;
                i += 1;
            }
            '\n' => return i,
            '/' if !in_class => {
                i += 1;
                while i < source.len() && source[i].is_ascii_alphabetic() {
                    i += 1;
                }
                return i;
            }
            _ => i += 1,
        }
    }
    source.len()
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
            // A comment inside the hole is not code, and a `}` written in one
            // closed the hole early: `` `a ${/* } */ Needed.make()}` `` lost
            // `Needed` and everything after it in that template.
            '/' if source.get(i + 1) == Some(&'/') => {
                while i < source.len() && source[i] != '\n' {
                    i += 1;
                }
            }
            '/' if source.get(i + 1) == Some(&'*') => {
                i += 2;
                while i + 1 < source.len() && !(source[i] == '*' && source[i + 1] == '/') {
                    i += 1;
                }
                i = (i + 2).min(source.len());
            }
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

    /// H5: an object literal's KEY is not a name the file writes.
    ///
    /// `.match({ Comparison: (v) => .. })` names the ARM the runtime dispatches
    /// on. Read as a name, `core/type_resolver.ts` imported a `Comparison` from
    /// `./lineage` — an unrelated private class the module does not export —
    /// and several of the import gate's unexported rows came from keys like it.
    #[test]
    fn an_object_literals_key_is_not_a_read() {
        let found = names("v.match({ Comparison: (a) => a.left, True: () => Yes })");
        assert!(!found.contains(&"Comparison".to_string()), "{:?}", found);
        assert!(!found.contains(&"True".to_string()), "{:?}", found);
        // The VALUE beside a key still is one, and so is a shorthand.
        assert!(found.contains(&"Yes".to_string()), "{:?}", found);
        assert!(names("{ Shorthand }").contains(&"Shorthand".to_string()));
        // A ternary is not this: `Wanted`'s predecessor is `?`.
        assert!(names("c ? Wanted : Other").contains(&"Wanted".to_string()));
        // And a type literal's key is a key while its type is a read.
        let typed = names("type V = { Some: { _0: Token } }");
        assert!(!typed.contains(&"Some".to_string()), "{:?}", typed);
        assert!(typed.contains(&"Token".to_string()), "{:?}", typed);
    }

    /// H6: a name written only as a spread operand is a name the file writes.
    ///
    /// Read a dot at a time, `xs` in `[...xs]` was a member read and the file
    /// imported nothing for it.
    #[test]
    fn a_spread_operand_is_a_read() {
        assert!(names("const all = [...Wanted];").contains(&"Wanted".to_string()));
        assert!(names("f(...Args)").contains(&"Args".to_string()));
        // And an ordinary member read is still one.
        assert!(!names("holder.Wanted").contains(&"Wanted".to_string()));
    }

    /// I7: a quote inside a regular-expression literal is not a string.
    ///
    /// Taken as one, it opened a string that swallowed the rest of the file and
    /// every name after it was lost — which drops a name the file has to
    /// import, not merely adds one.
    #[test]
    fn a_regular_expression_literal_is_not_code_and_does_not_open_a_string() {
        let found = names("const r = /\"/; Needed.make();");
        assert!(found.contains(&"Needed".to_string()), "{:?}", found);
        // A `/` inside a character class does not close the literal.
        assert!(names("const r = /[/\"]/g; Needed.make();").contains(&"Needed".to_string()));
        // And a division is still a division: the name after it is read.
        assert!(names("const half = total / Divisor;").contains(&"Divisor".to_string()));
        assert!(names("const q = xs[0] / Divisor;").contains(&"Divisor".to_string()));
    }

    /// I7: a comment inside a template hole does not close the hole.
    #[test]
    fn a_comment_inside_a_template_hole_does_not_close_it() {
        let found = names("`a ${/* } */ Needed.make()}`");
        assert!(found.contains(&"Needed".to_string()), "{:?}", found);
        let line = names("`a ${\n  // }\n  Needed.make()\n}`");
        assert!(line.contains(&"Needed".to_string()), "{:?}", line);
    }

    /// S9: a `/` after a KEYWORD opens a regular-expression literal.
    ///
    /// Read as division, `return /"/;` left the lexer inside a string from the
    /// quote onwards and the rest of the file was never read — so a gate that
    /// asks this lexer which names a file writes got the wrong answer for it.
    #[test]
    fn a_slash_after_a_keyword_opens_a_literal() {
        let found = names("return /\"/;\nNeeded.make();\n");
        assert!(found.contains(&"Needed".to_string()), "the rest of the file was read: {found:?}");
    }

    /// And a `/` after a VALUE is still division.
    #[test]
    fn a_slash_after_a_value_is_still_division() {
        let found = names("const q = total / Scale.of(2);\n");
        assert!(found.contains(&"Scale".to_string()), "{found:?}");
        assert!(found.contains(&"total".to_string()), "{found:?}");
    }

    /// A name that merely BEGINS with a keyword is not one.
    #[test]
    fn a_name_beginning_with_a_keyword_is_not_a_keyword() {
        let found = names("const returned = a / b / Held.of(1);\n");
        assert!(found.contains(&"Held".to_string()), "{found:?}");
    }
}

/// The names a file's own top-level items are emitted under.
///
/// For: a name the file DECLARES must never be imported. A base helper and a
/// corpus function can be called the same thing — `filter_owned` is a name
/// either side may reasonably pick — and importing one the file declares is
/// `TS2440: Import declaration conflicts with local declaration`, which no gate
/// was asking about. The cross-module import list has always excluded a file's
/// own functions; the `@ankurah/base` list had not.
///
/// A struct, an enum and a trait keep the name Rust wrote; a function and a
/// const are emitted under the port's spelling of it.
pub(crate) fn names_the_file_declares(
    file: &crate::types::RustFile,
) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for s in &file.structs {
        out.insert(s.name.clone());
    }
    for e in &file.enums {
        out.insert(e.name.clone());
    }
    for t in &file.traits {
        out.insert(t.name.clone());
    }
    for f in &file.functions {
        out.insert(crate::name_map::escape_reserved(&crate::name_map::to_camel_case(&f.name)));
    }
    for c in &file.consts {
        out.insert(crate::name_map::escape_reserved(&c.name));
    }
    out
}

#[cfg(test)]
mod declares_tests {
    /// A name the file DECLARES is never imported: `core/task.rs` declares
    /// `spawn`, which `@ankurah/base` also exports, and the emitted file both
    /// imported and declared it. `tsc` calls that `TS2440` and the import gate
    /// was not asking.
    #[test]
    fn a_function_the_file_declares_keeps_its_name_to_itself() {
        let mut f = crate::testing::Fixture::build(&[(
            "lib.rs",
            "pub fn spawn<F>(future: F) { let _ = future; }\n\
             pub fn filter_owned(xs: Vec<u32>) -> Vec<u32> { xs }\n",
        )]);
        let ts = f.emitted("lib.rs");
        assert!(
            !ts.contains("from '@ankurah/base'"),
            "neither name is imported into the file that declares it:\n{}",
            ts
        );
        assert!(ts.contains("export function spawn"), "{}", ts);
        assert!(ts.contains("export function filterOwned"), "{}", ts);
    }
}
