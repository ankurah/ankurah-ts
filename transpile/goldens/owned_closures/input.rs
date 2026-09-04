//! A `move` closure owns what it captured and releases it when the closure is
//! released. A JS closure's captures are invisible to the cascade — a capture is
//! not a property — so a `move` closure that captures anything droppable has to
//! become a value that lists its captures. One that captures nothing droppable
//! has nothing for the cascade to find and stays an ordinary function, and so
//! does a closure that only borrows, because its captures still belong to the
//! block around it.

pub struct Entity {
    pub name: String,
}

pub fn borrow(entity: &Entity) -> usize { entity.name.len() }

/// Made and called in one statement: the closure and the `Entity` it took are
/// both released at the end of it.
pub fn run_now() -> usize {
    let entity = Entity { name: "abc".to_string() };
    (move || borrow(&entity))()
}

/// Bound to a local: the block owns the closure, releases it, and the `Entity`
/// the closure took goes with it.
pub fn run_later() -> usize {
    let entity = Entity { name: "abcd".to_string() };
    let f = move || borrow(&entity);
    f() + f()
}

/// A `move` closure over a `Copy` local: nothing droppable was captured, so
/// there is nothing to release.
pub fn plain(n: usize) -> usize {
    let f = move || n + 1;
    f()
}

/// No `move`: the closure borrows, and the block still owns the `Entity`.
pub fn borrowing() -> usize {
    let entity = Entity { name: "ab".to_string() };
    let f = || borrow(&entity);
    f()
}
