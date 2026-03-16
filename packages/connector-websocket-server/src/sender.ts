// MIRRORS: ankurah/connectors/websocket-server/src/sender.rs

import type { PeerSender } from '@ankurah/core';
import type { EntityId, NodeMessage } from '@ankurah/proto';
import { Message, BincodeWriter } from '@ankurah/proto';

// Divergence: Rust uses axum::extract::ws::Message + mpsc channel + tokio::spawn.
// TS uses a WebSocket send callback directly — no channel needed in single-threaded JS [E8].

/// Callback type for sending binary data over the WebSocket.
export type WsSendFn = (data: Uint8Array) => void;

/// PeerSender for sending messages to a websocket client.
/// Mirrors Rust `WebSocketClientSender`.
export class WebSocketClientSender implements PeerSender {
  private readonly _recipientNodeId: EntityId;
  private readonly send: WsSendFn;

  constructor(nodeId: EntityId, send: WsSendFn) {
    this._recipientNodeId = nodeId;
    this.send = send;
  }

  /// Serialize and send a proto::Message over the WebSocket.
  sendProtoMessage(message: Message): void {
    const writer = new BincodeWriter();
    message.encode(writer);
    const data = writer.finish();
    this.send(data);
  }

  // impl PeerSender

  sendMessage(message: NodeMessage): void {
    // Wrap in Message::PeerMessage, matching Rust
    const serverMessage = new Message('PeerMessage', { nodeMessage: message });
    this.sendProtoMessage(serverMessage);
  }

  recipientNodeId(): EntityId {
    return this._recipientNodeId;
  }

  cloned(): PeerSender {
    return new WebSocketClientSender(this._recipientNodeId, this.send);
  }
}

// Divergence: Rust `impl Drop for Inner` aborts the tokio task.
// TS has no background task to abort — the send callback is synchronous [E8].
