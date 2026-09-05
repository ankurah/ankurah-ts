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

/// A `Display` that COMPOSES: several writes in sequence, the last of them the
/// method's tail. Only `write!(..)?;` used to append, so the semicolon form was
/// an unused string expression and the tail write replaced everything written
/// before it — `write!(f, "a")?; write!(f, "b")` answered `"b"`.
pub struct Parts {
    pub head: String,
    pub tail: String,
}

impl std::fmt::Display for Parts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        write!(f, "{}", self.head)?;
        write!(f, "|{}", self.tail)?;
        write!(f, "]")
    }
}

/// The same, ending in `Ok(())` rather than in a write — which is the value the
/// accumulator stands for.
pub struct Lines {
    pub first: String,
}

impl std::fmt::Display for Lines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.first)?;
        write!(f, "end");
        Ok(())
    }
}

/// A placeholder naming an argument the call does not have. rustc refuses this,
/// so it can only reach the port through a macro the engine reads differently
/// from rustc — and it used to be written as the name itself, which the emitted
/// template reads and does not find.
pub fn absent(a: u32) -> String {
    format!("{0} {1}", a)
}

/// Every `write!` inside a `Display` APPENDS to what the formatter has composed,
/// in all the forms a source writes it: with and without `?`, with and without a
/// semicolon — and as `return write!(..)`, which appends and THEN answers.
/// Read as an ordinary `return`, that last one made the string it wrote the
/// whole answer and discarded everything written before it: `Size(200)` printed
/// as `big)` where Rust prints `Size(big)`.
pub struct Size(pub u32);

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Size(")?;
        if self.0 > 100 {
            return write!(f, "big)");
        }
        write!(f, "{})", self.0)
    }
}
