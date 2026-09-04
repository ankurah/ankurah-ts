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
    if has_emitted_class(reg, &def.self_ty) || is_reference_forwarding(&def.self_ty, &def.generics)
    {
        return None;
    }
    let trait_name = def.trait_ref.as_ref().map(|t| leaf(reg.name_of(t.id)));
    let type_args: Vec<String> = def
        .trait_ref
        .as_ref()
        .map(|t| t.args.iter().map(|ty| crate::name_map::map_ty(reg, ty)).collect())
        .unwrap_or_default();
    let symbol = super::method_symbol(
        trait_name.as_deref(),
        &type_args,
        &crate::name_map::map_fn_name(method),
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
    let Some(id) = self_ty.peel_refs().id() else {
        return false;
    };
    if id.is_foreign() {
        return false;
    }
    let Some(def) = reg.def(id) else {
        return false;
    };
    // A declared system type is the runtime's, not this crate's: `Arc` has no
    // emitted class here whatever methods an impl adds to it.
    if reg.is_system(id) {
        return false;
    }
    matches!(
        def.kind,
        crate::registry::TypeKind::Struct | crate::registry::TypeKind::Enum { .. }
    )
}

/// The last segment of a module-qualified name.
fn leaf(name: String) -> String {
    name.rsplit("::").next().unwrap_or(&name).to_string()
}
