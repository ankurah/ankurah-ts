//! What an arm's body IS, and whether it leaves — the two facts the lowering
//! now carries beside the text it wrote (K2).
//!
//! Three heuristics used to answer them by reading the rendered TypeScript
//! back, and each was wrong about a shape the corpus writes:
//!
//! - `is_statements` called a body a run of statements when it held `";\n"`.
//!   A nested match written as an if-chain does, so no `return` went in front
//!   of either branch and the arm answered `undefined` —
//!   `ankql::ast::Expr::populate_recursive`'s `Placeholder` arm, whose type is
//!   `Result<Expr, ParseError>`, is that shape exactly.
//! - `leaves_the_arm` read the text backwards for a `return` on the last line
//!   that was not a closing brace. An arm ending in a CONDITIONAL jump has one
//!   there, so the chain wrote no jump over the arms below it and a second arm
//!   naming the same variant ran as well.
//! - The same reading decided whether a guarded consuming arm had handed its
//!   value back.

pub struct Token {
    pub n: u32,
}

impl Token {
    pub fn new(n: u32) -> Token {
        Token { n }
    }
}

pub enum Source {
    Given(u32),
    Absent,
}

pub enum Answer {
    Number(u32),
    Missing,
}

/// The `populate_recursive` shape: a nested match IS the arm's value.
///
/// Written as bare statements the arm handed back `undefined`, and the caller
/// read `undefined.n`.
pub fn resolve(source: Source, fallback: Option<u32>) -> Answer {
    match source {
        Source::Given(n) => Answer::Number(n),
        Source::Absent => match fallback {
            Some(n) => Answer::Number(n),
            None => Answer::Missing,
        },
    }
}

/// The same shape one level deeper: the nested match's own arm is a nested
/// match, and every one of them owes the value back.
pub fn resolve_twice(source: Source, fallback: Option<u32>, floor: Option<u32>) -> Answer {
    match source {
        Source::Given(n) => Answer::Number(n),
        Source::Absent => match fallback {
            Some(n) => Answer::Number(n),
            None => match floor {
                Some(n) => Answer::Number(n),
                None => Answer::Missing,
            },
        },
    }
}

pub enum Weight {
    Light(u32),
    Heavy(u32),
}

/// A chain arm whose body ends in a CONDITIONAL jump. The guarded arm and the
/// arm below it name the same variant, so they are written as one chain, and
/// an arm that ran has to stop the arm below it running. `Light(5)` takes the
/// guarded arm, whose `if` does not fire — and used to fall into `Light(_)` as
/// well, pushing 0 after it.
pub fn record(w: &Weight, into: &mut Vec<u32>) -> u32 {
    match w {
        Weight::Light(n) if *n > 3 => {
            into.push(*n);
            if *n > 100 {
                return 1;
            }
        }
        Weight::Light(_) => {
            into.push(0);
        }
        Weight::Heavy(n) => {
            into.push(*n);
        }
    }
    0
}

/// A guarded CONSUMING arm whose value is what the function answers. The arm
/// takes the payload apart, so it cannot be written as the if-chain; the value
/// still has to come back out of the arrow.
pub fn weigh(input: Weight, floor: u32) -> u32 {
    match input {
        Weight::Light(n) if n > floor => n,
        Weight::Light(n) => floor,
        Weight::Heavy(n) => n * 2,
    }
}

/// A guarded consuming arm over a DROPPABLE payload, whose body is a nested
/// match. Both halves at once: the arm owes the payload a release on every
/// path, and the nested match owes the value.
pub fn tally(input: Source, token: Token, floor: Option<u32>) -> u32 {
    let answer = match input {
        Source::Given(n) if n > 0 => n,
        Source::Given(_) => match floor {
            Some(n) => n,
            None => 0,
        },
        Source::Absent => 0,
    };
    let total = answer + token.n;
    drop(token);
    total
}

pub enum Holder {
    One(Token),
    Two(Token),
}

/// A CONSUMING match whose arms hand a value back through `intoMatch`, which
/// runs each arm as an arrow function — so the value has to come back out of
/// that arrow. The `Two` arm's body is a nested match, written as bare
/// statements, and the arm answered `undefined`. `One` is the guarded shape
/// beside it: two arms name it, so they are one chain, the payload is
/// droppable, and the arm owes it a release on every path.
pub fn pick(input: Holder, floor: u32) -> u32 {
    match input {
        Holder::One(t) if t.n > floor => {
            if t.n > 100 {
                100
            } else {
                t.n
            }
        }
        Holder::One(_) => floor,
        Holder::Two(t) => match floor {
            0 => t.n,
            _ => floor,
        },
    }
}
