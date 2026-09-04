//! Impls that have no class to hang their methods on.
//!
//! For: an `impl` in Rust does not need its self type to be a struct the same
//! crate declared. `impl<F: Fn(T)> IntoBroadcastListener<T> for F` is written
//! for whatever satisfies its bound, `impl<T> Observer for Arc<Inner<T>>` is
//! written for a std wrapper, and neither has a TypeScript class to become a
//! method on. Emission looked for a class named by the impl's target, found
//! none, and dropped the impl — so signals' listener conversion and its
//! observer both vanished from the output, and every call to them named
//! something that did not exist.
//!
//! Rust's own answer is that a method is a function whose first parameter is
//! the receiver, and that is what these become: a module-level function in the
//! module that declares the impl, taking the receiver first.
//!
//! # The name
//!
//! One function stands for one method of one impl, and the name has to come out
//! the same wherever it is computed — at the impl, and at every call.
//!
//! - The self type is written as its named constructors from the outside in,
//!   joined by `_`, with arguments that are the impl's own parameters dropped:
//!   `Arc<Inner<T>>` is `Arc_Inner`, `std::sync::mpsc::Sender<T>` is `Sender`.
//!   A callable bound inside it carries how many arguments it takes, because
//!   `Arc<dyn Fn(T)>` and `Arc<dyn Fn()>` are two impls that differ in nothing
//!   else: they are `Arc_Fn1` and `Arc_Fn0`.
//! - The method's TypeScript name follows, after an underscore.
//! - A blanket impl — one written for a bare parameter of its own — has no
//!   constructor to name, so its function takes the method's TypeScript name
//!   alone: `intoBroadcastListener`.
//!
//! Two impls in one file that would take the same name is a diagnostic, not a
//! silent overwrite.

mod dispatch;
mod name;
#[cfg(test)]
mod tests;

pub use dispatch::{free_call, has_emitted_class, is_reference_forwarding, FreeCall};
pub use name::free_fn_name;

use syn::spanned::Spanned;

use crate::registry::{ModuleId, TypeEnv, TypeRegistry};
use crate::types::{FnInfo, ImplInfo, RustFile};

/// One method of one impl, written as a module-level function.
pub struct FreeFn {
    pub name: String,
    pub text: String,
}

/// The functions one file's impls contribute, in the order the impls are
/// written.
///
/// The impls that need them are the ones whose self type has no emitted class:
/// a bare type parameter, a declared system type, a foreign type. Everything
/// else is a method on a class and is emitted there.
pub fn free_functions(reg: &TypeRegistry, module: ModuleId, file: &RustFile) -> Vec<FreeFn> {
    // Resolving the impl's self type a second time can only repeat what the
    // registry already reported when it built the impl table, so the repeat is
    // dropped rather than counted twice.
    let quiet = crate::diag::DiagSink::new();
    let mut out: Vec<FreeFn> = Vec::new();
    for imp in &file.impls {
        let Some(self_ty) = resolved_self(reg, module, imp, &quiet) else {
            continue;
        };
        if has_emitted_class(reg, &self_ty)
            || is_reference_forwarding(&self_ty, &imp.type_params)
        {
            continue;
        }
        for method in &imp.methods {
            if method.is_test {
                continue;
            }
            let name = free_fn_name(reg, &self_ty, &imp.type_params, &method.ts_name);
            if out.iter().any(|f| f.name == name) {
                crate::diag::pending::park(
                    imp.self_ty.as_ref().map(|t| t.span()).unwrap_or_else(proc_macro2::Span::call_site),
                    format!(
                        "`{}` and an earlier impl in this file both emit a module-level \
                         function called `{}`, so one of them would be lost: the two impls \
                         differ in something the naming scheme does not write down",
                        method.name, name
                    ),
                );
                continue;
            }
            out.push(FreeFn {
                name: name.clone(),
                text: write(reg, module, &self_ty, imp, method, &name),
            });
        }
    }
    out
}

/// Does this impl's self type have an emitted class for its methods to hang
/// on? An impl the engine could not resolve a self type for keeps the
/// behaviour it had, which is to be written onto a class named by its target.
pub fn impl_has_class(reg: &TypeRegistry, module: ModuleId, imp: &ImplInfo) -> bool {
    let quiet = crate::diag::DiagSink::new();
    match resolved_self(reg, module, imp, &quiet) {
        Some(ty) => has_emitted_class(reg, &ty) || is_reference_forwarding(&ty, &imp.type_params),
        None => true,
    }
}

/// The impl's self type as the registry reads it.
fn resolved_self(
    reg: &TypeRegistry,
    module: ModuleId,
    imp: &ImplInfo,
    sink: &crate::diag::DiagSink,
) -> Option<crate::ty::Ty> {
    let written = imp.self_ty.as_ref()?;
    let env = TypeEnv::new(reg, module, sink).with_params(&imp.type_params);
    crate::registry::resolve_type(written, &env).ok()
}

/// One method, as `export function name(self: T, ..): R { .. }`.
///
/// The receiver keeps the name Rust gave it. The body already reads `self`
/// wherever the source did, because the body translator emits `self` as this
/// parameter rather than as `this` when it is writing a free function.
fn write(
    reg: &TypeRegistry,
    module: ModuleId,
    self_ty: &crate::ty::Ty,
    imp: &ImplInfo,
    method: &FnInfo,
    name: &str,
) -> String {
    let generics = declared_generics(reg, module, imp, method);
    let mut params = vec![format!("self: {}", crate::name_map::map_ty(reg, self_ty))];
    params.extend(
        method
            .params
            .iter()
            .filter(|p| !p.is_self)
            .map(|p| format!("{}: {}", crate::name_map::to_camel_case(&p.name), p.ty)),
    );
    let ret = if method.return_type.is_empty() {
        "void".to_string()
    } else {
        method.return_type.clone()
    };
    let ret = if method.is_async {
        format!("Promise<{}>", ret)
    } else {
        ret
    };
    let body = match &method.body_ts {
        Some(text) => text.clone(),
        None => "throw new Error('TODO');\n".to_string(),
    };
    let body = body
        .lines()
        .map(|line| if line.is_empty() { String::new() } else { format!("  {}", line) })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    format!(
        "export {}function {}{}({}): {} {{\n{}}}\n\n",
        if method.is_async { "async " } else { "" },
        name,
        generics,
        params.join(", "),
        ret,
        body,
    )
}

/// The generic parameters the function declares: the impl's own, then any the
/// method adds on top, each carrying what its bounds require of it.
///
/// The constraint is not decoration. `impl<F: Fn(u8) -> u8> IntoListener for F`
/// emits a function whose body calls its receiver, and a receiver declared as a
/// bare `F` is an `unknown` that TypeScript refuses to call. Writing the bound
/// out is what makes the emitted function say what Rust says.
fn declared_generics(
    reg: &TypeRegistry,
    module: ModuleId,
    imp: &ImplInfo,
    method: &FnInfo,
) -> String {
    let quiet = crate::diag::DiagSink::new();
    let env = TypeEnv::new(reg, module, &quiet).with_params(&imp.type_params);
    let bounds = crate::registry::resolve_bounds(&imp.generics, &env, &quiet);

    let mut names: Vec<String> = imp.type_params.clone();
    for extra in &method.type_params {
        if !names.iter().any(|n| n == extra) {
            names.push(extra.clone());
        }
    }
    if names.is_empty() {
        return String::new();
    }
    let written: Vec<String> = names
        .iter()
        .map(|name| match constraint(reg, name, &bounds) {
            Some(ts) => format!("{} extends {}", name, ts),
            None => name.clone(),
        })
        .collect();
    format!("<{}>", written.join(", "))
}

/// What one parameter's bounds require of it, as TypeScript.
///
/// `Send`, `Sync` and `Sized` hold or do not hold by the type's own shape and
/// have nothing to say in TypeScript, so they are left out; a parameter left
/// with nothing else gets no constraint at all.
fn constraint(
    reg: &TypeRegistry,
    param: &str,
    bounds: &[crate::registry::impls::Bound],
) -> Option<String> {
    let sized = reg.system_type("std::marker::Sized");
    let carried: Vec<crate::ty::TraitRef> = bounds
        .iter()
        .filter(|b| matches!(&b.subject, crate::ty::Ty::Param(name) if name == param))
        .map(|b| b.trait_ref.clone())
        .filter(|t| Some(t.id) != sized)
        .filter(|t| !reg.trait_def(t.id).is_some_and(|d| d.is_auto))
        .collect();
    if carried.is_empty() {
        return None;
    }
    Some(crate::name_map::map_ty(
        reg,
        &crate::ty::Ty::ImplTrait { bounds: carried },
    ))
}
