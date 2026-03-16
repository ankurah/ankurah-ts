// MIRRORS: ankurah/connectors/websocket-client/src/sender.rs

import type { EntityId, NodeMessage } from '@ankurah/proto';
import type { PeerSender } from '@ankurah/core';
import { SendError } from '@ankurah/core';

// ── WebsocketPeerSender ─────────────────────────────────────────────────────
// Rust: pub struct WebsocketPeerSender { tx: mpsc::UnboundedSender<NodeMessage>, recipient_node_id: EntityId }
// Divergence: Rust uses tokio mpsc channel; TS uses a callback + queue since
// there's no unbounded channel primitive. The WebSocket send loop pulls from
// this queue. [E17]

type MessageCallback = (message: NodeMessage) => void;

export class WebsocketPeerSender implements PeerSender {
  private readonly _recipientNodeId: EntityId;
  private _onMessage: MessageCallback | null;
  private _closed: boolean;

  private constructor(recipientNodeId: EntityId, onMessage: MessageCallback) {
    this._recipientNodeId = recipientNodeId;
    this._onMessage = onMessage;
    this._closed = false;
  }

  // Rust: pub fn new(recipient_node_id: EntityId) -> (Self, UnboundedReceiver<NodeMessage>)
  // Divergence: Rust returns (sender, receiver) tuple; TS uses a callback pattern
  // since there's no mpsc channel. The caller provides a callback that handles
  // outgoing messages. [E17]
  static new(recipientNodeId: EntityId): { sender: WebsocketPeerSender; receiver: MessageCallback } {
    const queue: NodeMessage[] = [];
    let listener: ((message: NodeMessage) => void) | null = null;

    const onMessage: MessageCallback = (message: NodeMessage) => {
      if (listener !== null) {
        listener(message);
      } else {
        queue.push(message);
      }
    };

    const sender = new WebsocketPeerSender(recipientNodeId, onMessage);

    // The "receiver" is a function that sets up the listener for outgoing messages
    // When the WebSocket loop calls setOutgoingHandler, queued messages are flushed
    const receiver: MessageCallback = (message: NodeMessage) => {
      // This won't be used directly — see setOutgoingHandler below
      onMessage(message);
    };

    // Attach a way to set the outgoing handler and flush the queue
    (sender as any)._queue = queue;
    (sender as any)._setListener = (fn: (message: NodeMessage) => void) => {
      listener = fn;
      // Flush queued messages
      while (queue.length > 0) {
        fn(queue.shift()!);
      }
    };

    return { sender, receiver };
  }

  /// Set the handler that receives outgoing messages destined for the peer.
  /// Flushes any messages queued before the handler was set.
  setOutgoingHandler(handler: (message: NodeMessage) => void): void {
    const setListener = (this as any)._setListener as ((fn: (message: NodeMessage) => void) => void) | undefined;
    if (setListener) {
      setListener(handler);
    }
  }

  // impl PeerSender

  // Rust: fn send_message(&self, message: NodeMessage) -> Result<(), SendError>
  // Divergence: Rust returns Result; TS throws SendError [E3]
  sendMessage(message: NodeMessage): void {
    console.debug(`Queuing message for peer ${this._recipientNodeId}`);

    if (this._closed || this._onMessage === null) {
      console.warn(`Failed to send message to peer ${this._recipientNodeId} - channel closed`);
      throw SendError.connectionClosed();
    }

    this._onMessage(message);
  }

  // Rust: fn recipient_node_id(&self) -> EntityId
  recipientNodeId(): EntityId {
    return this._recipientNodeId;
  }

  // Rust: fn cloned(&self) -> Box<dyn PeerSender>
  // Divergence: Rust clones the sender (Arc-backed mpsc); TS returns the same instance
  // since JS objects are reference types [E8]
  cloned(): PeerSender {
    return this;
  }

  /// Close the sender — prevents further messages from being queued
  close(): void {
    this._closed = true;
    this._onMessage = null;
  }
}
