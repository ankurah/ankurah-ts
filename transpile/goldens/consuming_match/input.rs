//! A `match` on a scrutinee taken by value hands the payload to the arm as the
//! arm's own and leaves the enum moved: an arm that hands the payload on
//! releases nothing, and an arm that only reads it releases it itself. A `match`
//! on a reference reads the payload and leaves the enum whole for its owner.

pub struct Entity {
    pub name: String,
}

pub enum Slot {
    Empty,
    Filled(Entity),
}

pub fn consume(entity: Entity) -> usize { entity.name.len() }

pub fn borrow(entity: &Entity) -> usize { entity.name.len() }

/// By value, and the arm hands the payload on: nothing releases the `Slot`
/// afterwards, because after the move there is no `Slot` left to release.
pub fn take(slot: Slot) -> usize {
    match slot {
        Slot::Empty => 0,
        Slot::Filled(entity) => consume(entity),
    }
}

/// By value, and the arm keeps the payload: the arm is what releases it.
pub fn width(slot: Slot) -> usize {
    match slot {
        Slot::Empty => 0,
        Slot::Filled(entity) => borrow(&entity) + 1,
    }
}

/// By value, and the arm hands the payload to the caller.
pub fn into_entity(slot: Slot) -> Option<Entity> {
    match slot {
        Slot::Empty => None,
        Slot::Filled(entity) => Some(entity),
    }
}

/// An arm with a block body is the arrow function's own statements, so the
/// value its tail produces is the arm's and the release still runs.
pub fn label(slot: Slot) -> usize {
    match slot {
        Slot::Empty => 0,
        Slot::Filled(entity) => {
            let width = borrow(&entity);
            width * 2
        }
    }
}

/// By reference: the arm borrows the payload and the caller still owns the
/// whole `Slot`.
pub fn peek(slot: &Slot) -> usize {
    match slot {
        Slot::Empty => 0,
        Slot::Filled(entity) => borrow(entity),
    }
}
