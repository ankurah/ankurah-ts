// A local whose only binding constraint sits inside a macro argument.
//
// The emitter parses these arguments and types what it finds there, so the
// walk that types the body must read the same operands: one list for both.
// Read without them, `kept` is unknown where it is bound and nothing releases
// the `Tag`s it holds.

pub struct Tag {
    pub n: u64,
}

impl Tag {
    pub fn new(n: u64) -> Tag {
        Tag { n }
    }
}

pub fn logged() -> usize {
    let mut kept = Vec::new();
    tracing::info!("kept {}", {
        kept.push(Tag::new(1));
        kept.len()
    });
    kept.len()
}

pub fn printed() -> usize {
    let mut held = Vec::new();
    print!("{}", {
        held.push(Tag::new(2));
        held.len()
    });
    held.len()
}

pub fn run() -> usize {
    logged() + printed()
}
