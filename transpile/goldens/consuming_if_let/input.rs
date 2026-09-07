//! Y3: `if let PAT = e { A } else { B }` where PAT takes the payload OUT is
//! `match e { PAT => A, _ => B }`, which is Rust's own desugaring of it.
//!
//! Such a pattern CONSUMES the value it tests: Rust binds what the pattern
//! names, drops the fields it did not name, and the value the `if let` read is
//! gone on both paths. The port has one construct that does that — `intoMatch`,
//! which marks the enum moved and hands the arm every part of the payload — and
//! it belongs to the match writer.
//!
//! Written as an `if`, `if let Predicate::Comparison { right: val, .. } =
//! *inner_left` read `val` out of the enum, released it, and then let the block
//! release `inner_left` as well, with `val` inside it — a double drop, reported
//! as "Predicate was used after being moved" — while `left` and `operator`,
//! which Rust drops where the pattern moves out, were released by nobody. Five
//! of ankql's nine `ast` tests died on that pair.
//!
//! `*node` is the other half: Rust's deref-move takes the value out of the box
//! and the box goes with it. A `Deref` through a `MutexGuard` is NOT that — it
//! borrows — which is why the rule asks the type rather than the syntax.

pub struct Op {
    pub n: u32,
}

pub struct Leaf {
    pub n: u32,
}

pub enum Node {
    Pair { left: Box<Node>, op: Op, right: Box<Node> },
    End(Leaf),
}

impl Node {
    pub fn leaf(n: u32) -> Node {
        Node::End(Leaf { n })
    }
}

/// The corpus shape: a struct variant, a `..` leaving two fields unnamed, and a
/// `Box` the pattern moves out of.
pub fn right_leaf(node: Box<Node>) -> u32 {
    if let Node::Pair { right: val, .. } = *node {
        return depth(*val);
    }
    0
}

/// How deep the tree is, so the golden has something to consume the binding
/// with rather than dropping it where it stands.
pub fn depth(node: Node) -> u32 {
    match node {
        Node::Pair { left, op, right } => op.n + depth(*left) + depth(*right),
        Node::End(leaf) => leaf.n,
    }
}

/// The path where the pattern does NOT match: Rust drops the value the `if let`
/// read, and so does the wildcard arm.
pub fn only_pairs(node: Box<Node>) -> u32 {
    if let Node::Pair { op, .. } = *node {
        return op.n;
    }
    7
}

/// A field taken out of a value the port holds in a TEMPORARY. Rust's temporary
/// knows the field is gone; the port's `_tN` release cascades into it unless the
/// read comes out through `takeField`.
#[derive(Clone)]
pub struct Inner {
    pub n: u32,
}

#[derive(Clone)]
pub struct Holder {
    pub inner: Inner,
    pub tag: u32,
}

pub fn eat(i: Inner) -> u32 {
    i.n
}

pub fn from_a_clone(h: Holder) -> u32 {
    let taken = h.clone().inner;
    eat(taken) + h.tag
}
