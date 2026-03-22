//! Native type translations — Rust types that map to JS/TS native types.
//!
//! Each submodule handles one Rust→TS type mapping and its method translations.
//! The body translator resolves the receiver type, then dispatches here.
//! System types with proper TS implementations (Arc, RwLock, Result, etc.)
//! don't need entries here — they pass through as-is.

mod array;      // Vec<T> → T[]
mod string;     // String/&str → string
mod map;        // HashMap<K,V>/BTreeMap<K,V> → Map<K,V>
mod set;        // HashSet<T>/BTreeSet<T> → Set<T>
mod nullable;   // Option<T> → T | null
mod number;     // AtomicUsize/AtomicU32 → number
mod iterator;   // Iterator trait methods on arrays
mod conversion; // into/from/as_ref — type-erased identity transforms

use crate::resolve::ResolvedType;

/// Translate a static/associated function call (e.g., Vec::new(), HashMap::new()).
/// Returns Some(translation) if the call matches a native type constructor.
pub fn translate_static_call(func: &str, args: &[String]) -> Option<String> {
    // Try each native type module's static translator
    array::translate_static(func, args)
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
}

/// Translate a method call based on the resolved receiver type.
/// Returns Some(translation) if the type has a native mapping, None if unknown.
pub fn translate_method(
    receiver_ty: &ResolvedType,
    receiver: &str,
    rust_method: &str,
    args: &[String],
) -> MethodTranslation {
    // Check type-erased conversions first (apply to any type)
    if let Some(result) = conversion::translate(receiver, rust_method, args) {
        return MethodTranslation::Expr(result);
    }

    match receiver_ty {
        ResolvedType::Array(_) => array::translate(receiver, rust_method, args),
        ResolvedType::Nullable(_) => nullable::translate(receiver, rust_method, args),
        ResolvedType::Named { name, .. } => {
            match name.as_str() {
                "Map" | "HashMap" | "BTreeMap" => map::translate(receiver, rust_method, args),
                "Set" | "HashSet" | "BTreeSet" => set::translate(receiver, rust_method, args),
                _ => MethodTranslation::Passthrough,
            }
        }
        ResolvedType::Primitive(p) => {
            match p.as_str() {
                "string" => string::translate(receiver, rust_method, args),
                "number" => number::translate(receiver, rust_method, args),
                _ => MethodTranslation::Passthrough,
            }
        }
        _ => MethodTranslation::Passthrough,
    }
}

/// Translate a method call when receiver type is unknown.
/// Handles methods that are unambiguous regardless of type, plus common
/// fallbacks for methods that are almost always the same translation.
pub fn translate_untyped(
    receiver: &str,
    rust_method: &str,
    args: &[String],
) -> MethodTranslation {
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

        // .unwrap() → identity (most common: Result.unwrap() is on the class,
        // but when type is unknown and the source had .unwrap(), it's usually
        // Option which is nullable — stripping is the safer default)
        "unwrap" | "expect" => receiver.to_string(),

        // Nullable checks
        "is_some" => format!("{} != null", receiver),
        "is_none" => format!("{} == null", receiver),
        "unwrap_or" if args.len() == 1 => format!("{} ?? {}", receiver, args[0]),
        "unwrap_or_else" if args.len() == 1 => format!("{} ?? ({})()", receiver, args[0]),

        // .contains() → .includes() for arrays (Map/Set use .has() via typed dispatch)
        "contains" if args.len() == 1 => format!("{}.includes({})", receiver, args[0]),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}
