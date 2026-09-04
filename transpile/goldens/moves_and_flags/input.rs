//! A local handed to somebody else is not released here. Where it is handed
//! away on some paths and not others, Rust compiles a drop flag and so does the
//! emitter.

pub struct Entity {
    pub name: String,
}

pub struct Pair {
    pub left: Entity,
}

pub fn consume(entity: Entity) -> usize { entity.name.len() }

pub fn borrow(entity: &Entity) -> usize { entity.name.len() }

/// Passed by value: gone.
pub fn moved_into_a_call() -> usize {
    let entity = Entity { name: String::new() };
    consume(entity)
}

/// Passed by reference: still ours.
pub fn borrowed_by_a_call() -> usize {
    let entity = Entity { name: String::new() };
    borrow(&entity)
}

/// Into a struct literal, and out to the caller: gone twice over.
pub fn moved_into_a_literal() -> Pair {
    let entity = Entity { name: String::new() };
    Pair { left: entity }
}

/// Handed away only where the flag is set.
pub fn moved_on_one_path(hand_it_on: bool) -> usize {
    let entity = Entity { name: String::new() };
    if hand_it_on {
        return consume(entity);
    }
    borrow(&entity)
}

/// `drop(x)` releases it where the source says, and the block does not release
/// it again.
pub fn dropped_by_hand() -> usize {
    let entity = Entity { name: String::new() };
    drop(entity);
    0
}
