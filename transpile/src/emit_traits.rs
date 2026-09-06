//! What a trait declaration passes on to its implementors.
//!
//! Split out of `emit.rs`, which is long enough already.

use crate::registry::TypeRegistry;
use crate::types::TraitInfo;

/// The supertraits a trait declaration passes on to its implementors, in the
/// spelling emission writes.
///
/// I8: a supertrait is part of what an implementor promises — `trait Super:
/// Tell` means every `Super` is a `Tell` — and the emitted interface said
/// nothing, so `fn f<T: Super>(t: &T) { t.tell() }` resolved through the bound
/// and then `tsc` reported TS2339 on `T`. A marker carries no shape and a
/// derive-provided trait has no interface of its own, so neither is written.
pub(crate) fn supertrait_surface(
    reg: &TypeRegistry,
    here: Option<crate::registry::ModuleId>,
    t: &TraitInfo,
) -> String {
    let written: Vec<String> = t
        .supertraits
        .iter()
        .filter_map(|bound| {
            let seg = bound.path.segments.last()?;
            let name = seg.ident.to_string();
            // Only a trait the port EMITS as a type of its own: a marker
            // carries no shape, a derive-provided trait has no interface, and
            // the declared surface's traits — `futures::Stream` is the one the
            // corpus writes — are declarations the port reads and emits
            // nothing for. Naming one is a name nothing declares.
            if !emitted_here(reg, here, &name) {
                return None;
            }
            let args: Vec<String> = match &seg.arguments {
                syn::PathArguments::AngleBracketed(args) => args
                    .args
                    .iter()
                    .filter_map(|a| match a {
                        syn::GenericArgument::Type(ty) => Some(crate::name_map::map_type(ty)),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            Some(match args.is_empty() {
                true => name,
                false => format!("{}<{}>", name, args.join(", ")),
            })
        })
        .collect();
    match written.is_empty() {
        true => String::new(),
        false => format!(" extends {}", written.join(", ")),
    }
}

/// Does this crate — or another the port emits — declare a trait of this name?
///
/// The declared std surface is read into the same registry, so the question is
/// not "is it there" but "is it something the emission writes a type for".
fn emitted_here(reg: &TypeRegistry, here: Option<crate::registry::ModuleId>, name: &str) -> bool {
    let Some(here) = here else { return false };
    let Ok(Some(crate::registry::Def::Type(id))) =
        reg.lookup_type(here, &[name.to_string()])
    else {
        return false;
    };
    reg.trait_def(id).is_some() && !reg.is_system(id)
}

/// What goes in the trait's own head, having written whatever has to stand
/// BEFORE it.
///
/// An interface says its supertraits directly; a trait with default bodies is
/// an abstract CLASS, which cannot extend an interface — so they are declared
/// on an interface of the same name, which TypeScript merges into the class
/// type. That gives `this.tell()` inside a default body a declaration without a
/// body, which is what the trait promised its implementors.
pub(crate) fn declare_supertraits(
    out: &mut String,
    reg: &TypeRegistry,
    here: Option<crate::registry::ModuleId>,
    t: &TraitInfo,
    export: &str,
) -> String {
    let supers = supertrait_surface(reg, here, t);
    if supers.is_empty() || !t.has_default_impls {
        return supers;
    }
    out.push_str(&format!("{}interface {}{}{} {{}}\n\n", export, t.name, t.generics, supers));
    String::new()
}
