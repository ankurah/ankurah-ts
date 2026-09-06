//! Moving one field out of a struct leaves the rest of the struct behind. The
//! field's value becomes the new owner's, reading it again is what Rust would
//! have rejected, and the struct itself is not moved — its remaining fields are
//! still its own and its drop still runs.

pub struct Entity {
    pub name: String,
}

pub struct Pair {
    pub one: Entity,
    pub two: Entity,
}

pub struct Single {
    pub only: Entity,
}

pub fn consume(entity: Entity) -> usize { entity.name.len() }

pub fn borrow(entity: &Entity) -> usize { entity.name.len() }

/// `let one = pair.one;` is the partial move. `pair` keeps `two`, and the block
/// still releases `pair`.
pub fn take_one(pair: Pair) -> usize {
    let one = pair.one;
    let seen = borrow(&pair.two);
    consume(one) + seen
}

/// The same move, straight into a struct literal. Whatever `pair` still holds
/// is released when `pair` is.
pub fn split(pair: Pair) -> Single {
    Single { only: pair.one }
}

/// Both fields move out, so `pair` is left holding nothing and its drop has
/// nothing to cascade into.
pub fn take_both(pair: Pair) -> usize {
    let one = pair.one;
    let two = pair.two;
    consume(one) + consume(two)
}

impl Entity {
    /// A method declared `self` takes its receiver with it.
    pub fn into_name(self) -> String {
        self.name
    }

    pub fn width(&self) -> usize {
        self.name.len()
    }
}

/// Calling a `self` method on a FIELD of a place is a partial move too: Rust
/// takes the field out of the struct and leaves the rest where it was. Written
/// as a plain property read the struct and the callee both owned it, so the
/// struct's own drop released a value the callee had already taken —
/// `BUG: Entity was used after being moved`. Six of ankql's seven
/// `ast.test.ts` failures are this shape:
/// `selection.predicate.populate(..)`.
pub fn name_of_one(pair: Pair) -> String {
    pair.one.into_name()
}

/// A method taking `&self` takes nothing, and the field is read where it is.
pub fn width_of_one(pair: &Pair) -> usize {
    pair.one.width()
}
