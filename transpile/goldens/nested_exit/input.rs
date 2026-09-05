//! A `?` or a `return` written inside a match that stands as a STATEMENT within
//! another match's arm.
//!
//! An arm of `match` or `intoMatch` is an arrow function, so a `return` written
//! in one leaves the ARM. The arm hands its own exit back as a sentinel and the
//! statement after the match performs it — but a match written as a STATEMENT
//! inside such an arm has nowhere to hand anything: a statement's value is
//! discarded where it stands. So the sentinel is tested there too, and handed
//! ON whole, because the test outside is what turns it back into the function's
//! return.
//!
//! ankql's `generate_expr_sql` is written exactly this way, and every `Err` its
//! inner match produced was dropped where it stood: the function answered
//! `Ok(())` for an expression it could not render, and the `Err` and its
//! payload leaked.

pub struct Token {
    pub n: usize,
}

impl Token {
    pub fn new(n: usize) -> Token { Token { n } }
}

pub enum Inner {
    Good,
    Bad,
}

pub enum Outer {
    One(Token),
    Two,
}

/// The inner match's `return` has to leave `run`, not the arm — and the arm
/// still releases the payload it took on the way out.
pub fn run(outer: Outer, inner: &Inner, out: &mut String) -> Result<usize, String> {
    match outer {
        Outer::One(token) => {
            match inner {
                Inner::Good => {
                    out.push('g');
                }
                Inner::Bad => {
                    return Err("bad".to_string());
                }
            }
            out.push('1');
            let n = token.n;
            drop(token);
            return Ok(n);
        }
        Outer::Two => {
            out.push('2');
        }
    }
    Ok(0)
}
