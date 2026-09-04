//! Building the registry from the extracted crate.
//!
//! Two passes, because a field's type can name a type declared in another file:
//! first every module, `use` binding and declaration; then every field,
//! variant, constant and method signature, resolved against the declarations.

use super::module::{ModuleId, UseBinding, Vis};
use super::{
    resolve_type, AliasDef, MethodSig, SystemTypeDecl, TypeDecl, TypeEnv, TypeKind, TypeRegistry,
    ValueDef, VariantDef,
};
use crate::diag::DiagSink;
use crate::ty::{Ty, TypeId};
use crate::types::{FieldInfo, FnInfo, ImplInfo, RustFile, VisInfo};

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
    Method {
        id: TypeId,
        name: String,
        sig: MethodSig,
    },
    ConstType {
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
            Update::Method { id, name, sig } => {
                if let Some(def) = reg.def_mut(id) {
                    def.methods.insert(name, sig);
                }
            }
            Update::ConstType { id, ty } => {
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
            updates.push(Update::Fields { id, fields });
        }
    }

    for e in &mut file.enums {
        let id = reg.module_type(module, &e.name);
        let mut variants = Vec::new();
        for v in &mut e.variants {
            resolve_fields(reg, module, &e.type_params, &mut v.fields, sink);
            variants.push(VariantDef {
                name: v.name.clone(),
            });
        }
        if let Some(id) = id {
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

    for imp in &mut file.impls {
        resolve_impl(reg, module, imp, sink, updates);
    }

    for (name, sub_file) in &mut file.inline_modules {
        let Some(child) = reg.modules().get(module).children.get(name).copied() else {
            continue;
        };
        resolve_file(reg, child, sub_file, sink, updates);
    }
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

fn resolve_impl(
    reg: &TypeRegistry,
    module: ModuleId,
    imp: &ImplInfo,
    sink: &DiagSink,
    updates: &mut Vec<Update>,
) {
    let self_ty = imp.self_ty.as_ref().and_then(|ty| {
        let env = TypeEnv::new(reg, module, sink).with_params(&imp.type_params);
        match resolve_type(ty, &env) {
            Ok(ty) => Some(ty),
            Err(diag) => {
                sink.push(diag);
                None
            }
        }
    });

    // Methods attach to the type the impl block names. `impl TryInto<Clock> for
    // Vec<Vec<u8>>` is recorded against `Clock`, which is where extraction
    // points `target_type` and where the emitted class puts the method.
    let target = reg
        .module_type(module, &imp.target_type)
        .or_else(|| reg.lookup_item(module, &imp.target_type));
    let Some(target) = target else {
        // A blanket impl, or an impl on a type from another crate. Neither has
        // a home in this table; the impl table gives them one.
        if let Some(syn_ty) = &imp.self_ty {
            sink.report(
                syn::spanned::Spanned::span(syn_ty),
                format!(
                    "impl target `{}` does not resolve here; its methods are not registered",
                    imp.target_type
                ),
            );
        }
        return;
    };

    // The impl writes its own names for the receiver's arguments.
    let receiver_params = receiver_params(self_ty.as_ref(), &imp.type_params);

    for method in &imp.methods {
        let mut params = imp.type_params.clone();
        params.extend(method.type_params.iter().cloned());
        if let Some(sig) = method_sig(
            reg,
            module,
            &params,
            self_ty.as_ref(),
            &receiver_params,
            method,
            sink,
        ) {
            updates.push(Update::Method {
                id: target,
                name: method.name.clone(),
                sig,
            });
        }
    }
}

/// The impl's parameter name standing at each argument position of its self
/// type. `impl<E> Wrap<E>` gives `[Some("E")]`, whatever the struct calls its
/// own parameter; `impl Signal for Arc<Inner<T>>` gives `[None]`, because the
/// argument there is not a bare parameter and binds nothing.
fn receiver_params(self_ty: Option<&Ty>, impl_params: &[String]) -> Vec<Option<String>> {
    let Some(Ty::Named { args, .. }) = self_ty else {
        return Vec::new();
    };
    args.iter()
        .map(|arg| match arg {
            Ty::Param(name) if impl_params.iter().any(|p| p == name) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// A method's signature, or nothing when the engine could not name a type in
/// it. A method whose return type the engine cannot read stays out of the
/// table: "no answer" is the truth, and calling it `()` would not be.
#[allow(clippy::too_many_arguments)]
fn method_sig(
    reg: &TypeRegistry,
    module: ModuleId,
    params: &[String],
    self_ty: Option<&Ty>,
    receiver_params: &[Option<String>],
    method: &FnInfo,
    sink: &DiagSink,
) -> Option<MethodSig> {
    let env = TypeEnv::new(reg, module, sink)
        .with_params(params)
        .with_self(self_ty);
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
        is_static: method.is_static,
        receiver_params: receiver_params.to_vec(),
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
