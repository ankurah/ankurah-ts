// MIRRORS: ankurah/core/src/peer_subscription/server.rs

import type { EntityId, QueryId, CollectionId, EntityState, KnownEntity, EntityIdRange } from '@ankurah/proto';
import {
  Attested,
  AttestationSet,
  NodeResponseBody,
  UpdateContent,
  MembershipChange as ProtoMembershipChange,
  SubscriptionUpdateItem,
  StateFragment,
  EventFragment,
} from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';
import type { SubscriptionGuard } from '@ankurah/signals';

import type { Node } from '../node.ts';
import type { ReactorSubscription } from '../reactor/subscription.ts';
import type { ReactorSubscriptionId } from '../reactor/watcherset.ts';
import type { ReactorUpdate, ReactorUpdateItem, MembershipChange } from '../reactor/update.ts';

// ---------------------------------------------------------------------------
// SubscriptionHandler
// ---------------------------------------------------------------------------

/**
 * Manages a peer's subscription to this node's reactor.
 *
 * This handler owns both the ReactorSubscription and the SubscriptionGuard
 * for listening to changes on that subscription.
 *
 * Rust: `pub struct SubscriptionHandler`
 */
export class SubscriptionHandler {
  private readonly _peerId: EntityId;
  private readonly _subscription: ReactorSubscription;
  private readonly _guard: SubscriptionGuard;

  constructor(peerId: EntityId, node: Node) {
    this._peerId = peerId;
    this._subscription = node.reactor.subscribe();

    // Subscribe to changes on this subscription
    // Rust: subscription.subscribe(move |update: ReactorUpdate| { ... })
    this._guard = this._subscription.subscribe((update: ReactorUpdate) => {
      console.info(
        `SubscriptionHandler[${peerId}] received reactor update with ${update.items.length} items`,
      );

      // Deferred: node.sendUpdate(peerId, ...) — requires Node.sendUpdate (Layer 7)
      // Rust: node.send_update(peer_id, NodeUpdateBody::SubscriptionUpdate { items: ... })
      const items = update.items
        .map((item) => convertItem(node, peerId, item))
        .filter((x): x is SubscriptionUpdateItem => x !== null);

      if (items.length > 0) {
        console.debug(
          `SubscriptionHandler[${peerId}] would send ${items.length} items to peer (sendUpdate deferred)`,
        );
        // TODO: node.sendUpdate(peerId, new NodeUpdateBody('SubscriptionUpdate', { items }));
      }
    });
  }

  /** Get the subscription ID for this peer. */
  subscriptionId(): ReactorSubscriptionId {
    return this._subscription.id();
  }

  /** Get a reference to the subscription for adding/removing predicates. */
  subscription(): ReactorSubscription {
    return this._subscription;
  }

  /** Remove a predicate from this peer's subscription. */
  removePredicate(queryId: QueryId): void {
    this._subscription.removePredicate(queryId);
  }

  /** Remove entity subscriptions from this peer's subscription. */
  removeEntities(entityIds: Iterable<EntityId>): void {
    this._subscription.removeEntitySubscriptions(entityIds);
  }

  /**
   * Remove entity subscriptions from this peer's subscription using inclusive ranges.
   *
   * Rust: `pub fn remove_entity_ranges(&self, ranges: &[EntityIdRange])`
   * Deferred: removeEntitySubscriptionRanges not yet on ReactorSubscription
   */
  removeEntityRanges(_ranges: EntityIdRange[]): void {
    // TODO: this._subscription.removeEntitySubscriptionRanges(ranges);
    console.warn('removeEntityRanges: not yet implemented');
  }

  /**
   * Handle an entity subscription request for this peer.
   *
   * Rust: `pub async fn subscribe_entities<SE, PA>(...) -> anyhow::Result<NodeResponseBody>`
   * Deferred: Requires Node.generateEntityDelta, policy.checkRead (Layer 7)
   */
  async subscribeEntities(
    _node: Node,
    _collectionId: CollectionId,
    _ids: EntityId[],
    _knownEntities: KnownEntity[],
  ): Promise<NodeResponseBody> {
    // TODO: Full implementation requires:
    //   - node.policyAgent.canAccessCollection(cdata, collectionId)
    //   - storage_collection.getStates(ids)
    //   - node.policyAgent.checkRead(cdata, entityId, collectionId, state)
    //   - subscription.addEntitySubscriptions(subscribedIds)
    //   - node.generateEntityDelta(knownMap, state, storageCollection)
    throw new Error('subscribeEntities: not yet implemented (Layer 7)');
  }

  /**
   * Handle a subscription request for this peer.
   *
   * Rust: `pub async fn subscribe_query<SE, PA>(...) -> anyhow::Result<NodeResponseBody>`
   * Deferred: Requires Reactor.upsertQuery, Node.generateEntityDelta (Layer 7)
   */
  async subscribeQuery(
    _node: Node,
    _queryId: QueryId,
    _collectionId: CollectionId,
    _selection: Selection,
    _version: number,
    _knownMatches: KnownEntity[],
  ): Promise<NodeResponseBody> {
    // TODO: Full implementation requires:
    //   - version validation
    //   - node.policyAgent.canAccessCollection(cdata, collectionId)
    //   - node.policyAgent.filterPredicate(cdata, collectionId, predicate)
    //   - node.reactor.upsertQuery(subscriptionId, queryId, collectionId, selection, node, cdata, version)
    //   - node.policyAgent.attestState(node, entityState)
    //   - expandStates(initialStates, knownMatchIds, storageCollection)
    //   - node.generateEntityDelta(knownMap, state, storageCollection)
    throw new Error('subscribeQuery: not yet implemented (Layer 7)');
  }
}

// ---------------------------------------------------------------------------
// convertItem (module-private)
// ---------------------------------------------------------------------------

/**
 * Convert a single ReactorUpdateItem to a SubscriptionUpdateItem.
 *
 * Rust: `fn convert_item<SE, PA>(node, peer_id, item) -> Option<SubscriptionUpdateItem>`
 */
function convertItem(
  node: Node,
  peerId: EntityId,
  item: ReactorUpdateItem,
): SubscriptionUpdateItem | null {
  // Convert entity to EntityState and attest it
  let entityState: EntityState;
  try {
    entityState = item.entity.toEntityState();
  } catch (e) {
    console.warn(
      `Failed to convert entity ${item.entity.id()} to EntityState for peer ${peerId}: ${e}`,
    );
    return null;
  }

  const attestation = node.policyAgent.attestState(entityState);
  // Rust: Attested::opt(entity_state, attestation) — attestation is null for OpenPolicy
  // For now, create unattested state fragment
  const stateFragment = StateFragment.fromEntityState(entityState);

  // Events should already be attested
  const eventFragments = item.events.map((e) => e.payload.toEventFragment());

  // Determine content based on whether we have events
  const content = new UpdateContent('StateAndEvent', {
    state: stateFragment,
    events: eventFragments,
  });

  // Convert predicate relevance from reactor types to proto types
  const predicateRelevance = item.predicateRelevance.map(
    ([predId, membership]) => {
      const protoMembership = membership.match({
        Initial: () => new ProtoMembershipChange('Initial', {}),
        Add: () => new ProtoMembershipChange('Add', {}),
        Remove: () => new ProtoMembershipChange('Remove', {}),
      });
      return [predId, protoMembership] as [typeof predId, ProtoMembershipChange];
    },
  );

  // Create subscription update item
  return new SubscriptionUpdateItem(
    item.entity.id(),
    item.entity.collection(),
    content,
    predicateRelevance,
  );
}
