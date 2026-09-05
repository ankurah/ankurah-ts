//! Writing a resolved type as TypeScript.
//!
//! This is where a `Ty` becomes text, and the only place it does. It
//! reproduces the syntactic mapping in `map_type` case for case, including its
//! warts, so that moving a consumer from the written type to the resolved one
//! cannot change what is emitted. The warts themselves — `&[u32]` becoming
//! `Uint8Array` — are emission decisions to
//! revisit on their own.

use super::map_type_name;
use super::shape::{js_shape, JsShape};
use crate::registry::TypeRegistry;
use crate::ty::Ty;

/// An element type, written so that `[]` after it means what Rust meant.
///
/// TypeScript's `[]` binds TIGHTER than `|`, so `Vec<Option<T>>` written
/// `T | null[]` is read as `T | (null[])` — a `T` or an array of nulls, which is
/// neither of the things Rust said. Five sites in `storage-common/sorting.ts`
/// declared `Value | null[] | null` and meant `(Value | null)[] | null`.
pub(crate) fn as_an_element(written: &str) -> String {
    match has_a_top_level_union(written) {
        true => format!("({})", written),
        false => written.to_string(),
    }
}

/// Is there a `|` this type's OWN, rather than one inside a tuple, an argument
/// list or a parenthesised group? `[string, Value | null]` carries its own
/// brackets and needs none from here.
fn has_a_top_level_union(written: &str) -> bool {
    let mut depth = 0i32;
    let bytes = written.as_bytes();
    for (at, c) in written.char_indices() {
        match c {
            '(' | '[' | '<' | '{' => depth += 1,
            ')' | ']' | '>' | '}' => depth -= 1,
            '|' if depth == 0 => {
                // ` | `, not the `|` of some other spelling.
                let before = at > 0 && bytes[at - 1] == b' ';
                let after = bytes.get(at + 1) == Some(&b' ');
                if before && after {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Map a resolved Rust type to its TypeScript spelling.
pub fn map_ty(reg: &TypeRegistry, ty: &Ty) -> String {
    match js_shape(reg, ty) {
        JsShape::Bytes => "Uint8Array".to_string(),
        JsShape::Array(elem) => format!("{}[]", as_an_element(&map_ty(reg, &elem))),
        // R5: an `Option<T>` is `T | null`, and that spelling has room for ONE
        // `null`. `Option<Option<T>>` collapses — `Some(None)` and the outer
        // `None` become the same value — and so does an `Option` of anything
        // whose own spelling already admits null. The port has no tagged shape
        // for it, so the site says so rather than writing a type that cannot
        // tell the two apart.
        JsShape::Nullable(inner) => {
            let written = map_ty(reg, &inner);
            if written.ends_with(" | null") || written == "null" {
                crate::diag::pending::park_at(
                    0,
                    0,
                    format!(
                        "`Option<{}>` is written `{} | null`, and that spelling holds one \
                         `null`: the outer `None` and an inner one are the same value, so \
                         `Some(None)` and `None` cannot be told apart",
                        written, written
                    ),
                );
            }
            format!("{} | null", written)
        }
        // The runtime's own keyed containers, not JavaScript's `Map` and
        // `Set`. Those compare keys by IDENTITY, so a `HashMap<EntityId, _>`
        // answered nothing for every key that was not the very object it had
        // been stored under — which is every key read back off the wire.
        JsShape::Map(k, v) => format!("HashMap<{}, {}>", map_ty(reg, &k), map_ty(reg, &v)),
        JsShape::Set(elem) => format!("HashSet<{}>", map_ty(reg, &elem)),
        JsShape::Result(args) => match args.len() {
            2 => format!(
                "Result<{}, {}>",
                map_ty(reg, &args[0]),
                map_ty(reg, &args[1])
            ),
            // A `Result` written with one argument is `anyhow::Result<T>`,
            // whose alias defaults the error to `anyhow::Error`. `Error` is
            // JavaScript's own error and promised the wrong type.
            1 => format!("Result<{}, AnyhowError>", map_ty(reg, &args[0])),
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
        JsShape::BigInt => "bigint".to_string(),
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
        // The primitives `js_shape` has no shape for. Each one still has a
        // spelling a reader can use, and writing the debug form put `Prim(Char)`
        // in a type position, which is not a TypeScript type at all.
        Ty::Prim(crate::ty::Prim::Char) => "string".to_string(),
        Ty::Prim(crate::ty::Prim::Isize) => "number".to_string(),
        Ty::Prim(crate::ty::Prim::U128) | Ty::Prim(crate::ty::Prim::I128) => "bigint".to_string(),
        Ty::Str => "string".to_string(),
        Ty::Unit => "void".to_string(),
        Ty::Never => "never".to_string(),
        other => format!("{:?}", other),
    }
}
