// MIRRORS: ankurah/core/src/system.rs

import {
  CollectionId,
  Attested,
  EntityState,
  Clock,
} from '@ankurah/proto';
import type { Item } from '@ankurah/proto'; // sys::Item
import { Selection } from '@ankurah/ankql';

import { CollectionSet } from './collectionset.ts';
import { Entity, WeakEntitySet } from './entity.ts';
import { MutationError } from './error.ts';
import { PropertyError } from './property/traits.ts';
import { LWWBackend } from './property/backend/lww.ts';
import { Reactor } from './reactor/index.ts';
import { LocalRetriever } from './retrieval.ts';
import type { StorageCollection } from './storage.ts';
import type { Value } from './value/index.ts';

// ── Constants ─────────────────────────────────────────────────────────

export const SYSTEM_COLLECTION_ID = '_ankurah_system';
export const PROTECTED_COLLECTIONS: readonly string[] = [SYSTEM_COLLECTION_ID];

// ── Deferred helper ───────────────────────────────────────────────────

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason: unknown) => void;
}

function createDeferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

// ── sys::Item <-> Value conversion ────────────────────────────────────
// Replaces Rust's `impl Property for proto::sys::Item`
// See Rust: ankurah/core/src/system.rs:300-316

/**
 * Serialize a sys::Item to a Value for storage in an LWW backend.
 *
 * Uses Rust serde_json externally-tagged enum format:
 *   SysRoot -> "SysRoot"
 *   Collection { name } -> {"Collection":{"name":"..."}}
 *   Other -> "Other"
 */
export function sysItemToValue(item: Item): Value | null {
  // Convert TS discriminated union to Rust serde_json externally-tagged format
  let serdeObj: unknown;
  switch (item.type) {
    case 'SysRoot':
      serdeObj = 'SysRoot';
      break;
    case 'Collection':
      serdeObj = { Collection: { name: item.name } };
      break;
    case 'Other':
      serdeObj = 'Other';
      break;
  }
  return { type: 'String', value: JSON.stringify(serdeObj) };
}

/**
 * Deserialize a sys::Item from a Value retrieved from an LWW backend.
 *
 * Parses Rust serde_json externally-tagged enum format back to TS discriminated union.
 */
export function sysItemFromValue(value: Value | null): Item {
  if (value !== null && value.type === 'String') {
    const parsed = JSON.parse(value.value);
    if (parsed === 'SysRoot') return { type: 'SysRoot' };
    if (parsed === 'Other') return { type: 'Other' };
    if (typeof parsed === 'object' && parsed !== null && 'Collection' in parsed) {
      return { type: 'Collection', name: parsed.Collection.name };
    }
  }
  throw PropertyError.invalidValue('', 'sys::Item');
}

// ── SystemManager ─────────────────────────────────────────────────────

/**
 * System catalog manager for storing various metadata about the system:
 * - root clock
 * - valid collections (TODO)
 * - property definitions (TODO)
 *
 * Rust: `pub struct SystemManager<SE, PA>(Arc<Inner<SE, PA>>)`
 * Divergence: No Arc/Inner split -- single-threaded JS, plain class [E8].
 * Divergence: Not generic over SE/PA -- TS uses interfaces directly [E8].
 * Divergence: No RwLock on fields -- single-threaded JS [E8].
 * Divergence: No PhantomData<PA> -- not needed in TS [E8].
 */
export class SystemManager {
  // Divergence: Rust uses Arc<Inner> with separate Inner struct; TS folds all fields into the class [E8].
  private readonly collectionset: CollectionSet;
  private readonly entities: WeakEntitySet;
  private readonly durable: boolean;
  private readonly reactor: Reactor;

  // Divergence: Rust uses RwLock<BTreeMap<CollectionId, Entity>>; TS uses plain Map [E8].
  private readonly collectionMap: Map<string, Entity> = new Map();

  // Divergence: Rust uses RwLock<Option<Attested<EntityState>>>; TS uses plain property [E8].
  private _root: Attested<EntityState> | null = null;

  // Divergence: Rust uses RwLock<Vec<Entity>>; TS uses plain array [E8].
  private _items: Entity[] = [];

  // Divergence: Rust uses OnceLock<()>; TS uses boolean flag [E8].
  private loaded = false;

  // Divergence: Rust uses tokio::sync::Notify; TS uses deferred Promise [E8].
  private loadingDeferred: Deferred<void> = createDeferred<void>();

  // Divergence: Rust uses RwLock<bool>; TS uses plain boolean [E8].
  private systemReady = false;

  // Divergence: Rust uses tokio::sync::Notify; TS uses deferred Promise (resettable) [E8].
  private systemReadyDeferred: Deferred<void> = createDeferred<void>();

  constructor(
    collectionset: CollectionSet,
    entities: WeakEntitySet,
    reactor: Reactor,
    durable: boolean,
  ) {
    this.collectionset = collectionset;
    this.entities = entities;
    this.reactor = reactor;
    this.durable = durable;

    // Divergence: Rust uses crate::task::spawn(async move { ... }); TS uses fire-and-forget promise [E8/A9].
    this.loadSystemCatalog().catch((e) =>
      console.error('Failed to load system catalog:', e),
    );
  }

  // ── root() ────────────────────────────────────────────────────────

  /**
   * Get the root state, if set.
   *
   * Rust: `pub fn root(&self) -> Option<Attested<EntityState>>`
   * Divergence: No clone needed -- JS reference semantics [E8].
   */
  root(): Attested<EntityState> | null {
    return this._root;
  }

  // ── getItems() ────────────────────────────────────────────────────

  /**
   * Get a copy of the items list.
   *
   * Rust: `pub fn items(&self) -> Vec<Entity>`
   * Divergence: Method named getItems() to avoid collision with field [A2].
   */
  getItems(): Entity[] {
    return [...this._items];
  }

  // ── isLoaded() ────────────────────────────────────────────────────

  /**
   * Returns true if the local system catalog is loaded.
   *
   * Rust: `pub fn is_loaded(&self) -> bool`
   */
  isLoaded(): boolean {
    return this.loaded;
  }

  // ── isSystemReady() ───────────────────────────────────────────────

  /**
   * Returns true if we've successfully initialized or joined a system.
   *
   * Rust: `pub fn is_system_ready(&self) -> bool`
   */
  isSystemReady(): boolean {
    return this.systemReady;
  }

  // ── waitLoaded() ──────────────────────────────────────────────────

  /**
   * Waits for the local system catalog to be loaded.
   *
   * Rust: `pub async fn wait_loaded(&self)`
   */
  async waitLoaded(): Promise<void> {
    if (this.loaded) return;
    await this.loadingDeferred.promise;
  }

  // ── waitSystemReady() ─────────────────────────────────────────────

  /**
   * Waits until we've successfully initialized or joined a system.
   *
   * Rust: `pub async fn wait_system_ready(&self)`
   */
  async waitSystemReady(): Promise<void> {
    if (this.systemReady) return;
    await this.systemReadyDeferred.promise;
  }

  // ── collection() ──────────────────────────────────────────────────

  /**
   * Get an existing collection if it's defined in the system catalog,
   * else insert a SysItem::Collection, then return the StorageCollection.
   *
   * Rust: `pub async fn collection(&self, id: &CollectionId) -> Result<StorageCollectionWrapper, RetrievalError>`
   * Throws RetrievalError.
   */
  async collection(id: CollectionId): Promise<StorageCollection> {
    await this.waitLoaded();
    // TODO: update the system catalog to create an entity for this collection
    return this.collectionset.get(id);
  }

  // ── create() ──────────────────────────────────────────────────────

  /**
   * Creates a new system root. This should only be called once per system by durable nodes.
   * The rest of the nodes must "join" this system.
   *
   * Rust: `pub async fn create(&self) -> Result<()>`
   * Throws Error (Rust uses anyhow::Error).
   */
  async create(): Promise<void> {
    if (!this.durable) {
      throw new Error('Only durable nodes can create a new system');
    }

    // Wait for local system catalog to be loaded
    await this.waitLoaded();

    if (this._items.length > 0) {
      throw new Error('System root already exists');
    }

    // TODO - see if we can use the Model derive macro for a SysCatalogItem model rather than doing this manually
    const collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID);
    const storage = await this.collectionset.get(collectionId);

    const systemEntity = this.entities.create(collectionId);

    const lwwBackend = systemEntity.getBackend(LWWBackend);
    lwwBackend.set('item', sysItemToValue({ type: 'SysRoot' }));

    const event = systemEntity.generateCommitEvent();
    if (event === null) {
      throw new Error('Expected event');
    }
    const rootClock: Clock = Clock.fromEventId(event.id());

    // Add the event to storage first
    // Rust: storage.add_event(&event.into()) -- .into() converts Event to Attested<Event> (unattested)
    const attestedEvent = new Attested(event);
    await storage.addEvent(attestedEvent);

    // Update the entity's head clock
    systemEntity.commitHead(rootClock);

    // Now get the entity state after the head is updated
    // Rust: system_entity.to_entity_state()?.into() -- .into() wraps as Attested<EntityState> (unattested)
    const attestedState = new Attested(systemEntity.toEntityState());
    await storage.setState(attestedState);

    // Update our system state
    this._items.push(systemEntity);
    this._root = attestedState;

    // Mark system as ready and notify waiters
    this.systemReady = true;
    this.systemReadyDeferred.resolve(undefined);
  }

  // ── joinSystem() ──────────────────────────────────────────────────

  /**
   * Joins an existing system. This should only be called by ephemeral nodes.
   *
   * Rust: `pub async fn join_system(&self, state: Attested<EntityState>) -> Result<(), MutationError>`
   * Throws MutationError.
   */
  async joinSystem(state: Attested<EntityState>): Promise<void> {
    // Wait for catalog to be loaded before proceeding
    await this.waitLoaded();

    // If node is durable, fail - durable nodes should not join an existing system
    if (this.durable) {
      console.warn('Durable node attempted to join system - this is not allowed');
      throw MutationError.general(
        new Error('Durable nodes cannot join an existing system'),
      );
    }

    const rootState = this.root();

    // If we have a matching root, we're already in sync - just mark ready and return
    if (rootState !== null) {
      if (rootState.payload.state.head.equals(state.payload.state.head)) {
        console.info('Found matching root - Node is part of the same system');
        this.systemReady = true;
        this.systemReadyDeferred.resolve(undefined);
        return;
      }

      console.warn(
        'Mismatched root state during join: local=%s, remote=%s',
        rootState.payload.state.head.toBase64Short(),
        state.payload.state.head.toBase64Short(),
      );

      // Only reset storage if we have a root that needs to be replaced
      console.info('Resetting storage to replace mismatched root');
      // Clear root before reset
      this._root = null;
      try {
        await this.hardReset();
      } catch (e) {
        throw MutationError.general(
          e instanceof Error ? e : new Error(String(e)),
        );
      }
    }

    const collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID);
    const storage = await this.collectionset.get(collectionId);

    // Set the state
    await storage.setState(state);

    // Set root and mark system as ready
    this._root = state;
    this.systemReady = true;
    this.systemReadyDeferred.resolve(undefined);
  }

  // ── hardReset() ───────────────────────────────────────────────────

  /**
   * Resets all storage by deleting all collections, including the system collection.
   * This is used when an ephemeral node needs to join a system with a different root.
   * **This is a destructive operation and should be used with extreme caution.**
   *
   * Rust: `pub async fn hard_reset(&self) -> Result<()>`
   * Throws Error.
   */
  async hardReset(): Promise<void> {
    // Delete all collections from storage
    // Divergence: TS CollectionSet.deleteAllCollections() is synchronous (only clears cache) [E7].
    this.collectionset.deleteAllCollections();

    // Reset our state
    this._items = [];
    this._root = null;
    this.collectionMap.clear();
    this.systemReady = false;

    // Re-create the deferred so future waitSystemReady() calls block again
    this.systemReadyDeferred = createDeferred<void>();

    // Reset the reactor state to notify subscriptions
    this.reactor.systemReset();
  }

  // ── loadSystemCatalog() (private) ─────────────────────────────────

  /**
   * Load the system catalog from storage on startup.
   *
   * Rust: `async fn load_system_catalog(&self) -> Result<()>`
   */
  private async loadSystemCatalog(): Promise<void> {
    if (this.loaded) {
      throw new Error('System catalog already loaded');
    }

    const collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID);
    const storage = await this.collectionset.get(collectionId);

    const entities: Entity[] = [];
    let rootState: Attested<EntityState> | null = null;

    // Fetch all states with a "true" predicate
    const selection = new Selection({ type: 'True' });
    const states = await storage.fetchStates(selection);

    for (const state of states) {
      // Divergence: Rust calls entities.with_state(&retriever, ...) taking a retriever param;
      // TS WeakEntitySet.withState() does not take a retriever (simplified) [E7].
      const [_entityChanged, entity] = this.entities.withState(
        state.payload.entityId,
        collectionId,
        state.payload.state,
      );

      const lwwBackend = entity.getBackend(LWWBackend);
      const value = lwwBackend.get('item');

      if (value !== null) {
        const item = sysItemFromValue(value);

        if (item.type === 'SysRoot') {
          rootState = state;
        }
        entities.push(entity);
      }
    }

    // Update our system state
    this._items.push(...entities);

    // If we loaded a system root and we're a durable node, we're ready
    const hasRoot = rootState !== null;
    this._root = rootState;

    // Only mark ready if we're a durable node and found a root
    // Ephemeral nodes must explicitly join via joinSystem()
    if (hasRoot && this.durable) {
      this.systemReady = true;
      this.systemReadyDeferred.resolve(undefined);
    }

    // Set loaded state and notify waiters
    this.loaded = true;
    this.loadingDeferred.resolve(undefined);
  }
}
