//! `vec![a, b]` takes its elements by value, so the locals it names are gone and
//! the block releases none of them. `format!` takes its arguments by reference,
//! so the locals it names are still the block's and the block releases every one.

pub struct Entity {
    pub name: String,
}

pub struct Batch {
    pub entities: Vec<Entity>,
}

pub fn borrow(entity: &Entity) -> usize { entity.name.len() }

/// Both locals move into the vector, and the vector into the struct, and the
/// struct out to the caller.
pub fn gather() -> Batch {
    let first = Entity { name: "a".to_string() };
    let second = Entity { name: "bb".to_string() };
    Batch { entities: vec![first, second] }
}

/// The same two locals, read by a `format!`. Nothing moved, so the block
/// releases both.
pub fn describe() -> String {
    let first = Entity { name: "a".to_string() };
    let second = Entity { name: "bb".to_string() };
    format!("{}:{}", first.name, borrow(&second))
}
