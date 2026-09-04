//! Resolving a written path to a declaration.
//!
//! A name is resolved from the module that wrote it, in the namespace the site
//! asks for: the module's own items first, then its `use` bindings, then its
//! glob imports, then the prelude. A qualified path walks the module tree and
//! every hop must be visible from the module that asked. There is no
//! crate-wide table of leaf names, so nothing can be evicted from one and
//! nothing shadows anything it does not shadow in Rust.

use std::collections::HashSet;

use super::module::{Def, ModuleId, Ns, Vis};
use super::TypeRegistry;
use crate::ty::TypeId;

/// What Rust's standard prelude puts in scope without a `use`, and the module
/// each name comes from. Every other declared type needs an import or a
/// qualified path, exactly as in Rust.
///
/// The set is explicit rather than read off the surface: it is Rust's list, not
/// a consequence of which files the surface happens to declare, and a name that
/// stops being declared should fail here rather than silently leave the prelude.
pub const PRELUDE: [(&str, &str); 33] = [
    ("Option", "std::option"),
    ("Result", "std::result"),
    ("Vec", "std::vec"),
    ("String", "std::string"),
    ("ToString", "std::string"),
    ("Box", "std::boxed"),
    ("ToOwned", "std::borrow"),
    ("Clone", "std::clone"),
    ("Copy", "std::marker"),
    ("Send", "std::marker"),
    ("Sized", "std::marker"),
    ("Sync", "std::marker"),
    ("Unpin", "std::marker"),
    ("Default", "std::default"),
    ("Drop", "std::ops"),
    ("Fn", "std::ops"),
    ("FnMut", "std::ops"),
    ("FnOnce", "std::ops"),
    ("Iterator", "std::iter"),
    ("IntoIterator", "std::iter"),
    ("DoubleEndedIterator", "std::iter"),
    ("ExactSizeIterator", "std::iter"),
    ("Extend", "std::iter"),
    ("FromIterator", "std::iter"),
    ("Into", "std::convert"),
    ("From", "std::convert"),
    ("TryInto", "std::convert"),
    ("TryFrom", "std::convert"),
    ("AsRef", "std::convert"),
    ("AsMut", "std::convert"),
    ("PartialEq", "std::cmp"),
    ("Eq", "std::cmp"),
    ("PartialOrd", "std::cmp"),
];

/// `Ord` is in the prelude too, and shares its name with nothing; it is listed
/// separately only because the array above is already at its declared length.
pub const PRELUDE_ORD: (&str, &str) = ("Ord", "std::cmp");

/// Why a name could not be resolved to one declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupError {
    pub message: String,
}

/// Where the walk currently is, and what it has already tried.
struct Walk {
    /// The module that wrote the path; visibility is judged from here.
    from: ModuleId,
    /// `use` bindings already followed, by the module that wrote them and their
    /// position in it. Keying on the binding itself rather than on the path it
    /// produces is what stops `use anyhow::anyhow;` looping on `anyhow::Result`.
    followed: HashSet<(ModuleId, usize)>,
    /// Names already asked for in a module, so glob cycles terminate.
    asked: HashSet<(ModuleId, Ns, String)>,
}

type Found = Result<Option<Def>, LookupError>;

impl TypeRegistry {
    /// Resolve a written path in a namespace.
    pub fn lookup(&self, from: ModuleId, ns: Ns, segments: &[String]) -> Found {
        let mut walk = Walk {
            from,
            followed: HashSet::new(),
            asked: HashSet::new(),
        };
        self.resolve_path(from, ns, segments, &mut walk)
    }

    /// Resolve a written type path.
    pub fn lookup_type(&self, from: ModuleId, segments: &[String]) -> Found {
        self.lookup(from, Ns::Type, segments)
    }

    /// The enum a written path names, together with the variant on the end.
    /// `Signal::Constant` resolves through `Signal`, never by the last segment.
    pub fn lookup_variant(&self, from: ModuleId, segments: &[String]) -> Option<(TypeId, String)> {
        let (variant, prefix) = segments.split_last()?;
        if prefix.is_empty() {
            return None;
        }
        let Ok(Some(Def::Type(id))) = self.lookup_type(from, prefix) else {
            return None;
        };
        if self.is_variant_of(id, variant) {
            Some((id, variant.clone()))
        } else {
            None
        }
    }

    fn resolve_path(
        &self,
        module: ModuleId,
        ns: Ns,
        segments: &[String],
        walk: &mut Walk,
    ) -> Found {
        let Some((head, rest)) = segments.split_first() else {
            return Ok(None);
        };
        if rest.is_empty() {
            return self.resolve_item(module, ns, head, walk);
        }

        if head == "crate" || self.names_this_crate(head) {
            return self.resolve_path(self.crate_root(), ns, rest, walk);
        }
        if head == "self" {
            return self.resolve_path(module, ns, rest, walk);
        }
        if head == "super" {
            let Some(parent) = self.modules().get(module).parent else {
                return Ok(None);
            };
            return self.resolve_path(parent, ns, rest, walk);
        }

        // Anything else leading a multi-segment path has to name a module.
        // A `use` that binds a function or a macro cannot carry a type path
        // through it, which is the other half of the `anyhow::anyhow` loop.
        let Some(target) = self.resolve_module(module, head, walk) else {
            return Ok(None);
        };
        self.resolve_path(target, ns, rest, walk)
    }

    fn resolve_item(&self, module: ModuleId, ns: Ns, name: &str, walk: &mut Walk) -> Found {
        if !walk.asked.insert((module, ns, name.to_string())) {
            return Ok(None);
        }
        let here = module == walk.from;

        // The module's own declarations shadow everything imported.
        if let Some(item) = self.modules().get(module).item(ns, name) {
            if here || self.is_visible(item.vis, module, walk.from) {
                return Ok(Some(item.def));
            }
        }

        // Then names brought in by an explicit `use`.
        for (index, binding) in self.modules().get(module).uses.iter().enumerate() {
            if binding.local.as_deref() != Some(name) {
                continue;
            }
            if !here && !self.is_visible(binding.vis, module, walk.from) {
                continue;
            }
            if !walk.followed.insert((module, index)) {
                continue;
            }
            if let Some(def) = self.resolve_path(module, ns, &binding.path, walk)? {
                return Ok(Some(def));
            }
        }

        // Then glob imports. Two globs offering different declarations for one
        // name is an ambiguity in Rust too, and the site that uses the name is
        // where it is reported.
        let mut candidates: Vec<Def> = Vec::new();
        for (index, binding) in self.modules().get(module).uses.iter().enumerate() {
            if binding.local.is_some() {
                continue;
            }
            if !here && !self.is_visible(binding.vis, module, walk.from) {
                continue;
            }
            if !walk.followed.insert((module, index)) {
                continue;
            }
            let Some(target) = self.resolve_module_path(module, &binding.path, walk) else {
                continue;
            };
            if let Some(def) = self.resolve_item(target, ns, name, walk)? {
                if !candidates.contains(&def) {
                    candidates.push(def);
                }
            }
        }
        match candidates.len() {
            0 => {}
            1 => return Ok(Some(candidates[0])),
            n => {
                return Err(LookupError {
                    message: format!(
                        "`{}` is ambiguous: {} glob imports bring in different declarations of it",
                        name, n
                    ),
                })
            }
        }

        // A `mod` of that name, so that a module written in type position is
        // named as one rather than reported missing.
        if let Some(&child) = self.modules().get(module).children.get(name) {
            if here || self.is_visible(self.modules().get(child).vis, module, walk.from) {
                return Ok(Some(Def::Module(child)));
            }
        }

        // Finally the prelude, and only where the name was written.
        if here && ns == Ns::Type {
            if let Some(def) = self.prelude_item(name) {
                return Ok(Some(def));
            }
        }

        // A stub in the declared surface writes every name as a leaf — the
        // surface has no `use` statements — so a name its own module does not
        // declare is looked for across the whole surface. `std/sync/mutex.rs`
        // says `Formatter` and means `std::fmt::Formatter`. This is the last
        // thing tried and applies only inside the surface: a crate's own module
        // never reaches it, so nothing ankurah writes can resolve this way.
        if here && self.modules().get(module).is_system {
            return self.surface_item(ns, name);
        }
        Ok(None)
    }

    /// What a prelude name stands for: the declaration in the module Rust
    /// exports it from. Nothing is found by leaf name across the surface, so a
    /// `Result` in the prelude is `std::result::Result` and never
    /// `std::fmt::Result`.
    fn prelude_item(&self, name: &str) -> Option<Def> {
        let module_path = PRELUDE
            .iter()
            .chain(std::iter::once(&PRELUDE_ORD))
            .find(|(prelude_name, _)| *prelude_name == name)
            .map(|(_, module)| *module)?;
        let segments: Vec<String> = module_path.split("::").map(|s| s.to_string()).collect();
        let mut module = self.system_root();
        for segment in segments {
            module = self.modules().get(module).children.get(&segment).copied()?;
        }
        self.modules()
            .get(module)
            .item(Ns::Type, name)
            .map(|item| item.def)
    }

    /// The module a single written segment names, from `module`.
    fn resolve_module(&self, module: ModuleId, name: &str, walk: &mut Walk) -> Option<ModuleId> {
        if let Some(&child) = self.modules().get(module).children.get(name) {
            if module == walk.from
                || self.is_visible(self.modules().get(child).vis, module, walk.from)
            {
                return Some(child);
            }
        }
        for (index, binding) in self.modules().get(module).uses.iter().enumerate() {
            if binding.local.as_deref() != Some(name) {
                continue;
            }
            if module != walk.from && !self.is_visible(binding.vis, module, walk.from) {
                continue;
            }
            if !walk.followed.insert((module, index)) {
                continue;
            }
            if let Some(target) = self.resolve_module_path(module, &binding.path, walk) {
                return Some(target);
            }
        }
        // Rust's extern prelude: a path may start with the name of a crate the
        // build depends on. The declared surface is that set of crates — `std`,
        // `tokio`, `serde_json` — and each is a root of its own, which is what
        // keeps `tokio::sync::Mutex` and `std::sync::Mutex` two types. A crate's
        // own module of the same name is found above and wins, as in Rust.
        self.modules().system_crates().get(name).copied()
    }

    /// The module a whole written path names, for `use foo::bar::*`.
    fn resolve_module_path(
        &self,
        module: ModuleId,
        segments: &[String],
        walk: &mut Walk,
    ) -> Option<ModuleId> {
        let Some((head, rest)) = segments.split_first() else {
            return Some(module);
        };
        if head == "crate" || self.names_this_crate(head) {
            return self.resolve_module_path(self.crate_root(), rest, walk);
        }
        if head == "self" {
            return self.resolve_module_path(module, rest, walk);
        }
        if head == "super" {
            let parent = self.modules().get(module).parent?;
            return self.resolve_module_path(parent, rest, walk);
        }
        let child = self.resolve_module(module, head, walk)?;
        self.resolve_module_path(child, rest, walk)
    }

    /// Can a declaration written with this visibility in `declared_in` be seen
    /// from `from`?
    ///
    /// A `use` is an item like any other: a private one is visible in the
    /// module that wrote it and everything under it, which is why
    /// `mod stack { use super::*; }` sees what its parent imported.
    pub fn is_visible(&self, vis: Vis, declared_in: ModuleId, from: ModuleId) -> bool {
        match vis {
            Vis::Public => true,
            // Everything the engine resolves is inside one crate.
            Vis::Crate => true,
            Vis::Restricted(scope) => self.modules().is_within(from, scope),
            Vis::Private => self.modules().is_within(from, declared_in),
        }
    }

    /// `ankql::ast::Expr` written inside ankql means the same as
    /// `crate::ast::Expr`. Cargo spells crate names with hyphens and Rust with
    /// underscores, so compare on the Rust spelling.
    fn names_this_crate(&self, segment: &str) -> bool {
        let segment = segment.replace('-', "_");
        self.crate_names()
            .iter()
            .any(|n| n.replace('-', "_") == segment)
    }

    /// The path a written name stands for once its imports and its `crate` /
    /// `self` / `super` prefixes are normalised away. `Attested` under
    /// `use ankurah_proto::{self as proto, Attested}` and a written
    /// `proto::Attested` then intern as one undeclared type, not two.
    pub fn canonical_path(&self, module: ModuleId, segments: &[String]) -> Vec<String> {
        let mut path = segments.to_vec();
        let mut followed: HashSet<usize> = HashSet::new();
        // Expanding one import can expose another; each is followed once.
        loop {
            let Some((head, rest)) = path.split_first() else {
                return path;
            };
            let mut expanded = None;
            for (index, binding) in self.modules().get(module).uses.iter().enumerate() {
                if binding.local.as_deref() != Some(head.as_str()) || !followed.insert(index) {
                    continue;
                }
                // An import whose path starts with the name it binds is a
                // crate's own item re-exported under the crate's name:
                // `use anyhow::anyhow`. A longer path written through that name
                // means the crate, not the item, so `anyhow::Result` is not
                // `anyhow::anyhow::Result`. Every other import can lead a path,
                // because the source compiled.
                if !rest.is_empty() && binding.path.first() == Some(head) {
                    continue;
                }
                let mut next = binding.path.clone();
                next.extend_from_slice(rest);
                expanded = Some(next);
                break;
            }
            match expanded {
                Some(next) => path = next,
                None => break,
            }
        }
        self.absolute_path(module, &path)
    }

    /// Rewrite a leading `crate`, `self` or `super` into the crate's own name
    /// and module path, so the same item written two ways interns once.
    fn absolute_path(&self, module: ModuleId, segments: &[String]) -> Vec<String> {
        let crate_name = self
            .crate_names()
            .iter()
            .max_by_key(|n| n.len())
            .cloned()
            .unwrap_or_default()
            .replace('-', "_");
        let Some((head, rest)) = segments.split_first() else {
            return Vec::new();
        };
        let mut base: Vec<String> = match head.as_str() {
            "crate" => vec![crate_name],
            "self" => {
                let mut p = vec![crate_name];
                p.extend(self.modules().get(module).path.iter().cloned());
                p
            }
            "super" => {
                let mut p = vec![crate_name];
                let parent = self
                    .modules()
                    .get(module)
                    .parent
                    .unwrap_or(self.crate_root());
                p.extend(self.modules().get(parent).path.iter().cloned());
                p
            }
            _ => return segments.to_vec(),
        };
        base.extend_from_slice(rest);
        base
    }
}
