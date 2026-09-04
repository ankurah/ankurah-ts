//! What the corpus is made of, counted with syn (spec section 6.2).
//!
//! Every capability the engine grows is justified by how often the corpus uses
//! the construct it serves. Those counts are only an argument if the corpus
//! holds still, so they are written down in `tests/inventory.toml` and this test
//! fails when a fresh count disagrees — a `git pull` in the Rust checkout that
//! adds forty method calls should be a decision, not a surprise.
//!
//! Two honest limits. Constructs inside a macro invocation are not counted:
//! syn hands us the macro's tokens unparsed, and the transpiler does not expand
//! macros either, so the macro tally below is by name and that is all. And
//! `cfg`-gated code is counted whether or not the transpiler would keep it,
//! because this measures the corpus, not the configured build.
//!
//! Refresh after a deliberate corpus update:
//!
//!     cd transpile && UPDATE_INVENTORY=1 cargo test --test corpus_inventory

mod common;

use common::{collect_files_with_ext, support_tree, transpile_dir};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use syn::visit::Visit;

/// Crate label, then the source directory under the support checkout.
const CRATES: [(&str, &str); 5] = [
    ("proto", "proto/src"),
    ("ankql", "ankql/src"),
    ("signals", "signals/src"),
    ("core", "core/src"),
    ("storage-common", "storage/common/src"),
];

#[test]
fn inventory_matches() {
    let actual = render(&count_all());
    let path = transpile_dir().join("tests/inventory.toml");

    if std::env::var_os("UPDATE_INVENTORY").is_some() {
        std::fs::write(&path, &actual).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
        eprintln!("updated {}", path.display());
        return;
    }

    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}\nCreate it with UPDATE_INVENTORY=1 cargo test", path.display()));
    let diff = common::unified_diff("tests/inventory.toml", &expected, &actual);
    assert!(
        diff.is_empty(),
        "the corpus changed under us:\n\n{diff}\n\
         If the Rust checkout moved on purpose, refresh with:\n    \
         cd transpile && UPDATE_INVENTORY=1 cargo test --test corpus_inventory"
    );
}

fn count_all() -> Vec<(&'static str, Counts)> {
    let support = support_tree();
    CRATES
        .iter()
        .map(|(name, rel)| {
            let dir = support.join(rel);
            assert!(dir.is_dir(), "no such corpus directory: {}", dir.display());
            let mut counts = Counts::default();
            for (rel_path, text) in collect_files_with_ext(&dir, Some("rs")) {
                let file = syn::parse_file(&text)
                    .unwrap_or_else(|e| panic!("syn cannot parse {}/{rel_path}: {e}", dir.display()));
                counts.files += 1;
                counts.visit_file(&file);
            }
            (*name, counts)
        })
        .collect()
}

#[derive(Default)]
struct Counts {
    files: usize,
    method_calls: usize,
    field_accesses: usize,
    closures: usize,
    try_ops: usize,
    awaits: usize,
    into_calls: usize,
    binary_ops: usize,
    unary_ops: usize,
    match_arms: usize,
    if_lets: usize,
    while_lets: usize,
    trait_impls: usize,
    generic_fns: usize,
    macros: BTreeMap<String, usize>,
}

impl<'ast> Visit<'ast> for Counts {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.method_calls += 1;
        let name = node.method.to_string();
        if name == "into" || name == "try_into" {
            self.into_calls += 1;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        self.field_accesses += 1;
        syn::visit::visit_expr_field(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.closures += 1;
        syn::visit::visit_expr_closure(self, node);
    }

    fn visit_expr_try(&mut self, node: &'ast syn::ExprTry) {
        self.try_ops += 1;
        syn::visit::visit_expr_try(self, node);
    }

    fn visit_expr_await(&mut self, node: &'ast syn::ExprAwait) {
        self.awaits += 1;
        syn::visit::visit_expr_await(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        self.binary_ops += 1;
        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_expr_unary(&mut self, node: &'ast syn::ExprUnary) {
        self.unary_ops += 1;
        syn::visit::visit_expr_unary(self, node);
    }

    fn visit_arm(&mut self, node: &'ast syn::Arm) {
        self.match_arms += 1;
        syn::visit::visit_arm(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        if matches!(*node.cond, syn::Expr::Let(_)) {
            self.if_lets += 1;
        }
        syn::visit::visit_expr_if(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        if matches!(*node.cond, syn::Expr::Let(_)) {
            self.while_lets += 1;
        }
        syn::visit::visit_expr_while(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let name = node
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_else(|| "?".to_string());
        *self.macros.entry(name).or_insert(0) += 1;
        syn::visit::visit_macro(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if node.trait_.is_some() {
            self.trait_impls += 1;
        }
        syn::visit::visit_item_impl(self, node);
    }

    fn visit_signature(&mut self, node: &'ast syn::Signature) {
        // Lifetime-only generics do not need substitution, so they do not count.
        let substitutable = node
            .generics
            .params
            .iter()
            .any(|p| matches!(p, syn::GenericParam::Type(_) | syn::GenericParam::Const(_)));
        if substitutable {
            self.generic_fns += 1;
        }
        syn::visit::visit_signature(self, node);
    }
}

fn render(all: &[(&str, Counts)]) -> String {
    let mut out = String::from(
        "# Corpus construct inventory, counted by transpile/tests/corpus_inventory.rs.\n\
         # Generated: do not hand-edit. Refresh with:\n\
         #     cd transpile && UPDATE_INVENTORY=1 cargo test --test corpus_inventory\n\
         # Constructs inside macro invocations are not counted; macros are tallied by name.\n",
    );
    for (name, c) in all {
        let _ = write!(
            out,
            "\n[{name}]\n\
             files = {}\n\
             method_calls = {}\n\
             field_accesses = {}\n\
             closures = {}\n\
             try_ops = {}\n\
             awaits = {}\n\
             into_calls = {}\n\
             binary_ops = {}\n\
             unary_ops = {}\n\
             match_arms = {}\n\
             if_lets = {}\n\
             while_lets = {}\n\
             trait_impls = {}\n\
             generic_fns = {}\n",
            c.files,
            c.method_calls,
            c.field_accesses,
            c.closures,
            c.try_ops,
            c.awaits,
            c.into_calls,
            c.binary_ops,
            c.unary_ops,
            c.match_arms,
            c.if_lets,
            c.while_lets,
            c.trait_impls,
            c.generic_fns,
        );
        let _ = write!(out, "\n[{name}.macros]\n");
        for (macro_name, n) in &c.macros {
            let _ = writeln!(out, "{macro_name} = {n}");
        }
    }
    out
}
