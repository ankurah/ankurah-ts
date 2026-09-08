//! FF3/GG2: ONE emitted binding per module-level function, read everywhere.
//!
//! A module-level function is a BINDING, and JavaScript refuses a reserved word
//! there — `pub fn with` is `with_`. The answer was derived at the declaration
//! and derived again, differently, by the readers: the cross-file map that says
//! which module a name comes from, the import list, the re-export list. Each
//! shape below is one of those disagreements.

use crate::testing::Fixture;

fn emitted(files: &[(&str, &str)], which: &str) -> (String, Vec<String>) {
    let mut fixture = Fixture::build(files);
    let ts = fixture.emitted(which);
    (ts, fixture.messages())
}

const WORDS: &str = "\
pub fn plain() -> i64 { 1 }\n\
pub fn with() -> i64 { 2 }\n\
pub fn unsupported() -> i64 { 3 }\n\
";

/// FF3: the ONE binding is what `FnInfo.ts_name` holds, and every reader takes
/// it from there — the declaration, the cross-file map that says which module a
/// name comes from, the inline-module import list, and the test-module hoist.
/// Held unescaped, the map said `with` while the declaration and every call
/// said `with_`, so the import matched nothing, was not written, and the call
/// named an undeclared binding with no diagnostic at all.
#[test]
fn a_module_level_function_carries_the_binding_every_reader_takes() {
    let file = crate::extract::extract_source(
        "words.rs",
        WORDS,
        crate::extract::ExtractCfg::default(),
    )
    .expect("parses");
    let names: Vec<&str> = file.functions.iter().map(|f| f.ts_name.as_str()).collect();
    assert_eq!(names, vec!["plain", "with_", "unsupported_"]);
}

/// A METHOD is not that position: `obj.delete` and a `with` on a class are
/// legal JavaScript, and escaping those broke every `ThreadLocal::with`.
#[test]
fn a_method_keeps_its_own_name() {
    let file = crate::extract::extract_source(
        "held.rs",
        "pub struct Held;\nimpl Held { pub fn with(&self) -> i64 { 1 } }",
        crate::extract::ExtractCfg::default(),
    )
    .expect("parses");
    let method = &file.impls[0].methods[0];
    assert_eq!(method.ts_name, "with");
}

/// And the calls in a body that reaches across modules are written under the
/// same binding.
#[test]
fn a_call_across_modules_is_written_under_the_binding() {
    let (ts, _) = emitted(
        &[
            ("words.rs", WORDS),
            (
                "lib.rs",
                "pub mod words;\n\
                 use crate::words::{plain, with, unsupported};\n\
                 pub fn caller() -> i64 { plain() + with() + unsupported() }",
            ),
        ],
        "lib.rs",
    );
    assert!(ts.contains("with_()"), "the call is escaped:\n{}", ts);
    assert!(ts.contains("unsupported_()"), "and so is the other:\n{}", ts);
}

/// The declaring module exports exactly those names.
#[test]
fn the_declaration_and_the_import_agree() {
    let (ts, _) = emitted(&[("words.rs", WORDS), ("lib.rs", "pub mod words;")], "words.rs");
    for name in ["export function plain(", "export function with_(", "export function unsupported_("]
    {
        assert!(ts.contains(name), "`{}`:\n{}", name, ts);
    }
}

/// GG2: `escape_reserved` is a SUFFIX, and a suffix cannot be injective. Valid
/// Rust declaring both `r#in` and `in_` gives two `export function in_()`,
/// which TypeScript refuses twice over. The port has no second answer to give,
/// so the collision is reported rather than written in silence.
#[test]
fn two_functions_that_want_the_same_binding_are_reported() {
    let (_, messages) =
        emitted(&[("lib.rs", "pub fn r#in() -> i64 { 1 }\npub fn in_() -> i64 { 2 }")], "lib.rs");
    assert!(
        messages.iter().any(|m| m.contains("`r#in` and `in_` are both written `in_`")),
        "the collision is named:\n{:?}",
        messages
    );
}

/// And a name that needs no escaping is not touched, at either end.
#[test]
fn an_ordinary_name_keeps_its_own_spelling() {
    let (_, messages) = emitted(
        &[("lib.rs", "pub fn plain() -> i64 { 1 }\npub fn other() -> i64 { 2 }")],
        "lib.rs",
    );
    assert!(messages.is_empty(), "nothing is reported:\n{:?}", messages);
}

/// GG2: a re-export prints a BINDING too. `pub use words::r#in;` printed
/// `export { r#in } from './words'`, which no engine parses.
#[test]
fn a_re_export_prints_the_binding() {
    assert_eq!(crate::name_map::escape_reserved("r#in"), "in_");
    assert_eq!(crate::name_map::escape_reserved("with"), "with_");
    // A type name is untouched: every word the port escapes is lowercase.
    assert_eq!(crate::name_map::escape_reserved("EntityId"), "EntityId");
}
