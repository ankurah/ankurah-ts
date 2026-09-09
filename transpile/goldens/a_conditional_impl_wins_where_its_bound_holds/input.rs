// While a wrapper's element is an unknown, which method `w.go()` calls is
// whatever the body settles that element to: the extension impl at depth 0
// where the element is `Red`, and `Inner::go` one `Deref` away where it is not.
// Choosing by depth answers a question the engine has not closed, and here it
// answers it wrongly.

pub trait Red {
    fn red(&self) -> u32;
}

pub struct R;

impl Red for R {
    fn red(&self) -> u32 {
        1
    }
}

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

pub fn through_the_bound() -> u32 {
    let mut w = Wrap::new();
    w.set(R);
    w.go()
}
