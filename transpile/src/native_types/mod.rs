//! Native type translations — Rust types that map to JS/TS native types.
//!
//! Each submodule handles one Rust→TS type mapping and its method translations.
//! The body translator resolves the receiver type, then dispatches here.
//! System types with proper TS implementations (Arc, RwLock, Result, etc.)
//! don't need entries here — they pass through as-is.

mod arc;
pub(crate) mod array; // Vec<T> → T[]
mod bytes; // Vec<u8>/[u8] → Uint8Array
pub(crate) mod conversion; // into/from/as_ref — the conversions the runtime performs
pub(crate) mod iterator; // Iterator trait methods on arrays
mod map; // HashMap<K,V>/BTreeMap<K,V> → Map<K,V>
mod nullable; // Option<T> → T | null
mod number; // AtomicUsize/AtomicU32 → number
mod set; // HashSet<T>/BTreeSet<T> → Set<T>
mod string; // String/&str → string // Arc<T>/Weak<T> — reference-counted pointer

use crate::name_map::shape::{js_shape, JsShape};
use crate::registry::TypeRegistry;
use crate::ty::Ty;

/// Translate a static/associated function call (e.g., Vec::new(), HashMap::new()).
/// Returns Some(translation) if the call matches a native type constructor.
pub fn translate_static_call(func: &str, args: &[String]) -> Option<String> {
    // Try each native type module's static translator
    arc::translate_static(func, args)
        .or_else(|| array::translate_static(func, args))
        .or_else(|| string::translate_static(func, args))
        .or_else(|| map::translate_static(func, args))
        .or_else(|| set::translate_static(func, args))
        .or_else(|| number::translate_static(func, args))
}

/// Result of a method translation
pub enum MethodTranslation {
    /// Translated to this expression string
    Expr(String),
    /// No translation needed — pass through as receiver.method(args)
    Passthrough,
    /// The call has no translation that runs. `fallback` is emitted anyway so
    /// the output keeps its shape, and `message` says what is missing.
    Refused {
        message: String,
        fallback: Box<MethodTranslation>,
    },
}

/// Translate a method call based on the resolved receiver type.
pub fn translate_method(
    reg: &TypeRegistry,
    receiver_ty: &Ty,
    receiver: &str,
    rust_method: &str,
    args: &[String],
) -> MethodTranslation {
    // Check type-erased conversions first (apply to any type)
    if let Some(result) = conversion::translate(receiver, rust_method, args) {
        return MethodTranslation::Expr(result);
    }

    // unwrap/expect is handled in body.rs before dispatch reaches here.
    // Result.unwrap() passes through to Passthrough (handled by Result's class method).

    // A projection the impl table could not answer — `<impl IntoIterator as
    // IntoIterator>::IntoIter` before the closures step types it — names no
    // type, so nothing is known about how a call on it is written. That is the
    // same position as a call whose receiver did not resolve at all, and it
    // takes the same table. (Emission still writes the projection's own name
    // where the *type* is written; this is only about calls.)
    if matches!(receiver_ty, Ty::Assoc { .. }) {
        return MethodTranslation::Refused {
            message: format!(
                "the receiver of `{}` is a projection the impl table could not settle, so the \
                 call is written from its name alone",
                rust_method
            ),
            fallback: Box::new(translate_untyped(receiver, rust_method, args)),
        };
    }

    // The shape a value takes in JavaScript decides which module knows how to
    // translate a call on it — the same table emission writes the type from.
    match js_shape(reg, receiver_ty) {
        JsShape::Array(_) => array::translate(receiver, rust_method, args),
        // A `Vec<u8>` is a `Uint8Array`, which is fixed-length and shares only
        // the reading half of an array's surface.
        JsShape::Bytes => bytes::translate(receiver, rust_method, args),
        JsShape::Nullable(_) => nullable::translate(receiver, rust_method, args),
        JsShape::Map(_, _) => map::translate(receiver, rust_method, args),
        JsShape::Set(_) => set::translate(receiver, rust_method, args),
        JsShape::Rc(name) => arc::translate(&name, receiver, rust_method, args),
        JsShape::Str => string::translate(receiver, rust_method, args),
        JsShape::Number => number::translate(receiver, rust_method, args),
        // `Box<T>` and `&T` are the value they hold.
        JsShape::SameAs(inner) => translate_method(reg, &inner, receiver, rust_method, args),
        _ => MethodTranslation::Passthrough,
    }
}

/// Translate a method call when receiver type is unknown.
/// Handles methods that are unambiguous regardless of type, plus common
/// fallbacks for methods that are almost always the same translation.
pub fn translate_untyped(receiver: &str, rust_method: &str, args: &[String]) -> MethodTranslation {
    // Type-erased conversions work without knowing the receiver type
    if let Some(result) = conversion::translate(receiver, rust_method, args) {
        return MethodTranslation::Expr(result);
    }

    // Iterator methods are commonly called on untyped receivers
    if let Some(result) = iterator::translate(receiver, rust_method, args) {
        return MethodTranslation::Expr(result);
    }

    // Common methods that have the same translation for most types.
    // These fire when we can't resolve the receiver type — they cover
    // the most common case (Array/string) and are correct for those.
    // If a type needs different behavior, it should be in the typed dispatch.
    let result = match rust_method {
        // .len() → .length for arrays and strings (most common case)
        // Map/Set use .size but those should resolve to typed dispatch
        "len" if args.is_empty() => format!("{}.length", receiver),
        "is_empty" if args.is_empty() => format!("{}.length === 0", receiver),

        // .iter() → spread (works for arrays, Maps, Sets)
        "iter" | "into_iter" => format!("[...{}]", receiver),
        "values" if args.is_empty() => format!("[...{}]", receiver),

        // unwrap/expect handled in body.rs before reaching here.

        // Nullable checks
        "is_some" => format!("{} != null", receiver),
        "is_none" => format!("{} == null", receiver),

        // .contains() → .includes() for arrays (Map/Set use .has() via typed dispatch)
        "contains" if args.len() == 1 => format!("{}.includes({})", receiver, args[0]),

        // Mutable variants → same as immutable in JS
        "values_mut" => format!("{}.values()", receiver),
        "get_mut" if args.len() == 1 => format!("{}.get({})", receiver, args[0]),

        // .retain(predicate) — works for Map/Set/Vec when type unknown
        "retain" if args.len() == 1 => format!(
            "{{ for (const [_k, _v] of {}) {{ if (!({}(_k, _v))) {}.delete(_k); }} }}",
            receiver, args[0], receiver
        ),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
