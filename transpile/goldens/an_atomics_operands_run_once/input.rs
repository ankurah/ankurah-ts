// Every atomic operand runs exactly once, in Rust's order, and every stored
// value comes back at the atomic's own width.
//
// `Ticker` counts its own calls, so an operand a short-circuit or an untaken
// branch skips, or one that runs twice, leaves a different count behind; the
// high bit proves the store is unsigned, JavaScript's `|` answering a signed 32.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub struct Ticker {
    pub calls: AtomicUsize,
}

impl Ticker {
    pub fn new() -> Ticker {
        Ticker { calls: AtomicUsize::new(0) }
    }

    pub fn next(&self) -> usize {
        self.calls.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn flag(&self) -> bool {
        self.next() > 0
    }
}

pub fn bounds_read_their_operand_once() -> Vec<usize> {
    let tick = Ticker::new();
    let seen = AtomicUsize::new(0);
    let was_max = seen.fetch_max(tick.next(), Ordering::SeqCst);
    let after_max = seen.load(Ordering::SeqCst);
    let was_min = seen.fetch_min(tick.next(), Ordering::SeqCst);
    vec![was_max, after_max, was_min, seen.load(Ordering::SeqCst), tick.calls.load(Ordering::SeqCst)]
}

pub fn logic_reads_its_operand_once() -> Vec<bool> {
    let tick = Ticker::new();
    let off = AtomicBool::new(false);
    let on = AtomicBool::new(true);
    let was_and = off.fetch_and(tick.flag(), Ordering::SeqCst);
    let was_or = on.fetch_or(tick.flag(), Ordering::SeqCst);
    vec![
        was_and,
        off.load(Ordering::SeqCst),
        was_or,
        on.load(Ordering::SeqCst),
        tick.calls.load(Ordering::SeqCst) == 2,
    ]
}

pub fn compare_exchange_reads_its_new_value_once() -> Vec<bool> {
    let tick = Ticker::new();
    let value = AtomicUsize::new(1);
    let missed = value.compare_exchange(2, tick.next(), Ordering::SeqCst, Ordering::SeqCst).is_ok();
    let took = value.compare_exchange(1, tick.next(), Ordering::SeqCst, Ordering::SeqCst).is_ok();
    vec![missed, took, value.load(Ordering::SeqCst) == 2, tick.calls.load(Ordering::SeqCst) == 2]
}

pub fn a_high_bit_stays_unsigned() -> Vec<u32> {
    let flags = AtomicU32::new(1);
    let was_or = flags.fetch_or(2147483648, Ordering::SeqCst);
    let after_or = flags.load(Ordering::SeqCst);
    let was_and = flags.fetch_and(4294901760, Ordering::SeqCst);
    vec![was_or, after_or, was_and, flags.load(Ordering::SeqCst)]
}
