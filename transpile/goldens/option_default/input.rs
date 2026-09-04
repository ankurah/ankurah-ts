//! `Option<T>` is `T | null`, so `?` on one is a null test and an early return.
//! `unwrap_or` takes its fallback by value and Rust builds it whether or not it
//! is wanted — so on the `Some` path the fallback is built and then dropped,
//! which is the difference between it and the closure `unwrap_or_else` takes.

pub struct Entity {
    pub name: String,
}

pub fn borrow(entity: &Entity) -> usize { entity.name.len() }

pub fn make(raw: &str) -> Option<Entity> {
    if raw.is_empty() {
        return None;
    }
    Some(Entity { name: raw.to_string() })
}

/// The fallback owns a `String` and is built on both paths. Where the option
/// was `Some`, the fallback nobody wanted is released.
pub fn or_fallback(raw: &str) -> Entity {
    make(raw).unwrap_or(Entity { name: "fallback".to_string() })
}

/// The same answer with the fallback built only where it is wanted.
pub fn or_else(raw: &str) -> Entity {
    make(raw).unwrap_or_else(|| Entity { name: "lazy".to_string() })
}

/// `?` bound to a name: the block owns the `Entity` from there and releases it.
pub fn width(raw: &str) -> Option<usize> {
    let entity = make(raw)?;
    Some(borrow(&entity))
}

/// `?` whose value nobody wants. Rust drops it at the end of the statement.
pub fn check(raw: &str) -> Option<usize> {
    make(raw)?;
    Some(0)
}
