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
