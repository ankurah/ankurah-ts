//! A name declared ABOVE a body, and a `let` inside it that shadows it.
//!
//! A function's parameters, a closure's, and the names an arm's pattern bound
//! are all declared above the body they are read in. Every move site in that
//! body used to be attributed to them, whatever stood between — so a top-level
//! `let` of the same name handed the outer value's release to the shadow's
//! move, and the outer value was released by nobody.
//!
//! The closure below is the second half: what `f` is at `apply`'s second
//! parameter is said only by `F`'s bound, which belongs to `apply`'s own
//! generics — and `F` is still an open parameter at the call, so the closure
//! was typed by nothing at all and its parameter released nothing.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub fn apply<F>(t: Token, f: F) -> i64
where
    F: Fn(Token) -> i64,
{
    f(t)
}

/// The parameter is shadowed, and the SHADOW is what `drop` takes.
pub fn shadow_moved(token: Token) -> i64 {
    let token = Token { n: 2 };
    drop(token);
    1
}

/// The shadow is RETURNED, which is the move the outer name least deserves to
/// be credited with.
pub fn shadow_returned(t: Token) -> Token {
    let t = Token { n: 9 };
    t
}

/// A closure's parameter is declared above its body in the same way — and it
/// only has a type at all because `apply`'s bound says so.
pub fn shadow_in_a_closure() -> i64 {
    apply(Token { n: 1 }, |token| {
        let token = Token { n: 2 };
        drop(token);
        1
    })
}

/// And so are the names an arm's pattern bound.
pub fn shadow_in_an_arm() -> i64 {
    let pair = (Token { n: 1 }, Token { n: 2 });
    match pair {
        (a, b) => {
            let a = Token { n: 3 };
            drop(a);
            b.n
        }
    }
}
