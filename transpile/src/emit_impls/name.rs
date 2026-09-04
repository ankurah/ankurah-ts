//! What a module-level function standing for an impl method is called.
//!
//! For: the emitter writes the function and the call sites write its name, and
//! the two are computed in different places from different starting points. One
//! function here answers for both, so a rename cannot land on one side only.
//!
//! The scheme is in this module's parent doc comment. In short: the self type's
//! constructors from the outside in, joined by `_`, then the method's
//! TypeScript name; a blanket impl, which has no constructor, takes the method
//! name alone.

use crate::registry::TypeRegistry;
use crate::ty::{TraitRef, Ty};

/// The method name a free function carries, disambiguated the way emission
/// disambiguates a method on a class.
///
/// `impl From<Clock> for Vec<EventId>` and `impl From<EventId> for
/// Vec<EventId>` are two impls of one trait for one self type, and the self
/// type and the method name are the same in both: without the trait's argument
/// they take one name and one of them is lost. On a class the two are `fromClock`
/// and `fromEventId`; here they are the same, because it is the same question.
pub fn method_symbol(trait_name: Option<&str>, type_args: &[String], ts_method: &str) -> String {
    match trait_name {
        // `Into` and `TryInto` name their *target*, not their source, so the
        // scheme's `from<Arg>` would read backwards on a function whose
        // receiver is the thing being converted. Their method name already says
        // which direction it goes.
        Some("Into") | Some("TryInto") | None => ts_method.to_string(),
        Some(trait_name) => crate::emit::impl_method_name(trait_name, "", ts_method, type_args),
    }
}

/// The name of the function one impl method is emitted as.
pub fn free_fn_name(
    reg: &TypeRegistry,
    self_ty: &Ty,
    generics: &[String],
    ts_method: &str,
) -> String {
    match self_shape(reg, self_ty, generics) {
        Some(shape) => format!("{}_{}", shape, ts_method),
        None => ts_method.to_string(),
    }
}

/// The impl's self type written as an identifier fragment, or `None` where it
/// is a bare parameter of the impl — a blanket impl, which has no type
/// constructor to name.
pub fn self_shape(reg: &TypeRegistry, ty: &Ty, generics: &[String]) -> Option<String> {
    match ty {
        // `impl<F: Fn(T)> Trait for F`: the impl is written for its own
        // parameter, so there is no constructor and the method name stands
        // alone.
        Ty::Param(name) if generics.iter().any(|g| g == name) => None,
        _ => Some(shape(reg, ty, generics)),
    }
}

fn shape(reg: &TypeRegistry, ty: &Ty, generics: &[String]) -> String {
    match ty {
        Ty::Named { id, args } => {
            let head = leaf(reg.name_of(*id));
            let inner: Vec<String> = args
                .iter()
                .filter(|arg| !is_parameter(arg, generics))
                .map(|arg| shape(reg, arg, generics))
                .collect();
            once_with(head, inner)
        }
        // `Arc<dyn Fn(T)>` and `Arc<dyn Fn()>` are two impls that differ in
        // nothing a name would otherwise catch, so a callable bound carries how
        // many arguments it takes.
        Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => {
            // `Send` and `Sync` hold or do not hold by the type's own shape and
            // never tell two impls apart, so they are left out of the name.
            let parts: Vec<String> = traits
                .iter()
                .filter(|t| !reg.trait_def(t.id).is_some_and(|d| d.is_auto))
                .map(|t| bound_shape(reg, t))
                .collect();
            parts.join("_")
        }
        // A reference is erased in emission and says nothing about which impl
        // this is.
        Ty::Ref { inner, .. } => shape(reg, inner, generics),
        Ty::Slice(elem) => format!("Slice_{}", shape(reg, elem, generics)),
        Ty::Array { elem, .. } => format!("Array_{}", shape(reg, elem, generics)),
        Ty::Tuple(elems) => {
            let parts: Vec<String> = elems.iter().map(|e| shape(reg, e, generics)).collect();
            once_with("Tuple".to_string(), parts)
        }
        Ty::Prim(prim) => {
            let mut name = format!("{:?}", prim);
            name.get_mut(0..1).map(|c| {
                c.make_ascii_uppercase();
            });
            name
        }
        Ty::Str => "Str".to_string(),
        Ty::Unit => "Unit".to_string(),
        Ty::Never => "Never".to_string(),
        Ty::Param(name) => name.clone(),
        Ty::Infer => "Infer".to_string(),
        Ty::Assoc { base, name, .. } => {
            format!("{}_{}", shape(reg, base, generics), name)
        }
    }
}

/// A bound in a trait object: the trait's leaf name, and for a callable the
/// number of arguments it takes.
fn bound_shape(reg: &TypeRegistry, bound: &TraitRef) -> String {
    let name = leaf(reg.name_of(bound.id));
    let callable = matches!(name.as_str(), "Fn" | "FnMut" | "FnOnce");
    if !callable {
        return name;
    }
    let arity = match bound.args.first() {
        Some(Ty::Tuple(inputs)) => inputs.len(),
        Some(Ty::Unit) | None => 0,
        Some(_) => 1,
    };
    format!("{}{}", name, arity)
}

fn once_with(head: String, inner: Vec<String>) -> String {
    if inner.is_empty() {
        head
    } else {
        format!("{}_{}", head, inner.join("_"))
    }
}

/// Is this argument one of the impl's own parameters? Those say nothing about
/// which impl the name stands for — `Arc<Inner<T>>` is `Arc_Inner` whatever `T`
/// turns out to be.
fn is_parameter(ty: &Ty, generics: &[String]) -> bool {
    matches!(ty, Ty::Param(name) if generics.iter().any(|g| g == name))
}

/// The last segment of a module-qualified name, and nothing but identifier
/// characters, so the result is a legal JavaScript identifier fragment.
fn leaf(name: String) -> String {
    let last = name.rsplit("::").next().unwrap_or(&name).to_string();
    last.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}
