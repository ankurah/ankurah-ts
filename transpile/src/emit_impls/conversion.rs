//! Writing the call that performs a `From` or a `TryFrom` conversion.
//!
//! For: `?` across two error types, and every `.into()`, calls a conversion the
//! engine picked out of the impl table. This turns that impl into the text the
//! call site writes, and — where it cannot be written — into the sentence that
//! says why.
//!
//! The name has to be the one emission gave the method, so it is computed by
//! the same function `emit.rs` names the method with rather than by a second
//! rule that could drift from it.

use super::dispatch::has_emitted_class;
use crate::registry::{ImplId, TypeRegistry};
use crate::ty::Ty;

/// Everything before the `(` of a conversion call: `MutationError.fromStateError`,
/// or the module-level function an impl with no class of its own is emitted as.
pub struct ConversionCall {
    pub callee: String,
}

/// The call a resolved conversion impl is written as, or the reason it cannot
/// be written.
///
/// `target` is the concrete type being converted to, which is what names the
/// class; the impl names the method, because the method's name is disambiguated
/// by the source type the impl was *written* for and not by whatever the call
/// site turned out to hold.
pub fn conversion_call(
    reg: &TypeRegistry,
    id: ImplId,
    target: &Ty,
) -> Result<ConversionCall, String> {
    let def = reg.impl_def(id);
    let Some(implemented) = def.trait_ref.as_ref() else {
        return Err("the impl performing it names no trait".to_string());
    };
    let Some(source) = implemented.args.first() else {
        return Err("the impl performing it takes no source type".to_string());
    };
    let trait_name = leaf(&reg.name_of(implemented.id));
    // The impl's trait argument *as written*, which is what named the emitted
    // method. Resolving it instead expands an alias — `bincode::Error` is
    // `Box<ErrorKind>` — and the call then named `fromErrorKind` on a class
    // that declares `fromError`. The written spelling is one rule for both.
    let source_ts = match def.trait_args_written.first() {
        Some(written) => written.clone(),
        None => crate::name_map::map_ty(reg, source),
    };
    // `From<Infallible>` is a conversion from a type that cannot be
    // constructed, so emission leaves the method out; a call to it would name
    // nothing.
    if source_ts == "never" || source_ts == "Infallible" {
        return Err(format!(
            "the conversion is `From<{}>`, which emission leaves out because the source \
             type cannot be constructed",
            source_ts
        ));
    }
    let base = match trait_name.as_str() {
        "From" => "from",
        "TryFrom" => "tryFrom",
        other => return Err(format!("`{}` is not a conversion trait", other)),
    };
    // The name the class DECLARED for this impl, read from the one decision
    // rather than computed a second time. Passing an empty self type here — as
    // this call and the two below used to — meant the call site never saw a
    // contest, so it wrote `fromError` against a class declaring
    // `fromBincodeError`.
    let method = crate::emit_impls::conversion_name_of_impl(reg, id).unwrap_or_else(|| {
        crate::emit::disambiguate_trait_method(
            base,
            &trait_name,
            &[source_ts.clone()],
            "",
            def.self_ty.peel_refs().id(),
        )
    });

    // An impl the declared surface wrote describes a conversion the runtime is
    // supposed to already have. `@ankurah/base` has no `anyhow::Error` and no
    // `String.from`, so there is nothing for the call to name; a site that
    // lands on one is reported rather than given a name nothing exports.
    if reg.modules().get(def.module).is_system {
        return Err(format!(
            "the impl performing it is the declared surface's, so the conversion is the \
             runtime's own, and the port has no `{}` for it to be a member of",
            reg.describe(target)
        ));
    }
    if let Some(class) = class_of(reg, target) {
        return Ok(ConversionCall {
            callee: format!("{}.{}", class, method),
        });
    }
    if has_emitted_class(reg, &def.self_ty) {
        return Err(format!(
            "`{}` is not a type with a class of its own, so `{}` has nothing to be a static \
             member of",
            reg.describe(target),
            method
        ));
    }
    Ok(ConversionCall {
        callee: super::free_fn_name(reg, &def.self_ty, &def.generics, &method),
    })
}

/// Every method name the `From` impls for one target would be emitted under.
///
/// Emission hangs them all on one class and keeps the first of any two that
/// agree, so a name that appears twice here is a name a call cannot be trusted
/// to reach.
pub fn conversion_names(reg: &TypeRegistry, target: &Ty, trait_path: &str) -> Vec<String> {
    let Some(trait_id) = reg.system_type(trait_path) else {
        return Vec::new();
    };
    let Some(target_id) = target.peel_refs().id() else {
        return Vec::new();
    };
    let trait_name = leaf(&reg.name_of(trait_id));
    let base = if trait_name == "TryFrom" { "tryFrom" } else { "from" };
    reg.impls()
        .of_trait(trait_id)
        .iter()
        .filter_map(|&id| {
            let def = reg.impl_def(id);
            if reg.modules().get(def.module).is_system {
                return None;
            }
            if def.self_ty.peel_refs().id() != Some(target_id) {
                return None;
            }
            let source = def.trait_ref.as_ref()?.args.first()?;
            // The written spelling, which is what names the emitted method —
            // the same rule `conversion_call` uses. Reading the resolved type
            // here and the written one there made this list disagree with the
            // names it is checking for collisions.
            let source_ts = match def.trait_args_written.first() {
                Some(written) => written.clone(),
                None => crate::name_map::map_ty(reg, source),
            };
            if source_ts == "never" || source_ts == "Infallible" {
                return None;
            }
            Some(crate::emit::disambiguate_trait_method(
                base,
                &trait_name,
                &[source_ts],
                "",
                Some(target_id),
            ))
        })
        .collect()
}

/// The class a static conversion is called on, where the target has one.
fn class_of(reg: &TypeRegistry, ty: &Ty) -> Option<String> {
    if !has_emitted_class(reg, ty) {
        return None;
    }
    let id = ty.peel_refs().id()?;
    Some(crate::name_map::map_ty(
        reg,
        &Ty::Named {
            id,
            args: Vec::new(),
        },
    ))
}

/// The last segment of a module-qualified name.
fn leaf(name: &str) -> String {
    name.rsplit("::").next().unwrap_or(name).to_string()
}
