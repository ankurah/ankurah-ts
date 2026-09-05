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
mod js_value; // serde_json::Value / JsValue → unknown
mod map; // HashMap<K,V>/BTreeMap<K,V> → Map<K,V>
mod nullable; // Option<T> → T | null
mod number; // AtomicUsize/AtomicU32 → number
pub(crate) mod ordering; // std::cmp::Ordering → -1 | 0 | 1
mod set; // HashSet<T>/BTreeSet<T> → Set<T>
mod string; // String/&str → string // Arc<T>/Weak<T> — reference-counted pointer

#[cfg(test)]
#[path = "tests_ordering.rs"]
mod tests_ordering;

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
    translate_method_using(reg, receiver_ty, receiver, rust_method, args, true)
}

/// The same, told whether the call's answer is used.
///
/// `HashMap::insert` answers the value it displaced and hands ownership of it
/// to the caller; a statement that discards the answer leaves the container to
/// release it. The two are different runtime methods, so the question has to
/// reach here.
pub fn translate_method_using(
    reg: &TypeRegistry,
    receiver_ty: &Ty,
    receiver: &str,
    rust_method: &str,
    args: &[String],
    used: bool,
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

    // `Ordering` is a number here, and so are the atomics — but what a call on
    // one means is nothing like what a call on the other means, so the type is
    // asked before the shape is.
    if ordering::is_ordering(reg, receiver_ty) {
        return ordering::translate(receiver, rust_method, args);
    }

    // A `map.entry(k)` is not a shape the table below knows: it is the std
    // surface's own `Entry`, and the three ways Rust finishes one are methods
    // the runtime's `MapEntry` spells differently.
    if let Some(translated) = map::translate_entry(reg, receiver_ty, receiver, rust_method, args) {
        return translated;
    }

    // The shape a value takes in JavaScript decides which module knows how to
    // translate a call on it — the same table emission writes the type from.
    match js_shape(reg, receiver_ty) {
        JsShape::Array(_) => array::translate(receiver, rust_method, args),
        // A `Vec<u8>` is a `Uint8Array`, which is fixed-length and shares only
        // the reading half of an array's surface.
        JsShape::Bytes => bytes::translate(receiver, rust_method, args),
        JsShape::Nullable(_) => nullable::translate(receiver, rust_method, args),
        JsShape::Map(_, _) => map::translate_using_result(receiver, rust_method, args, used),
        JsShape::Set(_) => set::translate_using_result(receiver, rust_method, args, used),
        // An `Arc<T>` answers `Arc`'s own methods; everything else is a method
        // on the `T` inside it, reached the way the runtime holds it — Rust's
        // own auto-deref, written out. `Arc<AtomicUsize>::fetch_add` was left
        // as `arc.fetchAdd(1, undefined)`, a method no number has.
        JsShape::Rc(name) => match arc::translate(&name, receiver, rust_method, args) {
            MethodTranslation::Passthrough => match inner_of(reg, receiver_ty) {
                Some((inner, accessor)) => {
                    translate_method_using(reg, &inner, &format!("{}{}", receiver, accessor), rust_method, args, used)
                }
                None => MethodTranslation::Passthrough,
            },
            translated => translated,
        },
        JsShape::Str => string::translate(receiver, rust_method, args),
        // `serde_json::Value` and `JsValue`: the value JavaScript already holds.
        JsShape::Unknown => js_value::translate(reg, receiver_ty, receiver, rust_method, args),
        // An `AtomicBool` is a boolean here, and `load`/`store`/`swap` on one are
        // the same rewrites the numeric atomics take.
        JsShape::Number | JsShape::Boolean => {
            // The WIDTH is what the arithmetic helpers need, and only the
            // resolved type carries it: `u8` and `usize` are both `number`.
            let width = match receiver_ty.peel_refs() {
                Ty::Prim(prim) => Some(*prim),
                _ => None,
            };
            number::translate(receiver, rust_method, args, width)
        }
        // `Box<T>` and `&T` are the value they hold.
        JsShape::SameAs(inner) => translate_method_using(reg, &inner, receiver, rust_method, args, used),
        _ => MethodTranslation::Passthrough,
    }
}

/// What a wrapper holds, and how the emitted code reaches it.
fn inner_of(reg: &TypeRegistry, ty: &Ty) -> Option<(Ty, String)> {
    let Ty::Named { id, args } = ty.peel_refs() else { return None };
    let inner = args.first()?.clone();
    // Only where the inner type has a translation of its own; otherwise the
    // call belongs to whatever class the wrapper holds, and reaching through
    // `.value` for that would write the method on the wrong thing.
    if matches!(js_shape(reg, &inner), JsShape::Plain | JsShape::Unknown) {
        return None;
    }
    match reg.shapes().accessor(*id)? {
        crate::name_map::system_shapes::Accessor::Field(name) => Some((inner, format!(".{}", name))),
        crate::name_map::system_shapes::Accessor::Transparent => Some((inner, String::new())),
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
