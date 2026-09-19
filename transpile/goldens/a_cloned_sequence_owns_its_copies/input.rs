// `cloned` over a sequence hands the caller its OWN values, so the collection
// keeps the ones it holds.
//
// Written as a bare spread it copied the map's slots and not its values, and
// the release the caller owes then dropped each value the map still holds a
// second time.

use std::collections::HashMap;

pub struct Listener {
    pub id: u64,
}

impl Drop for Listener {
    fn drop(&mut self) {}
}

impl Clone for Listener {
    fn clone(&self) -> Listener {
        Listener { id: self.id }
    }
}

pub fn total_of_every_listener() -> u64 {
    let mut listeners: HashMap<u64, Listener> = HashMap::new();
    listeners.insert(1, Listener { id: 1 });
    listeners.insert(2, Listener { id: 2 });
    let taken = listeners.values().cloned().collect::<Vec<_>>();
    let mut total = 0u64;
    for listener in taken {
        total += listener.id;
    }
    total
}
