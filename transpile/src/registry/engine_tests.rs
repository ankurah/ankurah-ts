//! The engine's executable specification.
//!
//! Each test hands syn a few lines of Rust, builds a registry from them, and
//! asserts the type that comes back or the diagnostic that does instead.

use super::{Def, Ns};
use crate::testing::Fixture;
use crate::ty::{ArrayLen, Prim, Ty};

const ARC: &str = "std::sync::Arc";
const OPTION: &str = "std::option::Option";
const VEC: &str = "std::vec::Vec";
const HASHMAP: &str = "std::collections::HashMap";
const CELL_REF: &str = "std::cell::Ref";
const RWLOCK: &str = "std::sync::RwLock";
const GUARD: &str = "std::sync::RwLockWriteGuard";

// ── Primitives, and the widths that must stay apart ───────────────────

#[test]
fn integer_widths_stay_distinct() {
    let c = Fixture::build(&[("lib.rs", "")]);
    for (src, prim) in [
        ("u8", Prim::U8),
        ("u16", Prim::U16),
        ("u32", Prim::U32),
        ("u64", Prim::U64),
        ("usize", Prim::Usize),
        ("i8", Prim::I8),
        ("i16", Prim::I16),
        ("i32", Prim::I32),
        ("i64", Prim::I64),
        ("isize", Prim::Isize),
        ("f32", Prim::F32),
        ("f64", Prim::F64),
        ("bool", Prim::Bool),
        ("char", Prim::Char),
    ] {
        assert_eq!(c.ty("lib.rs", src), Ty::Prim(prim), "{}", src);
    }
    assert_ne!(c.ty("lib.rs", "i64"), c.ty("lib.rs", "i32"));
    assert_ne!(c.ty("lib.rs", "u64"), c.ty("lib.rs", "i64"));
}

#[test]
fn references_keep_their_mutability_and_drop_their_lifetime() {
    let c = Fixture::build(&[("lib.rs", "")]);
    assert_eq!(
        c.ty("lib.rs", "&u8"),
        Ty::Ref {
            mutable: false,
            inner: Box::new(Ty::Prim(Prim::U8))
        }
    );
    assert_eq!(
        c.ty("lib.rs", "&mut u8"),
        Ty::Ref {
            mutable: true,
            inner: Box::new(Ty::Prim(Prim::U8))
        }
    );
    assert_eq!(c.ty("lib.rs", "&'a u8"), c.ty("lib.rs", "&u8"));
}

#[test]
fn tuples_arrays_slices_str_unit_and_never() {
    let c = Fixture::build(&[("lib.rs", "")]);
    assert_eq!(c.ty("lib.rs", "()"), Ty::Unit);
    assert_eq!(c.ty("lib.rs", "!"), Ty::Never);
    assert_eq!(c.ty("lib.rs", "str"), Ty::Str);
    assert_eq!(
        c.ty("lib.rs", "&str"),
        Ty::Ref {
            mutable: false,
            inner: Box::new(Ty::Str)
        }
    );
    assert_eq!(
        c.ty("lib.rs", "(u8, bool)"),
        Ty::Tuple(vec![Ty::Prim(Prim::U8), Ty::Prim(Prim::Bool)])
    );
    assert_eq!(
        c.ty("lib.rs", "[u8]"),
        Ty::Slice(Box::new(Ty::Prim(Prim::U8)))
    );
    assert_eq!(
        c.ty("lib.rs", "[u8; 32]"),
        Ty::Array {
            elem: Box::new(Ty::Prim(Prim::U8)),
            len: ArrayLen::Lit(32)
        }
    );
    assert_eq!(
        c.ty_in("lib.rs", "[T; N]", &["T"]).unwrap(),
        Ty::Array {
            elem: Box::new(Ty::Param("T".into())),
            len: ArrayLen::Named("N".into())
        }
    );
    assert_eq!(c.ty("lib.rs", "_"), Ty::Infer);
}

// ── Generics ──────────────────────────────────────────────────────────

#[test]
fn generic_arguments_are_carried_and_parameters_stay_parameters() {
    let c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\npub struct Inner<T> { pub v: T }",
    )]);
    let inner_t = c.named("lib.rs", "Inner", vec![Ty::Param("T".into())]);
    assert_eq!(
        c.ty_in("lib.rs", "Arc<Inner<T>>", &["T"]).unwrap(),
        c.system(ARC, vec![inner_t])
    );
    assert_eq!(
        c.ty_in("lib.rs", "std::collections::HashMap<usize, T>", &["T"])
            .unwrap(),
        c.system(HASHMAP, vec![Ty::Prim(Prim::Usize), Ty::Param("T".into())])
    );
    // Without a declaration in scope, `T` is a type, not a parameter.
    assert!(matches!(c.ty("lib.rs", "T"), Ty::Named { .. }));
}

#[test]
fn option_and_vec_come_from_the_prelude() {
    let c = Fixture::build(&[("lib.rs", "")]);
    assert_eq!(
        c.ty("lib.rs", "Option<u8>"),
        c.system(OPTION, vec![Ty::Prim(Prim::U8)])
    );
    assert_eq!(
        c.ty("lib.rs", "Vec<u8>"),
        c.system(VEC, vec![Ty::Prim(Prim::U8)])
    );
}

#[test]
fn dyn_traits_and_argument_position_impl_trait() {
    let c = Fixture::build(&[("lib.rs", "pub trait Observer { fn observe(&self); }")]);
    let observer = c.reg.module_type(c.module("lib.rs"), "Observer").unwrap();

    let Ty::Dyn { traits } = c.ty("lib.rs", "dyn Observer + Send + Sync") else {
        panic!("expected a trait object")
    };
    assert_eq!(traits.len(), 3, "every written trait is kept, in order");
    assert_eq!(traits[0].id, observer);

    // `Fn(A) -> R` keeps Rust's desugaring: the inputs as one tuple argument
    // plus an `Output` binding.
    let Ty::Dyn { traits } = c.ty("lib.rs", "dyn Fn(u8, bool) -> usize") else {
        panic!("expected a trait object")
    };
    assert_eq!(
        traits[0].args,
        vec![Ty::Tuple(vec![Ty::Prim(Prim::U8), Ty::Prim(Prim::Bool)])]
    );
    assert_eq!(
        traits[0].bindings,
        vec![("Output".to_string(), Ty::Prim(Prim::Usize))]
    );

    // `impl Trait` in argument position is an anonymous generic parameter, so
    // it carries the bounds it was written with.
    let Ty::ImplTrait { bounds } = c.ty("lib.rs", "impl Observer") else {
        panic!("expected argument-position impl Trait")
    };
    assert_eq!(bounds.len(), 1);
    assert_eq!(bounds[0].id, observer);
}

#[test]
fn a_projection_keeps_the_trait_it_projects_through() {
    let c = Fixture::build(&[("lib.rs", "pub trait Convert<A> { type Out; }")]);
    let convert = c.reg.module_type(c.module("lib.rs"), "Convert").unwrap();

    let ty = c
        .ty_in("lib.rs", "<T as Convert<u8>>::Out", &["T"])
        .unwrap();
    let Ty::Assoc { base, trait_, name } = ty else {
        panic!("expected a projection")
    };
    assert_eq!(*base, Ty::Param("T".into()));
    assert_eq!(name, "Out");
    let tr = trait_.expect("the trait is kept");
    assert_eq!(tr.id, convert);
    assert_eq!(
        tr.args,
        vec![Ty::Prim(Prim::U8)],
        "and so are its own arguments"
    );

    // A bare `Self::Error` has no trait written on it, and `Self` outside an
    // impl is refused rather than guessed at.
    assert!(c.ty_in("lib.rs", "Self::Error", &[]).is_err());
    assert_eq!(
        c.ty_in("lib.rs", "T::Item", &["T"]).unwrap(),
        Ty::Assoc {
            base: Box::new(Ty::Param("T".into())),
            trait_: None,
            name: "Item".into()
        }
    );
}

#[test]
fn arguments_that_are_not_types_are_refused_rather_than_dropped() {
    let c = Fixture::build(&[("lib.rs", "pub struct IVec<T> { pub v: T }")]);

    let err = c.ty_in("lib.rs", "IVec<u8, 4>", &[]).unwrap_err();
    assert!(err.message.contains("const generic"), "{}", err.message);

    let err = c.ty_in("lib.rs", "IVec<Item = u8>", &[]).unwrap_err();
    assert!(
        err.message.contains("associated type binding"),
        "{}",
        err.message
    );

    // The third case, `Fn(A) -> R` written as a plain type path, would lose
    // its return type. It is refused too, but a 2021-edition parse never
    // reaches it: syn requires `dyn` or `impl`, which go through the trait
    // bound instead, and that keeps the `Output`.
    let bound = c.ty("lib.rs", "Box<dyn Fn(u8) -> bool>");
    let Ty::Named { args, .. } = &bound else {
        panic!("expected Box")
    };
    let Ty::Dyn { traits } = &args[0] else {
        panic!("expected a trait object")
    };
    assert_eq!(
        traits[0].bindings,
        vec![("Output".to_string(), Ty::Prim(Prim::Bool))]
    );
}

// ── Paths, imports, and the module tree ───────────────────────────────

#[test]
fn use_aliases_and_renames_resolve_to_the_declaration() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod broadcast;"),
        ("broadcast.rs", "pub struct Broadcast<T> { pub v: T }"),
        (
            "signal.rs",
            "use crate::broadcast::Broadcast;\nuse crate::broadcast::Broadcast as Bc;\npub struct S;",
        ),
    ]);
    let broadcast = c.named("broadcast.rs", "Broadcast", vec![Ty::Prim(Prim::U8)]);
    assert_eq!(c.ty("signal.rs", "Broadcast<u8>"), broadcast);
    assert_eq!(c.ty("signal.rs", "Bc<u8>"), broadcast);
    assert_eq!(
        c.ty("signal.rs", "crate::broadcast::Broadcast<u8>"),
        broadcast
    );
}

#[test]
fn self_imports_bind_the_module_they_name() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod ast;"),
        ("ast.rs", "pub struct Expr;"),
        ("plain.rs", "use crate::ast::{self, Expr};\npub struct A;"),
        (
            "renamed.rs",
            "use crate::ast::{self as tree, Expr};\npub struct B;",
        ),
    ]);
    let expr = c.named("ast.rs", "Expr", vec![]);
    // `{self}` binds the parent segment, not a type called "self".
    assert_eq!(c.ty("plain.rs", "ast::Expr"), expr);
    assert_eq!(c.ty("plain.rs", "Expr"), expr);
    assert_eq!(c.ty("renamed.rs", "tree::Expr"), expr);
    assert!(c
        .reg
        .lookup_type(c.module("plain.rs"), &["self".into()])
        .unwrap()
        .is_none());
}

#[test]
fn a_name_and_the_path_it_was_imported_from_intern_as_one_undeclared_type() {
    let c = Fixture::build(&[(
        "lib.rs",
        "use ankurah_proto::{self as proto, Attested};\npub struct S;",
    )]);
    // The declared surface has reported its own gaps by now; this counts what
    // the fixture's own Rust adds to that.
    let before = c.reg.undeclared_reported();
    let bare = c.ty("lib.rs", "Attested");
    let qualified = c.ty("lib.rs", "proto::Attested");
    assert_eq!(bare, qualified, "one type, however it is written");
    assert!(bare.id().unwrap().is_foreign());
    assert_eq!(c.reg.undeclared_reported(), before + 1);
}

#[test]
fn self_super_and_crate_paths_walk_the_module_tree() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod signal;"),
        ("signal.rs", "pub mod memo;\npub struct Read;"),
        ("signal/memo.rs", "pub struct Memo;"),
    ]);
    let read = c.named("signal.rs", "Read", vec![]);
    let memo = c.named("signal/memo.rs", "Memo", vec![]);

    assert_eq!(c.ty("signal.rs", "self::Read"), read);
    assert_eq!(c.ty("signal.rs", "memo::Memo"), memo);
    assert_eq!(c.ty("signal/memo.rs", "super::Read"), read);
    assert_eq!(c.ty("signal/memo.rs", "crate::signal::Read"), read);
    // The crate answers to its own name as well as to `crate`.
    assert_eq!(c.ty("signal/memo.rs", "testcrate::signal::Read"), read);
}

#[test]
fn glob_reexports_are_followed() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod signal;\npub use signal::*;"),
        ("signal.rs", "pub mod read;\npub use read::*;"),
        ("signal/read.rs", "pub struct Peek;"),
        ("other.rs", "use crate::Peek;\npub struct S;"),
    ]);
    assert_eq!(
        c.ty("other.rs", "Peek"),
        c.named("signal/read.rs", "Peek", vec![])
    );
}

#[test]
fn two_globs_offering_different_declarations_are_ambiguous() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod a;\npub mod b;"),
        ("a.rs", "pub struct Foo;"),
        ("b.rs", "pub struct Foo;"),
        (
            "user.rs",
            "use crate::a::*;\nuse crate::b::*;\npub struct S;",
        ),
    ]);
    let err = c.ty_in("user.rs", "Foo", &[]).unwrap_err();
    assert!(err.message.contains("ambiguous"), "{}", err.message);

    // One of the two named directly is not ambiguous.
    assert_eq!(
        c.ty("user.rs", "crate::a::Foo"),
        c.named("a.rs", "Foo", vec![])
    );
}

#[test]
fn a_private_item_does_not_leak_and_a_pub_crate_reexport_does() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod reactor;\npub mod user;"),
        (
            "reactor.rs",
            "struct Hidden;\npub(crate) struct ReactorUpdate;",
        ),
        ("user.rs", "pub struct S;"),
    ]);
    // `struct Hidden` is private to the module that declares it.
    assert_eq!(
        c.reg.lookup_type(
            c.module("user.rs"),
            &["crate".into(), "reactor".into(), "Hidden".into()]
        ),
        Ok(None)
    );
    // `pub(crate)` is visible everywhere in this crate.
    let update = c.named("reactor.rs", "ReactorUpdate", vec![]);
    assert_eq!(c.ty("user.rs", "crate::reactor::ReactorUpdate"), update);
    // And the module that declares it still sees its own private item.
    assert!(c
        .reg
        .lookup_type(c.module("reactor.rs"), &["Hidden".into()])
        .unwrap()
        .is_some());
}

/// The defect this registry replaces: `signals::broadcast::Ref` used to delete
/// the system `Ref` from a crate-wide table of leaf names, after which no
/// `RefCell::borrow()` guard could be reached through. Both must resolve, each
/// to its own declaration.
#[test]
fn a_crate_ref_and_std_cell_ref_both_resolve() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod broadcast;\npub mod context;"),
        ("broadcast.rs", "pub struct Ref<'a, T> { pub inner: &'a T }"),
        ("context.rs", "use std::cell::Ref;\npub struct Stack;"),
    ]);

    let crate_ref = c.reg.module_type(c.module("broadcast.rs"), "Ref").unwrap();
    let system_ref = c.system_id(CELL_REF);
    assert_ne!(crate_ref, system_ref);

    // Inside the module that declares it, the bare name is the crate's own.
    assert_eq!(c.ty("broadcast.rs", "Ref<u8>").id(), Some(crate_ref));
    // The qualified std path still reaches the system type, from either module.
    assert_eq!(
        c.ty("broadcast.rs", "std::cell::Ref<u8>").id(),
        Some(system_ref)
    );
    // And where the module imported it, the bare name is the system type.
    assert_eq!(c.ty("context.rs", "Ref<u8>").id(), Some(system_ref));

    // The system type kept the accessor that reaching through it needs.
    let probe = c.probe("broadcast.rs");
    let step = probe
        .deref_once(&c.system(CELL_REF, vec![Ty::Prim(Prim::U8)]))
        .expect("std's Ref dereferences");
    assert_eq!(step.accessor.map(|a| a.written()).as_deref(), Some("value"));
    assert!(
        probe.deref_once(&c.ty("broadcast.rs", "Ref<u8>")).is_none(),
        "the crate's own Ref is not a wrapper"
    );
}

#[test]
fn a_system_type_outside_the_prelude_needs_an_import_or_a_path() {
    let c = Fixture::build(&[
        ("lib.rs", "pub struct S;"),
        ("with.rs", "use std::sync::Arc;\npub struct T;"),
    ]);
    // `Arc` is not in Rust's prelude, and it is not in this engine's either.
    let bare = c.ty("lib.rs", "Arc<u8>");
    assert!(
        bare.id().unwrap().is_foreign(),
        "a bare `Arc` names nothing declared"
    );
    assert!(
        c.messages()
            .iter()
            .any(|m| m == "no declaration for type `Arc`"),
        "{:?}",
        c.messages()
    );
    // With the import, or written out, it is the system type.
    assert_eq!(c.ty("with.rs", "Arc<u8>").id(), Some(c.system_id(ARC)));
    assert_eq!(
        c.ty("lib.rs", "std::sync::Arc<u8>").id(),
        Some(c.system_id(ARC))
    );
}

#[test]
fn a_std_path_does_not_collapse_onto_a_system_type_by_its_last_segment() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let real = c.ty("lib.rs", "Result<u8, S>");
    assert_eq!(real.id(), Some(c.system_id("std::result::Result")));

    // `std::fmt::Result` is a different declaration in a different module: an
    // alias for `Result<(), fmt::Error>`, which is what it expands to. The two
    // used to collapse onto each other because a std path was matched on its
    // last segment alone.
    let fmt = c.ty("lib.rs", "std::fmt::Result");
    assert_eq!(
        fmt,
        c.system(
            "std::result::Result",
            vec![Ty::Unit, c.system("std::fmt::Error", vec![])]
        )
    );
    assert_ne!(fmt, real, "and it is not the two-parameter `Result`");

    // And the right path with the wrong number of arguments is refused too.
    let err = c
        .ty_in("lib.rs", "std::result::Result<u8>", &[])
        .unwrap_err();
    assert!(
        err.message.contains("2 type argument(s) but 1"),
        "{}",
        err.message
    );
}

/// `use anyhow::anyhow;` binds a macro and a function, not a module. Splicing
/// a type path through it used to loop until the stack ran out.
#[test]
fn an_import_that_names_no_module_cannot_carry_a_type_path() {
    // `use anyhow::anyhow` binds a *function*, so a type path written through
    // that name means the crate and not the function — otherwise resolving
    // `anyhow::Result` follows the import back into itself and never stops.
    let c = Fixture::build(&[("node.rs", "use anyhow::anyhow;\npub struct Node;")]);
    let ty = c.ty("node.rs", "anyhow::Result<()>");
    assert_eq!(
        ty,
        c.system(
            "std::result::Result",
            vec![Ty::Unit, c.system("anyhow::Error", vec![])]
        ),
        "the crate's own `Result` alias, reached past the imported function"
    );
    assert!(c.messages().is_empty(), "{:?}", c.messages());
}

// ── Aliases ───────────────────────────────────────────────────────────

#[test]
fn a_type_alias_expands_where_it_is_used() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod property;"),
        (
            "property.rs",
            "pub type PropertyName = String;\npub type Pair<T> = (T, u8);",
        ),
        (
            "user.rs",
            "use crate::property::{PropertyName, Pair};\npub struct S;",
        ),
    ]);
    assert_eq!(
        c.ty("user.rs", "PropertyName"),
        c.system("std::string::String", vec![])
    );
    assert_eq!(
        c.ty("user.rs", "Pair<bool>"),
        Ty::Tuple(vec![Ty::Prim(Prim::Bool), Ty::Prim(Prim::U8)])
    );
}

#[test]
fn an_alias_that_expands_into_itself_is_refused() {
    let c = Fixture::build(&[("lib.rs", "pub type Loop = Loop;")]);
    let err = c.ty_in("lib.rs", "Loop", &[]).unwrap_err();
    assert!(
        err.message.contains("expands into itself"),
        "{}",
        err.message
    );
}

// ── The value namespace ───────────────────────────────────────────────

#[test]
fn a_constant_resolves_through_the_module_that_declares_it() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod policy;"),
        (
            "policy.rs",
            "pub const DEFAULT_CONTEXT: u32 = 3;\nstruct Private;",
        ),
        ("user.rs", "pub struct S;"),
    ]);
    let module = c.module("user.rs");
    let found = c.reg.lookup(
        module,
        Ns::Value,
        &["crate".into(), "policy".into(), "DEFAULT_CONTEXT".into()],
    );
    let Ok(Some(Def::Value(id))) = found else {
        panic!("expected a value, got {:?}", found)
    };
    assert_eq!(c.reg.value(id).unwrap().ty, Some(Ty::Prim(Prim::U32)));

    // A value name does not answer a type lookup, and vice versa.
    assert_eq!(
        c.reg.lookup_type(
            module,
            &["crate".into(), "policy".into(), "DEFAULT_CONTEXT".into()]
        ),
        Ok(None)
    );
}

#[test]
fn an_enum_variant_resolves_through_its_enum() {
    let c = Fixture::build(&[
        ("lib.rs", "pub mod signal;"),
        (
            "signal.rs",
            "pub enum Kind { Constant, Dynamic }\npub struct Constant;",
        ),
        ("user.rs", "use crate::signal::Kind;\npub struct S;"),
    ]);
    let module = c.module("user.rs");
    let kind = c.reg.module_type(c.module("signal.rs"), "Kind").unwrap();
    assert_eq!(
        c.reg
            .lookup_variant(module, &["Kind".into(), "Constant".into()]),
        Some((kind, "Constant".to_string()))
    );
    // The struct called `Constant` is not a variant of anything.
    assert_eq!(
        c.reg
            .lookup_variant(module, &["Kind".into(), "Missing".into()]),
        None
    );
}

// ── Fields, methods, and diagnostics ──────────────────────────────────

#[test]
fn struct_fields_are_recorded_as_resolved_types() {
    let c = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\npub struct Inner<T> { pub v: T }\npub struct Broadcast<T> { pub inner: Arc<Inner<T>> }",
    )]);
    let inner_t = c.named("lib.rs", "Inner", vec![Ty::Param("T".into())]);
    assert_eq!(
        c.field("lib.rs", "Broadcast", "inner"),
        c.system(ARC, vec![inner_t])
    );

    // Reaching a field through the wrapper reports the accessor emission needs.
    let broadcast = c.named("lib.rs", "Broadcast", vec![Ty::Prim(Prim::U8)]);
    let probe = c.probe("lib.rs");
    let found = probe.resolve_field(&broadcast, "inner").expect("field");
    assert!(found.accessors().is_empty());
    let inner = probe.resolve_field(&found.ty, "v").expect("field through Arc");
    assert_eq!(inner.accessors(), vec!["value"]);
    assert_eq!(
        inner.ty,
        Ty::Prim(Prim::U8),
        "the wrapper's argument is substituted in"
    );
}

#[test]
fn a_method_return_type_is_substituted_through_the_receiver() {
    let c = Fixture::build(&[("lib.rs", "")]);
    let lock = c.system(RWLOCK, vec![Ty::Prim(Prim::U8)]);
    let found = c.probe("lib.rs").resolve_method(&lock, "write").expect("declared");
    // `write` yields a `LockResult<RwLockWriteGuard<'_, T>>`, so the guard is
    // one `Result::unwrap` further on.
    let guard = c.system(GUARD, vec![Ty::Prim(Prim::U8)]);
    assert_eq!(
        found.ret,
        c.system(
            "std::result::Result",
            vec![
                guard.clone(),
                c.system("std::sync::PoisonError", vec![guard.clone()])
            ]
        )
    );
    let unwrapped = c
        .probe("lib.rs")
        .resolve_method(&found.ret, "unwrap")
        .expect("Result::unwrap");
    assert_eq!(unwrapped.ret, guard);
}

/// The impl block writes its own name for the receiver's argument. Two impls
/// that differ only in that letter have to behave identically.
#[test]
fn an_impl_substitutes_through_its_own_parameter_names() {
    let source = |letter: &str| {
        format!(
            "pub struct Wrap<R> {{ pub v: R }}\nimpl<{p}> Wrap<{p}> {{ pub fn get(&self) -> {p} {{ todo!() }} }}",
            p = letter
        )
    };
    let with_r = Fixture::build(&[("lib.rs", &source("R"))]);
    let with_e = Fixture::build(&[("lib.rs", &source("E"))]);

    for c in [&with_r, &with_e] {
        let wrap = c.named("lib.rs", "Wrap", vec![Ty::Prim(Prim::U8)]);
        let found = c.probe("lib.rs").resolve_method(&wrap, "get").expect("declared");
        assert_eq!(
            found.ret,
            Ty::Prim(Prim::U8),
            "the receiver's argument is bound to whatever the impl calls it"
        );
    }
}

#[test]
fn an_impl_whose_target_does_not_resolve_is_reported() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Listen<T> { fn listen(self); }\nimpl<F: Fn(u8)> Listen<u8> for F { fn listen(self) {} }",
    )]);
    // The impl table holds it as what it is: an impl written for a parameter,
    // which applies to whatever the receiver turns out to be. The std surface
    // contributes blanket impls of its own, so this asks for the one written
    // for the crate's trait.
    let listen = c.reg.module_type(c.module("lib.rs"), "Listen").unwrap();
    let mine: Vec<_> = c
        .reg
        .impls()
        .blanket()
        .iter()
        .map(|id| c.reg.impl_def(*id))
        .filter(|def| def.trait_ref.as_ref().is_some_and(|t| t.id == listen))
        .collect();
    assert_eq!(mine.len(), 1, "{:?}", c.messages());
    assert_eq!(mine[0].self_ty, Ty::Param("F".into()));
    assert!(mine[0].methods.contains_key("listen"));
}

#[test]
fn an_undeclared_type_gets_one_identity_and_one_diagnostic() {
    // A crate the declared surface says nothing about. (`ulid` used to serve
    // here and is declared now, which is the point of the surface.)
    let c = Fixture::build(&[("id.rs", "use nowhere::Ulid;\npub struct EntityId(Ulid);")]);
    let before = c.sink.len();

    let a = c.ty("id.rs", "Ulid");
    let b = c.ty("id.rs", "nowhere::Ulid");
    assert_eq!(a, b, "the bare name and the written path are the same type");
    assert!(a.id().unwrap().is_foreign());
    assert!(
        c.reg.def(a.id().unwrap()).is_none(),
        "nothing is known about its members"
    );
    assert_eq!(c.reg.name_of(a.id().unwrap()), "Ulid");

    // Reported once when first seen, not once per use.
    assert_eq!(
        c.sink.len(),
        before,
        "the field's own use already reported it"
    );
    assert!(
        c.messages()
            .iter()
            .any(|m| m == "no declaration for type `nowhere::Ulid`"),
        "{:?}",
        c.messages()
    );
}

#[test]
fn marker_traits_are_not_reported_and_a_real_trait_is_called_one() {
    let c = Fixture::build(&[("lib.rs", "pub struct S(Box<dyn Painter + Send + Sync>);")]);
    let messages = c.messages();
    assert!(
        messages
            .iter()
            .any(|m| m == "no declaration for trait `Painter`"),
        "{:?}",
        messages
    );
    for marker in ["Send", "Sync", "Sized"] {
        assert!(
            !messages.iter().any(|m| m.contains(marker)),
            "`{}` carries no shape and is not worth reporting: {:?}",
            marker,
            messages
        );
    }
}

#[test]
fn an_unmodelled_type_is_refused_with_its_position() {
    let c = Fixture::build(&[("lib.rs", "")]);
    let err = c.ty_in("lib.rs", "*const u8", &[]).unwrap_err();
    assert_eq!(err.file, "lib.rs");
    assert_eq!(err.line, 1);
    assert_eq!(err.col, 1);
    assert_eq!(err.message, "raw pointer type is not modelled");

    let err = c.ty_in("lib.rs", "fn(u8) -> u8", &[]).unwrap_err();
    assert_eq!(err.message, "function pointer type is not modelled");
}

#[test]
fn a_field_the_engine_refuses_is_left_unresolved_and_reported() {
    let c = Fixture::build(&[("lib.rs", "pub struct Raw { pub p: *const u8 }")]);
    let entry = c.files.iter().find(|e| e.path == "lib.rs").unwrap();
    assert!(entry.file.structs[0].fields[0].ty.is_none());
    assert!(c.messages().iter().any(|m| m.contains("raw pointer")));
}

/// `<F as Future>::Output` where nothing declares `Future`: the projection has
/// to keep the trait it was written through, or it is indistinguishable from
/// `F::Output`, which may be a different associated type entirely.
#[test]
fn a_projection_through_an_undeclared_trait_keeps_the_trait() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let through = c.ty_in("lib.rs", "<F as futures::Future>::Output", &["F"]).unwrap();
    let bare = c.ty_in("lib.rs", "F::Output", &["F"]).unwrap();
    let Ty::Assoc { trait_: Some(named), .. } = &through else {
        panic!("the trait was dropped: {:?}", through);
    };
    assert!(named.id.is_foreign(), "and it is interned as the foreign trait it is");
    assert_ne!(through, bare, "the two projections are not the same type");
    assert!(
        c.messages().iter().any(|m| m.contains("futures::Future")),
        "and the missing declaration is reported once: {:?}",
        c.messages()
    );
}

/// A crate type called `Result` is its own type. Recognising the runtime one by
/// leaf name gave it the runtime's `unwrap`.
#[test]
fn a_crate_result_is_not_the_runtime_result() {
    let c = Fixture::build(&[("lib.rs", "pub struct Result;")]);
    let cx = c.context("lib.rs", None);
    assert!(!cx.is_result(&c.named("lib.rs", "Result", vec![])));
    assert!(cx.is_result(&c.system(
        "std::result::Result",
        vec![Ty::Prim(Prim::U8), Ty::Prim(Prim::U8)]
    )));
}

// ── The declared std surface (spec 4.4, step 3) ───────────────────────
//
// Every std type the engine knows comes from a signature-only Rust stub under
// `transpile/std_surface/`, parsed by the extractor that reads ankurah's own
// source. These tests ask the questions that used to be answered by a table of
// strings in `transpile.toml`.

#[test]
fn a_stub_type_is_declared_under_its_real_path_and_the_prelude_rule_holds() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let module = c.module("lib.rs");

    // `Arc` is `std::sync::Arc` — the module its file says, not the file's name.
    let by_path = c.reg.lookup_type(module, &["std".into(), "sync".into(), "Arc".into()]);
    assert_eq!(by_path, Ok(Some(Def::Type(c.system_id(ARC)))));
    // And `core::sync::Arc` names the same one, because `core` re-exports into
    // `std` and ankurah writes both.
    assert_eq!(
        c.reg.lookup_type(module, &["core".into(), "sync".into(), "Arc".into()]),
        by_path
    );

    // A bare `Arc` is not in Rust's prelude and is not in scope without an
    // import; a bare `Vec` is in both.
    assert_eq!(c.reg.lookup_type(module, &["Arc".into()]), Ok(None));
    assert_eq!(
        c.reg.lookup_type(module, &["Vec".into()]),
        Ok(Some(Def::Type(c.system_id(VEC))))
    );

    // `HashMap` is declared in `std::collections::hash_map` and re-exported by
    // `std::collections`, exactly as std does it, and both paths reach it.
    assert_eq!(
        c.reg.lookup_type(module, &["std".into(), "collections".into(), "HashMap".into()]),
        c.reg.lookup_type(
            module,
            &["std".into(), "collections".into(), "hash_map".into(), "HashMap".into()]
        )
    );
}

#[test]
fn an_extern_crate_is_a_root_of_its_own() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let std_mutex = c.system_id("std::sync::Mutex");
    let tokio_mutex = c.system_id("tokio::sync::Mutex");
    assert_ne!(
        std_mutex, tokio_mutex,
        "`tokio::sync::Mutex` locks asynchronously and yields the guard itself; \
         they are two types and the registry is not flat"
    );
    // Reached by writing the crate's name, which is Rust's extern prelude.
    assert_eq!(
        c.reg.lookup_type(c.module("lib.rs"), &["tokio".into(), "sync".into(), "Mutex".into()]),
        Ok(Some(Def::Type(tokio_mutex)))
    );
    // And the two `lock`s hand back different things.
    let probe = c.probe("lib.rs");
    let std_lock = probe
        .resolve_method(&c.system("std::sync::Mutex", vec![Ty::Prim(Prim::U8)]), "lock")
        .expect("std::sync::Mutex::lock");
    let tokio_lock = probe
        .resolve_method(&c.system("tokio::sync::Mutex", vec![Ty::Prim(Prim::U8)]), "lock")
        .expect("tokio::sync::Mutex::lock");
    assert_ne!(std_lock.ret, tokio_lock.ret);
}

#[test]
fn a_stub_that_does_not_parse_names_the_file_it_is_in() {
    let dir = std::env::temp_dir().join(format!("std-surface-broken-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("std")).expect("scratch directory");
    std::fs::write(dir.join("std/vec.rs"), "pub struct Vec<T>;").expect("a stub that parses");
    std::fs::write(dir.join("std/broken.rs"), "pub struct {").expect("a stub that does not");

    let sink = crate::diag::DiagSink::new();
    let mut surface = crate::registry::Surface::load(&dir);
    let mut reg = crate::registry::TypeRegistry::new("testcrate");
    crate::registry::std_surface::declare(&mut reg, &mut surface, &sink);

    let messages: Vec<String> = sink.sorted().into_iter().map(|d| d.message).collect();
    assert!(
        messages.iter().any(|m| m.contains("std/broken.rs") && m.contains("could not be read")),
        "{:?}",
        messages
    );
    // The file that does parse is still declared: one broken stub is one gap,
    // not a surface that failed to load.
    assert!(reg.system_type("std::vec::Vec").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn lock_and_unwrap_reach_the_guard_through_lock_result() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let probe = c.probe("lib.rs");
    let mutex = c.system("std::sync::Mutex", vec![Ty::Prim(Prim::U8)]);
    let guard = c.system("std::sync::MutexGuard", vec![Ty::Prim(Prim::U8)]);

    // Rust's `Mutex::lock` yields `LockResult<MutexGuard<'_, T>>`, and the alias
    // is expanded to the `Result` it is.
    let lock = probe.resolve_method(&mutex, "lock").expect("Mutex::lock");
    assert_eq!(
        lock.ret,
        c.system(
            "std::result::Result",
            vec![guard.clone(), c.system("std::sync::PoisonError", vec![guard.clone()])]
        )
    );
    // The `unwrap` the source writes is `Result::unwrap`, resolved through the
    // impl table like any other call. There is no shim.
    let unwrapped = probe.resolve_method(&lock.ret, "unwrap").expect("Result::unwrap");
    assert_eq!(unwrapped.ret, guard);
}

#[test]
fn an_iterator_chain_types_through_the_adaptors_and_collect() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let probe = c.probe("lib.rs");
    let vec = c.system(VEC, vec![Ty::Prim(Prim::U8)]);

    // `v.iter()` is `<[T]>::iter` through the `Vec`'s `Deref`, and each adaptor
    // is a struct whose own `Iterator` impl says what it hands out.
    let iter = probe.resolve_method(&vec, "iter").expect("<[T]>::iter");
    let skipped = probe.resolve_method(&iter.ret, "skip").expect("Iterator::skip");
    assert_eq!(c.reg.name_of(skipped.ret.id().expect("named")), "Skip");
    let cloned = probe.resolve_method(&skipped.ret, "cloned").expect("Iterator::cloned");
    assert_eq!(c.reg.name_of(cloned.ret.id().expect("named")), "Cloned");
    // `Cloned<Skip<Iter<'_, u8>>>::Item` is `u8`: `Iter`'s is `&u8`, `Skip`
    // passes it through, and `Cloned`'s own bound fixes the element.
    assert_eq!(
        c.context("lib.rs", None).iteration_item(&cloned.ret),
        Some(Ty::Prim(Prim::U8))
    );

    // `collect` returns its own parameter, so only the turbofish says what the
    // chain produces — and `Vec<_>`'s hole is filled from `FromIterator`.
    let target = Ty::Named {
        id: c.system_id(VEC),
        args: vec![Ty::Infer],
    };
    let collected = probe
        .resolve_method_with(&cloned.ret, "collect", &[target])
        .expect("Iterator::collect");
    assert_eq!(collected.ret, vec);
}

#[test]
fn iterating_a_collection_goes_through_into_iterator() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);
    let vec = c.system(VEC, vec![Ty::Prim(Prim::U8)]);

    // `for x in v` hands out the element and `for x in &v` a reference to it,
    // because those are two `IntoIterator` impls with two different `Item`s.
    assert_eq!(cx.iteration_item(&vec), Some(Ty::Prim(Prim::U8)));
    assert_eq!(
        cx.iteration_item(&Ty::Ref { mutable: false, inner: Box::new(vec) }),
        Some(Ty::Ref { mutable: false, inner: Box::new(Ty::Prim(Prim::U8)) })
    );
    // A map hands out a pair, and an `Option` its payload.
    let map = c.system(HASHMAP, vec![Ty::Prim(Prim::U32), Ty::Prim(Prim::Bool)]);
    assert_eq!(
        cx.iteration_item(&map),
        Some(Ty::Tuple(vec![Ty::Prim(Prim::U32), Ty::Prim(Prim::Bool)]))
    );
    assert_eq!(
        cx.iteration_item(&c.system(OPTION, vec![Ty::Prim(Prim::U8)])),
        Some(Ty::Prim(Prim::U8))
    );
    // A type with no `IntoIterator` impl has no item type, and the loop
    // variable is bound without one rather than guessed at.
    assert_eq!(cx.iteration_item(&c.named("lib.rs", "S", vec![])), None);
}

#[test]
fn a_map_is_looked_up_by_a_borrowed_key_through_borrow() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let string = c.system("std::string::String", vec![]);
    let map = c.system(HASHMAP, vec![string, Ty::Prim(Prim::U8)]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&map, "get")
        .expect("HashMap::get");
    // `get<Q>(&self, k: &Q) -> Option<&V> where K: Borrow<Q>` — the generality
    // is kept so that `map.get("key")` on a `String`-keyed map resolves, and
    // what comes back is a borrow of the value.
    assert_eq!(
        found.ret,
        c.system(
            OPTION,
            vec![Ty::Ref { mutable: false, inner: Box::new(Ty::Prim(Prim::U8)) }]
        )
    );
    assert!(
        c.reg.impls().all().any(|def| {
            def.trait_ref.as_ref().is_some_and(|t| c.reg.name_of(t.id) == "Borrow")
                && def.self_ty == c.system("std::string::String", vec![])
        }),
        "`String: Borrow<str>` is what discharges the bound"
    );
}

#[test]
fn to_string_resolves_through_the_to_string_blanket() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct S;\n\
         impl std::fmt::Display for S {\n\
           fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { todo!() }\n\
         }",
    )]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&c.named("lib.rs", "S", vec![]), "to_string")
        .expect("`impl<T: Display> ToString for T`");
    assert!(matches!(found.callee, crate::registry::method::Callee::Blanket(..)));
    assert_eq!(found.ret, c.system("std::string::String", vec![]));
}

#[test]
fn a_blanket_is_taken_only_where_its_bound_is_met_or_reported() {
    // The mechanism `to_string` runs on, written out so the test does not
    // depend on the surface being able to read its own `Display` bound.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Shown { fn shown(&self) -> u8; }\n\
         pub trait Named { fn named(&self) -> u8; }\n\
         impl<T: Named> Shown for T { fn shown(&self) -> u8 { 0 } }\n\
         pub struct Yes;\n\
         impl Named for Yes { fn named(&self) -> u8 { 0 } }\n\
         pub struct No;",
    )]);
    let probe = c.probe("lib.rs");

    let found = probe
        .resolve_method(&c.named("lib.rs", "Yes", vec![]), "shown")
        .expect("the bound is met");
    assert!(matches!(found.callee, crate::registry::method::Callee::Blanket(..)));
    assert!(found.obligations.is_empty(), "{:?}", found.obligations);

    // With no `Named` impl the bound definitely fails, so the blanket is not a
    // candidate at all and nothing answers.
    assert!(probe
        .resolve_method(&c.named("lib.rs", "No", vec![]), "shown")
        .is_err());
}

#[test]
fn a_bound_the_engine_cannot_read_is_not_a_bound_that_holds() {
    // An impl whose requirement names something ambiguous or undeclared still
    // has that requirement. Dropping it turned `impl<T: Display> ToString for T`
    // into an impl of `ToString` for everything.
    let c = Fixture::build(&[(
        "lib.rs",
        "pub trait Shown { fn shown(&self) -> u8; }\n\
         impl<T: nowhere::Named> Shown for T { fn shown(&self) -> u8 { 0 } }\n\
         pub struct S;",
    )]);
    let found = c
        .probe("lib.rs")
        .resolve_method(&c.named("lib.rs", "S", vec![]), "shown")
        .expect("the impl is still a candidate");
    assert_eq!(found.obligations.len(), 1);
    assert_eq!(found.obligations[0].reason, crate::registry::Undecided::NoDeclaration);
}

#[test]
fn indexing_goes_through_index_output() {
    let c = Fixture::build(&[("lib.rs", "pub struct S;")]);
    let cx = c.context("lib.rs", None);

    // `v[0]` is `<Vec<T> as Index<usize>>::Output`, and `v[1..]` the slice its
    // range index produces. Both come from the declared `Index` family rather
    // than from a list of containers.
    let vec = c.system(VEC, vec![Ty::Prim(Prim::U8)]);
    assert_eq!(cx.index_of(&vec, "0"), Some(Ty::Prim(Prim::U8)));
    assert_eq!(
        cx.index_of(&vec, "1.."),
        Some(Ty::Slice(Box::new(Ty::Prim(Prim::U8))))
    );

    // `Index<&Q> for HashMap<K, V>` hands back the value, not an `Option`.
    let string = c.system("std::string::String", vec![]);
    let map = c.system(HASHMAP, vec![string, Ty::Prim(Prim::U8)]);
    assert_eq!(cx.index_of(&map, "\"k\""), Some(Ty::Prim(Prim::U8)));
}

#[test]
fn an_auto_trait_bound_is_answered_by_its_declaration() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct S;\n\
         pub trait Held { fn held(&self) -> u8; }\n\
         impl<T: Send + Sync> Held for T { fn held(&self) -> u8 { 0 } }",
    )]);
    // rustc decides `Send` and `Sync` from a type's fields, so there is no impl
    // in the table to find; the corpus compiles, which means every auto-trait
    // bound in it already holds. Looking for an impl would report an obligation
    // Rust had already discharged.
    let found = c
        .probe("lib.rs")
        .resolve_method(&c.named("lib.rs", "S", vec![]), "held")
        .expect("the blanket applies");
    assert!(found.obligations.is_empty(), "{:?}", found.obligations);
}

#[test]
fn the_emission_policy_is_keyed_on_identity_not_on_a_leaf_name() {
    let c = Fixture::build(&[(
        "lib.rs",
        "pub struct Vec<T> { pub v: T }\npub struct S;",
    )]);
    use crate::name_map::shape::{js_shape, JsShape};

    // The crate's own `Vec` is its own type and is emitted as itself.
    let crate_vec = c.named("lib.rs", "Vec", vec![Ty::Prim(Prim::U8)]);
    assert_eq!(js_shape(&c.reg, &crate_vec), JsShape::Plain);
    // The declared one is a JavaScript array, and a `Vec<u8>` a byte buffer.
    assert_eq!(
        js_shape(&c.reg, &c.system(VEC, vec![Ty::Prim(Prim::U16)])),
        JsShape::Array(Ty::Prim(Prim::U16))
    );
    assert_eq!(js_shape(&c.reg, &c.system(VEC, vec![Ty::Prim(Prim::U8)])), JsShape::Bytes);

    // Reaching through a wrapper is written where the port's runtime says, and
    // the accessor is looked up by the type's id.
    let step = c
        .probe("lib.rs")
        .deref_once(&c.system(ARC, vec![Ty::Prim(Prim::U8)]))
        .expect("Arc dereferences");
    assert_eq!(step.accessor.map(|a| a.written()).as_deref(), Some("value"));
    // A `Box` is transparent and a lock is not a wrapper at all.
    assert!(c
        .probe("lib.rs")
        .deref_once(&c.system("std::boxed::Box", vec![Ty::Prim(Prim::U8)]))
        .expect("Box dereferences")
        .accessor
        .is_none());

    // Nothing the shape table names has gone missing from the surface.
    assert!(c.reg.shapes().unresolved.is_empty(), "{:?}", c.reg.shapes().unresolved);
}

// ── Where the surface's own declarations live ─────────────────────────

#[test]
fn a_named_type_takes_its_module_from_its_file() {
    let c = Fixture::build(&[("lib.rs", "")]);
    assert!(
        c.reg.system_type("std::num::ParseIntError").is_some(),
        "`std/num.rs` declares it, so it lives at `std::num`"
    );
    assert!(
        c.reg.system_type("std::ParseIntError").is_none(),
        "and not at the crate root, which is where it landed while `std/num.rs` \
         was read as a drawer of primitive impls"
    );
    assert!(c.reg.system_type("std::num::NonZeroUsize").is_some());
    // `std/primitive.rs` is still read in `std`, and now holds no named type at
    // all, so nothing is reachable at the wrong path through it.
    assert!(c.reg.system_type("std::char::ToLowercase").is_some());
}
