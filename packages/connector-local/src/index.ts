// MIRRORS: ankurah/connectors/local-process/src/lib.rs
//
// @ankurah/connector-local — Local in-process connector.
//
// Connects multiple Node instances within the same process for testing.
// Rust crate: ankurah-connector-local-process

import type { EntityId, NodeMessage } from '@ankurah/proto';
import { Presence } from '@ankurah/proto';
import type { PeerSender, NodeComms } from '@ankurah/core';
import { SendError } from '@ankurah/core';

// ── LocalProcessSender ──────────────────────────────────────────────────────
// Rust: pub struct LocalProcessSender { sender: mpsc::Sender<NodeMessage>, node_id: EntityId }
// Divergence: Rust uses tokio mpsc channel; TS uses a direct callback since JS
// is single-threaded. Messages are delivered as microtasks [E18].

class LocalProcessSender implements PeerSender {
  private readonly onMessage: (message: NodeMessage) => void;
  private readonly nodeId: EntityId;
  private closed: boolean;

  constructor(nodeId: EntityId, onMessage: (message: NodeMessage) => void) {
    this.nodeId = nodeId;
    this.onMessage = onMessage;
    this.closed = false;
  }

  // impl PeerSender

  // Rust: fn send_message
  // Divergence: Rust returns Result; TS throws SendError [E3]
  sendMessage(message: NodeMessage): void {
    if (this.closed) {
      throw SendError.connectionClosed();
    }
    this.onMessage(message);
  }

  // Rust: fn recipient_node_id
  recipientNodeId(): EntityId {
    return this.nodeId;
  }

  // Rust: fn cloned
  // Divergence: Rust clones Arc-backed sender; TS returns same instance [E8]
  cloned(): PeerSender {
    return this;
  }

  /// Close this sender — prevents further messages
  close(): void {
    this.closed = true;
  }
}

// ── LocalProcessConnection ──────────────────────────────────────────────────
// Rust: pub struct LocalProcessConnection<SE1, PA1, SE2, PA2>
// Divergence: Rust is generic over StorageEngine/PolicyAgent; TS uses NodeComms interface [A6].
// Divergence: Rust stores WeakNode; TS stores NodeComms directly (no weak refs needed
// in single-threaded JS — the connection lifetime is explicit via destroy()) [E8].

/// Connector which establishes one sender between each of the two given nodes
export class LocalProcessConnection {
  private readonly node1: NodeComms;
  private readonly node2: NodeComms;
  private readonly node1Id: EntityId;
  private readonly node2Id: EntityId;
  private readonly sender1: LocalProcessSender;
  private readonly sender2: LocalProcessSender;
  private destroyed: boolean;

  private constructor(
    node1: NodeComms,
    node2: NodeComms,
    node1Id: EntityId,
    node2Id: EntityId,
    sender1: LocalProcessSender,
    sender2: LocalProcessSender,
  ) {
    this.node1 = node1;
    this.node2 = node2;
    this.node1Id = node1Id;
    this.node2Id = node2Id;
    this.sender1 = sender1;
    this.sender2 = sender2;
    this.destroyed = false;
  }

  /// Create a new LocalConnector and establish connection between the nodes
  // Rust: fn new
  static async new(node1: NodeComms, node2: NodeComms): Promise<LocalProcessConnection> {
    const node1Id = node1.nodeId();
    const node2Id = node2.nodeId();

    // Create senders that deliver messages to the other node
    // Divergence: Rust uses mpsc channels with spawned receiver tasks;
    // TS uses direct callbacks that schedule message handling as microtasks [E18].

    // sender2 sends to node2 (used by node1 to reach node2)
    const sender2 = new LocalProcessSender(node2Id, (message: NodeMessage) => {
      // Rust: tokio::spawn(async move { node.handle_message(message).await })
      // Divergence: Schedule as microtask instead of spawning a task [E8]
      node2.handleMessage(message).catch((e) => {
        console.warn(`Error handling message on node2: ${e}`);
      });
    });

    // sender1 sends to node1 (used by node2 to reach node1)
    const sender1 = new LocalProcessSender(node1Id, (message: NodeMessage) => {
      node1.handleMessage(message).catch((e) => {
        console.warn(`Error handling message on node1: ${e}`);
      });
    });

    // Register the senders with the nodes
    // Rust: node1.register_peer(Presence { node_id: node2.id, ... }, Box::new(sender))
    node1.registerPeer(
      new Presence(node2Id, node2.isDurable(), node2.systemRoot()),
      sender2,
    );
    node2.registerPeer(
      new Presence(node1Id, node1.isDurable(), node1.systemRoot()),
      sender1,
    );

    return new LocalProcessConnection(node1, node2, node1Id, node2Id, sender1, sender2);
  }

  // Rust: fn setup_receiver — SKIP: absorbed into new() callbacks [E18]

  /// Tear down the connection between the two nodes.
  // Rust: fn drop
  // Divergence: Rust uses Drop; TS requires explicit destroy() call [E8].
  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;

    // Close senders to prevent further message delivery
    this.sender1.close();
    this.sender2.close();

    // Deregister peers
    this.node1.deregisterPeer(this.node2Id);
    this.node2.deregisterPeer(this.node1Id);
  }
}
