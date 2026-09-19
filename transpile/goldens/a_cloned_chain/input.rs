//! `cloned` with an adaptor standing between it and the collection.
//!
//! The payload was read off the receiver's JavaScript shape, and a `Filter` is
//! neither a sequence nor an option there, so the copy was silently dropped:
//! the emitted spread handed back the borrowed values and the loop released
//! each of them a second time.

pub struct Item {
    pub id: u64,
}

impl Drop for Item {
    fn drop(&mut self) {}
}

impl Clone for Item {
    fn clone(&self) -> Item {
        Item { id: self.id }
    }
}

/// The adaptor form: the caller owns what it collects, and the argument keeps
/// what it lent.
pub fn kept_over(items: &Vec<Item>) -> u64 {
    let taken: Vec<Item> = items.iter().filter(|i| i.id > 1).cloned().collect();
    let mut total = 0u64;
    for t in taken {
        total += t.id;
    }
    total
}

/// The same with no adaptor, which always copied.
pub fn kept_whole(items: &Vec<Item>) -> u64 {
    let taken: Vec<Item> = items.iter().cloned().collect();
    let mut total = 0u64;
    for t in taken {
        total += t.id;
    }
    total
}
