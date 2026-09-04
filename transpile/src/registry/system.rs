//! Declaring the system types into the reserved system module.
//!
//! A system type is one of the foundational runtime types the port provides in
//! `@ankurah/base` — `Arc`, `RwLock`, `Option`, `HashMap`. They live in their
//! own module so that a crate type of the same leaf name shadows them where
//! Rust says it does and nowhere else, and each is declared under the full Rust
//! path it is written with, so `std::fmt::Result` cannot collapse onto
//! `std::result::Result`.
//!
//! Today the declarations come from `transpile.toml [system_types]`, which is
//! why every signature here is written as Rust and parsed by syn. The
//! std-surface step replaces that table with signature-only Rust stub files
//! and hands them to this same function.

use super::module::{Def, ItemDef, Ns, Vis};
use super::{resolve_type, MethodSig, TypeDecl, TypeEnv, TypeKind, TypeRegistry};
use crate::diag::{Diag, DiagSink};

/// One declared system type. Method return types are Rust source text, parsed
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
    /// How reaching through this type is written: `None` for a plain type,
    /// `Some("")` for a transparent wrapper, `Some("value")` for one that needs
    /// an accessor emitted.
    pub deref_field: Option<String>,
    /// Method name to its return type, in Rust: `("write", "RwLockWriteGuard<T>")`.
    pub methods: Vec<(String, String)>,
}

/// Declare every system type, then resolve their signatures. Two passes,
/// because `RwLock::write` returns an `RwLockWriteGuard` that is itself one of
/// these declarations.
pub fn declare_system_types(reg: &mut TypeRegistry, decls: &[SystemTypeDecl], sink: &DiagSink) {
    let system = reg.system_root();
    sink.set_file("<system types>");

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
        let mut methods = Vec::new();
        for (method, ret_src) in &decl.methods {
            let Ok(syn_ty) = syn::parse_str::<syn::Type>(ret_src) else {
                sink.push(flat_diag(format!(
                    "system type `{}`: return type of `{}` is not a Rust type: `{}`",
                    decl.name, method, ret_src
                )));
                continue;
            };
            let env = TypeEnv::new(reg, system, sink).with_params(&decl.type_params);
            match resolve_type(&syn_ty, &env) {
                Ok(ty) => methods.push((
                    method.clone(),
                    MethodSig {
                        params: Vec::new(),
                        ret: ty,
                        is_static: false,
                        // A system declaration writes the type's own parameters.
                        receiver_params: decl.type_params.iter().map(|p| Some(p.clone())).collect(),
                    },
                )),
                Err(diag) => sink.push(diag),
            }
        }
        if let Some(def) = reg.def_mut(id) {
            for (name, sig) in methods {
                def.methods.insert(name, sig);
            }
        }
    }
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
    use crate::ty::Ty;

    fn decls() -> Vec<SystemTypeDecl> {
        vec![
            SystemTypeDecl {
                name: "RwLock".into(),
                path: "std::sync::RwLock".into(),
                type_params: vec!["T".into()],
                deref_field: None,
                methods: vec![("write".into(), "std::sync::RwLockWriteGuard<T>".into())],
            },
            SystemTypeDecl {
                name: "RwLockWriteGuard".into(),
                path: "std::sync::RwLockWriteGuard".into(),
                type_params: vec!["T".into()],
                deref_field: Some("value".into()),
                methods: vec![],
            },
        ]
    }

    #[test]
    fn method_return_types_resolve_against_other_system_types() {
        let mut reg = TypeRegistry::new("signals");
        let sink = DiagSink::new();
        declare_system_types(&mut reg, &decls(), &sink);
        assert_eq!(sink.len(), 0, "{:?}", sink.sorted());

        let rwlock = reg.system_type("std::sync::RwLock").unwrap();
        let guard = reg.system_type("std::sync::RwLockWriteGuard").unwrap();
        let sig = reg.def(rwlock).unwrap().methods.get("write").unwrap();
        assert_eq!(
            sig.ret,
            Ty::Named {
                id: guard,
                args: vec![Ty::Param("T".into())]
            }
        );
        assert_eq!(
            reg.def(guard).unwrap().deref_field.as_deref(),
            Some("value")
        );
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
