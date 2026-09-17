// What a statement throws away after awaiting it.
//
// A call to a declared `async fn` answers what it writes (spec 4.10), so
// awaiting one is the identity — and a statement that discards the result
// still has to release it. Read as "the engine could not say what awaiting
// this produces", the `Held` was collected undropped.

pub struct Held {
    pub n: u64,
}

impl Held {
    pub fn new(n: u64) -> Held {
        Held { n }
    }
}

pub async fn make_held(n: u64) -> Held {
    Held::new(n)
}

pub async fn discards_what_it_awaited() -> u64 {
    make_held(1).await;
    make_held(2).await.n
}
