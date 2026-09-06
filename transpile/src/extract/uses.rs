//! What a `use` binds, and where.
//!
//! Rust's `use` is the whole of a module's imported surface, and two of its
//! shapes need saying out loud. A `use` inside a function BODY is scoped to the
//! block it stands in, where the engine's binding table is per module: the
//! bindings are hoisted, but only where the module does not already claim the
//! name. And `use path::Trait as _;` puts a trait in scope for method
//! resolution while binding no name at all — the reason for writing it that
//! way, and the reason a lookup by name cannot find it.

use super::*;

#[derive(Debug)]
pub struct UseInfo {
    pub path: String,
    pub vis: VisInfo,
    /// What this `use` binds in its module, one entry per imported name.
    pub bindings: Vec<UseBindingInfo>,
    /// Written inside a function BODY rather than at module level. Rust scopes
    /// such a `use` to its block; the engine has one binding table per module,
    /// so the binding is hoisted — but only where the module does not already
    /// claim the name, since widening a name's scope must not change what
    /// another body in the same module means by it.
    pub from_body: bool,
    /// Where the `use` was written. A report about a `use` — a glob a body
    /// wrote, a name two bodies contest — carries this, because a diagnostic at
    /// `Span::call_site()` reaches the reader with no file and no line (N20).
    pub span: proc_macro2::Span,
}

/// One name a `use` brings into scope. `local` is `None` for `use path::*`.
#[derive(Debug, Clone)]
pub struct UseBindingInfo {
    pub local: Option<String>,
    pub path: Vec<String>,
}

/// A `use` written INSIDE a function body, recorded here and HOISTED by
/// `registry::uses::module_use_bindings`, which is where the rule for it and
/// the reason for the rule are written down. The extractor's whole part is to
/// keep the flag: a body `use` that arrived indistinguishable from a
/// module-level one would be hoisted unconditionally.
struct BodyUses<'f> {
    uses: &'f mut Vec<UseInfo>,
}

impl syn::visit::Visit<'_> for BodyUses<'_> {
    fn visit_stmt(&mut self, stmt: &syn::Stmt) {
        if let syn::Stmt::Item(syn::Item::Use(u)) = stmt {
            self.uses.push(UseInfo { from_body: true, ..extract_use(u) });
        }
        syn::visit::visit_stmt(self, stmt);
    }
}

/// Every `use` statement in a body, at any block depth.
pub(super) fn body_uses(block: &syn::Block, into: &mut Vec<UseInfo>) {
    use syn::visit::Visit;
    BodyUses { uses: into }.visit_block(block);
}

pub(super) fn extract_use(u: &syn::ItemUse) -> UseInfo {
    let mut bindings = Vec::new();
    collect_use_bindings(&u.tree, &mut Vec::new(), &mut bindings);
    UseInfo {
        path: use_tree_to_string(&u.tree),
        vis: visibility(&u.vis),
        bindings,
        from_body: false,
        span: syn::spanned::Spanned::span(u),
    }
}

/// Flatten a `use` tree into the names it binds. `use a::{b, c as d}` binds `b`
/// to `a::b` and `d` to `a::c`; `use a::*` binds nothing under a name and is
/// recorded as a glob over `a`.
fn collect_use_bindings(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<UseBindingInfo>) {
    match tree {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            collect_use_bindings(&p.tree, prefix, out);
            prefix.pop();
        }
        // `use a::b::{self}` binds `b` to `a::b`, not a name called "self".
        syn::UseTree::Name(n) if n.ident == "self" => {
            if let Some(parent) = prefix.last().cloned() {
                out.push(UseBindingInfo { local: Some(parent), path: prefix.clone() });
            }
        }
        syn::UseTree::Rename(r) if r.ident == "self" => {
            out.push(UseBindingInfo { local: Some(r.rename.to_string()), path: prefix.clone() });
        }
        syn::UseTree::Name(n) => {
            let mut path = prefix.clone();
            path.push(n.ident.to_string());
            out.push(UseBindingInfo { local: Some(n.ident.to_string()), path });
        }
        syn::UseTree::Rename(r) => {
            let mut path = prefix.clone();
            path.push(r.ident.to_string());
            out.push(UseBindingInfo { local: Some(r.rename.to_string()), path });
        }
        syn::UseTree::Glob(_) => {
            out.push(UseBindingInfo { local: None, path: prefix.clone() });
        }
        syn::UseTree::Group(g) => {
            for item in &g.items {
                collect_use_bindings(item, prefix, out);
            }
        }
    }
}

fn use_tree_to_string(tree: &syn::UseTree) -> String {
    match tree {
        syn::UseTree::Path(p) => format!("{}::{}", p.ident, use_tree_to_string(&p.tree)),
        syn::UseTree::Name(n) => n.ident.to_string(),
        syn::UseTree::Rename(r) => format!("{} as {}", r.ident, r.rename),
        syn::UseTree::Glob(_) => "*".to_string(),
        syn::UseTree::Group(g) => {
            let items: Vec<String> = g.items.iter().map(|t| use_tree_to_string(t)).collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}
