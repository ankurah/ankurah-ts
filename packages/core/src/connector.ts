// MIRRORS: ankurah/core/src/connector.rs
import type { EntityId, NodeMessage, Presence, Attested, EntityState } from '@ankurah/proto';

// TODO redesign this such that:
// - the sender and receiver are disconnected at the same time
// - a connection id or dyn Ord/Eq/Hash is used to identify the connection for deregistration
//   so that we can have multiple connections to the same node without things getting mixed up

// ── PeerSender ───────────────────────────────────────────────────────────────
// Rust: #[async_trait] pub trait PeerSender: Send + Sync
// Divergence: No Send/Sync bounds — single-threaded JS [E8].

export interface PeerSender {
  sendMessage(message: NodeMessage): void; // Divergence: Rust returns Result<(), SendError>; TS throws SendError [E3]
  recipientNodeId(): EntityId;
  cloned(): PeerSender; // Divergence: Rust returns Box<dyn PeerSender>; TS uses interface [E7]
}

// ── SendError ────────────────────────────────────────────────────────────────
// Defined in error.ts — re-export for consumers who expect it from connector.
export { SendError } from './error.ts';

// ── NodeComms ────────────────────────────────────────────────────────────────
// Rust: #[async_trait] pub trait NodeComms: Send + Sync
// Divergence: No Send/Sync bounds — single-threaded JS [E8].

export interface NodeComms {
  nodeId(): EntityId; // Divergence: Rust fn id(); TS uses nodeId() to avoid collision with id property [E4]
  isDurable(): boolean; // Divergence: Rust fn durable(); TS uses isDurable() to avoid collision with durable property [E4]
  systemRoot(): Attested<EntityState> | null; // Divergence: Option<T> → T | null [E3]
  registerPeer(presence: Presence, sender: PeerSender): void; // Divergence: Rust takes Box<dyn PeerSender>; TS uses interface [E7]
  deregisterPeer(nodeId: EntityId): void;
  handleMessage(message: NodeMessage): Promise<void>; // Divergence: Rust returns anyhow::Result<()>; TS throws [E3]
  cloned(): NodeComms; // Divergence: Rust returns Box<dyn NodeComms>; TS uses interface [E7]
}

// ── impl NodeComms for Node ──────────────────────────────────────────────────
// Divergence: The Rust `impl NodeComms for Node<SE, PA>` block lives in this file,
// but in TS the implementation belongs on the Node class in node.ts [E7].
