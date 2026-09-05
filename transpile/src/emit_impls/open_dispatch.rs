//! Calling a trait method when the receiver's type is only known at run time.
//!
//! For: `Ref::listen<L: IntoBroadcastListener<T>>` calls
//! `listener.into_broadcast_listener()`, and `L` is open. At run time the
//! listener may be a closure, an `Arc` holding one, or a type the crate
//! declared, and Rust picks the impl per instantiation where one emitted body
//! cannot. Writing the blanket impl's function there reached the wrong impl for
//! every receiver the blanket was not for.
//!
//! So the port writes one function per trait method that selects the impl the
//! way the run time can: by the shape of the receiver. Each impl of the trait
//! contributes a test — `instanceof` for a class, `typeof === 'function'` for
//! the closure blanket — and the branch calls that impl's own function. A
//! receiver matching none is unreachable if rustc compiled the crate, so it is
//! fatal rather than silent.
//!
//! Two impls the run time cannot tell apart mean no dispatcher can be written
//! at all, and the site says so rather than choosing one of them.

use std::collections::HashMap;

use super::dispatch::has_emitted_class;
use crate::name_map::shape::{js_shape, JsShape};
use crate::registry::{ImplId, ModuleId, TypeRegistry};
use crate::ty::{Ty, TypeId};

/// One generated dispatcher.
pub struct Dispatcher {
    pub name: String,
    pub text: String,
}

/// The dispatcher for one trait method, named the same way wherever it is
/// asked for.
///
/// The name carries a STABLE trait identity, not the leaf alone: two traits of
/// one leaf name — a `convert::Convert::into` beside a `wire::Convert::into` —
/// wrote one function under one name, and the second silently replaced the
/// first. The qualifier is the segment in front of the leaf, and it is written
/// only where the leaf is contested, so an uncontested dispatcher keeps the
/// short name every call site already reads.
pub fn dispatcher_name(trait_name: &str, ts_method: &str) -> String {
    format!("{}_dispatch_{}", trait_identity(trait_name), ts_method)
}

/// A trait's leaf, qualified by the module in front of it where the leaf is
/// contested.
fn trait_identity(trait_name: &str) -> String {
    let leaf = leaf(trait_name);
    if !contested::holds(&leaf) {
        return leaf;
    }
    let segments: Vec<&str> = trait_name.split("::").filter(|s| !s.is_empty()).collect();
    match segments.len() {
        0 | 1 => leaf,
        n => {
            let qualifier = segments[n - 2];
            // A qualifier that says nothing the leaf does not, or that is only
            // a position in the crate, is left out — the same rule a contested
            // conversion static takes.
            if matches!(qualifier, "crate" | "self" | "super" | "std" | "core")
                || leaf.to_lowercase().starts_with(&qualifier.to_lowercase())
            {
                leaf
            } else {
                format!("{}{}", capitalised(qualifier), leaf)
            }
        }
    }
}

fn capitalised(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Which trait leaves more than one declared trait takes.
///
/// Filled once per run from the whole trait table, because the answer is a fact
/// about a trait's SIBLINGS and neither the call site nor the emitter can see
/// them from the trait in its hand.
mod contested {
    use std::cell::RefCell;
    use std::collections::BTreeSet;

    thread_local! {
        static CONTESTED: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
    }

    #[cfg_attr(test, allow(dead_code))]
    pub fn set(leaves: BTreeSet<String>) {
        CONTESTED.with(|c| *c.borrow_mut() = leaves);
    }

    pub fn holds(leaf: &str) -> bool {
        CONTESTED.with(|c| c.borrow().contains(leaf))
    }
}

/// Record which trait leaf names two declared traits would both take.
pub fn set_contested_traits(reg: &TypeRegistry) {
    let mut seen: std::collections::BTreeMap<String, usize> = Default::default();
    for id in reg.trait_ids() {
        *seen.entry(leaf(&reg.name_of(id))).or_default() += 1;
    }
    contested::set(seen.into_iter().filter(|(_, n)| *n > 1).map(|(leaf, _)| leaf).collect());
}

/// The trait methods some call site asked a dispatcher for.
///
/// A dispatcher is written only where a call needs one. Emitting one for every
/// trait with two impls filled `signals` with functions nobody calls, whose
/// signatures name the trait's own type parameters with nothing to bind them.
/// The record is filled while bodies are translated and read when the files are
/// written, which is the order the pipeline already runs in.
mod wanted {
    use std::cell::RefCell;
    use std::collections::BTreeSet;

    use crate::ty::TypeId;

    thread_local! {
        static WANTED: RefCell<BTreeSet<(TypeId, String)>> =
            const { RefCell::new(BTreeSet::new()) };
    }

    pub fn record(trait_id: TypeId, method: &str) {
        WANTED.with(|w| w.borrow_mut().insert((trait_id, method.to_string())));
    }

    pub fn asked(trait_id: TypeId, method: &str) -> bool {
        WANTED.with(|w| w.borrow().contains(&(trait_id, method.to_string())))
    }
}

pub use wanted::record as record_wanted;

/// Every dispatcher the traits this file declares were asked for.
pub fn dispatchers(reg: &TypeRegistry, module: ModuleId, file: &crate::types::RustFile) -> Vec<Dispatcher> {
    let mut out = Vec::new();
    for declared in &file.traits {
        let Some(id) = reg.module_type(module, &declared.name) else {
            continue;
        };
        let Some(def) = reg.trait_def(id) else { continue };
        let mut methods: Vec<&String> = def.methods.keys().collect();
        methods.sort();
        for method in methods {
            if !wanted::asked(id, method) {
                continue;
            }
            if let Ok(one) = write(reg, id, &declared.name, method) {
                out.push(one);
            }
        }
    }
    out
}

/// Can a dispatcher be written for this trait method, and what stops it?
pub fn refusal(reg: &TypeRegistry, trait_id: TypeId, trait_name: &str, method: &str) -> Option<String> {
    write(reg, trait_id, trait_name, method).err()
}

fn write(
    reg: &TypeRegistry,
    trait_id: TypeId,
    trait_name: &str,
    method: &str,
) -> Result<Dispatcher, String> {
    let def = reg.trait_def(trait_id).ok_or("the trait is not declared")?;
    let sig = &def
        .methods
        .get(method)
        .ok_or_else(|| format!("`{}` declares no `{}`", trait_name, method))?
        .sig;
    let ts_method = crate::name_map::map_fn_name(method);
    let impls = reg.impls().of_trait(trait_id);
    if impls.len() < 2 {
        return Err("the trait has fewer than two impls, so nothing has to be chosen".to_string());
    }
    let mut branches: Vec<(String, String)> = Vec::new();
    let mut catch_all: Option<String> = None;
    let mut seen: HashMap<String, ImplId> = HashMap::new();
    // ONE impl the engine cannot write must not cost the whole dispatcher.
    // Every call through the bound went to the fatal at the end when it did:
    // the impls that CAN be told apart are the ones the dispatcher is for, and
    // the one that cannot is reported at the site and left out of it.
    for &id in impls {
        let test = match shape_test(reg, id) {
            Ok(test) => test,
            Err(why) => {
                crate::diag::pending::park_at(
                    0,
                    0,
                    format!(
                        "`{}` for `{}` has no run-time test the dispatcher can make, because \
                         {}; the branch is left out and a receiver of that shape reaches the \
                         dispatcher's own fatal",
                        leaf(trait_name),
                        reg.describe(&reg.impl_def(id).self_ty),
                        why
                    ),
                );
                continue;
            }
        };
        let Some(test) = test else {
            // An impl written for a bare parameter with no bound the run time
            // can see applies to whatever the others do not, so it is the last
            // branch rather than a test. A second one has nothing to tell it
            // from the first, so it is reported and left out.
            if catch_all.is_some() {
                crate::diag::pending::park_at(
                    0,
                    0,
                    format!(
                        "`{}` for `{}` is written for anything at all, and so is an impl \
                         before it, so nothing chooses between them; this one is left out",
                        leaf(trait_name),
                        reg.describe(&reg.impl_def(id).self_ty),
                    ),
                );
                continue;
            }
            catch_all = Some(call(reg, id, &ts_method, sig));
            continue;
        };
        if let Some(other) = seen.insert(test.clone(), id) {
            // Two impls of one shape: the first one written wins, the way
            // Rust's own coherence would have refused the pair outright.
            crate::diag::pending::park_at(
                0,
                0,
                format!(
                    "`{}` for `{}` and for `{}` are the same shape at run time, so no test \
                     tells them apart; the first is what the dispatcher calls",
                    leaf(trait_name),
                    reg.describe(&reg.impl_def(other).self_ty),
                    reg.describe(&reg.impl_def(id).self_ty),
                ),
            );
            continue;
        }
        branches.push((test, call(reg, id, &ts_method, sig)));
    }
    if branches.len() + usize::from(catch_all.is_some()) < 2 {
        return Err(
            "fewer than two impls have a run-time test, so nothing has to be chosen".to_string(),
        );
    }
    // A strict refinement is tried before the test it refines: `self instanceof
    // Arc && <arity>` is a narrowing of `self instanceof Arc`, and written
    // after it the narrower impl was never reached. Rust picks the more
    // specific impl; this is that order written out.
    branches.sort_by(|(a, _), (b, _)| {
        let refines = |narrow: &str, wide: &str| narrow.len() > wide.len() && narrow.starts_with(wide);
        if refines(a, b) {
            std::cmp::Ordering::Less
        } else if refines(b, a) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    let params: Vec<String> = sig
        .params
        .iter()
        .filter(|(name, _)| name != "self")
        .map(|(name, ty)| {
            format!(
                "{}: {}",
                crate::name_map::to_camel_case(name),
                crate::name_map::map_ty(reg, ty)
            )
        })
        .collect();
    let ret = crate::name_map::map_ty(reg, &sig.ret);
    // The trait's own parameters and the method's are written in the signature,
    // so the function declares them: `Get::get(&self) -> Self::Target` on a
    // `trait Get<T>` names a `T` that nothing else here binds.
    let mut generics: Vec<String> = def.generics.clone();
    for extra in &sig.type_params {
        if !generics.contains(extra) {
            generics.push(extra.clone());
        }
    }
    let declared = if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    };
    // A closure the port had to own is an `OwnedClosure`, not a function, and
    // an `Arc` holding one answers `typeof 'object'`: a plain `.length` test
    // never matched it. `OwnedClosure.$arity` is the runtime's answer to that —
    // the arity of the function it holds, read through the same liveness check
    // every other read goes through — so an `Arc<dyn Fn(T)>` and an
    // `Arc<dyn Fn()>` are told apart whichever shape the value has.
    let name = dispatcher_name(trait_name, &ts_method);
    let mut body = String::new();
    for (test, call) in &branches {
        body.push_str(&format!("  if ({}) return {};\n", test, call));
    }
    match &catch_all {
        Some(call) => body.push_str(&format!("  return {};\n", call)),
        None => body.push_str(&format!(
            "  throw new Error(`BUG: no {} impl for ${{(self as object)?.constructor?.name ?? \
             typeof self}}`);\n",
            leaf(trait_name)
        )),
    }
    let mut written = vec!["self: unknown".to_string()];
    written.extend(params);
    Ok(Dispatcher {
        text: format!(
            "export function {}{}({}): {} {{\n{}}}\n\n",
            name,
            declared,
            written.join(", "),
            ret,
            body
        ),
        name,
    })
}

/// The run-time test that says a receiver is the one this impl is written for.
///
/// `Ok(None)` is an impl written for anything at all, which needs no test and
/// stands last.
fn shape_test(reg: &TypeRegistry, id: ImplId) -> Result<Option<String>, String> {
    let def = reg.impl_def(id);
    let describe = || reg.describe(&def.self_ty);
    match def.self_ty.peel_refs() {
        // A blanket impl is written for whatever satisfies its bound, and the
        // only bound the run time can see is a callable one: a closure is a
        // function, and a closure that owns something is an `OwnedClosure`.
        Ty::Param(name) if def.generics.iter().any(|g| g == name) => {
            let callable = def.bounds.iter().any(|b| {
                matches!(&b.subject, Ty::Param(subject) if subject == name)
                    && matches!(
                        leaf(&reg.name_of(b.trait_ref.id)).as_str(),
                        "Fn" | "FnMut" | "FnOnce"
                    )
            });
            if !callable {
                // `impl<T> Iterable<T> for T` applies to whatever the trait's
                // other impls do not. Rust tells it from those by the *item*
                // type, which emission erases, so what is left is the shape of
                // the receiver: the impls with a shape are tested, and this one
                // takes everything else. Spec 7a records what that misses.
                return Ok(None);
            }
            Ok(Some(
                "typeof self === 'function' || self instanceof OwnedClosure".to_string(),
            ))
        }
        ty => match class_test(reg, ty) {
            Some(test) => Ok(Some(test)),
            None => Err(format!(
                "the impl for `{}` is written for a type the port has no class to test against",
                describe()
            )),
        },
    }
}

/// The `instanceof` test that says a receiver is of this type.
fn class_test(reg: &TypeRegistry, ty: &Ty) -> Option<String> {
    let id = ty.peel_refs().id()?;
    let bare = crate::name_map::map_ty(
        reg,
        &Ty::Named {
            id,
            args: Vec::new(),
        },
    );
    if has_emitted_class(reg, ty) {
        return Some(format!("self instanceof {}", bare));
    }
    // A reference-counted wrapper is a class the runtime exports, and two impls
    // written for one differ only in what it holds: `Arc<dyn Fn(T)>` and
    // `Arc<dyn Fn()>` are two impls of one trait, and the closure they hold is
    // the only thing that tells them apart at run time. A JavaScript function
    // carries how many arguments it declares, which is what the two differ by.
    // The shapes JavaScript tests directly.
    match js_shape(reg, ty) {
        JsShape::Array(_) => return Some("Array.isArray(self)".to_string()),
        JsShape::Bytes => return Some("self instanceof Uint8Array".to_string()),
        JsShape::Set(_) => return Some("self instanceof Set".to_string()),
        JsShape::Map(_, _) => return Some("self instanceof Map".to_string()),
        JsShape::Str => return Some("typeof self === 'string'".to_string()),
        JsShape::Boolean => return Some("typeof self === 'boolean'".to_string()),
        JsShape::Number => return Some("typeof self === 'number'".to_string()),
        JsShape::BigInt => return Some("typeof self === 'bigint'".to_string()),
        _ => {}
    }
    if let JsShape::Rc(name) = js_shape(reg, ty) {
        let mut test = format!("self instanceof {}", name);
        if let Some(arity) = callable_arity(reg, ty) {
            // Either shape a callable takes here: a plain function, or the
            // `OwnedClosure` a closure that had to own its captures became.
            let function_form = format!("typeof self.value === 'function' && self.value.length === {}", arity);
            let closure_form = format!("self.value instanceof OwnedClosure && self.value.$arity === {}", arity);
            test.push_str(&format!(" && (({}) || ({}))", function_form, closure_form));
        }
        return Some(test);
    }
    // A declared system type is a class the runtime exports under the name the
    // port writes it by, which is the same name the emitted signatures use.
    if reg.is_system(id) && bare.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Some(format!("self instanceof {}", bare));
    }
    None
}

/// How many arguments the callable a wrapper holds declares, where it holds
/// one.
fn callable_arity(reg: &TypeRegistry, ty: &Ty) -> Option<usize> {
    let Ty::Named { args, .. } = ty.peel_refs() else {
        return None;
    };
    let traits = match args.first()? {
        Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => traits,
        _ => return None,
    };
    let callable = traits.iter().find(|t| {
        matches!(
            leaf(&reg.name_of(t.id)).as_str(),
            "Fn" | "FnMut" | "FnOnce"
        )
    })?;
    Some(match callable.args.first() {
        Some(Ty::Tuple(inputs)) => inputs.len(),
        Some(Ty::Unit) | None => 0,
        Some(_) => 1,
    })
}

/// What the branch calls once the shape has chosen the impl.
fn call(reg: &TypeRegistry, id: ImplId, ts_method: &str, sig: &crate::registry::MethodSig) -> String {
    let def = reg.impl_def(id);
    let args: Vec<String> = sig
        .params
        .iter()
        .filter(|(name, _)| name != "self")
        .map(|(name, _)| crate::name_map::to_camel_case(name))
        .collect();
    if has_emitted_class(reg, &def.self_ty) {
        let mut written = vec![format!("(self as any).{}", ts_method)];
        written.push(format!("({})", args.join(", ")));
        return written.join("");
    }
    let mut written = vec!["self as any".to_string()];
    written.extend(args);
    format!(
        "{}({})",
        super::free_fn_name(reg, &def.self_ty, &def.generics, ts_method),
        written.join(", ")
    )
}

fn leaf(name: &str) -> String {
    name.rsplit("::").next().unwrap_or(name).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Two traits of one leaf name wrote one dispatcher under one name, and the
    /// second silently replaced the first. The name carries the module in front
    /// of the leaf where the leaf is contested, and nothing where it is not.
    #[test]
    fn a_contested_trait_leaf_takes_its_module_into_the_name() {
        contested::set(BTreeSet::from(["Convert".to_string()]));
        assert_eq!(
            dispatcher_name("crate::wire::Convert", "into"),
            "WireConvert_dispatch_into"
        );
        // A qualifier that is only a position in the crate says nothing.
        assert_eq!(dispatcher_name("crate::Convert", "into"), "Convert_dispatch_into");
        contested::set(BTreeSet::new());
        assert_eq!(
            dispatcher_name("crate::wire::Convert", "into"),
            "Convert_dispatch_into"
        );
    }
}
