// What the emitted text EVALUATES, and in which order.
//
// Three shapes where the port's output ran the source's side effects in an
// order Rust does not, or ran one of them twice.
//
// `range.contains(&item)` builds the RANGE first and evaluates the item after
// it. The port lowers a method's arguments before it ever reaches the receiver,
// so the item's own temporary stood above both bounds: the emitted code printed
// I, S, E where rustc prints S, E, I.
//
// `vec![value; count]` evaluates the value and then the count.
// `Array(count).fill(value)` evaluates the count first.
//
// And a bound written as a reference to a CALL was hoisted TWICE — `_t0 =
// make(9)` followed by `_t1 = _t0`, a name aliasing a name — so the one value
// carried two releases and the second was a double drop.

use std::cell::RefCell;

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

impl PartialEq for Token {
    fn eq(&self, other: &Token) -> bool {
        self.n == other.n
    }
}

impl PartialOrd for Token {
    fn partial_cmp(&self, other: &Token) -> Option<std::cmp::Ordering> {
        self.n.partial_cmp(&other.n)
    }
}

pub fn look(t: &Token) -> i64 {
    t.n
}

/// Each call records the number it was given, so the ORDER the emitted text
/// evaluates them in is what the log says afterwards.
pub fn note(log: &RefCell<Vec<i64>>, n: i64) -> Token {
    log.borrow_mut().push(n);
    Token { n }
}

pub fn number(log: &RefCell<Vec<i64>>, n: i64) -> i64 {
    log.borrow_mut().push(n);
    n
}

/// The three side effects, in the order Rust runs them: start, end, item.
pub fn in_range(log: &RefCell<Vec<i64>>) -> bool {
    (note(log, 1)..note(log, 9)).contains(&note(log, 5))
}

/// The value, and then the count.
pub fn repeated(log: &RefCell<Vec<i64>>) -> Vec<i64> {
    vec![number(log, 7); number(log, 2) as usize]
}

/// A reference to a call, handed to a parameter that takes a reference: one
/// value, one name, one release.
pub fn doubled(log: &RefCell<Vec<i64>>) -> i64 {
    look(&&note(log, 3))
}
