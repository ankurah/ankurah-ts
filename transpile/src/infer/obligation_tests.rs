//! What becomes of an answer the impl table gave on a bound nothing had
//! settled, once the solve settles it.

use crate::testing::Fixture;
use crate::ty::Ty;
use syn::spanned::Spanned;

/// A source with one `IntoIterator` impl, gated on a bound only some types
/// meet, and a body that settles the source's own argument below the loop.
fn gated_source(settles_to: &str) -> String {
    format!(
        "pub trait Mark {{}}\n\
         pub struct Good;\n\
         impl Mark for Good {{}}\n\
         pub struct Bad;\n\
         pub struct Tag {{ pub n: u32 }}\n\
         pub struct Source<T> {{ pub items: Vec<Tag>, pub held: Option<T> }}\n\
         impl<T> Source<T> {{\n    \
             pub fn new() -> Source<T> {{ Source {{ items: Vec::new(), held: None }} }}\n    \
             pub fn set(&mut self, held: T) {{ self.held = Some(held); }}\n\
         }}\n\
         impl<'a, T: Mark> IntoIterator for &'a Source<T> {{\n    \
             type Item = &'a Tag;\n    \
             type IntoIter = std::slice::Iter<'a, Tag>;\n    \
             fn into_iter(self) -> Self::IntoIter {{ self.items.iter() }}\n\
         }}\n\
         pub fn walk() -> usize {{\n    \
             let mut source = Source::new();\n    \
             let mut seen = Vec::new();\n    \
             for value in &source {{ seen.push(value); }}\n    \
             source.set({settles_to});\n    \
             seen.len()\n\
         }}"
    )
}

/// The same source with nothing settling its argument, so the bound is neither
/// proved nor ruled out and the item the loop was handed is a type of its own.
fn unsettled_source() -> String {
    gated_source("Bad").replace("    source.set(Bad);\n", "")
}

#[test]
fn an_answer_left_standing_on_an_unsettled_bound_says_so_once() {
    let mut c = Fixture::build(&[("lib.rs", &unsettled_source())]);
    let _ = c.emitted("lib.rs");
    // `&Tag` names no unknown, so the body reads as if the bound held and
    // nothing else anywhere says what the answer rests on.
    assert_eq!(
        c.messages()
            .iter()
            .filter(|m| m.contains("which nothing in this body settles"))
            .count(),
        1,
        "{:?}",
        c.messages()
    );
}

#[test]
fn an_answer_read_through_a_bound_the_solve_falsifies_is_withdrawn() {
    let mut c = Fixture::build(&[("lib.rs", &gated_source("Bad"))]);
    let _ = c.emitted("lib.rs");
    assert_eq!(
        c.messages()
            .iter()
            .filter(|m| m.contains("was read through an impl that requires"))
            .count(),
        1,
        "{:?}",
        c.messages()
    );
}

#[test]
fn an_answer_read_through_a_bound_the_solve_proves_stands() {
    let mut c = Fixture::build(&[("lib.rs", &gated_source("Good"))]);
    let _ = c.emitted("lib.rs");
    assert!(
        !c.messages()
            .iter()
            .any(|m| m.contains("was read through an impl that requires")),
        "{:?}",
        c.messages()
    );
}

#[test]
fn what_took_a_withdrawn_answer_stands_for_nothing() {
    let c = Fixture::build(&[("lib.rs", &gated_source("Bad"))]);
    let mut cx = c.context("lib.rs", None);
    cx.push_fn(vec![]);
    let block: syn::Block = syn::parse_str(
        "{ let mut source = Source::new();\n  \
           let mut seen = Vec::new();\n  \
           for value in &source { seen.push(value); }\n  \
           source.set(Bad); }",
    )
    .unwrap();
    cx.collect_constraints(&block, None);
    let element = match &block.stmts[1] {
        syn::Stmt::Local(local) => {
            let at = local.init.as_ref().unwrap().expr.span().start();
            let table = cx.vars.borrow();
            Ty::Var(table.site(at.line, at.column, 0).expect("a variable stands at this site"))
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(
        cx.solved(&element),
        element,
        "the collection kept the withdrawn item type"
    );
}
