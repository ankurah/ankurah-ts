//! A tuple struct's name handed to a combinator is a function that constructs.
//!
//! The port writes such a struct as a class, and a class called without `new`
//! is a `TypeError`, so the name alone could not be the value: `map(Wrapper)`
//! came out `(Wrapper)(x)`. As the CALLEE of a call the name is still the
//! construction itself, which is the control below.

pub struct Wrapper(pub u64);

pub struct Holder {
    pub one: Option<u64>,
    pub many: Vec<u64>,
}

impl Holder {
    /// Through `Option::map`.
    pub fn wrapped(&self) -> Option<Wrapper> {
        self.one.map(Wrapper)
    }

    /// Through an iterator adaptor.
    pub fn all(&self) -> Vec<Wrapper> {
        self.many.iter().copied().map(Wrapper).collect()
    }

    /// The control: written out, the name is the construction.
    pub fn spelled(&self) -> Option<Wrapper> {
        self.one.map(|n| Wrapper(n))
    }
}
