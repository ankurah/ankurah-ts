# Ecosystem Research Findings

## expo-sqlite

expo-sqlite (v16.x) is bundled in Expo Go. No dev build needed.

### API

- `SQLite.openDatabaseSync(name)` / `SQLite.openDatabaseAsync(name)` - returns `SQLiteDatabase`
- Sync methods: `db.runSync(sql, ...params)`, `db.getFirstSync(sql, ...params)`, `db.getAllSync(sql, ...params)`, `db.prepareSync(sql)`, `db.closeSync()`
- Async methods: mirror of sync (`runAsync`, `getFirstAsync`, `getAllAsync`, `prepareAsync`, `closeAsync`)
- `db.execAsync(sql)` - raw SQL execution (DDL, bulk ops)
- React integration: `<SQLiteProvider>`, `useSQLiteContext()` hook
- Tagged template: `db.sql\`SELECT * FROM foo WHERE id = ${id}\``

### WAL mode

Supported and recommended: `db.execAsync('PRAGMA journal_mode = WAL')`

### Limitations

- **No batching API** - use prepared statements instead
- **Concurrent writes abort** with "database is locked" when exclusive transactions are used
- **Large result sets** degrade around 25,000-30,000 rows
- **Sync methods block JS thread** - avoid for heavy queries
- **No SQLCipher** in Expo Go (requires dev build)
- **Web platform** uses different underlying implementation

## CRDT Libraries - Expo Go Compatibility

### Yjs - VIABLE (Pure JS, no WASM)

This is the critical finding: **Yjs is pure JavaScript**. No WASM dependency. It works in Expo Go with the Hermes engine (may need `crypto.getRandomValues` polyfill).

- ~900k weekly npm downloads, production-proven (Notion, Linear, JupyterLab)
- Data types: Y.Map, Y.Array, Y.Text (rich text)
- **`y-expo-sqlite`** persistence adapter exists
- **`y-websocket`** sync provider works with standard WebSocket API
- ankurah's Yrs (Rust) is a port OF Yjs, so state encoding should be compatible (V1 format)

### Automerge - NOT VIABLE for Expo Go

- Core written in Rust, compiled to WASM
- Hermes does not support WebAssembly natively
- `wasm2js` workaround has significant performance penalty
- Planned Turbo Native Module requires dev build

### Loro - NOT VIABLE for Expo Go

- Written in Rust with WASM bindings
- `loro-react-native` exists but requires native code (dev build)
- Not yet production-ready (experimental API/encoding)

### Comparison

| Library | Pure JS? | Expo Go? | Production Ready? |
|---------|----------|----------|-------------------|
| **Yjs** | YES | YES | YES |
| Automerge | No (WASM) | No | Yes |
| Loro | No (WASM) | No | No |

## Expo Go Constraints

- **No custom native modules** unless bundled in Expo Go SDK
- **No WebAssembly** - Hermes engine lacks `WebAssembly` global
- **No JSI-based modules** unless part of Expo SDK
- **No Turbo Modules** unless bundled

### What IS available in Expo Go

- `expo-sqlite` (core SDK)
- `expo-file-system`, `expo-crypto`, `expo-secure-store`
- All pure JavaScript/TypeScript libraries
- Built-in React Native `WebSocket` API (supports text and binary/ArrayBuffer)
- `AsyncStorage`
- Standard networking (fetch, XMLHttpRequest)

## WebSocket in React Native

The built-in `WebSocket` API works in Expo Go with no additional libraries:

```typescript
const ws = new WebSocket('wss://example.com/ws');
ws.binaryType = 'arraybuffer'; // Important for bincode!
ws.onmessage = (e) => { const data = new Uint8Array(e.data); ... };
ws.send(binaryData); // ArrayBuffer supported
```

Auto-reconnection must be implemented manually or via a library like `react-use-websocket`.

## Existing Local-First Solutions for React Native

| Solution | Expo Go? | Model | Notes |
|----------|----------|-------|-------|
| Triplit | Likely yes | Custom resolution, WS sync | expo-sqlite adapter, pure TS |
| RxDB | Yes (basic) | Revision-based (CRDT is paid plugin) | expo-sqlite adapter |
| ElectricSQL | Partial | Server-authoritative (Postgres source of truth) | expo-sqlite adapter |
| PowerSync | Dev only | Server-authoritative | `sql.js` adapter for Expo Go (dev/prototype only) |
| CR-SQLite | No | Table-level CRDTs | Native SQLite extension, needs dev build |
| Jazz.tools | Yes | CoJSON CRDTs | Listed in Expo's official local-first guide |

## Rust Internals Worth Remembering

### Bincode Encoding Rules

ankurah uses default bincode configuration:
- **Integers**: Little-endian, fixed-size
- **String/Vec lengths**: u64 (8 bytes)
- **Enum variant index**: u32 (4 bytes)
- **Option**: u8 tag (0 = None, 1 = Some, then value)
- **BTreeMap**: u64 length, then key-value pairs in sorted order
- **Structs**: Fields in declaration order, no field names

### EventId Computation

`EventId` is a SHA256 hash of `bincode::serialize(entity_id) || bincode::serialize(operations) || bincode::serialize(parent)`. Deterministic - same inputs always produce the same EventId.

### Backend Configuration (RON files)

The derive macro resolves field types via RON config files:
- `core/src/property/value/lww.ron` - LWW backend config
- `core/src/property/value/yrs.ron` - Yrs backend config

Each config defines:
- `backend_name` (e.g., "LWWBackend")
- `type_pattern` (regex to match, e.g., `"LWW(?:<(.+)>)?"`)
- `accepts` (what field types this backend handles, e.g., `".*"` for LWW, `"^String$"` for YrsString)
- `methods` (set/get for LWW; insert/delete/replace for YrsString)
- `provided_wrapper_types` (built-in monomorphized wrappers: LWWString, LWWi32, etc.)

Resolution order: YrsString checked before LWW (higher precedence). Explicit `#[active_type(X)]` overrides inference.

### ankurah Wire Protocol

Uses serde + bincode. NOT protobuf. Key types:
- `NodeMessage` → `NodeRequest` / `NodeResponse` / `NodeUpdate` / `NodeUpdateAck`
- `NodeRequestBody`: `CommitTransaction`, `Get`, `GetEvents`, `Fetch`, `SubscribeQuery`
- `UpdateContent`: `EventOnly(Vec<EventFragment>)` or `StateAndEvent(StateFragment, Vec<EventFragment>)`
- `Presence`: Initial peer announcement with node_id, durable flag, system root
