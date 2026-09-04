//! A `tokio::sync::Mutex` guard held across an `.await` is what the async mutex
//! exists for: the lock is still held when the turn resumes, and the block
//! releases it whichever way it leaves. `select!` is a macro over syntax, so the
//! emitter turns each branch into a tagged promise and keeps the arm bodies
//! itself — and every branch future is released when the select returns, the one
//! that won included.

use tokio::sync::Mutex;
use tokio::sync::mpsc;

pub struct Gate {
    pub lock: Mutex<usize>,
}

pub async fn step() -> usize { 1 }

impl Gate {
    /// The guard crosses an await and is still the same guard afterwards.
    pub async fn bump(&self) -> usize {
        let mut guard = self.lock.lock().await;
        *guard += step().await;
        *guard
    }
}

/// Two branches. Both receivers are this function's by value, so it releases
/// both, and the branch futures go with the select.
pub async fn race(mut left: mpsc::Receiver<usize>, mut right: mpsc::Receiver<usize>) -> usize {
    let mut winner = 0usize;
    tokio::select! {
        _ = left.recv() => { winner = 1; }
        _ = right.recv() => { winner = 2; }
    }
    winner
}
