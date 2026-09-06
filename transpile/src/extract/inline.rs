//! Where an INLINE module's items live.
//!
//! For: `mod inner { .. }` is a module like any other, and the registry finds a
//! module by the PATH of the file its items were read from. An inline module's
//! sub-file carried no path at all, so `lookup_file` answered the crate root
//! for it — and an impl on a type the inline module declares then looked like
//! an impl on a type declared somewhere else, which is a module-level function
//! rather than a static of the class. `Kind_of(n)` stood beside a caller
//! writing `Kind.of(n)`: a `TypeError` with nothing said (G5).

/// Where an inline module's items live, as a path `lookup_file` reads.
///
/// The path is built from the parent's OWN module path so that the two cannot
/// disagree: `a/b.rs` with `mod inner { .. }` is `a/b/inner.rs`, and `a/mod.rs`
/// is `a/inner.rs`, because `file_module_path` drops a `mod` segment — and a
/// crate root's inline child is `inner.rs`, which the earlier spelling of this
/// got wrong. A parent with no path of its own — the one the std surface's
/// `extract_source` builds — leaves the child without one too, which is what it
/// was.
pub(super) fn inline_module_path(
    parent: &str,
    mod_name: &str,
    span: proc_macro2::Span,
) -> String {
    if parent.is_empty() {
        return String::new();
    }
    let mut segments = crate::registry::module::file_module_path(parent);
    segments.push(mod_name.to_string());
    let path = format!("{}.rs", segments.join("/"));
    // `file_module_path` drops a trailing `mod`, `lib` or `main` segment, so a
    // module of one of those names has no path that reads back as itself.
    // `mod mod` is not Rust; `mod lib {}` in a crate root is, and the items in
    // it would be read as the crate root's own — the very confusion G5 names —
    // so it is said rather than guessed at.
    if crate::registry::module::file_module_path(&path) != segments {
        crate::diag::pending::park(
            span,
            format!(
                "`mod {}` is an inline module whose name is one a file path drops, so the port \
                 has no path to give it and its items are read as the enclosing module's",
                mod_name
            ),
        );
        return String::new();
    }
    path
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    /// `a/b.rs` with `mod inner { .. }` is the module `a::b::inner`, and the
    /// file the port writes for it is `a/b/inner.ts`. `a/mod.rs` and a crate
    /// root ARE their module, so their inline child sits beside them.
    #[test]
    fn an_inline_module_lives_beside_the_file_that_holds_it() {
        let at = proc_macro2::Span::call_site();
        assert_eq!(super::inline_module_path("a/b.rs", "inner", at), "a/b/inner.rs");
        assert_eq!(super::inline_module_path("a/mod.rs", "inner", at), "a/inner.rs");
        assert_eq!(super::inline_module_path("lib.rs", "inner", at), "inner.rs");
        assert_eq!(super::inline_module_path("a/lib.rs", "inner", at), "a/inner.rs");
        // Deeper than one level down, a `lib.rs` is an ordinary file.
        assert_eq!(super::inline_module_path("a/b/lib.rs", "inner", at), "a/b/lib/inner.rs");
        // The std surface's files carry no path, and neither can their children.
        assert_eq!(super::inline_module_path("", "inner", at), "");
        // And a name a path would drop has no path at all, which is reported.
        assert_eq!(super::inline_module_path("lib.rs", "lib", at), "");
    }

    /// G5: an associated fn of a type an INLINE module declares is a STATIC of
    /// that type's class. Emission asks `lookup_file` which module the file it
    /// is writing is, and an inline sub-file with no path answered the crate
    /// root — so the impl looked like an impl on a type declared elsewhere,
    /// which is a module-level `Kind_of(n)` while every caller writes
    /// `Kind.of(n)`.
    #[test]
    fn an_associated_fn_of_an_inline_module_is_a_static_of_its_class() {
        let mut f = Fixture::build(&[
            ("lib.rs", "pub mod sub;"),
            (
                "sub.rs",
                "pub mod inner {\n\
                   pub struct Kind { pub n: u32 }\n\
                   impl Kind { pub fn of(n: u32) -> Kind { Kind { n } } }\n\
                 }\n\
                 pub fn make() -> inner::Kind { inner::Kind::of(3) }",
            ),
        ]);
        // The caller's side, which was right all along.
        assert!(f.emitted("sub.rs").contains("Kind.of(3)"));
        let entry = f.files.iter().find(|e| e.path == "sub.rs").expect("sub.rs");
        let (_, sub) = entry.file.inline_modules.first().expect("the inline module");
        let ts = crate::codegen::generate_ts(&f.reg, sub, &sub.path);
        assert!(ts.contains("static of("), "`of` is not a static of `Kind`:\n{ts}");
        assert!(!ts.contains("function Kind_of"), "`of` is a free function:\n{ts}");

        // And the same module written in the crate root, which the path built
        // by trimming the parent's suffix could not reach.
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub mod inner {\n\
               pub struct Kind { pub n: u32 }\n\
               impl Kind { pub fn of(n: u32) -> Kind { Kind { n } } }\n\
             }",
        )]);
        let _ = f.emitted("lib.rs");
        let entry = f.files.iter().find(|e| e.path == "lib.rs").expect("lib.rs");
        let (_, sub) = entry.file.inline_modules.first().expect("the inline module");
        let ts = crate::codegen::generate_ts(&f.reg, sub, &sub.path);
        assert!(ts.contains("static of("), "`of` is not a static of `Kind`:\n{ts}");
    }
}
