//! The type registry: every type the transpiler knows, keyed by identity.
//!
//! A type is reached by its id. Names are looked up through the module that
//! wrote them (see `lookup.rs`), in the namespace the site asks for and subject
//! to the visibility the declaration was written with, so two modules can each
//! declare a `Ref` and a crate type can never displace a system type.

mod assoc;
mod bounds;
mod build;
mod describe;
#[cfg(test)]
mod engine_tests;
pub mod impls;
mod lookup;
pub mod method;
#[cfg(test)]
mod method_tests;
mod module;
mod resolve_type;
pub mod std_surface;
mod traits;

use std::cell::RefCell;
use std::collections::HashMap;

use crate::ty::{Ty, TypeId};
use crate::types::SelfKind;

pub use build::{build_registry, resolve_bounds, ExtractedFile};
pub use impls::ImplId;
pub use method::{Callee, FieldResolution, MethodResolution, Probe, Undecided};
pub use module::{AliasId, Def, ModuleId, ModuleTree, Ns, ValueId, Vis};
pub use resolve_type::{resolve_type, TypeEnv};
pub use std_surface::Surface;


/// The Rust path of the trait every dereference goes through. The engine names
/// it structurally — the deref chain is not a list of types but a search of the
/// impl table — so the path is written here rather than looked up by leaf name.
pub const DEREF_PATH: &str = "std::ops::Deref";

/// The Rust path of `Clone`. Every `#[derive(Clone)]` in the corpus registers an
/// impl of it (spec 4.10), so that `guard.clone()` clones what the guard holds
/// rather than the guard.
pub const CLONE_PATH: &str = "std::clone::Clone";

/// What a named type is.
#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct,
    Enum { variants: Vec<VariantDef> },
    Trait,
}

/// An enum's variant, with the types of whatever it carries.
///
/// A tuple variant's fields are named `_0`, `_1`, the way emission writes them,
/// so that `Foo::Bar(x)` in a pattern reads its type off position 0.
#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub fields: Vec<(String, Ty)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodSig {
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    /// How the receiver is taken. `None` is an associated function, which a
    /// method call never reaches.
    pub self_kind: Option<SelfKind>,
    /// The type this method accepts as its receiver, written in terms of the
    /// impl's own parameters — `&Arc<Inner<T>>` for a `&self` method on
    /// `Arc<Inner<T>>`. Method resolution matches this against the receiver it
    /// has, which is the question Rust's probe asks; comparing borrow kinds
    /// instead let a blanket impl win a step early.
    pub receiver: Option<Ty>,
    /// Parameters the method declares on top of its impl's.
    pub type_params: Vec<String>,
    /// What those parameters have to implement. `collect<B: FromIterator<Item>>`
    /// is how a turbofish written as `collect::<Vec<_>>()` gets its element
    /// type: the bound says which `FromIterator` impl `Vec<_>` has to be.
    pub bounds: Vec<impls::Bound>,
}

impl MethodSig {
    pub fn is_static(&self) -> bool {
        self.self_kind.is_none()
    }
}

/// A declared type. Fields and method signatures are filled in after every
/// type has been declared, because resolving them needs the other declarations.
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub module: ModuleId,
    /// The leaf name as Rust writes it: `Broadcast`, `Arc`, `HashMap`.
    pub name: String,
    pub kind: TypeKind,
    /// Field name (in the TypeScript spelling emission uses) to field type.
    pub fields: Vec<(String, Ty)>,
    /// Declared generic parameter names, in order.
    pub type_params: Vec<String>,
    /// What a parameter falls back to where the use site leaves it unwritten:
    /// `HashMap<K, V, S = RandomState>` is declared with three and always
    /// written with two. Positional alongside `type_params`.
    pub param_defaults: Vec<Option<Ty>>,
}

/// What `declare_type` needs. The crate's own structs and enums, the system
/// types, and (from the std-surface step) types parsed out of Rust stub files
/// all arrive through this one door.
#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub name: String,
    pub kind: TypeKind,
    pub type_params: Vec<String>,
    pub vis: Vis,
}

/// A type alias. Aliases are expanded where they are used rather than given an
/// identity of their own, which is what Rust means by them.
#[derive(Debug, Clone)]
pub struct AliasDef {
    pub module: ModuleId,
    pub name: String,
    pub type_params: Vec<String>,
    /// `type Result<T, E = Error> = ..` — the fallback for a parameter the use
    /// site leaves unwritten, as written, resolved where the alias was declared.
    pub param_defaults: Vec<Option<syn::Type>>,
    pub rust_ty: syn::Type,
}

/// A function, constant or static.
#[derive(Debug, Clone)]
pub struct ValueDef {
    pub name: String,
    /// The declared type of a constant or static; `None` for a function, whose
    /// item type this engine does not model.
    pub ty: Option<Ty>,
}

/// Types the corpus names but nothing declares — `ulid::Ulid`, `anyhow::Error`,
/// `serde::Deserializer`. They keep a distinct identity and their written name,
/// and have no members, which is exactly what is known about them. Each one is
/// reported once as a diagnostic; the std-surface step turns the std ones into
/// real declarations.
#[derive(Debug, Default)]
struct ForeignTypes {
    by_path: HashMap<String, TypeId>,
    names: Vec<String>,
    /// The ones a diagnostic was filed for. Marker traits are interned like
    /// anything else but deliberately not reported, so counting every interned
    /// path would give a number the printed list does not account for.
    reported: std::collections::HashSet<TypeId>,
}

#[derive(Debug)]
pub struct TypeRegistry {
    defs: Vec<TypeDef>,
    aliases: Vec<AliasDef>,
    values: Vec<ValueDef>,
    modules: ModuleTree,
    /// The names the crate being transpiled answers to in a written path: the
    /// TypeScript package name the run was given plus the Cargo and Rust
    /// spellings of the crate it maps to.
    crate_names: Vec<String>,
    /// Declared system types by the full Rust path they are declared under, so
    /// `std::sync::Arc` reaches one and `std::io::Result` reaches none.
    system_by_path: HashMap<String, TypeId>,
    /// Every leaf name the declared surface holds, with everything that answers
    /// to it. The surface's files carry no `use` statements, so a stub names
    /// `Formatter` and means whichever module declares one; two answers is an
    /// ambiguity reported where the name was written. Only a module inside the
    /// surface reaches this.
    surface_names: HashMap<(Ns, String), Vec<Def>>,
    /// Every `impl` block, indexed by what it is written for.
    impls: impls::ImplTable,
    /// Every trait declaration, by the id its name resolves to.
    traits: HashMap<TypeId, traits::TraitDef>,
    foreign: RefCell<ForeignTypes>,
    /// Aliases part-way through expansion, so a cycle stops rather than recurses.
    expanding: RefCell<Vec<AliasId>>,
    /// What each declared system type becomes in TypeScript, by identity.
    /// Filled in once the surface is declared, because it is keyed on the ids
    /// the surface's own paths resolved to.
    shapes: crate::name_map::system_shapes::SystemShapes,
}

/// The id spaces are a partition of `u32`; running out of either is a bug in
/// the corpus size, not something to paper over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdSpaceExhausted;

impl std::fmt::Display for IdSpaceExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the type id space is exhausted")
    }
}

impl TypeRegistry {
    /// Does this enum have a variant of that name?
    pub fn is_variant_of(&self, id: TypeId, variant: &str) -> bool {
        matches!(
            self.def(id).map(|d| &d.kind),
            Some(TypeKind::Enum { variants }) if variants.iter().any(|v| v.name == variant)
        )
    }

    /// The payload a variant carries, by the field names emission writes.
    pub fn variant_fields(&self, id: TypeId, variant: &str) -> Option<&[(String, Ty)]> {
        let TypeKind::Enum { variants } = &self.def(id)?.kind else {
            return None;
        };
        variants
            .iter()
            .find(|v| v.name == variant)
            .map(|v| v.fields.as_slice())
    }

    pub fn new(crate_name: &str) -> Self {
        TypeRegistry {
            defs: Vec::new(),
            aliases: Vec::new(),
            values: Vec::new(),
            modules: ModuleTree::new(),
            crate_names: vec![crate_name.to_string()],
            system_by_path: HashMap::new(),
            surface_names: HashMap::new(),
            impls: impls::ImplTable::default(),
            traits: HashMap::new(),
            foreign: RefCell::new(ForeignTypes::default()),
            expanding: RefCell::new(Vec::new()),
            shapes: Default::default(),
        }
    }

    /// Bind the emission policy to the ids the declared surface produced. Until
    /// this runs no system type has a TypeScript shape, which is the truth
    /// before the surface is in.
    pub(super) fn resolve_shapes(&mut self) {
        self.shapes = crate::name_map::system_shapes::SystemShapes::resolve(self);
    }

    /// File every blanket impl under the method names it can answer to.
    ///
    /// A blanket impl is written for one of its own parameters, so it matches
    /// every receiver and every method call had to unify against all of them.
    /// The names come from the impl's own methods and, where it inherits a
    /// default body, from the trait's declarations — which is why this runs
    /// after both are resolved.
    pub(super) fn index_blankets(&mut self) {
        let mut index: HashMap<String, Vec<impls::ImplId>> = HashMap::new();
        for &id in self.impls.blanket() {
            let def = self.impls.get(id);
            let mut names: Vec<String> = def.methods.keys().cloned().collect();
            if let Some(tr) = &def.trait_ref {
                if let Some(declared) = self.traits.get(&tr.id) {
                    names.extend(declared.methods.keys().cloned());
                    for supertrait in &declared.supertraits {
                        if let Some(sup) = self.traits.get(&supertrait.id) {
                            names.extend(sup.methods.keys().cloned());
                        }
                    }
                }
            }
            names.sort();
            names.dedup();
            for name in names {
                index.entry(name).or_default().push(id);
            }
        }
        self.impls.set_blanket_index(index);
    }

    pub fn shapes(&self) -> &crate::name_map::system_shapes::SystemShapes {
        &self.shapes
    }

    pub fn crate_names(&self) -> &[String] {
        &self.crate_names
    }

    pub fn add_crate_name(&mut self, name: &str) {
        if !self.crate_names.iter().any(|n| n == name) {
            self.crate_names.push(name.to_string());
        }
    }

    pub fn modules(&self) -> &ModuleTree {
        &self.modules
    }
    pub fn modules_mut(&mut self) -> &mut ModuleTree {
        &mut self.modules
    }

    pub fn crate_root(&self) -> ModuleId {
        self.modules.crate_root()
    }
    pub fn system_root(&self) -> ModuleId {
        self.modules.system_root()
    }

    /// Register a type in a module. This is the only way a type enters the
    /// registry, whatever declared it.
    pub fn declare_type(
        &mut self,
        module: ModuleId,
        decl: TypeDecl,
    ) -> Result<TypeId, IdSpaceExhausted> {
        let index = u32::try_from(self.defs.len()).map_err(|_| IdSpaceExhausted)?;
        if index >= TypeId::FOREIGN_BASE {
            return Err(IdSpaceExhausted);
        }
        let id = TypeId(index);
        self.modules.get_mut(module).items.insert(
            (Ns::Type, decl.name.clone()),
            module::ItemDef {
                def: Def::Type(id),
                vis: decl.vis,
            },
        );
        self.defs.push(TypeDef {
            module,
            name: decl.name,
            kind: decl.kind,
            fields: Vec::new(),
            param_defaults: vec![None; decl.type_params.len()],
            type_params: decl.type_params,
        });
        Ok(id)
    }

    pub fn declare_alias(&mut self, module: ModuleId, def: AliasDef, vis: Vis) -> AliasId {
        let id = AliasId(self.aliases.len() as u32);
        self.modules.get_mut(module).items.insert(
            (Ns::Type, def.name.clone()),
            module::ItemDef {
                def: Def::Alias(id),
                vis,
            },
        );
        self.aliases.push(def);
        id
    }

    pub fn declare_value(&mut self, module: ModuleId, def: ValueDef, vis: Vis) -> ValueId {
        let id = ValueId(self.values.len() as u32);
        self.modules.get_mut(module).items.insert(
            (Ns::Value, def.name.clone()),
            module::ItemDef {
                def: Def::Value(id),
                vis,
            },
        );
        self.values.push(def);
        id
    }

    /// The definition behind an id, or nothing for a type with no declaration.
    pub fn def(&self, id: TypeId) -> Option<&TypeDef> {
        if id.is_foreign() {
            None
        } else {
            self.defs.get(id.index())
        }
    }

    pub fn def_mut(&mut self, id: TypeId) -> Option<&mut TypeDef> {
        if id.is_foreign() {
            None
        } else {
            self.defs.get_mut(id.index())
        }
    }

    pub fn alias(&self, id: AliasId) -> Option<&AliasDef> {
        self.aliases.get(id.0 as usize)
    }

    pub fn value(&self, id: ValueId) -> Option<&ValueDef> {
        self.values.get(id.0 as usize)
    }

    pub fn value_mut(&mut self, id: ValueId) -> Option<&mut ValueDef> {
        self.values.get_mut(id.0 as usize)
    }

    /// Run `f` with `alias` marked as being expanded. Returns nothing when the
    /// alias is already on the stack, which is how an alias cycle stops.
    pub fn expanding_alias<T>(&self, alias: AliasId, f: impl FnOnce() -> T) -> Option<T> {
        if self.expanding.borrow().contains(&alias) {
            return None;
        }
        self.expanding.borrow_mut().push(alias);
        let out = f();
        self.expanding.borrow_mut().pop();
        Some(out)
    }

    /// The leaf name a type is written with.
    pub fn name_of(&self, id: TypeId) -> String {
        if id.is_foreign() {
            self.foreign
                .borrow()
                .names
                .get(id.index())
                .cloned()
                .unwrap_or_default()
        } else {
            self.defs
                .get(id.index())
                .map(|d| d.name.clone())
                .unwrap_or_default()
        }
    }

    /// True when this type is one of the declared system types, which is how
    /// emission and the native-type translations tell `std::sync::Arc` from a
    /// crate type that happens to be called `Arc`.
    pub fn is_system(&self, id: TypeId) -> bool {
        self.def(id)
            .map(|d| self.modules.get(d.module).is_system)
            .unwrap_or(false)
    }

    /// A declared system type by the full path it is declared under.
    pub fn system_type(&self, path: &str) -> Option<TypeId> {
        self.system_by_path.get(path).copied()
    }

    pub(super) fn record_system_path(&mut self, path: &str, id: TypeId) {
        self.system_by_path.insert(path.to_string(), id);
    }

    pub(super) fn record_surface_name(&mut self, ns: Ns, name: &str, def: Def) {
        let found = self.surface_names.entry((ns, name.to_string())).or_default();
        if !found.contains(&def) {
            found.push(def);
        }
    }

    /// What a bare name written inside the surface stands for.
    ///
    /// A stub's own crate answers first. `serde::de::Deserializer` and
    /// `serde_wasm_bindgen::Deserializer` are two different declarations of one
    /// leaf name, and a bare `Deserializer` written inside `extern/serde.rs`
    /// means serde's, exactly as it does in Rust. Only where the writer's own
    /// crate has no declaration does the rest of the surface answer, and two
    /// answers there are a stub that has to say which.
    pub(super) fn surface_item(
        &self,
        ns: Ns,
        name: &str,
        from: ModuleId,
    ) -> Result<Option<Def>, lookup::LookupError> {
        let found = self
            .surface_names
            .get(&(ns, name.to_string()))
            .map(|found| found.as_slice())
            .unwrap_or(&[]);
        let own_crate = self.surface_crate_of(from);
        let same_crate: Vec<Def> = found
            .iter()
            .filter(|def| self.def_crate(**def) == own_crate)
            .copied()
            .collect();
        let candidates: &[Def] = if same_crate.is_empty() { found } else { &same_crate };
        match candidates {
            [] => Ok(None),
            [only] => Ok(Some(*only)),
            many => Err(lookup::LookupError {
                message: format!(
                    "`{}` is declared in {} places in the std surface; the stub that writes it \
                     has to say which",
                    name,
                    many.len()
                ),
            }),
        }
    }

    /// The surface crate a module belongs to — the first segment of its path in
    /// the system tree, which is the crate the stub file was read under.
    fn surface_crate_of(&self, module: ModuleId) -> Option<String> {
        self.modules.get(module).path.first().cloned()
    }

    /// The surface crate a declaration belongs to, where it is one the surface
    /// declared at all.
    fn def_crate(&self, def: Def) -> Option<String> {
        let module = match def {
            Def::Type(id) => self.def(id)?.module,
            Def::Alias(id) => self.alias(id)?.module,
            // A value — a `const`, a function — and a module record no module
            // of their own, so neither can be preferred by crate and every
            // candidate stands.
            Def::Value(_) | Def::Module(_) => return None,
        };
        self.surface_crate_of(module)
    }

    /// The id standing for a type nothing declares, created on first sight.
    pub fn foreign(&self, path: &[String]) -> Result<TypeId, IdSpaceExhausted> {
        let key = path.join("::");
        let mut foreign = self.foreign.borrow_mut();
        if let Some(&existing) = foreign.by_path.get(&key) {
            return Ok(existing);
        }
        let index = u32::try_from(foreign.names.len()).map_err(|_| IdSpaceExhausted)?;
        let raw = TypeId::FOREIGN_BASE
            .checked_add(index)
            .ok_or(IdSpaceExhausted)?;
        let id = TypeId(raw);
        foreign.names.push(path.last().cloned().unwrap_or_default());
        foreign.by_path.insert(key, id);
        Ok(id)
    }

    /// Record that this undeclared type was reported, so the count and the
    /// printed list agree.
    pub fn mark_reported(&self, id: TypeId) {
        self.foreign.borrow_mut().reported.insert(id);
    }

    /// How many distinct undeclared types this run reported. Marker traits are
    /// interned but carry no shape, so they are neither listed nor counted.
    pub fn undeclared_reported(&self) -> usize {
        self.foreign.borrow().reported.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_struct(name: &str) -> TypeDecl {
        TypeDecl {
            name: name.to_string(),
            kind: TypeKind::Struct,
            type_params: Vec::new(),
            vis: Vis::Public,
        }
    }

    #[test]
    fn declaring_the_same_leaf_name_in_two_modules_keeps_both() {
        let mut reg = TypeRegistry::new("signals");
        let broadcast = reg.modules_mut().module_for_file("broadcast.rs");
        let system = reg.system_root();
        let crate_ref = reg.declare_type(broadcast, a_struct("Ref")).unwrap();
        let system_ref = reg.declare_type(system, a_struct("Ref")).unwrap();

        assert_ne!(crate_ref, system_ref);
        assert!(!reg.is_system(crate_ref));
        assert!(reg.is_system(system_ref));
        assert_eq!(reg.modules().get(broadcast).path, vec!["broadcast"]);
        assert!(reg.modules().get(reg.system_root()).is_system);
    }

    #[test]
    fn foreign_types_are_stable_and_named_by_their_last_segment() {
        let reg = TypeRegistry::new("proto");
        let a = reg.foreign(&["ulid".into(), "Ulid".into()]).unwrap();
        let b = reg.foreign(&["ulid".into(), "Ulid".into()]).unwrap();
        let c = reg.foreign(&["anyhow".into(), "Error".into()]).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.is_foreign());
        assert_eq!(reg.name_of(a), "Ulid");
        assert!(reg.def(a).is_none());
        assert_eq!(reg.undeclared_reported(), 0, "nothing has been reported yet");
        reg.mark_reported(a);
        assert_eq!(reg.undeclared_reported(), 1);
    }

    #[test]
    fn an_alias_cycle_stops_instead_of_recursing() {
        let reg = TypeRegistry::new("c");
        let id = AliasId(0);
        let inner = reg.expanding_alias(id, || reg.expanding_alias(id, || 1u8));
        assert_eq!(inner, Some(None), "the second entry is refused");
        assert_eq!(
            reg.expanding_alias(id, || 2u8),
            Some(2),
            "and the stack unwinds"
        );
    }
}
