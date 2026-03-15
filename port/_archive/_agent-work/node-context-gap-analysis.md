# Node & Context Gap Analysis: Reactor Integration, Query/Subscribe, LiveQuery Support

**Source files analyzed:**
- Rust: `/Users/daniel/ak/ankurah/core/src/node.rs`, `/Users/daniel/ak/ankurah/core/src/context.rs`
- TS: `/Users/daniel/ak/ankurah-ts/packages/core/src/node.ts`, `/Users/daniel/ak/ankurah-ts/packages/core/src/context.ts`
- LiveQuery spec: `/Users/daniel/ak/ankurah-ts/specs/livequery-port-spec.md`
- Supporting: reactor/index.ts, changes.ts, system.ts, policy.ts, connector.ts, reactor/subscription.ts, reactor/fetch_gap.ts, resultset.ts, collectionset.ts, storage.ts

**Date:** 2026-02-11

---

## Part 1: Node Reactor Integration

### 1.1 MISSING: `reactor` field on Node

**Rust** (node.rs line 138):
```rust
pub(crate) reactor: Reactor,
```
Created in `Node::new()` (line 157) via `Reactor::new()`. Shared with `SystemManager` (line 160). Used for:
- `self.reactor.notify_change(changes)` in `commit_remote_transaction` (node.rs line 591)
- `self.node.reactor.notify_change(changes)` in `commit_local_trx` (context.rs line 341)
- `self.node.reactor.subscribe()` in `EntityLiveQuery::new` (livequery.rs line 100)
- `&self.0.reactor` returned from `TNodeErased::reactor()` (node.rs line 877)

**Current TS state:** `Node` class has no `reactor` field. The Reactor class is fully ported at `/Users/daniel/ak/ankurah-ts/packages/core/src/reactor/index.ts`.

**Required change:** Add `readonly reactor: Reactor` field to `Node`. Initialize in constructor:
```typescript
import { Reactor } from './reactor/index.ts';

// In constructor:
this.reactor = new Reactor();
```

**Downstream impact:** Enables Phase 7 in `commitLocalTrx`, `EntityLiveQuery.create()`, and `TNodeErased` implementation.

---

### 1.2 MISSING: `collections` field (CollectionSet) on Node

**Rust** (node.rs line 129):
```rust
pub collections: CollectionSet<SE>,
```
Created in `Node::new()` (line 154): `CollectionSet::new(engine.clone())`. Used for direct storage access: `self.collections.get(&collection_id)` in `commit_remote_transaction`, `fetch_entities_from_local`, `handle_request`, etc. Also passed to `SystemManager::new()`.

**Current TS state:** `Node` stores `storageEngine: StorageEngine` directly and calls `this.storageEngine.collection(collectionId)`. The `CollectionSet` class already exists at `/Users/daniel/ak/ankurah-ts/packages/core/src/collectionset.ts`.

**Required change:** Replace `storageEngine: StorageEngine` with `collections: CollectionSet`. Create in constructor:
```typescript
import { CollectionSet } from './collectionset.ts';

// In constructor:
this.collections = new CollectionSet(options.storageEngine);
```

**Impact on existing code:** `fetchEntitiesFromLocal()` currently calls `this.storageEngine.collection(collectionId)`. Change to `this.collections.get(collectionId)`. The `NodeAndContext.commitLocalTrx()` calls `this.node.storageEngine.collection(...)` -- change to `this.node.collections.get(...)`.

**Note:** `CollectionSet.get()` is synchronous (returns `StorageCollection`) while `StorageEngine.collection()` is async (returns `Promise<StorageCollection>`). The current TS `CollectionSet.get()` is synchronous. The Rust `CollectionSet.get()` is async. Verify the TS `CollectionSet.get()` signature -- if it returns a `Promise`, then `await` is needed; if synchronous, the callers that currently await need to be adjusted. Looking at `collectionset.ts` line 30+, `get()` returns `StorageCollection` synchronously (it lazily creates and caches). This is simpler than Rust and means callers can drop the `await`.

---

### 1.3 MISSING: `system` field (SystemManager) on Node

**Rust** (node.rs line 140):
```rust
pub system: SystemManager<SE, PA>,
```
Created in `Node::new()` (line 160):
```rust
let system_manager = SystemManager::new(collections.clone(), entityset.clone(), reactor.clone(), false);
```

Used for:
- `self.system.is_system_ready()` / `self.system.wait_system_ready()` in `context()` / `context_async()`
- `self.node.system.collection(id)` in TContext `collection()` impl (context.rs line 74)
- `self.system.join_system(system_root)` in `register_peer()` (node.rs line 244)

**Current TS state:** `Node` has no `system` field. The `SystemManager` class is fully ported at `/Users/daniel/ak/ankurah-ts/packages/core/src/system.ts`.

**Required change:** Add `readonly system: SystemManager` field to `Node`. Initialize in constructor:
```typescript
import { SystemManager } from './system.ts';

// In constructor:
this.system = new SystemManager(this.collections, this.entities, this.reactor, this.durable);
```

**Note:** `SystemManager` constructor spawns `loadSystemCatalog()` asynchronously on construction (fire-and-forget promise). This matches Rust.

---

### 1.4 MISSING: System readiness check in `context()` and `contextAsync()` method

**Rust** (node.rs lines 693-703):
```rust
pub fn context(&self, data: PA::ContextData) -> Result<Context, anyhow::Error> {
    if !self.system.is_system_ready() {
        return Err(anyhow!("System is not ready"));
    }
    Ok(Context::new(Node::clone(self), data))
}

pub async fn context_async(&self, data: PA::ContextData) -> Context {
    self.system.wait_system_ready().await;
    Context::new(Node::clone(self), data)
}
```

**Current TS state:** `Node.context()` (line 100) creates a context unconditionally with no readiness check. No `contextAsync()` method exists.

**Required changes:**

1. Update `context()` to check system readiness:
```typescript
context(contextData?: unknown): Context {
  if (!this.system.isSystemReady()) {
    throw new Error('System is not ready');
  }
  const cdata = contextData ?? this.defaultContextData;
  const nodeContext = new NodeAndContext(this, cdata);
  return new Context(nodeContext);
}
```

2. Add `contextAsync()`:
```typescript
async contextAsync(contextData?: unknown): Promise<Context> {
  await this.system.waitSystemReady();
  const cdata = contextData ?? this.defaultContextData;
  const nodeContext = new NodeAndContext(this, cdata);
  return new Context(nodeContext);
}
```

---

### 1.5 Phase 7: Reactor Notification in `commitLocalTrx()`

**Rust** (context.rs lines 315-341):
```rust
// All peers confirmed, persist state to storage
let mut changes: Vec<EntityChange> = Vec::new();
for (entity, attested_event) in entity_attested_events {
    // ... persist canonical entity state ...
    changes.push(EntityChange::new(canonical_entity, vec![attested_event])?);
}

// Notify reactor of ALL changes
self.node.reactor.notify_change(changes).await;
```

**Current TS state** (node.ts lines 297-326): Phase 5 persists state correctly but then has:
```typescript
// Phase 6: Peer replication -- deferred until connector layer is ported
// Phase 7: Reactor notification -- deferred until reactor is ported
```

**Required change:** After the existing Phase 5 loop (which persists canonical state), accumulate `EntityChange` objects and call `reactor.notifyChange()`:

```typescript
// Phase 7: Notify reactor of ALL changes
const changes: EntityChange[] = [];
// (Move EntityChange creation into the Phase 5 loop, or accumulate after)
```

**Detailed implementation:** The existing Phase 5 loop (lines 298-322) iterates `attestedEvents` and persists state. At the end of each iteration, add:
```typescript
changes.push(EntityChange.create(canonicalEntity, [attested]));
```

After the loop:
```typescript
// Phase 7: Notify reactor of ALL changes
await this.node.reactor.notifyChange(changes);
```

**Imports needed in node.ts:** `EntityChange` is already imported at line 20 (`import type { EntityChange } from './changes.ts';`) but needs to be a value import since `EntityChange.create()` is called:
```typescript
import { EntityChange } from './changes.ts';
```

**Note on `require()` calls:** The current TS code uses `require('@ankurah/proto')` on lines 285 and 319 for `Attested`. These should be converted to proper imports at the top of the file to avoid runtime issues in ESM environments. The `Attested` class is already imported as a type on line 8 -- it needs to be a value import.

---

### 1.6 MISSING: Node implementing `TNodeErased` / expanded `ReactorNodeLike`

**Rust** (node.rs lines 831-880):
```rust
pub trait TNodeErased<E = Entity>: Send + Sync + 'static {
    fn unsubscribe_remote_predicate(&self, query_id: proto::QueryId);
    fn update_remote_query(&self, query_id, selection, version) -> Result<(), anyhow::Error>;
    async fn fetch_entities_from_local(&self, collection_id, selection) -> Result<Vec<E>, RetrievalError>;
    fn reactor(&self) -> &Reactor<E>;
    fn has_subscription_relay(&self) -> bool;
}
```

Node implements all 5 methods (lines 845-880).

**Current TS state:** The `ReactorNodeLike` interface in `reactor/index.ts` (lines 86-91) only exposes:
```typescript
export interface ReactorNodeLike {
  fetchEntitiesFromLocal(collectionId: CollectionId, selection: Selection): Promise<Entity[]>;
}
```

`Node.fetchEntitiesFromLocal()` exists and satisfies this minimal interface. But `EntityLiveQuery` needs the full `TNodeErased`:
- `reactor()` -- to call `reactor.subscribe()`, `reactor.addQueryAndNotify()`, `reactor.updateQueryAndNotify()`
- `hasSubscriptionRelay()` -- to decide durable vs ephemeral initialization path
- `unsubscribeRemotePredicate()` -- called by `EntityLiveQuery.dispose()`
- `updateRemoteQuery()` -- called by `EntityLiveQuery.updateSelection()`

**Required changes:**

1. Expand `ReactorNodeLike` (or create a new `TNodeErased` interface):

```typescript
// In reactor/index.ts or node.ts
export interface TNodeErased {
  /** Fetch entities from local storage. */
  fetchEntitiesFromLocal(collectionId: CollectionId, selection: Selection): Promise<Entity[]>;

  /** Get the reactor instance. */
  reactor: Reactor;  // Rust: fn reactor(&self) -> &Reactor<E>

  /** Whether this node has a subscription relay (ephemeral nodes do, durable don't). */
  hasSubscriptionRelay(): boolean;

  /**
   * Unsubscribe a remote predicate (cleanup when LiveQuery is disposed).
   * Rust: fn unsubscribe_remote_predicate(&self, query_id)
   */
  unsubscribeRemotePredicate(queryId: QueryId): void;

  /**
   * Update a remote query's selection (for LiveQuery selection changes on ephemeral nodes).
   * Rust: fn update_remote_query(&self, query_id, selection, version) -> Result<()>
   */
  updateRemoteQuery(queryId: QueryId, selection: Selection, version: number): void;
}
```

2. Implement on `Node`:
```typescript
// Phase 1 stubs for remote methods:
hasSubscriptionRelay(): boolean {
  return false;  // Phase 1: durable-only, no relay
}

unsubscribeRemotePredicate(_queryId: QueryId): void {
  // Phase 1: no-op (no subscription relay)
  // TODO: Clean up predicate_context and notify subscription relay
}

updateRemoteQuery(_queryId: QueryId, _selection: Selection, _version: number): void {
  // Phase 1: no-op (no subscription relay)
  // TODO: Resolve types, notify subscription relay
}
```

**Where to define:** The `TNodeErased` interface should be defined in `node.ts` (not reactor/index.ts) since it represents a Node abstraction. The existing `ReactorNodeLike` in `reactor/index.ts` can be replaced or kept as a subset. The livequery-port-spec.md (Section 17) expects these methods to be on `Node` directly.

---

### 1.7 MISSING: `subscriptionRelay` field (Phase 1 stub)

**Rust** (node.rs lines 142-143):
```rust
pub(crate) subscription_relay: Option<SubscriptionRelay<PA::ContextData, WeakEntityLiveQuery>>,
```
`Some(SubscriptionRelay::new())` for ephemeral nodes, `None` for durable.

**Current TS state:** Not present.

**Required change (Phase 1):** Add a stub field that is always `null`:
```typescript
// Phase 1: Subscription relay not yet ported. Always null (durable-only mode).
readonly subscriptionRelay: null = null;
```

This satisfies `hasSubscriptionRelay()` returning `false` and allows the livequery spec's durable-only path to function.

---

### 1.8 MISSING: `predicateContext` field (Phase 1 stub)

**Rust** (node.rs line 136):
```rust
pub(crate) predicate_context: SafeMap<proto::QueryId, PA::ContextData>,
```

**Required change (Phase 1):** Stub for completeness -- not needed until remote subscriptions:
```typescript
// Phase 1 stub: context data per query for remote subscription cleanup
private readonly predicateContext: Map<string, unknown> = new Map();
```

---

### 1.9 MISSING: `subscribeRemoteQuery()` method (Phase 1 stub)

**Rust** (node.rs lines 797-812):
```rust
pub(crate) fn subscribe_remote_query(&self, query_id, collection_id, selection, cdata, version, livequery)
```

Called by `EntityLiveQuery::new()` for ephemeral nodes.

**Required change (Phase 1):** Stub that does nothing (ephemeral path won't be reached while `hasSubscriptionRelay()` returns `false`):
```typescript
/** @internal Phase 1 stub -- no-op until subscription relay is ported. */
subscribeRemoteQuery(
  _queryId: QueryId,
  _collectionId: CollectionId,
  _selection: Selection,
  _cdata: unknown,
  _version: number,
  _livequery: unknown,  // WeakEntityLiveQuery when livequery is ported
): void {
  // No-op: subscription relay not yet available
}
```

---

### 1.10 Node Constructor Refactoring Summary

The current `Node` constructor (node.ts lines 80-93) must be updated to create `CollectionSet`, `Reactor`, and `SystemManager`. Here is the complete required constructor shape:

```typescript
constructor(options: {
  id?: EntityId;
  durable?: boolean;
  storageEngine: StorageEngine;
  policyAgent: PolicyAgent<unknown>;
  contextData?: unknown;
}) {
  this.id = options.id ?? EntityIdClass.new();
  this.durable = options.durable ?? false;
  this.entities = new WeakEntitySet();
  this.collections = new CollectionSet(options.storageEngine);
  this.reactor = new Reactor();
  this.system = new SystemManager(this.collections, this.entities, this.reactor, this.durable);
  this.policyAgent = options.policyAgent;
  this.defaultContextData = options.contextData ?? null;
  this.subscriptionRelay = null;  // Phase 1 stub
}
```

**New imports at top of node.ts:**
```typescript
import { Reactor } from './reactor/index.ts';
import { SystemManager } from './system.ts';
import { CollectionSet } from './collectionset.ts';
import type { QueryId } from '@ankurah/proto';
import type { Selection } from '@ankurah/ankql';
```

---

## Part 2: Context -- query() and subscribe() Methods

### 2.1 MISSING: `query()` method on TContext interface

**Rust** (context.rs line 50):
```rust
fn query(&self, collection_id: CollectionId, args: MatchArgs) -> Result<EntityLiveQuery, RetrievalError>;
```

**Rust implementation** (context.rs lines 70-72):
```rust
fn query(&self, collection_id: CollectionId, args: MatchArgs) -> Result<EntityLiveQuery, RetrievalError> {
    EntityLiveQuery::new(&self.node, collection_id, args, self.cdata.clone())
}
```

**Current TS state:** `TContext` interface (context.ts lines 23-75) has no `query()` method.

**Required change:** Add to `TContext`:
```typescript
/**
 * Create a live query for entities matching a selection.
 *
 * Rust: `fn query(&self, collection_id, args) -> Result<EntityLiveQuery, RetrievalError>`
 */
query(collectionId: CollectionId, args: MatchArgs): EntityLiveQuery;
```

Implement in `NodeAndContext` (node.ts):
```typescript
query(collectionId: CollectionId, args: MatchArgs): EntityLiveQuery {
  return EntityLiveQuery.create(this.node, collectionId, args, this.cdata);
}
```

**Import needed in node.ts:**
```typescript
import { EntityLiveQuery } from './livequery.ts';
```

---

### 2.2 MISSING: `collection()` method on TContext interface

**Rust** (context.rs line 51):
```rust
async fn collection(&self, id: &proto::CollectionId) -> Result<StorageCollectionWrapper, RetrievalError>;
```

**Rust implementation** (context.rs lines 73-75):
```rust
async fn collection(&self, id: &proto::CollectionId) -> Result<StorageCollectionWrapper, RetrievalError> {
    self.node.system.collection(id).await
}
```

**Current TS state:** Not on `TContext`.

**Required change:** Add to `TContext`:
```typescript
/**
 * Get a storage collection handle via the system catalog.
 *
 * Rust: `async fn collection(&self, id) -> Result<StorageCollectionWrapper, RetrievalError>`
 */
collection(id: CollectionId): Promise<StorageCollection>;
```

Implement in `NodeAndContext`:
```typescript
async collection(id: CollectionId): Promise<StorageCollection> {
  return this.node.system.collection(id);
}
```

---

### 2.3 MISSING: Context public API methods

**Rust** (context.rs lines 116-167):

| Rust method | TS signature | Description |
|---|---|---|
| `get<R: View>(id)` | `get<V>(ViewCtor, id): Promise<V>` | Get single entity by ID, wrap as view |
| `get_cached<R: View>(id)` | `getCached<V>(ViewCtor, id): Promise<V>` | Same but allows cached (local) result |
| `fetch<R: View>(args)` | `fetch<V>(ViewCtor, args): Promise<V[]>` | Fetch entities matching selection |
| `fetch_one<R: View>(args)` | `fetchOne<V>(ViewCtor, args): Promise<V \| null>` | Fetch first matching entity |
| `query<R: View>(args)` | `query<V>(ViewCtor, args): LiveQuery<V>` | Create reactive live query |
| `query_wait<R: View>(args)` | `queryWait<V>(ViewCtor, args): Promise<LiveQuery<V>>` | Create live query, wait for init |
| `collection(id)` | `collection(id): Promise<StorageCollection>` | Get storage collection handle |

**Current TS state:** `Context` class (context.ts lines 95-128) only has `nodeId()`, `begin()`, and a `context` getter. None of the read/query methods exist.

**Required changes to Context class:**

```typescript
import type { ViewConstructor, ViewInstance } from './model.ts';
import type { EntityLiveQuery } from './livequery.ts';
import { LiveQuery } from './livequery.ts';
import type { MatchArgs } from './node.ts';
import type { StorageCollection } from './storage.ts';

export class Context {
  // ... existing constructor, nodeId(), begin(), context getter ...

  /**
   * Get a single entity by ID, returning a typed view.
   * Rust: pub async fn get<R: View>(&self, id) -> Result<R, RetrievalError>
   */
  async get<V extends ViewInstance>(
    viewCtor: ViewConstructor<V>,
    id: EntityId,
  ): Promise<V> {
    const entity = await this.inner.getEntity(id, viewCtor.collection(), false);
    return viewCtor.fromEntity(entity);
  }

  /**
   * Get a cached entity by ID (local storage only, no peer fetch).
   * Rust: pub async fn get_cached<R: View>(&self, id) -> Result<R, RetrievalError>
   */
  async getCached<V extends ViewInstance>(
    viewCtor: ViewConstructor<V>,
    id: EntityId,
  ): Promise<V> {
    const entity = await this.inner.getEntity(id, viewCtor.collection(), true);
    return viewCtor.fromEntity(entity);
  }

  /**
   * Fetch entities matching a selection.
   * Rust: pub async fn fetch<R: View>(&self, args) -> Result<Vec<R>, RetrievalError>
   */
  async fetch<V extends ViewInstance>(
    viewCtor: ViewConstructor<V>,
    args: MatchArgs,
  ): Promise<V[]> {
    const collectionId = viewCtor.collection();
    const entities = await this.inner.fetchEntities(collectionId, args);
    return entities.map((e) => viewCtor.fromEntity(e));
  }

  /**
   * Fetch the first entity matching a selection.
   * Rust: pub async fn fetch_one<R: View>(&self, args) -> Result<Option<R>, RetrievalError>
   */
  async fetchOne<V extends ViewInstance>(
    viewCtor: ViewConstructor<V>,
    args: MatchArgs,
  ): Promise<V | null> {
    const views = await this.fetch(viewCtor, args);
    return views.length > 0 ? views[0] : null;
  }

  /**
   * Create a reactive live query for entities matching a selection.
   * Rust: pub fn query<R: View>(&self, args) -> Result<LiveQuery<R>, RetrievalError>
   */
  query<V extends ViewInstance>(
    viewCtor: ViewConstructor<V>,
    args: MatchArgs,
  ): LiveQuery<V> {
    const entityLiveQuery = this.inner.query(viewCtor.collection(), args);
    return entityLiveQuery.map(viewCtor);
  }

  /**
   * Create a live query and wait for initial data to load.
   * Rust: pub async fn query_wait<R: View>(&self, args) -> Result<LiveQuery<R>, RetrievalError>
   */
  async queryWait<V extends ViewInstance>(
    viewCtor: ViewConstructor<V>,
    args: MatchArgs,
  ): Promise<LiveQuery<V>> {
    const liveQuery = this.query(viewCtor, args);
    await liveQuery.waitInitialized();
    return liveQuery;
  }

  /**
   * Get a storage collection handle.
   * Rust: pub async fn collection(&self, id) -> Result<StorageCollectionWrapper, RetrievalError>
   */
  async collection(id: CollectionId): Promise<StorageCollection> {
    return this.inner.collection(id);
  }
}
```

**ViewConstructor requirements:** The `ViewConstructor<V>` interface must provide:
- `collection(): CollectionId` -- static method returning the collection ID for this model
- `fromEntity(entity: Entity): V` -- factory method creating a view from an entity

Check `/Users/daniel/ak/ankurah-ts/packages/core/src/model.ts` to confirm these exist. The existing `ViewConstructor` type should have these based on the Rust `View` trait which defines `fn collection() -> CollectionId` and `fn from_entity(entity: Entity) -> Self`.

---

### 2.4 MISSING: `fetchEntities` in NodeAndContext should use `MatchArgs` properly

**Current TS state** (node.ts lines 215-219):
```typescript
async fetchEntities(collection: CollectionId, args: unknown): Promise<Entity[]> {
  const matchArgs = args as MatchArgs;
  this.node.policyAgent.canAccessCollection(this.cdata, collection);
  return this.node.fetchEntitiesFromLocal(collection, matchArgs.selection);
}
```

**Rust** (context.rs lines 212-239) does much more:
1. `canAccessCollection` check
2. `filterPredicate` to apply policy filtering to the selection predicate
3. `type_resolver.resolveSelectionTypes` for AST type resolution
4. For non-durable nodes: `fetch_from_peer()` instead of local fetch
5. For durable nodes: local storage fetch via `self.node.collections.get()`

**Required changes:**
```typescript
async fetchEntities(collection: CollectionId, args: MatchArgs): Promise<Entity[]> {
  this.node.policyAgent.canAccessCollection(this.cdata, collection);

  // Policy filtering of selection predicate
  if (this.node.policyAgent.filterPredicate) {
    args = {
      ...args,
      selection: {
        ...args.selection,
        predicate: this.node.policyAgent.filterPredicate(
          this.cdata, collection, args.selection.predicate
        ),
      },
    };
  }

  // TODO: type resolver -- args.selection = node.typeResolver.resolveSelectionTypes(args.selection)

  return this.node.fetchEntitiesFromLocal(collection, args.selection);
}
```

Also update `TContext.fetchEntities` signature to use `MatchArgs` instead of `unknown`:
```typescript
fetchEntities(collection: CollectionId, args: MatchArgs): Promise<Entity[]>;
```

---

## Part 3: Node Fields/Methods Required by LiveQuery

Based on the livequery-port-spec.md Section 17 and the Rust source, here is exactly what `EntityLiveQuery.create()` needs from `Node`:

### 3.1 Required by EntityLiveQuery constructor (create method)

| Requirement | Node field/method | Status | Priority |
|---|---|---|---|
| Policy access check | `node.policyAgent.canAccessCollection()` | EXISTS | -- |
| Policy predicate filter | `node.policyAgent.filterPredicate()` | EXISTS (optional on interface) | Medium |
| Type resolver | `node.typeResolver.resolveSelectionTypes()` | NOT PRESENT | Low (stub as passthrough) |
| Create reactor subscription | `node.reactor.subscribe()` | NEEDS `reactor` field (1.1) | HIGH |
| Determine relay status | `node.subscriptionRelay` / `node.hasSubscriptionRelay()` | NEEDS stub (1.7, 1.6) | HIGH |
| Register remote query | `node.subscribeRemoteQuery()` | NEEDS stub (1.9) | Medium |
| Fetch entities for gap filler | `node.fetchEntitiesFromLocal()` | EXISTS | -- |

### 3.2 Required by EntityLiveQuery.activate()

| Requirement | Node field/method | Status | Priority |
|---|---|---|---|
| Get reactor reference | `node.reactor` | NEEDS `reactor` field (1.1) | HIGH |
| `reactor.addQueryAndNotify()` | Method on Reactor | EXISTS in reactor/index.ts | -- |
| `reactor.updateQueryAndNotify()` | Method on Reactor | EXISTS in reactor/index.ts | -- |
| `fetchEntitiesFromLocal()` | Method on Node | EXISTS | -- |

### 3.3 Required by EntityLiveQuery.updateSelection()

| Requirement | Node field/method | Status | Priority |
|---|---|---|---|
| Check relay status | `node.hasSubscriptionRelay()` | NEEDS implementation (1.6) | HIGH |
| Update remote query | `node.updateRemoteQuery()` | NEEDS stub (1.6) | Medium |

### 3.4 Required by EntityLiveQuery.dispose() / Drop

| Requirement | Node field/method | Status | Priority |
|---|---|---|---|
| Unsubscribe remote predicate | `node.unsubscribeRemotePredicate()` | NEEDS stub (1.6) | Medium |

### 3.5 Required by QueryGapFetcher

The existing `QueryGapFetcher` at `reactor/fetch_gap.ts` uses a `NodeLike` interface:
```typescript
export interface NodeLike {
  fetchEntities(collectionId: CollectionId, selection: Selection): Promise<Entity[]>;
}
```

This does NOT match `Node.fetchEntitiesFromLocal()` (different name, `fetchEntities` vs `fetchEntitiesFromLocal`). The livequery spec (Section 5, step 7) says:
> `new QueryGapFetcher(node, cdata)` -- Check QueryGapFetcher TS constructor. The existing TS class takes `(nodeRef: WeakRef<NodeLike>)`. Adapt as needed.

**Required resolution:** Either:
1. Make `Node` implement `NodeLike` by adding a `fetchEntities(collectionId, selection)` method (which delegates to `fetchEntitiesFromLocal`), OR
2. Change `QueryGapFetcher` to accept the node directly and use `fetchEntitiesFromLocal`.

The Rust `QueryGapFetcher` stores a `Weak<NodeInner>` and calls the `NodeAndContext.fetch_entities()` path. In TS, since `QueryGapFetcher` needs context data for policy filtering, the simplest approach is:

```typescript
// Add to Node (or NodeAndContext):
fetchEntities(collectionId: CollectionId, selection: Selection): Promise<Entity[]> {
  // Delegates to fetchEntitiesFromLocal for Phase 1 (durable-only)
  return this.fetchEntitiesFromLocal(collectionId, selection);
}
```

Or alternatively, wrap in the EntityLiveQuery factory:
```typescript
const nodeForGap: NodeLike = {
  fetchEntities: (cid, sel) => node.fetchEntitiesFromLocal(cid, sel),
};
const gapFetcher = new QueryGapFetcher(nodeForGap);
```

---

## Part 4: fetchEntitiesFromLocal -- Storage Access Path

The current `Node.fetchEntitiesFromLocal()` (node.ts lines 111-124) uses `this.storageEngine.collection(collectionId)`:

```typescript
async fetchEntitiesFromLocal(collectionId: CollectionId, selection: Selection): Promise<Entity[]> {
  const collection = await this.storageEngine.collection(collectionId);
  const states = await collection.fetchStates(selection);
  // ...
}
```

After adding `collections: CollectionSet` (1.2), this must change to:
```typescript
async fetchEntitiesFromLocal(collectionId: CollectionId, selection: Selection): Promise<Entity[]> {
  const collection = this.collections.get(collectionId);
  const states = await collection.fetchStates(selection);
  // ...
}
```

Note: `CollectionSet.get()` may be sync or async depending on the TS port. Check `collectionset.ts` -- if it's synchronous, drop the `await`.

---

## Part 5: Import/Export Changes Required in index.ts

After implementing the changes above, the following exports need to be added to `/Users/daniel/ak/ankurah-ts/packages/core/src/index.ts`:

```typescript
// ── LiveQuery ──
export { EntityLiveQuery, WeakEntityLiveQuery, LiveQuery } from './livequery.ts';
export type { RemoteQuerySubscriber } from './livequery.ts';

// ── Changes (new ChangeSet type) ──
export type { ChangeSet } from './changes.ts';
```

The `TNodeErased` interface (if defined in node.ts) should also be exported:
```typescript
export type { TNodeErased } from './node.ts';
```

---

## Part 6: Complete Change Checklist

### node.ts changes:
- [ ] Add `reactor: Reactor` field, initialize in constructor
- [ ] Add `collections: CollectionSet` field, replace `storageEngine` usage
- [ ] Add `system: SystemManager` field, initialize in constructor
- [ ] Add `subscriptionRelay: null` field (Phase 1 stub)
- [ ] Add system readiness check to `context()` method
- [ ] Add `contextAsync()` method
- [ ] Add `hasSubscriptionRelay()` method (returns false, Phase 1)
- [ ] Add `unsubscribeRemotePredicate()` method stub
- [ ] Add `updateRemoteQuery()` method stub
- [ ] Add `subscribeRemoteQuery()` method stub
- [ ] Update `fetchEntitiesFromLocal()` to use `collections.get()`
- [ ] Define `TNodeErased` interface (or expand `ReactorNodeLike`)
- [ ] `NodeAndContext.query()` -- implement TContext method
- [ ] `NodeAndContext.collection()` -- implement TContext method
- [ ] `NodeAndContext.commitLocalTrx()` -- add Phase 7 reactor notification
- [ ] `NodeAndContext.commitLocalTrx()` -- fix `require()` calls to proper imports
- [ ] `NodeAndContext.fetchEntities()` -- use `MatchArgs` type, add policy filtering
- [ ] Add new imports: Reactor, SystemManager, CollectionSet, QueryId, Selection, EntityChange (value), EntityLiveQuery

### context.ts changes:
- [ ] Add `query()` to `TContext` interface
- [ ] Add `collection()` to `TContext` interface
- [ ] Change `fetchEntities` arg type from `unknown` to `MatchArgs`
- [ ] Add `get()`, `getCached()`, `fetch()`, `fetchOne()` to `Context` class
- [ ] Add `query()`, `queryWait()` to `Context` class
- [ ] Add `collection()` to `Context` class
- [ ] Add imports: ViewConstructor, ViewInstance, LiveQuery, EntityLiveQuery, MatchArgs, StorageCollection, EntityId, CollectionId

### changes.ts changes:
- [ ] Add `ChangeSet<V>` interface

### index.ts changes:
- [ ] Export LiveQuery, EntityLiveQuery, WeakEntityLiveQuery, RemoteQuerySubscriber
- [ ] Export ChangeSet type
- [ ] Export TNodeErased type (if defined)

### reactor/index.ts changes:
- [ ] Update `ReactorNodeLike` to match `TNodeErased` or mark as deprecated in favor of `TNodeErased`

---

## Part 7: Dependency Order for Implementation

The implementation must proceed in this order due to dependencies:

1. **Add `reactor`, `collections`, `system` fields to Node constructor** (1.1, 1.2, 1.3)
   - No dependencies on other new code
   - All three classes (Reactor, CollectionSet, SystemManager) already exist in TS

2. **Add TNodeErased stubs on Node** (1.6, 1.7, 1.8, 1.9)
   - Depends on: reactor field existing
   - All are Phase 1 stubs (no-op or false)

3. **Update `fetchEntitiesFromLocal` storage access path** (Part 4)
   - Depends on: collections field existing

4. **Add Phase 7 reactor notification to `commitLocalTrx`** (1.5)
   - Depends on: reactor field existing, EntityChange import as value

5. **Add `query()` and `collection()` to TContext and NodeAndContext** (2.1, 2.2)
   - Depends on: reactor field, system field
   - `query()` depends on EntityLiveQuery being available (from livequery.ts port)

6. **Add Context public API methods** (2.3)
   - Depends on: TContext having `query()`, `collection()`, `fetchEntities()`
   - Depends on: LiveQuery class being available

7. **Port livequery.ts** (separate task, covered by livequery-port-spec.md)
   - Depends on: All of the above being in place

Steps 1-4 can be done immediately without the livequery port.
Steps 5-6 require the livequery.ts file to exist (at minimum as types).
