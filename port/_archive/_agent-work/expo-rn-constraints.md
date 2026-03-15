# Expo Go & React Native Constraints for ankurah-ts

**Date**: 2026-02-10
**Purpose**: Audit of runtime constraints, API availability, and tooling considerations that affect the ankurah-ts project.

---

## 1. expo-sqlite Current API

**Package**: `expo-sqlite` (v16.x in SDK 53, v15.x in SDK 52)
**Bundled SQLite version**: 3.49.1 in SDK 53 (updated to 3.50.3 in a patch), 3.45.3 in SDK 52
**Included in Expo Go**: Yes (core SDK module)

### Database Opening

```typescript
import * as SQLite from 'expo-sqlite';

// Async (recommended for most use cases)
const db = await SQLite.openDatabaseAsync('mydb.db');

// Sync (blocks JS thread)
const db = SQLite.openDatabaseSync('mydb.db');
```

### Convenience Methods on SQLiteDatabase

| Method | Sync Variant | Description |
|--------|-------------|-------------|
| `runAsync(sql, ...params)` | `runSync` | Execute INSERT/UPDATE/DELETE, returns `{ changes, lastInsertRowId }` |
| `getFirstAsync(sql, ...params)` | `getFirstSync` | Returns first row or null |
| `getAllAsync(sql, ...params)` | `getAllSync` | Returns all rows as array |
| `getEachAsync(sql, ...params)` | `getEachSync` | Returns async iterator (memory-efficient) |
| `execAsync(sql)` | `execSync` | Raw SQL (DDL, PRAGMA, multi-statement). No parameter binding. |
| `prepareAsync(sql)` | `prepareSync` | Returns `SQLiteStatement` for repeated execution |

### Prepared Statements

```typescript
const stmt = await db.prepareAsync('INSERT INTO users (name, age) VALUES ($name, $age)');
try {
  await stmt.executeAsync({ $name: 'Alice', $age: 30 });
  await stmt.executeAsync({ $name: 'Bob', $age: 25 });
} finally {
  await stmt.finalizeAsync(); // MUST finalize when done
}
```

- `executeAsync()` / `executeSync()` on the statement object
- `finalizeAsync()` / `finalizeSync()` to release resources
- Orphaned statements are auto-finalized on db close, but explicit finalization is best practice

### Transaction Support

Three transaction APIs exist:

1. **`withTransactionAsync(callback)`** -- Wraps callback in BEGIN/COMMIT. **Caveat**: Due to async/await, ANY query running while the transaction is active (even outside the callback scope) will be included in the transaction. This can cause unexpected behavior.

2. **`withExclusiveTransactionAsync(callback)`** -- Fixes the scoping issue. Only queries within the callback participate in the transaction. **Recommended** for correctness.

3. **`withTransactionSync(callback)`** -- Synchronous transaction. Blocks JS thread. Use only for lightweight operations.

### WAL Mode

Supported and recommended:
```typescript
db.execSync('PRAGMA journal_mode = WAL');
```

### Tagged Template Literals

```typescript
const row = await db.getFirstAsync(
  SQLite.SQL`SELECT * FROM users WHERE id = ${userId}`
);
```

### React Integration

```tsx
import { SQLiteProvider, useSQLiteContext } from 'expo-sqlite';

// In component tree
<SQLiteProvider databaseName="mydb.db" onInit={migrateDb}>
  <App />
</SQLiteProvider>

// In child components
const db = useSQLiteContext();
```

### Limitations

- **No batching API** -- use prepared statements for bulk operations
- **Concurrent writes** can abort with "database is locked" on exclusive transactions
- **Large result sets** degrade around 25,000-30,000 rows
- **Sync methods block JS thread** -- avoid for heavy queries
- **No SQLCipher in Expo Go** (requires development build)
- **Web platform** uses wa-sqlite (WASM), requires special headers (COOP/COEP for SharedArrayBuffer)

### Impact on ankurah-ts

The spec's `ExpoSQLiteStorageEngine` example using `openDatabaseSync` and `runSync` is correct. The API supports both sync and async patterns. For ankurah-ts, the recommended approach:
- Use `openDatabaseSync` for initialization (WAL pragma, table creation)
- Use async methods for queries in reactive flows
- Use `withExclusiveTransactionAsync` (not `withTransactionAsync`) for transaction safety
- Use prepared statements for repeated operations (entity upserts)

**Sources**:
- [SQLite - Expo Documentation](https://docs.expo.dev/versions/latest/sdk/sqlite/)
- [expo-sqlite on GitHub](https://github.com/expo/expo/tree/main/packages/expo-sqlite)
- [Expo SDK 53 Changelog](https://expo.dev/changelog/sdk-53)

---

## 2. Expo Go Limitations

### Confirmed Constraints

1. **No custom native modules**: Expo Go includes a fixed set of native modules (everything in `bundledNativeModules.json`). You cannot add native modules not already in the Expo SDK. If you try to use a library with native code not bundled in Expo Go, the JS code will error because the native code does not exist.

2. **No WebAssembly**: Hermes does not implement the `WebAssembly` global. There is no WASM support in Expo Go. Workarounds like `wasm2js` exist but have significant performance penalties.

3. **No JSI-based third-party modules**: JSI modules not part of the Expo SDK cannot be loaded in Expo Go.

4. **No Turbo Modules**: Custom TurboModules require a development build.

### What IS Available in Expo Go

- `expo-sqlite` (core SDK)
- `expo-crypto` (provides `crypto.getRandomValues` polyfill and digest functions)
- `expo-file-system`, `expo-secure-store`, `expo-constants`
- All **pure JavaScript/TypeScript** libraries (this is why Yjs works)
- Built-in React Native `WebSocket` API (text and binary)
- `fetch`, `XMLHttpRequest`
- React Native `AsyncStorage`

### SDK 53 Changes (Current)

- **New Architecture enabled by default** for all projects (Fabric, TurboModules, JSI)
- React Native 0.76+ under the hood
- Push notifications removed from Expo Go on Android
- SQLite updated to 3.49.1+

### Development Builds as Escape Hatch

When Expo Go constraints are too limiting, development builds (via EAS Build or local builds) allow adding any native module. ankurah-ts targets Expo Go for maximum developer accessibility, but the architecture should not preclude development builds.

### Impact on ankurah-ts

The Expo Go constraint is the reason this project exists as a pure TypeScript port rather than using WASM bindings to Rust ankurah. The constraint is firm and unchanged in 2025-2026. All dependencies must be pure JS or part of the Expo SDK.

**Sources**:
- [Expo Go vs Development Builds](https://expo.dev/blog/expo-go-vs-development-builds)
- [Development Builds Introduction](https://docs.expo.dev/develop/development-builds/introduction/)
- [WASM support in Hermes - GitHub Issue #429](https://github.com/facebook/hermes/issues/429)

---

## 3. better-sqlite3 for Node Testing

### Status: Still the Right Choice

**better-sqlite3** (v11.x, 2025) remains the fastest and most widely used synchronous SQLite library for Node.js. ~1.4M weekly npm downloads.

### API Summary

```typescript
import Database from 'better-sqlite3';

const db = new Database('mydb.db');
db.pragma('journal_mode = WAL');

// Prepare + run
const stmt = db.prepare('INSERT INTO users (name, age) VALUES (?, ?)');
const info = stmt.run('Alice', 30); // { changes: 1, lastInsertRowId: 1 }

// Query
const row = db.prepare('SELECT * FROM users WHERE id = ?').get(1);
const rows = db.prepare('SELECT * FROM users').all();

// Transaction
const insertMany = db.transaction((users) => {
  for (const u of users) stmt.run(u.name, u.age);
});
insertMany([{ name: 'Alice', age: 30 }, { name: 'Bob', age: 25 }]);

// Raw exec
db.exec('CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)');
```

### API Comparison: better-sqlite3 vs expo-sqlite

| Operation | better-sqlite3 | expo-sqlite (sync) | expo-sqlite (async) |
|-----------|----------------|--------------------|--------------------|
| Open | `new Database(path)` | `SQLite.openDatabaseSync(name)` | `SQLite.openDatabaseAsync(name)` |
| Execute DDL | `db.exec(sql)` | `db.execSync(sql)` | `db.execAsync(sql)` |
| Run (write) | `stmt.run(...params)` | `db.runSync(sql, ...params)` | `db.runAsync(sql, ...params)` |
| Get one row | `stmt.get(...params)` | `db.getFirstSync(sql, ...params)` | `db.getFirstAsync(sql, ...params)` |
| Get all rows | `stmt.all(...params)` | `db.getAllSync(sql, ...params)` | `db.getAllAsync(sql, ...params)` |
| Prepare | `db.prepare(sql)` | `db.prepareSync(sql)` | `db.prepareAsync(sql)` |
| Transaction | `db.transaction(fn)` | `db.withTransactionSync(fn)` | `db.withExclusiveTransactionAsync(fn)` |
| Finalize stmt | (automatic via GC) | `stmt.finalizeSync()` | `stmt.finalizeAsync()` |

### Alternatives Considered

| Library | Status | Notes |
|---------|--------|-------|
| `node:sqlite` (built-in) | **Experimental** (still requires `--experimental-sqlite` flag as of Node.js 23.x) | Synchronous API similar to better-sqlite3, but not stable. |
| `sqlite3` (node-sqlite3) | Stable | Async/callback-based. Older API, less ergonomic. |
| `sql.js` | Stable | Pure JS (Emscripten WASM). No native dependency. Slower than better-sqlite3. |

### Recommendation

**better-sqlite3 remains the best choice** for the Node.js storage engine:
- Synchronous API is an advantage for testing (deterministic, no race conditions)
- API maps well enough to expo-sqlite that a shared `StorageEngine` interface is feasible
- The main divergence is that better-sqlite3 uses prepared statement objects (`db.prepare().run()`) while expo-sqlite has convenience methods directly on the database (`db.runSync()`). The shared interface should abstract over this.

**The `node:sqlite` built-in module** is worth monitoring. When it reaches stable status, it could replace better-sqlite3 and remove the native dependency. Its API is very similar to better-sqlite3 (synchronous, `DatabaseSync` class).

**Sources**:
- [better-sqlite3 GitHub](https://github.com/WiseLibs/better-sqlite3)
- [better-sqlite3 API docs](https://github.com/WiseLibs/better-sqlite3/blob/master/docs/api.md)
- [Node.js SQLite Documentation](https://nodejs.org/api/sqlite.html)
- [Bridging the Gap between Expo SQLite and Node.js](https://www.amarjanica.com/bridging-the-gap-between-expo-sqlite-and-node-js/)

---

## 4. React Native New Architecture

### Current State (2025-2026)

The New Architecture is now the **default** in React Native 0.76+ and Expo SDK 53:

- **JSI (JavaScript Interface)**: Replaces the old async bridge with synchronous, direct C++ calls from JavaScript. This is how expo-sqlite achieves synchronous methods.
- **Fabric**: New rendering system with a shared C++ core across platforms.
- **TurboModules**: Lazy-loaded, type-safe native modules using JSI for direct JS-to-native communication.
- **Bridgeless Mode**: The old bridge is removed entirely.

### Expo Go Support

Expo SDK 53 enables the New Architecture by default for all projects, including Expo Go. This means:
- expo-sqlite's synchronous methods work via JSI (no bridge overhead)
- Performance is improved for native module calls
- The architecture is stable and production-ready

### Hermes V1

React Native 0.82 introduces opt-in support for **Hermes V1**, the next evolution of the Hermes engine with improved compiler and VM performance. This is still opt-in, not the default.

### Impact on ankurah-ts

The New Architecture is **beneficial** for ankurah-ts:
- expo-sqlite's sync methods are fast (JSI-backed, no bridge serialization)
- No action needed -- Expo Go handles this transparently
- The project does not need to create any native modules, so TurboModules/Fabric are not directly relevant to development, only to runtime performance

**Sources**:
- [About the New Architecture - React Native](https://reactnative.dev/architecture/landing-page)
- [React Native's New Architecture - Expo Documentation](https://docs.expo.dev/guides/new-architecture/)
- [React Native 0.82 Release Blog](https://reactnative.dev/blog/2025/10/08/react-native-0.82)

---

## 5. Yjs in React Native

### Viability: Confirmed

Yjs is **pure JavaScript** with no WASM dependency. It works in Expo Go.

### Known Issues and Solutions

#### crypto.getRandomValues

**Problem**: Yjs (and its dependency `lib0`) uses `crypto.getRandomValues()` for generating client IDs and other random values. Hermes does NOT provide `crypto.getRandomValues` natively.

**Solution**: Use `expo-crypto` which is bundled in Expo Go and provides this polyfill:
```typescript
// At the top of your entry file, before any Yjs imports
import 'expo-crypto'; // Polyfills crypto.getRandomValues globally
```

Alternative: `react-native-get-random-values` (also works in Expo Go as it's a small, commonly used polyfill).

#### TextEncoder / TextDecoder

**Problem**: Yjs uses `TextEncoder` and `TextDecoder` for string encoding/decoding.

**Status in Hermes**:
- `TextEncoder`: Available natively in Hermes (React Native 0.74+)
- `TextDecoder`: **NOT fully available**. Not spec-compliant on native platforms; only UTF-8 is supported when available, and some versions of Hermes lack it entirely.

**Solution**: Multiple options:
1. `expo-encoding` (Expo SDK module, available in Expo Go) -- provides standard TextEncoder and TextDecoder globally
2. `@bacons/text-decoder` -- lightweight TextDecoder polyfill specifically for Expo 51+ apps
3. `fast-text-encoding` -- UTF-8-only polyfill, very small

Since Yjs only uses UTF-8 encoding, any of these polyfills work.

#### Community Reports

The [Yjs Community Forum](https://discuss.yjs.dev/t/has-anyone-installed-yjs-for-react-native/2137) has reports of users successfully running Yjs in React Native, with the main hurdles being the crypto and TextDecoder polyfills described above.

A dedicated `y-expo-sqlite` persistence adapter exists for persisting Yjs documents to expo-sqlite.

### Bundle Size

Yjs core (`yjs` package) is approximately:
- **Minified**: ~87 KB
- **Gzipped**: ~27 KB

This is well within acceptable limits for a React Native app. For context, React Native's JS bundle for a basic app is typically 1-3 MB. Yjs adds roughly 1-3% to that.

### Impact on ankurah-ts

Yjs is confirmed viable. The polyfill requirements are:
1. `crypto.getRandomValues` -- use `expo-crypto` (already in Expo Go)
2. `TextDecoder` -- use `@bacons/text-decoder` or `fast-text-encoding`

These must be imported at the app entry point before Yjs is loaded.

**Sources**:
- [Yjs Community: React Native Discussion](https://discuss.yjs.dev/t/has-anyone-installed-yjs-for-react-native/2137)
- [Expo Crypto Documentation](https://docs.expo.dev/versions/latest/sdk/crypto/)
- [Expo Encoding Documentation](https://docs.expo.dev/versions/v52.0.0/sdk/encoding/)
- [@bacons/text-decoder](https://github.com/EvanBacon/text-decoder)
- [react-native-get-random-values](https://github.com/LinusU/react-native-get-random-values)

---

## 6. WebSocket Client

### API Availability

React Native provides a **built-in WebSocket API** that follows the browser standard:

```typescript
const ws = new WebSocket('wss://example.com/sync');
ws.binaryType = 'arraybuffer'; // Required for binary protocols like bincode

ws.onopen = () => {
  console.log('Connected');
  ws.send(binaryData); // ArrayBuffer, Uint8Array, Blob supported
};

ws.onmessage = (event) => {
  if (event.data instanceof ArrayBuffer) {
    const bytes = new Uint8Array(event.data);
    // Process bincode message
  }
};

ws.onclose = (event) => {
  console.log('Disconnected', event.code, event.reason);
};

ws.onerror = (error) => {
  console.error('WebSocket error', error);
};
```

### Binary Data Support

- **Sending**: ArrayBuffer, Uint8Array, and other TypedArrays are supported
- **Receiving**: Set `ws.binaryType = 'arraybuffer'` to receive binary frames as ArrayBuffer
- **Known issue**: Some older versions had issues with receiving empty ArrayBuffers from the server. This is generally resolved in recent React Native versions.

### Limitations

- **No `ws` (Node.js) options**: Options like `agent`, `perMessageDeflate`, `pfx`, `key`, `passphrase`, `cert`, `ca`, `ciphers`, `rejectUnauthorized` are not supported. These are Node.js-specific and not part of the browser WebSocket spec.
- **No compression**: Per-message deflate is not available.
- **Cookie-based auth**: Unstable; prefer token-based auth (query params or initial handshake message).
- **Background handling**: When the app is backgrounded, WebSocket connections may be dropped by the OS. Reconnection logic is required.
- **No auto-reconnection**: Must be implemented manually.

### Impact on ankurah-ts

The WebSocket API is **fully adequate** for ankurah-ts:
- Binary data support means bincode-encoded `NodeMessage` can be sent/received directly
- The API matches the browser WebSocket API, so the connector can work in both React Native and web
- Auto-reconnection with exponential backoff must be implemented in `@ankurah/connector-websocket`
- `ws.binaryType = 'arraybuffer'` must be set for bincode support

**Sources**:
- [Networking - React Native](https://reactnative.dev/docs/network)
- [WebSocket React Native Guide](https://www.videosdk.live/developer-hub/websocket/websocket-react-native)

---

## 7. Monorepo Tooling

### Current Best Practice (2025-2026)

The dominant pattern for TypeScript monorepos targeting React Native is:

**pnpm workspaces + Turborepo**

- **pnpm**: Faster than npm/yarn, disk-efficient (content-addressable store), first-class workspace support
- **Turborepo**: Task orchestration, build caching, dependency-aware task scheduling
- **TypeScript project references**: Cross-package type checking

### Expo Monorepo Support

Since **Expo SDK 52**, Metro has automatic monorepo support for pnpm, npm, yarn, and bun. The `expo/metro-config` package detects the monorepo structure and configures itself.

### pnpm + React Native Gotchas

1. **Symlinks**: pnpm uses symlinks by default. Metro requires configuration:
   ```javascript
   // metro.config.js
   const { getDefaultConfig } = require('expo/metro-config');
   const config = getDefaultConfig(__dirname);
   // Symlinks are handled automatically by expo/metro-config since SDK 52
   module.exports = config;
   ```

2. **Hoisting**: Some React Native libraries may fail with pnpm's default isolated installation. Fix by adding to `.npmrc`:
   ```
   node-linker=hoisted
   ```
   This is often needed but check first -- since SDK 52, many issues are auto-resolved.

3. **Duplicate dependencies**: Duplicate React or React Native versions in a monorepo cause runtime errors. Use pnpm's `peerDependencyRules` or `overrides` to enforce single versions.

### Reference Example

[byCedric/expo-monorepo-example](https://github.com/byCedric/expo-monorepo-example) is the canonical reference for pnpm + Expo monorepos, maintained by an Expo team member.

### Recommended Setup for ankurah-ts

```
ankurah-ts/
  pnpm-workspace.yaml
  turbo.json
  tsconfig.base.json
  .npmrc                    # node-linker=hoisted (if needed)
  packages/
    proto/                  # @ankurah/proto
    core/                   # @ankurah/core
    signals/                # @ankurah/signals
    ankql/                  # @ankurah/ankql
    storage-common/         # @ankurah/storage-common
    storage-expo-sqlite/    # @ankurah/storage-expo-sqlite
    storage-better-sqlite3/ # @ankurah/storage-better-sqlite3
    storage-memory/         # @ankurah/storage-memory
    connector-websocket/    # @ankurah/connector-websocket
    connector-local/        # @ankurah/connector-local
    react-native/           # @ankurah/react-native
    ankurah/                # @ankurah/ankurah (facade)
  examples/
    expo-chat/              # Example app
```

```yaml
# pnpm-workspace.yaml
packages:
  - 'packages/*'
  - 'examples/*'
```

### Alternatives

| Tool | Notes |
|------|-------|
| **Nx** | More opinionated, heavier. Better for large enterprise teams. Overkill for ankurah-ts. |
| **Lerna** | Legacy. Now maintained by Nx. Most projects have migrated to pnpm workspaces + Turborepo. |
| **Bun workspaces** | Viable but less mature ecosystem. Bun's bundler does not handle React Native well yet. |
| **yarn workspaces** | Still works but pnpm is generally preferred in 2025 for performance and disk usage. |

**Sources**:
- [Work with Monorepos - Expo Documentation](https://docs.expo.dev/guides/monorepos/)
- [byCedric/expo-monorepo-example](https://github.com/byCedric/expo-monorepo-example)
- [2025 Monorepo That Actually Scales: Turborepo + PNPM](https://medium.com/@TheblogStacker/2025-monorepo-that-actually-scales-turborepo-pnpm-for-next-js-ab4492fbde2a)
- [Complete Monorepo Guide: pnpm + Workspace + Changesets](https://jsdev.space/complete-monorepo-guide/)

---

## 8. Bundle Size Concerns

### Yjs Size

| Metric | Size |
|--------|------|
| Minified | ~87 KB |
| Gzipped | ~27 KB |

### Context

- A minimal React Native app JS bundle: ~1-3 MB
- React Native core runtime: ~600-800 KB
- Adding Yjs adds ~3% to a typical bundle

### Other ankurah-ts Dependencies (Estimated)

| Dependency | Approximate Gzipped Size | Notes |
|------------|-------------------------|-------|
| `yjs` | ~27 KB | Core CRDT library |
| `lib0` | ~12 KB | Yjs utility dependency |
| `ulid` | ~1 KB | Entity ID generation |
| Bincode codec (custom) | ~5-10 KB | Handwritten, small |
| AnkQL parser (custom) | ~5-15 KB | Depends on parser approach |
| Total ankurah-ts | ~60-75 KB gzipped (est.) | |

### Verdict

**No concerns**. The total estimated bundle addition of ~60-75 KB gzipped is modest for a React Native app. For reference:
- `@react-navigation` adds ~50-100 KB
- `react-native-reanimated` adds ~100+ KB
- Firebase SDK adds 200-400 KB

React Native 0.79+ includes improvements to Metro bundling and tree-shaking that further reduce final bundle sizes.

### Optimization Tips

- Use `lib0/encoding` directly if only specific utilities are needed (Yjs already does this internally)
- The AnkQL parser should be written to minimize size (avoid large parser generator runtimes if possible)
- Consider lazy loading Yjs if it's not needed immediately at app startup

**Sources**:
- [yjs on Bundlephobia](https://bundlephobia.com/package/yjs)

---

## 9. TextEncoder / TextDecoder Availability

### Hermes Support Status

| API | Hermes Status | Notes |
|-----|---------------|-------|
| `TextEncoder` | **Available natively** since React Native 0.74 / Hermes | UTF-8 only, which is all that's needed |
| `TextDecoder` | **Partial / not spec-compliant** | UTF-8 only when available; some Hermes versions lack it entirely |

### What Needs TextEncoder/TextDecoder in ankurah-ts

1. **Yjs / lib0**: Uses both for internal string encoding in CRDT documents
2. **Bincode codec**: Needs to encode/decode UTF-8 strings in serialized data
3. **WebSocket message handling**: May need to encode/decode string payloads

### Recommended Polyfill Strategy

Since `TextEncoder` is available natively but `TextDecoder` may not be:

```typescript
// In app entry point (index.js or App.tsx), BEFORE any other imports:
import 'expo-crypto';  // Polyfills crypto.getRandomValues

// TextDecoder polyfill (only if needed)
if (typeof globalThis.TextDecoder === 'undefined') {
  const { TextDecoder } = require('@bacons/text-decoder');
  globalThis.TextDecoder = TextDecoder;
}
```

### Polyfill Options

| Package | Size | Notes |
|---------|------|-------|
| `@bacons/text-decoder` | ~2 KB | Expo-specific, by Evan Bacon (Expo team). UTF-8 only. |
| `fast-text-encoding` | ~3 KB | General purpose. UTF-8 only. Works everywhere. |
| `text-encoding-polyfill` | ~15 KB | Full spec. Larger. Usually overkill. |
| `expo-encoding` (Expo SDK) | Bundled | Available in Expo Go. Provides global TextEncoder/TextDecoder. |

### Recommendation

Use `fast-text-encoding` as a safety net (it no-ops if native implementations exist). It is tiny, has no dependencies, and covers both TextEncoder and TextDecoder:

```typescript
import 'fast-text-encoding'; // 3KB, no-ops if native exists
```

**Sources**:
- [Hermes TextEncoder Issue #948](https://github.com/facebook/hermes/issues/948)
- [Hermes TextDecoder Issue #1403](https://github.com/facebook/hermes/issues/1403)
- [@bacons/text-decoder](https://github.com/EvanBacon/text-decoder)
- [fast-text-encoding](https://github.com/samthor/fast-text-encoding)
- [Expo Encoding Documentation](https://docs.expo.dev/versions/v52.0.0/sdk/encoding/)

---

## 10. BigInt Support

### Hermes Status: Available

**BigInt has been available in Hermes since React Native 0.70** (released mid-2022). It was added because BigInt cannot be polyfilled -- it requires engine-level support for operator overloading (`+`, `-`, `*`, `<`, `>`, etc.).

### Current Status (2025)

BigInt is fully supported in Hermes and has been stable for 3+ years:
- Literal syntax: `123n`
- `BigInt()` constructor
- Arithmetic operators
- Comparison operators
- `BigInt.asIntN()`, `BigInt.asUintN()`
- JSON serialization requires custom `toJSON` (standard JS limitation)

### What Needs BigInt in ankurah-ts

The bincode wire format uses:
- `u64` for string/Vec lengths (8 bytes, little-endian)
- `u64` for map lengths
- Potentially `i64`/`u64` in LWW values

### Handling Strategy

There are two approaches:

**Option A: Use BigInt directly**
```typescript
// Reading a u64 from a DataView
const low = view.getUint32(offset, true);
const high = view.getUint32(offset + 4, true);
const value = BigInt(low) | (BigInt(high) << 32n);
```

**Option B: Use Number for values known to fit**
```typescript
// For lengths that will never exceed Number.MAX_SAFE_INTEGER (2^53 - 1)
const low = view.getUint32(offset, true);
const high = view.getUint32(offset + 4, true);
if (high > 0x1FFFFF) throw new Error('Length exceeds safe integer range');
const value = low + high * 0x100000000;
```

### Recommendation

For the bincode codec:
- **Use Number** for length fields (Vec/String/Map lengths) since they will never exceed 2^53
- **Use BigInt** only if actual `i64`/`u64` values appear in the data model (e.g., timestamps, large counters)
- This avoids unnecessary BigInt overhead for the common case while still supporting the full u64 range when needed

### DataView BigInt Methods

Note: `DataView.getBigInt64()` and `DataView.getBigUint64()` are also available in Hermes, providing direct 8-byte BigInt read/write:

```typescript
const value = view.getBigUint64(offset, true); // little-endian u64
view.setBigUint64(offset, value, true);
```

**Sources**:
- [Hermes as Default - React Native Blog](https://reactnative.dev/blog/2022/07/08/hermes-as-the-default)
- [Hermes GitHub](https://github.com/facebook/hermes)

---

## Summary: Required Polyfills for ankurah-ts in Expo Go

The app entry point should include these polyfills before any ankurah-ts or Yjs code:

```typescript
// index.js or App.tsx -- BEFORE all other imports

// 1. crypto.getRandomValues (needed by Yjs for client IDs)
import 'expo-crypto';
// OR: import 'react-native-get-random-values';

// 2. TextDecoder (needed by Yjs and bincode codec)
import 'fast-text-encoding';
// OR: import '@bacons/text-decoder';

// 3. No BigInt polyfill needed (native in Hermes since RN 0.70)
// 4. No TextEncoder polyfill needed (native in Hermes since RN 0.74)
// 5. No WebSocket polyfill needed (native in React Native)

// Now safe to import ankurah-ts
import { Node, Context } from '@ankurah/core';
```

## Summary: Key Constraints Table

| Constraint | Status | Impact |
|------------|--------|--------|
| No WASM in Expo Go | Confirmed, unchanged | Must use pure JS (Yjs, not Automerge) |
| No custom native modules in Expo Go | Confirmed, unchanged | All deps must be pure JS or Expo SDK |
| expo-sqlite available | Yes, with sync+async API | Primary storage engine for mobile |
| WebSocket binary data | Yes (ArrayBuffer) | bincode wire format works |
| TextEncoder | Native in Hermes | No polyfill needed |
| TextDecoder | Not reliable in Hermes | Polyfill required |
| crypto.getRandomValues | Not native in Hermes | Polyfill required (expo-crypto) |
| BigInt | Native since RN 0.70 | No polyfill needed |
| pnpm workspaces | Supported by Expo since SDK 52 | Recommended monorepo approach |
| New Architecture | Default in SDK 53 | Beneficial (faster JSI calls) |
| Yjs in Expo Go | Works with polyfills | Confirmed viable |
| Bundle size | ~60-75 KB gzipped total | No concerns |

## Corrections to Existing Specs

After this audit, the following items in the existing specs should be noted:

1. **`ecosystem-research.md`**: References `withTransactionAsync` without noting the scoping caveat. The recommended API is `withExclusiveTransactionAsync`. The difference is significant for correctness.

2. **`architecture.md`**: The example `ExpoSQLiteStorageEngine` uses `runSync` which is correct but should document that async methods are preferred for non-initialization queries to avoid blocking the JS thread.

3. **`ecosystem-research.md`**: States expo-sqlite is v16.x. This should be verified against the actual SDK version being targeted (SDK 52 ships v15.x, SDK 53 ships v16.x).

4. **No spec mentions the polyfill boot sequence**. The required polyfill imports (`expo-crypto`, `fast-text-encoding`) before any Yjs/ankurah code should be documented prominently, likely in the architecture spec or a new setup guide.
