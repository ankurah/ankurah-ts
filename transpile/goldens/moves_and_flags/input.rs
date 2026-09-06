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

pub struct Sink;

impl Sink {
    pub fn swallow(&self, entity: Entity, n: usize) -> usize {
        entity.name.len() + n
    }
}

pub struct Held {
    pub entity: Entity,
    pub n: usize,
}

pub fn eat(entity: Entity, n: usize) -> usize {
    entity.name.len() + n
}

/// E10: the flag stands after everything the statement evaluates, whatever
/// SHAPE the call is. It was written before the whole statement for every call
/// but `invoke(..)`, so an argument that throws left the flag set and the moved
/// value released by nobody.
pub fn plain_call(entity: Entity, n: Option<usize>, early: bool) -> usize {
    if early {
        return 0;
    }
    eat(entity, n.unwrap())
}

pub fn method_call(sink: &Sink, entity: Entity, n: Option<usize>, early: bool) -> usize {
    if early {
        return 0;
    }
    sink.swallow(entity, n.unwrap())
}

pub fn constructor(entity: Entity, n: Option<usize>, early: bool) -> Held {
    if early {
        return Held { entity: Entity { name: String::new() }, n: 0 };
    }
    Held { entity, n: n.unwrap() }
}
