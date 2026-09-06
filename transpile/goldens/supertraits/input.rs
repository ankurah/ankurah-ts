//! I8: a supertrait is part of what an implementor promises.

pub trait Tell {
    fn tell(&self) -> u32;
}

pub trait Super: Tell {}

// `Loud` has a default body, so the port writes it as an abstract CLASS —
// which cannot extend an interface. Its supertraits go on an interface of the
// same name, which TypeScript merges into the class type, so `this.tell()`
// inside the default body has a declaration. No struct implements it here: an
// implementor of a trait with default bodies has to EXTEND the abstract class
// to inherit them, and the port writes `implements` — a pre-existing gap of its
// own, and not this shape's.
pub trait Loud: Tell + Sized {
    fn shout(&self) -> u32 { self.tell() * 2 }
}

/// The call resolves through the bound; `tsc` reports TS2339 on `T` unless the
/// emitted `Super` says it is a `Tell`.
pub fn ask<T: Super>(t: &T) -> u32 { t.tell() }

pub struct One;
impl Tell for One { fn tell(&self) -> u32 { 1 } }
impl Super for One {}


pub mod nested {
    pub trait Buried<T> {
        fn buried(&self) -> T;
    }
}

/// S8: the supertrait is written QUALIFIED and with an argument. Keeping only
/// the last segment and resolving that bare name HERE answered "no" — nothing
/// brings `Buried` into this scope — so the port wrote `export interface Deep
/// {}` with the inherited method gone and an unused import above it. The whole
/// path is what the lookup takes now.
pub trait Deep: nested::Buried<u32> {
    fn deep(&self) -> u32;
}

pub struct Two;
impl nested::Buried<u32> for Two {
    fn buried(&self) -> u32 { 7 }
}
impl Deep for Two {
    fn deep(&self) -> u32 { 1 }
}

/// The call resolves through the bound, so `tsc` reports TS2339 on `T` unless
/// the emitted interface says `extends Buried<number>`.
pub fn dig<T: Deep>(t: &T) -> u32 {
    t.buried() + t.deep()
}
