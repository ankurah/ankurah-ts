# Structural Mapping Analysis: Rust → TypeScript

## The Core Question

How closely can ankurah-ts mirror the Rust codebase's file/struct/impl/test structure 1:1? The closer the mapping, the easier it is for agents to:
1. **Validate** that the port is correct by comparing corresponding files
2. **Detect drift** when the Rust codebase changes
3. **Apply patches** by translating Rust diffs to TS diffs mechanically
4. **Run conformance tests** that verify behavioral equivalence

## File-Level Mapping

### Highly Mappable (1:1)

These modules translate cleanly with near-identical file structures:

| Rust File | TS File | Mapping Quality | Notes |
|-----------|---------|-----------------|-------|
| `proto/src/lib.rs` | `packages/proto/src/index.ts` | Direct | Type definitions, enums |
| `proto/src/data.rs` | `packages/proto/src/data.ts` | Direct | Event, State, Clock types |
| `proto/src/update.rs` | `packages/proto/src/update.ts` | Direct | NodeMessage, UpdateContent |
| `proto/src/sys.rs` | `packages/proto/src/sys.ts` | Direct | System entity types |
| `core/src/entity.rs` | `packages/core/src/entity.ts` | Direct | Entity class |
| `core/src/model.rs` | `packages/core/src/model.ts` | Direct | Model/View/Mutable traits→interfaces |
| `core/src/context.rs` | `packages/core/src/context.ts` | Direct | Context class |
| `core/src/node.rs` | `packages/core/src/node.ts` | Direct | Node class |
| `core/src/transaction.rs` | `packages/core/src/transaction.ts` | Direct | Transaction class |
| `core/src/connector.rs` | `packages/core/src/connector.ts` | Direct | PeerSender interface |
| `core/src/changes.rs` | `packages/core/src/changes.ts` | Direct | EntityChange, ItemChange |
| `core/src/resultset.rs` | `packages/core/src/resultset.ts` | Direct | ResultSet class |
| `core/src/livequery.rs` | `packages/core/src/livequery.ts` | Direct | LiveQuery class |
| `core/src/value/mod.rs` | `packages/core/src/value/index.ts` | Direct | Value enum → discriminated union |
| `core/src/property/mod.rs` | `packages/core/src/property/index.ts` | Direct | Property trait → interface |
| `core/src/property/value/lww.rs` | `packages/core/src/property/value/lww.ts` | Direct | LWW wrapper |
| `core/src/property/value/yrs.rs` | `packages/core/src/property/value/yjs.ts` | Direct | YjsText wrapper (uses Yjs) |
| `core/src/property/value/entity_ref.rs` | `packages/core/src/property/value/entity-ref.ts` | Direct | Ref<T> |
| `core/src/property/value/json.rs` | `packages/core/src/property/value/json.ts` | Direct | Json wrapper |
| `core/src/property/backend/mod.rs` | `packages/core/src/property/backend/index.ts` | Direct | PropertyBackend interface |
| `core/src/property/backend/lww.rs` | `packages/core/src/property/backend/lww.ts` | Direct | LWW backend |
| `core/src/property/backend/yrs.rs` | `packages/core/src/property/backend/yjs.ts` | Near-direct | Uses Yjs instead of Yrs |
| `core/src/reactor.rs` | `packages/core/src/reactor.ts` | Direct | Reactor, Subscription |
| `core/src/selection/filter.rs` | `packages/core/src/selection/filter.ts` | Direct | Predicate evaluation |
| `core/src/retrieval.rs` | `packages/core/src/retrieval.ts` | Direct | LocalRetriever |
| `core/src/schema.rs` | `packages/core/src/schema.ts` | Direct | CollectionSchema |
| `core/src/type_resolver.rs` | `packages/core/src/type-resolver.ts` | Direct | TypeResolver |
| `core/src/system.rs` | `packages/core/src/system.ts` | Direct | SystemManager |
| `core/src/error.rs` | `packages/core/src/error.ts` | Direct | Error types |
| `core/src/node_applier.rs` | `packages/core/src/node-applier.ts` | Direct | NodeApplier |
| `signals/src/signal.rs` | `packages/signals/src/signal.ts` | Direct | Signal trait → interface |
| `signals/src/broadcast.rs` | `packages/signals/src/broadcast.ts` | Direct | Broadcast class |
| `signals/src/observer.rs` | `packages/signals/src/observer.ts` | Direct | Observer class |
| `ankql/src/grammar.rs` | `packages/ankql/src/grammar.ts` | Near-direct | Parser (Pest → PEG.js or custom) |
| `ankql/src/ast.rs` | `packages/ankql/src/ast.ts` | Direct | AST types |
| `ankql/src/conversion.rs` | `packages/ankql/src/conversion.ts` | Direct | AST conversions |
| `storage/common/src/lib.rs` | `packages/storage-common/src/index.ts` | Direct | StorageEngine interface |
| `storage/sqlite/src/lib.rs` | `packages/storage-expo-sqlite/src/index.ts` | Near-direct | Different SQLite API |
| `connectors/websocket-client/src/lib.rs` | `packages/connector-websocket/src/index.ts` | Near-direct | Different WS API |
| `connectors/local-process/src/lib.rs` | `packages/connector-local/src/index.ts` | Direct | In-process connector |

### Structurally Different (Requires Adaptation)

| Rust File | TS Equivalent | Why Different |
|-----------|--------------|---------------|
| `derive/src/**` | `cli/src/**` | Rust proc macros → CLI code generator |
| `derive/src/model/backend_registry.rs` | `cli/src/backend-registry.ts` | RON config → JSON/TS config |
| `derive/src/model/description.rs` | `cli/src/schema-parser.ts` | Parse schema file instead of Rust AST |
| `derive/src/model/view.rs` | `cli/src/generators/view.ts` | Code generation, not macro expansion |
| `derive/src/model/mutable.rs` | `cli/src/generators/mutable.ts` | Code generation, not macro expansion |
| `derive/src/model/wasm.rs` | N/A | WASM bindings not needed |
| `derive/src/model/uniffi.rs` | N/A | UniFFI bindings not needed |
| `derive/src/tsify/**` | N/A | TS types ARE the target |

### Not Needed (Rust-Specific)

| Rust File | Why Not Needed |
|-----------|---------------|
| `core/src/property/backend/pn_counter.rs` | De-scoped for Phase 1 |
| `storage/sled/**` | Not a target storage engine |
| `storage/postgres/**` | De-scoped for Phase 1 |
| `storage/indexeddb-wasm/**` | Not needed (use expo-sqlite) |
| `connectors/websocket-server/**` | De-scoped for Phase 1 |
| `connectors/websocket-client-wasm/**` | Not needed (use standard WS) |
| `tests-wasm/**` | Not applicable |

## Struct/Interface-Level Mapping

### Traits → Interfaces

Rust traits map cleanly to TypeScript interfaces:

```rust
// Rust
pub trait PropertyBackend: Any + Send + Sync + Debug {
    fn property_value(&self, name: &PropertyName) -> Option<Value>;
    fn property_values(&self) -> BTreeMap<PropertyName, Option<Value>>;
    fn to_state_buffer(&self) -> Result<Vec<u8>>;
    fn from_state_buffer(buffer: &Vec<u8>) -> Result<Self>;
    fn to_operations(&self) -> Result<Option<Vec<Operation>>>;
    fn apply_operations(&self, ops: &Vec<Operation>) -> Result<()>;
    fn listen_field(&self, field_name: &PropertyName, listener: Listener) -> ListenerGuard;
}
```

```typescript
// TypeScript
export interface PropertyBackend {
    propertyValue(name: PropertyName): Value | undefined;
    propertyValues(): Map<PropertyName, Value | undefined>;
    toStateBuffer(): Uint8Array;
    // static method expressed as factory
    // fromStateBuffer(buffer: Uint8Array): PropertyBackend; → via factory pattern
    toOperations(): Operation[] | undefined;
    applyOperations(ops: Operation[]): void;
    listenField(fieldName: PropertyName, listener: Listener): Subscription;
}
```

Mapping quality: **Direct** - every method maps 1:1.

### Enums → Discriminated Unions

```rust
// Rust
pub enum Value {
    I16(i16), I32(i32), I64(i64), F64(f64),
    Bool(bool), String(String),
    EntityId(proto::EntityId),
    Object(Vec<u8>), Binary(Vec<u8>),
    Json(serde_json::Value),
}
```

```typescript
// TypeScript
export type Value =
  | { type: 'i16'; value: number }
  | { type: 'i32'; value: number }
  | { type: 'i64'; value: bigint }
  | { type: 'f64'; value: number }
  | { type: 'bool'; value: boolean }
  | { type: 'string'; value: string }
  | { type: 'entity_id'; value: EntityId }
  | { type: 'object'; value: Uint8Array }
  | { type: 'binary'; value: Uint8Array }
  | { type: 'json'; value: unknown };
```

Mapping quality: **Direct** - every variant maps 1:1.

### Struct → Class

```rust
// Rust
pub struct Entity(Arc<EntityInner>);
pub struct EntityInner {
    pub id: EntityId,
    pub collection: CollectionId,
    state: RwLock<EntityInnerState>,
    pub kind: EntityKind,
    pub broadcast: Broadcast,
}
```

```typescript
// TypeScript
export class Entity {
    readonly id: EntityId;
    readonly collection: CollectionId;
    private state: EntityState;  // No RwLock needed (single-threaded)
    readonly kind: EntityKind;
    readonly broadcast: Broadcast;
}
```

Mapping quality: **Direct** - fields map 1:1 with simplification of concurrency primitives.

## Impl Block → Class Method Mapping

```rust
// Rust
impl Context {
    pub fn begin(&self) -> Transaction { ... }
    pub async fn get<R: View>(&self, id: EntityId) -> Result<R> { ... }
    pub async fn fetch<R: View>(&self, args: MatchArgs) -> Result<Vec<R>> { ... }
    pub fn query<R: View>(&self, args: MatchArgs) -> Result<LiveQuery<R>> { ... }
}
```

```typescript
// TypeScript
export class Context {
    begin(): Transaction { ... }
    async get<R extends View>(viewClass: ViewConstructor<R>, id: EntityId): Promise<R> { ... }
    async fetch<R extends View>(viewClass: ViewConstructor<R>, query: string): Promise<R[]> { ... }
    query<R extends View>(viewClass: ViewConstructor<R>, query: string): LiveQuery<R> { ... }
}
```

Mapping quality: **Near-direct** - generics require passing the class constructor explicitly (TS erases type parameters at runtime).

## Test-Level Mapping

### Integration Tests (1:1 Mappable)

Ankurah's integration tests in `tests/` are highly mappable:

```rust
// Rust: tests/basic.rs
#[tokio::test]
async fn test_create_and_get() {
    let (n1, n2) = setup_nodes().await;
    let ctx = n1.context(auth_data).unwrap();
    let trx = ctx.begin();
    let album = trx.create(&Album { title: "Test".into(), ... }).await.unwrap();
    trx.commit().await.unwrap();
    let fetched = ctx.get::<AlbumView>(album.id()).await.unwrap();
    assert_eq!(fetched.title(), "Test");
}
```

```typescript
// TypeScript: tests/basic.test.ts
test('create and get', async () => {
    const [n1, n2] = await setupNodes();
    const ctx = n1.context(authData);
    const trx = ctx.begin();
    const album = await trx.create(Album, { title: "Test", ... });
    await trx.commit();
    const fetched = await ctx.get(AlbumView, album.id);
    expect(fetched.title).toBe("Test");
});
```

Mapping quality: **Near-direct** - test structure is identical, only syntax differs.

### Unit Tests

Rust unit tests (inside `mod tests` blocks) map to colocated `.test.ts` files or `__tests__/` directories. The test logic is identical.

## Areas Where 1:1 Mapping Breaks Down

### 1. Lifetime/Ownership (Rust-specific)

Rust's `MutableBorrow<'rec, T>` uses lifetimes to prevent use-after-free. TS has no equivalent. We use a dispose pattern or rely on the transaction's alive flag:

```typescript
// No lifetime enforcement, but same semantic:
class MutableHandle<T extends Mutable> {
    private disposed = false;
    get(field: string) { if (this.disposed) throw new Error('Handle expired'); ... }
    [Symbol.dispose]() { this.disposed = true; }  // ES2023 Explicit Resource Management
}
```

### 2. Proc Macros → Code Generation

The derive macro system is fundamentally different. In Rust, `#[derive(Model)]` generates code at compile time inline. In TS, a CLI generates files. However, the **output** of both processes is structurally identical (View class, Mutable class, etc.), so the generated code itself is 1:1 mappable.

### 3. Feature Flags → Build Configuration

Rust uses `#[cfg(feature = "wasm")]` for conditional compilation. TS uses build-time configuration (webpack/metro) or runtime checks. This doesn't affect the core mapping.

### 4. Send + Sync + Arc → JS Reference Semantics

All Rust concurrency machinery (`Arc`, `Mutex`, `RwLock`, `AtomicBool`) disappears in TS. This is a simplification, not a complication. The TS code is simpler.

### 5. Error Handling

Rust uses `Result<T, E>` extensively. TS can use either:
- Exceptions (simpler, more idiomatic TS)
- `Result<T, E>` via a library like `neverthrow` (more faithful to Rust)

**Recommendation**: Use exceptions for the public API (idiomatic TS), but maintain 1:1 error types for agentic comparison.

### 6. AnkQL Parser

Rust uses Pest (PEG parser generator) with a `.pest` grammar file. TS options:
- **PEG.js / Peggy** (direct port of the grammar)
- **Hand-written recursive descent** (more flexible)
- **Port the Pest grammar to Peggy** (closest 1:1 mapping)

The Peggy grammar can be kept nearly identical to the Pest grammar, enabling line-by-line comparison.

## Quantitative Assessment

| Category | Files | 1:1 Mappable | Near-Direct | Requires Adaptation | N/A |
|----------|-------|-------------|-------------|--------------------|----|
| Proto types | ~5 | 5 | 0 | 0 | 0 |
| Core entity/model | ~8 | 8 | 0 | 0 | 0 |
| Core property | ~10 | 8 | 2 | 0 | 0 |
| Core infra | ~12 | 10 | 2 | 0 | 0 |
| Signals | ~6 | 6 | 0 | 0 | 0 |
| AnkQL | ~5 | 3 | 2 | 0 | 0 |
| Storage common | ~2 | 2 | 0 | 0 | 0 |
| Storage SQLite | ~3 | 0 | 3 | 0 | 0 |
| Connectors | ~4 | 2 | 2 | 0 | 0 |
| Derive/Codegen | ~10 | 0 | 0 | 6 | 4 |
| Tests | ~15 | 12 | 3 | 0 | 0 |
| **Total** | **~80** | **56 (70%)** | **14 (18%)** | **6 (7%)** | **4 (5%)** |

**~88% of files are directly or near-directly mappable** to their TypeScript equivalents.

## Implications for Agentic Maintenance

With 88% structural correspondence, an agent can:

1. **Diff-driven updates**: When a Rust file changes, identify the corresponding TS file and apply equivalent changes
2. **Conformance testing**: Run equivalent test suites against both implementations and compare results
3. **Structural validation**: Walk both file trees and verify that every Rust module has a corresponding TS module
4. **API surface comparison**: Compare the public interfaces of corresponding modules

The 7% that requires adaptation (derive macro → codegen) is a one-time design decision, not an ongoing maintenance burden. Once the codegen templates are established, updating them follows the same diff-driven pattern.

## Recommendation for Maximum Maintainability

1. **Mirror the directory structure exactly** where possible
2. **Use the same names** for types, functions, and test cases (with idiomatic casing: snake_case → camelCase)
3. **Maintain a mapping file** that pairs each Rust file with its TS counterpart
4. **Write conformance tests** that verify both implementations produce identical results for the same inputs
5. **Automate drift detection** with a CI job that checks for Rust changes without corresponding TS changes
