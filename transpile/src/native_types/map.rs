//! `HashMap<K, V>` and `BTreeMap<K, V>` → the runtime's `HashMap`.
//!
//! NOT JavaScript's `Map`. A `Map` compares its keys by identity, so a
//! `HashMap<EntityId, Entity>` answered nothing for every key that was not the
//! very object it had been stored under — and a key read back off the wire
//! never is. `@ankurah/base`'s `HashMap` hashes a key by its `hash()` and
//! compares by its `equals()`, which is what Rust's `Hash` + `Eq` bound asks
//! for, and it owns what it holds: dropping the map drops its keys and values.

use super::MethodTranslation;
use crate::registry::TypeRegistry;
use crate::ty::Ty;

/// Translate HashMap/BTreeMap static/associated function calls
pub fn translate_static(func: &str, args: &[String]) -> Option<String> {
    match func {
        "HashMap::new" | "HashMap.new" | "HashMap::with_capacity" | "HashMap.withCapacity"
        | "BTreeMap::new" | "BTreeMap.new" => Some("new HashMap()".to_string()),
        "HashMap::default" | "HashMap.default" | "BTreeMap::default" | "BTreeMap.default" => {
            Some("new HashMap()".to_string())
        }
        // `from([(k, v), ..])` builds a map WITH those entries. Written as an
        // empty one, every entry was discarded in silence.
        "HashMap::from" | "HashMap.from" | "BTreeMap::from" | "BTreeMap.from"
            if args.len() == 1 =>
        {
            Some(format!("HashMap.from({})", args[0]))
        }
        _ => None,
    }
}

/// `insert` and `remove`, or `set` and `delete`?
///
/// Rust's `insert` answers the value it displaced and `remove` the value it
/// took out, and both hand ownership of that value to the caller. Where the
/// source uses the answer, the emitted call has to be the one that gives it —
/// `insert`/`remove` on the runtime's map. Where the source discards it, the
/// container releases it, which is `set`/`delete`.
pub fn translate_using_result(receiver: &str, method: &str, args: &[String], used: bool) -> MethodTranslation {
    let result = match (method, used) {
        ("insert", true) if args.len() == 2 => format!("{}.insert({}, {})", receiver, args[0], args[1]),
        ("insert", false) if args.len() == 2 => format!("{}.set({}, {})", receiver, args[0], args[1]),
        ("remove", true) if args.len() == 1 => format!("{}.remove({})", receiver, args[0]),
        ("remove", false) if args.len() == 1 => format!("{}.delete({})", receiver, args[0]),
        _ => return translate(receiver, method, args),
    };
    MethodTranslation::Expr(result)
}

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let result = match method {
        // Properties
        "len" => format!("{}.size", receiver),
        "is_empty" => format!("{}.size === 0", receiver),

        // Renamed methods. The answer is discarded here; a caller that reads it
        // goes through `translate_using_result`.
        "insert" if args.len() == 2 => format!("{}.set({}, {})", receiver, args[0], args[1]),
        "contains_key" => format!("{}.has({})", receiver, args.join(", ")),
        "remove" if args.len() == 1 => format!("{}.delete({})", receiver, args[0]),

        // Passthrough
        "get" | "clear" | "keys" | "values" | "entries"
            => return MethodTranslation::Passthrough,

        // Mutable iterator variants → same as immutable in JS
        "values_mut" => format!("{}.values()", receiver),
        "get_mut" if args.len() == 1 => format!("{}.get({})", receiver, args[0]),

        // `retain(|k, v| p)` is a delete loop. The predicate is PARENTHESISED,
        // because an arrow's body extends as far as it can: `(k, v) =>
        // invokeRef(cb, k, v)(_k, _v)` made the call part of the body, so what
        // the `!` tested was the arrow itself — an object, always truthy — and
        // nothing was ever deleted.
        "retain" if args.len() == 1 => format!(
            "{{ for (const [_k, _v] of {}) {{ if (!(({})(_k, _v))) {}.delete(_k); }} }}",
            receiver, args[0], receiver
        ),

        // Iterator entry points
        "iter" | "into_iter" => format!("[...{}]", receiver),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}

/// The three ways Rust finishes a `map.entry(k)`, as the runtime's `MapEntry`
/// spells them.
///
/// `or_default()` reads `V: Default` off the TYPE, which TypeScript has no way
/// to do, so the port passes the value type's default as a thunk — which is
/// what `MapEntry::orDefault` takes. Emitted without one, `orDefault()` invoked
/// `undefined`.
///
/// A finisher answers `&'a mut V` in Rust and a write-through `Slot` here,
/// which is what `*counts.entry(w).or_insert(0) += 1` stores into. Every OTHER
/// position reads the value the slot holds: `entry(k).or_default().push(v)` is
/// a call on the `Vec`, and a `Slot` has no `push`.
pub fn translate_entry(
    reg: &TypeRegistry,
    receiver_ty: &Ty,
    receiver: &str,
    method: &str,
    args: &[String],
    reads_as_value: bool,
) -> Option<MethodTranslation> {
    let Ty::Named { id, args: held } = receiver_ty.peel_refs() else {
        return None;
    };
    // `hash_map::Entry` and `btree_map::Entry` are two declarations, as they are
    // in std. Reading only the first left every `BTreeMap` entry to the
    // name-by-name table, which wrote `orDefault()` with no thunk — and
    // `orDefault(undefined)` invokes `undefined` on the first unseen key.
    if !is_entry(reg, *id) {
        return None;
    }
    let finished = match (method, args.len()) {
        ("or_insert", 1) => format!("{}.orInsert({})", receiver, args[0]),
        ("or_insert_with", 1) => format!("{}.orInsertWith({})", receiver, args[0]),
        ("or_default", 0) => {
            // The value type is the entry's second argument, and its default is
            // what Rust's `V: Default` would have supplied.
            let value = held.get(1)?;
            match crate::derives::default_value::default_value(reg, value) {
                Ok(default) => format!("{}.orDefault(() => {})", receiver, default),
                Err(why) => {
                    return Some(MethodTranslation::Refused {
                        message: format!(
                            "`or_default()` needs the value type's default, and {}",
                            why
                        ),
                        fallback: Box::new(MethodTranslation::Passthrough),
                    })
                }
            }
        }
        _ => return None,
    };
    let written = if reads_as_value { format!("{}.value", finished) } else { finished };
    Some(MethodTranslation::Expr(written))
}

/// Is this the `Entry` of one of the two maps the port writes as one runtime
/// container?
fn is_entry(reg: &TypeRegistry, id: crate::ty::TypeId) -> bool {
    ["std::collections::hash_map::Entry", "std::collections::btree_map::Entry"]
        .iter()
        .any(|path| reg.system_type(path) == Some(id))
}
