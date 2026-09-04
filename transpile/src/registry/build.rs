//! Building the registry from the extracted crate.
//!
//! Two passes, because a field's type can name a type declared in another file:
//! first every module, `use` binding and declaration; then every field,
//! variant, constant, trait and `impl` block, resolved against the declarations.

use std::collections::HashMap;

use super::impls::{Bound, ImplDef, ImplId};
use super::module::{ModuleId, UseBinding, Vis};
use super::traits::{TraitDef, TraitMethod};
use super::{
    resolve_type, AliasDef, MethodSig, SystemTypeDecl, TypeDecl, TypeEnv, TypeKind, TypeRegistry,
    ValueDef, VariantDef,
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
enum Update {
    Fields {
        id: TypeId,
        fields: Vec<(String, Ty)>,
    },
    Variants {
        id: TypeId,
        variants: Vec<VariantDef>,
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
    system: &[SystemTypeDecl],
    crate_names: &[String],
    sink: &DiagSink,
) -> TypeRegistry {
    let primary = crate_names.first().cloned().unwrap_or_default();
    let mut reg = TypeRegistry::new(&primary);
    for name in crate_names.iter().skip(1) {
        reg.add_crate_name(name);
    }

    super::system::declare_system_types(&mut reg, system, sink);

    for entry in files.iter() {
        let module = reg.modules_mut().module_for_file(&entry.path);
        sink.set_file(&entry.path);
        declare_file(&mut reg, module, &entry.file, sink);
    }

    let mut updates = Vec::new();
    for entry in files.iter_mut() {
        sink.set_file(&entry.path);
        let Some(module) = reg.modules().lookup_file(&entry.path) else {
            continue;
        };
        resolve_file(&reg, module, &mut entry.file, sink, &mut updates);
    }

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

    reg
}

/// Pass one: the module's imports and everything it declares, in both namespaces.
fn declare_file(reg: &mut TypeRegistry, module: ModuleId, file: &RustFile, sink: &DiagSink) {
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
        deref_field: None,
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
fn resolve_file(
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
            if s.derives.iter().any(|d| d == "Clone") {
                if let Some(def) = derived_clone(reg, id, &s.type_params) {
                    updates.push(Update::Impl(def));
                }
            }
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
            if e.derives.iter().any(|d| d == "Clone") {
                if let Some(def) = derived_clone(reg, id, &e.type_params) {
                    updates.push(Update::Impl(def));
                }
            }
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

/// `#[derive(Clone)]` written out as the impl it stands for.
///
/// This is the first of the derive hooks the spec calls for (4.10, step 7), and
/// it is here now because the engine used to answer `.clone()` by assuming
/// every receiver was cloneable. With the impl registered, `node.clone()`
/// resolves on `Node` and `guard.clone()` reaches through the guard to what it
/// holds, which is what Rust does.
fn derived_clone(
    reg: &TypeRegistry,
    id: TypeId,
    type_params: &[String],
) -> Option<ImplDef> {
    let clone = reg.system_type(super::system::CLONE_PATH)?;
    let self_ty = Ty::Named {
        id,
        args: type_params.iter().map(|p| Ty::Param(p.clone())).collect(),
    };
    let mut methods = HashMap::new();
    methods.insert(
        "clone".to_string(),
        MethodSig {
            params: Vec::new(),
            ret: self_ty.clone(),
            self_kind: Some(crate::types::SelfKind::Ref),
            receiver: Some(Ty::Ref {
                mutable: false,
                inner: Box::new(self_ty.clone()),
            }),
            type_params: Vec::new(),
        },
    );
    Some(ImplDef {
        id: ImplId(0),
        generics: type_params.to_vec(),
        bounds: Vec::new(),
        self_ty,
        trait_ref: Some(crate::ty::TraitRef {
            id: clone,
            args: Vec::new(),
            bindings: Vec::new(),
        }),
        assoc_types: HashMap::new(),
        methods,
    })
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
    match super::resolve_type::trait_ref(t, env) {
        Ok(trait_ref) => out.push(Bound {
            subject: subject.clone(),
            trait_ref,
        }),
        Err(diag) => sink.push(diag),
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
