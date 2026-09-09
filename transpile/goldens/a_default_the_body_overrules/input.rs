// A type argument the source leaves off is an unknown the body settles, and the
// declaration's default is only what stands where the body said nothing.
//
// `Slot<A, B = ()>` built as `Slot::new(1, Thing::new(9))` holds a `Thing`:
// Rust reads `B` off the argument and never reaches the default. Applied
// eagerly, the default answered `Slot<u8, ()>`, and the `Thing` the slot holds
// was released by nobody.

pub struct Thing {
    pub id: u64,
}

impl Thing {
    pub fn new(id: u64) -> Thing {
        Thing { id }
    }
}

pub struct Slot<A, B = ()> {
    pub a: A,
    pub b: B,
}

impl<A, B> Slot<A, B> {
    pub fn new(a: A, b: B) -> Slot<A, B> {
        Slot { a, b }
    }
}

pub fn held() -> u64 {
    let s = Slot::new(1u8, Thing::new(9));
    s.b.id + s.a as u64
}

pub fn wrapped() -> u8 {
    let s = Slot::new(1u8, 200u8);
    s.b.wrapping_add(100)
}

pub fn empty() -> u8 {
    let s: Slot<u8> = Slot::new(2u8, ());
    s.a
}

pub fn run() -> u64 {
    held() + wrapped() as u64 + empty() as u64
}
