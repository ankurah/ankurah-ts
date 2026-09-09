// A candidate that applies only IF something nobody has decided holds ranks
// below one that applies outright, at every depth of the deref chain.
//
// `impl<T: Red> Ext for Wrap<T>` answers to `go` at depth 0 while `T` is still
// an unknown, and `Inner::go` answers one `Deref` away conditional on nothing.
// Picking the first answers a question the engine never closed, and here it is
// the wrong answer: `B` is not `Red`.

pub trait Red {
    fn red(&self) -> u32;
}

pub struct B;

pub struct Inner;

impl Inner {
    pub fn go(&self) -> u32 {
        20
    }
}

pub struct Wrap<T> {
    pub held: Vec<T>,
    pub inner: Inner,
}

impl<T> Wrap<T> {
    pub fn new() -> Wrap<T> {
        Wrap {
            held: Vec::new(),
            inner: Inner,
        }
    }

    pub fn set(&mut self, value: T) {
        self.held.push(value);
    }
}

impl<T> std::ops::Deref for Wrap<T> {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.inner
    }
}

pub trait Ext {
    fn go(&self) -> u32;
}

impl<T: Red> Ext for Wrap<T> {
    fn go(&self) -> u32 {
        99
    }
}

pub fn through_the_deref() -> u32 {
    let mut w = Wrap::new();
    w.set(B);
    w.go()
}
