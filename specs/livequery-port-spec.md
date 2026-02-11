# livequery.ts -- Implementation Spec

**Source:** `/Users/daniel/ak/ankurah/core/src/livequery.rs`
**Target:** `/Users/daniel/ak/ankurah-ts/packages/core/src/livequery.ts`
**MIRRORS:** `ankurah/core/src/livequery.rs`

---

## 1. Import Mapping

| Rust import | TS import | Source module |
|---|---|---|
| `std::marker::PhantomData` | *eliminated* -- TS generics don't need phantom types | -- |
| `std::sync::{Arc, Weak}` | *eliminated* / `WeakRef<T>` | built-in |
| `ankurah_proto::{self as proto, CollectionId}` | `CollectionId`, `QueryId`, `EntityId`, `Attested`, `Event` | `@ankurah/proto` |
| `ankurah_proto::QueryId::new()` | `QueryId.new()` | `@ankurah/proto` |
| `ankurah_signals::{broadcast::BroadcastId, porcelain::subscribe::*, signal::*, Get, Mut, Peek, Read, Signal, Subscribe}` | `Mut`, `Read` (class), `type Signal`, `type Listener`, `ListenerGuard`, `type BroadcastId`, `SubscriptionGuard` | `@ankurah/signals` |
| `tracing::{debug, warn}` | `console.debug` / `console.warn` (or structured logger) | built-in |
| `crate::changes::ChangeSet` | *new* -- `ChangeSet<V>` (see Section 8) | `./changes.ts` |
| `crate::entity::Entity` | `Entity` | `./entity.ts` |
| `crate::error::RetrievalError` | `RetrievalError` | `./error.ts` |
| `crate::model::View` | `ViewInstance`, `ViewConstructor<V>` | `./model.ts` |
| `crate::node::{MatchArgs, TNodeErased}` | `MatchArgs`, `Node` | `./node.ts` |
| `crate::policy::PolicyAgent` | `PolicyAgent` | `./policy.ts` |
| `crate::reactor::{fetch_gap::*, ReactorSubscription, ReactorUpdate}` | `ReactorSubscription`, `ReactorUpdate`, `GapFetcher`, `QueryGapFetcher` | `./reactor/index.ts` |
| `crate::resultset::{EntityResultSet, ResultSet}` | `EntityResultSet` | `./resultset.ts` |
| `crate::storage::StorageEngine` | `StorageEngine` | `./storage.ts` |
| `crate::reactor::MembershipChange` | `MembershipChange` | `./reactor/update.ts` |
| `tokio::sync::Notify` | Promise-based resolution pattern (see Section 6) | built-in |
| `std::sync::atomic::AtomicU32` | plain `number` (single-threaded JS) | -- |

---

## 2. ChangeSet<V> -- New Type Needed in changes.ts

Rust defines `ChangeSet` as an associated concept in livequery.rs via the `livequery_change_set_from` function. TS `changes.ts` does not yet have a `ChangeSet<V>` type. It must be added.

```typescript
// Add to /Users/daniel/ak/ankurah-ts/packages/core/src/changes.ts

/**
 * Batch of changes for a LiveQuery subscription.
 *
 * Rust: used inline in livequery.rs Subscribe impl
 * V is a ViewInstance type (e.g., the view class created by defineModel)
 */
export interface ChangeSet<V> {
  readonly changes: ReadonlyArray<ItemChange<V>>;
  readonly resultset: EntityResultSet;
}
```

Import `EntityResultSet` from `./resultset.ts`.

---

## 3. ResultSet<V> -- New Generic Wrapper Needed in resultset.ts

Rust has `pub struct ResultSet<R: View>(EntityResultSet<Entity>, PhantomData<R>)` with `wrap()`, `Peek<Vec<R>>`, `Get<Vec<R>>`. The TS `resultset.ts` only has `EntityResultSet`. A `ResultSet<V>` wrapper is needed.

```typescript
// Add to /Users/daniel/ak/ankurah-ts/packages/core/src/resultset.ts

import type { ViewConstructor, ViewInstance } from './model.ts';

/**
 * A typed view wrapper over EntityResultSet.
 *
 * Rust: `pub struct ResultSet<R: View>(EntityResultSet, PhantomData<R>)`
 * Divergence: TS uses ViewConstructor<V> instead of PhantomData [E8].
 */
export class ResultSet<V extends ViewInstance> {
  private readonly inner: EntityResultSet;
  private readonly viewCtor: ViewConstructor<V>;

  constructor(inner: EntityResultSet, viewCtor: ViewConstructor<V>) {
    this.inner = inner;
    this.viewCtor = viewCtor;
  }

  /** Rust: Peek<Vec<R>> -- get current items without tracking */
  peek(): V[] {
    return this.inner.keys().map((_id, i) => {
      // Read from order array via the inner EntityResultSet
      // Delegate to inner.read().iterEntities()
    });
    // Implementation: iterate inner state, call viewCtor.fromEntity for each
  }

  // ... Signal delegation, etc.
}
```

**Alternatively**, if `ResultSet<V>` is not yet needed by other code, `LiveQuery<V>` can inline the mapping (call `viewCtor.fromEntity()` directly). The spec below assumes `ResultSet<V>` is added.

---

## 4. EntityLiveQuery -- Inner State

### 4.1 Rust Struct: `Inner`

| Rust field | Rust type | TS field | TS type | Notes |
|---|---|---|---|---|
| `query_id` | `proto::QueryId` | `queryId` | `QueryId` | Immutable, assigned in constructor |
| `node` | `Box<dyn TNodeErased>` | `node` | `Node` | Concrete `Node` instance. Rust erases via trait object; TS uses the concrete Node class. |
| `subscription` | `ReactorSubscription` | `subscription` | `ReactorSubscription` | Already ported |
| `resultset` | `EntityResultSet` | `resultset` | `EntityResultSet` | Already ported |
| `error` | `Mut<Option<RetrievalError>>` | `error` | `Mut<RetrievalError \| null>` | Reactive signal |
| `initialized` | `tokio::sync::Notify` | `_initializedResolvers` | See Section 6 | Promise-based wait pattern |
| `initialized_version` | `AtomicU32` | `initializedVersion` | `number` | 0 = uninitialized |
| `current_version` | `AtomicU32` | `currentVersion` | `number` | Starts at 1 |
| `selection` | `Mut<(Selection, u32)>` | `selection` | `Mut<{ selection: Selection; version: number }>` | Reactive signal. Tuple becomes object. |
| `collection_id` | `CollectionId` | `collectionId` | `CollectionId` | Immutable |
| `gap_fetcher` | `Arc<dyn GapFetcher<Entity>>` | `gapFetcher` | `GapFetcher` | Interface, already ported |

### 4.2 TS Class: `EntityLiveQuery`

Rust wraps `Inner` in `Arc<Inner>`. TS does not need `Arc` (GC handles shared ownership). The class directly holds the fields.

```typescript
/**
 * A type-erased live query that manages reactor subscription and remote cleanup.
 *
 * Rust: `pub struct EntityLiveQuery(Arc<Inner>)`
 * Divergence: No Arc -- JS GC handles shared references [E8].
 * Divergence: No Drop on Inner -- dispose() pattern with FinalizationRegistry [E11].
 */
export class EntityLiveQuery {
  // -- Fields (mirrors Inner) --
  readonly queryId: QueryId;
  private readonly node: Node;
  readonly subscription: ReactorSubscription;         // pub(crate) in Rust
  readonly resultset: EntityResultSet;                // pub(crate) in Rust
  private readonly _error: Mut<RetrievalError | null>;
  private readonly _selection: Mut<{ selection: Selection; version: number }>;
  readonly collectionId: CollectionId;
  private readonly gapFetcher: GapFetcher;

  // -- Initialization tracking --
  private initializedVersion: number;    // 0 = uninitialized
  private currentVersion: number;        // starts at 1
  private _initResolve: (() => void) | null;
  private _initPromise: Promise<void>;

  // -- Weak reference support --
  // (see WeakEntityLiveQuery below)

  private constructor(...) { ... }

  static create(
    node: Node,
    collectionId: CollectionId,
    args: MatchArgs,
    cdata: unknown,
  ): EntityLiveQuery { ... }
}
```

---

## 5. EntityLiveQuery -- Constructor / `new()`

### Rust signature
```rust
pub fn new<SE, PA>(
    node: &Node<SE, PA>,
    collection_id: CollectionId,
    mut args: MatchArgs,
    cdata: PA::ContextData,
) -> Result<Self, RetrievalError>
```

### TS adaptation

**Static factory method** `EntityLiveQuery.create(...)` returns `EntityLiveQuery` (throws `RetrievalError` on policy failure).

```typescript
static create(
  node: Node,
  collectionId: CollectionId,
  args: MatchArgs,
  cdata: unknown,
): EntityLiveQuery
```

### Step-by-step logic

1. **Policy check:** `node.policyAgent.canAccessCollection(cdata, collectionId)` -- throws `RetrievalError` on failure.
2. **Filter predicate:** `args.selection.predicate = node.policyAgent.filterPredicate(cdata, collectionId, args.selection.predicate)` -- throws on failure.
3. **Resolve types:** `args.selection = node.typeResolver.resolveSelectionTypes(args.selection)`.
   - **Divergence:** `typeResolver` does not exist on TS `Node` yet. **Stub this call** -- pass selection through unchanged. Add a `// TODO: type resolver` comment.
4. **Create subscription:** `const subscription = node.reactor.subscribe()`.
   - **Divergence:** TS `Node` does not yet have a `reactor` field. This requires `Node` to expose a `Reactor` instance. Add `readonly reactor: Reactor` to `Node`.
5. **Create resultset:** `EntityResultSet.empty()`.
6. **Create queryId:** `QueryId.new()`.
7. **Create gapFetcher:** `new QueryGapFetcher(node, cdata)`.
   - **Divergence:** Check `QueryGapFetcher` TS constructor. The existing TS class takes `(nodeRef: WeakRef<NodeLike>)`. Adapt as needed.
8. **Construct `me`** with all fields, `initializedVersion = 0`, `currentVersion = 1`, selection `= { selection: args.selection, version: 1 }`.
9. **Determine relay status:**
   - Rust: `let has_relay = node.subscription_relay.is_some()`.
   - TS: `const hasRelay = node.subscriptionRelay !== null`.
   - **Divergence:** `subscriptionRelay` does not exist on TS `Node` yet. For Phase 1, **assume `hasRelay = false`** (durable-only path). Add a `// TODO: subscription relay` comment.
10. **Durable-node initialization** (`args.cached || !hasRelay`):
    - Rust spawns `tokio::spawn(async { me2.activate(1).await })`.
    - TS: **Fire-and-forget microtask:** `me.activate(1).catch(e => me._error.set(e))`.
    - Uses `void me.activate(1).then(undefined, (e) => { me._error.set(e instanceof RetrievalError ? e : RetrievalError.other(String(e))); })`.
    - No `tokio::spawn` equivalent needed -- JS `async` functions are already scheduled on the microtask queue. Just call the async method and don't await it.
11. **Ephemeral-node path** (`hasRelay`):
    - Rust: `node.subscribe_remote_query(...)`.
    - TS: **Stub for Phase 1.** Add `// TODO: subscribe_remote_query` comment.
12. Return `me`.

---

## 6. wait_initialized() -- Async Initialization Pattern

### Rust pattern
```rust
pub async fn wait_initialized(&self) {
    if self.0.initialized_version.load(Relaxed) >= self.0.current_version.load(Relaxed) {
        return;
    }
    self.0.initialized.notified().await;
}
```

Uses `tokio::sync::Notify` which supports `notified().await` / `notify_waiters()`.

### TS equivalent

Use a **resolvable Promise** pattern:

```typescript
private _initResolve: (() => void) | null = null;
private _initPromise: Promise<void>;

// In constructor:
this._initPromise = new Promise<void>((resolve) => {
  this._initResolve = resolve;
});
```

**wait_initialized():**
```typescript
async waitInitialized(): Promise<void> {
  if (this.initializedVersion >= this.currentVersion) {
    return;
  }
  // Wait for the current promise
  await this._initPromise;
}
```

**mark_initialized(version):**
```typescript
markInitialized(version: number): void {
  this.initializedVersion = version;
  // Resolve the current promise
  if (this._initResolve) {
    this._initResolve();
  }
  // Create a new promise for the next wait cycle
  this._initPromise = new Promise<void>((resolve) => {
    this._initResolve = resolve;
  });
}
```

**Key adaptation:** Rust `Notify` can have multiple concurrent waiters and `notify_waiters()` wakes all of them. The TS pattern above works because:
- JS is single-threaded, so at most one `waitInitialized()` is pending at a time.
- If `update_selection()` is called, it bumps `currentVersion`, so `waitInitialized()` will await again.
- `markInitialized()` resolves the old promise and creates a fresh one.

---

## 7. activate() Method

### Rust signature
```rust
async fn activate(&self, version: u32) -> Result<(), RetrievalError>
```

### TS signature
```typescript
private async activate(version: number): Promise<void>
```

### Logic

1. **Read current selection:** `const { selection, version: storedVersion } = this._selection.peek()`.
2. **Reject stale activation:** If `version < storedVersion`, log a warning and return (no-op).
3. **Get reactor:** `const reactor = this.node.reactor`.
   - **Divergence:** Rust uses `self.0.node.reactor()` (trait method). TS accesses `this.node.reactor` directly.
4. **Read initializedVersion:** `const initVer = this.initializedVersion`.
5. **Branch on first activation vs subsequent:**

**If `initVer === 0` (first activation):**
```typescript
await reactor.addQueryAndNotify(
  this.subscription.id(),
  this.queryId,
  this.collectionId,
  selection,
  this.node,                  // ReactorNodeLike
  this.resultset,
  this.gapFetcher,
  (version: number) => this.markInitialized(version),  // PreNotifyHook
);
```

**If `initVer > 0` (subsequent activation):**
```typescript
await reactor.updateQueryAndNotify(
  this.subscription.id(),
  this.queryId,
  this.collectionId,
  selection,
  this.node,                  // ReactorNodeLike
  version,
  (version: number) => this.markInitialized(version),  // PreNotifyHook
);
```

**PreNotifyHook adaptation:** Rust implements `PreNotifyHook for &EntityLiveQuery`. TS passes a closure `(v) => this.markInitialized(v)`. The existing TS `PreNotifyHook` type is `((version: number) => void) | null`, which fits perfectly.

---

## 8. update_selection() and update_selection_wait()

### update_selection

**Rust signature:**
```rust
pub fn update_selection(
    &self,
    new_selection: impl TryInto<ankql::ast::Selection, Error = impl Into<RetrievalError>>,
) -> Result<(), RetrievalError>
```

**TS signature:**
```typescript
updateSelection(newSelection: Selection | string): void
```

- If `newSelection` is a `string`, parse it via `parseSelection()` from `@ankurah/ankql`. Throw `RetrievalError` on parse failure.
- Increment `currentVersion`.
- `this.resultset.setLoaded(false)`.
- `this._selection.set({ selection: parsed, version: this.currentVersion })`.
- **Durable path** (no relay): fire-and-forget `this.activate(newVersion)`.
- **Ephemeral path** (has relay): call `this.node.updateRemoteQuery(...)` -- **stub for Phase 1**.

### update_selection_wait

**Rust signature:**
```rust
pub async fn update_selection_wait(...) -> Result<(), RetrievalError>
```

**TS signature:**
```typescript
async updateSelectionWait(newSelection: Selection | string): Promise<void>
```

- Calls `this.updateSelection(newSelection)`.
- Then `await this.waitInitialized()`.

---

## 9. Accessor Methods

| Rust method | TS method | Return type | Notes |
|---|---|---|---|
| `pub fn error(&self) -> Read<Option<RetrievalError>>` | `error(): Read<RetrievalError \| null>` | `Read<RetrievalError \| null>` | `this._error.read()` |
| `pub fn query_id(&self) -> proto::QueryId` | `queryId` (field) | `QueryId` | Direct field access |
| `pub fn selection(&self) -> Read<(Selection, u32)>` | `selection(): Read<{ selection: Selection; version: number }>` | `Read<...>` | `this._selection.read()` |
| `pub fn weak(&self) -> WeakEntityLiveQuery` | `weak(): WeakEntityLiveQuery` | `WeakEntityLiveQuery` | See Section 10 |
| `pub fn mark_initialized(&self, version: u32)` | `markInitialized(version: number): void` | `void` | See Section 6 |

---

## 10. WeakEntityLiveQuery

### Rust
```rust
pub struct WeakEntityLiveQuery(Weak<Inner>);
impl WeakEntityLiveQuery {
    pub fn upgrade(&self) -> Option<EntityLiveQuery> { ... }
}
impl Clone for WeakEntityLiveQuery { ... }
```

### TS adaptation

JS has `WeakRef<T>` which provides `deref() -> T | undefined`. However, `WeakRef` requires the referent to be a GC-collectable object. Since `EntityLiveQuery` is a class instance, this works.

```typescript
/**
 * Weak reference to EntityLiveQuery for breaking circular dependencies.
 *
 * Rust: `pub struct WeakEntityLiveQuery(Weak<Inner>)`
 * Divergence: Uses JS WeakRef instead of Rust Weak<Arc<Inner>> [E8].
 */
export class WeakEntityLiveQuery {
  private readonly ref: WeakRef<EntityLiveQuery>;

  constructor(liveQuery: EntityLiveQuery) {
    this.ref = new WeakRef(liveQuery);
  }

  upgrade(): EntityLiveQuery | null {
    return this.ref.deref() ?? null;
  }
}
```

**Usage in EntityLiveQuery:**
```typescript
weak(): WeakEntityLiveQuery {
  return new WeakEntityLiveQuery(this);
}
```

---

## 11. Drop / Cleanup

### Rust
```rust
impl Drop for Inner {
    fn drop(&mut self) { self.node.unsubscribe_remote_predicate(self.query_id); }
}
```

### TS adaptation

Use `dispose()` pattern with optional `FinalizationRegistry`:

```typescript
dispose(): void {
  // Unsubscribe from remote predicate
  // TODO: this.node.unsubscribeRemotePredicate(this.queryId);

  // Clean up reactor subscription
  this.subscription.dispose();
}

[Symbol.dispose](): void {
  this.dispose();
}
```

Optionally register with `FinalizationRegistry` for safety:
```typescript
// Module-level
const liveQueryRegistry = new FinalizationRegistry<{ node: Node; queryId: QueryId }>(
  ({ node, queryId }) => {
    // TODO: node.unsubscribeRemotePredicate(queryId);
  }
);

// In constructor:
liveQueryRegistry.register(this, { node: this.node, queryId: this.queryId });
```

---

## 12. LiveQuery<V> -- Generic Typed Wrapper

### Rust
```rust
pub struct LiveQuery<R: View>(EntityLiveQuery, PhantomData<R>);
impl<R: View> std::ops::Deref for LiveQuery<R> { ... }
```

### TS

```typescript
/**
 * A typed live query that wraps EntityLiveQuery with a specific View type.
 *
 * Rust: `pub struct LiveQuery<R: View>(EntityLiveQuery, PhantomData<R>)`
 * Divergence: Uses ViewConstructor<V> instead of PhantomData [E8].
 * Divergence: No Deref -- delegates explicitly [E8].
 */
export class LiveQuery<V extends ViewInstance> {
  readonly inner: EntityLiveQuery;
  private readonly viewCtor: ViewConstructor<V>;

  constructor(inner: EntityLiveQuery, viewCtor: ViewConstructor<V>) {
    this.inner = inner;
    this.viewCtor = viewCtor;
  }
}
```

### Fields
| Rust | TS | Type |
|---|---|---|
| `.0` (EntityLiveQuery) | `inner` | `EntityLiveQuery` |
| `PhantomData<R>` | `viewCtor` | `ViewConstructor<V>` |

### Methods

| Rust method | TS method | Signature | Description |
|---|---|---|---|
| `wait_initialized()` | `waitInitialized()` | `(): Promise<void>` | Delegates to `this.inner.waitInitialized()` |
| `resultset()` | `resultset()` | `(): ResultSet<V>` | Returns `new ResultSet(this.inner.resultset, this.viewCtor)`. **Requires ResultSet<V>** from Section 3. Alternatively, inline: `this.inner.resultset.keys().map(...)`. |
| `loaded()` | `loaded()` | `(): boolean` | `this.inner.resultset.isLoaded()` |
| `ids()` | `ids()` | `(): EntityId[]` | `this.inner.resultset.keys()` |
| `ids_sorted()` | `idsSorted()` | `(): EntityId[]` | `this.inner.resultset.keys().sort(compareEntityIds)` |

### EntityLiveQuery.map<V>()

```typescript
// On EntityLiveQuery:
map<V extends ViewInstance>(viewCtor: ViewConstructor<V>): LiveQuery<V> {
  return new LiveQuery(this, viewCtor);
}
```

---

## 13. Signal/Get/Peek/Subscribe Trait Implementations

### Signal for LiveQuery<V>

Rust delegates to `subscription` broadcast (not resultset). This ensures tracking fires on ALL entity changes.

```typescript
// LiveQuery<V> implements Signal
listen(listener: Listener): ListenerGuard {
  return this.inner.subscription.listen(listener);
}

broadcastId(): BroadcastId {
  return this.inner.subscription.broadcastId();
}
```

### Get<V[]> for LiveQuery<V>

Rust: tracks via `CurrentObserver::track(&self)`, then peeks resultset.

```typescript
get(): V[] {
  // TODO: CurrentObserver.track(this) when observer system is ported
  return this.peek();
}
```

### Peek<V[]> for LiveQuery<V>

```typescript
peek(): V[] {
  const read = this.inner.resultset.read();
  return read.iterEntities().map(([_id, entity]) =>
    this.viewCtor.fromEntity(entity)
  );
}
```

### Subscribe<ChangeSet<V>> for LiveQuery<V>

Rust subscribes to `ReactorUpdate` on the subscription, then converts to `ChangeSet<V>`.

```typescript
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
```

---

## 14. livequery_change_set_from() -- Free Function

### Rust signature
```rust
fn livequery_change_set_from<R: View>(
    resultset: ResultSet<R>,
    reactor_update: ReactorUpdate,
) -> ChangeSet<R>
```

### TS signature
```typescript
function liveQueryChangeSetFrom<V extends ViewInstance>(
  resultset: EntityResultSet,
  viewCtor: ViewConstructor<V>,
  reactorUpdate: ReactorUpdate,
): ChangeSet<V>
```

### Logic

```typescript
function liveQueryChangeSetFrom<V extends ViewInstance>(
  resultset: EntityResultSet,
  viewCtor: ViewConstructor<V>,
  reactorUpdate: ReactorUpdate,
): ChangeSet<V> {
  const changes: ItemChange<V>[] = [];

  for (const item of reactorUpdate.items) {
    const view = viewCtor.fromEntity(item.entity);

    // Single-predicate subscription: take first predicate_relevance entry
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
```

---

## 15. RemoteQuerySubscriber -- Trait Stub

### Rust trait
```rust
#[async_trait]
pub trait RemoteQuerySubscriber: Clone + Send + Sync + 'static {
    async fn subscription_established(&self, version: u32);
    fn set_last_error(&self, error: RetrievalError);
}
```

### TS interface

```typescript
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
```

### WeakEntityLiveQuery implements RemoteQuerySubscriber

```typescript
// On WeakEntityLiveQuery:
async subscriptionEstablished(version: number): Promise<void> {
  const liveQuery = this.upgrade();
  if (liveQuery) {
    try {
      await liveQuery['activate'](version);  // private method access
    } catch (e) {
      liveQuery['_error'].set(
        e instanceof RetrievalError ? e : RetrievalError.other(String(e))
      );
    }
  }
}

setLastError(error: RetrievalError): void {
  const liveQuery = this.upgrade();
  if (liveQuery) {
    liveQuery['_error'].set(error);
  }
}
```

**Divergence:** Accessing private fields from `WeakEntityLiveQuery` requires either:
- Making `activate` and `_error` package-internal (use `/** @internal */` JSDoc), or
- Adding internal helper methods on `EntityLiveQuery`:
  ```typescript
  /** @internal */
  _activateInternal(version: number): Promise<void> { return this.activate(version); }
  /** @internal */
  _setError(error: RetrievalError): void { this._error.set(error); }
  ```

---

## 16. PreNotifyHook Integration

Rust has:
```rust
impl crate::reactor::PreNotifyHook for &EntityLiveQuery {
    fn pre_notify(&self, version: u32) {
        self.mark_initialized(version);
    }
}
```

TS already defines `PreNotifyHook` as `((version: number) => void) | null` in `reactor/index.ts`. `EntityLiveQuery` passes a closure:

```typescript
const preNotifyHook: PreNotifyHook = (version: number) => this.markInitialized(version);
```

No trait impl needed.

---

## 17. Node Dependencies (Required Additions to Node)

For `EntityLiveQuery` to work, `Node` needs the following additions:

| Field/Method | Type | Status | Notes |
|---|---|---|---|
| `reactor` | `Reactor` | **NOT YET on Node** | Must be added |
| `subscriptionRelay` | `SubscriptionRelay \| null` | **NOT YET on Node** | Stub as `null` for Phase 1 |
| `typeResolver` | `TypeResolver` | **NOT YET on Node** | Stub (pass-through) for Phase 1 |
| `policyAgent.canAccessCollection()` | Method | **EXISTS** on `PolicyAgent` | Already ported |
| `policyAgent.filterPredicate()` | Method | **CHECK** | May need to be added |
| `fetchEntitiesFromLocal()` | Method | **EXISTS** on `Node` | Already ported, satisfies `ReactorNodeLike` |

---

## 18. Complete Type Summary

### Exports from livequery.ts

```typescript
export class EntityLiveQuery { ... }
export class WeakEntityLiveQuery { ... }
export class LiveQuery<V extends ViewInstance> { ... }
export interface RemoteQuerySubscriber { ... }
```

### Exports to add to changes.ts

```typescript
export interface ChangeSet<V> { ... }
```

### Exports to add to resultset.ts (optional, if ResultSet<V> is desired)

```typescript
export class ResultSet<V extends ViewInstance> { ... }
```

---

## 19. Async Pattern Summary

| Rust pattern | TS equivalent | Location |
|---|---|---|
| `tokio::spawn(async { me.activate(1).await })` | `void me.activate(1).catch(...)` (fire-and-forget Promise) | `EntityLiveQuery.create()` |
| `tokio::sync::Notify` / `notified().await` / `notify_waiters()` | Resolvable `Promise<void>` with stored resolver | `waitInitialized()` / `markInitialized()` |
| `std::sync::atomic::AtomicU32` | Plain `number` (single-threaded JS) | `initializedVersion`, `currentVersion` |
| `Arc<Inner>` / `Weak<Inner>` | Direct reference / `WeakRef<EntityLiveQuery>` | `EntityLiveQuery` / `WeakEntityLiveQuery` |
| `Mut<T>` / `Read<T>` | `Mut<T>` / `Read<T>` from `@ankurah/signals` | `_error`, `_selection` |
| `impl Drop for Inner` | `dispose()` + `[Symbol.dispose]()` + optional `FinalizationRegistry` | Cleanup |

---

## 20. Phase 1 Stubs

The following are required by the Rust source but can be stubbed in Phase 1:

1. **`node.subscriptionRelay`** -- always `null` (durable-only path).
2. **`node.subscribe_remote_query()`** -- no-op.
3. **`node.update_remote_query()`** -- no-op.
4. **`node.unsubscribe_remote_predicate()`** -- no-op in dispose().
5. **Type resolver** (`node.typeResolver.resolveSelectionTypes()`) -- pass-through.
6. **`CurrentObserver.track()`** in `Get` impl -- no-op comment.

---

## 21. File Dependencies Graph

```
livequery.ts
  imports from:
    @ankurah/proto          -- QueryId, CollectionId, EntityId, Attested, Event
    @ankurah/ankql          -- Selection, parseSelection
    @ankurah/signals        -- Mut, Read, Signal, Listener, ListenerGuard,
                               BroadcastId, SubscriptionGuard
    ./entity.ts             -- Entity
    ./error.ts              -- RetrievalError
    ./model.ts              -- ViewInstance, ViewConstructor
    ./node.ts               -- Node, MatchArgs
    ./changes.ts            -- ItemChange, ChangeSet (new)
    ./resultset.ts          -- EntityResultSet, ResultSet (new)
    ./reactor/index.ts      -- Reactor, ReactorSubscription, ReactorUpdate,
                               MembershipChange, PreNotifyHook, GapFetcher,
                               QueryGapFetcher, ReactorNodeLike
```
