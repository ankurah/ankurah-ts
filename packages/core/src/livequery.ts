// MIRRORS: ankurah/core/src/livequery.rs
// SOURCE-HASH: TODO-run-shasum (compute via: shasum -a 256 ankurah/core/src/livequery.rs)

import type { CollectionId, EntityId, QueryId as QueryIdType, Attested, Event } from '@ankurah/proto';
import { QueryId } from '@ankurah/proto';
import { type Selection, parseSelection } from '@ankurah/ankql';
import {
  Mut,
  Read,
  type Signal,
  type Listener,
  ListenerGuard,
  type BroadcastId,
  SubscriptionGuard,
} from '@ankurah/signals';

import type { Entity } from './entity.ts';
import { Disposable } from '@ankurah/std';
import { RetrievalError } from './error.ts';
import type { ViewInstance, ViewConstructor } from './model.ts';
import { type MatchArgs, Node } from './node.ts';
import type { ItemChange, ChangeSet } from './changes.ts';
import { EntityResultSet } from './resultset.ts';
import {
  type ReactorNodeLike,
  type PreNotifyHook,
  type GapFetcher,
  QueryGapFetcher,
  type ReactorUpdate,
  ReactorSubscription,
} from './reactor/index.ts';
import type { NodeLike } from './reactor/fetch_gap.ts';

// ---------------------------------------------------------------------------
// NodeLikeAdapter -- bridges Node to NodeLike for QueryGapFetcher
// ---------------------------------------------------------------------------

/**
 * Adapter to make Node conform to the NodeLike interface expected by QueryGapFetcher.
 *
 * Rust: QueryGapFetcher::new(node, cdata) uses NodeAndContext which has fetch_entities.
 * Divergence: TS wraps Node + cdata into a NodeLike adapter [E8].
 */
class NodeLikeAdapter implements NodeLike {
  private readonly node: Node;
  private readonly cdata: unknown;

  constructor(node: Node, cdata: unknown) {
    this.node = node;
    this.cdata = cdata;
  }

  async fetchEntities(collectionId: CollectionId, selection: Selection): Promise<Entity[]> {
    return this.node.fetchEntitiesFromLocal(collectionId, selection);
  }
}

// ---------------------------------------------------------------------------
// RemoteQuerySubscriber -- trait stub
// ---------------------------------------------------------------------------

/**
 * Interface for remote query subscriber callbacks.
 *
 * Rust: `pub trait RemoteQuerySubscriber`
 * Used by SubscriptionRelay to notify LiveQuery of remote subscription events.
 * Phase 1: Stubbed -- only WeakEntityLiveQuery implements it.
 */
export interface RemoteQuerySubscriber {
  subscriptionEstablished(version: number): Promise<void>;
  setLastError(error: RetrievalError): void;
}

// ---------------------------------------------------------------------------
// EntityLiveQuery
// ---------------------------------------------------------------------------

/**
 * A type-erased live query that manages reactor subscription and remote cleanup.
 *
 * Rust: `pub struct EntityLiveQuery(Arc<Inner>)`
 * Divergence: No Arc -- JS GC handles shared references [E8].
 * Divergence: impl Drop -> extends Disposable [E11].
 */
export class EntityLiveQuery extends Disposable {
  // -- Fields (mirrors Inner) --
  readonly queryId: QueryIdType;
  private readonly node: Node;
  readonly subscription: ReactorSubscription;       // pub(crate) in Rust
  readonly resultset: EntityResultSet;              // pub(crate) in Rust
  private readonly _error: Mut<RetrievalError | null>;
  private readonly _selection: Mut<{ selection: Selection; version: number }>;
  readonly collectionId: CollectionId;
  private readonly gapFetcher: GapFetcher;
  // Strong reference to the NodeLikeAdapter to prevent GC from collecting it
  // while the QueryGapFetcher holds only a WeakRef.
  // Divergence: Rust Arc prevents drop; TS needs explicit strong ref [E8].
  private readonly _nodeLikeAdapter: NodeLike;

  // -- Initialization tracking --
  // Divergence: Rust uses AtomicU32; TS uses plain number (single-threaded JS) [E8].
  private initializedVersion: number;    // 0 = uninitialized
  private currentVersion: number;        // starts at 1

  // Divergence: Rust uses tokio::sync::Notify; TS uses Promise with stored resolver [E8].
  private _initResolve: (() => void) | null = null;
  private _initPromise: Promise<void>;

  private constructor(
    queryId: QueryIdType,
    node: Node,
    subscription: ReactorSubscription,
    resultset: EntityResultSet,
    error: Mut<RetrievalError | null>,
    selection: Mut<{ selection: Selection; version: number }>,
    collectionId: CollectionId,
    gapFetcher: GapFetcher,
    nodeLikeAdapter: NodeLike,
  ) {
    super('EntityLiveQuery', 'fatal');
    this.queryId = queryId;
    this.node = node;
    this.subscription = subscription;
    this.resultset = resultset;
    this._error = error;
    this._selection = selection;
    this.collectionId = collectionId;
    this.gapFetcher = gapFetcher;
    this._nodeLikeAdapter = nodeLikeAdapter;
    this.initializedVersion = 0;
    this.currentVersion = 1;

    // Divergence: Rust uses tokio::sync::Notify; TS uses resolvable Promise [E8].
    this._initPromise = new Promise<void>((resolve) => {
      this._initResolve = resolve;
    });
  }

  /**
   * Create a new EntityLiveQuery.
   *
   * Rust: `pub fn new<SE, PA>(node, collection_id, args, cdata) -> Result<Self, RetrievalError>`
   * Divergence: Static factory method instead of constructor; throws RetrievalError on failure [A8].
   * Divergence: Rust generic SE/PA type parameters eliminated -- TS Node is concrete [E8].
   */
  static create(
    node: Node,
    collectionId: CollectionId,
    args: MatchArgs,
    cdata: unknown,
  ): EntityLiveQuery {
    // Step 1: Policy check
    // Rust: node.policy_agent.can_access_collection(&cdata, &collection_id)?;
    node.policyAgent.canAccessCollection(cdata, collectionId);

    // Step 2: Filter predicate
    // Rust: args.selection.predicate = node.policy_agent.filter_predicate(&cdata, &collection_id, args.selection.predicate)?;
    if (node.policyAgent.filterPredicate) {
      args.selection.predicate = node.policyAgent.filterPredicate(
        cdata,
        collectionId,
        args.selection.predicate,
      ) as Selection['predicate'];
    }

    // Step 3: Resolve types
    // TODO: type resolver -- args.selection = node.typeResolver.resolveSelectionTypes(args.selection)
    // Divergence: typeResolver does not exist on TS Node yet. Pass selection through unchanged [E8].

    // Step 4: Create subscription
    // Rust: node.reactor.subscribe()
    const subscription = node.reactor.subscribe();

    // Step 5: Create resultset
    const resultset = EntityResultSet.empty();

    // Step 6: Create queryId
    const queryId = QueryId.new();

    // Step 7: Create gapFetcher
    // Rust: Arc::new(QueryGapFetcher::new(&node, cdata.clone()))
    // Divergence: TS QueryGapFetcher takes NodeLike; wrap Node+cdata with adapter [E8].
    // The adapter is stored as a strong ref in EntityLiveQuery to prevent GC while
    // QueryGapFetcher holds only a WeakRef to it.
    const nodeLikeAdapter = new NodeLikeAdapter(node, cdata);
    const gapFetcher: GapFetcher = new QueryGapFetcher(nodeLikeAdapter);

    // Step 8: Construct instance
    const me = new EntityLiveQuery(
      queryId,
      node,
      subscription,
      resultset,
      new Mut<RetrievalError | null>(null),
      new Mut({ selection: args.selection, version: 1 }),
      collectionId,
      gapFetcher,
      nodeLikeAdapter,
    );

    // Step 9: Determine relay status
    // Rust: let has_relay = node.subscription_relay.is_some();
    // TODO: subscription relay -- subscriptionRelay does not exist on TS Node yet.
    // Divergence: For Phase 1, assume hasRelay = false (durable-only path) [E8].
    const hasRelay = false;

    // Step 10: Durable-node initialization (args.cached || !hasRelay)
    // Rust: tokio::spawn(async move { me2.activate(1).await })
    // Divergence: Fire-and-forget microtask -- JS async functions schedule on microtask queue [E8].
    if (args.cached || !hasRelay) {
      console.debug(`LiveQuery::new() spawning initialization task for durable node predicate ${queryId}`);
      void me.activate(1).then(undefined, (e: unknown) => {
        console.debug(`LiveQuery initialization failed for predicate ${queryId}: ${e}`);
        me._error.set(
          e instanceof RetrievalError ? e : RetrievalError.other(String(e)),
        );
      });
    }

    // Step 11: Ephemeral-node path (hasRelay)
    // Rust: node.subscribe_remote_query(query_id, collection_id, args.selection, cdata, 1, me.weak())
    // TODO: subscribe_remote_query -- stub for Phase 1
    if (hasRelay) {
      // TODO: node.subscribeRemoteQuery(queryId, collectionId, args.selection, cdata, 1, me.weak());
    }

    // Step 12: Return
    return me;
  }

  // ── map<V> ──────────────────────────────────────────────────────────

  /**
   * Wrap this EntityLiveQuery with a typed View.
   *
   * Rust: `pub fn map<R: View>(self) -> LiveQuery<R> { LiveQuery(self, PhantomData) }`
   * Divergence: Uses ViewConstructor<V> instead of PhantomData [E8].
   */
  map<V extends ViewInstance>(viewCtor: ViewConstructor<V>): LiveQuery<V> {
    return new LiveQuery(this, viewCtor);
  }

  // ── wait_initialized ────────────────────────────────────────────────

  /**
   * Wait for the LiveQuery to be fully initialized with initial states.
   *
   * Rust: `pub async fn wait_initialized(&self)`
   * Divergence: Uses Promise-based resolution instead of tokio::sync::Notify [E8].
   */
  async waitInitialized(): Promise<void> {
    // Divergence: Rust uses AtomicU32 load with Relaxed ordering; TS uses plain number [E8].
    if (this.initializedVersion >= this.currentVersion) {
      return;
    }
    // FIXME - this should be waiting for the correct version, not any version
    await this._initPromise;
  }

  // ── update_selection ────────────────────────────────────────────────

  /**
   * Update the selection predicate for this live query.
   *
   * Rust: `pub fn update_selection(&self, new_selection: impl TryInto<Selection>) -> Result<(), RetrievalError>`
   * Divergence: Accepts Selection | string instead of TryInto<Selection> [E8].
   */
  updateSelection(newSelection: Selection | string): void {
    let parsed: Selection;
    if (typeof newSelection === 'string') {
      try {
        parsed = parseSelection(newSelection);
      } catch (e) {
        throw RetrievalError.other(`Failed to parse selection: ${e}`);
      }
    } else {
      parsed = newSelection;
    }

    // Increment current_version
    // Divergence: Rust uses AtomicU32::fetch_add with SeqCst; TS uses plain increment [E8].
    this.currentVersion += 1;
    const newVersion = this.currentVersion;

    // Mark resultset as not loaded since we're changing the selection
    this.resultset.setLoaded(false);

    // Store new selection and version
    this._selection.set({ selection: parsed, version: newVersion });

    // Check relay status
    // TODO: subscription relay -- stub for Phase 1
    // Divergence: hasRelay always false in Phase 1 [E8].
    const hasRelay = false;

    if (hasRelay) {
      // Ephemeral node: delegate to relay
      // TODO: this.node.updateRemoteQuery(this.queryId, parsed, newVersion);
    } else {
      // Durable node: spawn task to call activate directly
      // Divergence: Fire-and-forget microtask instead of tokio::spawn [E8].
      void this.activate(newVersion).then(undefined, (e: unknown) => {
        console.error(`LiveQuery update failed for predicate ${this.queryId}: ${e}`);
        this._error.set(
          e instanceof RetrievalError ? e : RetrievalError.other(String(e)),
        );
      });
    }
  }

  // ── update_selection_wait ───────────────────────────────────────────

  /**
   * Update the selection and wait for the update to complete.
   *
   * Rust: `pub async fn update_selection_wait(&self, new_selection) -> Result<(), RetrievalError>`
   */
  async updateSelectionWait(newSelection: Selection | string): Promise<void> {
    this.updateSelection(newSelection);
    await this.waitInitialized();
  }

  // ── activate ────────────────────────────────────────────────────────

  /**
   * Activate the LiveQuery by fetching entities and calling reactor.add_query or reactor.update_query.
   *
   * Rust: `async fn activate(&self, version: u32) -> Result<(), RetrievalError>`
   */
  private async activate(version: number): Promise<void> {
    // Get the current selection and its version
    // Rust: let (selection, stored_version) = self.0.selection.value();
    const { selection, version: storedVersion } = this._selection.peek();

    // Reject stale activation
    if (version < storedVersion) {
      console.warn(
        `LiveQuery - Dropped stale activation request for version ${version} (current version is ${storedVersion})`,
      );
      return;
    }

    console.debug(`LiveQuery.activate() for predicate ${this.queryId} (version ${version})`);

    // Divergence: Rust uses self.0.node.reactor() trait method; TS accesses this.node.reactor directly [E8].
    const reactor = this.node.reactor;
    const initVer = this.initializedVersion;

    // Rust: PreNotifyHook is impl'd for &EntityLiveQuery
    // Divergence: TS passes a closure instead of trait impl [E8].
    const preNotifyHook: PreNotifyHook = (v: number) => this.markInitialized(v);

    if (initVer === 0) {
      // First activation: call reactor.addQueryAndNotify
      await reactor.addQueryAndNotify(
        this.subscription.id(),
        this.queryId,
        this.collectionId,
        selection,
        this.node as ReactorNodeLike,  // Node structurally conforms to ReactorNodeLike
        this.resultset,
        this.gapFetcher,
        preNotifyHook,
      );
    } else {
      // Subsequent activation: call reactor.updateQueryAndNotify
      await reactor.updateQueryAndNotify(
        this.subscription.id(),
        this.queryId,
        this.collectionId,
        selection,
        this.node as ReactorNodeLike,
        version,
        preNotifyHook,
      );
    }
  }

  // ── Accessor methods ────────────────────────────────────────────────

  /**
   * Get the error signal.
   *
   * Rust: `pub fn error(&self) -> Read<Option<RetrievalError>>`
   */
  error(): Read<RetrievalError | null> {
    return this._error.read();
  }

  /**
   * Get the selection signal.
   *
   * Rust: `pub fn selection(&self) -> Read<(Selection, u32)>`
   * Divergence: Rust tuple becomes object { selection, version } [E8].
   */
  selection(): Read<{ selection: Selection; version: number }> {
    return this._selection.read();
  }

  /**
   * Create a weak reference to this LiveQuery.
   *
   * Rust: `pub fn weak(&self) -> WeakEntityLiveQuery`
   */
  weak(): WeakEntityLiveQuery {
    return new WeakEntityLiveQuery(this);
  }

  /**
   * Mark initialization as complete for a given version.
   *
   * Rust: `pub fn mark_initialized(&self, version: u32)`
   */
  markInitialized(version: number): void {
    // TASK: Serialize or coalesce concurrent activations to prevent version regression
    // https://github.com/ankurah/ankurah/issues/146
    // Divergence: Rust uses AtomicU32::store with Relaxed; TS uses plain assignment [E8].
    this.initializedVersion = version;
    // Resolve the current promise
    // Divergence: Rust uses Notify::notify_waiters; TS resolves stored Promise [E8].
    if (this._initResolve) {
      this._initResolve();
    }
    // Create a new promise for the next wait cycle
    this._initPromise = new Promise<void>((resolve) => {
      this._initResolve = resolve;
    });
  }

  // ── Internal helpers for WeakEntityLiveQuery ────────────────────────

  /** @internal */
  _activateInternal(version: number): Promise<void> {
    return this.activate(version);
  }

  /** @internal */
  _setError(error: RetrievalError): void {
    this._error.set(error);
  }

  // ── Cleanup (mirrors Rust Drop) ─────────────────────────────────────

  /**
   * Rust: `impl Drop for Inner { fn drop(&mut self) { self.node.unsubscribe_remote_predicate(self.query_id); } }`
   */
  protected onDispose(): void {
    // Unsubscribe from remote predicate
    // TODO: this.node.unsubscribeRemotePredicate(this.queryId) -- stub for Phase 1

    // Clean up reactor subscription
    this.subscription.dispose();
  }
}

// ---------------------------------------------------------------------------
// WeakEntityLiveQuery
// ---------------------------------------------------------------------------

/**
 * Weak reference to EntityLiveQuery for breaking circular dependencies.
 *
 * Rust: `pub struct WeakEntityLiveQuery(Weak<Inner>)`
 * Divergence: Uses JS WeakRef instead of Rust Weak<Arc<Inner>> [E8].
 */
export class WeakEntityLiveQuery implements RemoteQuerySubscriber {
  private readonly ref: WeakRef<EntityLiveQuery>;

  constructor(liveQuery: EntityLiveQuery) {
    this.ref = new WeakRef(liveQuery);
  }

  /**
   * Attempt to upgrade the weak reference.
   *
   * Rust: `pub fn upgrade(&self) -> Option<EntityLiveQuery>`
   * Divergence: Returns null instead of None [E8].
   */
  upgrade(): EntityLiveQuery | null {
    return this.ref.deref() ?? null;
  }

  // ── RemoteQuerySubscriber implementation ────────────────────────────

  /**
   * Called when the remote subscription is established.
   *
   * Rust: `async fn subscription_established(&self, version: u32)`
   */
  async subscriptionEstablished(version: number): Promise<void> {
    const liveQuery = this.upgrade();
    if (liveQuery) {
      try {
        await liveQuery._activateInternal(version);
      } catch (e) {
        liveQuery._setError(
          e instanceof RetrievalError ? e : RetrievalError.other(String(e)),
        );
      }
    }
    // If upgrade fails, the LiveQuery was already dropped - nothing to do
  }

  /**
   * Set the last error on the LiveQuery.
   *
   * Rust: `fn set_last_error(&self, error: RetrievalError)`
   */
  setLastError(error: RetrievalError): void {
    const liveQuery = this.upgrade();
    if (liveQuery) {
      liveQuery._setError(error);
    }
    // If upgrade fails, the LiveQuery was already dropped - nothing to do
  }
}

// ---------------------------------------------------------------------------
// LiveQuery<V> -- Generic Typed Wrapper
// ---------------------------------------------------------------------------

/**
 * A typed live query that wraps EntityLiveQuery with a specific View type.
 *
 * Rust: `pub struct LiveQuery<R: View>(EntityLiveQuery, PhantomData<R>)`
 * Divergence: Uses ViewConstructor<V> instead of PhantomData [E8].
 * Divergence: No Deref -- delegates explicitly [E8].
 */
export class LiveQuery<V extends ViewInstance> extends Disposable implements Signal {
  readonly inner: EntityLiveQuery;
  private readonly viewCtor: ViewConstructor<V>;

  constructor(inner: EntityLiveQuery, viewCtor: ViewConstructor<V>) {
    super('LiveQuery', 'fatal');
    this.inner = inner;
    this.viewCtor = viewCtor;
  }

  // ── Delegated methods ───────────────────────────────────────────────

  /**
   * Wait for the LiveQuery to be fully initialized with initial states.
   *
   * Rust: `pub async fn wait_initialized(&self) { self.0.wait_initialized().await; }`
   */
  async waitInitialized(): Promise<void> {
    return this.inner.waitInitialized();
  }

  /**
   * Check if the resultset is loaded.
   *
   * Rust: `pub fn loaded(&self) -> bool { self.0.0.resultset.is_loaded() }`
   */
  loaded(): boolean {
    return this.inner.resultset.isLoaded();
  }

  /**
   * Get the entity IDs in the resultset.
   *
   * Rust: `pub fn ids(&self) -> Vec<proto::EntityId>`
   */
  ids(): EntityId[] {
    return this.inner.resultset.keys();
  }

  /**
   * Get the entity IDs in sorted order.
   *
   * Rust: `pub fn ids_sorted(&self) -> Vec<proto::EntityId>`
   * Divergence: Uses compareEntityIds helper instead of Itertools::sorted [E8].
   */
  idsSorted(): EntityId[] {
    return this.inner.resultset.keys().sort((a, b) => {
      const aBytes = a.toBytes();
      const bBytes = b.toBytes();
      const len = Math.min(aBytes.length, bBytes.length);
      for (let i = 0; i < len; i++) {
        if (aBytes[i] < bBytes[i]) return -1;
        if (aBytes[i] > bBytes[i]) return 1;
      }
      return aBytes.length - bBytes.length;
    });
  }

  // ── Signal implementation ───────────────────────────────────────────
  // Rust: impl<R: View> Signal for LiveQuery<R>
  // Delegates to the subscription (not resultset).
  // This ensures that LiveQuery tracking fires on ALL entity changes.

  /**
   * Listen to changes (notify-only, no payload).
   *
   * Rust: `fn listen(&self, listener: Listener) -> ListenerGuard { self.0.0.subscription.listen(listener) }`
   */
  listen(listener: Listener): ListenerGuard {
    return this.inner.subscription.listen(listener);
  }

  /**
   * Get the broadcast identifier for this signal.
   *
   * Rust: `fn broadcast_id(&self) -> BroadcastId { self.0.0.subscription.broadcast_id() }`
   */
  broadcastId(): BroadcastId {
    return this.inner.subscription.broadcastId();
  }

  // ── Get<V[]> ────────────────────────────────────────────────────────

  /**
   * Get the current view items with observer tracking.
   *
   * Rust: `impl<R: View + Clone + 'static> Get<Vec<R>> for LiveQuery<R>`
   */
  get(): V[] {
    // TODO: CurrentObserver.track(this) when observer system is ported
    return this.peek();
  }

  // ── Peek<V[]> ───────────────────────────────────────────────────────

  /**
   * Get the current view items without observer tracking.
   *
   * Rust: `impl<R: View + Clone + 'static> Peek<Vec<R>> for LiveQuery<R>`
   */
  peek(): V[] {
    const read = this.inner.resultset.read();
    return read.iterEntities().map(([_id, entity]) =>
      this.viewCtor.fromEntity(entity),
    );
  }

  // ── Subscribe<ChangeSet<V>> ─────────────────────────────────────────

  /**
   * Subscribe to change notifications.
   *
   * Rust: `impl<R: View> Subscribe<ChangeSet<R>> for LiveQuery<R>`
   * Divergence: Direct callback instead of IntoSubscribeListener trait [E8].
   */
  subscribe(listener: (changeset: ChangeSet<V>) => void): SubscriptionGuard {
    return this.inner.subscription.subscribe((reactorUpdate: ReactorUpdate) => {
      const changeset = liveQueryChangeSetFrom(
        this.inner.resultset,
        this.viewCtor,
        reactorUpdate,
      );
      listener(changeset);
    });
  }

  // ── Cleanup delegation ──────────────────────────────────────────────

  protected onDispose(): void {
    this.inner.dispose();
  }
}

// ---------------------------------------------------------------------------
// liveQueryChangeSetFrom -- free function
// ---------------------------------------------------------------------------

/**
 * Convert a ReactorUpdate to a ChangeSet<V> for a single-predicate LiveQuery subscription.
 *
 * Rust: `fn livequery_change_set_from<R: View>(resultset: ResultSet<R>, reactor_update: ReactorUpdate) -> ChangeSet<R>`
 * Divergence: Takes EntityResultSet + ViewConstructor instead of ResultSet<R> [E8].
 *
 * Notably, this function does not filter by query_id, because it should only be used
 * by LiveQuery, which entails a single-predicate subscription.
 */
function liveQueryChangeSetFrom<V extends ViewInstance>(
  resultset: EntityResultSet,
  viewCtor: ViewConstructor<V>,
  reactorUpdate: ReactorUpdate,
): ChangeSet<V> {
  const changes: ItemChange<V>[] = [];

  for (const item of reactorUpdate.items) {
    const view = viewCtor.fromEntity(item.entity);

    // Single-predicate subscription: take first predicate_relevance entry
    // Ignore the query_id, because this should only be used by LiveQuery,
    // which entails a single-predicate subscription
    if (item.predicateRelevance.length > 0) {
      const [_queryId, membershipChange] = item.predicateRelevance[0];

      switch (membershipChange) {
        case 'Initial':
          changes.push({ kind: 'Initial', item: view });
          break;
        case 'Add':
          changes.push({ kind: 'Add', item: view, events: item.events });
          break;
        case 'Remove':
          changes.push({ kind: 'Remove', item: view, events: item.events });
          break;
      }
    } else {
      // No membership change -- just an update
      changes.push({ kind: 'Update', item: view, events: item.events });
    }
  }

  return { changes, resultset };
}
