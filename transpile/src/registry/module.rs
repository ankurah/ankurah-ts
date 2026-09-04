//! The module tree a name is looked up in.
//!
//! Rust resolves a bare `Ref` to the `Ref` its own module declares, and only
//! reaches `std::cell::Ref` through an import or the prelude. The registry
//! mirrors that: every item belongs to a module and to a namespace, carries the
//! visibility it was written with, and is reached by walking the module's own
//! items and its `use` bindings. There is no crate-wide table of leaf names, so
//! nothing can be evicted from one and nothing shadows anything it does not
//! shadow in Rust.

use std::collections::{BTreeMap, HashMap};

use crate::ty::TypeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub u32);

/// A type alias, expanded where it is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AliasId(pub u32);

/// A function, constant, static, or other value item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValueId(pub u32);

/// The namespaces this engine reads. A name means different things in each:
/// `struct Foo;` declares `Foo` in the type namespace and `const FOO` declares
/// `FOO` in the value one, so `use anyhow::anyhow` — a function and a macro —
/// declares nothing a written type can resolve to. Rust's third namespace,
/// macros, arrives with the macro step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Ns {
    Type,
    Value,
}

/// What a resolved name turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Def {
    Type(TypeId),
    Alias(AliasId),
    Value(ValueId),
    /// A `mod`, reached when a path walks through it.
    Module(ModuleId),
}

/// How far a declaration can be seen, as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vis {
    Public,
    /// `pub(crate)`, and `pub(in path)` which this engine does not narrow further.
    Crate,
    /// `pub(super)` — visible within the named module and everything under it.
    Restricted(ModuleId),
    /// No modifier: visible in the declaring module and its descendants.
    Private,
}

#[derive(Debug, Clone)]
pub struct ItemDef {
    pub def: Def,
    pub vis: Vis,
}

/// One name a `use` statement brings into a module.
#[derive(Debug, Clone)]
pub struct UseBinding {
    /// The name this binding introduces. `None` for `use path::*`.
    pub local: Option<String>,
    /// The path as written, e.g. `["crate", "broadcast", "Broadcast"]`.
    pub path: Vec<String>,
    /// The `use` item's own visibility: a `pub use` re-exports, a plain `use`
    /// is private to the module that wrote it.
    pub vis: Vis,
}

#[derive(Debug)]
pub struct ModuleDef {
    pub parent: Option<ModuleId>,
    /// Path from the root of its own tree.
    pub path: Vec<String>,
    /// True for the reserved module holding the declared system types.
    pub is_system: bool,
    /// The visibility of the `mod` declaration itself.
    pub vis: Vis,
    pub children: BTreeMap<String, ModuleId>,
    /// Items declared directly here, per namespace.
    pub items: HashMap<(Ns, String), ItemDef>,
    pub uses: Vec<UseBinding>,
}

impl ModuleDef {
    pub fn item(&self, ns: Ns, name: &str) -> Option<&ItemDef> {
        self.items.get(&(ns, name.to_string()))
    }
}

/// The crate's modules plus the reserved system module.
#[derive(Debug)]
pub struct ModuleTree {
    modules: Vec<ModuleDef>,
    crate_root: ModuleId,
    system_root: ModuleId,
}

impl ModuleTree {
    pub fn new() -> Self {
        let mut tree = ModuleTree {
            modules: Vec::new(),
            crate_root: ModuleId(0),
            system_root: ModuleId(1),
        };
        tree.crate_root = tree.push(None, Vec::new(), false);
        tree.system_root = tree.push(None, Vec::new(), true);
        tree
    }

    fn push(&mut self, parent: Option<ModuleId>, path: Vec<String>, is_system: bool) -> ModuleId {
        let id = ModuleId(self.modules.len() as u32);
        self.modules.push(ModuleDef {
            parent,
            path,
            is_system,
            vis: Vis::Public,
            children: BTreeMap::new(),
            items: HashMap::new(),
            uses: Vec::new(),
        });
        id
    }

    pub fn crate_root(&self) -> ModuleId {
        self.crate_root
    }
    pub fn system_root(&self) -> ModuleId {
        self.system_root
    }

    pub fn get(&self, id: ModuleId) -> &ModuleDef {
        &self.modules[id.0 as usize]
    }
    pub fn get_mut(&mut self, id: ModuleId) -> &mut ModuleDef {
        &mut self.modules[id.0 as usize]
    }

    /// Is `module` inside `ancestor`, or the same module?
    pub fn is_within(&self, module: ModuleId, ancestor: ModuleId) -> bool {
        let mut cursor = Some(module);
        while let Some(id) = cursor {
            if id == ancestor {
                return true;
            }
            cursor = self.get(id).parent;
        }
        false
    }

    /// The child module of `parent` called `name`, creating it if it is new.
    pub fn child(&mut self, parent: ModuleId, name: &str) -> ModuleId {
        if let Some(&existing) = self.get(parent).children.get(name) {
            return existing;
        }
        let mut path = self.get(parent).path.clone();
        path.push(name.to_string());
        let is_system = self.get(parent).is_system;
        let id = self.push(Some(parent), path, is_system);
        self.get_mut(parent).children.insert(name.to_string(), id);
        id
    }

    /// The module a source file declares, creating the chain to it.
    /// `lib.rs` and `main.rs` are the crate root, `foo/mod.rs` is `foo`, and
    /// `foo/bar.rs` is `foo::bar`.
    pub fn module_for_file(&mut self, rel_path: &str) -> ModuleId {
        let mut module = self.crate_root;
        for segment in file_module_path(rel_path) {
            module = self.child(module, &segment);
        }
        module
    }

    /// The module a written path names under `root`, creating the chain to it.
    /// This is how the std surface's `std::collections::hash_map` is built.
    pub fn module_for_path(&mut self, root: ModuleId, path: &[String]) -> ModuleId {
        let mut module = root;
        for segment in path {
            module = self.child(module, segment);
        }
        module
    }

    /// Bind `name` in `parent` to a module that already exists, the way `core`
    /// and `alloc` name the same items `std` does.
    pub fn alias_child(&mut self, parent: ModuleId, name: &str, target: ModuleId) {
        self.get_mut(parent).children.insert(name.to_string(), target);
    }

    /// The crates the surface declares: the children of the system root, which
    /// is where a path leaving the crate is looked up.
    pub fn system_crates(&self) -> &BTreeMap<String, ModuleId> {
        &self.get(self.system_root).children
    }

    /// The module a source file declares, if the tree already has it.
    pub fn lookup_file(&self, rel_path: &str) -> Option<ModuleId> {
        let mut module = self.crate_root;
        for segment in file_module_path(rel_path) {
            module = *self.get(module).children.get(&segment)?;
        }
        Some(module)
    }
}

fn file_module_path(rel_path: &str) -> Vec<String> {
    let stem = rel_path.trim_end_matches(".rs");
    let mut segments: Vec<String> = stem
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .map(|s| s.to_string())
        .collect();
    match segments.last().map(|s| s.as_str()) {
        Some("mod") => {
            segments.pop();
        }
        Some("lib") | Some("main") if segments.len() == 1 => segments.clear(),
        _ => {}
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_paths_become_module_paths() {
        assert!(file_module_path("lib.rs").is_empty());
        assert_eq!(file_module_path("signal.rs"), vec!["signal"]);
        assert_eq!(file_module_path("signal/memo.rs"), vec!["signal", "memo"]);
        assert_eq!(file_module_path("selection/mod.rs"), vec!["selection"]);
        assert_eq!(file_module_path("a/b/lib.rs"), vec!["a", "b", "lib"]);
    }

    #[test]
    fn child_modules_are_created_once() {
        let mut tree = ModuleTree::new();
        let a = tree.module_for_file("signal/memo.rs");
        let b = tree.module_for_file("signal/memo.rs");
        assert_eq!(a, b);
        assert_eq!(tree.get(a).path, vec!["signal", "memo"]);
        assert!(!tree.get(a).is_system);
        assert!(tree.get(tree.system_root()).is_system);
    }

    #[test]
    fn containment_walks_up_the_tree() {
        let mut tree = ModuleTree::new();
        let memo = tree.module_for_file("signal/memo.rs");
        let signal = tree.lookup_file("signal.rs").unwrap();
        let other = tree.module_for_file("context.rs");
        assert!(tree.is_within(memo, signal));
        assert!(tree.is_within(memo, tree.crate_root()));
        assert!(!tree.is_within(memo, other));
        assert!(tree.is_within(signal, signal));
    }
}
