# Port Spec: `system.rs` -> `system.ts`

**Source**: `/Users/daniel/ak/ankurah/core/src/system.rs` (317 lines)
**Target**: `/Users/daniel/ak/ankurah-ts/packages/core/src/system.ts`
**Line 1 annotation**: `// MIRRORS: ankurah/core/src/system.rs`

---

## 1. Constants

| Rust Name | Rust Type | TS Name | TS Type | Visibility |
|---|---|---|---|---|
| `SYSTEM_COLLECTION_ID` | `&str` = `"_ankurah_system"` | `SYSTEM_COLLECTION_ID` | `string` = `"_ankurah_system"` | `pub` -> `export` |
| `PROTECTED_COLLECTIONS` | `&[&str]` = `&[SYSTEM_COLLECTION_ID]` | `PROTECTED_COLLECTIONS` | `readonly string[]` = `[SYSTEM_COLLECTION_ID]` | `pub` -> `export` |

---

## 2. Types / Structs

### 2.1 `SystemManager<SE, PA>` (public)

Rust is generic over `SE: StorageEngine + Send + Sync + 'static` and `PA: PolicyAgent + Send + Sync + 'static`. TS drops the generics entirely since the TS codebase uses `StorageEngine` and `PolicyAgent` as interfaces (not generics on Node or other types). See Exception [E8] -- no Arc/Send/Sync.

**Rust definition:**
```rust
pub struct SystemManager<SE, PA>(Arc<Inner<SE, PA>>);
```
With a manual `Clone` impl that clones the `Arc`.

**TS equivalent**: A plain `class SystemManager`. No Arc, no clone. The TS `Node` holds a single instance by reference.

### 2.2 `Inner<SE, PA>` (private)

All fields become plain class properties on `SystemManager` (folded in, per [E8]).

| Rust Field | Rust Type | TS Field | TS Type | Notes |
|---|---|---|---|---|
| `collectionset` | `CollectionSet<SE>` | `collectionset` | `CollectionSet` | Already ported |
| `collection_map` | `RwLock<BTreeMap<CollectionId, Entity>>` | `collectionMap` | `Map<string, Entity>` | No RwLock [E8]. Key is `CollectionId.toString()` |
| `entities` | `WeakEntitySet` | `entities` | `WeakEntitySet` | Already ported |
| `durable` | `bool` | `durable` | `boolean` | |
| `root` | `RwLock<Option<Attested<EntityState>>>` | `root` | `Attested<EntityState> \| null` | No RwLock [E8] |
| `items` | `RwLock<Vec<Entity>>` | `items` | `Entity[]` | No RwLock [E8] |
| `loaded` | `OnceLock<()>` | `loaded` | `boolean` | Simple boolean flag |
| `loading` | `Notify` | `loadingPromise` | `PromiseWithResolvers<void>` | See TS porting notes |
| `system_ready` | `RwLock<bool>` | `systemReady` | `boolean` | No RwLock [E8] |
| `system_ready_notify` | `Notify` | `systemReadyPromise` | `PromiseWithResolvers<void>` | See TS porting notes |
| `reactor` | `Reactor` | `reactor` | `Reactor` | Already ported |
| `_phantom` | `PhantomData<PA>` | (omitted) | N/A | Not needed in TS [E8] |

---

## 3. Methods

### 3.1 Constructor

**Rust**: `pub(crate) fn new(collections: CollectionSet<SE>, entities: WeakEntitySet, reactor: Reactor, durable: bool) -> Self`

- Visibility: `pub(crate)` -> not exported (internal to `@ankurah/core`).
- Creates the `Inner` struct wrapped in `Arc`.
- Spawns an async task via `crate::task::spawn()` to call `self.load_system_catalog()`.
- On error, logs `error!("Failed to load system catalog: {}", e)`.

**TS signature**:
```typescript
constructor(
  collectionset: CollectionSet,
  entities: WeakEntitySet,
  reactor: Reactor,
  durable: boolean,
)
```

**TS porting notes**:
- Replace `crate::task::spawn(async move { ... })` with a fire-and-forget promise:
  ```typescript
  this.loadSystemCatalog().catch(e => console.error('Failed to load system catalog:', e));
  ```
  This matches Rust's `tokio::spawn` behavior: the task runs asynchronously and errors are logged, not propagated. The constructor is synchronous.
- Initialize `loadingPromise = Promise.withResolvers<void>()` and `systemReadyPromise = Promise.withResolvers<void>()` in the constructor.
- If `Promise.withResolvers` is not available in the target environment, use the manual pattern:
  ```typescript
  let resolve: () => void;
  const promise = new Promise<void>(r => { resolve = r; });
  ```

### 3.2 `root()` (public)

**Rust**: `pub fn root(&self) -> Option<Attested<EntityState>>`
Returns a clone of the root state.

**TS signature**: `root(): Attested<EntityState> | null`
Returns `this._root` (the underscore-prefixed private field to avoid collision with the method name). No clone needed -- JS reference semantics.

### 3.3 `items()` (public)

**Rust**: `pub fn items(&self) -> Vec<Entity>`
Returns a clone of the items vector.

**TS signature**: `getItems(): Entity[]`
Returns `[...this._items]` (shallow copy of the array). Name `getItems()` avoids collision with the field. Alternatively, name the field `_items` and the method `items()`.

### 3.4 `collection(id)` (public, async)

**Rust**: `pub async fn collection(&self, id: &CollectionId) -> Result<StorageCollectionWrapper, RetrievalError>`
- Calls `self.wait_loaded().await` first.
- Then returns `self.0.collectionset.get(id).await`.
- Has a TODO comment about updating the system catalog.

**TS signature**: `async collection(id: CollectionId): Promise<StorageCollection>`
Throws `RetrievalError`.

**Implementation**:
```typescript
async collection(id: CollectionId): Promise<StorageCollection> {
  await this.waitLoaded();
  // TODO: update the system catalog to create an entity for this collection
  return this.collectionset.get(id);
}
```

### 3.5 `isSystemReady()` (public)

**Rust**: `pub fn is_system_ready(&self) -> bool`

**TS signature**: `isSystemReady(): boolean`
Returns `this.systemReady`.

### 3.6 `waitSystemReady()` (public, async)

**Rust**: `pub async fn wait_system_ready(&self)`
- If not ready, awaits `self.0.system_ready_notify.notified()`.

**TS signature**: `async waitSystemReady(): Promise<void>`

**TS porting notes for `Notify` pattern**:
Rust `tokio::sync::Notify` is a one-shot or multi-shot notification primitive. In TS, use a `Promise` that is resolved when the system becomes ready. The `systemReadyPromise` is created in the constructor. When the system becomes ready, `systemReadyPromise.resolve()` is called. However, since multiple callers can `await` it, and it might need to be re-created after `hard_reset()`, use a pattern like:

```typescript
async waitSystemReady(): Promise<void> {
  if (this.systemReady) return;
  await this.systemReadyPromise.promise;
}
```

When `hard_reset()` is called, a NEW `systemReadyPromise` must be created so future callers will block again.

### 3.7 `create()` (public, async)

**Rust**: `pub async fn create(&self) -> Result<()>`

**TS signature**: `async create(): Promise<void>`
Throws `Error` (Rust uses `anyhow::Error`; TS uses plain `Error` per rule A8).

**Implementation (step by step)**:
1. Guard: if `!this.durable`, throw `new Error("Only durable nodes can create a new system")`.
2. `await this.waitLoaded()`.
3. Guard: if `this._items.length > 0`, throw `new Error("System root already exists")`.
4. Create `collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID)`.
5. Get storage: `const storage = await this.collectionset.get(collectionId)`.
6. Create entity: `const systemEntity = this.entities.create(collectionId)`.
7. Get LWW backend: `const lwwBackend = systemEntity.getBackend(LWWBackend)`.
8. Set the "item" property: `lwwBackend.set('item', sysItemToValue({ type: 'SysRoot' }))`.
   - **Important**: See section 4 for the `sysItemToValue` / `sysItemFromValue` helper functions (replaces Rust's `Property` impl for `proto::sys::Item`).
9. Generate commit event: `const event = systemEntity.generateCommitEvent()`. If null, throw `new Error("Expected event")`.
10. Derive root clock: `const rootClock = Clock.fromEventId(event.id())`.
11. Store event: `await storage.addEvent(attestedEvent)`.
    - **Note**: Rust calls `storage.add_event(&event.into())`. The `.into()` converts `Event` to `Attested<Event>` (with no attestation). TS needs to wrap with `Attested.none(event)` or equivalent.
12. Commit head: `systemEntity.commitHead(rootClock)`.
13. Get entity state: `const attestedState = Attested.none(systemEntity.toEntityState())`.
    - **Note**: Rust calls `system_entity.to_entity_state()?.into()` where `.into()` is `Into<Attested<EntityState>>` which creates an unattested wrapper.
14. Store state: `await storage.setState(attestedState)`.
15. Update items: `this._items.push(systemEntity)`.
16. Update root: `this._root = attestedState`.
17. Mark ready: `this.systemReady = true; this.systemReadyPromise.resolve()`.

### 3.8 `joinSystem(state)` (public, async)

**Rust**: `pub async fn join_system(&self, state: Attested<EntityState>) -> Result<(), MutationError>`

**TS signature**: `async joinSystem(state: Attested<EntityState>): Promise<void>`
Throws `MutationError`.

**Implementation**:
1. `await this.waitLoaded()`.
2. Guard: if `this.durable`, throw `MutationError.general(new Error("Durable nodes cannot join an existing system"))`. (With a `console.warn` before.)
3. Get current root: `const rootState = this.root()`.
4. If root exists:
   a. If `root.payload.state.head` equals `state.payload.state.head` -- matching root. Log info, set `systemReady = true`, resolve promise, return.
   b. Otherwise: Log warning about mismatch. Log info about resetting storage. Clear `this._root = null`. Call `await this.hardReset()` (wrap errors in MutationError).
5. Create `collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID)`.
6. Get storage: `const storage = await this.collectionset.get(collectionId)`.
7. Store state: `await storage.setState(state)`.
8. Set root and mark ready: `this._root = state; this.systemReady = true; this.systemReadyPromise.resolve()`.

**Clock comparison note**: Rust compares `root.payload.state.head == state.payload.state.head`. The TS `Clock` class needs an `equals()` method (or compare via `toBase64()`).

### 3.9 `hardReset()` (public, async)

**Rust**: `pub async fn hard_reset(&self) -> Result<()>`

**TS signature**: `async hardReset(): Promise<void>`
Throws `Error`.

**Implementation**:
1. `this.collectionset.deleteAllCollections()` (TS version is synchronous, clears cache).
   - **Note**: The Rust version calls `self.0.collectionset.delete_all_collections().await?` which delegates to `StorageEngine::delete_all_collections()`. The TS `CollectionSet.deleteAllCollections()` currently only clears the in-memory cache. If the `StorageEngine` interface gains a `deleteAllCollections()` method, this should call it.
2. Clear items: `this._items = []`.
3. Clear root: `this._root = null`.
4. Clear collection map: `this.collectionMap.clear()`.
5. Set not ready: `this.systemReady = false`.
6. **Re-create** `this.systemReadyPromise = Promise.withResolvers<void>()` so future `waitSystemReady()` calls block again.
7. Reset reactor: `this.reactor.systemReset()`.

### 3.10 `isLoaded()` (public)

**Rust**: `pub fn is_loaded(&self) -> bool`

**TS signature**: `isLoaded(): boolean`
Returns `this.loaded`.

### 3.11 `waitLoaded()` (public, async)

**Rust**: `pub async fn wait_loaded(&self)`

**TS signature**: `async waitLoaded(): Promise<void>`

```typescript
async waitLoaded(): Promise<void> {
  if (this.loaded) return;
  await this.loadingPromise.promise;
}
```

### 3.12 `loadSystemCatalog()` (private, async)

**Rust**: `async fn load_system_catalog(&self) -> Result<()>`

**TS signature**: `private async loadSystemCatalog(): Promise<void>`

**Implementation**:
1. Guard: if `this.loaded`, throw `new Error("System catalog already loaded")`.
2. Create `collectionId = CollectionId.fixedName(SYSTEM_COLLECTION_ID)`.
3. Get storage: `const storage = await this.collectionset.get(collectionId)`.
4. Create LocalRetriever: `const retriever = new LocalRetriever(storage)`.
5. Fetch all states with a "true" predicate:
   ```typescript
   const selection: Selection = { predicate: { type: 'True' }, orderBy: null, limit: null };
   const states = await storage.fetchStates(selection);
   ```
   **Note**: The Rust code creates `ankql::ast::Selection { predicate: Predicate::True, order_by: None, limit: None }`. The TS `@ankurah/ankql` package must have a matching `Selection` type with a `True` predicate variant.
6. For each state:
   a. Call `this.entities.withState(state.payload.entityId, collectionId, state.payload.state)` to get `[changed, entity]`.
      - **Note**: Rust calls `self.0.entities.with_state(&retriever, ...)`. The TS `WeakEntitySet.withState()` does NOT take a retriever (it was simplified). This is fine because the retriever is only used in Rust for lineage comparison during `apply_state`, which the TS `withState` does not do.
   b. Get LWW backend: `const lwwBackend = entity.getBackend(LWWBackend)`.
   c. Get the "item" property: `const value = lwwBackend.get('item')`.
   d. If value exists, parse it: `const item = sysItemFromValue(value)`.
   e. If item is `{ type: 'SysRoot' }`, set `rootState = state`.
   f. Push entity to `entities` array.
7. Update items: `this._items.push(...entities)`.
8. Check if root was found. If so and `this.durable`, mark system ready.
9. Set loaded: `this.loaded = true; this.loadingPromise.resolve()`.

---

## 4. Property Implementation for `sys::Item`

**Rust** (lines 300-316): Implements the `Property` trait for `proto::sys::Item`:
```rust
impl Property for proto::sys::Item {
    fn into_value(&self) -> Result<Option<Value>, PropertyError> {
        Ok(Some(Value::String(serde_json::to_string(self)?)))
    }
    fn from_value(value: Option<Value>) -> Result<Self, PropertyError> {
        if let Some(Value::String(string)) = value {
            let item: proto::sys::Item = serde_json::from_str(&string)?;
            Ok(item)
        } else {
            Err(PropertyError::InvalidValue { ... })
        }
    }
}
```

**Key insight**: The Rust code serializes `sys::Item` to/from JSON strings stored as `Value::String`. It does NOT use the bincode encoding from proto/sys.ts. Instead it uses `serde_json`.

**TS equivalent**: Two standalone functions (since TS has no trait impl):

```typescript
import type { Item } from '@ankurah/proto'; // sys::Item type
import type { Value } from './value/index.ts';

/** Serialize a sys::Item to a Value for storage in an LWW backend. */
export function sysItemToValue(item: Item): Value | null {
  // Mirrors: serde_json::to_string -- JSON serialization
  return { type: 'String', value: JSON.stringify(item) };
}

/** Deserialize a sys::Item from a Value retrieved from an LWW backend. */
export function sysItemFromValue(value: Value | null): Item {
  if (value !== null && value.type === 'String') {
    return JSON.parse(value.value) as Item;
  }
  throw new PropertyError('InvalidValue', '', 'sys::Item');
}
```

**JSON format**: Since Rust uses `serde_json` with the default enum representation (externally tagged), the JSON for `Item::SysRoot` is `"SysRoot"` and for `Item::Collection { name }` is `{"Collection":{"name":"..."}}`.

**Wire compatibility note**: The TS `Item` discriminated union uses `{ type: 'SysRoot' }` shape. The JSON round-trip functions must convert between the serde JSON format and the TS discriminated union format:

```typescript
function sysItemToValue(item: Item): Value | null {
  // Convert TS discriminated union to Rust serde_json format
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

function sysItemFromValue(value: Value | null): Item {
  if (value !== null && value.type === 'String') {
    const parsed = JSON.parse(value.value);
    if (parsed === 'SysRoot') return { type: 'SysRoot' };
    if (parsed === 'Other') return { type: 'Other' };
    if (typeof parsed === 'object' && parsed !== null && 'Collection' in parsed) {
      return { type: 'Collection', name: parsed.Collection.name };
    }
  }
  throw new PropertyError('InvalidValue', '', 'sys::Item');
}
```

This is critical for cross-language interop: a system root created by the Rust server stores `"\"SysRoot\""` as the JSON value, and the TS client must parse it correctly.

---

## 5. Dependencies

### 5.1 Imports from `@ankurah/proto`

| Rust Import | TS Import | Notes |
|---|---|---|
| `ankurah_proto as proto` | Various named imports | |
| `Attested` | `Attested` from `@ankurah/proto` | Class in `/packages/proto/src/auth.ts` |
| `Clock` | `Clock` from `@ankurah/proto` | |
| `CollectionId` | `CollectionId` from `@ankurah/proto` | Has `fixedName()` static method |
| `EntityState` | `EntityState` from `@ankurah/proto` | |
| `proto::sys::Item` | `Item` from `@ankurah/proto` | sys.ts discriminated union |

### 5.2 Imports from `@ankurah/core` (internal, same package)

| Rust Import | TS Import | Source File |
|---|---|---|
| `crate::collectionset::CollectionSet` | `CollectionSet` | `./collectionset.ts` |
| `crate::entity::{Entity, WeakEntitySet}` | `Entity, WeakEntitySet` | `./entity.ts` |
| `crate::error::MutationError` | `MutationError` | `./error.ts` |
| `crate::error::RetrievalError` | `RetrievalError` | `./error.ts` |
| `crate::property::Property` (trait) | Not imported as trait | Replaced by standalone functions |
| `crate::property::PropertyError` | `PropertyError` | `./property/traits.ts` |
| `crate::property::backend::LWWBackend` | `LWWBackend` | `./property/backend/lww.ts` |
| `crate::reactor::Reactor` | `Reactor` | `./reactor/index.ts` |
| `crate::retrieval::LocalRetriever` | `LocalRetriever` | `./retrieval.ts` |
| `crate::storage::StorageCollectionWrapper` | `StorageCollection` | `./storage.ts` |
| `crate::storage::StorageEngine` | (not directly used) | |
| `crate::value::Value` | `Value` | `./value/index.ts` |
| `crate::policy::PolicyAgent` | (not directly used) | PhantomData in Rust |
| `crate::notice_info` | `console.info` / logging | [E13] Macro -> function |

### 5.3 Imports from `@ankurah/ankql`

| Import | Notes |
|---|---|
| `Selection` (type) | For the "fetch all" query in `loadSystemCatalog` |
| `Predicate` (type) | Need `{ type: 'True' }` variant |

### 5.4 Rust-only imports NOT needed in TS

| Rust Import | Why Not Needed |
|---|---|
| `std::sync::{Arc, OnceLock, RwLock}` | [E8] No concurrency primitives |
| `std::marker::PhantomData` | [E8] No phantom types |
| `std::collections::BTreeMap` | Use `Map` |
| `tokio::sync::Notify` | Use `Promise.withResolvers` |
| `tracing::{error, warn}` | Use `console.error`, `console.warn` |

---

## 6. Tests

**There are NO inline tests in `system.rs`** (no `#[cfg(test)]` block, no `#[test]` functions).

System functionality is tested through integration tests in the broader test suite (e.g., tests that create nodes, call `system.create()`, connect peers, etc.). A `system.test.ts` file should be created with at least:

1. **Unit test: construct + load** -- Create a SystemManager with a mock/memory StorageEngine that returns empty states. Verify `isLoaded()` becomes true after the loading promise resolves.
2. **Unit test: create** -- Create a durable SystemManager, call `create()`, verify `root()` returns a non-null value and `isSystemReady()` is true.
3. **Unit test: create fails if not durable** -- Create a non-durable SystemManager, call `create()`, verify it throws "Only durable nodes can create a new system".
4. **Unit test: joinSystem** -- Create a non-durable SystemManager, call `joinSystem()` with a valid state, verify system becomes ready.
5. **Unit test: joinSystem fails if durable** -- Create a durable SystemManager, call `joinSystem()`, verify it throws.
6. **Unit test: hardReset** -- Create a SystemManager, make it ready, call `hardReset()`, verify all state is cleared and `isSystemReady()` returns false.
7. **Unit test: sysItemToValue / sysItemFromValue** -- Round-trip test for each Item variant.

---

## 7. How SystemManager Is Used (Integration Points)

### 7.1 In `Node` (node.rs / node.ts)

- **Field**: `pub system: SystemManager<SE, PA>` on `NodeInner`. TS Node should add a `system: SystemManager` field.
- **Construction**: Called in `Node::new()` (ephemeral, `durable=false`) and `Node::new_durable()` (`durable=true`).
  ```typescript
  this.system = new SystemManager(this.collectionset, this.entities, this.reactor, this.durable);
  ```
  Where `this.collectionset` would need to be a `CollectionSet` instance (currently TS Node uses `storageEngine` directly; a `CollectionSet` should be added).
- **`context()` gating**: `Node::context()` checks `self.system.is_system_ready()` and returns error if not. `Node::context_async()` awaits `self.system.wait_system_ready()`.
- **`system_root()`**: `NodeComms::system_root()` delegates to `self.system.root()`.

### 7.2 In `Context` (context.rs / context.ts)

- **Collection access**: `NodeAndContext::collection()` calls `self.node.system.collection(id).await`.
  Currently the TS `NodeAndContext` accesses `this.node.storageEngine.collection(...)` directly. After porting, it should go through `this.node.system.collection(...)` for the `wait_loaded` gating.

### 7.3 In `Connector` (connector.rs / connector.ts)

- **`NodeComms::system_root()`**: Returns `self.system.root()`. Already declared in TS interface.
- **Peer registration**: When a durable peer registers, its presence includes a `system_root`. The node calls `self.system.join_system(system_root)` to adopt the root.

### 7.4 In `Reactor` (reactor.rs / reactor/index.ts)

- **`Reactor::system_reset()`**: Called by `SystemManager::hard_reset()`. Already ported in TS.

---

## 8. TS Porting Notes

### 8.1 `tokio::sync::Notify` -> `Promise.withResolvers`

The Rust `Notify` is used in two places:
1. `loading` / `loaded`: One-shot notification that the catalog has loaded.
2. `system_ready` / `system_ready_notify`: Notification that the system is ready, but can be reset.

For one-shot (loading): A single `Promise.withResolvers<void>()` works perfectly. Once resolved, all current and future `await` calls return immediately.

For resettable (system_ready): After `hardReset()`, a new promise must be created. Pattern:

```typescript
private systemReadyDeferred = createDeferred<void>();

// Mark ready:
this.systemReadyDeferred.resolve();

// Hard reset:
this.systemReadyDeferred = createDeferred<void>();

// Wait:
async waitSystemReady(): Promise<void> {
  if (this.systemReady) return;
  await this.systemReadyDeferred.promise;
}
```

Where `createDeferred()` is a helper:
```typescript
function createDeferred<T>(): { promise: Promise<T>; resolve: (value: T) => void; reject: (reason: unknown) => void } {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}
```

Or use `Promise.withResolvers()` if available (ES2024+, available in Bun).

### 8.2 `OnceLock<()>` -> boolean + deferred

Rust `OnceLock` guarantees one-time initialization. In TS, use a `boolean` flag `loaded` plus the deferred promise pattern. The `loaded` flag prevents double-loading; the deferred promise allows `await waitLoaded()`.

### 8.3 Fire-and-forget async task

Rust constructor spawns `load_system_catalog()` via `crate::task::spawn()`. TS uses:
```typescript
this.loadSystemCatalog().catch(e => console.error('Failed to load system catalog:', e));
```

### 8.4 `Attested<T>` wrapping

The Rust code uses `.into()` to convert `Event`/`EntityState` into `Attested<Event>`/`Attested<EntityState>` (unattested wrapper). The TS `Attested` class at `/Users/daniel/ak/ankurah-ts/packages/proto/src/auth.ts` should have a static factory for creating unattested wrappers. Check if `Attested.none(payload)` or `Attested.opt(payload, null)` exists.

### 8.5 Logging

Rust uses `tracing::{error, warn}` and a custom `notice_info!` macro. TS should use:
- `console.error(...)` for `error!(...)`
- `console.warn(...)` for `warn!(...)`
- `console.info(...)` for `notice_info!(...)`

### 8.6 `CollectionSet` integration

The TS `Node` currently uses `storageEngine` directly. To match Rust, `Node` should hold a `CollectionSet` instance and pass it to `SystemManager`. This is a prerequisite change to `node.ts`.

Alternatively, since `CollectionSet` is already ported and `Node` already has a `storageEngine` field, `SystemManager` could create its own `CollectionSet` internally -- but this would diverge from Rust where the same `CollectionSet` is shared between `Node` and `SystemManager`.

### 8.7 AnkQL `True` predicate

The `loadSystemCatalog` method needs to fetch ALL states from the system collection. It uses:
```rust
Selection { predicate: Predicate::True, order_by: None, limit: None }
```

Verify that the TS `@ankurah/ankql` package has a `True` predicate variant. If the AST types match the Rust structure, it should be:
```typescript
const selection: Selection = {
  predicate: { type: 'True' },
  orderBy: null,
  limit: null,
};
```

### 8.8 `PropertyError` import

The `sysItemFromValue` function needs `PropertyError`. Check the existing type at `/Users/daniel/ak/ankurah-ts/packages/core/src/property/traits.ts`.

### 8.9 Node.ts changes needed

After `system.ts` is ported, `node.ts` needs these updates:
1. Add `system: SystemManager` field to `Node`.
2. Construct it in the constructor.
3. Add `collectionset: CollectionSet` field (or create inside SystemManager).
4. Gate `context()` with `system.isSystemReady()`.
5. Add `contextAsync()` method that awaits `system.waitSystemReady()`.
6. Implement `systemRoot()` in `NodeComms` (connector.ts already declares it).

### 8.10 Re-export from index.ts

Add to `/Users/daniel/ak/ankurah-ts/packages/core/src/index.ts`:
```typescript
export { SystemManager, SYSTEM_COLLECTION_ID, PROTECTED_COLLECTIONS } from './system.ts';
```

---

## 9. What Already Exists in the TS Codebase

| Component | File | Status |
|---|---|---|
| `sys::Item` type + bincode codec | `/packages/proto/src/sys.ts` | Fully ported. Has `Item` type, `encodeItem()`, `decodeItem()`. |
| `CollectionId.fixedName()` | `/packages/proto/src/collection.ts` | Exists |
| `Attested<T>` class | `/packages/proto/src/auth.ts` | Exists, line 109 |
| `EntityState` class | `/packages/proto/src/data.ts` (or similar) | Exists |
| `Clock` class | `/packages/proto/src/clock.ts` | Exists |
| `CollectionSet` | `/packages/core/src/collectionset.ts` | Fully ported |
| `Entity` + `WeakEntitySet` | `/packages/core/src/entity.ts` | Fully ported |
| `LWWBackend` (with `get`/`set`) | `/packages/core/src/property/backend/lww.ts` | Fully ported |
| `LocalRetriever` | `/packages/core/src/retrieval.ts` | Fully ported |
| `Reactor.systemReset()` | `/packages/core/src/reactor/index.ts` line 490 | Fully ported |
| `StorageCollection` interface | `/packages/core/src/storage.ts` | Fully ported |
| `StorageEngine` interface | `/packages/core/src/storage.ts` | Fully ported |
| `MutationError` | `/packages/core/src/error.ts` | Fully ported |
| `RetrievalError` | `/packages/core/src/error.ts` | Fully ported |
| `PropertyError` | `/packages/core/src/property/traits.ts` | Exists |
| `Property` interface | `/packages/core/src/property/index.ts` | Exists |
| `Value` type | `/packages/core/src/value/index.ts` | Exists |
| `Node` class | `/packages/core/src/node.ts` | Ported but needs SystemManager integration |
| `NodeComms.systemRoot()` | `/packages/core/src/connector.ts` line 68 | Interface declared, not yet implemented |

---

## 10. Proposed File Structure

```
packages/core/src/system.ts           # Main SystemManager class
packages/core/src/system.test.ts      # Tests for SystemManager
```

---

## 11. Complete Class Skeleton

```typescript
// MIRRORS: ankurah/core/src/system.rs

import {
  type CollectionId,
  CollectionId as CollectionIdClass,
  type Attested,
  type EntityState,
  Clock,
} from '@ankurah/proto';
import type { Item } from '@ankurah/proto';  // sys::Item
import type { Selection } from '@ankurah/ankql';

import { CollectionSet } from './collectionset.ts';
import { Entity, WeakEntitySet } from './entity.ts';
import { MutationError, RetrievalError } from './error.ts';
import { PropertyError } from './property/traits.ts';
import { LWWBackend } from './property/backend/lww.ts';
import { Reactor } from './reactor/index.ts';
import { LocalRetriever } from './retrieval.ts';
import type { StorageCollection } from './storage.ts';
import type { Value } from './value/index.ts';

export const SYSTEM_COLLECTION_ID = '_ankurah_system';
export const PROTECTED_COLLECTIONS: readonly string[] = [SYSTEM_COLLECTION_ID];

export class SystemManager {
  private readonly collectionset: CollectionSet;
  private readonly entities: WeakEntitySet;
  private readonly durable: boolean;
  private readonly reactor: Reactor;
  private readonly collectionMap: Map<string, Entity> = new Map();

  private _root: Attested<EntityState> | null = null;
  private _items: Entity[] = [];
  private loaded = false;
  private loadingDeferred = createDeferred<void>();
  private systemReady = false;
  private systemReadyDeferred = createDeferred<void>();

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

    // Fire-and-forget load, matching Rust's task::spawn
    this.loadSystemCatalog().catch(e =>
      console.error('Failed to load system catalog:', e)
    );
  }

  root(): Attested<EntityState> | null { return this._root; }
  getItems(): Entity[] { return [...this._items]; }
  isLoaded(): boolean { return this.loaded; }
  isSystemReady(): boolean { return this.systemReady; }

  async waitLoaded(): Promise<void> { /* ... */ }
  async waitSystemReady(): Promise<void> { /* ... */ }
  async collection(id: CollectionId): Promise<StorageCollection> { /* ... */ }
  async create(): Promise<void> { /* ... */ }
  async joinSystem(state: Attested<EntityState>): Promise<void> { /* ... */ }
  async hardReset(): Promise<void> { /* ... */ }
  private async loadSystemCatalog(): Promise<void> { /* ... */ }
}

// ── sys::Item <-> Value conversion ──
// Replaces Rust's `impl Property for proto::sys::Item`

export function sysItemToValue(item: Item): Value | null { /* ... */ }
export function sysItemFromValue(value: Value | null): Item { /* ... */ }

// ── Deferred helper ──

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}
```
