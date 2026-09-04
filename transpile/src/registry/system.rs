//! Declaring the system types into the reserved system module.
//!
//! A system type is one of the foundational runtime types the port provides in
//! `@ankurah/base` — `Arc`, `RwLock`, `Option`, `HashMap`. They live in their
//! own module so that a crate type of the same leaf name shadows them where
//! Rust says it does and nowhere else, and each is declared under the full Rust
//! path it is written with, so `std::fmt::Result` cannot collapse onto
//! `std::result::Result`.
//!
//! Each declaration becomes an ordinary inherent `impl` in the impl table, and
//! a wrapper's `deref_field` becomes an ordinary `impl Deref`, so that method
//! resolution has exactly one mechanism to run. Today the declarations come
//! from `transpile.toml [system_types]`, which is why every signature here is
//! written as Rust and parsed by syn. The std-surface step replaces that table
//! with signature-only Rust stub files and hands them to this same function.

use std::collections::HashMap;

use super::impls::{ImplDef, ImplId};
use super::module::{Def, ItemDef, Ns, Vis};
use super::traits::{TraitDef, TraitMethod};
use super::{resolve_type, MethodSig, TypeDecl, TypeEnv, TypeKind, TypeRegistry};
use crate::diag::{Diag, DiagSink};
use crate::ty::{TraitRef, Ty, TypeId};
use crate::types::SelfKind;

/// The Rust path of the trait every dereference goes through. It is declared
/// here rather than read from the config because method resolution names it
/// structurally; the std-surface step replaces this declaration with the stub
/// for `std::ops::Deref` and nothing else changes.
pub const DEREF_PATH: &str = "std::ops::Deref";

/// The Rust path of `Clone`. Every `#[derive(Clone)]` in the corpus registers an
/// impl of it (spec 4.10), and a `T: Clone` bound resolves to it, so that
/// `guard.clone()` clones what the guard holds rather than the guard.
pub const CLONE_PATH: &str = "std::clone::Clone";

/// One declared system type. Method signatures are Rust source text, parsed
/// here — never TypeScript.
#[derive(Debug, Clone)]
pub struct SystemTypeDecl {
    /// The name emission writes, which is the port's name for the type.
    pub name: String,
    /// The full Rust path a written type has to use to reach it, e.g.
    /// `std::sync::Arc`. A bare name reaches it only through an import or the
    /// prelude.
    pub path: String,
    pub type_params: Vec<String>,
    /// How reaching through this type is written in TypeScript: `None` for a
    /// plain type, `Some("")` for a transparent wrapper, `Some("value")` for one
    /// that needs an accessor emitted.
    pub deref_field: Option<String>,
    /// What this type dereferences to, in Rust: `Vec<T>` gives `[T]`. Left
    /// unwritten, a wrapper with a `deref_field` dereferences to its first type
    /// argument, which is what `Arc<T>` and every guard do.
    pub deref_target: Option<String>,
    /// Method name to its whole Rust signature, e.g.
    /// `("write", "fn write(&self) -> RwLockWriteGuard<T>")`.
    pub methods: Vec<(String, String)>,
}

/// Declare every system type, then resolve their signatures. Two passes,
/// because `RwLock::write` returns an `RwLockWriteGuard` that is itself one of
/// these declarations.
pub fn declare_system_types(reg: &mut TypeRegistry, decls: &[SystemTypeDecl], sink: &DiagSink) {
    let system = reg.system_root();
    sink.set_file("<system types>");

    let deref = declare_deref_trait(reg, sink);
    declare_clone_trait(reg, sink);

    for decl in decls {
        let leaf = leaf_of(&decl.path);
        let type_decl = TypeDecl {
            name: decl.name.clone(),
            kind: TypeKind::Struct,
            type_params: decl.type_params.clone(),
            deref_field: decl.deref_field.clone(),
            vis: Vis::Public,
        };
        let id = match reg.declare_type(system, type_decl) {
            Ok(id) => id,
            Err(err) => {
                sink.push(flat_diag(format!(
                    "cannot declare system type `{}`: {}",
                    decl.name, err
                )));
                continue;
            }
        };
        reg.record_system_path(&decl.path, id);
        // The module entry is keyed by the Rust leaf name, which is what the
        // other declarations write and what the prelude exports; `decl.name` is
        // the port's name and belongs to emission.
        if leaf != decl.name {
            reg.modules_mut()
                .get_mut(system)
                .items
                .remove(&(Ns::Type, decl.name.clone()));
        }
        reg.modules_mut().get_mut(system).items.insert(
            (Ns::Type, leaf.to_string()),
            ItemDef {
                def: Def::Type(id),
                vis: Vis::Public,
            },
        );
    }

    for decl in decls {
        let Some(id) = reg.system_type(&decl.path) else {
            continue;
        };
        let self_ty = Ty::Named {
            id,
            args: decl
                .type_params
                .iter()
                .map(|p| Ty::Param(p.clone()))
                .collect(),
        };

        let mut methods = HashMap::new();
        for (name, source) in &decl.methods {
            match parse_signature(reg, system, decl, &self_ty, source, sink) {
                Some(sig) => {
                    methods.insert(name.clone(), sig);
                }
                None => continue,
            }
        }
        if !methods.is_empty() {
            reg.add_impl(ImplDef {
                id: ImplId(0),
                generics: decl.type_params.clone(),
                bounds: Vec::new(),
                self_ty: self_ty.clone(),
                trait_ref: None,
                assoc_types: HashMap::new(),
                methods,
            });
        }

        // A wrapper's accessor and its `Deref` target are two different facts;
        // the table carries the second and `TypeDef.deref_field` the first.
        if decl.deref_field.is_some() {
            let Some(target) = deref_target(reg, system, decl, &self_ty, sink) else {
                continue;
            };
            let mut assoc_types = HashMap::new();
            assoc_types.insert("Target".to_string(), target);
            reg.add_impl(ImplDef {
                id: ImplId(0),
                generics: decl.type_params.clone(),
                bounds: Vec::new(),
                self_ty,
                trait_ref: Some(TraitRef {
                    id: deref,
                    args: Vec::new(),
                    bindings: Vec::new(),
                }),
                assoc_types,
                methods: HashMap::new(),
            });
        }
    }
}

/// What a wrapper dereferences to: the target it declares, or its first type
/// argument. The second is the shim `deref_field` stood for before the impl
/// table existed, and it is what `Arc<T>` and every lock guard mean.
fn deref_target(
    reg: &TypeRegistry,
    system: super::ModuleId,
    decl: &SystemTypeDecl,
    self_ty: &Ty,
    sink: &DiagSink,
) -> Option<Ty> {
    if let Some(source) = &decl.deref_target {
        let Ok(syn_ty) = syn::parse_str::<syn::Type>(source) else {
            sink.push(flat_diag(format!(
                "system type `{}`: deref target `{}` is not a Rust type",
                decl.name, source
            )));
            return None;
        };
        let env = TypeEnv::new(reg, system, sink)
            .with_params(&decl.type_params)
            .with_self(Some(self_ty));
        return match resolve_type(&syn_ty, &env) {
            Ok(ty) => Some(ty),
            Err(diag) => {
                sink.push(diag);
                None
            }
        };
    }
    match decl.type_params.first() {
        Some(param) => Some(Ty::Param(param.clone())),
        None => {
            sink.push(flat_diag(format!(
                "system type `{}` is written as a wrapper but has nothing to \
                 dereference to: give it a type parameter or a `deref_target`",
                decl.name
            )));
            None
        }
    }
}

/// One declared method, parsed as the Rust signature it is written as.
fn parse_signature(
    reg: &TypeRegistry,
    system: super::ModuleId,
    decl: &SystemTypeDecl,
    self_ty: &Ty,
    source: &str,
    sink: &DiagSink,
) -> Option<MethodSig> {
    let Ok(item) = syn::parse_str::<syn::TraitItemFn>(&format!("{};", source)) else {
        sink.push(flat_diag(format!(
            "system type `{}`: `{}` is not a Rust method signature",
            decl.name, source
        )));
        return None;
    };
    let mut params = decl.type_params.clone();
    for generic in &item.sig.generics.params {
        if let syn::GenericParam::Type(t) = generic {
            params.push(t.ident.to_string());
        }
    }
    let env = TypeEnv::new(reg, system, sink)
        .with_params(&params)
        .with_self(Some(self_ty));

    let mut self_kind = None;
    let mut resolved = Vec::new();
    for arg in &item.sig.inputs {
        match arg {
            syn::FnArg::Receiver(r) => {
                self_kind = Some(match (&r.reference, r.mutability.is_some()) {
                    (Some(_), true) => SelfKind::RefMut,
                    (Some(_), false) => SelfKind::Ref,
                    (None, _) => SelfKind::Value,
                });
            }
            syn::FnArg::Typed(pat) => {
                let name = match &*pat.pat {
                    syn::Pat::Ident(id) => crate::name_map::to_camel_case(&id.ident.to_string()),
                    _ => "arg".to_string(),
                };
                match resolve_type(&pat.ty, &env) {
                    Ok(ty) => resolved.push((name, ty)),
                    Err(diag) => {
                        sink.push(diag);
                        return None;
                    }
                }
            }
        }
    }
    let ret = match &item.sig.output {
        syn::ReturnType::Default => Ty::Unit,
        syn::ReturnType::Type(_, ty) => match resolve_type(ty, &env) {
            Ok(ty) => ty,
            Err(diag) => {
                sink.push(diag);
                return None;
            }
        },
    };
    let receiver = self_kind.map(|kind| match kind {
        SelfKind::Ref => Ty::Ref {
            mutable: false,
            inner: Box::new(self_ty.clone()),
        },
        SelfKind::RefMut => Ty::Ref {
            mutable: true,
            inner: Box::new(self_ty.clone()),
        },
        _ => self_ty.clone(),
    });
    Some(MethodSig {
        params: resolved,
        ret,
        self_kind,
        receiver,
        type_params: Vec::new(),
    })
}

/// `std::ops::Deref`, which every dereference in the impl table goes through.
fn declare_deref_trait(reg: &mut TypeRegistry, sink: &DiagSink) -> TypeId {
    let system = reg.system_root();
    let decl = TypeDecl {
        name: "Deref".to_string(),
        kind: TypeKind::Trait,
        type_params: Vec::new(),
        deref_field: None,
        vis: Vis::Public,
    };
    let id = match reg.declare_type(system, decl) {
        Ok(id) => id,
        Err(err) => {
            sink.push(flat_diag(format!("cannot declare `Deref`: {}", err)));
            // Nothing else can be declared either if the id space is gone; the
            // caller's own diagnostics will say so.
            return TypeId(0);
        }
    };
    reg.record_system_path(DEREF_PATH, id);

    let mut methods = HashMap::new();
    methods.insert(
        "deref".to_string(),
        TraitMethod {
            sig: MethodSig {
                params: Vec::new(),
                ret: Ty::Ref {
                    mutable: false,
                    inner: Box::new(Ty::Assoc {
                        base: Box::new(Ty::Param("Self".to_string())),
                        trait_: None,
                        name: "Target".to_string(),
                    }),
                },
                self_kind: Some(SelfKind::Ref),
                receiver: Some(Ty::Ref {
                    mutable: false,
                    inner: Box::new(Ty::Param("Self".to_string())),
                }),
                type_params: Vec::new(),
            },
            has_default: false,
        },
    );
    reg.insert_trait(TraitDef {
        id,
        generics: Vec::new(),
        supertraits: Vec::new(),
        assoc_types: vec!["Target".to_string()],
        methods,
    });
    id
}

/// `std::clone::Clone`, whose single method every derive supplies.
fn declare_clone_trait(reg: &mut TypeRegistry, sink: &DiagSink) -> TypeId {
    let system = reg.system_root();
    let decl = TypeDecl {
        name: "Clone".to_string(),
        kind: TypeKind::Trait,
        type_params: Vec::new(),
        deref_field: None,
        vis: Vis::Public,
    };
    let id = match reg.declare_type(system, decl) {
        Ok(id) => id,
        Err(err) => {
            sink.push(flat_diag(format!("cannot declare `Clone`: {}", err)));
            return TypeId(0);
        }
    };
    reg.record_system_path(CLONE_PATH, id);
    let mut methods = HashMap::new();
    methods.insert(
        "clone".to_string(),
        TraitMethod {
            sig: MethodSig {
                params: Vec::new(),
                ret: Ty::Param("Self".to_string()),
                self_kind: Some(SelfKind::Ref),
                receiver: Some(Ty::Ref {
                    mutable: false,
                    inner: Box::new(Ty::Param("Self".to_string())),
                }),
                type_params: Vec::new(),
            },
            has_default: false,
        },
    );
    reg.insert_trait(TraitDef {
        id,
        generics: Vec::new(),
        supertraits: Vec::new(),
        assoc_types: Vec::new(),
        methods,
    });
    id
}

/// The Rust name a system type is written with: the last segment of its path.
pub(super) fn leaf_of(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

fn flat_diag(message: String) -> Diag {
    Diag {
        file: "<system types>".to_string(),
        line: 0,
        col: 0,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Probe;
    use crate::ty::Prim;

    fn decls() -> Vec<SystemTypeDecl> {
        vec![
            SystemTypeDecl {
                name: "RwLock".into(),
                path: "std::sync::RwLock".into(),
                type_params: vec!["T".into()],
                deref_field: None,
                deref_target: None,
                methods: vec![(
                    "write".into(),
                    "fn write(&self) -> std::sync::RwLockWriteGuard<T>".into(),
                )],
            },
            SystemTypeDecl {
                name: "RwLockWriteGuard".into(),
                path: "std::sync::RwLockWriteGuard".into(),
                type_params: vec!["T".into()],
                deref_field: Some("value".into()),
                deref_target: None,
                methods: vec![],
            },
        ]
    }

    #[test]
    fn a_declared_method_becomes_an_inherent_impl() {
        let mut reg = TypeRegistry::new("signals");
        let sink = DiagSink::new();
        declare_system_types(&mut reg, &decls(), &sink);
        assert_eq!(sink.len(), 0, "{:?}", sink.sorted());

        let rwlock = reg.system_type("std::sync::RwLock").unwrap();
        let guard = reg.system_type("std::sync::RwLockWriteGuard").unwrap();
        let probe = Probe::new(&reg, reg.crate_root());
        let lock = Ty::Named {
            id: rwlock,
            args: vec![Ty::Prim(Prim::U8)],
        };
        let found = probe.resolve_method(&lock, "write").expect("resolves");
        assert_eq!(
            found.ret,
            Ty::Named {
                id: guard,
                args: vec![Ty::Prim(Prim::U8)]
            }
        );
        assert!(found.steps.is_empty(), "the method is on the lock itself");
    }

    #[test]
    fn a_wrapper_becomes_a_deref_impl_and_keeps_its_accessor() {
        let mut reg = TypeRegistry::new("signals");
        let sink = DiagSink::new();
        declare_system_types(&mut reg, &decls(), &sink);
        let guard = reg.system_type("std::sync::RwLockWriteGuard").unwrap();
        let probe = Probe::new(&reg, reg.crate_root());
        let held = Ty::Named {
            id: guard,
            args: vec![Ty::Prim(Prim::U8)],
        };
        let step = probe.deref_once(&held).expect("a guard dereferences");
        assert_eq!(step.to, Ty::Prim(Prim::U8));
        assert_eq!(step.accessor.map(|a| a.written()).as_deref(), Some("value"));
    }

    #[test]
    fn a_system_type_outside_the_prelude_needs_its_path() {
        let mut reg = TypeRegistry::new("signals");
        let sink = DiagSink::new();
        declare_system_types(&mut reg, &decls(), &sink);
        let module = reg.modules_mut().module_for_file("broadcast.rs");

        // Reachable by the path it is declared under.
        let by_path = reg.lookup_type(module, &["std".into(), "sync".into(), "RwLock".into()]);
        assert!(matches!(by_path, Ok(Some(super::Def::Type(_)))));
        // But not as a bare name: `RwLock` is not in Rust's prelude either.
        assert_eq!(reg.lookup_type(module, &["RwLock".into()]), Ok(None));
    }
}
