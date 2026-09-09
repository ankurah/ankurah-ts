// A call goes through the helper that tells a plain function from an
// OwnedClosure, and the helper has to be handed the CALLABLE.
//
// Handed the holder, `invokeRef(self.call)` called an Arc, which is not a
// function. A callee an expression produced is also the statement's to release,
// and a boxed `FnOnce` is consumed by the call that runs it.

use std::sync::Arc;

pub struct Tag {
    pub n: u64,
}

impl Tag {
    pub fn new(n: u64) -> Tag {
        Tag { n }
    }
}

pub struct Holder {
    pub call: Arc<dyn Fn(u64) -> u64 + Send + Sync>,
}

impl Holder {
    pub fn new(call: Arc<dyn Fn(u64) -> u64 + Send + Sync>) -> Holder {
        Holder { call }
    }

    pub fn run(&self, n: u64) -> u64 {
        (self.call)(n)
    }
}

pub fn through_a_holder() -> u64 {
    let tag = Tag::new(3);
    let holder = Holder::new(Arc::new(move |n| n + tag.n));
    holder.run(4)
}

pub fn make_box() -> Box<dyn Fn(u64) -> u64> {
    let tag = Tag::new(2);
    Box::new(move |n| n + tag.n)
}

pub fn from_an_expression() -> u64 {
    (make_box())(5)
}

pub fn make_once() -> Box<dyn FnOnce() -> u64> {
    let tag = Tag::new(9);
    Box::new(move || tag.n)
}

pub fn consumed_once() -> u64 {
    (make_once())()
}
