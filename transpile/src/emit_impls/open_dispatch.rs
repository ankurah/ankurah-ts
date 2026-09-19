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

use super::shape_tests::{call, shape_test};
use crate::registry::{ImplId, ModuleId, TypeRegistry};
use crate::ty::TypeId;

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

/// The function a call through an OPEN BOUND has to name, where the trait's
/// impls are emitted as module-level functions rather than as methods.
///
/// `subject.members()` where `subject: C, C: TClock` is a method call on a
/// value whose class does not carry the method: `impl TClock for Clock` is
/// written in core while `Clock` is declared in proto, so its method is
/// `Clock_members(self)`. Where the trait has one such impl there is nothing to
/// choose and the call names it; where it has several, the dispatcher chooses.
/// Where every impl IS a method on its class, the call stays what it was.
pub enum OpenCall {
    /// One impl, emitted as a function: call it by name.
    One(String),
    /// Several: the dispatcher picks by the receiver's shape at run time.
    Dispatch,
}

pub fn open_bound_call(
    reg: &TypeRegistry,
    trait_id: TypeId,
    trait_name: &str,
    method: &str,
) -> Option<OpenCall> {
    let impls = reg.impls().of_trait(trait_id);
    let free: Vec<ImplId> = impls
        .iter()
        .copied()
        .filter(|&id| {
            let def = reg.impl_def(id);
            !reg.modules().get(def.module).is_system
                && super::dispatch::emits_as_free_function(
                    reg,
                    &def.self_ty,
                    &def.generics,
                    def.module,
                )
        })
        .collect();
    if free.is_empty() {
        return None;
    }
    if impls.len() == 1 {
        let def = reg.impl_def(free[0]);
        let symbol = super::method_symbol(
            Some(leaf(trait_name)).as_deref(),
            &def.trait_args_written,
            &crate::name_map::map_fn_name(method),
            &crate::name_map::map_ty(reg, &def.self_ty),
            def.self_ty.peel_refs().id(),
        );
        return Some(OpenCall::One(super::free_fn_name(
            reg,
            &def.self_ty,
            &def.generics,
            &symbol,
        )));
    }
    Some(OpenCall::Dispatch)
}

/// Can a dispatcher be written for this trait method, and what stops it?
pub fn refusal(reg: &TypeRegistry, trait_id: TypeId, trait_name: &str, method: &str) -> Option<String> {
    // Asking is not writing. Every call site through a bound asks whether a
    // dispatcher can be written, and parking one impl's "no run-time test"
    // report on each ask filed it 22 times for `Collatable`. The report belongs
    // to the dispatcher that is actually emitted, which `dispatchers` writes
    // once per file.
    write_with(reg, trait_id, trait_name, method, Report::No).err()
}

/// Whether this call is the one that emits the dispatcher, and so the one that
/// owes a report for each impl it had to leave out.
#[derive(Clone, Copy, PartialEq)]
enum Report {
    Yes,
    No,
}

fn write(
    reg: &TypeRegistry,
    trait_id: TypeId,
    trait_name: &str,
    method: &str,
) -> Result<Dispatcher, String> {
    write_with(reg, trait_id, trait_name, method, Report::Yes)
}

fn write_with(
    reg: &TypeRegistry,
    trait_id: TypeId,
    trait_name: &str,
    method: &str,
    report: Report,
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
        // An impl emitted as module-level functions has a function per method
        // it WRITES, and a method it takes from the trait's default is written
        // nowhere: `Collatable for f64` supplies `to_bytes` and inherits
        // `compare`, so a branch for it would call `F64_compare`, which nothing
        // declares. The branch is left out, as one with no test is.
        if !declares_the_method(reg, id, method) {
            if report == Report::Yes {
                crate::diag::pending::park_at(
                    0,
                    0,
                    format!(
                        "`{}` for `{}` takes `{}` from the trait's default, and an impl written \
                         as module-level functions has no function for a method it does not \
                         write; the branch is left out and a receiver of that shape reaches the \
                         dispatcher's own fatal",
                        leaf(trait_name),
                        reg.describe(&reg.impl_def(id).self_ty),
                        method
                    ),
                );
            }
            continue;
        }
        let test = match shape_test(reg, id) {
            Ok(test) => test,
            Err(why) => {
                if report == Report::Yes {
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
                }
                continue;
            }
        };
        let Some(test) = test else {
            // An impl written for a bare parameter with no bound the run time
            // can see applies to whatever the others do not, so it is the last
            // branch rather than a test. A second one has nothing to tell it
            // from the first, so it is reported and left out.
            if catch_all.is_some() {
                if report == Report::Yes {
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
                }
                continue;
            }
            catch_all = Some(call(reg, id, &ts_method, sig));
            continue;
        };
        if let Some(other) = seen.insert(test.clone(), id) {
            // Two impls of one shape: the first one written wins, the way
            // Rust's own coherence would have refused the pair outright.
            if report == Report::Yes {
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
            }
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

/// Is there a function or a method for this impl's half of the trait method?
///
/// An impl with a CLASS carries the trait's default on that class, so it always
/// answers; one emitted as module-level functions answers only for what it
/// writes itself.
fn declares_the_method(reg: &TypeRegistry, id: ImplId, method: &str) -> bool {
    let def = reg.impl_def(id);
    !super::dispatch::emits_as_free_function(reg, &def.self_ty, &def.generics, def.module)
        || def.methods.contains_key(method)
}

pub(super) fn leaf(name: &str) -> String {
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
