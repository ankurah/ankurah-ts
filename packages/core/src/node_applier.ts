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
  UpdateContent,
  DeltaContent,
} from '@ankurah/proto';

import { Entity } from './entity.ts';
import type { Node } from './node.ts';
import { EntityChange } from './changes.ts';
import { MutationError, ApplyError, ApplyErrorItem } from './error.ts';
import type { StorageCollection } from './storage.ts';
import type { Retrieve } from './retrieval.ts';
import { ReadyChunks } from './util/ready_chunks.ts';

// ---------------------------------------------------------------------------
// NodeApplier — consolidates all logic for applying remote updates to a node
// ---------------------------------------------------------------------------

/**
 * Consolidates all logic for applying remote updates to a node.
 * Handles both SubscriptionUpdateItem (streaming updates) and EntityDelta (initial Fetch/QuerySubscribed).
 *
 * Rust: `pub struct NodeApplier;` (unit struct with associated methods)
 * Divergence: TS uses a class with static methods [E8].
 */
export class NodeApplier {
  /**
   * Similar to commit_transaction, except that we check event attestations instead of checking
   * write permissions. We also don't need to fan events out to peers because we're receiving
   * them from a peer.
   *
   * Rust: `pub(crate) async fn apply_updates(...) -> Result<(), MutationError>`
   *
   * NOTE: Since SubscriptionRelay is not yet ported, the guard will always throw.
   */
  static async applyUpdates(
    node: Node,
    fromPeerId: EntityId,
    items: SubscriptionUpdateItem[],
  ): Promise<void> {
    // In theory, if initialized_predicate is specified, we could narrow it down to just the
    // context for that predicate, but this feels brittle because failure to apply this event
    // would affect the other contexts on this node.
    if (node.subscriptionRelay === null) {
      throw MutationError.invalidUpdate(
        'Should not be receiving updates without a subscription relay',
      );
    }

    // TODO: When SubscriptionRelay is ported, get context data from relay:
    // const cdata = relay.getContextsForPeer(fromPeerId);
    // if (cdata.length === 0) {
    //   throw MutationError.invalidUpdate(
    //     'Should not be receiving updates without at least predicate context',
    //   );
    // }

    // Apply all updates and notify reactor
    const changes: EntityChange[] = [];
    for (const update of items) {
      // TODO: When EphemeralNodeRetriever is ported, create retriever per update:
      // const retriever = new EphemeralNodeRetriever(update.collection, node, cdata);
      // await NodeApplier.applyUpdate(node, fromPeerId, update, retriever, changes, null);
      // await retriever.storeUsedEvents();

      // For now, pass null retriever (applyUpdate will handle it)
      await NodeApplier.applyUpdate(node, fromPeerId, update, null, changes, null);
    }

    await node.reactor.notifyChange(changes);
  }

  /**
   * Apply multiple EntityDeltas in parallel with batched reactor notification.
   * Drains all ready futures per wake and calls reactor.notifyChange for each batch.
   * Collects all errors and returns them at the end - caller decides whether to fail or log.
   *
   * Rust: `pub(crate) async fn apply_deltas(...) -> Result<(), ApplyError>`
   */
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

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  /**
   * Apply a single SubscriptionUpdateItem.
   *
   * Rust: `async fn apply_update(...) -> Result<(), MutationError>`
   *
   * @param entities - EntityChange[] to collect into, or null to discard (Pushable pattern)
   */
  private static async applyUpdate(
    node: Node,
    fromPeerId: EntityId,
    update: SubscriptionUpdateItem,
    retriever: Retrieve | null,
    changes: EntityChange[],
    entities: Entity[] | null,
  ): Promise<void> {
    // TODO: do we actually need predicate_relevance?
    const { entityId, collection: collectionId, content } = update;
    const collection = await node.storageEngine.collection(collectionId);

    switch (content.type) {
      case 'EventOnly': {
        // EventOnly: equivalent to old SubscriptionItem::Change
        const events = await NodeApplier.saveEvents(
          node,
          fromPeerId,
          entityId,
          collectionId,
          content.events,
          collection,
        );

        // We did not receive an entity fragment, so we need to retrieve it from local
        // storage or a remote peer
        // TODO: update the retriever to support bulk retrieval for multiple entities at once
        const entity = await NodeApplier.getRetrieveOrCreate(
          node,
          collectionId,
          entityId,
          retriever,
        );
        if (entities !== null) {
          entities.push(entity);
        }

        const appliedEvents: Attested<Event>[] = [];
        for (const event of events) {
          // TODO: Pass lineage retriever when available
          // Events should always be appliable sequentially
          if (entity.applyEvent(event.payload)) {
            appliedEvents.push(event);
          }
        }

        if (appliedEvents.length > 0) {
          changes.push(EntityChange.create(entity, appliedEvents));
        }
        break;
      }

      case 'StateAndEvent': {
        // StateAndEvent: equivalent to old SubscriptionItem::Add
        const events = await NodeApplier.saveEvents(
          node,
          fromPeerId,
          entityId,
          collectionId,
          content.events,
          collection,
        );

        const attestedState = StateFragment.toAttestedEntityState(
          entityId,
          collectionId,
          content.state,
        );
        node.policyAgent.validateReceivedState(fromPeerId, attestedState);

        // withState only updates the in-memory entity, it does NOT persist to storage
        const [changed, entity] = node.entities.withState(
          entityId,
          collectionId,
          attestedState.payload.state,
        );
        if (entities !== null) {
          entities.push(entity);
        }

        // TODO: get the list of events that were actually applied - don't just pass them
        // all through blindly
        if (changed === true || changed === null) {
          await NodeApplier.saveState(node, entity, collection);
          changes.push(EntityChange.create(entity, events));
        }
        break;
      }
    }
  }

  /**
   * Apply a single EntityDelta. Returns EntityChange if the delta resulted in a change,
   * null otherwise. On error, returns an ApplyErrorItem (wraps entity_id + collection + cause).
   *
   * Rust: `async fn apply_delta(...) -> Result<Option<EntityChange>, ApplyErrorItem>`
   *
   * Divergence: Returns EntityChange | null | ApplyErrorItem instead of Result, because
   * ReadyChunks collects settled promise values (not rejections) [E8].
   */
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

  /**
   * Inner implementation of applyDelta. Throws MutationError on failure.
   *
   * Rust: `async fn apply_delta_inner(...) -> Result<Option<EntityChange>, MutationError>`
   */
  private static async applyDeltaInner(
    node: Node,
    fromPeerId: EntityId,
    delta: EntityDelta,
    retriever: Retrieve,
  ): Promise<EntityChange | null> {
    const collection = await node.storageEngine.collection(delta.collection);

    switch (delta.content.type) {
      case 'StateSnapshot': {
        const attestedState = StateFragment.toAttestedEntityState(
          delta.entityId,
          delta.collection,
          delta.content.state,
        );
        node.policyAgent.validateReceivedState(fromPeerId, attestedState);

        const [_changed, entity] = node.entities.withState(
          delta.entityId,
          delta.collection,
          attestedState.payload.state,
        );

        // Save state to storage
        await NodeApplier.saveState(node, entity, collection);

        // Phase 1: Return EntityChange with empty events
        return EntityChange.create(entity, []);
      }

      case 'EventBridge': {
        const attestedEvents: Attested<Event>[] = delta.content.events.map((f) =>
          EventFragment.toAttestedEvent(delta.entityId, delta.collection, f),
        );

        retriever.stageEvents(attestedEvents);

        // Get or create entity
        const entity = await NodeApplier.getRetrieveOrCreate(
          node,
          delta.collection,
          delta.entityId,
          retriever,
        );

        // HACK - applying events in reverse order to avoid triggering the NotDescends bug
        // in apply_event where the event is wrongly made concurrent
        for (const event of [...attestedEvents].reverse()) {
          // TODO: Pass lineage retriever when available
          entity.applyEvent(event.payload);
          retriever.markEventUsed(event.payload.id());
        }

        // Save updated state
        await NodeApplier.saveState(node, entity, collection);

        // Phase 1: Return EntityChange with empty events
        return EntityChange.create(entity, []);
      }

      case 'StateAndRelation': {
        // Phase 2: Will validate causal assertion and apply state
        throw MutationError.general(
          new Error('StateAndRelation not yet implemented in Phase 1'),
        );
      }
    }
  }

  /**
   * Helper to process events: validate, store, and return attested events.
   *
   * Rust: `async fn save_events(...) -> Result<Vec<Attested<Event>>, MutationError>`
   */
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

  /**
   * Save entity state to storage.
   *
   * Rust: `async fn save_state(...) -> Result<(), MutationError>`
   */
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

  /**
   * Get an entity from the node's entity set, try retriever if available, or create a new one.
   *
   * Rust: `node.entities.get_retrieve_or_create(retriever, collection_id, entity_id)`
   * Divergence: Rust's WeakEntitySet has this method natively via the Retrieve trait;
   * TS inlines the logic here since WeakEntitySet doesn't have retrieve [E8].
   */
  private static async getRetrieveOrCreate(
    node: Node,
    collectionId: CollectionId,
    entityId: EntityId,
    retriever: Retrieve | null,
  ): Promise<Entity> {
    // Check local resident entities first
    const local = node.entities.get(entityId);
    if (local) {
      return local;
    }

    // Try retriever if available
    if (retriever !== null) {
      const attestedState = await retriever.getState(entityId);
      if (attestedState !== null) {
        const [_changed, entity] = node.entities.withState(
          entityId,
          collectionId,
          attestedState.payload.state,
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
