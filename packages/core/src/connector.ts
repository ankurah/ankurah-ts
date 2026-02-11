// MIRRORS: ankurah/core/src/connector.rs

import type { EntityId, NodeMessage, Presence, Attested, EntityState } from '@ankurah/proto';
import { SendError } from './error.ts';

// Re-export SendError for convenience (defined in error.ts, mirroring Rust connector.rs)
export { SendError } from './error.ts';

// ---------------------------------------------------------------------------
// PeerSender — trait for sending messages to a peer
// ---------------------------------------------------------------------------

/**
 * Interface for sending messages to a connected peer node.
 *
 * Rust: `pub trait PeerSender: Send + Sync`
 * Divergence: No Send/Sync bounds — single-threaded JS [E8].
 * Divergence: No `cloned()` method — JS objects are reference types [E8].
 */
export interface PeerSender {
  /**
   * Send a message to the peer.
   *
   * Rust: `fn send_message(&self, message: proto::NodeMessage) -> Result<(), SendError>`
   * Throws SendError on failure [A8].
   */
  sendMessage(message: NodeMessage): void;

  /**
   * The node ID of the recipient of this message.
   *
   * Rust: `fn recipient_node_id(&self) -> proto::EntityId`
   */
  recipientNodeId(): EntityId;
}

// ---------------------------------------------------------------------------
// NodeComms — trait for node communication
// ---------------------------------------------------------------------------

/**
 * Interface for node communication. Implemented by Node.
 *
 * Rust: `pub trait NodeComms: Send + Sync`
 * Divergence: No Send/Sync bounds — single-threaded JS [E8].
 * Divergence: No `cloned()` method — JS objects are reference types [E8].
 */
export interface NodeComms {
  /**
   * The node ID.
   *
   * Rust: `fn id(&self) -> proto::EntityId`
   */
  id(): EntityId;

  /**
   * Whether this node has durable storage.
   *
   * Rust: `fn durable(&self) -> bool`
   */
  durable(): boolean;

  /**
   * The attested system root state, if any.
   *
   * Rust: `fn system_root(&self) -> Option<Attested<EntityState>>`
   */
  systemRoot(): Attested<EntityState> | null;

  /**
   * Register a peer with its presence and sender.
   *
   * Rust: `fn register_peer(&self, presence: proto::Presence, sender: Box<dyn PeerSender>)`
   * Divergence: No Box<dyn> needed — JS uses plain interface [E8].
   */
  registerPeer(presence: Presence, sender: PeerSender): void;

  /**
   * Deregister a peer by node ID.
   *
   * Rust: `fn deregister_peer(&self, node_id: proto::EntityId)`
   */
  deregisterPeer(nodeId: EntityId): void;

  /**
   * Handle an incoming message from a peer.
   *
   * Rust: `async fn handle_message(&self, message: proto::NodeMessage) -> anyhow::Result<()>`
   * Throws on failure [A8].
   */
  handleMessage(message: NodeMessage): Promise<void>;
}
