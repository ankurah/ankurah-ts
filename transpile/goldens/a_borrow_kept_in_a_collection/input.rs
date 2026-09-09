// A borrow put into a collection makes the collection hold borrows.
//
// `picked.push(t)` over a `&Thing` says the element is `&Thing`, and a
// collection of borrows releases nothing: the `Thing`s belong to the caller.
// Read as `Vec<Thing>` the function released them under the caller, and the
// caller released them again.

pub struct Thing {
    pub id: u64,
}

impl Thing {
    pub fn new(id: u64) -> Thing {
        Thing { id }
    }
}

pub fn total_over(items: &Vec<Thing>) -> u64 {
    let mut picked = Vec::new();
    for t in items {
        if t.id > 1 {
            picked.push(t);
        }
    }
    let mut total = 0u64;
    for p in picked {
        total += p.id;
    }
    total
}

pub fn kept(t: &Thing) -> usize {
    let mut picked = Vec::new();
    picked.push(t);
    picked.len()
}

pub fn run() -> u64 {
    let items = vec![Thing::new(1), Thing::new(2), Thing::new(3)];
    let total = total_over(&items) + kept(&items[0]) as u64;
    total
}
