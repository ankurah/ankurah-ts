//! A `move` closure owns every local its body names, including one it names
//! only inside a macro.
//!
//! The capture scan walks expressions and a macro's arguments are tokens, so a
//! capture named only there was invisible: the closure came out a bare arrow
//! owning nothing, and the block released what it had given away.

use std::sync::Arc;

pub struct Name {
    pub text: String,
}

pub struct Greeter {
    pub f: Box<dyn Fn() -> String>,
}

impl Greeter {
    pub fn new<F: Fn() -> String + 'static>(f: F) -> Greeter {
        Greeter { f: Box::new(f) }
    }

    pub fn greet(&self) -> String {
        (self.f)()
    }
}

/// The block hands both locals to the closure, which names them only inside
/// `format!`.
pub fn greeter(first_text: String, last_text: String) -> Greeter {
    let held = {
        let first = Arc::new(Name { text: first_text });
        let last = Arc::new(Name { text: last_text });
        Greeter::new(move || format!("{} {}", first.text, last.text))
    };
    held
}
