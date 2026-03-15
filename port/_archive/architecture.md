# ankurah-ts Architecture

## Overview

ankurah-ts is a fully faithful TypeScript port of [ankurah](https://github.com/ankurah/ankurah), targeting React Native / Expo Go as its primary runtime. It preserves the same architectural principles: entity-backed CRDT storage, typed model wrappers over untyped entities, reactive subscriptions, event-sourced sync, and pluggable storage/connectors.

## Why a TS Port (Not WASM/UniFFI)

Expo Go cannot load WASM modules or native modules. The existing WASM and UniFFI binding strategies require either browser WASM support or a native build step. A pure TypeScript implementation is the only path to Expo Go compatibility while maintaining the full ankurah programming model.

## Key Architectural Insight: Yjs

ankurah's Rust CRDT text backend uses **Yrs** (the Rust port of Yjs). The TS port can use **Yjs directly** - the original JavaScript implementation. This means:

- CRDT text operations (insert, delete, replace) use the same underlying algorithm
- Yjs documents are wire-compatible with Yrs documents (same encoding format)
- No WASM required - Yjs is pure JavaScript
- Battle-tested in production (used by Notion, Linear, etc.)

## Architecture Layers

```
┌─────────────────────────────────────────────────┐
│  Generated Model Wrappers (ModelView, Mutable)  │  ← CLI-generated from schema registry
├─────────────────────────────────────────────────┤
│  ankurah-ts Core                                │
│  ├── Entity (untyped, Arc-equivalent)           │
│  ├── PropertyBackend (LWW, Yjs)                 │
│  ├── Transaction                                │
│  ├── Node / Context                             │
│  ├── Reactor (subscriptions)                    │
│  ├── Signals (reactive primitives)              │
│  └── AnkQL (query parser/evaluator)             │
├─────────────────────────────────────────────────┤
│  Storage Engines                                │
│  ├── expo-sqlite (primary target)               │
│  ├── better-sqlite3 (Node.js testing/server)    │
│  └── in-memory (unit tests)                     │
├─────────────────────────────────────────────────┤
│  Connectors                                     │
│  ├── WebSocket Client (Expo-compatible)         │
│  ├── Local Process (in-memory testing)          │
│  └── (future: WebSocket Server for Node.js)     │
└─────────────────────────────────────────────────┘
```

## Module Mapping (Rust → TypeScript)

| Rust Crate | TS Package | Notes |
|------------|------------|-------|
| `ankurah-proto` | `@ankurah/proto` | Protocol types (EntityId, Event, State, Clock, etc.) |
| `ankurah-core` | `@ankurah/core` | Entity, Node, Context, Transaction, Reactor, backends |
| `ankurah-derive` | `@ankurah/cli` | CLI tool that generates typed wrappers from schema registry |
| `ankurah-signals` | `@ankurah/signals` | Reactive primitives (Signal, Broadcast, Observer) |
| `ankql` | `@ankurah/ankql` | Query parser and evaluator |
| `ankurah-storage-sqlite` | `@ankurah/storage-expo-sqlite` | expo-sqlite storage engine |
| `ankurah-storage-sqlite` | `@ankurah/storage-better-sqlite3` | Node.js sqlite storage engine |
| `ankurah-connectors-websocket-client` | `@ankurah/connector-websocket` | WebSocket client connector |
| `ankurah-connectors-local-process` | `@ankurah/connector-local` | In-process test connector |
| `ankurah` (facade) | `@ankurah/ankurah` | Re-exports, convenience API |
| (new) | `@ankurah/react-native` | React Native hooks, Expo integration |

## Core Type Correspondences

### Entity System

| Rust | TypeScript | Notes |
|------|-----------|-------|
| `Entity` (Arc<EntityInner>) | `Entity` (class, shared ref via JS reference semantics) | JS objects are reference types by default |
| `EntityId` (ULID) | `EntityId` (branded string, ULID) | Use `ulid` package |
| `CollectionId` | `CollectionId` (branded string) | |
| `Clock` (Vec<EventId>) | `Clock` (EventId[]) | |
| `Value` enum | `Value` (discriminated union) | |
| `PropertyBackend` trait | `PropertyBackend` (interface/abstract class) | |
| `LWWBackend` | `LWWBackend` | Pure TS implementation |
| `YrsBackend` | `YjsBackend` | Uses Yjs directly |

### Model System

| Rust | TypeScript | Notes |
|------|-----------|-------|
| `Model` trait | Generated class per model | From schema registry |
| `View` trait | `{Name}View` class | Read-only, typed getters |
| `Mutable` trait | `{Name}Mutable` class | CRDT wrapper accessors |
| `MutableBorrow<'rec, T>` | `MutableHandle<T>` | No lifetime in TS; use dispose pattern |
| `Ref<T>` | `Ref<T>` | Typed entity reference |

### Reactive System

| Rust | TypeScript | Notes |
|------|-----------|-------|
| `Signal` trait | `Signal` interface | |
| `Broadcast` | `Broadcast` class | EventEmitter-like |
| `Listener` (Arc<dyn Fn>) | `Listener` (callback function) | |
| `ListenerGuard` | `Subscription` (with `unsubscribe()`) | |
| `LiveQuery<T>` | `LiveQuery<T>` | |
| `ResultSet<T>` | `ResultSet<T>` | |

### Storage

| Rust | TypeScript | Notes |
|------|-----------|-------|
| `StorageEngine` trait | `StorageEngine` interface | |
| `StorageCollection` trait | `StorageCollection` interface | |
| `EntityState` | `EntityState` | |
| `Attested<T>` | `Attested<T>` | |

### Networking

| Rust | TypeScript | Notes |
|------|-----------|-------|
| `PeerSender` trait | `PeerSender` interface | |
| `NodeMessage` | `NodeMessage` | |
| `NodeMessageBody` | `NodeMessageBody` (discriminated union) | |

## Serialization Strategy

The Rust implementation uses **bincode** for binary serialization of state buffers and events. The TS port needs a compatible serialization format:

- **State buffers**: Use the same binary format as Rust (bincode for LWW state, Yjs update encoding for Yjs state)
- **Wire protocol**: Use the same bincode-encoded `NodeMessage` format for interop with Rust nodes
- **Storage**: Can use a TS-native format (JSON or msgpack) since storage is local-only, but must be convertible to/from the wire format

**Critical for interop**: If TS nodes need to sync with Rust nodes (they will), the wire format must be byte-compatible. This means implementing bincode encoding/decoding in TS for the protocol types, or defining a shared format (e.g., protobuf or msgpack with a shared schema).

### Recommended approach: Binary interop via shared encoding

1. Define `.proto` or similar IDL for all wire types
2. Generate both Rust and TS serializers from the same schema
3. This ensures wire compatibility without manual bincode implementation

**Alternative**: Use JSON as the wire format between TS and Rust nodes (simpler, but larger payloads). The Rust `NodeMessage` types already derive `Serialize`/`Deserialize` with serde, so JSON encoding is trivially available.

## Concurrency Model

Rust uses `Arc<Mutex<>>`, `RwLock`, `AtomicBool` for concurrency. TypeScript is single-threaded (event loop), which simplifies things significantly:

- No `Mutex`/`RwLock` needed - JS is single-threaded
- `Arc` equivalent: JS reference semantics (objects are always shared by reference)
- `AtomicBool` → simple boolean field
- `async/await` maps directly
- `tokio::sync::Mutex` → not needed (no concurrent access)
- `BroadcastChannel` → EventEmitter or custom Broadcast class

The single-threaded model means we can often simplify the internal locking structure while preserving the external API semantics.

## React Native Integration

### Hooks

```typescript
// Equivalent to ankurah-signals React integration
function useAnkurahQuery<T extends View>(
  ctx: Context,
  model: ModelClass<T>,
  query: string
): { items: T[], loading: boolean, error?: Error }

function useAnkurahEntity<T extends View>(
  ctx: Context,
  model: ModelClass<T>,
  id: EntityId
): { entity: T | null, loading: boolean, error?: Error }

function useAnkurahSignal<T>(signal: Signal<T>): T
```

### Expo SQLite Integration

```typescript
import * as SQLite from 'expo-sqlite';

class ExpoSQLiteStorageEngine implements StorageEngine {
  private db: SQLite.SQLiteDatabase;

  constructor(dbName: string) {
    this.db = SQLite.openDatabaseSync(dbName);
    // Enable WAL mode for better concurrent read performance
    this.db.runSync('PRAGMA journal_mode = WAL');
  }

  async collection(id: CollectionId): Promise<StorageCollection> {
    // Create tables if needed, return collection wrapper
  }
}
```

## Package Structure

```
ankurah-ts/
├── packages/
│   ├── proto/              # @ankurah/proto - Protocol types
│   ├── core/               # @ankurah/core - Entity, Node, Context, etc.
│   ├── signals/            # @ankurah/signals - Reactive primitives
│   ├── ankql/              # @ankurah/ankql - Query parser
│   ├── storage-common/     # @ankurah/storage-common - Storage traits
│   ├── storage-expo-sqlite/  # @ankurah/storage-expo-sqlite
│   ├── storage-better-sqlite3/ # @ankurah/storage-better-sqlite3
│   ├── storage-memory/     # @ankurah/storage-memory - For tests
│   ├── connector-websocket/  # @ankurah/connector-websocket
│   ├── connector-local/    # @ankurah/connector-local
│   ├── react-native/       # @ankurah/react-native - Hooks
│   └── ankurah/            # @ankurah/ankurah - Facade re-exports
├── cli/                    # @ankurah/cli - Code generator
├── specs/                  # This directory
└── examples/
    └── expo-chat/          # Example Expo Go chat app
```

## De-scoped Items (Phase 1)

### In Scope
- Full Entity/Model/View/Mutable system
- LWW and Yjs CRDT backends
- Transaction system with commit/rollback
- Reactive signals and LiveQuery
- AnkQL parser and evaluator
- expo-sqlite storage engine
- better-sqlite3 storage engine (Node.js testing)
- In-memory storage engine (unit tests)
- WebSocket client connector (Expo-compatible)
- Local process connector (testing)
- CLI code generator for typed wrappers
- React Native hooks

### De-scoped (Future Phases)
- WebSocket server connector (use Rust server)
- PostgreSQL storage engine
- IndexedDB storage engine
- Sled storage engine
- PN Counter CRDT backend
- Policy agent (authorization) - start with PermissiveAgent only
- Lineage attestation / cryptographic verification
- WASM build target
