//! Building the registry from the extracted crate.
//!
//! Two passes, because a field's type can name a type declared in another file:
//! first every module, `use` binding and declaration; then every field,
//! variant, constant, trait and `impl` block, resolved against the declarations.

use std::collections::HashMap;

use super::impls::{Bound, ImplDef, ImplId};
use super::module::{ModuleId, UseBinding, Vis};
use super::traits::{TraitDef, TraitMethod};
use super::std_surface::Surface;
use super::{
    resolve_type, AliasDef, MethodSig, TypeDecl, TypeEnv, TypeKind, TypeRegistry, ValueDef,
    VariantDef, CLONE_PATH,
};
use crate::diag::DiagSink;
use crate::ty::{Ty, TypeId};
use crate::types::{FieldInfo, FnInfo, ImplInfo, RustFile, TraitInfo, VisInfo};

/// A parsed file, the path it came from, and whether anything is emitted for it.
#[derive(Debug)]
pub struct ExtractedFile {
    pub path: String,
    pub file: RustFile,
    /// Read for its declarations only: nothing is emitted for it, but the types
    /// it declares are part of what everything else resolves through. Two kinds
    /// of file are read this way — a `[[provided]]` module, whose TypeScript
    /// somebody wrote, and an in-family crate loaded because this one depends
    /// on it.
    pub declarations_only: bool,
    /// Of those two, the first: a module whose members are whatever the person
    /// who wrote the file wrote, so a hook must not call a method it did not
    /// emit. A sibling crate's file is NOT this — its TypeScript is emitted by
    /// its own run — and marking one as such silently dropped every impl
    /// written for one of its types (`impl From<EntityId> for ankql::ast::Expr`
    /// vanished from proto with nothing said).
    pub hand_written: bool,
}

/// What pass two learned, applied to the registry once pass two is done
/// borrowing it.
pub(super) enum Update {
    Fields {
        id: TypeId,
        fields: Vec<(String, Ty)>,
    },
    Variants {
        id: TypeId,
        variants: Vec<VariantDef>,
    },
    ParamDefaults {
        id: TypeId,
        defaults: Vec<Option<Ty>>,
    },
    Impl(ImplDef),
    Trait(TraitDef),
    ConstType {
        id: super::ValueId,
        ty: Ty,
    },
    /// A function's return type, so a call to it can be typed.
    ValueType {
        id: super::ValueId,
        ty: Ty,
    },
    /// A free function's whole signature, so a call to it can hand each
    /// argument the type its parameter was written with.
    ValueSig {
        id: super::ValueId,
        sig: MethodSig,
    },
}

/// Translate the visibility a declaration was written with into the module the
/// resolver judges it against.
fn vis_of(vis: VisInfo, module: ModuleId, reg: &TypeRegistry, sink: &DiagSink) -> Vis {
    match vis {
        VisInfo::Public => Vis::Public,
        VisInfo::Crate => Vis::Crate,
        VisInfo::Super => {
            Vis::Restricted(reg.modules().get(module).parent.unwrap_or(reg.crate_root()))
        }
        VisInfo::Private => Vis::Private,
        // Widening is the safe direction for a resolver that only looks inside
        // one crate, but it is still not what was written.
        VisInfo::InPath { line, col } => {
            sink.push(crate::diag::Diag {
                file: sink.file(),
                line,
                col,
                message: "`pub(in path)` is not modelled; read as `pub(crate)`".to_string(),
            });
            Vis::Crate
        }
    }
}

pub fn build_registry(
    files: &mut [ExtractedFile],
    surface: &mut Surface,
    crate_names: &[String],
    sink: &DiagSink,
) -> TypeRegistry {
    build_registry_with_siblings(files, surface, crate_names, &[], sink)
}

/// The same, told which of the files belong to other in-family crates.
pub fn build_registry_with_siblings(
    files: &mut [ExtractedFile],
    surface: &mut Surface,
    crate_names: &[String],
    siblings: &[String],
    sink: &DiagSink,
) -> TypeRegistry {
    let primary = crate_names.first().cloned().unwrap_or_default();
    let mut reg = TypeRegistry::new(&primary);
    for name in crate_names.iter().skip(1) {
        reg.add_crate_name(name);
    }

    super::std_surface::declare(&mut reg, surface, sink);
    reg.resolve_shapes();
    // A path the emission policy names that the surface no longer declares
    // would silently emit a plain class where the port has a wrapper.
    for path in &reg.shapes().unresolved {
        sink.push(crate::diag::Diag {
            file: format!("{}/", super::std_surface::DIR_NAME),
            line: 0,
            col: 0,
            message: format!(
                "the TypeScript shape table names `{}`, which the declared surface does not \
                 declare",
                path
            ),
        });
    }

    // An in-family crate read for its declarations arrives under a directory
    // named for the crate — `ankql/ast.rs` — so its module is `crate::ankql::
    // ast`, and the crate's own name is recorded as a root a path can start at.
    for entry in files.iter().filter(|e| e.declarations_only) {
        if let Some((head, _)) = entry.path.split_once('/') {
            if siblings.iter().any(|s| s == head) {
                let root = reg.modules_mut().module_for_file(&format!("{}/lib.rs", head));
                reg.add_sibling_crate(head, root);
            }
        }
    }

    for entry in files.iter() {
        let module = reg.modules_mut().module_for_file(&entry.path);
        sink.set_file(&entry.path);
        declare_file(&mut reg, module, &entry.file, sink);
    }

    let mut defaults = Vec::new();
    for entry in files.iter() {
        sink.set_file(&entry.path);
        if let Some(module) = reg.modules().lookup_file(&entry.path) {
            resolve_param_defaults(&reg, module, &entry.file, sink, &mut defaults);
        }
    }
    apply(&mut reg, defaults);

    let mut updates = Vec::new();
    for entry in files.iter_mut() {
        sink.set_file(&entry.path);
        let Some(module) = reg.modules().lookup_file(&entry.path) else {
            continue;
        };
        resolve_file(&reg, module, &mut entry.file, sink, &mut updates);
    }
    apply(&mut reg, updates);

    // Both the surface's and the crate's declarations are in by now, which is
    // what the blanket index needs to know which methods each blanket offers.
    reg.index_blankets();

    crate::emit::set_contested_conversions(contested_conversions(&reg));

    reg
}

/// Which conversion statics two impls of one type would both take.
///
/// `RetrievalError` has `From<bincode::Error>`, `From<crate::selection::filter::
/// Error>` and `From<anyhow::Error>`; all three name `fromError` from the leaf
/// alone, and emission wrote one and dropped two. The answer is a fact about a
/// type's SIBLING impls, which neither the class being written nor the call site
/// being named can see from the impl in its hand — so it is computed once here,
/// over the whole impl table, and both halves read it.
///
/// Keyed by the self type's own leaf name, which is what both halves have.
fn contested_conversions(
    reg: &TypeRegistry,
) -> std::collections::HashSet<(String, String)> {
    let mut seen: std::collections::HashMap<(String, String), usize> = Default::default();
    for i in 0..reg.impls().len() {
        let def = reg.impl_def(crate::registry::ImplId(i as u32));
        let Some(implemented) = def.trait_ref.as_ref() else {
            continue;
        };
        let trait_name = reg
            .name_of(implemented.id)
            .rsplit("::")
            .next()
            .unwrap_or_default()
            .to_string();
        if !matches!(trait_name.as_str(), "From" | "TryFrom") {
            continue;
        }
        let Some(source) = def.trait_args_written.first() else {
            continue;
        };
        let Some(self_id) = def.self_ty.peel_refs().id() else {
            continue;
        };
        let self_name = reg.name_of(self_id);
        let self_leaf = self_name.rsplit("::").next().unwrap_or(&self_name).to_string();
        let leaf = source.rsplit("::").next().unwrap_or(source);
        let base = if trait_name == "From" { "from" } else { "tryFrom" };
        *seen.entry((self_leaf, format!("{}{}", base, leaf))).or_default() += 1;
    }
    seen.into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(key, _)| key)
        .collect()
}

/// Write what pass two learned into the registry.
pub(super) fn apply(reg: &mut TypeRegistry, updates: Vec<Update>) {
    for update in updates {
        match update {
            Update::Fields { id, fields } => {
                if let Some(def) = reg.def_mut(id) {
                    def.fields = fields;
                }
            }
            Update::Variants { id, variants } => {
                if let Some(def) = reg.def_mut(id) {
                    def.kind = TypeKind::Enum { variants };
                }
            }
            Update::ParamDefaults { id, defaults } => {
                if let Some(def) = reg.def_mut(id) {
                    def.param_defaults = defaults;
                }
            }
            Update::Impl(def) => {
                reg.add_impl(def);
            }
            Update::Trait(def) => reg.insert_trait(def),
            Update::ConstType { id, ty } | Update::ValueType { id, ty } => {
                if let Some(value) = reg.value_mut(id) {
                    value.ty = Some(ty);
                }
            }
            Update::ValueSig { id, sig } => {
                if let Some(value) = reg.value_mut(id) {
                    value.sig = Some(sig);
                }
            }
        }
    }
}

/// Pass one: the module's imports and everything it declares, in both namespaces.
pub(super) fn declare_file(reg: &mut TypeRegistry, module: ModuleId, file: &RustFile, sink: &DiagSink) {
    let bindings: Vec<UseBinding> = file
        .uses
        .iter()
        .flat_map(|u| {
            let vis = vis_of(u.vis, module, reg, sink);
            u.bindings.iter().map(move |b| UseBinding {
                local: b.local.clone(),
                path: b.path.clone(),
                vis,
            })
        })
        .collect();
    reg.modules_mut().get_mut(module).uses.extend(bindings);

    for s in &file.structs {
        let vis = vis_of(s.vis, module, reg, sink);
        declare(
            reg,
            module,
            s.name.clone(),
            TypeKind::Struct,
            s.type_params.clone(),
            vis,
            sink,
        );
    }
    for e in &file.enums {
        let vis = vis_of(e.vis, module, reg, sink);
        let kind = TypeKind::Enum {
            variants: Vec::new(),
        };
        declare(
            reg,
            module,
            e.name.clone(),
            kind,
            e.type_params.clone(),
            vis,
            sink,
        );
    }
    for t in &file.traits {
        let vis = vis_of(t.vis, module, reg, sink);
        declare(
            reg,
            module,
            t.name.clone(),
            TypeKind::Trait,
            t.type_params.clone(),
            vis,
            sink,
        );
    }
    for a in &file.type_aliases {
        let vis = vis_of(a.vis, module, reg, sink);
        let def = AliasDef {
            module,
            name: a.name.clone(),
            type_params: a.type_params.clone(),
            param_defaults: a.param_defaults.clone(),
            rust_ty: a.rust_ty.clone(),
        };
        reg.declare_alias(module, def, vis);
    }
    for c in &file.consts {
        let vis = vis_of(c.vis, module, reg, sink);
        reg.declare_value(
            module,
            ValueDef {
                name: c.name.clone(),
                ty: None,
                sig: None,
            },
            vis,
        );
    }
    for f in &file.functions {
        let vis = vis_of(f.vis, module, reg, sink);
        reg.declare_value(
            module,
            ValueDef {
                name: f.name.clone(),
                ty: None,
                sig: None,
            },
            vis,
        );
    }

    for (name, sub_file) in &file.inline_modules {
        let child = reg.modules_mut().child(module, name);
        let vis = vis_of(sub_file.vis, child, reg, sink);
        reg.modules_mut().get_mut(child).vis = vis;
        declare_file(reg, child, sub_file, sink);
    }
}

fn declare(
    reg: &mut TypeRegistry,
    module: ModuleId,
    name: String,
    kind: TypeKind,
    type_params: Vec<String>,
    vis: Vis,
    sink: &DiagSink,
) {
    let decl = TypeDecl {
        name: name.clone(),
        kind,
        type_params,
        vis,
    };
    if let Err(err) = reg.declare_type(module, decl) {
        sink.push(crate::diag::Diag {
            file: sink.file(),
            line: 0,
            col: 0,
            message: format!("cannot declare `{}`: {}", name, err),
        });
    }
}

/// Pass two: every written type, resolved in the module that wrote it.
pub(super) fn resolve_file(
    reg: &TypeRegistry,
    module: ModuleId,
    file: &mut RustFile,
    sink: &DiagSink,
    updates: &mut Vec<Update>,
) {
    for s in &mut file.structs {
        let id = reg.module_type(module, &s.name);
        let fields = resolve_fields(reg, module, &s.type_params, &mut s.fields, sink);
        if let Some(id) = id {
            derived_impls(reg, module, id, &s.type_params, &s.derives, updates);
            updates.push(Update::Fields { id, fields });
        }
    }

    for e in &mut file.enums {
        let id = reg.module_type(module, &e.name);
        let mut variants = Vec::new();
        for v in &mut e.variants {
            let fields = resolve_fields(reg, module, &e.type_params, &mut v.fields, sink);
            variants.push(VariantDef {
                name: v.name.clone(),
                fields,
            });
        }
        if let Some(id) = id {
            derived_impls(reg, module, id, &e.type_params, &e.derives, updates);
            thiserror_from_impls(reg, module, id, e, updates);
            updates.push(Update::Variants { id, variants });
        }
    }

    for c in &file.consts {
        let Some(id) = reg.module_value(module, &c.name) else {
            continue;
        };
        let Some(rust_ty) = &c.rust_ty else { continue };
        let env = TypeEnv::new(reg, module, sink);
        match resolve_type(rust_ty, &env) {
            Ok(ty) => updates.push(Update::ConstType { id, ty }),
            Err(diag) => sink.push(diag),
        }
    }

    // A free function's signature: its return type, so that `foo()` in
    // expression position has a type, and its parameters, so that an argument
    // written as `x.into()` or as a closure can read what it has to be.
    //
    // The parameters are resolved into a sink nobody reads. A signature the
    // engine cannot name in full is simply not stored — the call falls back to
    // saying nothing about its arguments, which is where it stood before — and
    // reporting it here would count one gap once per declaration and again at
    // every use.
    let quiet = DiagSink::new();
    for f in &file.functions {
        let Some(id) = reg.module_value(module, &f.name) else {
            continue;
        };
        if let Some(sig) = method_sig(reg, module, &f.type_params, None, f, &quiet) {
            updates.push(Update::ValueSig { id, sig });
        }
        let Some(rust_ty) = &f.rust_return else {
            updates.push(Update::ValueType { id, ty: Ty::Unit });
            continue;
        };
        let env = TypeEnv::new(reg, module, sink).with_params(&f.type_params);
        match resolve_type(rust_ty, &env) {
            Ok(ty) => updates.push(Update::ValueType { id, ty }),
            Err(diag) => sink.push(diag),
        }
    }

    for t in &file.traits {
        if let Some(def) = resolve_trait(reg, module, t, sink) {
            updates.push(Update::Trait(def));
        }
    }

    for imp in &file.impls {
        if let Some(def) = resolve_impl(reg, module, imp, sink) {
            updates.push(Update::Impl(def));
        }
    }

    for (name, sub_file) in &mut file.inline_modules {
        let Some(child) = reg.modules().get(module).children.get(name).copied() else {
            continue;
        };
        resolve_file(reg, child, sub_file, sink, updates);
    }
}

/// The impls a `#[derive(..)]` stands for.
///
/// A derive is a written fact about the type, and the engine needs it as one:
/// `HashMap::entry` is declared `where K: Eq + Hash`, so a key whose `Eq` and
/// `Hash` nobody registered makes every `entry` call unresolvable, and
/// `guard.clone()` used to clone the guard rather than what it holds. The impl
/// is registered with no methods of its own; the trait's own declarations
/// supply the signatures, which is what a derive does.
///
/// This is the first of the derive hooks the spec calls for (4.10, step 7),
/// narrowed to the derives whose absence stops resolution. `Serialize` and
/// `Deserialize` produce code as well as impls and are that step's business.
const DERIVED: [(&str, &str); 9] = [
    ("Clone", CLONE_PATH),
    ("Copy", "std::marker::Copy"),
    ("PartialEq", "std::cmp::PartialEq"),
    ("Eq", "std::cmp::Eq"),
    ("PartialOrd", "std::cmp::PartialOrd"),
    ("Ord", "std::cmp::Ord"),
    ("Hash", "std::hash::Hash"),
    ("Default", "std::default::Default"),
    ("Debug", "std::fmt::Debug"),
];

/// What `#[derive(thiserror::Error)]` writes that the engine has to know about.
///
/// The derive generates a `Display` from the `#[error("..")]` attributes and an
/// `std::error::Error` from the type being one. Neither has a method the engine
/// needs to read, and both are what other bounds are decided against: `impl<E:
/// std::error::Error> From<E> for anyhow::Error` is the impl every `?` into an
/// `anyhow::Result` selects, and without these two registered it selects
/// nothing and 33 sites in core report a conversion that does exist.
///
/// The `From` impls the derive writes for `#[from]` fields are registered
/// alongside, in `thiserror_from_impls`.
const THISERROR_DERIVED: [&str; 2] = ["std::error::Error", "std::fmt::Display"];

/// Is this the thiserror derive? It is written `Error` behind a `use
/// thiserror::Error`, and `thiserror::Error` where the import is not there.
///
/// `std::error::Error` cannot be derived, so an `Error` in a derive list is
/// thiserror's in every case rustc accepts.
fn is_thiserror(derives: &[String]) -> bool {
    derives
        .iter()
        .any(|d| d == "Error" || d.replace(' ', "") == "thiserror::Error")
}

fn derived_impls(
    reg: &TypeRegistry,
    module: ModuleId,
    id: TypeId,
    type_params: &[String],
    derives: &[String],
    updates: &mut Vec<Update>,
) {
    let self_ty = Ty::Named {
        id,
        args: type_params.iter().map(|p| Ty::Param(p.clone())).collect(),
    };
    for (derive, path) in DERIVED {
        if !derives.iter().any(|d| d == derive) {
            continue;
        }
        let Some(trait_id) = reg.system_type(path) else {
            continue;
        };
        // rustc's derive puts the derived trait on every type parameter:
        // `#[derive(Clone)] struct W<T>(T)` expands to
        // `impl<T: Clone> Clone for W<T>`, so a `W<NoClone>` is not `Clone`.
        // Registering the impl without those bounds proved every instantiation
        // clonable, and a bound that rests on one then held for the wrong
        // reason.
        let bounds: Vec<crate::registry::impls::Bound> = type_params
            .iter()
            .map(|param| crate::registry::impls::Bound {
                subject: Ty::Param(param.clone()),
                trait_ref: crate::ty::TraitRef {
                    id: trait_id,
                    args: Vec::new(),
                    bindings: Vec::new(),
                },
            })
            .collect();
        updates.push(Update::Impl(ImplDef {
            id: ImplId(0),
            module,
            generics: type_params.to_vec(),
            bounds,
            self_ty: self_ty.clone(),
            trait_ref: Some(crate::ty::TraitRef {
                id: trait_id,
                args: Vec::new(),
                bindings: Vec::new(),
            }),
            trait_args_written: Vec::new(),
            assoc_types: HashMap::new(),
            methods: HashMap::new(),
        }));
    }
    if is_thiserror(derives) {
        // The derive proves these of the type itself, with no bound on the
        // parameters: `#[error("{0}")]` writes a `Display` whatever the fields
        // hold. That is thiserror's own rule and not rustc's derive rule, so
        // the bounds the loop above adds do not belong here.
        for path in THISERROR_DERIVED {
            let Some(trait_id) = reg.system_type(path) else {
                continue;
            };
            updates.push(Update::Impl(ImplDef {
                id: ImplId(0),
                module,
                generics: type_params.to_vec(),
                bounds: Vec::new(),
                self_ty: self_ty.clone(),
                trait_ref: Some(crate::ty::TraitRef {
                    id: trait_id,
                    args: Vec::new(),
                    bindings: Vec::new(),
                }),
                trait_args_written: Vec::new(),
                assoc_types: HashMap::new(),
                methods: HashMap::new(),
            }));
        }
    }
}

/// The `impl From<Inner> for Outer` that `#[derive(thiserror::Error)]` writes
/// for each variant field marked `#[from]`.
///
/// One field in the corpus carries the attribute — `SendError::Other(#[from]
/// anyhow::Error)` — and a `?` handing an `anyhow::Error` into a function
/// returning `Result<_, SendError>` calls it. Nothing else generates the impl,
/// so without this hook that site has no conversion to find.
fn thiserror_from_impls(
    reg: &TypeRegistry,
    module: ModuleId,
    id: TypeId,
    e: &crate::types::EnumInfo,
    updates: &mut Vec<Update>,
) {
    if !is_thiserror(&e.derives) {
        return;
    }
    let Some(trait_id) = reg.system_type(super::convert::FROM_PATH) else {
        return;
    };
    let self_ty = Ty::Named {
        id,
        args: e.type_params.iter().map(|p| Ty::Param(p.clone())).collect(),
    };
    for variant in &e.variants {
        for field in &variant.fields {
            if !field.is_from {
                continue;
            }
            let Some(source) = field.ty.clone() else {
                continue;
            };
            updates.push(Update::Impl(ImplDef {
                id: ImplId(0),
                module,
                generics: e.type_params.clone(),
                bounds: Vec::new(),
                self_ty: self_ty.clone(),
                trait_ref: Some(crate::ty::TraitRef {
                    id: trait_id,
                    args: vec![source],
                    bindings: Vec::new(),
                }),
                // The derive writes `impl From<the field's type>`, so the
                // field's type as written is what names the emitted static.
                trait_args_written: vec![crate::name_map::map_type(&field.rust_ty)],
                assoc_types: HashMap::new(),
                methods: HashMap::new(),
            }));
        }
    }
}

/// Pass one and a half: `HashMap<K, V, S = RandomState>` — what a parameter the
/// use site leaves unwritten falls back to.
///
/// It runs between the two main passes because pass two *reads* these: a written
/// `HashMap<String, u8>` is completed from them, and completing it after every
/// other written type had already been resolved would be too late for all of
/// them. Resolving a default needs only the declarations, which pass one has.
pub(super) fn resolve_param_defaults(
    reg: &TypeRegistry,
    module: ModuleId,
    file: &RustFile,
    sink: &DiagSink,
    updates: &mut Vec<Update>,
) {
    for s in &file.structs {
        if let Some(id) = reg.module_type(module, &s.name) {
            push_param_defaults(reg, module, id, &s.type_params, &s.param_defaults, sink, updates);
        }
    }
    for e in &file.enums {
        if let Some(id) = reg.module_type(module, &e.name) {
            push_param_defaults(reg, module, id, &e.type_params, &e.param_defaults, sink, updates);
        }
    }
    for (name, sub_file) in &file.inline_modules {
        if let Some(child) = reg.modules().get(module).children.get(name).copied() {
            resolve_param_defaults(reg, child, sub_file, sink, updates);
        }
    }
}

fn push_param_defaults(
    reg: &TypeRegistry,
    module: ModuleId,
    id: TypeId,
    params: &[String],
    written: &[Option<syn::Type>],
    sink: &DiagSink,
    updates: &mut Vec<Update>,
) {
    if written.iter().all(|d| d.is_none()) {
        return;
    }
    let env = TypeEnv::new(reg, module, sink).with_params(params);
    let mut defaults = Vec::new();
    for default in written {
        match default {
            None => defaults.push(None),
            Some(rust_ty) => match resolve_type(rust_ty, &env) {
                Ok(ty) => defaults.push(Some(ty)),
                Err(diag) => {
                    sink.push(diag);
                    defaults.push(None);
                }
            },
        }
    }
    updates.push(Update::ParamDefaults { id, defaults });
}

fn resolve_fields(
    reg: &TypeRegistry,
    module: ModuleId,
    params: &[String],
    fields: &mut [FieldInfo],
    sink: &DiagSink,
) -> Vec<(String, Ty)> {
    let env = TypeEnv::new(reg, module, sink).with_params(params);
    let mut out = Vec::new();
    for field in fields.iter_mut() {
        match resolve_type(&field.rust_ty, &env) {
            Ok(ty) => {
                let name = field.name.clone().unwrap_or_else(|| "_0".to_string());
                out.push((name, ty.clone()));
                field.ty = Some(ty);
            }
            Err(diag) => {
                sink.push(diag);
                field.ty = None;
            }
        }
    }
    out
}

/// A trait declaration: its supertraits, the associated types an impl has to
/// supply, and the signature of every method it declares.
fn resolve_trait(
    reg: &TypeRegistry,
    module: ModuleId,
    t: &TraitInfo,
    sink: &DiagSink,
) -> Option<TraitDef> {
    let id = reg.module_type(module, &t.name)?;
    // A trait's own declarations speak about `Self`, which stands for whatever
    // implements it. Method resolution substitutes the real type in.
    let self_ty = Ty::Param("Self".to_string());
    let env = TypeEnv::new(reg, module, sink)
        .with_params(&t.type_params)
        .with_self(Some(&self_ty));

    let mut supertraits = Vec::new();
    for bound in &t.supertraits {
        match super::resolve_type::trait_ref(bound, &env) {
            Ok(tr) => supertraits.push(tr),
            Err(diag) => sink.push(diag),
        }
    }

    let mut methods = HashMap::new();
    for method in &t.methods {
        let mut params = t.type_params.clone();
        params.extend(method.type_params.iter().cloned());
        if let Some(sig) = method_sig(reg, module, &params, Some(&self_ty), method, sink) {
            methods.insert(
                method.name.clone(),
                TraitMethod {
                    sig,
                },
            );
        }
    }

    Some(TraitDef {
        id,
        generics: t.type_params.clone(),
        supertraits,
        assoc_types: t.assoc_types.clone(),
        methods,
        is_auto: t.is_auto,
    })
}

/// An `impl` block: what it is for, the trait it implements, what its `where`
/// clauses require, and every signature it supplies.
fn resolve_impl(
    reg: &TypeRegistry,
    module: ModuleId,
    imp: &ImplInfo,
    sink: &DiagSink,
) -> Option<ImplDef> {
    let syn_self = imp.self_ty.as_ref()?;
    let env = TypeEnv::new(reg, module, sink).with_params(&imp.type_params);
    let self_ty = match resolve_type(syn_self, &env) {
        Ok(ty) => ty,
        Err(diag) => {
            sink.push(diag);
            return None;
        }
    };
    let env = TypeEnv::new(reg, module, sink)
        .with_params(&imp.type_params)
        .with_self(Some(&self_ty));

    let trait_ref = match &imp.trait_path {
        None => None,
        Some(path) => match super::resolve_type::trait_ref_of_path(path, &env) {
            Ok(tr) => Some(tr),
            Err(diag) => {
                sink.push(diag);
                return None;
            }
        },
    };

    let bounds = resolve_bounds(&imp.generics, &env, sink);

    let mut assoc_types = HashMap::new();
    for (name, rust_ty) in &imp.assoc_types {
        match resolve_type(rust_ty, &env) {
            Ok(ty) => {
                assoc_types.insert(name.clone(), ty);
            }
            Err(diag) => sink.push(diag),
        }
    }

    let mut methods = HashMap::new();
    for method in &imp.methods {
        let mut params = imp.type_params.clone();
        params.extend(method.type_params.iter().cloned());
        if let Some(sig) = method_sig(reg, module, &params, Some(&self_ty), method, sink) {
            methods.insert(method.name.clone(), sig);
        }
    }

    Some(ImplDef {
        // Filled in when the table takes it.
        id: ImplId(0),
        module,
        generics: imp.type_params.clone(),
        bounds,
        self_ty,
        trait_ref,
        trait_args_written: imp.trait_type_arg_paths(),
        assoc_types,
        methods,
    })
}

/// The `T: Trait` requirements an impl or trait writes, inline and in its
/// `where` clause alike.
pub fn resolve_bounds(generics: &syn::Generics, env: &TypeEnv, sink: &DiagSink) -> Vec<Bound> {
    let mut out = Vec::new();
    for param in &generics.params {
        let syn::GenericParam::Type(t) = param else {
            continue;
        };
        let subject = Ty::Param(t.ident.to_string());
        for bound in &t.bounds {
            push_bound(&subject, bound, env, sink, &mut out);
        }
    }
    let Some(where_clause) = &generics.where_clause else {
        return out;
    };
    for pred in &where_clause.predicates {
        let syn::WherePredicate::Type(pt) = pred else {
            continue;
        };
        let subject = match resolve_type(&pt.bounded_ty, env) {
            Ok(ty) => ty,
            Err(diag) => {
                sink.push(diag);
                continue;
            }
        };
        for bound in &pt.bounds {
            push_bound(&subject, bound, env, sink, &mut out);
        }
    }
    out
}

fn push_bound(
    subject: &Ty,
    bound: &syn::TypeParamBound,
    env: &TypeEnv,
    sink: &DiagSink,
    out: &mut Vec<Bound>,
) {
    let syn::TypeParamBound::Trait(t) = bound else {
        return;
    };
    // `T: ?Sized` lifts the implicit `Sized` requirement; it does not add one.
    // Reading it as a requirement made `impl<T: ?Sized> Deref for Arc<T>` — the
    // shape half the std surface is written in — demand a proof of the opposite
    // of what it says.
    if matches!(t.modifier, syn::TraitBoundModifier::Maybe(_)) {
        return;
    }
    match super::resolve_type::trait_ref(t, env) {
        Ok(trait_ref) => out.push(Bound {
            subject: subject.clone(),
            trait_ref,
        }),
        Err(diag) => {
            sink.push(diag);
            // A bound the engine could not read is still a bound. Dropping it
            // turned `impl<T: Display> ToString for T` into an impl with no
            // requirement at all, which then answered `to_string` on every type
            // there is. Standing it up against the written name — which nothing
            // declares, as far as this run could tell — makes it an obligation
            // nobody could decide, reported at each call.
            let segments: Vec<String> = t.path.segments.iter().map(|s| s.ident.to_string()).collect();
            let canonical = env.reg.canonical_path(env.module, &segments);
            if let Ok(id) = env.reg.foreign(&canonical) {
                out.push(Bound {
                    subject: subject.clone(),
                    trait_ref: crate::ty::TraitRef {
                        id,
                        args: Vec::new(),
                        bindings: Vec::new(),
                    },
                });
            }
        }
    }
}

/// A method's signature, or nothing when the engine could not name a type in
/// it. A method whose return type the engine cannot read stays out of the
/// table: "no answer" is the truth, and calling it `()` would not be.
fn method_sig(
    reg: &TypeRegistry,
    module: ModuleId,
    params: &[String],
    self_ty: Option<&Ty>,
    method: &FnInfo,
    sink: &DiagSink,
) -> Option<MethodSig> {
    let env = TypeEnv::new(reg, module, sink)
        .with_params(params)
        .with_self(self_ty);

    // `self: Arc<Self>` and the other written receivers put the method on a
    // type the engine does not yet walk to. Reading one as by-value would say it
    // sits on `Self`; leaving it out says the truth, and the written type is
    // kept on the extracted function for the step that supports it.
    if method.self_kind == Some(crate::types::SelfKind::Arbitrary) {
        if let Some(written) = &method.self_receiver {
            sink.push(crate::diag::Diag::at(
                &sink.file(),
                syn::spanned::Spanned::span(written),
                format!(
                    "`self: {}` is a receiver the engine does not model; `{}` is left out of the method table",
                    quote::ToTokens::to_token_stream(written),
                    method.name
                ),
            ));
        }
        return None;
    }

    let receiver = self_ty.map(|ty| match method.self_kind {
        Some(crate::types::SelfKind::Ref) => Ty::Ref {
            mutable: false,
            inner: Box::new(ty.clone()),
        },
        Some(crate::types::SelfKind::RefMut) => Ty::Ref {
            mutable: true,
            inner: Box::new(ty.clone()),
        },
        _ => ty.clone(),
    });
    let receiver = method.self_kind.and(receiver);

    let mut resolved_params = Vec::new();
    for param in &method.params {
        let Some(rust_ty) = &param.rust_ty else {
            continue;
        };
        match resolve_type(rust_ty, &env) {
            Ok(ty) => resolved_params.push((param.name.clone(), ty)),
            Err(diag) => {
                sink.push(diag);
                return None;
            }
        }
    }
    // A function with no written return type returns the unit type.
    let ret = match &method.rust_return {
        None => Ty::Unit,
        Some(rust_ty) => match resolve_type(rust_ty, &env) {
            Ok(ty) => ty,
            Err(diag) => {
                sink.push(diag);
                return None;
            }
        },
    };
    Some(MethodSig {
        params: resolved_params,
        ret,
        self_kind: method.self_kind,
        receiver,
        type_params: method.type_params.clone(),
        bounds: resolve_bounds(&method.syn_generics, &env, sink),
    })
}

impl TypeRegistry {
    /// A type declared directly in this module, ignoring imports and the prelude.
    pub fn module_type(&self, module: ModuleId, name: &str) -> Option<TypeId> {
        match self
            .modules()
            .get(module)
            .item(super::Ns::Type, name)
            .map(|i| i.def)
        {
            Some(super::Def::Type(id)) => Some(id),
            _ => None,
        }
    }

    /// A value declared directly in this module.
    pub fn module_value(&self, module: ModuleId, name: &str) -> Option<super::ValueId> {
        match self
            .modules()
            .get(module)
            .item(super::Ns::Value, name)
            .map(|i| i.def)
        {
            Some(super::Def::Value(id)) => Some(id),
            _ => None,
        }
    }
}
