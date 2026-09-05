//! What the JSON half writes, against what serde writes.
//!
//! The oracle for the four attributes is `serde-reference`, a Rust binary that
//! prints what `serde_json` does with the same declarations. Its answers are
//! quoted beside each test.

use crate::testing::Fixture;

fn built(src: &str) -> Fixture {
    Fixture::build(&[("lib.rs", src)])
}

const DERIVE: &str = "#[derive(Serialize, Deserialize)]\n";

/// serde refuses `{}` for a struct with a required field:
/// `missing_required_is_err=true`. The reader used to write `v as T`, which
/// checks nothing, so `{}` answered `Ok` with every field `undefined`.
#[test]
fn a_missing_required_field_is_an_error() {
    let mut f = built(&format!(
        "{}pub struct Required {{ pub text: String, pub count: i32 }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("missing field `text`"), "{}", ts);
    assert!(ts.contains("missing field `count`"), "{}", ts);
    // The cast is still written — after the check, which is what makes it
    // sound. What is gone is a cast with nothing in front of it.
    assert!(ts.contains("typeof v === 'string' ? Result.Ok(v as string)"), "{}", ts);
}

/// An `Option` field is the one case a missing key is not an error: serde reads
/// it as `None`.
#[test]
fn a_missing_option_field_is_none() {
    let mut f = built(&format!(
        "{}pub struct Holder {{ pub maybe: Option<String> }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(!ts.contains("missing field `maybe`"), "{}", ts);
    assert!(ts.contains("v == null ? Result.Ok(null)"), "{}", ts);
}

/// The primitive checks serde makes.
#[test]
fn a_primitive_is_checked_before_it_is_taken() {
    let mut f = built(&format!(
        "{}pub struct Row {{ pub text: String, pub flag: bool, pub n: i32 }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("typeof v === 'string'"), "{}", ts);
    assert!(ts.contains("typeof v === 'boolean'"), "{}", ts);
    assert!(ts.contains("typeof v === 'number'"), "{}", ts);
}

/// R3. serde_json keeps a `u64` token exactly: `wide_roundtrip_exact=true` for
/// `9007199254740993`. `Number(x)` on the way out and `BigInt(v as number)` on
/// the way in rounded it in both directions and could emit a token above
/// `u64::MAX` that Rust refuses to read.
#[test]
fn a_wide_integer_keeps_its_token() {
    let mut f = built(&format!(
        "{}pub struct Wide {{ pub unsigned: u64, pub signed: i64 }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(!ts.contains("Number(this.unsigned)"), "{}", ts);
    assert!(ts.contains("typeof v === 'bigint' && v >= 0n && v <= 18446744073709551615n"), "{}", ts);
    assert!(
        ts.contains("typeof v === 'bigint' && v >= -9223372036854775808n && v <= 9223372036854775807n"),
        "{}",
        ts
    );
    // A `number` wider than 2^53 has already lost digits, so it is refused
    // rather than converted: `BigInt` of it would invent the difference.
    assert!(ts.contains("Number.isSafeInteger(v)"), "{}", ts);
    // The value goes out as the bigint it is; `serde_json.stringify` writes the
    // bare integer token.
    assert!(ts.contains("'unsigned': this.unsigned"), "{}", ts);
}

/// #4: a reader typed by the TypeScript SPELLING accepted `1.5`, `-1` and `256`
/// for a `u8`, because all three are `typeof v === 'number'`. serde reads the
/// field by its RUST type, so the check comes from that.
#[test]
fn an_integer_field_is_read_by_its_rust_width() {
    let mut f = built(&format!(
        "{}pub struct Widths {{ pub byte: u8, pub small: i16, pub big: u32, pub size: usize }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("Number.isInteger(v) && v >= 0 && v <= 255"), "u8:\n{}", ts);
    assert!(ts.contains("v >= -32768 && v <= 32767"), "i16:\n{}", ts);
    assert!(ts.contains("v >= 0 && v <= 4294967295"), "u32:\n{}", ts);
    // R13: usize is 32-bit here, because the port's target is wasm32.
    assert_eq!(
        ts.matches("v >= 0 && v <= 4294967295").count(),
        2,
        "usize reads as a 32-bit unsigned:\n{}",
        ts
    );
    assert!(ts.contains("expected a u8"), "the message names the width:\n{}", ts);
}

/// A float field is not an integer field: serde reads any JSON number into an
/// `f64`, including one written without a fractional part.
#[test]
fn a_float_field_takes_any_number() {
    let mut f = built(&format!("{}pub struct Point {{ pub x: f64 }}", DERIVE));
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("typeof v === 'number' ? Result.Ok(v as number)"), "{}", ts);
    assert!(!ts.contains("Number.isInteger"), "{}", ts);
}

/// #2: the runtime `HashMap` a reader BUILDS is tracked whatever it holds, so a
/// document that fails on a later field has to release it. Taking the container's
/// ownership from its member left a partly decoded map unreleased.
///
/// PREMISE CHANGED 2026-09-05 (fixpass5 item 9, X14): the release used to be a
/// closure written into every `return` on the error paths, which covered an
/// expected `Err` and NOT an exception — a throwing property getter on a late
/// field left every earlier field with nobody. R4 says a decoder owns what it
/// has built until it RETURNS one, and a `finally` over a bag is the only form
/// that covers both, so what this test now pins is the bag and the flag.
#[test]
fn a_decoded_map_is_released_when_a_later_field_fails() {
    let mut f = built(&format!(
        "{}pub struct Holder {{ pub names: std::collections::HashMap<String, String>, pub count: u32 }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("$built.push(names);"), "the map goes into the bag:\n{}", ts);
    assert!(
        ts.contains("finally {\n      if (!$kept) dropOwned($built);"),
        "and the bag is released unless the reader handed one back:\n{}",
        ts
    );
    // The flag is set AFTER the value is built and before it is returned, so a
    // constructor that raised would still leave the fields to the `finally`.
    assert!(ts.contains("const $out = new Holder(names, count);\n      $kept = true;"), "{}", ts);
}

/// An EXCEPTION during a late field is the path a per-return closure could not
/// cover, and the reader with nothing to release writes neither the bag nor the
/// `finally`.
#[test]
fn a_reader_that_owns_nothing_writes_no_cleanup() {
    let mut f = built(&format!(
        "{}pub struct Plain {{ pub a: String, pub b: u8 }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(!ts.contains("$built"), "nothing to release:\n{}", ts);
    assert!(!ts.contains("$kept"), "nothing to release:\n{}", ts);
}

/// X12: a field named `value` declared `const value = _rvalue.unwrap();` in the
/// same block as the parameter `value`, so every read of the parameter above it
/// was `Cannot access 'value' before initialization` and the reader answered
/// `Err` for every document. The parameter takes a name none of the members
/// holds.
#[test]
fn a_field_named_value_does_not_shadow_the_readers_parameter() {
    let mut f = built(&format!(
        "{}pub struct Nested {{ pub value: String }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("static fromJson($value: unknown)"), "{}", ts);
    assert!(ts.contains("const value = _rvalue.unwrap();"), "{}", ts);
    // And a struct with no such field keeps the plain name.
    let mut g = built(&format!("{}pub struct Plain {{ pub a: String }}", DERIVE));
    let plain = g.emitted("lib.rs");
    assert!(plain.contains("static fromJson(value: unknown)"), "{}", plain);
}

/// X13: `#[serde(with = "json_as_bytes")]` handed its array straight to
/// `Uint8Array`, which truncates — `[305]` became byte 49 and the reader
/// answered `Ok`. serde reads each element as a `u8` before anything decodes
/// them, and both byte readers make the same test.
#[test]
fn the_json_as_bytes_module_checks_its_bytes() {
    let mut f = built(&format!(
        "mod json_as_bytes {{}}\n{}pub struct Doc {{ #[serde(with = \"json_as_bytes\")] pub body: String }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(
        ts.contains("v.every((b) => typeof b === 'number' && Number.isInteger(b) && b >= 0 && b <= 255)"),
        "the with-module reader checks its bytes:\n{}",
        ts
    );
}

/// `#[serde(transparent)]` on a NAMED struct: serde writes the one remaining
/// field alone. `transparent_json="id-1"` for `Ref<T>`.
#[test]
fn a_transparent_named_struct_is_its_one_field() {
    let mut f = built(&format!(
        "use std::marker::PhantomData;\n\
         {}#[serde(transparent)]\n\
         pub struct Ref<T> {{ pub id: String, #[serde(skip)] pub _phantom: PhantomData<T> }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("return this.id;"), "{}", ts);
    let reader = ts.split("static fromJson").nth(1).unwrap_or(&ts);
    assert!(
        !reader.contains("_phantom"),
        "the skipped field is in neither half:\n{}",
        reader
    );
    // And a static cannot name the class's own type parameters, so it declares
    // them: `static fromJson(value): Result<Ref<T>, JsonError>` is TS2302.
    assert!(ts.contains("static fromJson<T>(value: unknown)"), "{}", ts);
}

/// `#[serde(other)]`: serde yields that variant for a tag it does not know.
/// `unknown_is_other=true`. The bincode half has always read it and this one
/// ended every reader with an `Err`.
#[test]
fn an_unknown_tag_reads_as_the_other_variant() {
    let mut f = built(&format!(
        "{}pub enum Item {{ SysRoot, #[serde(other)] Other }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    let reader = ts.split("static fromJson").nth(1).unwrap_or(&ts);
    assert!(
        reader.contains("case 'SysRoot'"),
        "the known tag is a case:\n{}",
        reader
    );
    assert!(
        reader.contains("return Result.Ok(new Item('Other', {}));"),
        "an unknown tag reads as `Other`:\n{}",
        reader
    );
}

/// R4: the inner `Err` is handed straight out, never re-created, and every
/// owned field already built is released first.
#[test]
fn a_nested_failure_passes_out_and_releases_what_is_built() {
    let mut f = built(&format!(
        "{0}pub struct Inner {{ pub text: String }}\n\
         {0}pub struct Outer {{ pub first: Inner, pub second: Inner }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    let reader = ts.split("class Outer").nth(1).unwrap_or(&ts);
    assert!(
        reader.contains("Inner.fromJson(v)"),
        "the inner reader is called:\n{}",
        reader
    );
    assert!(
        reader.contains("$built.push(first);"),
        "the first field is released when the second fails:\n{}",
        reader
    );
    assert!(
        !reader.contains("throw r.unwrapErr()"),
        "no exception as control flow:\n{}",
        reader
    );
}

/// A `catch` that swallows an `OwnershipFatal` disarms the leak registry inside
/// every emitted reader. `port/ownership.md` says so; the 49 emitted catch
/// blocks did not.
///
/// PREMISE EXTENDED 2026-09-05 (fixpass4 item 7): an `UnsupportedShape` is
/// rethrown with it. That is what an R12 hole throws, and it says the ENGINE has
/// no lowering for a Rust shape — answering `Err` for one turns a loud refusal
/// into a silent wrong answer.
#[test]
fn the_catch_rethrows_an_ownership_fatal_and_an_unsupported_shape() {
    let mut f = built(&format!("{}pub struct Row {{ pub text: String }}", DERIVE));
    let ts = f.emitted("lib.rs");
    assert!(
        ts.contains("if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;"),
        "{}",
        ts
    );
    // and the name is imported, or the test the `catch` makes is a ReferenceError.
    assert!(ts.contains("UnsupportedShape") && ts.contains("from '@ankurah/base'"), "{}", ts);
}

/// A type whose JSON half was refused has no `fromJson`, and neither does
/// anything that contains one. Deciding from the CAPITAL LETTER put ten calls
/// in the corpus to a static no class declares.
#[test]
fn the_refusal_is_transitive() {
    let mut f = built(&format!(
        "use std::collections::HashMap;\n\
         {0}pub struct Refused {{ pub keyed: HashMap<u32, String> }}\n\
         {0}pub struct Holder {{ pub inner: Refused }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(
        !ts.contains("Refused.fromJson"),
        "nothing calls a static no class declares:\n{}",
        ts
    );
    assert!(
        f.messages().iter().any(|m| m.contains("has no `fromJson` in the port")),
        "{:?}",
        f.messages()
    );
}

/// A `Map<string, V>` and a tuple both have exact serde spellings, and both
/// used to be refused.
#[test]
fn a_string_keyed_map_and_a_tuple_have_json_spellings() {
    let mut f = built(&format!(
        "use std::collections::HashMap;\n\
         {}pub struct Row {{ pub keyed: HashMap<String, i32>, pub pair: (String, i32) }}",
        DERIVE
    ));
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("Object.fromEntries"), "{}", ts);
    assert!(ts.contains("new HashMap<string, number>"), "{}", ts);
    assert!(ts.contains("v.length === 2"), "a tuple's length is checked:\n{}", ts);
}
