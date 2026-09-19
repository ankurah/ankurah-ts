//! How the run time is asked whether a receiver is the one an impl is for.
//!
//! For: a dispatcher chooses among a trait's impls by the shape of the value in
//! hand, and each shape needs a test JavaScript can run — `instanceof` for a
//! class, `typeof` for a primitive, an arity check for a callable a wrapper
//! holds. An impl whose shape answers no test is left out and reported.

use super::dispatch::has_emitted_class;
use crate::name_map::shape::{js_shape, JsShape};
use crate::registry::{ImplId, TypeRegistry};
use crate::ty::Ty;

use super::open_dispatch::leaf;

/// The run-time test that says a receiver is the one this impl is written for.
///
/// `Ok(None)` is an impl written for anything at all, which needs no test and
/// stands last.
pub(super) fn shape_test(reg: &TypeRegistry, id: ImplId) -> Result<Option<String>, String> {
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
        // An impl for `Option<T>` is written for a type the port spells
        // `T | null`, which has no run-time shape of its own: a present value is
        // whatever `T` is, and `null` is nothing any test matches. So it cannot
        // be chosen by a test, only by exhaustion — the last branch, exactly as
        // an impl written for a bare parameter is. `WaitResult for bool` beside
        // `WaitResult for Option<T>` is that pair, and refusing the nullable
        // left the trait with one testable impl and no dispatcher at all.
        ty if matches!(js_shape(reg, ty), JsShape::Nullable(_)) => Ok(None),
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
///
/// A PRIMITIVE has no id and needs none: `impl WaitResult for bool` is a
/// `typeof self === 'boolean'`, and requiring an id first sent every primitive
/// impl to the refusal, which took the whole dispatcher with it.
fn class_test(reg: &TypeRegistry, ty: &Ty) -> Option<String> {
    let id = ty.peel_refs().id();
    let bare = id.map(|id| {
        crate::name_map::map_ty(
            reg,
            &Ty::Named {
                id,
                args: Vec::new(),
            },
        )
    });
    if let Some(bare) = &bare {
        if has_emitted_class(reg, ty) {
            return Some(format!("self instanceof {}", bare));
        }
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
        // The runtime containers, which is what every construction now builds:
        // a test against JavaScript's `Set` and `Map` matched nothing the port
        // makes, so the dispatcher fell through for every map and set.
        JsShape::Set(_) => return Some("self instanceof HashSet".to_string()),
        JsShape::Map(_, _) => return Some("self instanceof HashMap".to_string()),
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
    let (Some(id), Some(bare)) = (id, bare) else { return None };
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
pub(super) fn call(reg: &TypeRegistry, id: ImplId, ts_method: &str, sig: &crate::registry::MethodSig) -> String {
    let def = reg.impl_def(id);
    let args: Vec<String> = sig
        .params
        .iter()
        .filter(|(name, _)| name != "self")
        .map(|(name, _)| crate::name_map::to_camel_case(name))
        .collect();
    if !super::dispatch::emits_as_free_function(reg, &def.self_ty, &def.generics, def.module) {
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
