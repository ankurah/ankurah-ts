//! A block releases what it still owns, in a `finally`, in reverse declaration
//! order, and every way out of the block goes through it.

pub struct Entity {
    pub name: String,
}

pub struct Registry {
    pub count: usize,
}

impl Registry {
    pub fn new() -> Self { Registry { count: 0 } }

    /// Two owned locals and an early return: the `finally` releases both,
    /// whichever way the body leaves.
    pub fn describe(&self, empty: bool) -> usize {
        let first = Entity { name: String::new() };
        let second = Entity { name: String::new() };
        if empty {
            return 0;
        }
        first.name.len() + second.name.len()
    }

    /// A `Copy` type and a primitive have no drop glue, so this block owns
    /// nothing and needs no `finally` at all.
    pub fn tally(&self) -> usize {
        let n = 3usize;
        n + self.count
    }
}
