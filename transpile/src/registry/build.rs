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
    /// Hardcoded files are read for their declarations only. Their TypeScript
    /// is hand-written, so nothing may be emitted for them, but the types they
    /// declare are part of the crate and other files resolve through them.
    pub declarations_only: bool,
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

    reg
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
            derived_impls(reg, id, &s.type_params, &s.derives, updates);
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
            derived_impls(reg, id, &e.type_params, &e.derives, updates);
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

    // A free function's return type, so that `foo()` in expression position has
    // a type. Its parameters are the caller's business and are not stored.
    for f in &file.functions {
        let Some(id) = reg.module_value(module, &f.name) else {
            continue;
        };
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
/// narrowed to the derives whose absence stops resolution. `Serialize`,
/// `Deserialize` and `thiserror::Error` produce code as well as impls and are
/// that step's business.
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

fn derived_impls(
    reg: &TypeRegistry,
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
        updates.push(Update::Impl(ImplDef {
            id: ImplId(0),
            generics: type_params.to_vec(),
            bounds: Vec::new(),
            self_ty: self_ty.clone(),
            trait_ref: Some(crate::ty::TraitRef {
                id: trait_id,
                args: Vec::new(),
                bindings: Vec::new(),
            }),
            assoc_types: HashMap::new(),
            methods: HashMap::new(),
        }));
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
                    has_default: method.has_default_body,
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
        generics: imp.type_params.clone(),
        bounds,
        self_ty,
        trait_ref,
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
