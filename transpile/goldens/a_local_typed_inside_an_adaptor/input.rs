// A local the body fills from inside an adaptor's closure is typed before any
// of the body is written.
//
// `items.iter().map(|t| { kept.push(t.wrap()); .. })` says what `kept` holds,
// and the adaptor writes its bound in its own terms — `FnMut(Self::Item)` — so
// the receiver has to settle that projection before the closure's parameter can
// type anything. Untyped at its `let`, `kept` was released by nobody on the
// path that leaves before the loop.

pub struct Thing {
    pub id: u64,
}

pub struct Tag {
    pub id: u64,
}

impl Thing {
    pub fn new(id: u64) -> Thing {
        Thing { id }
    }

    pub fn wrap(&self) -> Tag {
        Tag { id: self.id }
    }
}

pub fn total_of(items: Vec<Thing>, stop: bool) -> u64 {
    let mut kept = Vec::new();
    let ids: Vec<u64> = items
        .iter()
        .map(|t| {
            kept.push(t.wrap());
            t.id
        })
        .collect();
    if stop {
        return ids.len() as u64;
    }
    let mut total = 0u64;
    for k in kept {
        total += k.id;
    }
    total
}

pub fn run() -> u64 {
    let taken = total_of(vec![Thing::new(1), Thing::new(2)], false);
    let left = total_of(vec![Thing::new(3), Thing::new(4)], true);
    taken + left
}
