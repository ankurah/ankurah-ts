//! A module-level `const` and `static`, and a struct literal that names its
//! fields out of declaration order.
//!
//! `ConstInfo` used to carry the const's TYPE and never its value, so every
//! module-level const came out `undefined as any` — including `human_id`'s word
//! list, which `humanize` indexes, and the tag byte every JSON value in an
//! index key is written with. `static` had no arm in the item walk at all, so
//! the item vanished and every use of it named nothing.
//!
//! And a struct literal was emitted POSITIONALLY in the order the literal
//! happened to write its fields, while the emitted constructor takes them in
//! DECLARATION order. Two fields of one type swap in silence, which is what
//! `connectors/local-process/src/lib.rs:70` does with its two `EntityId`s.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub const TAG_NULL: u8 = 0x00;
pub const TAG_STRING: u8 = 0x04;
pub const WORDS: &[&str; 3] = &["ack", "alabama", "alanine"];
pub static SYSTEM_COLLECTION: &str = "_ankurah_system";
const SHIFT: u64 = 1 << 40;

pub struct Rec {
    pub first: u32,
    pub second: String,
    pub third: bool,
}

impl Rec {
    /// Written third, first, second — the constructor takes first, second, third.
    pub fn make(a: u32, b: String, c: bool) -> Rec {
        Self { third: c, first: a, second: b }
    }

    pub fn tag(&self) -> u8 {
        if self.third { TAG_STRING } else { TAG_NULL }
    }
}

pub fn word(index: usize) -> String {
    WORDS[index].to_string()
}

pub fn collection() -> String {
    SYSTEM_COLLECTION.to_string()
}

pub fn shifted() -> u64 {
    SHIFT
}

// A `const` of a non-Copy type is INLINED at each use, so each use is its own
// value: `let mut a = ORIGIN; a.x = 9;` mutates nobody else's. Bound to one
// module object, two uses shared an identity, a mutation and a release.
#[derive(Clone)]
pub struct Point {
    pub x: u32,
    pub y: String,
}

pub const ORIGIN: Point = Point { x: 0, y: String::new() };

// A `static` whose type carries interior mutability is written THROUGH, and an
// atomic is its value here — so the binding has to be reassignable.
pub static COUNTER: AtomicUsize = AtomicUsize::new(0);
pub static READY: AtomicBool = AtomicBool::new(false);

// A negated literal in a const initialiser is one literal in Rust and two
// tokens here; the width belongs to the literal.
pub const FLOOR: i64 = -9007199254740991;

pub const BASE: u32 = 36;

pub fn moved_origin() -> u32 {
    let mut first = ORIGIN;
    first.x = 9;
    let second = ORIGIN;
    first.x + second.x
}

pub fn bump() -> usize {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

pub fn arm(ready: bool) -> bool {
    READY.store(ready, Ordering::SeqCst);
    READY.load(Ordering::SeqCst)
}

// A `const` in a pattern is a comparison against its value, not a binding.
pub fn radix(n: u32) -> u32 {
    match n {
        BASE => 1,
        0 => 2,
        _ => 3,
    }
}

// J: Rust's items are order-independent; JavaScript's `const` is not. `const
// LATE = EARLY + 1;` written above `const EARLY = 1;` is
// `ReferenceError: Cannot access 'EARLY' before initialization` at module load,
// so the whole file fails to load and every import of it with it. The emitted
// consts are ordered by what their initialisers name.
pub const LATE: u32 = EARLY + 1;
pub const EARLY: u32 = 1;
pub const LATEST: u32 = LATE + EARLY;

pub fn ordered() -> u32 {
    LATEST
}

// K: Rust's atomics WRAP at their width whatever the build's debug assertions
// say — `AtomicU32::MAX.fetch_add(1)` stores `0` — and a `+=` on a `number`
// went on counting. A `static mut` beside it already went through the checked
// helper, so the two spellings of one idea disagreed.
pub static WRAPS: AtomicU32 = AtomicU32::new(u32::MAX);

pub fn wrap_around() -> u32 {
    WRAPS.fetch_add(1, Ordering::SeqCst)
}

// D5: a constant Rust puts on a PRIMITIVE type. The port writes the type as a
// JavaScript primitive, which has no members, so `f64::EPSILON` came out
// `f64.EPSILON` — a name the file never declares — and nothing typed the
// expression, so `.max()` on it fell through the number translations too and
// wrote a method a JavaScript number has not got. Live at
// `storage/indexeddb-wasm/src/planner_integration.rs:19`, which is this line.
pub fn epsilon_near(v: f64) -> f64 {
    f64::EPSILON.max(v.abs() * f64::EPSILON)
}

pub fn widths() -> (u32, i64, u64, f64, f64) {
    (u32::MAX, i64::MIN, u64::MAX, f64::INFINITY, f64::NAN)
}
