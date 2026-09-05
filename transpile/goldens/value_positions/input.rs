// Four expressions Rust reads as VALUES and TypeScript writes as statements.
// Each came out as the statement, in a place where a statement does not parse —
// or, worse, with a `return` the block translator added to a loop body's tail,
// which left the loop on its first turn.
pub enum Refusal {
    Empty,
}

fn checked(n: u32) -> Result<u32, Refusal> {
    if n == 0 {
        Err(Refusal::Empty)
    } else {
        Ok(n)
    }
}

// #14: a `loop` whose value the `let` wants.
pub fn first_even(mut n: u32) -> u32 {
    let found = loop {
        if n % 2 == 0 {
            break n;
        }
        n += 1;
    };
    found + 1
}

// P1: a block whose single statement is an `if` used as its value.
pub fn pick(ok: bool) -> u32 {
    let n = { if ok { 1 } else { 2 } };
    n + 1
}

// P2: a jump written where a ternary branch would stand.
pub fn until_zero(v: &Vec<u32>) -> u32 {
    let mut total = 0u32;
    for x in v {
        total += if *x == 0 { break } else { *x };
    }
    total
}

// The tail of a `for` body is not the function's value, and a `?` inside an arm
// of a match written there has to leave the FUNCTION.
pub fn total(v: &Vec<u32>) -> Result<u32, Refusal> {
    let mut sum = 0u32;
    for x in v {
        match x {
            0 => {
                sum += 1;
            }
            n => {
                sum += checked(*n)?;
            }
        }
    }
    Ok(sum)
}

// X8: an `if` used as an ordinary BINARY OPERAND. The narrower `total += if ..`
// case was already covered; this one came out `checkedAdd((if (yes) { return 1;
// } else { return 2; }), 3, 'u32')`, which a JavaScript engine refuses to parse.
// And a ternary written bare beside a comparison is swallowed by it: `a == if
// yes { 1 } else { 2 }` came out `a === yes ? 1 : 2`, which reads as
// `(a === yes) ? 1 : 2`.
pub fn operand(yes: bool) -> u32 {
    (if yes { 1 } else { 2 }) + 3
}

pub fn compared(a: u32, yes: bool) -> bool {
    a == if yes { 1 } else { 2 }
}

// X1: a labelled jump written BELOW an ordinary expression — here a call
// argument. The jump analysis stopped at every expression kind but blocks, ifs,
// matches and loops, so emission wrote `sink(break outer)`, which does not
// parse. Every child expression is visited now, and a jump in a value position
// travels out through the sentinel.
fn sink(n: u32) -> u32 {
    n
}

pub fn jump_in_an_argument(stop_at: u32) -> u32 {
    let mut total = 0u32;
    'outer: loop {
        total = sink(if total >= stop_at { break 'outer } else { total + 1 });
    }
    total
}

pub fn jump_in_a_block_argument(stop: bool) -> u32 {
    let mut total = 0u32;
    'outer: loop {
        if stop {
            total = sink({ break 'outer; });
        }
        total += 1;
    }
    total
}

// H: a value-position `loop` in TAIL position. The `let` form was already
// hoisted-and-labelled; the tail came out `break /* 9 */` and the function fell
// off the end returning `undefined`.
pub fn first_even_tail(mut n: u32) -> u32 {
    loop {
        if n % 2 == 0 {
            break n;
        }
        n += 1;
    }
}

/// A tail `loop` with no payload is still a statement whose value is `()`.
pub fn spin(mut n: u32) -> u32 {
    let mut seen = 0u32;
    loop {
        if n == 0 {
            break;
        }
        n -= 1;
        seen += 1;
    }
    seen
}
