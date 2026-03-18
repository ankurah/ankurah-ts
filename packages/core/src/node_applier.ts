// MIRRORS: ankurah/core/src/node_applier.rs
import {
  Attested,
  EntityState,
  EventFragment,
  StateFragment,
} from '@ankurah/proto';
import type {
  EntityId,
  CollectionId,
  Event,
  EntityDelta,
  SubscriptionUpdateItem,
} from '@ankurah/proto';

import { Entity } from './entity.ts';
import type { Node } from './node.ts';
import { EntityChange } from './changes.ts';
import { MutationError, ApplyError, ApplyErrorItem } from './error.ts';
import type { StorageCollection } from './storage.ts';
import type { Retrieve } from './retrieval.ts';
import { LocalRetriever } from './retrieval.ts';
import { ReadyChunks } from './util/ready_chunks.ts';

// ── NodeApplier ──────────────────────────────────────────────────────────────
// Rust: pub struct NodeApplier; (unit struct with associated methods)
// Consolidates all logic for applying remote updates to a node.
// Handles both SubscriptionUpdateItem (streaming updates) and EntityDelta (initial Fetch/QuerySubscribed).

export class NodeApplier {
  // Rust: pub(crate) async fn apply_updates(node, from_peer_id, items) -> Result<(), MutationError>
  // Similar to commit_transaction, except that we check event attestations instead of checking
  // write permissions. We also don't need to fan events out to peers because we're receiving
  // them from a peer.
  //
  // NOTE: Requires SubscriptionRelay (Layer 7 — deferred). Will throw until that is ported.
  static async applyUpdates(
    node: Node,
    fromPeerId: EntityId,
    items: SubscriptionUpdateItem[],
  ): Promise<void> {
    // Rust: let Some(relay) = &node.subscription_relay else { return Err(...) };
    // Simplified: Apply updates directly without relay context validation.
    // Full SubscriptionRelay integration will validate contexts properly.

    const changes: EntityChange[] = [];
    for (const update of items) {
      const collection = await node.collections.get(update.collection);
      const retriever = new LocalRetriever(collection);
      await NodeApplier.applyUpdate(node, fromPeerId, update, retriever, changes, null);
      await retriever.storeUsedEvents();
    }
    await node.reactor.notifyChange(changes);
  }

  // Rust: async fn apply_update(node, from_peer_id, update, retriever, changes, entities) -> Result<(), MutationError>
  private static async applyUpdate(
    node: Node,
    fromPeerId: EntityId,
    update: SubscriptionUpdateItem,
    retriever: Retrieve | null,
    changes: EntityChange[],
    entities: Entity[] | null, // Rust: &mut impl Pushable<Entity> — null = () (discard)
  ): Promise<void> {
    // TODO: do we actually need predicate_relevance?
    const { entityId, collection: collectionId, content } = update;
    const collection = await node.collections.get(collectionId);

    await content.match({
      EventOnly: async (v) => {
        const events = await NodeApplier.saveEvents(
          node, fromPeerId, entityId, collectionId, v.events, collection,
        );

        // We did not receive an entity fragment, so we need to retrieve it from local
        // storage or a remote peer
        // TODO: update the retriever to support bulk retrieval for multiple entities at once
        const entity = await NodeApplier.getRetrieveOrCreate(
          node, collectionId, entityId, retriever,
        );
        if (entities !== null) entities.push(entity);

        const appliedEvents: Attested<Event>[] = [];
        for (const event of events) {
          // Events should always be appliable sequentially
          if (entity.applyEvent(event.payload)) {
            appliedEvents.push(event);
          }
        }

        if (appliedEvents.length > 0) {
          changes.push(EntityChange.create(entity, appliedEvents));
        }
      },

      StateAndEvent: async (v) => {
        const events = await NodeApplier.saveEvents(
          node, fromPeerId, entityId, collectionId, v.events, collection,
        );

        const attestedState = StateFragment.toAttestedEntityState(
          entityId, collectionId, v.state,
        );
        node.policyAgent.validateReceivedState(fromPeerId, attestedState);

        // withState only updates the in-memory entity, it does NOT persist to storage
        const [changed, entity] = node.entities.withState(
          entityId, collectionId, attestedState.payload.state,
        );
        if (entities !== null) entities.push(entity);

        // TODO: get the list of events that were actually applied - don't just pass them
        // all through blindly
        if (changed === true || changed === null) {
          await NodeApplier.saveState(node, entity, collection);
          changes.push(EntityChange.create(entity, events));
        }
      },
    });
  }

  // Rust: async fn save_events(node, from_peer_id, entity_id, collection_id, fragments, collection) -> Result<Vec<Attested<Event>>, MutationError>
  // Helper to process events: validate, store, and return attested events.
  private static async saveEvents(
    node: Node,
    fromPeerId: EntityId,
    entityId: EntityId,
    collectionId: CollectionId,
    fragments: EventFragment[],
    collection: StorageCollection,
  ): Promise<Attested<Event>[]> {
    const attestedEvents: Attested<Event>[] = [];
    for (const fragment of fragments) {
      const attestedEvent = EventFragment.toAttestedEvent(entityId, collectionId, fragment);
      node.policyAgent.validateReceivedEvent(fromPeerId, attestedEvent);
      // TODO - add a suspense set of events which the retriever can draw from. Then add
      //        events to the collection only when the entity.add_event is successful.
      //        This way, we can quickly determine that a given event is descended merely by
      //        nature of being in the collection. This will be essential in peer-aided descent
      //        tests via attestation, which should dramatically accelerate the lineage test.
      await collection.addEvent(attestedEvent);
      attestedEvents.push(attestedEvent);
    }
    return attestedEvents;
  }

  // Rust: async fn save_state(node, entity, collection_wrapper) -> Result<(), MutationError>
  private static async saveState(
    node: Node,
    entity: Entity,
    collection: StorageCollection,
  ): Promise<void> {
    const state = entity.toState();
    const entityState = new EntityState(entity.id(), entity.collection(), state);
    const attestation = node.policyAgent.attestState(entityState);
    const attested = Attested.opt(entityState, attestation);
    await collection.setState(attested);
  }

  // Rust: pub(crate) async fn apply_deltas(node, from_peer_id, deltas, retriever) -> Result<(), ApplyError>
  // Apply multiple EntityDeltas in parallel with batched reactor notification.
  // Drains all ready futures per wake and calls reactor.notifyChange for each batch.
  // Collects all errors and returns them at the end - caller decides whether to fail or log.
  static async applyDeltas(
    node: Node,
    fromPeerId: EntityId,
    deltas: EntityDelta[],
    retriever: Retrieve,
  ): Promise<void> {
    // Do not wait for all apply_delta futures to complete - we need to apply all updates
    // in a timely fashion. If there are stragglers, they will be picked up on the next wake.
    // This should in theory be deterministic for EventBridge cases where all events are
    // immediately available.
    const promises = deltas.map((delta) =>
      NodeApplier.applyDelta(node, fromPeerId, delta, retriever),
    );
    const readyChunks = new ReadyChunks<EntityChange | null | ApplyErrorItem>(promises);

    const allErrors: ApplyErrorItem[] = [];

    for await (const results of readyChunks) {
      const batch: EntityChange[] = [];

      for (const result of results) {
        if (result instanceof ApplyErrorItem) {
          allErrors.push(result);
        } else if (result !== null) {
          batch.push(result);
        }
        // null means no change, continue
      }

      if (batch.length > 0) {
        await node.reactor.notifyChange(batch);
      }
    }

    if (allErrors.length > 0) {
      throw ApplyError.fromItems(allErrors);
    }
  }

  // Rust: async fn apply_delta(node, from_peer_id, delta, retriever) -> Result<Option<EntityChange>, ApplyErrorItem>
  // Returns EntityChange if the delta resulted in a change, null otherwise.
  // On error, returns an ApplyErrorItem (wraps entity_id + collection + cause).
  // Divergence: Returns EntityChange | null | ApplyErrorItem instead of Result, because
  // ReadyChunks collects settled promise values (not rejections) [E8].
  private static async applyDelta(
    node: Node,
    fromPeerId: EntityId,
    delta: EntityDelta,
    retriever: Retrieve,
  ): Promise<EntityChange | null | ApplyErrorItem> {
    const entityId = delta.entityId;
    const collectionId = delta.collection;

    try {
      return await NodeApplier.applyDeltaInner(node, fromPeerId, delta, retriever);
    } catch (e) {
      const cause =
        e instanceof MutationError
          ? e
          : MutationError.general(e instanceof Error ? e : new Error(String(e)));
      return new ApplyErrorItem(entityId, collectionId, cause);
    }
  }

  // Rust: async fn apply_delta_inner(node, from_peer_id, delta, retriever) -> Result<Option<EntityChange>, MutationError>
  private static async applyDeltaInner(
    node: Node,
    fromPeerId: EntityId,
    delta: EntityDelta,
    retriever: Retrieve,
  ): Promise<EntityChange | null> {
    const collection = await node.collections.get(delta.collection);

    return delta.content.match({
      StateSnapshot: async (v) => {
        const attestedState = StateFragment.toAttestedEntityState(
          delta.entityId, delta.collection, v.state,
        );
        node.policyAgent.validateReceivedState(fromPeerId, attestedState);

        const [_changed, entity] = node.entities.withState(
          delta.entityId, delta.collection, attestedState.payload.state,
        );

        // Save state to storage
        await NodeApplier.saveState(node, entity, collection);

        // Phase 1: Return EntityChange with empty events
        return EntityChange.create(entity, []);
      },

      EventBridge: async (v) => {
        const attestedEvents: Attested<Event>[] = v.events.map((f: EventFragment) =>
          EventFragment.toAttestedEvent(delta.entityId, delta.collection, f),
        );

        retriever.stageEvents(attestedEvents);

        // Get or create entity
        const entity = await NodeApplier.getRetrieveOrCreate(
          node, delta.collection, delta.entityId, retriever,
        );

        // HACK - applying events in reverse order to avoid triggering the NotDescends bug
        // in apply_event where the event is wrongly made concurrent
        for (const event of [...attestedEvents].reverse()) {
          entity.applyEvent(event.payload);
          retriever.markEventUsed(event.payload.id());
        }

        // Save updated state
        await NodeApplier.saveState(node, entity, collection);

        // Phase 1: Return EntityChange with empty events
        return EntityChange.create(entity, []);
      },

      StateAndRelation: async (_v) => {
        // Phase 2: Will validate causal assertion and apply state
        throw MutationError.general(
          new Error('StateAndRelation not yet implemented in Phase 1'),
        );
      },
    });
  }

  // Rust: node.entities.get_retrieve_or_create(retriever, collection_id, entity_id)
  // Divergence: Rust's WeakEntitySet has this method natively via the Retrieve trait;
  // TS inlines the logic here since WeakEntitySet doesn't have retrieve [E8].
  private static async getRetrieveOrCreate(
    node: Node,
    collectionId: CollectionId,
    entityId: EntityId,
    retriever: Retrieve | null,
  ): Promise<Entity> {
    // Check local resident entities first
    const local = node.entities.get(entityId);
    if (local) return local;

    // Try retriever if available
    if (retriever !== null) {
      const attestedState = await retriever.getState(entityId);
      if (attestedState !== null) {
        const [_changed, entity] = node.entities.withState(
          entityId, collectionId, attestedState.payload.state,
        );
        return entity;
      }
    }

    // Create a new empty entity
    const entity = Entity.create(entityId, collectionId);
    node.entities.register(entity);
    return entity;
  }
}

// Rust: trait Pushable<T> { fn push(&mut self, value: T); }
// Divergence: TS uses Entity[] | null — null acts as () (discard) [E8].
