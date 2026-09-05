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

/// A closure whose body hands a capture away is an `FnOnce`: Rust lets it run
/// once, and the capture is the body's from there. The port had no call that
/// transferred a capture, so the capture was left out of what the closure owned
/// and nothing released it — reported, and a leak. `callOnce` transfers them
/// and marks the closure moved, so the closure is not dropped after one.
pub fn consumed(entity: Entity) -> usize {
    let take = move || {
        let held = entity;
        borrow(&held)
    };
    take()
}

/// R10: a callee sees only the BOUND, and whether the closure it is handed
/// needed wrapping is a property of what that closure captured. Written `f(x)`,
/// this raised `TypeError: f is not a function` the moment a caller passed one
/// the emitter had wrapped — live at core's `node_applier`, core's `entity` and
/// storage-sqlite's `engine`.
pub fn through_a_bound<F>(f: F, n: usize) -> usize
where
    F: FnOnce(usize) -> usize,
{
    f(n)
}

pub fn hands_a_wrapped_one(entity: Entity) -> usize {
    through_a_bound(move |n| n + entity.name.len(), 1)
}

pub fn hands_a_plain_one(n: usize) -> usize {
    through_a_bound(|x| x + 1, n)
}

/// A callable parameter written BY VALUE is the body's: Rust drops it at the
/// end, and only the CALL borrows. The port wrote the call as `invokeRef`,
/// which leaves the closure whole, and nothing released it — so every capture
/// of every wrapped closure handed to one leaked. Live at core's
/// `ResultSet::retain_dirty` and signals' `Value::set_with`.
pub fn twice_by_value<F>(mut f: F, n: usize) -> usize
where
    F: FnMut(usize) -> usize,
{
    f(n) + f(n)
}

/// The same bound written as a REFERENCE. Here the closure is the caller's, and
/// releasing it in this body would drop a value somebody else still holds.
pub fn twice_by_reference<F>(f: &mut F, n: usize) -> usize
where
    F: FnMut(usize) -> usize,
{
    f(n) + f(n)
}
