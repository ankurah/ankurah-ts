// The frame's-until-invoked rule, asked ACROSS the statements of a block, and
// at a transfer the lift cannot reorder.
//
// A move written straight-line is on every path — until something ABOVE it can
// leave the block. The port already knew that of a `return`, a `?`, a `break`
// and a `continue`; it did not know it of a statement that THROWS. So
// `let _n = o.unwrap(); [a, b]` was "a and b are gone on every path", the block
// wrote no release for either, and a `None` left both of them handed to nobody
// — which Rust drops while it unwinds.
//
// The second half is where the flag goes. A move nested inside a tuple or array
// ELEMENT is performed by that element's own evaluation, so it cannot be lifted
// above the flag; the flag has to be written AT the transfer instead. Where the
// statement is one the port REFUSED, nothing wrote it at all, and the frame's
// guarded release was then dropped as a flag nothing sets.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub fn take(t: Token) -> i64 {
    t.n
}

pub fn fallible(ok: bool) -> Result<i64, String> {
    match ok {
        true => Ok(1),
        false => Err(String::from("no")),
    }
}

/// A statement that throws, above a statement that moves.
pub fn throws_above_a_move(a: Token, b: Token, o: Option<i64>) -> [Token; 2] {
    let _n = o.unwrap();
    [a, b]
}

/// The same with one value, and the throw inside a `let`.
pub fn throws_above_one_move(a: Token, o: Option<i64>) -> Token {
    let _n = o.unwrap();
    a
}

/// A move nested in an ARRAY element, with a throwing element before it: the
/// early return leaves the frame still holding both tokens.
pub fn a_throwing_element_before_the_move(
    a: Token,
    b: Token,
    ok: bool,
) -> Result<[i64; 2], String> {
    Ok([fallible(ok)?, take(a) + take(b)])
}

/// And a value the statement that throws BINDS is not the frame's on that path,
/// so nothing releases it: there is no `t` yet when `Token::new` throws.
pub fn the_binding_of_the_throwing_statement(n: i64) -> i64 {
    let t = Token { n };
    take(t)
}
