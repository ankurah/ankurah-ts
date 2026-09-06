//! The type registry: every type the transpiler knows, keyed by identity.
//!
//! A type is reached by its id. Names are looked up through the module that
//! wrote them (see `lookup.rs`), in the namespace the site asks for and subject
//! to the visibility the declaration was written with, so two modules can each
//! declare a `Ref` and a crate type can never displace a system type.

mod fields;
mod provided;
mod uses;
pub use crate::ty::IdSpaceExhausted;
mod assoc;
mod bounds;
mod build;
pub mod convert;
mod describe;
#[cfg(test)] mod engine_tests;
pub mod impls;
mod lookup;
pub mod method;
#[cfg(test)] mod assoc_tests;
#[cfg(test)] mod method_tests;
mod module;
mod resolve_type;
pub mod std_surface;
mod traits;

use std::cell::RefCell;
use std::collections::HashMap;

use crate::ty::{Ty, TypeId};
use crate::types::SelfKind;

pub use build::{
    build_registry, build_registry_with_siblings, mark_fresh_consts, narrow_reads_json,
    resolve_bounds,
    ExtractedFile,
};
pub use convert::{Conversion, NoConversion};
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
    ///
    /// A field whose type the engine could not resolve is NOT here — the pair
    /// has nowhere to put a type it does not have. `field_order` carries every
    /// field, which is what a constructor call needs.
    pub fields: Vec<(String, Ty)>,
    /// Every field this declaration has, in the order it declares them, whether
    /// or not its type resolved. The emitted constructor takes its parameters
    /// in exactly this order, so a struct literal is written from it.
    pub field_order: Vec<String>,
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
    /// The declared type of a constant or static, and a function's return type;
    /// `None` where the engine could not name it.
    pub ty: Option<Ty>,
    /// What a free function declares, so a call to one can hand each argument
    /// the type its parameter was written with. Without it, `wants(x.into())`
    /// and `wants(|v| ..)` have nothing to read: only associated functions were
    /// reached, and 89 closures and 48 `.into()`s in the corpus stood in a
    /// position that said nothing.
    pub sig: Option<MethodSig>,
    /// Is every use of this name a FRESH value?
    ///
    /// A Rust `const` is inlined at each use, so `let mut a = ORIGIN; a.x = 9;`
    /// mutates a value of its own and `let b = ORIGIN;` gets another. Bound to
    /// one module object, the two uses shared an identity, a mutation and a
    /// release — the second `.drop()` on that object aborts the run. A `static`
    /// is the opposite: ONE place for the life of the program, and shared on
    /// purpose. Only a non-`Copy` `const` is fresh, and the emitted name is a
    /// function each use calls.
    pub fresh_at_each_use: bool,
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
    /// Types whose TypeScript a person wrote: the `[provided_impls]` entries and
    /// everything declared in a `[hardcode]` file. See `mark_hand_written`.
    hand_written: std::collections::HashSet<TypeId>,
    /// Types whose serde derive writes them a `static fromJson`.
    reads_json: std::collections::HashSet<TypeId>,
    /// The other in-family crates loaded for their declarations, by the Rust
    /// identifier a path names them with. Each is a child of the crate root,
    /// which is where an extern crate sits in Rust too: `ankql::ast::Selection`
    /// written inside proto reaches ankql's real declaration and its real id,
    /// rather than a foreign name with no fields and no methods.
    sibling_crates: HashMap<String, ModuleId>,
    /// Types whose MEMBERS a person wrote, in this crate or in a sibling. See
    /// `members_are_hand_written`.
    members_hand_written: std::collections::HashSet<TypeId>,
    /// Hand-written types whose file declares `debug(): string`. Only the
    /// config entry can say: the engine never reads the TypeScript it did not
    /// write.
    declares_debug: std::collections::HashSet<TypeId>,
    /// Aliases part-way through expansion, so a cycle stops rather than recurses.
    expanding: RefCell<Vec<AliasId>>,
    /// What each declared system type becomes in TypeScript, by identity.
    /// Filled in once the surface is declared, because it is keyed on the ids
    /// the surface's own paths resolved to.
    shapes: crate::name_map::system_shapes::SystemShapes,
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
            hand_written: std::collections::HashSet::new(),
            reads_json: std::collections::HashSet::new(),
            sibling_crates: HashMap::new(),
            members_hand_written: std::collections::HashSet::new(),
            declares_debug: std::collections::HashSet::new(),
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

    /// Record that `ident` names an in-family crate rooted at `module`.
    pub fn add_sibling_crate(&mut self, ident: &str, module: ModuleId) {
        self.sibling_crates.insert(ident.replace('-', "_"), module);
    }

    /// Every in-family crate root loaded for this run.
    pub fn sibling_crate_roots(&self) -> Vec<ModuleId> {
        self.sibling_crates.values().copied().collect()
    }

    /// A name declared anywhere inside a sibling crate, searched by leaf.
    ///
    /// Rust would need a `use` for this and the port's import map is keyed by
    /// the leaf name alone, so the two agree only if the registry answers the
    /// same question the import map does. A name two sibling crates both
    /// declare answers with the first found, which is the same rule the import
    /// map applies.
    pub(super) fn sibling_module_scan(
        &self,
        root: ModuleId,
        ns: Ns,
        name: &str,
    ) -> Option<Def> {
        if ns != Ns::Type {
            return None;
        }
        let mut stack = vec![root];
        while let Some(module) = stack.pop() {
            if let Some(id) = self.module_type(module, name) {
                return Some(Def::Type(id));
            }
            stack.extend(self.modules.get(module).children.values().copied());
        }
        None
    }

    /// The root a sibling crate's name reaches, if it names one.
    pub fn sibling_crate(&self, ident: &str) -> Option<ModuleId> {
        self.sibling_crates.get(&ident.replace('-', "_")).copied()
    }

    /// The crate root `module` belongs to: a sibling's own root when the module
    /// is inside one, and this crate's otherwise. `crate::ast::Predicate`
    /// written inside ankql means ankql's, whichever crate is being emitted.
    pub fn crate_root_of(&self, module: ModuleId) -> ModuleId {
        for root in self.sibling_crates.values() {
            if self.modules.is_within(module, *root) {
                return *root;
            }
        }
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
            field_order: Vec::new(),
            module,
            name: decl.name,
            kind: decl.kind,
            fields: Vec::new(),
            param_defaults: vec![None; decl.type_params.len()],
            type_params: decl.type_params,
        });
        Ok(id)
    }

    /// Every trait the registry knows, so a run can ask which leaf names two
    /// of them share.
    pub fn trait_ids(&self) -> Vec<TypeId> {
        self.traits.keys().copied().collect()
    }

    /// Is there an alias of this leaf name anywhere in the crate?
    ///
    /// A struct field carries no module of its own, and the port emits an alias
    /// under its own name, so a field written as `Listener` has to be written
    /// back as one. Two modules declaring one alias name is a shape the port
    /// has never had, and a false yes costs only the syntactic spelling, which
    /// is what the source wrote.
    pub fn has_alias_named(&self, name: &str) -> bool {
        self.aliases.iter().any(|alias| alias.name == name)
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
    /// Does the port write a TypeScript interface for this trait?
    ///
    /// A trait this crate declares is emitted as an interface, so a class that
    /// implements it can say so in its `implements` clause. A trait the
    /// declared surface holds — `Add`, `Iterator`, `Clone`, `Future` — has no
    /// TypeScript at all, and naming one there named something that does not
    /// exist: `class Weight extends Struct implements Add`.
    pub fn emits_interface(&self, trait_name: &str) -> bool {
        self.defs.iter().any(|def| {
            def.name == trait_name
                && matches!(def.kind, TypeKind::Trait)
                && !self.modules().get(def.module).is_system
        })
    }

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

    /// What a free function declares, where this value is one.
    pub fn function_sig(&self, id: ValueId) -> Option<&MethodSig> {
        self.value(id)?.sig.as_ref()
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

    /// A declared type of this crate or a sibling, found by its leaf name.
    ///
    /// The import map is keyed by the leaf too, so a name two crates declare
    /// answers with the first found — which is the same rule the import writes.
    /// Emission holds a TypeScript spelling and needs the identity behind it;
    /// the leaf is all a spelling carries.
    pub fn type_by_leaf(&self, leaf: &str) -> Option<TypeId> {
        self.defs
            .iter()
            .position(|def| def.name == leaf)
            .map(|i| TypeId(i as u32))
    }

    /// How many types are declared, for a pass that has to walk them all.
    pub fn declared_count(&self) -> usize {
        self.defs.len()
    }

    /// This type's JSON half was refused, so nothing may call its `fromJson`.
    ///
    /// A `#[derive(Deserialize)]` on a type the port does NOT emit a class for
    /// says what serde would have done, not what the port wrote: `Attested`
    /// derives it and `auth.provided.ts` declares no `fromJson`, so three
    /// emitted call sites named a static nothing declares.
    pub fn clear_reads_json(&mut self, id: TypeId) {
        self.reads_json.remove(&id);
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

    /// "Hand-written" answered two questions at once, and they have different
    /// answers for a SIBLING crate's provided type: proto's `EntityId` has no
    /// emitted `debug()` for core to call — its TypeScript is in
    /// `id.provided.ts` — and core's `impl OrderedCollation for EntityId` is
    /// core's own code and still emits.
    #[test]
    fn a_siblings_provided_type_has_no_members_and_still_takes_impls() {
        let mut reg = TypeRegistry::new("core");
        let here = reg.modules_mut().module_for_file("entity.rs");
        let ours = reg.declare_type(here, a_struct("Attested")).unwrap();
        let theirs = reg.declare_type(here, a_struct("EntityId")).unwrap();

        reg.mark_hand_written(ours);
        reg.mark_members_hand_written(theirs);

        // This crate's own provided type answers both questions the same way.
        assert!(reg.is_hand_written(ours));
        assert!(reg.members_are_hand_written(ours));

        // A sibling's answers only the members question, so an impl this crate
        // writes for it is still emitted.
        assert!(!reg.is_hand_written(theirs));
        assert!(reg.members_are_hand_written(theirs));

        // And a type nobody wrote by hand answers neither.
        let emitted = reg.declare_type(here, a_struct("Entity")).unwrap();
        assert!(!reg.is_hand_written(emitted));
        assert!(!reg.members_are_hand_written(emitted));
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
