//! X5: a value moved into a call is the frame's until the call is INVOKED.
//!
//! `take2(token, o.unwrap())` moves `token` on every path the source has, so
//! the disposition read it as straight-line and the block wrote no release at
//! all — and `unwrap` on a `None` throws with the token handed to nobody, which
//! Rust drops while it unwinds. The same holds for a `?` standing in a later
//! field of the struct literal that moves the value: ankql's
//! `Predicate::Comparison { left: Box::new(left.populate_recursive(values)?),
//! operator, .. }` left `operator` released by nobody on the error path, which
//! was the last leak report in `bun test packages/ankql/`.
//!
//! So a move with anything after it that can throw is a move under a branch:
//! the block declares a flag, the operands still to be evaluated are lifted
//! above it, and the flag stands immediately before the call.
//!
//! A value the port writes as a LITERAL is not lifted: it builds nothing that
//! can throw, and lifting it takes it out of the position that types it —
//! `const _b2 = [];` is `any[]`, which `noImplicitAny` reports twice.

pub struct Token {
    pub n: u32,
}

pub struct Op {
    pub n: u32,
}

pub struct Oops;

pub fn take2(t: Token, n: u32) -> u32 {
    t.n + n
}

/// The argument after the move throws.
pub fn later_throws(t: Token, o: Option<u32>) -> u32 {
    take2(t, o.unwrap())
}

pub struct Pair {
    pub op: Op,
    pub items: Vec<u32>,
    pub n: u32,
}

pub fn fallible(fail: bool) -> Result<u32, Oops> {
    if fail {
        Err(Oops)
    } else {
        Ok(7)
    }
}

/// ankql's shape: the `?` in a later field leaves the frame before the earlier
/// field is handed over. `Vec::new()` between them is a literal in the port and
/// stays where the field types it.
pub fn field_after_a_question(op: Op, fail: bool) -> Result<Pair, Oops> {
    Ok(Pair { op, items: Vec::new(), n: fallible(fail)? })
}
