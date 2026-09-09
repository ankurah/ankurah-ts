// An unknown on the ARGUMENT side is settled by the parameter it is handed to.
//
// `fill(&mut found)` says what `found` holds, because `fill` writes its own
// parameter type out in full. A constraint that ran only where the DECLARED
// side carried an unknown left `found` untyped, so nothing released what it
// collected.

pub struct Tag {
    pub id: u64,
}

impl Tag {
    pub fn new(id: u64) -> Tag {
        Tag { id }
    }
}

pub fn fill(out: &mut Vec<Tag>, upto: u64) {
    let mut n = 0u64;
    while n < upto {
        out.push(Tag::new(n));
        n += 1;
    }
}

pub fn how_many(upto: u64) -> usize {
    let mut found = Vec::new();
    fill(&mut found, upto);
    found.len()
}

pub fn run() -> usize {
    how_many(3)
}
