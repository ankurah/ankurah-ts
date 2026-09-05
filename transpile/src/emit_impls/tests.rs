//! What an impl with no class of its own is called, and what it emits.

use crate::testing::Fixture;

/// The functions one file's impls contribute, by name.
fn names(c: &Fixture, file: &str) -> Vec<String> {
    let module = c.module(file);
    let entry = c.files.iter().find(|e| e.path == file).expect("file");
    super::free_functions(&c.reg, module, &entry.file)
        .into_iter()
        .map(|f| f.name)
        .collect()
}

#[test]
fn a_blanket_impl_takes_the_method_name_alone() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Listener;\n\
         pub trait IntoListener { fn into_listener(self) -> Listener; }\n\
         impl<F> IntoListener for F where F: Fn(u32) {\n\
             fn into_listener(self) -> Listener { Listener }\n\
         }",
    )]);
    assert_eq!(names(&c, "lib.rs"), vec!["intoListener".to_string()]);
}

#[test]
fn an_impl_on_a_system_wrapper_names_its_constructors_outside_in() {
    let c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Inner<T> { pub value: T }\n\
         pub trait Observer { fn observe(&self); }\n\
         impl<T> Observer for Arc<Inner<T>> { fn observe(&self) {} }",
    )]);
    assert_eq!(names(&c, "lib.rs"), vec!["Arc_Inner_observe".to_string()]);
}

#[test]
fn two_callable_impls_of_one_shape_are_told_apart_by_arity() {
    // `Arc<dyn Fn(T)>` and `Arc<dyn Fn()>` differ in nothing a name would
    // otherwise catch, and both are written for the same trait, so the number
    // of arguments the callable takes is part of the name.
    let c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Listener;\n\
         pub trait IntoListener { fn into_listener(self) -> Listener; }\n\
         impl IntoListener for Arc<dyn Fn(u32)> {\n\
             fn into_listener(self) -> Listener { Listener }\n\
         }\n\
         impl IntoListener for Arc<dyn Fn()> {\n\
             fn into_listener(self) -> Listener { Listener }\n\
         }",
    )]);
    assert_eq!(
        names(&c, "lib.rs"),
        vec![
            "Arc_Fn1_intoListener".to_string(),
            "Arc_Fn0_intoListener".to_string()
        ]
    );
}

#[test]
fn an_impl_on_a_crate_struct_stays_a_method_on_its_class() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct S;\npub trait T { fn go(&self); }\nimpl T for S { fn go(&self) {} }",
    )]);
    assert!(names(&c, "lib.rs").is_empty());
}

#[test]
fn the_receiver_is_the_functions_first_parameter() {
    let c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Inner<T> { pub value: T }\n\
         pub trait Observer { fn observe(&self, tag: u32); }\n\
         impl<T> Observer for Arc<Inner<T>> { fn observe(&self, tag: u32) {} }",
    )]);
    let module = c.module("lib.rs");
    let entry = c.files.iter().find(|e| e.path == "lib.rs").expect("file");
    let emitted = super::free_functions(&c.reg, module, &entry.file);
    assert!(
        emitted[0]
            .text
            .contains("export function Arc_Inner_observe<T>(self: Arc<Inner<T>>, tag: number)"),
        "the receiver comes first, under the name Rust gave it:\n{}",
        emitted[0].text
    );
}

/// R8: a contested conversion is named by the RUST source type. Read through
/// the TypeScript spelling, `i64`, `i32` and `f64` were `bigint`, `number` and
/// `number`, so three impls looked like two and one was never emitted;
/// `bincode::Error`, `anyhow::Error` and a crate `Error` all spelled `Error`,
/// and two of the three bodies were lost.
#[test]
fn a_contested_conversion_is_named_by_its_rust_source() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub enum QV { S(String), I(i64), F(f64) }\n\
         impl From<String> for QV { fn from(s: String) -> Self { QV::S(s) } }\n\
         impl From<&str> for QV { fn from(s: &str) -> Self { QV::S(s.to_string()) } }\n\
         impl From<i64> for QV { fn from(i: i64) -> Self { QV::I(i) } }\n\
         impl From<i32> for QV { fn from(i: i32) -> Self { QV::I(i as i64) } }\n\
         impl From<f64> for QV { fn from(x: f64) -> Self { QV::F(x) } }\n",
    )]);
    let ts = f.emitted("lib.rs");
    for named in ["static fromI64(", "static fromI32(", "static fromF64("] {
        assert!(ts.contains(named), "{named} is missing:\n{ts}");
    }
    // `str` is `String`'s borrowed form and the port writes both as `string`,
    // so those two are one conversion — and one name, not two methods with one
    // body.
    assert_eq!(ts.matches("static fromString(").count(), 1, "{ts}");
    assert!(!ts.contains("static fromStr("), "{ts}");
    assert!(!ts.contains("static fromNumber("), "no name comes from the spelling:\n{ts}");
}

/// Two sources that share a leaf are told apart by the module in front of it.
#[test]
fn two_sources_sharing_a_leaf_keep_the_qualifier() {
    let mut f = Fixture::build(&[
        ("lib.rs", "pub mod first;\npub mod second;\npub struct Wrapped;\n\
                    impl From<first::Error> for Wrapped { fn from(e: first::Error) -> Self { Wrapped } }\n\
                    impl From<second::Error> for Wrapped { fn from(e: second::Error) -> Self { Wrapped } }\n"),
        ("first.rs", "pub struct Error;\n"),
        ("second.rs", "pub struct Error;\n"),
    ]);
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("static fromFirstError("), "{ts}");
    assert!(ts.contains("static fromSecondError("), "{ts}");
    assert!(!ts.contains("static fromError("), "{ts}");
}

/// A1.5: the same decision names the module-level function an impl with no
/// class of its own becomes. It used to compute a second name that did not see
/// the contest, so an owned conversion and a borrowed one collapsed onto one
/// symbol only one of them was emitted under.
#[test]
fn a_free_function_reads_the_same_naming() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag { pub n: u32 }\n\
         impl From<Tag> for String { fn from(t: Tag) -> String { String::new() } }\n\
         impl From<&Tag> for String { fn from(t: &Tag) -> String { String::new() } }\n",
    )]);
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("export function String_fromTag("), "{ts}");
    assert!(ts.contains("export function String_fromRefTag("), "{ts}");
}

/// R12: a method whose emitted name is taken is dropped, and every call to it
/// goes to the other one — but a method carrying a HOLE is the engine refusing a
/// shape it cannot write, and dropping it drops the refusal. `Property::
/// from_value` for `Json` collided with `From<serde_json::Value>` on
/// `fromValue`; the diagnostic was filed and `Json.fromValue` answered
/// `new Json(value)` where Rust answers `Err(PropertyError::Missing)`.
#[test]
fn a_dropped_method_carrying_a_hole_is_written_under_its_trait() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Held { pub n: u32 }\n\
         pub enum Slot { Held(Held), Empty }\n\
         pub struct Box2(pub u32);\n\
         pub trait Take { fn take_from(slot: Option<Slot>) -> Result<Box2, String>; }\n\
         impl Box2 {\n\
           pub fn take_from(n: u32) -> Box2 { Box2(n) }\n\
         }\n\
         impl Take for Box2 {\n\
           fn take_from(slot: Option<Slot>) -> Result<Box2, String> {\n\
             match slot {\n\
               Some(Slot::Held(h)) => Ok(Box2(h.n)),\n\
               _ => Err(\"empty\".to_string()),\n\
             }\n\
           }\n\
         }",
    )]);
    let ts = f.emitted("lib.rs");
    // The inherent method keeps the name every call site writes.
    assert!(ts.contains("static takeFrom(n: number): Box2"), "{}", ts);
    // and the trait's, whose body the engine refused, is written under the
    // trait rather than dropped with its refusal inside it.
    assert!(ts.contains("static Take_takeFrom("), "{}", ts);
    assert!(ts.contains("unsupported("), "{}", ts);
    assert!(
        f.messages().iter().any(|m| m.contains("this one carries a hole")),
        "and it says what it did and what it did not do: {:?}",
        f.messages()
    );
}
