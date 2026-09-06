// Iterating a collection by reference and iterating it by value are two
// different `IntoIterator` impls, and only one of them hands the loop anything
// to release. Written `&` erased, `sum_borrowed` released the borrowed key and
// value on every turn and the map released them again: a double drop.
use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash)]
pub struct Key {
    pub name: String,
}

pub struct Cell {
    pub value: u32,
}

pub fn sum_borrowed(map: &HashMap<Key, Cell>) -> u32 {
    let mut total = 0u32;
    for (_key, cell) in map {
        total += cell.value;
    }
    total
}

pub fn sum_amp(map: HashMap<Key, Cell>) -> u32 {
    let mut total = 0u32;
    for (_key, cell) in &map {
        total += cell.value;
    }
    total
}

pub fn sum_consuming(map: HashMap<Key, Cell>) -> u32 {
    let mut total = 0u32;
    for (_key, cell) in map {
        total += cell.value;
    }
    total
}

pub fn widths(keys: &Vec<Key>) -> usize {
    let mut total = 0usize;
    for key in keys {
        total += key.name.len();
    }
    total
}

pub fn first_width(keys: Vec<Key>) -> usize {
    let mut total = 0usize;
    for key in keys {
        total += key.name.len();
        break;
    }
    total
}

// A pattern matched against a REFERENCE binds by reference (RFC 2005), so
// nothing the arm binds is the arm's to release. ankql's `Display for Selection`
// is written this way — `if let Some(order_by) = &self.order_by` — and the
// emitted `dropOwned(orderBy)` released a vector the field still holds: a double
// drop that aborted the whole ankql suite from the first ORDER BY case on.
pub struct Ordering {
    pub keys: Option<Vec<Key>>,
}

pub fn ordering_width(o: &Ordering) -> usize {
    let mut total = 0usize;
    if let Some(keys) = &o.keys {
        for key in keys {
            total += key.name.len();
        }
    }
    total
}

// An explicit `ref` binding over an OWNED sequence still consumes the sequence:
// Rust's `IntoIter` hands out one element per turn and drops it at the end of
// that turn, and the `ref` binds a reference INTO the element the loop owns. The
// binding's own type is a `&Key`, which owns nothing, so nothing released the
// element — and the tail release starts after the current index and cannot
// reach what the turn already handed out.
pub fn ref_widths(keys: Vec<Key>) -> usize {
    let mut total = 0usize;
    for ref key in keys {
        total += key.name.len();
    }
    total
}

/// The same written over a REFERENCE, which owns nothing at all.
pub fn ref_widths_borrowed(keys: &Vec<Key>) -> usize {
    let mut total = 0usize;
    for ref key in keys {
        total += key.name.len();
    }
    total
}

/// E11: `(&keys).into_iter()` written as a CALL selects the same
/// `IntoIterator for &Vec<T>` the sugar selects, and hands out `&Key`. The `&`
/// on a receiver was erased before the probe ran, so the by-value impl answered
/// and the loop released every element the caller still owns — a double drop
/// where the block released them too, and a release of somebody else's elements
/// where it did not.
pub fn widths_via_call(keys: Vec<Key>) -> usize {
    let mut total = 0usize;
    for key in (&keys).into_iter() {
        total += key.name.len();
    }
    total
}

/// F4/E12: `iter_mut` had no lowering at all, so it came out as
/// `cells.iterMut()` — a method no array declares, and a `TypeError` the first
/// time the loop is reached. Live at `core/node.ts` and
/// `core/property/backend/lww.ts`. It hands out `&mut Cell`, which the port
/// writes as the element object itself, and the elements stay the caller's.
pub fn bump_cells(cells: &mut Vec<Cell>) {
    for cell in cells.iter_mut() {
        cell.value += 1;
    }
}

/// The same over a map, which hands out `(&K, &mut V)`.
pub fn bump_map(map: &mut HashMap<Key, Cell>) {
    for (_key, cell) in map.iter_mut() {
        cell.value += 1;
    }
}
