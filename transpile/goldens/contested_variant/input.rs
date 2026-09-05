// A variant SEVERAL arms name is one Rust tries in ORDER, testing the patterns
// inside the payload; the runtime's `.match({..})` has one key per variant and
// dispatches on the variant name alone, so the first of those arms ran for
// every value of the variant. `Expr::Literal(Literal::Bool(true)) => Ok(True)`
// beside `..(false) => Ok(False)` answered `True` for `false`, which is how
// ankql's `Predicate::try_from` turned `FALSE` into `TRUE`; and
// `Poll::Ready(Some(item))` beside `Poll::Ready(None)` read the end of a stream
// as an item.
//
// One key holds the chain those arms describe: each arm's inner pattern is the
// test, and what no arm matched falls through to the catch-all's body — or,
// where the match has no catch-all, to the last arm, which rustc proved the
// value must match. The payload arrives once, as the key's parameter, and
// whichever branch runs is what settles it.

pub struct Payload {
    pub n: u32,
}

pub enum Lit {
    Flag(bool),
    Count(u32),
}

pub enum Expr {
    Literal(Lit),
    Held(Payload),
    Nothing,
}

/// Two arms name `Literal` and a catch-all stands below them.
pub fn truthy(e: Expr) -> Result<bool, String> {
    match e {
        Expr::Literal(Lit::Flag(true)) => Ok(true),
        Expr::Literal(Lit::Flag(false)) => Ok(false),
        _ => Err("not a flag".to_string()),
    }
}

pub enum Step {
    Ready(Option<Payload>),
    Pending,
}

/// Two arms name `Ready` and there is no catch-all: rustc proved `Some` and
/// `None` exhaustive between them, so what fails the first test matches the
/// second. The subject is MOVED in, so the branch that runs settles the
/// payload.
pub fn take_one(step: Step, into: &mut Vec<u32>) -> bool {
    match step {
        Step::Ready(Some(item)) => {
            into.push(item.n);
            true
        }
        Step::Ready(None) => false,
        Step::Pending => false,
    }
}

/// The subject is BORROWED, so the enum stays whole and the arms only read it.
pub fn describe(e: &Expr) -> String {
    match e {
        Expr::Literal(Lit::Flag(_)) => "flag".to_string(),
        Expr::Literal(Lit::Count(_)) => "count".to_string(),
        Expr::Held(_) => "held".to_string(),
        Expr::Nothing => "nothing".to_string(),
    }
}

fn width(e: &Expr) -> Result<u32, String> {
    match e {
        Expr::Literal(Lit::Count(n)) => Ok(*n),
        _ => Err("no width".to_string()),
    }
}

/// A `?` and an early `return` inside two links of one chain: both leave the
/// FUNCTION, not the branch they stand in.
pub fn widen(e: Expr, source: &Expr) -> Result<u32, String> {
    match e {
        Expr::Literal(Lit::Flag(true)) => {
            let n = width(source)?;
            Ok(n + 1)
        }
        Expr::Literal(Lit::Flag(false)) => {
            return Err("false".to_string());
        }
        Expr::Literal(Lit::Count(n)) => Ok(n),
        _ => Err("no".to_string()),
    }
}

/// The subject is a CALL, which Rust evaluates once. Each turn takes one item
/// and the end of the queue leaves the loop.
fn next_step(items: &mut Vec<Payload>) -> Step {
    match items.pop() {
        Some(p) => Step::Ready(Some(p)),
        None => Step::Ready(None),
    }
}

pub fn drain(items: &mut Vec<Payload>, into: &mut Vec<u32>) -> u32 {
    let mut turns = 0u32;
    loop {
        match next_step(items) {
            Step::Ready(Some(item)) => {
                into.push(item.n);
                turns += 1;
            }
            Step::Ready(None) => break,
            Step::Pending => break,
        }
    }
    turns
}

pub trait Widths {
    fn width_of(&self) -> u32;
}

/// A chain inside a trait impl, over a BORROWED receiver.
impl Widths for Expr {
    fn width_of(&self) -> u32 {
        match self {
            Expr::Literal(Lit::Count(n)) => *n,
            Expr::Literal(Lit::Flag(_)) => 1,
            Expr::Held(p) => p.n,
            Expr::Nothing => 0,
        }
    }
}

/// A chain in a GENERIC function, where the subject's type is the parameter's.
pub fn widest<W: Widths>(a: &W, b: &W) -> u32 {
    if a.width_of() >= b.width_of() {
        a.width_of()
    } else {
        b.width_of()
    }
}
