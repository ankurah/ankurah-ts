// MIRRORS: ankurah/connectors/websocket-client/src/lib.rs
//
// @ankurah/connector-websocket — WebSocket client connector.
//
// A WebSocket client for connecting an Ankurah node to another Ankurah node
// which hosts a WebSocket server.
//
// ## Automatic reconnection
//   Reconnects to the server if the connection is lost using exponential backoff.
//
// ## Graceful shutdown
//   Call shutdown() to close the connection and stop reconnection attempts.
//
// Rust crate: ankurah-websocket-client
// Divergence: Rust uses tokio-tungstenite; TS uses standard WebSocket API [E17].

// Rust: pub mod client;
// Rust: pub mod sender;

// Re-export the main types for easy use
// Rust: pub use client::{ConnectionState, WebsocketClient, WebsocketClientBuilder};
export { ConnectionState, ConnectionError, WebsocketClient, WebsocketClientBuilder } from './client.ts';

// Rust: pub use sender::WebsocketPeerSender;
export { WebsocketPeerSender } from './sender.ts';

// Divergence: Rust re-exports WebSocketConfig and TungsteniteError from tokio-tungstenite.
// These are not applicable in TS — the standard WebSocket API has no config type or
// tungstenite-specific error type [E17].
