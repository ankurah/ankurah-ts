// An empty collection is typed by the use BELOW it, not by its own statement.
//
// `Vec::new()` writes no element type; Rust reads it off the `push` further
// down the body. Typing and emission were one walk, so the `let` was written
// before anything below it had been read: the local stayed untyped, every use
// of it was refused, and nothing released what it held.

pub struct Entity {
    pub id: u64,
}

impl Entity {
    pub fn new(id: u64) -> Entity {
        Entity { id }
    }
}

pub fn collect_entities(ids: Vec<u64>) -> Vec<Entity> {
    let mut entities = Vec::new();
    for id in ids {
        entities.push(Entity::new(id));
    }
    entities
}

pub fn count_entities(ids: Vec<u64>) -> usize {
    let mut entities = Vec::new();
    for id in ids {
        entities.push(Entity::new(id));
    }
    entities.len()
}

pub fn first_id(ids: Vec<u64>) -> u64 {
    let entities = collect_entities(ids);
    entities[0].id
}
