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
    assert!(!ts.contains("BigInt(v as number)"), "{}", ts);
    assert!(ts.contains("typeof v === 'bigint' ? Result.Ok(v)"), "{}", ts);
    // The value goes out as the bigint it is; `serde_json.stringify` writes the
    // bare integer token.
    assert!(ts.contains("'unsigned': this.unsigned"), "{}", ts);
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
        reader.contains("dropOwned([first])"),
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
#[test]
fn the_catch_rethrows_an_ownership_fatal() {
    let mut f = built(&format!("{}pub struct Row {{ pub text: String }}", DERIVE));
    let ts = f.emitted("lib.rs");
    assert!(
        ts.contains("if (e instanceof OwnershipFatal) throw e;"),
        "{}",
        ts
    );
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
