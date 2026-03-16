// MIRRORS: ankurah/core/src/peer_subscription/client_relay.rs

// TODO: Rename this module from client_relay to remote_subscription for clarity

import type { CollectionId, EntityId, QueryId } from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';

import { RequestError, RetrievalError } from '../error.ts';
import { spawn } from '../task.ts';

// ── RemoteQuerySubscriber ───────────────────────────────────────────────────
// Rust: pub trait RemoteQuerySubscriber: Clone + Send + Sync + 'static

/// Trait for query initialization that can be driven by SubscriptionRelay
/// Abstracts the relay's interaction with LiveQuery
export interface RemoteQuerySubscriber {
  /// Called after remote subscription deltas have been applied
  /// Dispatches to initialize (version 1) or update_selection_init (version >1) internally
  /// Handles marking initialization as complete and setting last_error on failure
  subscriptionEstablished(version: number): Promise<void>;

  /// Set the last error for this subscription
  setLastError(error: RetrievalError): void;
}

// ── Status ──────────────────────────────────────────────────────────────────
// Rust: pub enum Status { PendingRemote, Requested(EntityId, u32), Established(EntityId, u32), PendingUpdate(EntityId, u32), Failed }

export type Status =
  | { type: 'PendingRemote' }
  | { type: 'Requested'; peerId: EntityId; version: number }
  | { type: 'Established'; peerId: EntityId; version: number }
  | { type: 'PendingUpdate'; peerId: EntityId; version: number }
  | { type: 'Failed' }; // Non-retryable

// Divergence: Rust uses enum variants with positional fields; TS uses discriminated union [E8]

// ── Content ─────────────────────────────────────────────────────────────────
// Rust: pub struct Content<CD: ContextData>

export interface Content<CD> {
  readonly queryId: QueryId;
  readonly collectionId: CollectionId;
  readonly selection: Selection;
  readonly contextData: CD;
  readonly version: number;
}

// ── RemoteQueryState ────────────────────────────────────────────────────────
// Rust: pub struct RemoteQueryState<CD: ContextData, Q: RemoteQuerySubscriber>

export interface RemoteQueryState<CD, Q extends RemoteQuerySubscriber> {
  content: Content<CD>;
  status: Status;
  livequery: Q;
}

// ── TNode ───────────────────────────────────────────────────────────────────
// Rust: pub trait TNode<CD: ContextData>: Send + Sync

/// Trait for communicating with remote peers (abstraction over WeakNode for testing)
export interface TNode<CD> {
  /// Send a predicate registration request to a remote peer, fetch known matches,
  /// apply received deltas, and store used events.
  /// Returns Ok(()) if subscription was established and deltas applied successfully.
  remoteSubscribe(
    peerId: EntityId,
    queryId: QueryId,
    collectionId: CollectionId,
    selection: Selection,
    contextData: CD,
    version: number,
  ): Promise<void>;

  /// Send a predicate unregistration message to a remote peer
  /// This is a one-way message, no response expected
  peerUnsubscribe(peerId: EntityId, queryId: QueryId): Promise<void>;
}

// ── SubscriptionRelay ───────────────────────────────────────────────────────
// Rust: pub struct SubscriptionRelay<CD: ContextData, Q: RemoteQuerySubscriber>
//
// Manages predicate registration on remote peer reactor subscriptions.
//
// The SubscriptionRelay provides a resilient, event-driven approach to managing which predicates
// are registered with remote durable peers. It automatically handles:
// - Registering predicates on peer reactor subscriptions when peers connect
// - Re-registering predicates when peers disconnect and reconnect
// - Retrying failed predicate registration attempts
// - Clean teardown when predicates are removed
// - Storing ContextData for each predicate to enable proper authorization
//
// This design separates predicate management concerns from the main Node implementation,
// making it easier to test and reason about predicate lifecycle management.
//
// # Public API (for Node integration)
//
// - `subscribeQuery()` - Call when local subscriptions are created (parallel to reactor.subscribe)
// - `unsubscribePredicate()` - Call when local subscriptions are removed (parallel to reactor.unsubscribe)
// - `notifyPeerConnected()` - Call when durable peers connect (triggers automatic predicate registration)
// - `notifyPeerDisconnected()` - Call when durable peers disconnect (orphans predicate registrations)
// - `getStatus()` - Query current state of a predicate registration
//
// # Internal/Testing API
//
// - `setupRemoteSubscriptions()` - Internal method for triggering predicate registration with specific peers
//   (called automatically by notifyPeerConnected, but exposed for testing)

export class SubscriptionRelay<CD, Q extends RemoteQuerySubscriber> {
  // Divergence: Rust uses Arc<SubscriptionRelayInner> for shared ownership;
  // TS is single-threaded, so fields are directly on the class [E8].

  // All subscription information in one place
  // Divergence: Rust uses Mutex<HashMap>; TS uses plain Map (single-threaded) [E8]
  private readonly subscriptions: Map<string, RemoteQueryState<CD, Q>> = new Map();

  // Track connected durable peers
  // Divergence: Rust uses SafeSet<EntityId>; TS uses Set with string keys for lookup [E8]
  private readonly connectedPeers: Set<string> = new Set();
  private readonly connectedPeerIds: Map<string, EntityId> = new Map();

  // Node for communicating with remote peers
  // Divergence: Rust uses OnceLock<Arc<dyn TNode<CD>>>; TS uses nullable field [E8]
  private node: TNode<CD> | null = null;

  // Shutdown signal for retry task
  // Divergence: Rust uses tokio::sync::mpsc::Sender; TS uses clearInterval [E8]
  private retryIntervalId: ReturnType<typeof setInterval> | null = null;

  // impl Default
  // Rust: fn default() -> Self { Self::new() }

  constructor() {
    this.startRetryTask();
  }

  /// Inject the node (typically a WeakNode for production)
  ///
  /// This should be called once during initialization. Returns an error if
  /// the node has already been set.
  setNode(node: TNode<CD>): void {
    if (this.node !== null) {
      throw new Error('Node has already been set');
    }
    this.node = node;
  }

  /// Notify the relay that a new predicate needs to be registered on remote peer subscriptions
  ///
  /// This should be called whenever a local subscription is established. The relay will
  /// track this predicate and automatically attempt to register it with available durable peers.
  subscribeQuery(
    queryId: QueryId,
    collectionId: CollectionId,
    selection: Selection,
    contextData: CD,
    version: number,
    livequery: Q,
  ): void {
    console.debug(`SubscriptionRelay.subscribePredicate() - New predicate ${queryId} needs remote registration`);

    const key = queryId.toUlidString();
    this.subscriptions.set(key, {
      content: { collectionId, selection, contextData, queryId, version },
      status: { type: 'PendingRemote' },
      livequery,
    });

    // Immediately attempt setup with available peers
    if (this.connectedPeers.size > 0) {
      this.setupRemoteSubscriptions();
    }
  }

  updateQuery(queryId: QueryId, selection: Selection, version: number): void {
    console.debug(`SubscriptionRelay.updateQuery() - New query ${queryId} needs remote registration`);

    const key = queryId.toUlidString();
    const state = this.subscriptions.get(key);
    if (state === undefined) {
      throw new Error(`Predicate ${queryId} not found`);
    }

    // Update the content with new predicate and version
    const oldContent = state.content;
    state.content = {
      collectionId: oldContent.collectionId,
      selection,
      contextData: oldContent.contextData,
      queryId: oldContent.queryId,
      version,
    };

    if (state.status.type === 'Established') {
      const peerId = state.status.peerId;
      // Update to new version, mark as requested for this peer
      state.status = { type: 'Requested', peerId, version };
      this.updateQueryOnPeer(peerId, queryId, state.content.collectionId, selection, version, state.content.contextData);
    } else {
      // Not established yet, just update to PendingRemote and setup
      state.status = { type: 'PendingRemote' };
      this.setupRemoteSubscriptions();
    }
  }

  private updateQueryOnPeer(
    peerId: EntityId,
    queryId: QueryId,
    collectionId: CollectionId,
    selection: Selection,
    version: number,
    contextData: CD,
  ): void {
    spawn((async () => {
      if (this.node === null) return;

      const key = queryId.toUlidString();
      // Get the livequery for error handling
      const livequery = this.subscriptions.get(key)?.livequery ?? null;

      try {
        // Send the updated predicate to the peer
        await this.node.remoteSubscribe(peerId, queryId, collectionId, selection, contextData, version);

        // Deltas applied successfully, now activate the livequery
        if (livequery !== null) {
          await livequery.subscriptionEstablished(version);
        }

        // Mark as established - subscription succeeded even if livequery activation had issues
        const info = this.subscriptions.get(key);
        if (info !== undefined) {
          info.status = { type: 'Established', peerId, version };
        }
        console.debug(`Successfully updated predicate ${queryId} on peer ${peerId} subscription`);
      } catch (e) {
        // Handle error with retry logic
        await this.handleError(queryId, peerId, e as RetrievalError, livequery);
      }
    })());
  }

  /// Notify the relay that a predicate should be removed from remote peer subscriptions
  ///
  /// This will clean up all tracking state and send unsubscribe requests to any
  /// remote peers that have this predicate registered.
  unsubscribePredicate(queryId: QueryId): void {
    console.debug(`Unregistering predicate ${queryId}`);

    const key = queryId.toUlidString();
    const info = this.subscriptions.get(key);
    if (info !== undefined) {
      this.subscriptions.delete(key);

      if (info.status.type === 'Established') {
        const peerId = info.status.peerId;
        if (this.node !== null) {
          const node = this.node;
          spawn((async () => {
            try {
              await node.peerUnsubscribe(peerId, queryId);
              console.debug(`Successfully sent unsubscribe message for ${queryId}`);
            } catch (e) {
              console.warn(`Failed to send unsubscribe message for ${queryId}: ${e}`);
            }
          })());
        }
      }
    }
  }

  /// Handle peer disconnection - mark all predicates for that peer as needing re-registration
  ///
  /// This should be called when a durable peer disconnects. All predicates registered
  /// with that peer will be marked as pending and will be automatically re-registered
  /// when the peer reconnects or another suitable peer becomes available.
  notifyPeerDisconnected(peerId: EntityId): void {
    console.debug(`Peer ${peerId} disconnected, orphaning predicate registrations`);

    const peerKey = peerId.toBase64();
    // Remove from connected peers
    this.connectedPeers.delete(peerKey);
    this.connectedPeerIds.delete(peerKey);

    for (const info of this.subscriptions.values()) {
      if (
        (info.status.type === 'Established' || info.status.type === 'Requested') &&
        info.status.peerId.equals(peerId)
      ) {
        // Update state to pending
        info.status = { type: 'PendingRemote' };
        console.warn(`Predicate ${info.content.queryId} orphaned due to peer ${peerId} disconnect`);
      }
    }

    // Resubscribe any orphaned subscriptions
    this.setupRemoteSubscriptions();
  }

  /// Handle peer connection - trigger predicate registration on the new peer subscription
  ///
  /// This should be called when a new durable peer connects. The relay will automatically
  /// attempt to register any pending predicates on the newly connected peer's subscription.
  notifyPeerConnected(peerId: EntityId): void {
    console.debug(`SubscriptionRelay.notifyPeerConnected() - Peer ${peerId} connected, registering predicates on peer subscription`);

    const peerKey = peerId.toBase64();
    // Add to connected peers
    this.connectedPeers.add(peerKey);
    this.connectedPeerIds.set(peerKey, peerId);

    // Trigger setup with all connected peers
    this.setupRemoteSubscriptions();
  }

  /// Get the current state of a predicate registration
  getStatus(queryId: QueryId): Status | null {
    const key = queryId.toUlidString();
    const info = this.subscriptions.get(key);
    return info?.status ?? null;
  }

  /// Get all unique contexts for predicates established or requested with a specific peer
  /// TODO: update the data structure to do this via a direct lookup rather than having to scan the entire map
  getContextsForPeer(peerId: EntityId): Set<CD> {
    const contexts = new Set<CD>();
    for (const state of this.subscriptions.values()) {
      if (
        (state.status.type === 'Established' || state.status.type === 'Requested') &&
        state.status.peerId.equals(peerId)
      ) {
        contexts.add(state.content.contextData);
      }
    }
    return contexts;
  }

  /// Register predicates on available durable peer subscriptions
  // Divergence: Rust marks this as private (not pub); exposed here for testing like Rust does [E8]
  setupRemoteSubscriptions(): void {
    if (this.node === null) {
      console.warn('No node configured for remote subscription setup');
      return;
    }

    // For now, use the first available peer (could be made smarter)
    if (this.connectedPeers.size === 0) {
      console.warn('No durable peers available for remote subscription setup');
      return;
    }

    const firstPeerKey = this.connectedPeers.values().next().value!;
    const targetPeer = this.connectedPeerIds.get(firstPeerKey)!;

    // Atomically get pending subscriptions and mark them as requested
    const pending: Content<CD>[] = [];
    for (const info of this.subscriptions.values()) {
      if (info.status.type === 'PendingRemote') {
        info.status = { type: 'Requested', peerId: targetPeer, version: info.content.version };
        pending.push(info.content);
      }
    }

    if (pending.length === 0) {
      return;
    }

    console.debug(`Registering ${pending.length} predicates on ${this.connectedPeers.size} peer subscriptions`);

    const node = this.node;
    for (const content of pending) {
      spawn(this.attemptSubscribe(node, targetPeer, content));
    }
  }

  private async attemptSubscribe(node: TNode<CD>, targetPeer: EntityId, content: Content<CD>): Promise<void> {
    const queryId = content.queryId;
    const selection = content.selection;
    const contextData = content.contextData;
    const version = content.version;

    const key = queryId.toUlidString();
    // Get the livequery for error handling
    const livequery = this.subscriptions.get(key)?.livequery ?? null;

    try {
      // Call remote_subscribe which fetches known matches, subscribes, applies deltas, and stores events
      await node.remoteSubscribe(targetPeer, queryId, content.collectionId, selection, contextData, version);

      // Deltas applied successfully, now activate the livequery
      // The livequery handles its own errors internally
      if (livequery !== null) {
        await livequery.subscriptionEstablished(version);
      }

      // Mark as established - subscription succeeded even if livequery activation had issues
      const info = this.subscriptions.get(key);
      if (info !== undefined) {
        info.status = { type: 'Established', peerId: targetPeer, version };
      }
      console.debug(`Successfully registered predicate ${queryId} on peer ${targetPeer} subscription`);
    } catch (e) {
      // Handle error with retry logic
      await this.handleError(queryId, targetPeer, e as RetrievalError, livequery);
    }
  }

  /// Start background task that periodically retries pending subscriptions
  // Divergence: Rust uses tokio::select! with mpsc shutdown; TS uses setInterval + clearInterval [E8]
  private startRetryTask(): void {
    this.retryIntervalId = setInterval(() => {
      // Attempt to setup any pending subscriptions
      this.setupRemoteSubscriptions();
    }, 5000);
  }

  /// Stop the background retry task
  // Divergence: Rust relies on dropping _shutdown_tx; TS explicitly clears the interval [E8]
  destroy(): void {
    if (this.retryIntervalId !== null) {
      clearInterval(this.retryIntervalId);
      this.retryIntervalId = null;
    }
  }

  /// Handle errors with retry logic
  private async handleError(
    queryId: QueryId,
    targetPeer: EntityId,
    error: RetrievalError,
    livequery: Q | null,
  ): Promise<void> {
    const errorMsg = String(error);

    // Evaluate retriability at failure time
    let isRetryable = false;
    if (error instanceof RetrievalError && error.kind === 'RequestError') {
      const reqErr = error.detail;
      if (reqErr instanceof RequestError) {
        switch (reqErr.kind) {
          case 'PeerNotConnected':
          case 'ConnectionLost':
          case 'SendError':
          case 'InternalChannelClosed':
            isRetryable = true;
            break;
          case 'ServerError':
          case 'UnexpectedResponse':
          case 'AccessDenied':
            isRetryable = false;
            break;
        }
      }
    }
    // Other retrieval errors are not retryable

    // Update state based on retriability
    const key = queryId.toUlidString();
    const info = this.subscriptions.get(key);
    if (info !== undefined) {
      if (isRetryable) {
        // Retryable errors go back to pending for retry by background task
        info.status = { type: 'PendingRemote' };
        console.warn(`Retryable failure for predicate ${queryId} with peer ${targetPeer}: ${errorMsg} - will retry`);
      } else {
        // Non-retryable errors are permanently failed
        info.status = { type: 'Failed' };
        console.error(`Permanent failure for predicate ${queryId} with peer ${targetPeer}: ${errorMsg} - no retry`);

        // Set error on livequery
        if (livequery !== null) {
          livequery.setLastError(error);
        }
      }
    }
  }
}

// Divergence: WeakNode impl of TNode omitted — production implementation depends on
// Node internals (request, fetch_entities_from_local, EphemeralNodeRetriever, NodeApplier)
// that are not yet ported (Layer 7). Will be added when those dependencies are available [E8].
