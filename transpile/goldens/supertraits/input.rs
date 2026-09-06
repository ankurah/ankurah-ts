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
