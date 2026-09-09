// What the engine writes where nothing in a body says what an unknown stands
// for, and what it says about it.
//
// `held` is a `Vec<?>` all the way down: `remove` takes an element out and
// `len` counts them, and neither names one. The local reports once for what
// nothing can release, each USE reports what the translator did instead, and
// the output keeps its shape. `filled` is the same body with one line that
// says what it holds.

pub struct Tag {
    pub id: u64,
}

impl Tag {
    pub fn new(id: u64) -> Tag {
        Tag { id }
    }
}

pub fn counted(n: usize) -> usize {
    let mut held = Vec::new();
    if n > 0 {
        held.remove(0);
    }
    held.len()
}

pub fn filled() -> usize {
    let mut held = Vec::new();
    held.push(Tag::new(1));
    held.len()
}
