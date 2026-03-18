// MIRRORS: ankurah/connectors/websocket-server/src/lib.rs
//
// Divergence: Rust only does `pub use server::*` and `pub use user_agent::OptionalUserAgent`.
// TS exports all sub-modules because the framework-agnostic approach requires consumers
// to access Connection, WebSocketClientSender, smartClientIp, etc. directly [E17].

export { smartClientIp } from './client_ip.ts';
export { WebSocketClientSender, type WsSendFn } from './sender.ts';
export { WebsocketServer, WebSocketConnectionHandler } from './server.ts';
export {
  type Connection,
  connectionInitial,
  connectionEstablished,
  connectionSend,
} from './state.ts';
export { extractUserAgent } from './user_agent.ts';
