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
            //
            // S8: asked of the WHOLE path. Keeping only the last segment and
            // resolving that bare name here answered "no" for every supertrait
            // written qualified — `trait Child: parent::Parent<u32>` came out
            // as `export interface Child {}` with the inherited method gone and
            // an unused import above it — because nothing brings `Parent` into
            // the child's own scope. The rendered name stays the last segment,
            // which is what the import writes.
            if !emitted_here(reg, here, &path_segments(&bound.path)) {
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
fn emitted_here(
    reg: &TypeRegistry,
    here: Option<crate::registry::ModuleId>,
    path: &[String],
) -> bool {
    let Some(here) = here else { return false };
    let Ok(Some(crate::registry::Def::Type(id))) = reg.lookup_type(here, path) else {
        return false;
    };
    reg.trait_def(id).is_some() && !reg.is_system(id)
}

/// A written path as the segments a lookup takes, generic arguments dropped.
fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments.iter().map(|s| s.ident.to_string()).collect()
}

/// What goes in the trait's own head, having written whatever has to stand
/// BEFORE it.
///
/// An interface says its supertraits directly; a trait with default bodies is
/// an abstract CLASS, which cannot extend an interface — so they are declared
/// on an interface of the same name, which TypeScript merges into the class
/// type. That gives `this.tell()` inside a default body a declaration without a
/// body, which is what the trait promised its implementors.
///
/// R13(f), the CONDITION this shape rests on, written down where it is relied
/// on: `export interface Retrieve extends GetEvents`, with `GetEvents` emitted
/// as an abstract class, is legal only while that class has no private or
/// protected member. TypeScript gives a class with one a nominal type an
/// interface cannot restate, and every implementor would then fail TS2420.
///
/// It is stated rather than checked because the condition cannot arise from
/// this emitter: a trait's members are its declared methods, `TraitMethod` is
/// `{ sig }` and carries no visibility at all, and the port writes no private
/// member on a trait's class. A check here would be reading a field that does
/// not exist. Whatever gives a trait's emitted class a private member is what
/// has to add the refusal, and this is the line that says so.
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

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    /// S8: a supertrait written QUALIFIED is still a supertrait.
    ///
    /// Keeping only the last segment and resolving that bare name in the
    /// child's module answered "no" for every one of them, so
    /// `trait Child: parent::Parent<u32>` came out as
    /// `export interface Child {}` — the inherited method gone, and an unused
    /// import written above it.
    #[test]
    fn a_qualified_generic_supertrait_is_written() {
        let mut f = Fixture::build(&[
            (
                "lib.rs",
                "pub mod parent;\n\
                 pub trait Child: parent::Parent<u32> { fn child(&self) -> u32; }\n",
            ),
            ("parent.rs", "pub trait Parent<T> { fn inherited(&self) -> T; }\n"),
        ]);
        let ts = f.emitted("lib.rs");
        assert!(
            ts.contains("export interface Child extends Parent<number>"),
            "the supertrait, with its argument substituted:\n{}",
            ts
        );
    }

    /// The same through a two-segment path, so the answer is not "one `::`".
    #[test]
    fn a_supertrait_two_modules_down_is_written() {
        let mut f = Fixture::build(&[
            (
                "lib.rs",
                "pub mod deep;\n\
                 pub trait Grand: deep::inner::Buried<u32> { fn grand(&self) -> u32; }\n",
            ),
            ("deep.rs", "pub mod inner;\n"),
            ("deep/inner.rs", "pub trait Buried<T> { fn buried(&self) -> T; }\n"),
        ]);
        let ts = f.emitted("lib.rs");
        assert!(
            ts.contains("export interface Grand extends Buried<number>"),
            "the rendered name is the last segment, which is what the import \
             writes:\n{}",
            ts
        );
    }

    /// And a bound the port emits nothing for is still not named. A marker
    /// carries no shape and naming one is a name nothing declares.
    #[test]
    fn a_supertrait_the_port_emits_nothing_for_is_still_left_out() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub trait Child: std::fmt::Debug { fn child(&self) -> u32; }\n",
        )]);
        let ts = f.emitted("lib.rs");
        assert!(
            !ts.contains("extends Debug"),
            "the declared surface's traits are declarations the port reads and \
             emits nothing for:\n{}",
            ts
        );
    }
}
