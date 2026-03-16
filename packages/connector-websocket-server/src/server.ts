// MIRRORS: ankurah/connectors/websocket-server/src/server.rs

// Divergence: Rust uses axum/tokio/tungstenite for HTTP+WebSocket.
// TS defines a WebSocket handler interface so callers can integrate with any
// server framework (ws, Bun.serve, uWebSockets, etc.) [E8].

import type { NodeComms } from '@ankurah/core';
import { Message, Presence, BincodeReader } from '@ankurah/proto';
import { WebSocketClientSender, type WsSendFn } from './sender.ts';
import {
  type Connection,
  connectionInitial,
  connectionEstablished,
  connectionSend,
} from './state.ts';
import { smartClientIp } from './client_ip.ts';
import { extractUserAgent } from './user_agent.ts';

// ── WebsocketServer ──────────────────────────────────────────────────

export class WebsocketServer {
  private readonly node: NodeComms;

  constructor(node: NodeComms) {
    this.node = node;
  }

  /// Handle a new WebSocket connection.
  ///
  /// This is the main entry point called by the HTTP server's WebSocket upgrade handler.
  /// It mirrors Rust's `handle_websocket()` function.
  ///
  /// @param send - callback to send binary data to the client
  /// @param onMessage - call this with each incoming binary message
  /// @param onClose - call this when the connection closes
  /// @param clientIp - client IP address (from headers or socket)
  handleConnection(
    send: WsSendFn,
    clientIp: string,
  ): WebSocketConnectionHandler {
    return new WebSocketConnectionHandler(this.node.cloned(), send, clientIp);
  }

  /// Convenience: extract client IP from headers + socket, then create handler.
  handleConnectionFromHeaders(
    send: WsSendFn,
    headers: Record<string, string | string[] | undefined>,
    remoteAddress?: string,
  ): WebSocketConnectionHandler {
    const clientIp = smartClientIp(headers, remoteAddress) ?? 'unknown';
    return this.handleConnection(send, clientIp);
  }
}

// ── WebSocketConnectionHandler ───────────────────────────────────────

/// Per-connection handler that processes the WebSocket lifecycle.
/// Mirrors Rust `handle_websocket()` and `process_message()`.
export class WebSocketConnectionHandler {
  private readonly node: NodeComms;
  private conn: Connection;
  private readonly clientIp: string;

  constructor(node: NodeComms, send: WsSendFn, clientIp: string) {
    this.node = node;
    this.conn = connectionInitial(send);
    this.clientIp = clientIp;
  }

  /// Call once immediately after connection to send server presence.
  /// Mirrors Rust: sending `Message::Presence(...)` right after connection.
  sendPresence(): void {
    const presence = new Presence(
      this.node.id(),
      this.node.durable(),
      this.node.systemRoot(),
    );
    const message = new Message('Presence', { presence });
    connectionSend(this.conn, message);
  }

  /// Process an incoming binary message from the client.
  /// Mirrors Rust `process_message()`.
  ///
  /// Returns false if the connection should be closed (break), true to continue.
  async onBinaryMessage(data: Uint8Array): Promise<boolean> {
    const reader = new BincodeReader(data);
    let message: Message;
    try {
      message = Message.decode(reader);
    } catch {
      console.error(`Failed to deserialize message from ${this.clientIp}`);
      return true; // continue, matching Rust behavior (logs error but doesn't break)
    }

    // Divergence: Can't use .match() because PeerMessage arm is async [E8].
    // Use .is() narrowing instead.
    if (message.is('Presence')) {
      const presence = (message.value as { presence: Presence }).presence;
      if (this.conn.type === 'Initial') {
        const send = this.conn.send;
        if (send !== null) {
          // Register peer sender for this client
          const sender = new WebSocketClientSender(presence.nodeId, send);

          this.node.registerPeer(presence, sender);
          this.conn = connectionEstablished(sender);
        }
      } else {
        console.warn(
          `Received presence from ${this.clientIp} but already have a peer sender - ignoring`,
        );
      }
    } else if (message.is('PeerMessage')) {
      const nodeMessage = (message.value as { nodeMessage: import('@ankurah/proto').NodeMessage }).nodeMessage;
      if (this.conn.type === 'Established') {
        // Divergence: Rust spawns a tokio task. TS awaits directly (single-threaded) [E8].
        try {
          await this.node.handleMessage(nodeMessage);
        } catch (e) {
          console.error(`Error handling message from ${this.clientIp}:`, e);
        }
      } else {
        console.warn(
          `Received peer message from ${this.clientIp} but not connected as a peer`,
        );
      }
    }

    return true; // continue
  }

  /// Call when the WebSocket connection is closed.
  /// Cleans up peer registration if established.
  /// Mirrors Rust cleanup after the receive loop exits.
  onClose(): void {
    if (this.conn.type === 'Established') {
      this.node.deregisterPeer(this.conn.sender.recipientNodeId());
    }
  }
}

// ── Helpers ──────────────────────────────────────────────────────────

function formatUserAgent(userAgent: string | null): string {
  return userAgent ?? 'Unknown browser';
}
