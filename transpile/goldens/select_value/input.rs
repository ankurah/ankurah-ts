//! `select!` races its branches and takes the arm that wins, and that arm's
//! value is the select's value. It has to reach wherever the select was
//! written — the initialiser of a `let`, an argument to a call — so the
//! arbitration goes inside a function of its own and the arm returns from it.
//! Written as bare statements the value went nowhere, and where something bound
//! it the output was not a program.
//!
//! An arm that leaves the function or the loop around the select is the case
//! that function cannot carry: the `return` would leave the arm's own function
//! instead. Those keep the statement form, which is where such an arm can
//! stand, and the select produces no value there — as it does not in Rust
//! either.

use tokio::sync::mpsc;

/// The winning arm's value reaches the `let` that binds it.
pub async fn first_of(mut left: mpsc::Receiver<u32>, mut right: mpsc::Receiver<u32>) -> u32 {
    let winner = tokio::select! {
        _ = left.recv() => 1,
        _ = right.recv() => 2,
    };
    winner * 10
}

/// The same value as an argument, where a run of statements cannot stand.
pub async fn doubled(mut left: mpsc::Receiver<u32>, mut right: mpsc::Receiver<u32>) -> u32 {
    twice(tokio::select! {
        _ = left.recv() => 3,
        _ = right.recv() => 4,
    })
}

pub fn twice(n: u32) -> u32 { n * 2 }

/// The select as the block's last expression, where what the winning arm
/// produced is what the function hands back.
pub async fn last_word(mut left: mpsc::Receiver<u32>, mut right: mpsc::Receiver<u32>) -> u32 {
    tokio::select! {
        _ = left.recv() => 5,
        _ = right.recv() => 6,
    }
}

/// An arm that returns from the function around the select. The `return` has
/// to leave `answer`, so this select keeps the statement form.
pub async fn answer(mut left: mpsc::Receiver<u32>, mut right: mpsc::Receiver<u32>) -> u32 {
    tokio::select! {
        _ = left.recv() => { return 7; }
        _ = right.recv() => { return 8; }
    }
    0
}
