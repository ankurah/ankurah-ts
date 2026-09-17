// A literal whose width only a later use decides.
//
// The arithmetic helper is told the width the solve settled, so the literal
// beside it has to be written at that width too: a JavaScript number handed to
// `checkedAdd(x, 1n, 'u64')` matches neither overload, and JavaScript refuses
// to mix a number with a bigint at run time.

pub fn take_u64(v: u64) -> u64 {
    v
}

pub fn settled_later() -> u64 {
    let x = 3;
    take_u64(x);
    x + 1
}

pub fn settled_by_a_return() -> u64 {
    let y = 5;
    y
}
