//! A `match` whose arms are literals is written as an if-chain, and an arm that
//! hands a local away sets that local's drop flag on the way — the same line
//! the enclosing block would have written had the arm been a statement of it.
//! Without it the `finally` released a value the arm had already handed on.

pub struct Entity {
    pub name: String,
}

pub fn consume(entity: Entity) -> usize {
    entity.name.len()
}

pub fn borrow(entity: &Entity) -> usize {
    entity.name.len()
}

/// One arm hands the Entity to the callee and the other keeps it, so the block
/// releases it only on the path that still owns it.
pub fn by_flag(hand_it_on: bool) -> usize {
    let entity = Entity { name: String::new() };
    match hand_it_on {
        true => consume(entity),
        false => borrow(&entity),
    }
}

/// The same over a number, where the arm that hands it away stands between two
/// that do not.
pub fn by_number(which: u32) -> usize {
    let entity = Entity { name: String::new() };
    match which {
        0 => borrow(&entity),
        1 => consume(entity),
        _ => borrow(&entity) + 1,
    }
}
