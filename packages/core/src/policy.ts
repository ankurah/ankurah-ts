// MIRRORS: ankurah/core/src/policy.rs

import type {
  CollectionId,
  Attestation,
  Attested,
  AuthData,
  EntityId,
  EntityState,
  Event,
  NodeRequest,
  State,
  CausalAssertion,
} from '@ankurah/proto';
import type { Predicate } from '@ankurah/ankql';
import type { Entity } from './entity.ts';
import type { AccessDenied } from './error.ts';
import type { ValidationError } from './error.ts';

// ---------------------------------------------------------------------------
// PolicyAgent — trait for access control policy implementations
// ---------------------------------------------------------------------------

/**
 * Interface for policy agent implementations that control access to entities.
 *
 * Rust: `pub trait PolicyAgent: Clone + Send + Sync + 'static`
 *
 * The policy agent is generic over ContextData (authentication/authorization context).
 * It validates reads, writes, and event attestation.
 *
 * Divergence: Rust uses associated type `type ContextData: ContextData`; TS uses
 * generic parameter [A6].
 * Divergence: Rust methods take `node: &Node<SE, Self>` parameter; TS omits it to
 * avoid circular import (node.ts imports policy.ts) [A6].
 */
export interface PolicyAgent<ContextData = unknown> {
  /**
   * Create relevant auth data for a given request.
   *
   * Rust: `fn sign_request(&self, node: &NodeInner<SE, Self>, cdata: &C, request: &NodeRequest) -> Result<Vec<AuthData>, AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  signRequest(cdata: ContextData[], request: NodeRequest): AuthData[];

  /**
   * Validate auth data and yield the context data if valid.
   *
   * Rust: `async fn check_request(&self, node: &Node<SE, Self>, auth: &A, request: &NodeRequest) -> Result<Vec<ContextData>, ValidationError>`
   * Throws ValidationError on failure [A8].
   */
  checkRequest(auth: AuthData[], request: NodeRequest): Promise<ContextData[]>;

  /**
   * Check an event against policy and optionally return an attestation.
   *
   * Rust: `fn check_event(&self, node: &Node, cdata: &CD, before: &Entity, after: &Entity, event: &Event) -> Result<Option<Attestation>, AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  checkEvent(
    cdata: ContextData,
    entityBefore: Entity,
    entityAfter: Entity,
    event: Event,
  ): Attestation | null;

  /**
   * Validate an event received from a remote peer.
   *
   * Rust: `fn validate_received_event(&self, node: &Node, from_node: &EntityId, event: &Attested<Event>) -> Result<(), AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  validateReceivedEvent(fromPeerId: EntityId, event: Attested<Event>): void;

  /**
   * Create an attestation for a state snapshot.
   *
   * Rust: `fn attest_state(&self, node: &Node, state: &EntityState) -> Option<Attestation>`
   */
  attestState(state: EntityState): Attestation | null;

  /**
   * Validate a state received from a remote peer.
   *
   * Rust: `fn validate_received_state(&self, node: &Node, from_node: &EntityId, state: &Attested<EntityState>) -> Result<(), AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  validateReceivedState(fromPeerId: EntityId, state: Attested<EntityState>): void;

  /**
   * Check if a collection can be accessed.
   *
   * Rust: `fn can_access_collection(&self, data: &C, collection: &CollectionId) -> Result<(), AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  canAccessCollection(cdata: ContextData[], collection: CollectionId): void;

  /**
   * Filter a predicate based on the context data.
   *
   * Rust: `fn filter_predicate(&self, data: &C, collection: &CollectionId, predicate: Predicate) -> Result<Predicate, AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  filterPredicate(cdata: ContextData[], collection: CollectionId, predicate: Predicate): Predicate;

  /**
   * Check if a context can read an entity.
   *
   * Rust: `fn check_read(&self, data: &C, id: &EntityId, collection: &CollectionId, state: &State) -> Result<(), AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  checkRead(cdata: ContextData[], id: EntityId, collection: CollectionId, state: State): void;

  /**
   * Check if a context can read an event.
   *
   * Rust: `fn check_read_event(&self, data: &C, event: &Attested<Event>) -> Result<(), AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  checkReadEvent(cdata: ContextData[], event: Attested<Event>): void;

  /**
   * Check if the given entity can be written to.
   *
   * Rust: `fn check_write(&self, cdata: &CD, entity: &Entity, event: Option<&Event>) -> Result<(), AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  checkWrite(cdata: ContextData, entity: Entity, event: Event | null): void;

  /**
   * Validate a causal assertion from a peer.
   *
   * Rust: `fn validate_causal_assertion(&self, node: &Node, peer_id: &EntityId, head_relation: &CausalAssertion) -> Result<(), AccessDenied>`
   * Throws AccessDenied on failure [A8].
   */
  validateCausalAssertion(peerId: EntityId, headRelation: CausalAssertion): void;
}

// ---------------------------------------------------------------------------
// PermissiveAgent — a policy agent that allows everything
// ---------------------------------------------------------------------------

/**
 * A policy agent that allows all operations (no restrictions).
 * Useful for testing and development.
 *
 * Rust: `pub struct PermissiveAgent {}`
 */
export class PermissiveAgent implements PolicyAgent<DefaultContext> {
  signRequest(_cdata: DefaultContext[], _request: NodeRequest): AuthData[] {
    // Divergence: Returns empty array. Rust creates one AuthData(vec![]) per context [E8].
    return [];
  }

  async checkRequest(auth: AuthData[], _request: NodeRequest): Promise<DefaultContext[]> {
    // PermissiveAgent accepts all auth attempts and returns one context per auth
    return auth.map(() => DEFAULT_CONTEXT);
  }

  checkEvent(): Attestation | null {
    return null; // No attestation
  }

  validateReceivedEvent(_fromPeerId: EntityId, _event: Attested<Event>): void {
    // Allow all received events
  }

  attestState(_state: EntityState): Attestation | null {
    return null; // No attestation
  }

  validateReceivedState(_fromPeerId: EntityId, _state: Attested<EntityState>): void {
    // Allow all received states
  }

  canAccessCollection(): void {
    // Allow all collection access
  }

  filterPredicate(_cdata: DefaultContext[], _collection: CollectionId, predicate: Predicate): Predicate {
    // PermissiveAgent passes predicate through unchanged
    return predicate;
  }

  checkRead(): void {
    // Allow all reads
  }

  checkReadEvent(): void {
    // Allow all event reads
  }

  checkWrite(): void {
    // Allow all writes
  }

  validateCausalAssertion(_peerId: EntityId, _headRelation: CausalAssertion): void {
    // PermissiveAgent trusts all causal assertions
  }
}

// Backward compatibility alias
// Divergence: Rust uses PermissiveAgent; existing TS code uses OpenPolicy [E8].
export { PermissiveAgent as OpenPolicy };

// ---------------------------------------------------------------------------
// DefaultContext — default context for PermissiveAgent
// ---------------------------------------------------------------------------

/**
 * A default context that is used when no context is needed.
 *
 * Rust: `pub struct DefaultContext {}`
 */
export class DefaultContext {}

/** Singleton default context instance. Mirrors Rust `DEFAULT_CONTEXT`. */
export const DEFAULT_CONTEXT = new DefaultContext();
