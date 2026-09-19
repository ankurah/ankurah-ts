//! A trait method called on what a closure ANSWERS with.
//!
//! `predicate(v)` names no function: `predicate` is a parameter whose bound says
//! what calling it does, and the call answers that bound's output. Untyped, the
//! `met()` after it was written as a method on a value that carries none. The
//! bound is open, so the call goes through the trait's dispatcher.

pub trait Check {
    fn met(self) -> bool;
}

impl Check for bool {
    fn met(self) -> bool { self }
}

impl<T> Check for Option<T> {
    fn met(self) -> bool { self.is_some() }
}

pub fn any_met<F, R>(values: Vec<u64>, predicate: F) -> bool
where
    F: Fn(u64) -> R,
    R: Check,
{
    for value in values {
        if predicate(value).met() {
            return true;
        }
    }
    false
}
