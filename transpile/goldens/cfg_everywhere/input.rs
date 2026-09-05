//! `#[cfg]` decides more than a top-level item.
//!
//! Rust drops a struct field, an enum variant, an impl method, a statement, a
//! `let`, a match arm and a field of a struct literal on exactly the same
//! evidence it drops a whole `fn` on. The emitter used to ask only about
//! top-level items, so a `#[cfg(debug_assertions)]` field stayed in the
//! constructor and BOTH branches of a `debug_assertions`/`not(debug_assertions)`
//! pair were emitted — the shadowing rename gave the second one a fresh name, so
//! the file still compiled and the code below it read the RELEASE branch. That
//! inverted the `debug_assertions = true` ruling in the one place it was made
//! for, `storage/indexeddb-wasm/src/collection.rs`.
//!
//! This build has `debug_assertions` true, so every `#[cfg(debug_assertions)]`
//! here is in and every `#[cfg(not(debug_assertions))]` is out.

pub struct Bucket {
    pub prefix_len: u32,
    #[cfg(debug_assertions)]
    pub guard_disabled: bool,
    #[cfg(not(debug_assertions))]
    pub never_here: u32,
}

pub enum Mode {
    Fast,
    #[cfg(debug_assertions)]
    Checked,
    #[cfg(not(debug_assertions))]
    NeverHere,
}

impl Bucket {
    pub fn new(prefix_len: u32) -> Bucket {
        Bucket {
            prefix_len,
            #[cfg(debug_assertions)]
            guard_disabled: false,
            #[cfg(not(debug_assertions))]
            never_here: 0,
        }
    }

    /// The pair the ruling is about: two `let`s of one name, one per build.
    pub fn effective(&self, open_ended: bool) -> u32 {
        #[cfg(debug_assertions)]
        let effective = if open_ended && self.prefix_len > 0 && !self.guard_disabled { self.prefix_len } else { 0 };
        #[cfg(not(debug_assertions))]
        let effective = if open_ended && self.prefix_len > 0 { self.prefix_len } else { 0 };
        effective
    }

    #[cfg(debug_assertions)]
    pub fn checked(&self) -> u32 {
        self.prefix_len + 1
    }

    #[cfg(not(debug_assertions))]
    pub fn checked(&self) -> u32 {
        0
    }

    pub fn describe(&self, mode: Mode) -> u32 {
        match mode {
            Mode::Fast => 0,
            #[cfg(debug_assertions)]
            Mode::Checked => 1,
            #[cfg(not(debug_assertions))]
            Mode::NeverHere => 2,
        }
    }
}
