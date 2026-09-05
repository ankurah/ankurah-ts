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

mod conversion;
mod naming;
pub use naming::{
    conversion_name, conversion_name_of_impl, resolve_conversion_names, set_conversion_names,
};
mod dispatch;
mod name;
mod open_dispatch;
#[cfg(test)]
mod tests;

pub use conversion::{conversion_call, conversion_names};
pub use dispatch::{
    class_module,
    emits_as_free_function, forwards_every_method, free_call, has_emitted_class,
    is_reference_forwarding, FreeCall,
};
pub use open_dispatch::set_contested_traits;
pub use open_dispatch::{
    dispatcher_name, dispatchers, open_bound_call, record_wanted, refusal as dispatcher_refusal,
    Dispatcher, OpenCall,
};
pub use name::{free_fn_name, method_symbol};

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
    free_functions_reporting(reg, module, file, &crate::diag::DiagSink::new())
}

/// The same, with a sink the run reads.
///
/// `free_functions` is asked several times per run — once to build the import
/// map, once to write the file — so what it reports has to go somewhere a
/// caller decides, or one gap would be counted once per ask. The pass that
/// owns the file's diagnostics hands in the run's sink; every other ask hands
/// in one nobody reads.
pub fn free_functions_reporting(
    reg: &TypeRegistry,
    module: ModuleId,
    file: &RustFile,
    sink: &crate::diag::DiagSink,
) -> Vec<FreeFn> {
    // Resolving the impl's self type a second time can only repeat what the
    // registry already reported when it built the impl table, so the repeat is
    // dropped rather than counted twice.
    let quiet = crate::diag::DiagSink::new();
    let mut out: Vec<FreeFn> = Vec::new();
    // What each emitted name's impl was written for, so a name two impls take
    // can say whether they differ in anything emission keeps.
    let mut written_for: Vec<(String, String)> = Vec::new();
    for imp in &file.impls {
        let Some(self_ty) = resolved_self(reg, module, imp, &quiet) else {
            continue;
        };
        // An impl whose class is emitted in THIS file becomes methods on it.
        // One whose class is emitted somewhere else cannot: a TypeScript class
        // is one declaration in one file, and Rust lets the impl sit anywhere.
        // Those become module-level functions here, like an impl on a type the
        // port does not declare at all, and the import map carries their names.
        match class_module(reg, &self_ty) {
            Some(home) if home == module => continue,
            // A type whose TypeScript is written by hand carries its own
            // methods: `Attested<T>`'s conversions are in auth.provided.ts, and
            // emitting them again beside it would give the port two of each.
            Some(_)
                if self_ty
                    .peel_refs()
                    .id()
                    .is_some_and(|id| reg.is_hand_written(id)) =>
            {
                continue;
            }
            Some(_) => {
                sink.push(crate::diag::Diag::at(
                    &sink.file(),
                    imp.self_ty
                        .as_ref()
                        .map(|t| t.span())
                        .unwrap_or_else(proc_macro2::Span::call_site),
                    format!(
                        "`{}` is declared in another module, so this impl is written as module-level \
                         functions here rather than as methods on its class",
                        imp.target_type
                    ),
                ));
            }
            None => {}
        }
        // An impl written for a reference to its own parameter forwards to the
        // value inside, and emission erases the reference — so emitting it
        // would write a function whose body calls itself. That is true only of
        // an impl that really does forward; one that does something of its own
        // is a real impl, and skipping it left every call to it naming nothing.
        if is_reference_forwarding(&self_ty, &imp.type_params) {
            if forwards_every_method(imp) {
                continue;
            }
            sink.push(crate::diag::Diag::at(
                &sink.file(),
                imp.self_ty
                    .as_ref()
                    .map(|t| t.span())
                    .unwrap_or_else(proc_macro2::Span::call_site),
                format!(
                    "`impl {} for &{}` does something of its own rather than forwarding to the \
                     value inside, and emission erases the reference, so its function and the \
                     one for the value itself would be the same name",
                    imp.trait_name().unwrap_or_default(),
                    imp.target_type
                ),
            ));
            // Emitting it anyway would write a function no call site names —
            // `free_call` decides from the impl's shape, which is all the
            // registry keeps — so the impl stays out and the line above is what
            // says it did.
            continue;
        }
        for method in &imp.methods {
            if method.is_test {
                continue;
            }
            let symbol = method_symbol(
                imp.trait_name().as_deref(),
                &imp.trait_type_args(),
                &method.ts_name,
                &imp.target_type,
            );
            let name = free_fn_name(reg, &self_ty, &imp.type_params, &symbol);
            // Two impls of two traits can write one method name for one self
            // type — `Get::get` and `Peek::get` on the same `Arc<Inner<T>>` —
            // and the scheme, which names the self type and the method, gives
            // both the same function. Adding the trait to the name would
            // separate them, but the name is computed here *and* at every call
            // site from the impl alone, so a rule that reads "unless another
            // impl took it first" cannot be computed at a call. The rule that
            // can — every trait impl's function carries its trait — renames
            // every module-level function the port emits, which is a decision
            // of its own. Until it is taken, the second impl is lost and the
            // site says so.
            let source = trait_source(imp);
            if out.iter().any(|f| f.name == name) {
                let earlier = written_for
                    .iter()
                    .find(|(taken, _)| *taken == name)
                    .map(|(_, source)| source.clone())
                    .unwrap_or_default();
                let why = if earlier.trim_start_matches('&').trim() == source.trim_start_matches('&').trim() {
                    "the two are written for the same type through a reference and without \
                     one, and emission erases the reference, so they are one function here"
                } else {
                    "the two impls differ in something the naming scheme does not write down"
                };
                sink.push(crate::diag::Diag::at(
                    &sink.file(),
                    imp.self_ty
                        .as_ref()
                        .map(|t| t.span())
                        .unwrap_or_else(proc_macro2::Span::call_site),
                    format!(
                        "`{}` and an earlier impl in this file both emit a module-level \
                         function called `{}`, so one of them is lost: {}",
                        method.name, name, why
                    ),
                ));
                continue;
            }
            written_for.push((name.clone(), source));
            out.push(FreeFn {
                name: name.clone(),
                text: write(reg, module, &self_ty, imp, method, &name),
            });
        }
    }
    out
}

/// The trait argument an impl was written with, as the source wrote it.
fn trait_source(imp: &ImplInfo) -> String {
    let Some(path) = &imp.trait_path else {
        return String::new();
    };
    let Some(segment) = path.segments.last() else {
        return String::new();
    };
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return String::new();
    };
    args.args
        .iter()
        .map(|a| quote::ToTokens::to_token_stream(a).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Does this impl's self type have an emitted class for its methods to hang
/// on? An impl the engine could not resolve a self type for keeps the
/// behaviour it had, which is to be written onto a class named by its target.
pub fn impl_has_class(reg: &TypeRegistry, module: ModuleId, imp: &ImplInfo) -> bool {
    let quiet = crate::diag::DiagSink::new();
    match resolved_self(reg, module, imp, &quiet) {
        Some(ty) => {
            class_module(reg, &ty) == Some(module)
                || is_reference_forwarding(&ty, &imp.type_params)
        }
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
    // An associated function has no receiver — `From::from(value)` takes only
    // its argument — so writing a `self` parameter for it declared a parameter
    // no call site passes and no body reads.
    let mut params = if method.is_static {
        Vec::new()
    } else {
        vec![format!("self: {}", crate::name_map::map_ty(reg, self_ty))]
    };
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
