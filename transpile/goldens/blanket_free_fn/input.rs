//! An impl does not need its self type to be a struct this crate declared. One
//! written for a bare type parameter, or for a std wrapper, has no TypeScript
//! class to be a method on, so its methods are emitted as module-level
//! functions taking the receiver first.

use std::sync::Arc;

pub struct Listener {
    pub tag: u8,
}

pub struct Inner {
    pub tag: u8,
}

pub trait IntoListener {
    fn into_listener(self) -> Listener;
}

/// Written for a bare parameter of its own, so there is no constructor to name
/// and the function takes the method's name alone.
impl<F> IntoListener for F
where
    F: Fn(u8) -> u8,
{
    fn into_listener(self) -> Listener {
        Listener { tag: self(1) }
    }
}

/// Written for a std wrapper, so the function is named after the wrapper's
/// constructors from the outside in.
impl IntoListener for Arc<Inner> {
    fn into_listener(self) -> Listener {
        Listener { tag: self.tag }
    }
}

/// A call whose receiver the engine can name reaches the right function.
pub fn from_wrapped(inner: Arc<Inner>) -> Listener {
    inner.into_listener()
}

/// A call through a bound the engine cannot close: several impls satisfy it and
/// the blanket one is what is written, which the site reports.
pub fn from_any<L: IntoListener>(listener: L) -> Listener {
    listener.into_listener()
}
