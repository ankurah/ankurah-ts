//! `cloned` and `copied` over a payload that owes no release.
//!
//! A `Vec<u64>` and a `#[derive(Clone, Copy)]` struct are both mutable objects
//! in the port, and handing back the collection's own value let a write on the
//! result reach the collection: the port answered 5 where Rust answers 10.

use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct Dot {
    pub x: u64,
}

pub fn write_through_a_sequence(map: &HashMap<u64, Vec<u64>>, k: u64) -> u64 {
    if let Some(mut taken) = map.get(&k).cloned() {
        taken[0] = 5;
    }
    match map.get(&k) {
        Some(v) => v[0],
        None => 0,
    }
}

pub fn write_through_a_copy_struct(map: &HashMap<u64, Dot>, k: u64) -> u64 {
    if let Some(mut taken) = map.get(&k).copied() {
        taken.x = 5;
    }
    match map.get(&k) {
        Some(d) => d.x,
        None => 0,
    }
}

/// The collected form: the write lands on one element of the new vector.
pub fn write_through_a_collection(map: &HashMap<u64, Dot>, k: u64) -> u64 {
    let mut taken: Vec<Dot> = map.values().copied().collect();
    taken[0].x = 5;
    match map.get(&k) {
        Some(d) => d.x,
        None => 0,
    }
}
