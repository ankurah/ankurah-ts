//! How a written path becomes the name the emitted file uses: a module
//! qualifier dropped, a sibling crate's leaf, a static on a class, a local
//! under the identifier the translator had to give it.

use crate::testing::Fixture;

/// `use ankurah_proto as proto;` gives another crate a LOCAL name, and the
/// code below writes `proto::Presence`. The port flattens a crate into a
/// package, so the type is imported by its leaf; keeping the qualifier
/// emitted `new proto.Presence(..)` against a `proto` that exists nowhere
/// in the module. Live at `connectors/local-process/src/lib.rs:59`.
#[test]
fn a_crate_under_a_local_name_is_still_a_package() {
    let mut f = Fixture::build_with_siblings(
        "connector-local",
        &[(
            "lib.rs",
            "use ankurah_proto as proto;\n\
             pub struct Sender { pub id: usize }\n\
             impl Sender {\n\
               pub fn announce(&self) -> proto::Presence { proto::Presence { node_id: 1 } }\n\
             }",
        )],
        &[("ankurah_proto", &[("lib.rs", "pub struct Presence { pub node_id: usize }")])],
    );
    let ts = f.translated_method("lib.rs", "announce");
    assert!(!ts.contains("proto."), "the qualifier names nothing here:\n{}", ts);
    assert!(ts.contains("new Presence("), "{}", ts);
}

/// A temporary the translator needs must not take the name of a binding
/// that is live: `const _v2 = ..` written into a body that declared its own
/// `let _v2` shadows it for the rest of the block, and every later read of
/// the Rust name reads the temporary instead. The names are the shape the
/// translator hands out, so a body that happens to use them is exactly the
/// collision.
#[test]
fn a_temporary_does_not_shadow_a_binding_in_scope() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub fn find(n: usize) -> Option<usize> { Some(n) }\n\
         pub fn read(n: usize) -> usize {\n\
           let _v = 1;\n\
           let _v1 = 2;\n\
           let _v2 = 3;\n\
           let _v3 = 4;\n\
           let m = match find(n) { Some(x) => x, None => 0 };\n\
           _v + _v1 + _v2 + _v3 + m\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "read");
    for name in ["_v", "_v1", "_v2", "_v3"] {
        assert_eq!(
            ts.matches(&format!("const {} = ", name)).count(),
            1,
            "`{}` is declared twice — the match's temporary took a live binding's name:\n{}",
            name,
            ts
        );
    }
}

/// A path through one of this crate's OWN modules keeps no qualifier: the
/// port flattens the module tree into a package's exports, so `ast::Expr`
/// is the `Expr` the emitted file imports from `./ast` and
/// `parser::parse_selection` the `parseSelection` beside it. ankql's
/// `conversion.ts` wrote `new ast.Expr(..)` and `parser.parseSelection(..)`
/// against an `ast` and a `parser` that exist nowhere in the module.
#[test]
fn a_module_of_this_crate_is_not_a_name_in_the_emitted_file() {
    let mut f = Fixture::build_named(
        "testcrate",
        &[
            ("lib.rs", "pub mod ast;\npub mod parser;\npub mod use_it;"),
            ("ast.rs", "pub enum Literal { I64(i64) }\npub struct Path { pub n: usize }"),
            ("parser.rs", "pub fn parse_one(n: usize) -> usize { n }"),
            (
                "use_it.rs",
                "use crate::{ast, parser};\n\
                 pub fn make(v: i64) -> ast::Literal { ast::Literal::I64(v) }\n\
                 pub fn build(n: usize) -> ast::Path { ast::Path { n } }\n\
                 pub fn call(n: usize) -> usize { parser::parse_one(n) }",
            ),
        ],
    );
    let make = f.translated_method("use_it.rs", "make");
    assert!(make.contains("new Literal('I64', { _0: v })"), "{}", make);
    assert!(!make.contains("ast."), "{}", make);
    let build = f.translated_method("use_it.rs", "build");
    assert!(build.contains("new Path("), "{}", build);
    assert!(!build.contains("ast."), "{}", build);
    let call = f.translated_method("use_it.rs", "call");
    assert!(call.contains("parseOne(n)"), "{}", call);
    assert!(!call.contains("parser."), "{}", call);
}

/// `Option::unwrap` and `Option::expect` PANIC when there is nothing
/// there. Written as the identity — which is right for a guard, whose
/// `unwrap` the port's `lock()` has already performed — they handed the
/// `null` on to be read further down, and `expect`'s message was thrown
/// away with it: ankql's `PathExpr::property` answered `undefined` for an
/// empty path where Rust stops the program.
#[test]
fn unwrap_on_an_option_panics_rather_than_handing_the_nothing_on() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Path { pub steps: Vec<String> }\n\
         impl Path {\n\
           pub fn property(&self) -> &str { self.steps.last().expect(\"needs a step\") }\n\
           pub fn first(&self) -> &str { self.steps.first().unwrap() }\n\
         }",
    )]);
    let expect = f.translated_method("lib.rs", "property");
    assert!(expect.contains("?? (() => { throw new Error('needs a step'); })()"), "{}", expect);
    let unwrap = f.translated_method("lib.rs", "first");
    assert!(
        unwrap.contains("throw new Error('called `Option::unwrap()` on a `None` value')"),
        "{}",
        unwrap
    );
}

/// `Self::setup_receiver(..)` inside an impl is a static of that class. The
/// path had already been written in TypeScript by the time the call was
/// built — `Self.setupReceiver` — and splitting it on `::` alone left the
/// whole of it, so connector-local's emitted file called
/// `LocalProcessConnection.Self.setupReceiver`.
#[test]
fn a_self_qualified_static_is_the_class_and_the_method() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Conn { pub n: usize }\n\
         impl Conn {\n\
           pub fn setup_receiver(n: usize) -> usize { n }\n\
           pub fn start(&self) -> usize { Self::setup_receiver(1) }\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "start");
    assert!(ts.contains("Conn.setupReceiver(1)"), "{}", ts);
    assert!(!ts.contains("Self"), "{}", ts);
}
