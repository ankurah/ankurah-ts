// MIRRORS: ankurah/core/src/retrieval.rs
//
// Implements GetEvents for local and remote retrieval, allowing event retrieval
// from local storage and (eventually) remote peers.
// This lives alongside lineage because event retrieval is a lineage concern,
// not a context/session concern.

import type { Attested, Clock, EntityId, EntityState, Event, EventId } from '@ankurah/proto';
import type { StorageCollection } from './storage.ts';
import { RetrievalError } from './error.ts';

// ---------------------------------------------------------------------------
// TEvent — trait for events and event-like things that can be descended
// ---------------------------------------------------------------------------

/**
 * Interface for events and event-like things that can be descended.
 *
 * Rust: `pub trait TEvent: std::fmt::Display`
 * Divergence: Rust uses associated types (Id, Parent); TS uses concrete
 * types (EventId, TClock) since we only instantiate with proto::Event [E7].
 */
export interface TEvent {
  id(): EventId;
  parent(): TClock;
  toString(): string;
}

// ---------------------------------------------------------------------------
// TClock — trait for clocks
// ---------------------------------------------------------------------------

/**
 * Interface wrapping proto Clock with convenience methods.
 *
 * Rust: `pub trait TClock { type Id; fn members(&self) -> &[Self::Id]; }`
 * Divergence: Rust uses associated type Id; TS uses concrete EventId [E7].
 */
export interface TClock {
  members(): readonly EventId[];
}

// ---------------------------------------------------------------------------
// clockMembers — Clock implements TClock via adapter function
// ---------------------------------------------------------------------------

/**
 * Adapter: extract members from a proto Clock, satisfying TClock.
 *
 * Rust: `impl TClock for Clock { fn members(&self) -> &[EventId] { self.as_slice() } }`
 * Divergence: TS Clock class uses asSlice(); we wrap it rather than patching
 * the prototype [E7].
 */
export function clockMembers(clock: Clock): readonly EventId[] {
  return clock.asSlice();
}

// ---------------------------------------------------------------------------
// eventAsTEvent — Event implements TEvent via adapter function
// ---------------------------------------------------------------------------

/**
 * Adapter: wrap a proto Event as a TEvent.
 *
 * Rust: `impl TEvent for Event { fn id() -> EventId; fn parent() -> &Clock; }`
 * Divergence: TS returns a lightweight wrapper object rather than impl on
 * the Event class directly [E7].
 */
export function eventAsTEvent(event: Event): TEvent {
  return {
    id: () => event.id(),
    parent: () => ({ members: () => event.parent.asSlice() }),
    toString: () => event.toString(),
  };
}

// ---------------------------------------------------------------------------
// GetEvents — interface for retrieving events (from storage or network)
// ---------------------------------------------------------------------------

/**
 * Interface for retrieving events from storage or network.
 *
 * Rust: `#[async_trait] pub trait GetEvents`
 * Divergence: Rust uses associated types (Id, Event); TS uses concrete
 * types (EventId, Event) since the only instantiation is with proto types [E7].
 * Divergence: async Rust -> async/Promise in TS.
 */
export interface GetEvents {
  /**
   * Estimate the budget cost for retrieving a batch of events.
   * This allows different implementations to model their cost structure.
   *
   * Rust: `fn estimate_cost(&self, _batch_size: usize) -> usize`
   * Default: 1 per batch.
   */
  estimateCost(batchSize: number): number;

  /**
   * Retrieve events by their IDs.
   * Returns [cost, events] tuple.
   *
   * Rust: `async fn retrieve_event(&self, event_ids: Vec<Self::Id>) -> Result<(usize, Vec<Attested<Self::Event>>), RetrievalError>`
   */
  retrieveEvent(eventIds: EventId[]): Promise<[number, Attested<Event>[]]>;

  /**
   * Stage events for immediate retrieval without storage.
   * Used when applying EventBridge deltas.
   * Staged events are available for lineage comparison at zero budget cost
   * before being persisted.
   *
   * Rust: `fn stage_events(&self, events: impl IntoIterator<Item = Attested<Self::Event>>)`
   */
  stageEvents(events: Iterable<Attested<Event>>): void;

  /**
   * Mark an event as used. Used when applying EventBridge deltas.
   *
   * Rust: `fn mark_event_used(&self, event_id: &Self::Id)`
   */
  markEventUsed(eventId: EventId): void;
}

// ---------------------------------------------------------------------------
// Retrieve — extends GetEvents with state retrieval
// ---------------------------------------------------------------------------

/**
 * Main retrieval interface: extends GetEvents with state retrieval.
 * Each implementation determines whether to use local or remote storage.
 *
 * Rust: `#[async_trait] pub trait Retrieve: GetEvents`
 */
export interface Retrieve extends GetEvents {
  /**
   * Get the state for an entity. Returns null if entity not found.
   *
   * Rust: `async fn get_state(&self, entity_id: EntityId) -> Result<Option<Attested<EntityState>>, RetrievalError>`
   */
  getState(entityId: EntityId): Promise<Attested<EntityState> | null>;
}

// ---------------------------------------------------------------------------
// LocalRetriever — durable node retriever, reads from local storage
// ---------------------------------------------------------------------------

/**
 * Durable node retriever - retrieves everything locally from storage.
 *
 * Rust: `pub struct LocalRetriever(Arc<LocalRetrieverInner>)`
 * Divergence: No Arc/Mutex — single-threaded JS, plain fields [E8].
 * Divergence: StorageCollectionWrapper flattened to StorageCollection [E7].
 */
export class LocalRetriever implements Retrieve {
  private readonly collection: StorageCollection;
  /** Map from EventId base64 -> [Attested<Event>, wasUsed]. Null means taken. */
  private stagedEvents: Map<string, [Attested<Event>, boolean]> | null;

  constructor(collection: StorageCollection) {
    this.collection = collection;
    this.stagedEvents = new Map();
  }

  /**
   * Store all staged events that were marked as used into persistent storage.
   *
   * Rust: `pub async fn store_used_events(&mut self) -> Result<(), RetrievalError>`
   */
  async storeUsedEvents(): Promise<void> {
    const staged = this.stagedEvents;
    this.stagedEvents = null;

    if (staged !== null) {
      for (const [_id, [event, used]] of staged) {
        if (used) {
          await this.collection.addEvent(event);
        }
      }
    }
  }

  // ── GetEvents implementation ─────────────────────────────────────

  estimateCost(_batchSize: number): number {
    // Default: fixed cost of 1 per batch
    return 1;
  }

  async retrieveEvent(eventIds: EventId[]): Promise<[number, Attested<Event>[]]> {
    const events: Attested<Event>[] = [];
    // Collect remaining IDs that weren't found in staged events
    const remaining: EventId[] = [];

    // First check staged events (zero cost)
    if (this.stagedEvents !== null) {
      for (const id of eventIds) {
        const key = id.toBase64();
        const entry = this.stagedEvents.get(key);
        if (entry !== undefined) {
          const [event] = entry;
          events.push(event);
          entry[1] = true; // mark used
        } else {
          remaining.push(id);
        }
      }
    } else {
      remaining.push(...eventIds);
    }

    if (remaining.length === 0) {
      return [0, events];
    }

    // staged events are free
    // cost for local retrieval is 1 per batch

    // Then retrieve from storage if needed
    const storedEvents = await this.collection.getEvents(remaining);
    events.push(...storedEvents);

    // TODO: push the consumption figure to the store, because its not necessarily the same for all stores
    return [1, events];
  }

  stageEvents(events: Iterable<Attested<Event>>): void {
    if (this.stagedEvents === null) {
      this.stagedEvents = new Map();
    }

    for (const event of events) {
      const key = event.payload.id().toBase64();
      this.stagedEvents.set(key, [event, false]);
    }
  }

  markEventUsed(eventId: EventId): void {
    if (this.stagedEvents === null) {
      this.stagedEvents = new Map();
    }

    const key = eventId.toBase64();
    const entry = this.stagedEvents.get(key);
    if (entry !== undefined) {
      entry[1] = true;
    }
  }

  // ── Retrieve implementation ──────────────────────────────────────

  async getState(entityId: EntityId): Promise<Attested<EntityState> | null> {
    try {
      const state = await this.collection.getState(entityId);
      return state;
    } catch (e: unknown) {
      if (e instanceof RetrievalError && e.kind === 'EntityNotFound') {
        return null;
      }
      throw e;
    }
  }
}

// ---------------------------------------------------------------------------
// NOTE: EphemeralNodeRetriever is NOT ported here.
//
// Rust: `pub struct EphemeralNodeRetriever<'a, SE, PA, C>` (lines 173-325)
// This type is heavily parameterized over SE (StorageEngine), PA (PolicyAgent),
// and C (Iterable<PA::ContextData>) with lifetime 'a. It also depends on
// Node<SE, PA> and proto::NodeRequestBody/NodeResponseBody for remote peer
// fetching. It will be ported when the full Node generic infrastructure and
// remote peer connectivity are available in the TS port.
// ---------------------------------------------------------------------------
