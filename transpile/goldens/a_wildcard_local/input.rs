//! `let _ = e;` binds nothing in Rust: what `e` built dies at the semicolon,
//! and a place read through `_` is not moved at all. Written as `const _ = e`
//! it became a binding nothing releases — the shape that leaked an awaited
//! `Result` at connectors/local-process/src/lib.rs:82.

pub struct Ticket {
    pub n: u64,
}

impl Drop for Ticket {
    fn drop(&mut self) {}
}

pub struct Desk {
    pub issued: u64,
}

impl Desk {
    pub fn issue(&mut self, n: u64) -> Ticket {
        self.issued += 1;
        Ticket { n }
    }

    pub async fn serve(&mut self, n: u64) -> Ticket {
        self.issue(n)
    }
}

/// The corpus shape: an awaited call whose value nobody binds.
pub async fn serve_and_forget(desk: &mut Desk, n: u64) {
    let _ = desk.serve(n).await;
}

/// The same value, built without an await.
pub fn issue_and_forget(desk: &mut Desk, n: u64) {
    let _ = desk.issue(n);
}

/// A place read through `_` is not moved, so the block still owns `held` and
/// releases it once, at the end.
pub fn read_without_moving(desk: &mut Desk) -> u64 {
    let held = desk.issue(7);
    let _ = held;
    held.n
}
