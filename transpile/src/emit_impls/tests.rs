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
