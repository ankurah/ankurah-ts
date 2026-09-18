//! What a branch join says about the unknowns the branches stand on.

use crate::testing::Fixture;

/// A join the solve cannot meet is one disagreement, not one per later use:
/// read through the settled table it had no unknown to take back, so the first
/// branch's width stood and every constraint after it said the same thing
/// again.
#[test]
fn a_join_that_cannot_be_met_is_said_once_and_leaves_the_local_untyped() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub fn read(v: &Vec<usize>) -> usize { v.len() }\n\
         pub fn join(flag: bool) -> usize {\n    \
             let mut xs = Vec::new();\n    \
             xs.push(1u32);\n    \
             let ys = if flag { xs } else { vec![2usize] };\n    \
             read(&ys)\n\
         }",
    )]);
    let _ = c.emitted("lib.rs");
    let said = c.messages();
    assert_eq!(
        said.iter().filter(|m| m.contains("constrained to be the same type")).count(),
        1,
        "{said:?}"
    );
    assert!(
        said.iter().any(|m| m.contains("`ys` has a type the engine cannot say owns anything")),
        "{said:?}"
    );
}

/// An `if let`'s then-branch reads the pattern's OWN names. A name the pattern
/// shadows is a different value, and reading the outer one reported valid Rust
/// as contradicting itself.
#[test]
fn an_if_lets_branch_reads_the_names_its_pattern_binds() {
    let mut c = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag { pub n: u32 }\n\
         pub enum Slot { Full(Tag), Empty }\n\
         pub fn read(t: &Tag) -> u32 { t.n }\n\
         pub fn pick(slot: Slot, value: u32) -> u32 {\n    \
             if let Slot::Full(value) = slot { read(&value) + 1 } else { value }\n\
         }",
    )]);
    let _ = c.emitted("lib.rs");
    assert!(
        !c.messages().iter().any(|m| m.contains("constrained to be the same type")),
        "{:?}",
        c.messages()
    );
}

/// A block written as an expression says what it is only where its tail can see
/// the names the block itself wrote above it.
#[test]
fn a_blocks_own_lets_are_in_scope_when_its_tail_is_read() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Tag { pub n: u32 }\n\
         pub fn hold() -> Vec<Tag> { Vec::new() }\n\
         pub fn count() -> usize { let held = { let tags = hold(); tags.len() }; held }",
    )]);
    let cx = c.context("lib.rs", None);
    let block: syn::Block =
        syn::parse_str("{ let tags = hold(); tags.len() }").expect("a block");
    let tail = syn::Expr::Block(syn::ExprBlock { attrs: Vec::new(), label: None, block });
    assert_eq!(cx.resolve_expr(&tail).ok(), Some(crate::ty::Ty::Prim(crate::ty::Prim::Usize)));
}
