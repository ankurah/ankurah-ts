# Ankurah Rust Architecture Findings

Deep analysis of the ankurah Rust codebase for the TypeScript port. All code references verified against actual source files.

---

## 1. Entity/Model/View/Mutable Lifecycle

### EntityInner Structure

**File: `/Users/daniel/ak/ankurah/core/src/entity.rs`**

An `Entity` is a newtype around `Arc<EntityInner>`. Equality is pointer equality (`Arc::ptr_eq`).

```rust
pub struct Entity(Arc<EntityInner>);

pub struct EntityInner {
    pub id: EntityId,
    pub collection: CollectionId,
    /// Combined state RwLock for atomic head/backends updates
    state: std::sync::RwLock<EntityInnerState>,
    pub(crate) kind: EntityKind,
    /// Broadcast for notifying Signal subscribers about entity changes
    pub(crate) broadcast: ankurah_signals::broadcast::Broadcast,
}

struct EntityInnerState {
    head: Clock,
    backends: BTreeMap<String, Arc<dyn PropertyBackend>>,
}

pub enum EntityKind {
    Primary,
    Transacted { trx_alive: Arc<AtomicBool>, upstream: Entity },
}
```

Key design points:
- `EntityInnerState` wraps both `head` and `backends` under a single `RwLock` for atomic updates
- `EntityKind::Transacted` links back to the `upstream` primary entity and tracks whether the transaction is still alive via `AtomicBool`
- `WeakEntity` wraps `Weak<EntityInner>` for the `WeakEntitySet` (prevents memory leaks)
- `TemporaryEntity` is a separate type used only for reconstituting state to filter database results; provides no duplication guarantees
- `is_writable()` returns `true` only for `Transacted` entities where `trx_alive` is still `true`
- The entity Broadcast fires `()` (unit) on any state change -- field-level broadcasts are on the backends

### Entity Creation Flow

1. **`WeakEntitySet::create(collection)`** generates a new `EntityId::new()` (ULID), calls `Entity::create(id, collection)` which builds a fresh `EntityInner` with empty clock, empty backends, `EntityKind::Primary`, and inserts a weak reference.

2. **`Transaction::create<M>(model)`** (file: `/Users/daniel/ak/ankurah/core/src/transaction.rs`):
   ```rust
   pub async fn create<'rec, 'trx: 'rec, M: Model>(&'trx self, model: &M) -> Result<MutableBorrow<'rec, M::Mutable>, MutationError> {
       let entity = self.dyncontext.create_entity(M::collection(), self.alive.clone());
       model.initialize_new_entity(&entity);
       self.dyncontext.check_write(&entity)?;
       self.created_entity_ids.write().unwrap().insert(entity.id);
       let entity_ref = self.add_entity(entity);
       Ok(MutableBorrow::new(entity_ref))
   }
   ```
   The `context.create_entity()` (in `/Users/daniel/ak/ankurah/core/src/context.rs`):
   ```rust
   fn create_entity(&self, collection: proto::CollectionId, trx_alive: Arc<AtomicBool>) -> Entity {
       let primary_entity = self.node.entities.create(collection);
       primary_entity.snapshot(trx_alive)
   }
   ```
   So: creates a primary in the WeakEntitySet, then immediately snapshots it for the transaction.

3. **`Transaction::get<M>(id)`** either returns an already-forked entity from the transaction, or fetches from context, then `snapshot(trx_alive)` to fork for the transaction.

4. **`Entity::snapshot(trx_alive)`** creates a deep clone:
   ```rust
   pub fn snapshot(&self, trx_alive: Arc<AtomicBool>) -> Self {
       let state = self.state.read().expect("...");
       let mut forked = BTreeMap::new();
       for (name, backend) in &state.backends {
           forked.insert(name.clone(), backend.fork());
       }
       Self(Arc::new(EntityInner {
           id: self.id,
           collection: self.collection.clone(),
           state: RwLock::new(EntityInnerState { head: state.head.clone(), backends: forked }),
           kind: EntityKind::Transacted { trx_alive, upstream: self.clone() },
           broadcast: Broadcast::new(),
       }))
   }
   ```

### Transaction Structure

**File: `/Users/daniel/ak/ankurah/core/src/transaction.rs`**

```rust
pub struct Transaction {
    pub(crate) dyncontext: Arc<dyn TContext + Send + Sync + 'static>,
    pub(crate) id: proto::TransactionId,
    pub(crate) entities: AppendOnlyVec<Entity>,
    pub(crate) alive: Arc<AtomicBool>,
    pub(crate) created_entity_ids: std::sync::RwLock<std::collections::HashSet<EntityId>>,
}
```

- Uses `AppendOnlyVec` for entities (lock-free append)
- `created_entity_ids` tracks which entities were actually created in this transaction (for phantom validation)
- `Drop` implementation marks `alive` as `false`
- `rollback()` explicitly marks `alive` as `false`

### Commit Flow

**File: `/Users/daniel/ak/ankurah/core/src/context.rs` (method `commit_local_trx`)**

1. Atomically CAS `trx.alive` from `true` to `false` (prevents double-commit)
2. For each entity in the transaction, call `entity.generate_commit_event()`:
   ```rust
   pub(crate) fn generate_commit_event(&self) -> Result<Option<Event>, MutationError> {
       let state = self.state.read().expect("...");
       let mut operations = BTreeMap::<String, Vec<Operation>>::new();
       for (name, backend) in &state.backends {
           if let Some(ops) = backend.to_operations()? {
               operations.insert(name.clone(), ops);
           }
       }
       if operations.is_empty() { return Ok(None); }
       Ok(Some(Event { entity_id: self.id, collection: self.collection.clone(),
                        operations: OperationSet(operations), parent: state.head.clone() }))
   }
   ```
3. Validate phantom entities: if `event.is_entity_create()` (empty parent), the entity must be in `created_entity_ids`
4. Policy check with `check_event()` on a temporary fork
5. Store events and update heads: `entity.commit_head(Clock::new([event_id]))`
6. Relay to durable peers (`relay_to_required_peers`) -- waits for all durable peers to confirm
7. Apply event to the canonical upstream entity:
   ```rust
   let canonical_entity = match &entity.kind {
       EntityKind::Transacted { upstream, .. } => {
           upstream.apply_event(&retriever, &attested_event.payload).await?;
           upstream.clone()
       }
       EntityKind::Primary => entity,
   };
   ```
8. Persist state to storage via `collection.set_state(attested)`
9. Notify reactor: `reactor.notify_change(changes)`

### Model/View/Mutable Traits

**File: `/Users/daniel/ak/ankurah/core/src/model.rs`**

```rust
pub trait Model: Sized {
    type View: View;
    type Mutable: Mutable;
    fn collection() -> CollectionId;
    fn initialize_new_entity(&self, entity: &Entity);
}

pub trait View {
    type Model: Model;
    type Mutable: Mutable;
    fn id(&self) -> EntityId { self.entity().id() }
    fn collection() -> CollectionId { <Self::Model as Model>::collection() }
    fn entity(&self) -> &Entity;
    fn from_entity(inner: Entity) -> Self;
    fn to_model(&self) -> Result<Self::Model, PropertyError>;
}

pub trait Mutable {
    type Model: Model;
    type View: View;
    fn id(&self) -> EntityId { self.entity().id() }
    fn entity(&self) -> &Entity;
    fn new(entity: Entity) -> Self where Self: Sized;
    fn state(&self) -> Result<State, StateError> { self.entity().to_state() }
    fn read(&self) -> Self::View {
        let inner = self.entity();
        let new_inner = match &inner.kind {
            EntityKind::Transacted { upstream, .. } => upstream.clone(),
            EntityKind::Primary => inner.clone(),
        };
        Self::View::from_entity(new_inner)
    }
}
```

- `MutableBorrow<'rec, T: Mutable>` provides lifetime-constrained access, tied to the transaction's entity reference
- `Mutable::read()` returns a View of the **upstream** entity (not the transaction fork), so reads during a transaction see committed state

### Example Model (from `/Users/daniel/ak/ankurah/examples/model/src/lib.rs`)

```rust
#[derive(Model, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    #[active_type(LWW)]
    pub timestamp: String,
    pub level: Level,           // defaults to LWW for custom Property types
    #[active_type(YrsString)]
    pub message: String,        // CRDT text
    #[active_type(LWW)]
    pub source: String,
    #[active_type(LWW)]
    pub node_id: String,
    pub payload: Payload,       // custom Property enum via derive(Property)
}
```

The `#[active_type(...)]` attribute selects the property backend. Default is LWW for simple types. The `#[derive(Model)]` macro generates `LogEntryView`, `LogEntryMutable`, plus all trait impls.

---

## 2. Property Backends

### Backend Trait

**File: `/Users/daniel/ak/ankurah/core/src/property/backend/mod.rs`**

```rust
pub trait PropertyBackend: Any + Send + Sync + Debug + 'static {
    fn as_arc_dyn_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync + 'static>;
    fn as_debug(&self) -> &dyn Debug;
    fn fork(&self) -> Arc<dyn PropertyBackend>;
    fn properties(&self) -> Vec<PropertyName>;
    fn property_value(&self, property_name: &PropertyName) -> Option<Value>;
    fn property_values(&self) -> BTreeMap<PropertyName, Option<Value>>;
    fn property_backend_name() -> String where Self: Sized;
    fn to_state_buffer(&self) -> Result<Vec<u8>, StateError>;
    fn from_state_buffer(state_buffer: &Vec<u8>) -> Result<Self, RetrievalError> where Self: Sized;
    fn to_operations(&self) -> Result<Option<Vec<Operation>>, MutationError>;
    fn apply_operations(&self, operations: &Vec<Operation>) -> Result<(), MutationError>;
    fn listen_field(&self, field_name: &PropertyName, listener: Listener) -> ListenerGuard;
}
```

Backend dispatch is **hardcoded string-based** (not a registry):

```rust
pub fn backend_from_string(name: &str, buffer: Option<&Vec<u8>>) -> Result<Arc<dyn PropertyBackend>, RetrievalError> {
    if name == "yrs" { ... }
    else if name == "lww" { ... }
    else { panic!("unknown backend: {:?}", name); }
}
```

### LWW Backend

**File: `/Users/daniel/ak/ankurah/core/src/property/backend/lww.rs`**

```rust
pub struct LWWBackend {
    values: RwLock<BTreeMap<PropertyName, ValueEntry>>,
    field_broadcasts: Mutex<BTreeMap<PropertyName, Broadcast>>,
}

struct ValueEntry {
    value: Option<Value>,
    committed: bool,
}

struct LWWDiff {
    version: u8,    // always 1 currently (const LWW_DIFF_VERSION: u8 = 1)
    data: Vec<u8>,  // inner bincode payload
}
```

- **State serialization**: `bincode::serialize(&BTreeMap<PropertyName, Option<Value>>)` -- the full property map
- **Operations (diff)**: **Double-bincode encoded**:
  ```rust
  // to_operations():
  Operation {
      diff: bincode::serialize(&LWWDiff {
          version: LWW_DIFF_VERSION,           // u8 = 1
          data: bincode::serialize(&changed_values)?  // BTreeMap<PropertyName, Option<Value>>
      })?
  }
  ```
  So the wire format is: `bincode(LWWDiff { version: u8, data: bincode(BTreeMap<String, Option<Value>>) })`
- **apply_operations**: deserializes the outer LWWDiff, checks version==1, then deserializes the inner `data` to get changed values
- Has per-field `Broadcast` for field-level change notifications
- `fork()` clones all values, creates fresh broadcasts (transaction isolation)

### Yrs Backend

**File: `/Users/daniel/ak/ankurah/core/src/property/backend/yrs.rs`**

```rust
pub struct YrsBackend {
    pub(crate) doc: yrs::Doc,
    previous_state: Mutex<StateVector>,
    field_broadcasts: Mutex<BTreeMap<PropertyName, Broadcast>>,
}
```

- **State serialization**: `txn.encode_state_as_update_v2(&StateVector::default())` -- Yrs native v2 encoding, NOT bincode
- **State deserialization**: `Update::decode_v2(state_buffer)` then `txn.apply_update(update)`
- **Operations (diff)**: `txn.encode_diff_v2(&previous_state)` -- Yrs native v2 diff format
  - `Operation::diff` contains raw Yrs v2 update bytes
  - Checks `diff == Update::EMPTY_V2` to detect no-op, returns `None`
- **apply_operations**: `Update::decode_v2(update)` then `txn.apply_update(update)`
- `fork()` does full round-trip: `to_state_buffer()` -> `from_state_buffer()` (severs internal Yrs Arcs, gets new random client_id)
- Per-field broadcasts: during `apply_update`, registers Yrs `observe()` handlers on each known text field to detect which ones changed

### PNCounter Backend (quarantined/disabled)

**File: `/Users/daniel/ak/ankurah/core/src/property/backend/pn_counter.rs`**

- Module is commented out: `//pub mod pn_counter;`
- Would support positive-negative integer counters with types: i8/u8/i16/u16/i32/u32/i64/u64
- Uses bincode for both state and operations

### Property Value Types

**File: `/Users/daniel/ak/ankurah/core/src/property/value/`**

- `LWW<T: Property>` (file: `lww.rs`) -- Last-writer-wins wrapper
  - `set(&self, value: &T)` checks `entity.is_writable()`, then delegates to `backend.set(property_name, value)`
  - `get(&self) -> Result<T, PropertyError>` reads from backend
  - Implements `Signal`, `Subscribe<T>` for reactivity
- `YrsString<Projected>` (file: `yrs.rs`) -- CRDT text
  - `insert(index: u32, value: &str)`, `delete(index: u32, length: u32)`, `replace(value: &str)`, `overwrite(start, length, value)`
  - All mutations check `entity.is_writable()`
  - Implements `Signal`, `Subscribe<String>`
- `Json` (file: `json.rs`) -- Wraps `serde_json::Value`, stored via LWW
  - Supports dot-path queries via `get_path(&[&str])`
  - `#[serde(transparent)]` for clean serialization
- `Ref<T>` (file: `entity_ref.rs`) -- Typed entity reference
  - `#[serde(transparent)]` wrapping `EntityId`
  - `async get(&self, ctx: &Context) -> Result<T::View, RetrievalError>`
  - `Property` impl maps to/from `Value::EntityId`

### Value Enum

**File: `/Users/daniel/ak/ankurah/core/src/value/mod.rs`**

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Value {
    I16(i16),       // variant 0
    I32(i32),       // variant 1
    I64(i64),       // variant 2
    F64(f64),       // variant 3
    Bool(bool),     // variant 4
    String(String), // variant 5
    EntityId(proto::EntityId),  // variant 6
    Object(Vec<u8>),            // variant 7
    Binary(Vec<u8>),            // variant 8
    #[serde(with = "json_as_bytes")]
    Json(serde_json::Value),    // variant 9
}
```

The `Json` variant uses a custom serde wrapper `json_as_bytes` that serializes `serde_json::Value` to JSON bytes first, then lets bincode serialize those bytes as a `Vec<u8>`. This is because **bincode does not support `deserialize_any`** which `serde_json::Value` requires.

```rust
mod json_as_bytes {
    pub fn serialize<S>(value: &serde_json::Value, serializer: S) -> Result<S::Ok, S::Error> {
        let bytes = serde_json::to_vec(value).map_err(serde::ser::Error::custom)?;
        bytes.serialize(serializer)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<serde_json::Value, D::Error> {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        serde_json::from_slice(&bytes).map_err(serde::de::Error::custom)
    }
}
```

---

## 3. Clock/ID System

### EntityId

**File: `/Users/daniel/ak/ankurah/proto/src/id.rs`**

```rust
#[derive(PartialEq, Eq, Hash, Clone, Copy, Ord, PartialOrd)]
pub struct EntityId(pub(crate) Ulid);  // 16 bytes
```

- Generated via `Ulid::new()` (monotonic, timestamp-sorted)
- Custom serde depending on format:
  ```rust
  impl Serialize for EntityId {
      fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
          if serializer.is_human_readable() {
              serializer.serialize_str(&self.to_base64())   // base64url-no-pad
          } else {
              self.to_bytes().serialize(serializer)          // raw [u8; 16]
          }
      }
  }
  ```
- Base64 encoding uses `general_purpose::URL_SAFE_NO_PAD`
- `to_base64_short()` returns last 6 characters (for display)
- Confirmed by test: `bincode::serialize(&id)` produces exactly 16 bytes (no length prefix)

### EventId

**File: `/Users/daniel/ak/ankurah/proto/src/data.rs`**

```rust
pub struct EventId([u8; 32]);  // SHA-256 hash
```

- **Content-addressed**: `EventId::from_parts(entity_id, operations, parent)`:
  ```rust
  pub fn from_parts(entity_id: &EntityId, operations: &OperationSet, parent: &Clock) -> Self {
      let mut hasher = Sha256::new();
      hasher.update(bincode::serialize(&entity_id).unwrap());   // 16 bytes (fixed array)
      hasher.update(bincode::serialize(&operations).unwrap());   // variable
      hasher.update(bincode::serialize(&parent).unwrap());       // variable
      Self(hasher.finalize().into())
  }
  ```
- Same dual serialization as EntityId (base64 for JSON, raw `[u8; 32]` for bincode)
- Deterministic: same event data always produces same EventId

### Clock

**File: `/Users/daniel/ak/ankurah/proto/src/clock.rs`**

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct Clock(pub(crate) Vec<EventId>);
```

- A **sorted vector of EventIds** representing a frontier (antichain) in a DAG
- `is_empty()` indicates entity creation (no parent events)
- `insert()` uses binary search to maintain sorted order, skips duplicates
- `contains()` uses binary search
- `with_event(id)` creates a clone with the new event inserted
- Serialized by serde as a `Vec<EventId>` -- which in bincode means: 8-byte u64 LE length prefix, then each EventId as 32 raw bytes

### Other ID Types

| Type | Location | Structure |
|------|----------|-----------|
| `TransactionId(Ulid)` | `proto/src/transaction.rs` | ULID-based |
| `RequestId(Ulid)` | `proto/src/request.rs` | ULID-based |
| `UpdateId(Ulid)` | `proto/src/update.rs` | ULID-based |
| `QueryId(Ulid)` | `proto/src/subscription.rs` | ULID-based |
| `CollectionId(String)` | `proto/src/collection.rs` | String-based |

All ULID-based IDs use `Ulid::new()` for generation. Note that `Ulid` itself uses the standard serde derivation (not custom like EntityId), so it serializes as a 16-byte tuple struct in bincode.

---

## 4. Node/Context/Reactor

### Node

**File: `/Users/daniel/ak/ankurah/core/src/node.rs`**

```rust
pub struct Node<SE, PA>(pub(crate) Arc<NodeInner<SE, PA>>);

pub struct NodeInner<SE, PA> where PA: PolicyAgent {
    pub id: proto::EntityId,
    pub durable: bool,
    pub collections: CollectionSet<SE>,
    pub(crate) entities: WeakEntitySet,
    peer_connections: SafeMap<proto::EntityId, Arc<PeerState>>,
    durable_peers: SafeSet<proto::EntityId>,
    pub(crate) predicate_context: SafeMap<proto::QueryId, PA::ContextData>,
    pub(crate) reactor: Reactor,
    pub(crate) policy_agent: PA,
    pub system: SystemManager<SE, PA>,
    pub(crate) subscription_relay: Option<SubscriptionRelay<PA::ContextData, WeakEntityLiveQuery>>,
    pub(crate) type_resolver: crate::TypeResolver,
}

pub struct PeerState {
    sender: Box<dyn PeerSender>,
    _durable: bool,
    subscription_handler: SubscriptionHandler,
    pending_requests: SafeMap<proto::RequestId, oneshot::Sender<Result<NodeResponseBody, RequestError>>>,
    pending_updates: SafeMap<proto::UpdateId, oneshot::Sender<Result<NodeResponseBody, RequestError>>>,
}
```

- Node is generic over `SE: StorageEngine` and `PA: PolicyAgent`
- Two creation modes: `new()` (ephemeral, gets subscription_relay) and `new_durable()` (durable, no relay)
- Node ID is a fresh EntityId on each creation
- Request pattern: `request()` sends `NodeMessage::Request`, stores `oneshot::Sender` in `pending_requests`, awaits response on the `oneshot::Receiver`
- Peer lifecycle: `register_peer()` / `deregister_peer()` manage peer connections
- `handle_message()` dispatches to `handle_request()` / `handle_update()` depending on message type

### Context

**File: `/Users/daniel/ak/ankurah/core/src/context.rs`**

```rust
pub struct Context(Arc<dyn TContext + Send + Sync + 'static>);

pub struct NodeAndContext<SE, PA: PolicyAgent> {
    pub node: Node<SE, PA>,
    pub cdata: PA::ContextData,
}
```

- Context is type-erased (`dyn TContext`) to hide the generic parameters SE and PA
- `NodeAndContext<SE, PA>` implements `TContext` combining Node + ContextData
- API surface:
  - `begin()` -> `Transaction`
  - `get<V: View>(id)` -> fetch single entity
  - `get_cached<V: View>(id)` -> fetch, ok to use local cache
  - `fetch<V: View>(args)` -> query multiple entities
  - `fetch_one<V: View>(args)` -> query, return first
  - `query<V: View>(args)` -> live query (subscription)
  - `query_wait<V: View>(args)` -> live query, wait for initialization

### Reactor

**File: `/Users/daniel/ak/ankurah/core/src/reactor.rs`**

```rust
pub struct Reactor<E = Entity, Ev = Attested<Event>>(Arc<ReactorInner<E, Ev>>);

struct ReactorInner<E, Ev> {
    subscriptions: Mutex<HashMap<ReactorSubscriptionId, Subscription<E, Ev>>>,
    watcher_set: Arc<Mutex<WatcherSet>>,
    notify_lock: tokio::sync::Mutex<()>,
}
```

Subscription flow:
1. `reactor.subscribe()` creates a `ReactorSubscription` with a `Broadcast` for notifications
2. `add_query_and_notify()` fetches initial entities from local storage, registers with WatcherSet, sends Initial notifications
3. `update_query_and_notify()` diffs against current resultset, sends Add/Remove/Update notifications

Notification flow (`notify_change`):
1. Acquires `notify_lock` (serializes all notifications)
2. First pass (lock watcher_set): accumulate interested watchers per subscription using `CandidateChanges`
3. Second pass (lock subscriptions): parallel `evaluate_changes()` via `join_all()` -- evaluates predicates, determines membership changes
4. Third pass (lock watcher_set): apply watcher changes (add/remove entity watchers)

WatcherSet supports three watcher types:
- **Index watchers**: field-value specific (e.g., watch for changes to `status` field)
- **Wildcard watchers**: collection-wide (e.g., watch all entities in "album" collection)
- **Entity watchers**: specific entities by ID

`AbstractEntity` trait enables generic reactor use:
```rust
pub trait AbstractEntity: Clone + std::fmt::Debug {
    fn collection(&self) -> proto::CollectionId;
    fn id(&self) -> &proto::EntityId;
    fn value(&self, field: &str) -> Option<Value>;
}
```

---

## 5. Signals System

**File: `/Users/daniel/ak/ankurah/signals/src/`**

### Core Traits (file: `signal.rs`)

```rust
pub type Listener = Arc<dyn Fn(()) + Send + Sync + 'static>;

/// Core observation (no value payload)
pub trait Signal {
    fn listen(&self, listener: Listener) -> ListenerGuard;
    fn broadcast_id(&self) -> BroadcastId;
}

/// Value reading (tracked by context for automatic dependency tracking)
pub trait Get<T: 'static>: Signal { fn get(&self) -> T; }

/// Closure-based value access
pub trait With<T: 'static> { fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R; }

/// Untracked value reading
pub trait Peek<T: 'static> { fn peek(&self) -> T; }
```

### Subscribe Trait (file: `porcelain/subscribe.rs`)

```rust
pub trait Subscribe<T: 'static> {
    fn subscribe<F: IntoSubscribeListener<T>>(&self, listener: F) -> SubscriptionGuard;
}

pub struct SubscriptionGuard {
    _listenerguard: Box<dyn std::any::Any + Send + Sync>,
}
```

### Broadcast (file: `broadcast.rs`)

```rust
pub struct Broadcast<T = ()>(Arc<Inner<T>>);

struct Inner<T> {
    listeners: RwLock<HashMap<usize, BroadcastListener<T>>>,
    next_id: AtomicUsize,
}

pub struct BroadcastId(usize);  // Arc pointer address

pub enum BroadcastListener<T = ()> {
    Payload(Arc<dyn Fn(T) + Send + Sync + 'static>),
    NotifyOnly(Arc<dyn Fn() + Send + Sync + 'static>),
}
```

- `BroadcastId` is derived from `Arc::as_ptr()` cast to `usize` -- unique per allocation, stable while any Arc/Weak exists
- `send(value)` snapshots listeners (releases RwLock), then calls each without holding locks -- reentrant-safe
- `ListenerGuard` holds `Weak<Inner<T>>` and auto-unsubscribes on drop

### Signal Types (file: `signal/`)

- `Mut<T>` -- Mutable signal (read/write)
- `Read<T>` -- Read-only derived signal
- `Memo<T>` -- Memoized computed signal
- `Map<S, T>` -- Mapped signal
- `Calculated<T>` -- Computed signal

### How Properties Integrate with Signals

LWW and Yrs property types implement `Signal` and `Subscribe<T>`:
```rust
// LWW<T> implements Signal
impl<T: Property> Signal for LWW<T> {
    fn listen(&self, listener: Listener) -> ListenerGuard {
        self.backend.listen_field(&self.property_name, listener)
    }
    fn broadcast_id(&self) -> BroadcastId {
        self.backend.field_broadcast_id(&self.property_name)
    }
}

// LWW<T> implements Subscribe<T>
impl<T: Property + Clone + Send + Sync + 'static> Subscribe<T> for LWW<T> {
    fn subscribe<F>(&self, listener: F) -> SubscriptionGuard {
        let lww = self.clone();
        let subscription = self.listen(Arc::new(move |_| {
            if let Ok(current_value) = lww.get() {
                listener(current_value);
            }
        }));
        SubscriptionGuard::new(subscription)
    }
}
```

---

## 6. Proto Types (Wire Format)

### Message Hierarchy

**File: `/Users/daniel/ak/ankurah/proto/src/message.rs`**

```rust
pub enum Message {                              // variant 0 or 1
    Presence(Presence),
    PeerMessage(NodeMessage),
}

pub enum NodeMessage {                          // variant 0..4
    Request { auth: Vec<AuthData>, request: NodeRequest },
    Response(NodeResponse),
    Update(NodeUpdate),
    UpdateAck(NodeUpdateAck),
    UnsubscribeQuery { from: EntityId, query_id: QueryId },
}
```

### Core Data Types

**File: `/Users/daniel/ak/ankurah/proto/src/data.rs`**

```rust
pub struct Event {
    pub collection: CollectionId,       // String (length-prefixed)
    pub entity_id: EntityId,            // [u8; 16]
    pub operations: OperationSet,       // BTreeMap<String, Vec<Operation>>
    pub parent: Clock,                  // Vec<EventId>
}

pub struct OperationSet(pub BTreeMap<String, Vec<Operation>>);
pub struct Operation { pub diff: Vec<u8> }

pub struct State {
    pub state_buffers: StateBuffers,    // BTreeMap<String, Vec<u8>>
    pub head: Clock,                    // Vec<EventId>
}
pub struct StateBuffers(pub BTreeMap<String, Vec<u8>>);

pub struct EntityState {
    pub entity_id: EntityId,
    pub collection: CollectionId,
    pub state: State,
}

pub struct EventFragment {
    pub operations: OperationSet,
    pub parent: Clock,
    pub attestations: AttestationSet,
}

pub struct StateFragment {
    pub state: State,
    pub attestations: AttestationSet,
}
```

### Request/Response Types

**File: `/Users/daniel/ak/ankurah/proto/src/request.rs`**

```rust
pub struct NodeRequest {
    pub id: RequestId,          // Ulid
    pub to: EntityId,
    pub from: EntityId,
    pub body: NodeRequestBody,
}

pub enum NodeRequestBody {                                      // variant 0..4
    CommitTransaction { id: TransactionId, events: Vec<Attested<Event>> },
    Get { collection: CollectionId, ids: Vec<EntityId> },
    GetEvents { collection: CollectionId, event_ids: Vec<EventId> },
    Fetch { collection: CollectionId, selection: ast::Selection, known_matches: Vec<KnownEntity> },
    SubscribeQuery { query_id: QueryId, collection: CollectionId, selection: ast::Selection, version: u32, known_matches: Vec<KnownEntity> },
}

pub struct NodeResponse {
    pub request_id: RequestId,
    pub from: EntityId,
    pub to: EntityId,
    pub body: NodeResponseBody,
}

pub enum NodeResponseBody {                                     // variant 0..6
    CommitComplete { id: TransactionId },
    Fetch(Vec<EntityDelta>),
    Get(Vec<Attested<EntityState>>),
    GetEvents(Vec<Attested<Event>>),
    QuerySubscribed { query_id: QueryId, deltas: Vec<EntityDelta> },
    Success,
    Error(String),
}

pub struct KnownEntity {
    pub entity_id: EntityId,
    pub head: Clock,
}

pub enum DeltaContent {                                         // variant 0..2
    StateSnapshot { state: StateFragment },
    EventBridge { events: Vec<EventFragment> },
    StateAndRelation { state: StateFragment, relation: CausalAssertionFragment },
}

pub struct EntityDelta {
    pub entity_id: EntityId,
    pub collection: CollectionId,
    pub content: DeltaContent,
}
```

### Update Types

**File: `/Users/daniel/ak/ankurah/proto/src/update.rs`**

```rust
pub enum NodeUpdateBody {
    SubscriptionUpdate { items: Vec<SubscriptionUpdateItem> },
}

pub enum UpdateContent {                                        // variant 0..1
    EventOnly(Vec<EventFragment>),
    StateAndEvent(StateFragment, Vec<EventFragment>),
}

pub enum MembershipChange {                                     // variant 0..2
    Initial,
    Add,
    Remove,
}

pub struct SubscriptionUpdateItem {
    pub entity_id: EntityId,
    pub collection: CollectionId,
    pub content: UpdateContent,
    pub predicate_relevance: Vec<(QueryId, MembershipChange)>,
}
```

### Auth Types

**File: `/Users/daniel/ak/ankurah/proto/src/auth.rs`**

```rust
pub struct AuthData(pub Vec<u8>);
pub struct Attestation(pub Vec<u8>);
pub struct AttestationSet(pub Vec<Attestation>);
pub struct Attested<T> {
    pub payload: T,
    pub attestations: AttestationSet,
}
```

### Presence

**File: `/Users/daniel/ak/ankurah/proto/src/peering.rs`**

```rust
pub struct Presence {
    pub node_id: EntityId,
    pub durable: bool,
    pub system_root: Option<Attested<EntityState>>,
}
```

### Wire Serialization

All websocket communication uses **bincode** for the outer `Message` envelope:

Server side (`/Users/daniel/ak/ankurah/connectors/websocket-server/src/server.rs`):
```rust
if let Ok(message) = deserialize::<proto::Message>(&d) { ... }
```

Server sender (`/Users/daniel/ak/ankurah/connectors/websocket-server/src/sender.rs`):
```rust
let data = bincode::serialize(&message).map_err(...)?;
```

Client side (`/Users/daniel/ak/ankurah/connectors/websocket-client/src/client.rs`):
```rust
sink.send(Message::Binary(bincode::serialize(&presence)?.into())).await?;
// ...
match bincode::serialize(&proto_message) { ... }
// ...
Some(Ok(Message::Binary(data))) => match bincode::deserialize(&data) { ... }
```

The entire `Message` enum tree (including nested AST selections, events, states) goes through bincode in a single pass.

---

## 7. PR #236 Status / sys.rs

**File: `/Users/daniel/ak/ankurah/proto/src/sys.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    SysRoot,
    Collection { name: String },
    #[serde(other)]
    Other,
}
```

This is minimal. There are **no `BackendKind` or `ValueType` enums in sys.rs**.

The `ValueType` enum exists in `/Users/daniel/ak/ankurah/core/src/value/mod.rs`:
```rust
pub enum ValueType { I16, I32, I64, F64, Bool, String, EntityId, Object, Binary, Json }
```

There is no explicit `BackendKind` enum anywhere in the codebase. Backends are identified by string name (`"lww"`, `"yrs"`) and dispatched via the hardcoded `backend_from_string()` function. If PR #236 proposed adding BackendKind/ValueType to sys.rs, it has not landed in the current codebase.

---

## 8. AnkQL Grammar

### PEG Grammar (Pest)

**File: `/Users/daniel/ak/ankurah/ankql/src/ankql.pest`**

```pest
Selection = _{ SOI ~ Expr ~ OrderByClause? ~ LimitClause? ~ EOI }

Expr = { ExprAtomValue ~ (ExprInfixOp ~ ExprAtomValue)* }
    ExprInfixOp = _{ Between | ArithInfixOp | CmpInfixOp | And | Or }
        And = @{ ^"and" ~ !IDENT_CONT }
        Or  = @{ ^"or" ~ !IDENT_CONT }
        CmpInfixOp = _{ NotEq | GtEq | Gt | LtEq | Lt | Eq | Lt | In }
            Eq = { "=" }  Gt = { ">" }  GtEq = { ">=" }
            Lt = { "<" }  LtEq = { "<=" }
            NotEq = { "<>" | "!=" }
            In = { NotFlag? ~ ^"in" }
    ExprAtomValue = _{ UnaryNot* ~ AtomicExpr ~ IsNullPostfix? }
        IsNullPostfix = { ^"is" ~ NotFlag? ~ ^"null" }
        AtomicExpr = _{ Literal | QuestionParameter | PathExpr | ExpressionInParentheses | Row }
            Literal = _{ True | False | Null | Double | Decimal | Unsigned | Integer | SingleQuotedString }
            QuestionParameter = @{ "?" }
            PathExpr = { Identifier ~ ("." ~ Identifier)* }
            Row = { "(" ~ Expr ~ ("," ~ Expr)* ~ ")" }

OrderByClause = ${ ^"order" ~ WS+ ~ ^"by" ~ WS+ ~ (OrderByItem ~ ...) }
OrderByItem = ${ Identifier ~ (WS+ ~ OrderDirection)? }
OrderDirection = { ^"asc" | ^"desc" }
LimitClause = ${ ^"limit" ~ WS+ ~ Unsigned }
```

Key features:
- SQL-like predicate syntax with AND/OR/NOT/IN/BETWEEN/IS NULL
- Dot-separated paths for nested property access (e.g., `licensing.territory`)
- ORDER BY with ASC/DESC direction, multiple columns
- LIMIT clause
- Placeholder `?` for parameterized queries
- Keywords are case-insensitive (`^"and"` means case-insensitive match)
- Identifiers can overlap with reserved words when not in keyword position (e.g., `limit = 1` is valid because the parser tries `Reserved` first and it doesn't match the context)

### AST Types

**File: `/Users/daniel/ak/ankurah/ankql/src/ast.rs`**

```rust
pub enum Expr {
    Literal(Literal),
    Path(PathExpr),
    Predicate(Predicate),
    InfixExpr { left: Box<Expr>, operator: InfixOperator, right: Box<Expr> },
    ExprList(Vec<Expr>),
    Placeholder,
}

pub enum Literal {
    I16(i16), I32(i32), I64(i64), F64(f64),
    Bool(bool), String(String),
    EntityId(Ulid),
    Object(Vec<u8>), Binary(Vec<u8>),
    #[serde(with = "json_as_bytes")]
    Json(serde_json::Value),
}

pub struct PathExpr { pub steps: Vec<String> }

pub struct Selection {
    pub predicate: Predicate,
    pub order_by: Option<Vec<OrderByItem>>,
    pub limit: Option<u64>,
}

pub struct OrderByItem {
    pub path: PathExpr,
    pub direction: OrderDirection,
}

pub enum Predicate {
    Comparison { left: Box<Expr>, operator: ComparisonOperator, right: Box<Expr> },
    IsNull(Box<Expr>),
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),
    True, False, Placeholder,
}

pub enum ComparisonOperator { Equal, NotEqual, GreaterThan, GreaterThanOrEqual, LessThan, LessThanOrEqual, In, Between }
pub enum InfixOperator { Add, Subtract, Multiply, Divide }
```

The entire AST is `Serialize + Deserialize` because it gets sent over the wire (in `NodeRequestBody::Fetch` and `SubscribeQuery`). The `Literal::Json` variant uses the same `json_as_bytes` pattern to work around bincode's lack of `deserialize_any`.

Predicate supports parameterized queries via `.populate(values)` which fills in `Placeholder` nodes.

---

## 9. Yrs Client ID Assignment

**Confirmed open question.** Searching the entire Rust codebase reveals no matches for `client_id`, `client_options`, `Options`, or `ClientId`.

Every Yrs Doc creation uses default options:

```rust
// YrsBackend::new()
let doc = yrs::Doc::new();

// YrsBackend::from_state_buffer()
let doc = yrs::Doc::new();
```

`yrs::Doc::new()` internally generates a random `u64` client_id via `rand`. This means:
- Each `YrsBackend::new()` gets a unique random Yrs client_id
- Each `fork()` (which round-trips through `to_state_buffer()`/`from_state_buffer()`) gets a **different** random client_id
- Multiple edits from the same "user" or "node" may appear as different Yrs clients
- There is no deterministic client_id derivation from node ID or entity ID

For the TS port, this is an important design decision: the Yjs `Doc` constructor accepts a `clientID` option. Whether to assign deterministic client IDs (e.g., derived from node ID or hash thereof) depends on desired CRDT merge semantics and debugging traceability.

---

## 10. Bincode Configuration -- CRITICAL FOR TS PORT

### Version

All Cargo.toml files use **bincode 1.3.x** (not bincode 2):
```toml
bincode = "1.3.3"
```

### Configuration: DEFAULT LEGACY FORMAT

**Every single bincode call in the entire codebase uses bare `bincode::serialize()` / `bincode::deserialize()`.** There is no use of `DefaultOptions`, `with_varint`, `with_fixint`, `Options` trait, or any configuration whatsoever.

This means the codebase uses **bincode 1.3's legacy format**, which is different from `DefaultOptions`:

| Aspect | Legacy (what ankurah uses) | DefaultOptions |
|--------|---------------------------|----------------|
| Sequence lengths | u64 fixed LE | varint |
| Integer encoding | Fixed-width LE | Fixed-width LE |
| Enum variants | u32 fixed LE | varint |
| Byte limit | Unlimited | Unlimited |
| Trailing bytes | Allowed | Rejected |

### Complete Type Encoding Table

| Rust Type | Wire Encoding |
|-----------|--------------|
| `bool` | 1 byte: `0x00` (false) or `0x01` (true) |
| `u8` / `i8` | 1 byte |
| `u16` / `i16` | 2 bytes little-endian |
| `u32` / `i32` | 4 bytes little-endian |
| `u64` / `i64` | 8 bytes little-endian |
| `f64` | 8 bytes little-endian IEEE 754 |
| `String` | 8 bytes LE u64 byte-length + UTF-8 bytes |
| `Vec<T>` | 8 bytes LE u64 element-count + elements in sequence |
| `Option<T>` | 1 byte tag (`0x00`=None, `0x01`=Some) + value if Some |
| enum variant | **4 bytes LE u32** variant index + variant fields |
| `BTreeMap<K,V>` | 8 bytes LE u64 entry-count + entries in key-sorted order |
| `[u8; N]` (fixed array) | N bytes raw (**NO** length prefix) |
| `(A, B)` tuple | A then B, no delimiters |
| `struct { a, b }` | a then b in declaration order, no delimiters |
| newtype struct `Foo(T)` | same as T (transparent) |

### Special Serialization Patterns

**EntityId** (custom Serialize/Deserialize):
- Binary (bincode): raw `[u8; 16]` -- 16 bytes, NO length prefix
- JSON: base64url-no-pad string

**EventId** (custom Serialize/Deserialize):
- Binary (bincode): raw `[u8; 32]` -- 32 bytes, NO length prefix
- JSON: base64url-no-pad string

**Clock** (standard derive):
- `Vec<EventId>` so: 8-byte u64 LE count + (count * 32 bytes raw EventIds)

**Value::Json** / **Literal::Json** (custom `serde(with = "json_as_bytes")`):
- Serialize: `serde_json::to_vec(value)` -> `Vec<u8>` -> bincode `Vec<u8>` (8-byte length prefix + JSON bytes)
- Deserialize: bincode `Vec<u8>` -> `serde_json::from_slice()`

**LWW Operations** (double-encoded):
```
bincode(LWWDiff {
    version: u8,                              // 1 byte
    data: Vec<u8>,                            // 8-byte len + bincode(BTreeMap<String, Option<Value>>)
})
```
The outer struct is bincode-serialized, and the `data` field contains another bincode payload.

**Yrs State/Operations** -- NOT bincode:
- State buffer: `yrs::encode_state_as_update_v2()` -- Yrs native v2 format
- Operation diff: `yrs::encode_diff_v2()` -- Yrs native v2 format
- These are stored as `Vec<u8>` within the bincode-serialized State/OperationSet

### Critical Bincode Serialization Points

1. **Wire protocol** (all websocket traffic): `bincode::serialize(&proto::Message)` / `bincode::deserialize::<proto::Message>(&data)`
2. **LWW state buffer**: `bincode::serialize(&BTreeMap<PropertyName, Option<Value>>)`
3. **LWW operation diff**: double-encoded as described above
4. **EventId computation**: SHA-256 of `bincode(entity_id) || bincode(operations) || bincode(parent)`
5. **Storage engines** (sled, sqlite, postgres): bincode for StateFragments, OperationSets, AttestationSets
6. **Yrs state/operations**: Yrs native v2 encoding inside `Vec<u8>` fields

---

## Summary of Critical Patterns for TS Port

1. **Entity lifecycle**: Primary entities live in WeakEntitySet; transactions work on `snapshot()` forks with `EntityKind::Transacted`. The upstream entity is the source of truth for reads during transactions.

2. **Backend dispatch**: String-based (`"lww"`, `"yrs"`), hardcoded in `backend_from_string()`, not extensible via registry.

3. **State format**: `State { state_buffers: BTreeMap<String, Vec<u8>>, head: Clock }` where each `Vec<u8>` is backend-specific (bincode for LWW, Yrs v2 for Yrs).

4. **Operation format**: `OperationSet(BTreeMap<String, Vec<Operation>>)` where each `Operation { diff: Vec<u8> }` is backend-specific. LWW uses double-bincode; Yrs uses native v2 diffs.

5. **EventId is content-addressed**: SHA-256 of `bincode(entity_id) || bincode(operations) || bincode(parent)`. The TS port must produce identical hashes.

6. **Clock is a sorted Vec of EventId**: represents a DAG frontier. Empty clock means creation event.

7. **Bincode 1.3 legacy format**: u64 LE length prefixes for sequences, u32 LE enum variants, fixed-width integers. NOT varint.

8. **Yrs uses native v2 encoding**: NOT bincode for CRDT data. This maps to Yjs `encodeStateAsUpdateV2` / `applyUpdateV2` in the JS ecosystem.

9. **No explicit Yrs client_id**: random per `Doc::new()`. Open design question for TS port.

10. **Wire protocol**: bincode-serialized `Message` enum over WebSocket binary frames. The entire message tree including AnkQL AST nodes goes through a single bincode pass.

11. **AnkQL AST is wire-serializable**: `Selection`, `Predicate`, `Expr`, `Literal` all derive `Serialize + Deserialize`. The TS port needs matching AST types that produce identical bincode.

12. **Value::Json / Literal::Json workaround**: serde_json::Value cannot be directly bincode-serialized. It is first converted to JSON bytes, then those bytes are serialized as `Vec<u8>` by bincode.

13. **Transaction commit is atomic but multi-step**: CAS on alive flag -> generate events -> policy check -> store events -> relay to peers (await) -> apply to upstream entity -> persist state -> notify reactor.

14. **Reactor notifications are serialized**: The `notify_lock` ensures only one `notify_change` runs at a time, but individual subscription evaluations within that call are parallelized.
