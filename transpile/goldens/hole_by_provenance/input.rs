//! X3/W9: "this value IS a hole" is answered by the LOWERING, not by the
//! characters the lowering happened to write.
//!
//! `hole_text` is the one place a hole is made, and it counts. Asked of the
//! rendered text alone, a user function named `unsupported` answered yes: `let
//! n = unsupported("x")?;` on a valid program lost the null test its `Option`
//! needs and went on to add 1 to `null`. Asked of the counter first, the
//! question is provenance — a call to a user function makes no hole — and the
//! text is only consulted where a hole really was written.
//!
//! And where one was, it may be WRAPPED: the port puts parentheses round a
//! value the position needs parenthesised and an `await` in front of one it has
//! to await. The text test now looks under both, so the `?` above a hole writes
//! no wrapper test below a throw. That half has no driver, because the code it
//! removes stands BELOW a throw and nothing can reach it; it is the unit tests
//! in `src/body/holes.rs` that hold it. This golden holds the half a program
//! can run.

/// A user function whose name is the port's own hole spelling. Rust allows it,
/// so the port has to.
pub fn unsupported(label: &str) -> Option<u32> {
    match label {
        "missing" => None,
        _ => Some(3),
    }
}

/// The `?` here is a real one: it leaves with `None` where the callee answers
/// `None`, and the port must keep the test that does it. The argument is a
/// string LITERAL, because that is what makes the rendered call read
/// `unsupported('..')` — exactly the characters a hole is spelled with.
pub fn asked_missing() -> Option<u32> {
    let n = unsupported("missing")?;
    Some(n + 1)
}

/// The same call with a label the callee answers, so the `?` does not leave.
pub fn asked_present() -> Option<u32> {
    let n = unsupported("present")?;
    Some(n + 1)
}
