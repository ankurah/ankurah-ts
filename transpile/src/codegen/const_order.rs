//! The order a file's `const`s are written in.
//!
//! For: Rust's items are order-independent and JavaScript's `const` is not.
//! `const A = B + 1;` written above `const B = 1;` is
//! `ReferenceError: Cannot access 'B' before initialization` at module load, so
//! the whole file fails to load and every import of it with it.

/// The consts and statics of a file, ordered so that each stands after the ones
/// its initialiser names.
///
/// A cycle cannot be written at all — Rust would have refused it too, since a
/// const's value has to be computable — so one is left in source order and the
/// names it holds report for themselves at the site that reads them.
pub(super) fn in_dependency_order(consts: &[crate::types::ConstInfo]) -> Vec<&crate::types::ConstInfo> {
    let names: Vec<&str> = consts.iter().map(|c| c.name.as_str()).collect();
    let mut done = vec![false; consts.len()];
    let mut out: Vec<&crate::types::ConstInfo> = Vec::new();
    // One pass per const at most: each pass emits every const whose remaining
    // dependencies are already out, and a pass that emits nothing has only a
    // cycle left, which goes out in source order.
    for _ in 0..consts.len() {
        let mut moved = false;
        for (at, c) in consts.iter().enumerate() {
            if done[at] {
                continue;
            }
            let waiting = names.iter().enumerate().any(|(other, name)| {
                other != at && !done[other] && names_the_const(c, name)
            });
            if waiting {
                continue;
            }
            done[at] = true;
            out.push(c);
            moved = true;
        }
        if !moved {
            break;
        }
    }
    for (at, c) in consts.iter().enumerate() {
        if !done[at] {
            out.push(c);
        }
    }
    out
}

/// Does this const's initialiser name the given const?
///
/// Asked of the Rust EXPRESSION, not of the rendered TypeScript. A const's
/// initialiser is one expression and the paths in it are the values it needs
/// before it can be evaluated — and a name inside a STRING is not one of them:
/// `const FIRST = SECOND;` beside `const SECOND: &str = "FIRST";` read as text
/// was a cycle, and a cycle goes out in source order, which is the very order
/// that throws.
fn names_the_const(c: &crate::types::ConstInfo, rust_name: &str) -> bool {
    let Some(init) = &c.init else { return false };
    struct Paths<'n> {
        wanted: &'n str,
        found: bool,
    }
    impl syn::visit::Visit<'_> for Paths<'_> {
        fn visit_path(&mut self, path: &syn::Path) {
            // The LAST segment: a const is reached as `NAME` or as
            // `module::NAME`, and either names the same const.
            if path.segments.last().is_some_and(|s| s.ident == self.wanted) {
                self.found = true;
            }
            syn::visit::visit_path(self, path);
        }
    }
    let mut paths = Paths { wanted: rust_name, found: false };
    syn::visit::Visit::visit_expr(&mut paths, init);
    paths.found
}

#[cfg(test)]
mod const_order_tests {
    use crate::testing::Fixture;

    /// Rust's items are order-independent; JavaScript's `const` is not. `const
    /// A = B + 1;` written above `const B = 1;` is `ReferenceError: Cannot
    /// access 'B' before initialization` at module load, so the whole file
    /// fails to load and every import of it with it.
    #[test]
    fn a_const_stands_after_the_ones_its_initialiser_names() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub const A: i32 = B + 1;\npub const B: i32 = 1;\npub const C: i32 = A + B;",
        )]);
        let ts = f.emitted("lib.rs");
        let at = |name: &str| ts.find(&format!("const {}:", name)).expect(name);
        assert!(at("B") < at("A"), "B stands before A:\n{ts}");
        assert!(at("A") < at("C"), "A stands before C:\n{ts}");
    }

    /// A name that is a PREFIX of another is not a dependency: `BASE` does not
    /// find `BASELINE`.
    #[test]
    fn a_prefix_of_another_name_is_not_a_dependency() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub const BASELINE: i32 = 1;\npub const BASE: i32 = 2;\n\
             pub const USES: i32 = BASELINE;",
        )]);
        let ts = f.emitted("lib.rs");
        let at = |name: &str| ts.find(&format!("const {}:", name)).expect(name);
        assert!(at("BASELINE") < at("USES"), "{ts}");
        // BASE depends on nothing, so it keeps its source position relative to
        // BASELINE.
        assert!(at("BASELINE") < at("BASE"), "{ts}");
    }

    /// Z6: a name inside a STRING is not a dependency. Read as text,
    /// `const SECOND: &str = "FIRST"` made `FIRST` and `SECOND` a cycle, and a
    /// cycle goes out in SOURCE order — which is the order that throws at
    /// module load.
    #[test]
    fn a_name_inside_a_string_is_not_a_dependency() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub const FIRST: i32 = SECOND;
pub const SECOND: i32 = 1;
             pub const LABEL: &str = \"FIRST SECOND\";",
        )]);
        let ts = f.emitted("lib.rs");
        let at = |name: &str| ts.find(&format!("const {}:", name)).expect(name);
        assert!(at("SECOND") < at("FIRST"), "the string made a cycle:\n{ts}");
    }

    /// A cycle cannot be written at all — Rust would have refused it, since a
    /// const's value has to be computable — so one is left in source order
    /// rather than looping.
    #[test]
    fn a_cycle_is_left_in_source_order() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub const A: i32 = B;\npub const B: i32 = A;",
        )]);
        let ts = f.emitted("lib.rs");
        assert!(ts.contains("const A:") && ts.contains("const B:"), "{ts}");
    }
}
