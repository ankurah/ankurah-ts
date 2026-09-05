//! Names the emitter chooses, and the places it cannot choose one at all.
//!
//! Every name the emitter writes has to miss every name the Rust already used
//! in the same scope. Where it does not, the emitted file does not run: a
//! redeclared `const` is a TDZ `ReferenceError`, not a warning.

use crate::testing::Fixture;

fn emitted(src: &str) -> String {
    let mut fixture = Fixture::build(&[("lib.rs", src)]);
    fixture.emitted("lib.rs")
}

const PRELUDE: &str = "pub struct Oops;\n\
                       pub enum Choice { A(u32), B }\n\
                       pub fn fallible(n: u32) -> Result<u32, Oops> { Ok(n) }\n";

/// D14: the arm of a runtime `match` is an arrow whose parameter carries the
/// payload, and the arm's own body may declare a name of its own. Written as
/// `v` beside a `const v`, the arrow threw before it read either.
#[test]
fn the_arm_parameter_misses_a_name_the_arm_declares() {
    let ts = emitted(&format!(
        "{PRELUDE}pub fn run(c: Choice) -> Result<u32, Oops> {{\n\
           match c {{\n\
             Choice::A(n) => {{ let v = fallible(n)?; Ok(v + 1) }}\n\
             Choice::B => Ok(0),\n\
           }}\n\
         }}"
    ));
    assert!(ts.contains("A: (_v) =>"), "the parameter steps around `v`:\n{ts}");
    assert!(ts.contains("const n = _v._0;"), "and the payload comes out of it:\n{ts}");
    assert!(ts.contains("const v = _r0.unwrap();"), "the Rust name is unchanged:\n{ts}");
}

/// Where nothing collides, the parameter keeps the name a reader of the port
/// expects.
#[test]
fn the_arm_parameter_is_v_where_nothing_takes_it() {
    let ts = emitted(&format!(
        "{PRELUDE}pub fn run(c: Choice) -> u32 {{\n\
           match c {{ Choice::A(n) => n, Choice::B => 0 }}\n\
         }}"
    ));
    assert!(ts.contains("A: (v) =>"), "{ts}");
}

/// D11: a `&mut T` whose `T` the port writes as a VALUE is a cell, and only a
/// LOCAL can be held in one. `&mut c.n` hands the callee a copy of the number,
/// so the write reaches nobody — `port/ownership.md` said this was reported and
/// it was not.
#[test]
fn a_mutable_borrow_of_a_place_that_is_not_a_local_is_a_hole() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        "pub struct Counter { pub n: u32 }\n\
         pub fn bump(n: &mut u32) { *n += 1; }\n\
         pub fn bump_field(c: &mut Counter) { bump(&mut c.n); }\n\
         pub fn bump_local() -> u32 { let mut x = 1u32; bump(&mut x); x }",
    )]);
    let ts = fixture.emitted("lib.rs");
    assert!(
        ts.contains("bump(unsupported("),
        "the field place stops there rather than updating a copy:\n{ts}"
    );
    assert!(
        fixture
            .messages()
            .iter()
            .any(|m| m.contains("borrows a place that is not a local")),
        "and the site says so: {:?}",
        fixture.messages()
    );
    // A local is the case the cell rule covers, and it is untouched.
    assert!(ts.contains("const x = new BorrowMut(1);"), "{ts}");
    assert!(ts.contains("bump(x);"), "{ts}");
}
