// A callable reached through a holder, and one called by a bare name.
//
// `Arc<F>` where the signature wrote `F: Fn(..)` holds the same value
// `Arc<dyn Fn(..)>` does, so the call goes through the invoke helper and the
// argument takes the type the callable declares. Called directly, the Arc is
// not a function; written without the declared input, `4` is not `4u64`.

use std::sync::Arc;

pub struct Doubler<F: Fn(u64) -> u64> {
    pub inner: Arc<F>,
}

impl<F: Fn(u64) -> u64> Doubler<F> {
    pub fn new(inner: F) -> Doubler<F> {
        Doubler { inner: Arc::new(inner) }
    }

    pub fn apply(&self, n: u64) -> u64 {
        (self.inner)(n)
    }
}

pub fn through_a_bare_name(f: Arc<dyn Fn(u64) -> u64>) -> u64 {
    f(4)
}

pub fn held_behind_a_bound() -> u64 {
    let doubler = Doubler::new(|n: u64| n * 2);
    doubler.apply(21)
}
