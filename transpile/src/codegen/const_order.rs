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
pub(super) fn in_dependency_order<'f>(
    reg: &crate::registry::TypeRegistry,
    here: Option<crate::registry::ModuleId>,
    file: &'f crate::types::RustFile,
) -> Vec<&'f crate::types::ConstInfo> {
    let consts = &file.consts;
    let names: Vec<&str> = consts.iter().map(|c| c.name.as_str()).collect();
    let reads = Reads { reg, here, file };
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
                other != at && !done[other] && reads.names_the_const(c, name)
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

/// The question "does this const's initialiser need that one first?", asked
/// with the registry in hand.
struct Reads<'a> {
    reg: &'a crate::registry::TypeRegistry,
    here: Option<crate::registry::ModuleId>,
    file: &'a crate::types::RustFile,
}

impl Reads<'_> {
    /// Does this const's initialiser reach the given const, directly or through
    /// a `const fn` this file declares?
    ///
    /// Asked of the Rust EXPRESSION, not of the rendered TypeScript. A const's
    /// initialiser is one expression and the paths in it are the values it
    /// needs before it can be evaluated — and a name inside a STRING is not one
    /// of them: `const FIRST = SECOND;` beside `const SECOND: &str = "FIRST";`
    /// read as text was a cycle, and a cycle goes out in source order, which is
    /// the very order that throws.
    ///
    /// J6: a path is matched by IDENTITY, not by its last segment.
    /// `pub const B: u32 = outer::foreign::A;` names a const of another crate,
    /// and matching the leaf ordered it against the LOCAL `A` — a dependency
    /// that does not exist, which can only push the local one later than it
    /// needs to be or invent a cycle. And a const reached through a call —
    /// `const A: u32 = double(B);` — was not reached at all, so `A` could be
    /// written above `B` and the module failed to load.
    fn names_the_const(&self, c: &crate::types::ConstInfo, rust_name: &str) -> bool {
        let Some(init) = &c.init else { return false };
        let Some(wanted) = self.value_id(&[rust_name.to_string()]) else {
            // A const the registry does not carry — a `static` inside a test
            // module, a shape the declaration pass skipped — is matched by name
            // alone, which is what this did for everything before.
            return names_leaf(init, rust_name);
        };
        let mut followed: Vec<String> = Vec::new();
        self.reaches(init, wanted, &mut followed)
    }

    fn reaches(
        &self,
        expr: &syn::Expr,
        wanted: crate::registry::ValueId,
        followed: &mut Vec<String>,
    ) -> bool {
        let mut walk = Walk { reads: self, wanted, followed, found: false };
        syn::visit::Visit::visit_expr(&mut walk, expr);
        walk.found
    }

    fn value_id(&self, segments: &[String]) -> Option<crate::registry::ValueId> {
        match self.reg.lookup(self.here?, crate::registry::Ns::Value, segments) {
            Ok(Some(crate::registry::Def::Value(id))) => Some(id),
            _ => None,
        }
    }

    /// The body of a function this FILE declares, so a const reached through a
    /// `const fn` is reached. A function from anywhere else cannot read a const
    /// of this module before the module has finished loading.
    fn body_of(&self, name: &str) -> Option<&syn::Block> {
        self.file
            .functions
            .iter()
            .find(|f| f.name == name)
            .and_then(|f| f.body_ast.as_ref())
    }
}

struct Walk<'a, 'r> {
    reads: &'a Reads<'r>,
    wanted: crate::registry::ValueId,
    followed: &'a mut Vec<String>,
    found: bool,
}

impl syn::visit::Visit<'_> for Walk<'_, '_> {
    fn visit_path(&mut self, path: &syn::Path) {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        if self.reads.value_id(&segments) == Some(self.wanted) {
            self.found = true;
        }
        // A call to a function this file declares: its body runs when the const
        // is initialised, so what it reads is what the const needs first. The
        // `followed` list stops a recursive one from walking for ever.
        if let Some(leaf) = segments.last() {
            if !self.followed.iter().any(|f| f == leaf) {
                if let Some(body) = self.reads.body_of(leaf) {
                    self.followed.push(leaf.clone());
                    let mut inner = Walk {
                        reads: self.reads,
                        wanted: self.wanted,
                        followed: self.followed,
                        found: false,
                    };
                    syn::visit::Visit::visit_block(&mut inner, body);
                    if inner.found {
                        self.found = true;
                    }
                }
            }
        }
        syn::visit::visit_path(self, path);
    }
}

/// The old question, by leaf name, for a const the registry does not carry.
fn names_leaf(init: &syn::Expr, rust_name: &str) -> bool {
    struct Paths<'n> {
        wanted: &'n str,
        found: bool,
    }
    impl syn::visit::Visit<'_> for Paths<'_> {
        fn visit_path(&mut self, path: &syn::Path) {
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

#[cfg(test)]
mod j6_tests {
    use crate::testing::Fixture;

    /// J6: a const reached through a `const fn` this file declares has to be
    /// written first. `const A: u32 = double(B);` names no `B` in its own
    /// expression, so `A` could stand above `B` and the module failed to load
    /// with `Cannot access 'B' before initialization`.
    #[test]
    fn a_const_reached_through_a_const_fn_is_written_first() {
        let mut c = Fixture::build(&[(
            "lib.rs",
            "pub const fn double(n: u32) -> u32 { n * 2 }\n\
             pub const A: u32 = double(B);\n\
             pub const B: u32 = 3;\n",
        )]);
        let ts = c.emitted("lib.rs");
        let a = ts.find("const A").expect("A is emitted");
        let b = ts.find("const B").expect("B is emitted");
        assert!(b < a, "B stands before A:\n{}", ts);
    }

    /// And the other side: a path that only SHARES a leaf with a local const is
    /// not a dependency on it. Matching the last segment ordered
    /// `const B = other::A;` against the local `A`, which is a dependency that
    /// does not exist.
    #[test]
    fn a_foreign_const_with_the_same_leaf_is_not_the_local_one() {
        let mut c = Fixture::build(&[
            ("lib.rs", "pub mod other;\npub mod here;\n"),
            ("other.rs", "pub const A: u32 = 1;\n"),
            (
                "here.rs",
                "pub const A: u32 = B + 1;\n\
                 pub const B: u32 = crate::other::A;\n",
            ),
        ]);
        let ts = c.emitted("here.rs");
        let b = ts.find("const B").expect("B is emitted");
        let a = ts.find("const A").expect("A is emitted");
        // `A` reads `B`, and `B` reads the OTHER module's `A`. Read by leaf
        // those are two dependencies, so a cycle, so source order — which puts
        // `A` first and throws. Read by identity there is one dependency, and
        // `B` goes first because `A` needs it.
        assert!(b < a, "B stands before A, and for the right reason:\n{}", ts);
    }
}
