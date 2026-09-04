//! One format-string translator, used by every formatting macro.
//!
//! `{}` prints through `Display`, which the port writes as the value itself or
//! its `toString`; `{:?}` prints through `Debug`, which is the `debug()` the
//! derive writes. A placeholder names its argument by position, by name, or —
//! since Rust 2021 — by naming a variable directly. What the port cannot carry
//! over is reported at the site rather than written wrong.

#[derive(Debug)]
pub struct Peer {
    pub id: u32,
    pub name: String,
}

pub fn greeting(peer: &Peer) -> String {
    format!("hello {}", peer.name)
}

pub fn positional(a: u32, b: u32) -> String {
    format!("{0} then {1}, and {0} again", a, b)
}

pub fn named(peer: &Peer) -> String {
    format!("{who} is {id}", who = peer.name, id = peer.id)
}

/// Rust 2021 captures a variable a placeholder names.
pub fn captured(name: String) -> String {
    format!("captured {name}")
}

/// `{:?}` goes through the type's `Debug`.
pub fn debugged(peer: &Peer) -> String {
    format!("peer {:?}", peer)
}

/// Escaped braces stay text.
pub fn braces(n: u32) -> String {
    format!("{{{}}}", n)
}

/// `write!` into a formatter is what the `Display` impl returns.
impl std::fmt::Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}#{}", self.name, self.id)
    }
}

/// `panic!` carries the same rendering.
pub fn refuse(n: u32) -> u32 {
    if n == 0 {
        panic!("refusing {}", n);
    }
    n
}
