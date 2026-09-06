//! Where an expectation reaches, and where it stops. G3.

use crate::testing::Fixture;

/// G3: an adaptor whose result type fixes its operand's payload is
/// transparent to an expectation. `let bytes: [u8; 32] =
/// id_bytes.try_into().map_err(..)?;` is the only thing that says which
/// `TryFrom` impl `try_into` picks, and Rust picks it by the TARGET type.
#[test]
fn an_expectation_reaches_through_map_err_and_a_question_mark() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Held { pub n: u32 }\n\
         impl std::convert::TryFrom<Vec<u8>> for Held {\n\
           type Error = String;\n\
           fn try_from(v: Vec<u8>) -> Result<Held, String> { Ok(Held { n: v.len() as u32 }) }\n\
         }\n\
         pub fn read(bytes: Vec<u8>) -> Result<Held, u32> {\n\
           let held: Held = bytes.try_into().map_err(|_| 0u32)?;\n\
           Ok(held)\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "read");
    assert!(
        ts.contains("Held_tryFromVecU8") || ts.contains("Held.tryFrom") || ts.contains("Held_try"),
        "the conversion the target names is written:\n{}",
        ts
    );
    assert!(!ts.contains(".tryInto()"), "not the name-based call:\n{}", ts);
}

/// And a form NOT on the list stops it, as every form used to: the list is
/// closed, and inference across a chain stays refused.
#[test]
fn an_expectation_stops_at_a_form_that_is_not_transparent() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Held { pub n: u32 }\n\
         impl std::convert::TryFrom<Vec<u8>> for Held {\n\
           type Error = String;\n\
           fn try_from(v: Vec<u8>) -> Result<Held, String> { Ok(Held { n: v.len() as u32 }) }\n\
         }\n\
         pub fn read(bytes: Vec<u8>) -> Option<Held> {\n\
           let held: Option<Held> = bytes.try_into().ok().filter(|h| h.n > 0);\n\
           held\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "read");
    assert!(ts.contains("tryInto"), "the expectation does not reach through `filter`:\n{}", ts);
}
