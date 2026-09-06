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
pub(crate) mod map; // HashMap<K,V>/BTreeMap<K,V> → Map<K,V>
pub mod nullable; // Option<T> → T | null
mod number; // AtomicUsize/AtomicU32 → number
pub(crate) mod ordering; // std::cmp::Ordering → -1 | 0 | 1
mod set; // HashSet<T>/BTreeSet<T> → Set<T>
mod string; // String/&str → string // Arc<T>/Weak<T> — reference-counted pointer

#[cfg(test)]
#[path = "tests_entry.rs"]
mod tests_entry;
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
    translate_method_using(
        reg,
        receiver_ty,
        receiver,
        rust_method,
        args,
        used_and_read(),
        &nullable::Once::unasked(),
    )
}

/// What the position a call stands in says about its answer.
///
/// Two questions, both the caller's rather than the call's. `HashMap::insert`
/// answers the value it displaced and hands ownership of it to the caller; a
/// statement that discards the answer leaves the container to release it, and
/// the two are different runtime methods. And `map.entry(k).or_insert(0)` is a
/// write-through `Slot`, which a `*` stores into and every other position reads
/// the value out of.
#[derive(Clone, Copy)]
pub struct Position {
    /// Is the call's answer used at all?
    pub used: bool,
    /// Is the answer read as a VALUE, rather than written through by a `*`?
    pub reads_as_value: bool,
    /// Does the lowering own the sequence's ELEMENTS? A consuming iterator
    /// terminal does, and then it is what releases the ones it does not hand
    /// back (F1). The call's own text cannot say — `xs.find(p)` looks the same
    /// either way — so the translator that resolved the method says instead.
    pub elements: iterator::Elements,
}

/// The position a caller with nothing to say about it stands in.
pub fn used_and_read() -> Position {
    Position { used: true, reads_as_value: true, elements: iterator::Elements::Borrowed }
}

/// The same, told what the position the call stands in wants of its answer.
pub fn translate_method_using(
    reg: &TypeRegistry,
    receiver_ty: &Ty,
    receiver: &str,
    rust_method: &str,
    args: &[String],
    at: Position,
    once: &nullable::Once<'_>,
) -> MethodTranslation {
    let used = at.used;
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
    if let Some(translated) =
        map::translate_entry(reg, receiver_ty, receiver, rust_method, args, at.reads_as_value)
    {
        return translated;
    }

    // The shape a value takes in JavaScript decides which module knows how to
    // translate a call on it — the same table emission writes the type from.
    match js_shape(reg, receiver_ty) {
        JsShape::Array(inner) => {
            array::translate(receiver, rust_method, args, &array::Element::of(reg, &inner), at.elements)
        }
        // A `Vec<u8>` is a `Uint8Array`, which is fixed-length and shares only
        // the reading half of an array's surface.
        JsShape::Bytes => bytes::translate(receiver, rust_method, args),
        JsShape::Nullable(_) => nullable::translate(receiver, rust_method, args, once),
        JsShape::Map(_, value) => map::translate_using_result(
            receiver,
            rust_method,
            args,
            used,
            crate::name_map::shape::writes_by_reference(reg, &value),
        ),
        JsShape::Set(_) => set::translate_using_result(receiver, rust_method, args, used),
        // An `Arc<T>` answers `Arc`'s own methods; everything else is a method
        // on the `T` inside it, reached the way the runtime holds it — Rust's
        // own auto-deref, written out. `Arc<AtomicUsize>::fetch_add` was left
        // as `arc.fetchAdd(1, undefined)`, a method no number has.
        JsShape::Rc(name) => match arc::translate(&name, receiver, rust_method, args) {
            MethodTranslation::Passthrough => match inner_of(reg, receiver_ty) {
                Some((inner, accessor)) => {
                    translate_method_using(reg, &inner, &format!("{}{}", receiver, accessor), rust_method, args, at, once)
                }
                None => MethodTranslation::Passthrough,
            },
            translated => translated,
        },
        JsShape::Str => string::translate(receiver, rust_method, args),
        // `serde_json::Value` and `JsValue`: the value JavaScript already holds.
        JsShape::Unknown => js_value::translate(reg, receiver_ty, receiver, rust_method, args),
        // An `AtomicBool` is a boolean here, and `load`/`store`/`swap` on one are
        // the same rewrites the numeric atomics take. A `bigint` takes the same
        // arm: `u64::wrapping_add` is the same free helper `u32::wrapping_add`
        // is, told a different width — written as a METHOD it was a `TypeError`
        // on every 64-bit receiver.
        JsShape::Number | JsShape::Boolean | JsShape::BigInt => {
            // The WIDTH is what the arithmetic helpers need, and only the
            // resolved type carries it: `u8` and `usize` are both `number`.
            let width = match receiver_ty.peel_refs() {
                Ty::Prim(prim) => Some(*prim),
                // An ATOMIC is the value it holds, and it holds a width:
                // `AtomicU32::fetch_add` wraps at 2^32 whatever the build's
                // debug assertions say, and the port went on counting in a
                // double.
                other => atomic_width(reg, other),
            };
            number::translate(receiver, rust_method, args, width)
        }
        // `Box<T>` and `&T` are the value they hold.
        JsShape::SameAs(inner) => translate_method_using(reg, &inner, receiver, rust_method, args, at, once),
        _ => MethodTranslation::Passthrough,
    }
}

/// The integer width an atomic holds, where the receiver is one.
///
/// An atomic IS the value it holds here, and `fetch_add` is a read-modify-write
/// of the place — but Rust's atomics WRAP at their width, whatever the build's
/// debug assertions say, and a `+=` on a `number` goes on counting.
/// `AtomicU64` is deliberately absent: the port writes it as a `number` (its
/// entry in `system_shapes` is `Form::Number`) where Rust holds a `u64`, so a
/// `u64` width here would put a `bigint` operand beside a `number` place —
/// which JavaScript refuses to mix. That disagreement is a type-mapping gap,
/// not a wrapping one, and `number::translate` reports the operation rather
/// than writing either wrong answer.
fn atomic_width(reg: &TypeRegistry, ty: &Ty) -> Option<crate::ty::Prim> {
    let Ty::Named { id, .. } = ty else { return None };
    for (path, prim) in [
        ("std::sync::atomic::AtomicUsize", crate::ty::Prim::Usize),
        ("std::sync::atomic::AtomicU32", crate::ty::Prim::U32),
    ] {
        if reg.system_type(path) == Some(*id) {
            return Some(prim);
        }
    }
    None
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
    // The three ways Rust finishes a `map.entry(k)` are the one family this
    // table must not answer. What a finisher writes needs the map's value type
    // — `or_default` needs that type's default, which TypeScript has no way to
    // read — so a receiver the engine could not type says nothing a finisher
    // can be written from. Written from the name, `orDefault()` invoked
    // `undefined` on the first unseen key, on core's property write path.
    if matches!(rust_method, "or_insert" | "or_insert_with" | "or_default" | "and_modify") {
        let message = format!(
            "`{}` is one of the ways Rust reads or finishes a `map.entry(..)`, and this receiver \
             is not an `Entry` the engine could type, so neither what the map holds nor its \
             default is known here",
            rust_method
        );
        return MethodTranslation::Refused {
            fallback: Box::new(MethodTranslation::Expr(crate::body::hole_text(&message))),
            message,
        };
    }

    // Type-erased conversions work without knowing the receiver type
    if let Some(result) = conversion::translate(receiver, rust_method, args) {
        return MethodTranslation::Expr(result);
    }

    // Iterator methods are commonly called on untyped receivers
    // A receiver the engine could not name has no ownership answer, so the
    // reading helpers are what is written and the site is already reported for
    // being untyped.
    if let Some(result) =
        iterator::translate(receiver, rust_method, args, iterator::Receiver::Unknown, iterator::Elements::Borrowed)
    {
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

        // .retain(predicate) — for a Map or a Set whose type is unknown, in the
        // same shape `map::translate` writes it: the predicate is evaluated
        // once, reached through `invokeRef`, and released when the call ends.
        "retain" if args.len() == 1 => format!(
            "((<K, V>($m: {{ [Symbol.iterator](): IterableIterator<[K, V]>; delete(key: K): unknown }}, $p: Invocable<[K, V], boolean>) => {{\n\
             \x20 try {{\n\
             \x20   for (const [$k, $v] of $m) {{ if (!invokeRef($p, $k, $v)) $m.delete($k); }}\n\
             \x20 }} finally {{\n\
             \x20   dropOwned($p);\n\
             \x20 }}\n\
             }})({}, {}))",
            receiver, args[0]
        ),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
