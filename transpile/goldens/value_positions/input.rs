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
