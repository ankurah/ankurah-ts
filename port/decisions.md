# Architectural Decisions

Last updated: 2026-02-10

---

## Wire Format

- Bincode only. No JSON wire format alternative.
- TS must implement bincode reader/writer for all proto types.
- **ankurah/proto structs are authoritative** -- specs must not duplicate or paraphrase the structs defined in `ankurah/proto/src/`. Any spec that overlaps with proto definitions should delete the overlapping descriptions (tracked in `ankurah-rs-spec-cleanup.md`).
- **domcorder as reference implementation** -- `/Users/daniel/code/domcorder/proto-ts/` is the authoritative reference for how to implement a bincode codec in TypeScript. Patterns from domcorder (reader/writer structure, enum variant encoding, Vec lengths, Option encoding) should be followed in ankurah-ts.
- **Reference fixtures** -- Rust-side tests generate bincode reference fixtures at `ankurah/proto/test/fixtures/` for cross-language validation. ankurah-ts tests read fixtures from the sibling `../ankurah/` checkout (required).

## CRDT Backend

- Yjs (pure JS) replaces Yrs (Rust) for collaborative text fields.
- **Yrs V2 encoding confirmed** -- The Rust code uses V2 exclusively (`encode_state_as_update_v2`, `decode_v2`, `encode_diff_v2`). The TS port must use `Y.applyUpdateV2()` / `Y.encodeStateAsUpdateV2()`. V1 encoding is NOT used.
- Yrs/Yjs interop must be validated carefully before proceeding (state buffers, operations, merge behavior).
- **Yrs client ID: random** -- Use default `Y.Doc()` with random client ID, matching Rust behavior.

## Port Fidelity

- Fully faithful port. No corners cut.
- Mirror Rust file/struct/impl/test structure 1:1 wherever possible.
- The closer the structural mapping, the easier agentic validation and ongoing maintenance.
- camelCase for TS names, but preserve the same logical names.
- **Error handling: parity with Rust** -- Mirror Rust error types (`MutationError`, `RetrievalError`, `StateError`, `PropertyError`, `DecodeError`) as TS `Error` subclasses. Goal is the simplest, most consistent mapping between Rust and TS for port maintenance.

## Codebase Organization

- Three clear zones in the TS codebase:
  - **1:1 mapped**: Files mirroring a specific Rust file. Annotate with `// MIRRORS: ankurah/core/src/entity.rs`
  - **Completely different**: Files with no Rust counterpart. Annotate with `// TS-ONLY: no Rust counterpart`
  - **Bridge code**: Files translating between Rust-compatible and TS-native patterns. Annotate inline where mapping diverges.

## Phase 1 Scope

- **No descoping of core libraries** -- `proto`, `core`, `signals`, `ankql` are ALL fully in scope for Phase 1. This includes `Attested<T>`, `CausalRelation`, and all lineage types (they appear in the wire protocol and must be ported).
- **In scope**: Entity, Model/View/Mutable, LWW backend, Yjs backend, Transaction, Node, Context, Reactor, LiveQuery, Signals, AnkQL parser, expo-sqlite storage, better-sqlite3 storage (Node testing), in-memory storage, WebSocket client, local connector, React Native hooks, Attested<T>, CausalRelation, lineage types.
- **Out of scope**: WebSocket server (use Rust server), PostgreSQL storage, Sled storage, PN Counter backend.
- IndexedDB storage, WASM build target, and policy agent (beyond PermissiveAgent) remain out of scope for Phase 1 but are not permanently descoped.

## Model Definition API

- **`defineModel()` from day one** -- The TS equivalent of Rust's `#[derive(Model)]` proc macro. Functional API that generates View/Mutable/Input/Ops types from a schema definition:
  ```typescript
  const Document = defineModel('document', {
    title: lww<string>(),
    body: yrsText(),
  });
  ```
- **No decorators required** -- TC39 decorators are broken on Hermes/Expo Go (`_initClass` runtime error, unresolved). `Symbol.metadata` unavailable on Hermes. The canonical API is `defineModel()` — portable, works everywhere.
- **Optional legacy decorator sugar** -- Legacy Babel decorators may be provided as opt-in syntax sugar over `defineModel()`. Used in tests and non-RN environments. Never required. Documented as "not supported with TC39 decorators on Hermes/Expo Go."
- **Codegen CLI (future)** -- May generate `defineModel()` calls from a schema file. Not needed for Phase 1.

## Monorepo Tooling

- **bun workspaces** -- Confirmed. Already proven in domcorder. Built-in test runner, no turborepo needed. Metro (Expo's bundler) resolves from `node_modules` identically regardless of package manager, so fully compatible with Expo Go.
- **bun test** -- Built-in test runner (vitest-compatible API). No separate test framework dependency.
- **tsc** -- For type-checking and declaration file generation. No bundler needed (Metro consumes TS source directly for RN; bun runs TS directly for tests).

## Polyfills and Runtime

- **Polyfill boot sequence** -- `expo-crypto` for `crypto.getRandomValues`, `fast-text-encoding` for `TextDecoder`. Import before any ankurah/Yjs code. Use whatever approach is most elegant.
- **Primary target**: React Native / Expo Go (no WASM, no native modules).
- **Secondary target**: Node.js (for testing with better-sqlite3).
- expo-sqlite for mobile storage, standard WebSocket API for sync.

## Sibling Rust Worktree

- **Rust ankurah checkout required as sibling** -- ankurah-ts assumes a sibling checkout of ankurah/ for fixtures, integration tests, and structural comparison.
- **Path configuration**: `ANKURAH_RS_PATH` env var, defaulting to `../ankurah`. Defined in `.env` (gitignored), with `.env.example` checked in documenting the default.
- **Integration tests start real Rust servers** -- Some test suites `cargo build` and spawn Rust WebSocket servers from the sibling checkout. Test harness manages lifecycle (spawn, wait for ready, run tests, kill).
- **Co-development branch** -- Fixture generation, RS/TS integration test runners, and spec cleanups are developed on a dedicated branch/worktree in the Rust repo.

## AnkQL Parser

- **Hand-written recursive descent** -- Zero dependencies, most portable, full control over AST shape matching Rust 1:1.
- **Parse + local predicate evaluation** -- The TS client evaluates predicates locally for optimistic filtering, not just parse-and-send.

## Storage Interface

- **Fully async** -- All `StorageEngine` methods return Promises. Matches Rust (`async_trait`).
- **Two separate SQLite packages** -- `@ankurah/storage-expo-sqlite` (React Native/Expo Go) and `@ankurah/storage-better-sqlite3` (Node.js testing). Both implement `StorageEngine` from `@ankurah/storage-common`. No shoehorning different implementations into one package.
- **In-memory storage** -- `@ankurah/storage-memory` for unit testing.

## Resolved Open Questions (2026-02-10)

- **Bincode ULID serialization**: Match Rust exactly — 26-char string for TransactionId/RequestId/QueryId/UpdateId (34 bytes on wire: u64 length + 26 ASCII).
- **BTreeMap sort order**: UTF-8 byte-lexicographic order (match Rust `Ord` for `String`). Use explicit UTF-8 byte comparison in TS, not `Array.sort()`.
- **Bincode u64 lengths**: `number` for length fields (with bounds check), `bigint` for actual i64/u64 data values. Per domcorder patterns.
- **EntityId types**: Shared `EntityId` type. Different codec paths for Literal::EntityId (26-char ULID string) vs proto EntityId (16 raw bytes).
- **Entity identity in TS**: `EntityId` value equality (not pointer equality). No `Arc::ptr_eq` equivalent needed.
- **Transaction cleanup**: Explicit `dispose()` + try/finally pattern. No reliance on `Symbol.dispose` (Hermes support unverified).
- **AppendOnlyVec**: Plain array (TS is single-threaded).
- **WeakRef**: Use `WeakRef` with manual cleanup fallback if unavailable.
- **Signals Phase 1 minimum**: `Broadcast`, `Signal` trait, `Subscribe`, `ListenerGuard`. Defer `Memo`, `Map`, `Calculated`, `reactive_graph`.
- **Signal auto-dependency tracking**: Defer. Use explicit subscriptions; React's own reactivity model handles UI.
- **BroadcastId**: Auto-incrementing integer counter (no pointer identity in TS).
- **Wire stub types**: Full types with empty/default attestation values. `Attested<T>` wraps with empty `AttestationSet` on commit.
- **`sys::Item::Other` / `#[serde(other)]`**: Replicate — return `{ type: 'Other' }` for unknown enum variants.
- **Presence `system_root`**: Hardcode `null` for Phase 1 (ephemeral TS nodes).
- **In-memory storage**: Yes, for unit testing. Separate package or built into storage-common.
- **Storage common scope**: Traits + types for Phase 1. Skip query planner (server-side concern).
- **Reactor**: Simplified client-side implementation. All three watcher types (index, wildcard, entity) but less complexity than server.
- **React hooks**: Signals-first architecture. `useObserve()` + `signalObserver()` HOC using `useSyncExternalStore`. Absorbed from existing `ankurah-react-hooks` package into `@ankurah/react`. Factory pattern: `createAnkurahReactHooks(bindings)` takes TS `ReactObserver` implementation.
- **WebSocket reconnection**: Built into `@ankurah/connector-websocket` with exponential backoff.
- **Yjs empty V2 update**: Validate empirically during implementation.
- **Yjs getText on nonexistent key**: Track initialized properties explicitly in YjsBackend.
- **Spec cleanup**: Delete wrong sections, keep valid architectural context. Do not duplicate proto type definitions.

## Port Structure Rules

- **See `specs/port-rules.md`** for the exhaustive bidirectional file mapping and all exception rules.
- **Zero freestyling** -- Every TS file maps 1:1 to a Rust file. Suboptimal or inelegant Rust structures are faithfully reproduced. Exceptions require an explicit rule citation (e.g., `// Exception E1: no proc macro equivalent`) and justification based on language or environmental limitation.
- **Rust feature-gated modules → TS separate packages** -- Rust uses `#[cfg(feature = "...")]` for optional deps (e.g., `react` feature in signals). TS has no conditional compilation, so these become separate packages (e.g., `@ankurah/react` instead of a feature flag on `@ankurah/signals`).
- **Rust single-crate multi-platform → TS separate packages** -- Where Rust has one crate with one binding (e.g., `storage/sqlite/` with `rusqlite`), but TS has fundamentally different platform bindings (expo-sqlite vs better-sqlite3), use separate TS packages.

## TS-Only Packages

Packages with no direct Rust crate equivalent:

| Package | Purpose | Origin |
|---------|---------|--------|
| `@ankurah/react` | React hooks (`useObserve`, `signalObserver` HOC) | Absorbed from `ankurah-react-hooks` repo; replaces Rust `signals/src/react.rs` feature-gated module |
| `@ankurah/storage-expo-sqlite` | Expo Go SQLite storage | Platform-specific split of Rust `storage/sqlite/` |
| `@ankurah/storage-better-sqlite3` | Node.js SQLite storage (testing) | Platform-specific split of Rust `storage/sqlite/` |
| `@ankurah/storage-memory` | In-memory storage (unit testing) | TS-only test utility |

## Test Porting

- **Every test must be ported** -- All unit tests and integration tests in every in-scope crate must be ported to TS. The only exceptions are tests in crates expressly de-scoped (storage-postgres, storage-sled, storage-indexeddb-wasm, connector-websocket-server, connector-websocket-client-wasm, derive). Tests gated behind de-scoped features (e.g., `#[cfg(feature = "postgres")]`) are also excluded.
- **Test structure mirrors Rust** -- Rust inline `#[cfg(test)] mod tests` → TS `foo.test.ts` adjacent to `foo.ts`. Rust `tests/` integration tests → TS `__tests__/` directory. Test names and coverage should match.
- **Hard fail when Rust checkout missing** -- All tests that need `ANKURAH_RS_PATH` fail immediately with a clear error, not skip. Prevents shipping without integration coverage.
- **Rust WS server tests** -- Test harness `cargo build`s and spawns Rust WebSocket servers. Manages full lifecycle: spawn → wait for ready → run tests → kill.

## Audit Script

- **Programmatic compliance audit** -- A script (`scripts/audit-port.ts` or similar) that programmatically validates the bidirectional mapping between the Rust and TS repos. Provides a clear pass/fail signal. Checks:
  - Every in-scope Rust source file has a corresponding TS file
  - Every TS file has a valid line 1 annotation (`MIRRORS` or `TS-ONLY`)
  - Every `MIRRORS` annotation points to an existing Rust file
  - Exception rules are cited where the mapping diverges
  - Every Rust test module/file has a corresponding `.test.ts`
  - No orphaned TS files (TS file claims to mirror a Rust file that doesn't exist)
- Run as part of CI. Reads `ANKURAH_RS_PATH` to locate the Rust checkout.

## Async Serialization

- **Reactor notification pipeline**: Uses `PromiseMutex` (mirrors Rust's `tokio::sync::Mutex<()> notify_lock`).
- **WatcherSet gap-fill**: Fire-and-forget `fillGapsAndNotify()` mutates WatcherSet outside notify lock. Needs awaiting or its own PromiseMutex.
- **LiveQuery activation**: Concurrent activations can race (same bug in Rust, issue #146). Needs serialization.
- **SystemManager lifecycle**: Low risk, initialization-time only. Consider PromiseMutex if connector porting surfaces races.

## Known Gotchas

- **NodeLikeAdapter**: Adapters bridging reactor interfaces to Node must hold strong references. WeakRef-only adapters get GC'd while the subscription is still active.
- **Transaction alive gap**: `commit()` and `rollback()` set `alive = false` eagerly to close the gap between unreachability and GC.
- **`using` escape hatch**: `let bar; { using foo = ..; bar = foo; }` leaks a disposed reference. `assertNotDisposed()` converts this from silent failure to loud error.
- **Observer stack balance**: Reactive tracking context push/pop must use try/finally.

## Minimal Rust Changes Required

- Complete PR #236 (property registration / schema registry).
- Add bincode reference fixture generation to `ankurah/proto`.
- Add schema export capability (derive macro emits `schema_json()` method on Model trait).
- Add integration test infrastructure (TS-callable WS server startup).
- These benefit the Rust codebase regardless of the TS port.
