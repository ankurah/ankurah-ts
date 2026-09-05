//! Writing a call that lands on an impl with no class to be a method of.
//!
//! For: once such an impl is emitted as a module-level function, the call sites
//! have to name that function instead of writing a method call on the receiver.
//! The engine already resolved which impl the call lands on, so this only turns
//! that answer into the name and says whether the receiver is passed first.

use crate::registry::{Callee, ImplId, MethodResolution, TypeRegistry};
use crate::ty::Ty;

/// A call written as a module-level function taking its receiver first.
pub struct FreeCall {
    /// The function's name, which is also the symbol the calling module
    /// imports.
    pub name: String,
    /// The impl the call landed on, so a caller can say which one it wrote.
    pub impl_id: ImplId,
    /// True when the impl is written for a bare type parameter, so the engine
    /// picked it without knowing what the receiver will be at run time.
    pub is_blanket: bool,
}

/// The function a resolved call is written as, where the impl it landed on has
/// no class to be a method of.
///
/// A call that lands on a crate's own struct or enum is a method on the emitted
/// class and answers `None`; so does a call the engine resolved only to a
/// trait's declaration, which has no impl to name.
pub fn free_call(reg: &TypeRegistry, found: &MethodResolution) -> Option<FreeCall> {
    let (impl_id, method) = match &found.callee {
        Callee::Inherent(id, name) | Callee::TraitImpl(id, name) | Callee::Blanket(id, name) => {
            (*id, name)
        }
        Callee::TraitObject(..) => return None,
    };
    let def = reg.impl_def(impl_id);
    // Only an impl the corpus wrote becomes a function here. The declared std
    // surface's impls describe what the runtime already has — `Mutex::lock` is
    // a method on the runtime's `Mutex` — and the native-type table is what
    // writes those.
    if reg.modules().get(def.module).is_system {
        return None;
    }
    if !emits_as_free_function(reg, &def.self_ty, &def.generics, def.module) {
        return None;
    }
    let trait_name = def.trait_ref.as_ref().map(|t| leaf(reg.name_of(t.id)));
    // The Rust spelling, because that is the key R8's one decision is written
    // under; a name computed from the TypeScript spelling reached a function
    // nothing declares.
    let type_args: Vec<String> = def.trait_args_written.clone();
    let symbol = super::method_symbol(
        trait_name.as_deref(),
        &type_args,
        &crate::name_map::map_fn_name(method),
        &crate::name_map::map_ty(reg, &def.self_ty),
        def.self_ty.peel_refs().id(),
    );
    Some(FreeCall {
        name: super::free_fn_name(reg, &def.self_ty, &def.generics, &symbol),
        impl_id,
        is_blanket: def.is_blanket(),
    })
}

/// Is this impl written for a *reference* to one of its own parameters?
///
/// `impl<T: Signal> Signal for &T` exists in Rust because `&T` is a type of its
/// own and needs the trait spelled out for it again; every one of its methods
/// forwards to the same method on the `T` inside. Emission erases the
/// reference, so `&T` and `T` are one value here, the value already carries the
/// method, and the impl has nothing left to say. Emitting it would write a
/// function whose body calls itself.
pub fn is_reference_forwarding(self_ty: &Ty, generics: &[String]) -> bool {
    let Ty::Ref { inner, .. } = self_ty else {
        return false;
    };
    matches!(inner.as_ref(), Ty::Param(name) if generics.iter().any(|g| g == name))
}

/// Does this impl's body actually forward — every method calling the same
/// method on the value the reference points at?
///
/// The shape alone is not enough. `impl<T: Signal> Signal for &T` forwards, and
/// emitting it would write a function whose body calls itself; but an
/// `impl<T> Trait for &T` whose methods do something of their own is a real
/// impl, and dropping it left its call sites naming a function nothing
/// declares. Only a body that is one call to its own method name is skipped.
pub fn forwards_every_method(imp: &crate::types::ImplInfo) -> bool {
    imp.methods.iter().all(|method| {
        let Some(block) = &method.body_ast else {
            // A method that inherits the trait's default body writes nothing of
            // its own, so there is nothing here that would call itself.
            return true;
        };
        forwards(block, &method.name)
    })
}

/// Is this block one call to `name`, on whatever it was handed?
///
/// Both spellings count: `self.0.listen(l)` and the qualified
/// `Signal::listen(*self, l)` that the corpus writes to say which trait's
/// `listen` it means.
fn forwards(block: &syn::Block, name: &str) -> bool {
    let [syn::Stmt::Expr(expr, None)] = &block.stmts[..] else {
        return false;
    };
    let mut expr = expr;
    loop {
        match expr {
            syn::Expr::Paren(p) => expr = &p.expr,
            syn::Expr::Group(g) => expr = &g.expr,
            syn::Expr::MethodCall(call) => return call.method == name,
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path) = &*call.func else {
                    return false;
                };
                return path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == name);
            }
            _ => return false,
        }
    }
}

/// Does the port emit a class whose methods this impl's could be?
///
/// A struct or an enum the corpus declares does; a type parameter, a declared
/// system type and a type nothing declares do not.
pub fn has_emitted_class(reg: &TypeRegistry, self_ty: &Ty) -> bool {
    class_module(reg, self_ty).is_some()
}

/// Do this impl's methods become module-level FUNCTIONS rather than members of
/// a class?
///
/// This is the ONE question three different places used to answer differently.
/// A TypeScript class is one declaration in one file, and Rust lets an impl sit
/// anywhere in the crate, so an impl written away from its type's declaration
/// cannot add methods to that class — `impl Lineage for Clock` in
/// `core/src/lineage.rs` cannot put `members` on the `Clock` declared in
/// `retrieval.rs`. Asking only whether a class exists ANYWHERE — which is what
/// `has_emitted_class` asks — made the three disagree: the body was translated
/// with `this` as the receiver (so `Clock_members` read `this.asSlice()`, and
/// `this` is `undefined` in a module function), the wrapper was emitted with a
/// `self` parameter nothing read, and the call site went on writing
/// `subject.members()` against a class with no such method.
///
/// `impl_module` is the module the IMPL is written in, which is where its free
/// function is emitted — not the module of the call site.
pub fn emits_as_free_function(
    reg: &TypeRegistry,
    self_ty: &Ty,
    generics: &[String],
    impl_module: crate::registry::ModuleId,
) -> bool {
    // An impl for a reference to its own parameter forwards to the value
    // inside, and emission erases the reference, so it has nothing of its own
    // to emit anywhere.
    if is_reference_forwarding(self_ty, generics) {
        return false;
    }
    match class_module(reg, self_ty) {
        // The class is written in this very file: the methods join it.
        Some(home) if home == impl_module => false,
        // A type whose TypeScript is written by hand carries its own methods;
        // emitting them again beside it would give the port two of each.
        Some(_) if self_ty.peel_refs().id().is_some_and(|id| reg.is_hand_written(id)) => false,
        // A class elsewhere, or no class at all: module-level functions.
        _ => true,
    }
}

/// Where the class an impl's methods would join is written, if there is one.
///
/// Rust lets an impl sit anywhere in the crate; a TypeScript class is one
/// declaration in one file. An impl written beside its type becomes methods on
/// that class, and an impl written elsewhere cannot — `impl TryFrom<Expr> for
/// Predicate` lives in ankql's conversion.rs while `Predicate` is declared in
/// ast.rs, and all five of that file's conversions used to be emitted nowhere
/// at all: conversion.ts was a header and a blank line.
pub fn class_module(reg: &TypeRegistry, self_ty: &Ty) -> Option<crate::registry::ModuleId> {
    let id = self_ty.peel_refs().id()?;
    if id.is_foreign() {
        return None;
    }
    let def = reg.def(id)?;
    // A declared system type is the runtime's, not this crate's: `Arc` has no
    // emitted class here whatever methods an impl adds to it.
    if reg.is_system(id) {
        return None;
    }
    match def.kind {
        crate::registry::TypeKind::Struct | crate::registry::TypeKind::Enum { .. } => {
            Some(def.module)
        }
        _ => None,
    }
}

/// The last segment of a module-qualified name.
fn leaf(name: String) -> String {
    name.rsplit("::").next().unwrap_or(&name).to_string()
}
