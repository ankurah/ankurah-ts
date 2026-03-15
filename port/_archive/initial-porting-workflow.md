# Initial Porting Workflow

## Overview

This document describes the step-by-step process for the initial port of ankurah from Rust to TypeScript. It is designed to be followed by human developers or agentic workflows.

## Prerequisites

### Rust Side (ankurah/)

1. **Complete PR #236** - Property registration (Model/Property as system entities)
2. **Add schema export CLI** - `cargo run --bin ankurah-schema-export` outputs schema.json
3. **Add bincode reference fixtures** - `cargo test -p ankurah-proto generate_reference_fixtures`
4. **Document all public trait/struct signatures** - Machine-readable API surface

### TypeScript Side (ankurah-ts/)

1. **Monorepo setup** - pnpm/npm workspaces or turborepo
2. **TypeScript configuration** - Strict mode, ES2022+ target
3. **Testing framework** - Vitest (fast, ESM-native)
4. **Build tooling** - tsup or tsc for library builds

## Phase 0: Project Scaffolding

### 0.1 Initialize monorepo

```
ankurah-ts/
├── package.json           # Workspace root
├── tsconfig.base.json     # Shared TS config
├── turbo.json             # Build orchestration
├── vitest.workspace.ts    # Test config
├── packages/
│   ├── proto/
│   ├── core/
│   ├── signals/
│   ├── ankql/
│   ├── storage-common/
│   ├── storage-expo-sqlite/
│   ├── storage-better-sqlite3/
│   ├── storage-memory/
│   ├── connector-websocket/
│   ├── connector-local/
│   ├── react-native/
│   └── ankurah/
├── cli/
└── examples/
    └── expo-chat/
```

### 0.2 Set up each package

Each package gets:
- `package.json` with `@ankurah/` scope
- `tsconfig.json` extending base
- `src/index.ts` entry point
- `tests/` directory

### 0.3 Set up CI

- Type checking: `tsc --noEmit`
- Tests: `vitest run`
- Lint: `eslint` (optional)
- Bincode compatibility tests (against Rust fixtures)

## Phase 1: Protocol Types (@ankurah/proto)

**Rust source**: `ankurah/proto/src/`
**Target**: `packages/proto/src/`

### Files to port (in order):

1. **`lib.rs` → `index.ts`**: Module exports, EntityId, CollectionId, RequestId types
2. **`clock.rs` → `clock.ts`**: Clock type (Vec<EventId> → EventId[])
3. **`data.rs` → `data.ts`**: Event, EventId, EntityState, Operation, StateFragment, EventFragment
4. **`update.rs` → `update.ts`**: NodeMessage, NodeMessageBody, SubscriptionUpdateItem, UpdateContent
5. **`sys.rs` → `sys.ts`**: System entity types (Item, BackendKind, ValueType)
6. **`attestation.rs` → `attestation.ts`**: Attested<T>, AttestationSet (stub for Phase 1)

### Porting procedure per file:

1. Read the Rust source file
2. For each `pub struct`, `pub enum`, `pub trait`:
   - Create the corresponding TypeScript type/interface/class
   - Preserve field names (convert to camelCase)
   - Map Rust types to TS types per the type mapping table
3. For each `impl` block:
   - Create corresponding class methods
   - Preserve method names (convert to camelCase)
4. Write unit tests that mirror any existing Rust tests
5. If the type needs bincode serialization, implement encode/decode and test against fixtures

### Validation:
- Every public type in the Rust proto crate must have a corresponding TS export
- Bincode round-trip tests pass against Rust-generated fixtures

## Phase 2: Signals (@ankurah/signals)

**Rust source**: `ankurah/signals/src/`
**Target**: `packages/signals/src/`

### Files to port:

1. **`broadcast.rs` → `broadcast.ts`**: Broadcast, BroadcastId, ListenerGuard
2. **`signal.rs` → `signal.ts`**: Signal interface, Listener, ListenerGuard, Get, Peek, With
3. **`signal/mutable.rs` → `signal/mutable.ts`**: Mut<T> mutable signal
4. **`signal/read.rs` → `signal/read.ts`**: Read<T> read-only signal
5. **`signal/calculated.rs` → `signal/calculated.ts`**: Calculated signals
6. **`signal/memo.rs` → `signal/memo.ts`**: Memoized signals
7. **`observer.rs` → `observer.ts`**: CallbackObserver, CurrentObserver

### Key adaptation:
- Rust uses `Arc<dyn Fn>` for listeners → TS uses plain callback functions
- Rust uses `Weak<>` for preventing cycles → TS uses WeakRef
- Rust ListenerGuard drops on scope exit → TS Subscription needs explicit `unsubscribe()` or `Symbol.dispose`

### Validation:
- Unit tests for each signal type
- Observer dependency tracking tests

## Phase 3: AnkQL Parser (@ankurah/ankql)

**Rust source**: `ankurah/ankql/src/`
**Target**: `packages/ankql/src/`

### Files to port:

1. **`ast.rs` → `ast.ts`**: AST types (Selection, Predicate, Comparison, PathExpr, Literal, etc.)
2. **`grammar.rs` + `ankql.pest` → `parser.ts`**: Parser (port Pest grammar to Peggy or hand-written)
3. **`conversion.rs` → `conversion.ts`**: AST conversions and utilities

### Parser strategy:

**Option A: Port to Peggy** (recommended for 1:1 mapping)
```
# ankql.pest (Rust/Pest)
selection = { predicate ~ (ws ~ order_by)? ~ (ws ~ limit)? }

// ankql.peggy (TypeScript/Peggy)
Selection = predicate:Predicate ws? orderBy:OrderBy? ws? limit:Limit? { ... }
```

**Option B: Hand-written recursive descent** (more control, harder to compare)

### Validation:
- Parse the same set of query strings in both Rust and TS
- Compare AST output for equivalence
- Run all existing ankql parser tests with the TS parser

## Phase 4: Storage Common (@ankurah/storage-common)

**Rust source**: `ankurah/storage/common/src/`
**Target**: `packages/storage-common/src/`

### Files to port:

1. **`lib.rs` → `index.ts`**: StorageEngine interface, StorageCollection interface

This is a small package - just trait definitions.

## Phase 5: Core - Value and Property System (@ankurah/core, part 1)

**Rust source**: `ankurah/core/src/value/`, `ankurah/core/src/property/`
**Target**: `packages/core/src/value/`, `packages/core/src/property/`

### Files to port:

1. **`value/mod.rs` → `value/index.ts`**: Value enum, ValueType enum, Property trait
2. **`property/mod.rs` → `property/index.ts`**: Property interface, PropertyName
3. **`property/backend/mod.rs` → `property/backend/index.ts`**: PropertyBackend interface
4. **`property/backend/lww.rs` → `property/backend/lww.ts`**: LWW backend implementation
5. **`property/backend/yrs.rs` → `property/backend/yjs.ts`**: Yjs backend implementation
6. **`property/value/lww.rs` → `property/value/lww.ts`**: LWW<T> active type wrapper
7. **`property/value/yrs.rs` → `property/value/yjs.ts`**: YjsText active type wrapper
8. **`property/value/entity_ref.rs` → `property/value/entity-ref.ts`**: Ref<T>
9. **`property/value/json.rs` → `property/value/json.ts`**: Json wrapper

### Key adaptation:
- Yrs (Rust) → Yjs (JS): Different API but same semantics
  - `yrs::Doc` → `Y.Doc`
  - `yrs::TextRef` → `Y.Text`
  - `yrs::TransactionMut` → `Y.Transaction` (implicit in Yjs)
- LWW backend: Pure TS implementation of timestamp-based conflict resolution

### Validation:
- LWW: Set/get round-trip, conflict resolution with timestamps
- Yjs: Insert/delete/replace operations, state encoding round-trip
- Yrs/Yjs interop: Load Rust-generated Yrs state buffers in Yjs

## Phase 6: Core - Entity System (@ankurah/core, part 2)

**Rust source**: `ankurah/core/src/`
**Target**: `packages/core/src/`

### Files to port:

1. **`entity.rs` → `entity.ts`**: Entity, EntityInner, EntityKind
2. **`model.rs` → `model.ts`**: Model, View, Mutable interfaces
3. **`schema.rs` → `schema.ts`**: CollectionSchema
4. **`type_resolver.rs` → `type-resolver.ts`**: TypeResolver
5. **`error.rs` → `error.ts`**: Error types

### Validation:
- Entity creation, backend initialization
- Entity state serialization/deserialization
- Entity forking (transaction snapshots)

## Phase 7: Core - Transaction and Context (@ankurah/core, part 3)

### Files to port:

1. **`transaction.rs` → `transaction.ts`**: Transaction class
2. **`context.rs` → `context.ts`**: Context class
3. **`node.rs` → `node.ts`**: Node class
4. **`system.rs` → `system.ts`**: SystemManager
5. **`connector.rs` → `connector.ts`**: PeerSender interface

### Validation:
- Create entity in transaction, commit, verify persistence
- Read entity by ID
- Transaction rollback (drop without commit)
- Transaction alive flag semantics

## Phase 8: Core - Reactor and LiveQuery (@ankurah/core, part 4)

### Files to port:

1. **`reactor.rs` → `reactor.ts`**: Reactor, ReactorSubscription, WatcherSet
2. **`resultset.rs` → `resultset.ts`**: ResultSet
3. **`livequery.rs` → `livequery.ts`**: LiveQuery
4. **`changes.rs` → `changes.ts`**: EntityChange, ItemChange, ChangeSet
5. **`selection/filter.rs` → `selection/filter.ts`**: Predicate evaluation, Filterable interface
6. **`retrieval.rs` → `retrieval.ts`**: LocalRetriever
7. **`node_applier.rs` → `node-applier.ts`**: NodeApplier (remote update application)

### Validation:
- Subscribe to query, verify initial results
- Modify entity, verify subscriber receives update
- Entity add/remove from query results based on predicate changes

## Phase 9: Storage Engines

### In-memory storage (@ankurah/storage-memory)

Simple Map-based implementation for testing. No Rust equivalent (Rust tests use Sled).

### expo-sqlite storage (@ankurah/storage-expo-sqlite)

**Rust reference**: `ankurah/storage/sqlite/src/`
**Target**: `packages/storage-expo-sqlite/src/`

Port the SQLite storage engine, adapting from `rusqlite` API to `expo-sqlite` API:
- Same table schema (`{collection}_state`, `{collection}_event`)
- Same column layout (id TEXT, state BLOB, head TEXT, attestations BLOB)
- Different query API (expo-sqlite's `db.runSync`/`db.getAllSync`)

### better-sqlite3 storage (@ankurah/storage-better-sqlite3)

Same as expo-sqlite but using `better-sqlite3` for Node.js. Useful for testing without a mobile device.

## Phase 10: Connectors

### Local process connector (@ankurah/connector-local)

**Rust source**: `ankurah/connectors/local-process/src/`

In-memory message passing between two Node instances. Critical for integration tests.

### WebSocket client connector (@ankurah/connector-websocket)

**Rust reference**: `ankurah/connectors/websocket-client/src/`

Uses the standard WebSocket API (available in React Native). Connects to a Rust WebSocket server.

## Phase 11: CLI Code Generator (@ankurah/cli)

Build the code generator that reads `schema.json` and outputs TypeScript model wrappers.

### Input: schema.json

```json
{
  "models": {
    "Album": {
      "collection": "album",
      "properties": {
        "title": { "backend": "yrs_text", "value_type": "string", "optional": false },
        "artist": { "backend": "lww", "value_type": "entity_id", "optional": false, "target_model": "Artist" },
        "year": { "backend": "lww", "value_type": "i32", "optional": false }
      }
    }
  }
}
```

### Output: Generated TypeScript files

- `AlbumView.ts`
- `AlbumMutable.ts`
- `AlbumInput.ts`
- `AlbumOps.ts`
- `AlbumRef.ts`
- `AlbumLiveQuery.ts`
- `index.ts` (re-exports)

## Phase 12: Integration Tests

Port the integration tests from `ankurah/tests/`:

1. **Basic CRUD**: Create, read, update, delete entities
2. **Transactions**: Commit, rollback, concurrent transactions
3. **LiveQuery**: Subscribe, receive updates, unsubscribe
4. **Sync**: Two nodes sync via local connector
5. **Cross-language sync**: TS node syncs with Rust node via WebSocket (key validation)

## Phase 13: React Native Integration

### @ankurah/react-native

- `useAnkurahQuery` hook
- `useAnkurahEntity` hook
- `useAnkurahSignal` hook
- Provider component for Node context

### Example app (expo-chat)

Port of the existing ankurah-react-sled-template chat app to Expo Go using ankurah-ts.

## Estimated Effort by Phase

| Phase | Description | Estimated Files | Complexity |
|-------|-------------|----------------|------------|
| 0 | Scaffolding | ~20 config files | Low |
| 1 | Proto types | ~6 | Low |
| 2 | Signals | ~7 | Medium |
| 3 | AnkQL parser | ~3 | High (parser) |
| 4 | Storage common | ~1 | Low |
| 5 | Core: values/properties | ~9 | High (CRDT backends) |
| 6 | Core: entity | ~5 | Medium |
| 7 | Core: transaction/context | ~5 | Medium |
| 8 | Core: reactor/livequery | ~7 | High (reactive) |
| 9 | Storage engines | ~3 | Medium |
| 10 | Connectors | ~2 | Medium |
| 11 | CLI codegen | ~5 | High |
| 12 | Integration tests | ~10 | Medium |
| 13 | React Native | ~5 | Medium |
| **Total** | | **~88 files** | |
