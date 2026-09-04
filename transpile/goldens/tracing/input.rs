//! `tracing::{trace, debug, info, warn, error}!`, emitted as calls.
//!
//! A running program's log is how anybody finds out what it did, and the port
//! emitted these as inert comments — so the ported program ran silently where
//! the Rust one narrated itself. Each becomes a call on the runtime's `tracing`
//! namespace, carrying the same rendered message, and the import comes with it.

pub struct Peer {
    pub id: u32,
}

pub fn connect(peer: &Peer) {
    tracing::info!("connecting to {}", peer.id);
    tracing::debug!("peer {} state ready", peer.id);
}

/// A bare `warn!` behind a `use tracing::warn` is the same macro.
use tracing::warn;

pub fn lost(peer: &Peer, reason: &str) {
    warn!("lost {}: {}", peer.id, reason);
    tracing::error!("giving up on {}", peer.id);
    tracing::trace!("done");
}
