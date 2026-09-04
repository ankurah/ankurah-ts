//! An assignment over a live place releases what was there. Rust builds the new
//! value first and drops the old one second, which is the order that lets the
//! right-hand side read the place it is about to overwrite.

use std::sync::Mutex;

pub struct Entity {
    pub name: String,
}

pub struct Holder {
    pub inner: Entity,
}

pub fn borrow(entity: &Entity) -> usize { entity.name.len() }

/// Over a binding: the first `Entity` is released where the second lands, and
/// the block releases only the one the binding holds at the end.
pub fn replace(first: &str, second: &str) -> usize {
    let mut entity = Entity { name: first.to_string() };
    entity = Entity { name: second.to_string() };
    borrow(&entity)
}

/// The same assignment on one path only. The path that skipped it still has the
/// first `Entity`, and the block releases whichever one is there.
pub fn maybe_replace(swap: bool) -> usize {
    let mut entity = Entity { name: "a".to_string() };
    if swap {
        entity = Entity { name: "bb".to_string() };
    }
    borrow(&entity)
}

/// Over a field: the struct releases the field's old value and keeps the new
/// one, and the struct itself is untouched.
pub fn set_field(holder: &mut Holder, name: &str) -> usize {
    holder.inner = Entity { name: name.to_string() };
    borrow(&holder.inner)
}

/// Through a guard: the write lands in the mutex's own storage, so what the
/// mutex held is what gets released.
pub fn set_through_guard(cell: &Mutex<Entity>, name: &str) -> usize {
    let mut guard = cell.lock().unwrap();
    *guard = Entity { name: name.to_string() };
    guard.name.len()
}
