//! Writing a resolved type as TypeScript.
//!
//! This is where a `Ty` becomes text, and the only place it does. It
//! reproduces the syntactic mapping in `map_type` case for case, including its
//! warts, so that moving a consumer from the written type to the resolved one
//! cannot change what is emitted. The warts themselves — `&[u32]` becoming
//! `Uint8Array`, `u64` becoming `bigint | number` — are emission decisions to
//! revisit on their own.

use super::map_type_name;
use super::shape::{js_shape, JsShape};
use crate::registry::TypeRegistry;
use crate::ty::Ty;

/// Map a resolved Rust type to its TypeScript spelling.
pub fn map_ty(reg: &TypeRegistry, ty: &Ty) -> String {
    match js_shape(reg, ty) {
        JsShape::Bytes => "Uint8Array".to_string(),
        JsShape::Array(elem) => format!("{}[]", map_ty(reg, &elem)),
        JsShape::Nullable(inner) => format!("{} | null", map_ty(reg, &inner)),
        JsShape::Map(k, v) => format!("Map<{}, {}>", map_ty(reg, &k), map_ty(reg, &v)),
        JsShape::Set(elem) => format!("Set<{}>", map_ty(reg, &elem)),
        JsShape::Result(args) => match args.len() {
            2 => format!(
                "Result<{}, {}>",
                map_ty(reg, &args[0]),
                map_ty(reg, &args[1])
            ),
            1 => format!("Result<{}, Error>", map_ty(reg, &args[0])),
            _ => named(reg, ty),
        },
        JsShape::Tuple(elems) => {
            let parts: Vec<String> = elems.iter().map(|e| map_ty(reg, e)).collect();
            format!("[{}]", parts.join(", "))
        }
        JsShape::Fn { params, ret } => {
            let params: Vec<String> = params
                .iter()
                .enumerate()
                .map(|(i, ty)| format!("arg{}: {}", i, map_ty(reg, ty)))
                .collect();
            format!("({}) => {}", params.join(", "), map_ty(reg, &ret))
        }
        JsShape::Future(output) => match output {
            Some(ty) => format!("Promise<{}>", map_ty(reg, &ty)),
            None => "Promise<void>".to_string(),
        },
        JsShape::Trait(name) => name,
        JsShape::SameAs(inner) => map_ty(reg, &inner),
        JsShape::Str => "string".to_string(),
        JsShape::Number => "number".to_string(),
        JsShape::BigInt => "bigint | number".to_string(),
        JsShape::Boolean => "boolean".to_string(),
        JsShape::Void => "void".to_string(),
        JsShape::Never => "never".to_string(),
        JsShape::Unknown => "unknown".to_string(),
        JsShape::Rc(_) | JsShape::Plain => named(reg, ty),
    }
}

/// A type with no shape of its own: written by its own name, with its
/// arguments. The name table still applies, because a foreign `AtomicUsize` or
/// pest's `Rule` has a TypeScript spelling and no declaration to hang it on.
fn named(reg: &TypeRegistry, ty: &Ty) -> String {
    match ty {
        Ty::Named { id, args } => {
            let name = reg.name_of(*id);
            // The name table renders std's and a foreign crate's types; a
            // crate's own declaration keeps the name it was declared with. A
            // `struct String` in ankurah is a class called `String`, not
            // TypeScript's `string`, and mapping it by leaf name silently made
            // it one.
            // A declared type the runtime exports under a different name is
            // written under that name; `tokio::sync::Mutex` is `AsyncMutex`,
            // and the leaf name would have handed it to std's `Mutex`.
            let mapped = if let Some(runtime) = reg.shapes().runtime_name(*id) {
                runtime.to_string()
            } else if id.is_foreign() || reg.is_system(*id) {
                map_type_name(&name).to_string()
            } else {
                name
            };
            if args.is_empty() {
                return mapped;
            }
            let inner: Vec<String> = args.iter().map(|a| map_ty(reg, a)).collect();
            format!("{}<{}>", mapped, inner.join(", "))
        }
        Ty::Param(name) => name.clone(),
        // A projection is written by its last segment, as any path is.
        Ty::Assoc { name, .. } => map_type_name(name).to_string(),
        other => format!("{:?}", other),
    }
}
