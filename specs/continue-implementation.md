# ankurah-ts Implementation Continuation

**Purpose**: This document provides sufficient context for a fresh agent to continue the ankurah-ts implementation. Read this first, then `architectural-decisions.md`, then `port-rules.md`, then refer to other specs as needed.

**Last updated**: 2026-02-10

**Status**: Proto, signals, ankql fully done. Yrs↔Yjs V2 interop validated. Core Layers 0-4 implemented (error types, Value, PropertyBackend, LWWBackend, YjsBackend, Model traits, defineModel, Entity, Transaction, Context, Changes, Node, StorageEngine/PolicyAgent interfaces, OpenPolicy). Next: Reactor, then storage engines/connectors.

## Supervision Model

A human supervisor remains in the loop throughout implementation. The supervisor orchestrates work by dispatching background agents heavily — this is the primary working pattern. The supervisor:
- Reviews agent output before committing
- Makes all architectural and design decisions
- Resolves conflicts between agent work products
- Manages the overall implementation sequence

Agents should surface uncertainties, ambiguities, and potential conflicts to the supervisor rather than making assumptions.

### Background Agent Pattern (CRITICAL)

This project uses **aggressive background agent parallelism**. The supervisor agent (you) should:

1. **Dispatch research agents first** — Before implementing anything, send Explore-type agents to read Rust source files and produce detailed specs. Launch 3-6 research agents in parallel for independent subsystems.
2. **Implement in layers** — Identify dependency order, then dispatch implementation agents for each independent layer simultaneously. Example: Layer 0 (errors, Value, traits) has no internal deps → 3 parallel agents. Layer 1 (LWWBackend, YjsBackend, defineModel) depends on L0 but not on each other → 3 parallel agents after L0 lands.
3. **Use `run_in_background: true`** — All research and implementation agents run in the background. Check on them with `TaskOutput` (non-blocking). Wait for all agents in a layer before dispatching the next layer.
4. **Verify after each layer** — Run `npx tsc --noEmit -p packages/core/tsconfig.json` and `bun test` after each layer completes. Fix conflicts from concurrent agents editing shared files (e.g., `backend/index.ts`).
5. **Context management** — This project regularly overflows context. When that happens, the supervisor rewinds (`/clear`), re-reads this file, recovers agent outputs from `/private/tmp/claude-501/-Users-daniel-ak/tasks/*.output` (JSONL format — use `tail -1 | python3 -c "import sys,json; ..."` to extract), and continues.
6. **Agent prompts must be self-contained** — Each background agent gets a fresh context. Include: (a) which Rust files to read, (b) which TS files to create, (c) the annotation format, (d) what to import from which packages, (e) verification commands to run.

### Proven layer pattern for @ankurah/core

```
Layer 0 (no deps):     error.ts, value/*.ts, property/traits.ts, property/backend/index.ts
Layer 1 (needs L0):    backend/lww.ts, backend/yjs.ts, property/value/*.ts, model.ts, define-model.ts
Layer 2 (needs L1):    entity.ts (the central type)
Layer 3 (needs L2):    transaction.ts
Layer 4 (needs L2-3):  context.ts, node.ts
Layer 5 (needs L2-4):  reactor/*.ts (8 files)
Supporting (any time):  changes.ts, lineage.ts, selection.ts, etc.
```

## Project Goal

Create a **fully faithful TypeScript port** of [ankurah](https://github.com/ankurah/ankurah) (a Rust CRDT-backed reactive database), targeting **React Native / Expo Go** as the primary runtime. Expo Go cannot use WASM or native modules, so this must be pure TypeScript.

## Repository Layout

```
/Users/daniel/ak/
├── ankurah/                 # Rust implementation (source of truth, main branch)
├── ankurah-ts-support/      # Rust worktree on ts-port-support branch (fixtures, integration test infra)
├── ankurah-ts/              # TypeScript port (this project)
│   ├── packages/            # bun workspace packages
│   ├── scripts/             # Audit and tooling scripts
│   └── specs/               # Design documents (this directory)
│       └── _agent-work/     # Detailed agent research outputs
├── ankurah-react-hooks/     # Existing React hooks package (absorbed into @ankurah/react)
/Users/daniel/code/
└── domcorder/               # Reference project for bincode TS patterns
    └── proto-ts/            # Bincode codec reference implementation
```

**Sibling Rust checkout required**: `ANKURAH_RS_PATH` env var (default `../ankurah`). Defined in `.env` (gitignored), documented in `.env.example`. Tests hard-fail if missing.

## Key Architectural Decisions (All Resolved)

See `architectural-decisions.md` for the full authoritative list. Summary:

1. **Fully faithful port** — Mirror Rust 1:1. Zero freestyling. See `port-rules.md` for exhaustive mapping rules and exception citations.
2. **Bincode wire format only** — No JSON alternative. Reference: `/Users/daniel/code/domcorder/proto-ts/`.
3. **ankurah/proto structs are authoritative** — Rust source is the single source of truth.
4. **Yrs V2 encoding** — `Y.applyUpdateV2()` / `Y.encodeStateAsUpdateV2()`. NOT V1.
5. **Phase 1 scope — nearly everything** (see architectural-decisions.md for full list).
6. **`defineModel()` API from day one** — TS equivalent of `#[derive(Model)]`. No decorator requirement. Optional legacy decorator sugar for non-RN environments.
7. **No TC39 decorators** — Broken on Hermes/Expo Go (`_initClass` runtime error, unresolved). `Symbol.metadata` unavailable.
8. **bun workspaces + bun test + tsc** — No turborepo, no separate test framework.
9. **Fully async StorageEngine** — All methods return Promises. Matches Rust `async_trait`.
10. **Two separate SQLite packages** — `@ankurah/storage-expo-sqlite` + `@ankurah/storage-better-sqlite3`.
11. **Hand-written recursive descent AnkQL parser** — Parse + local predicate evaluation.
12. **Signals-first React integration** — `useObserve()` + `signalObserver()` HOC via `useSyncExternalStore`. Absorbed from existing `ankurah-react-hooks` into `@ankurah/react`.
13. **Every test must be ported** — All unit/integration tests in every in-scope crate. No exceptions except de-scoped crates.
14. **Programmatic audit script** — `scripts/audit-port.ts` validates bidirectional mapping. Runs in CI.
15. **Co-development branch** — `ts-port-support` branch/worktree in Rust repo for fixtures and integration test infra.

## Package Structure

**Mirroring Rust crates:**

| TS Package | Rust Crate | Notes |
|-----------|------------|-------|
| `@ankurah/proto` | `ankurah-proto` | Types + bincode codec |
| `@ankurah/core` | `ankurah-core` | Entity, Transaction, Node, Reactor, etc. |
| `@ankurah/signals` | `ankurah-signals` | Reactive signal library |
| `@ankurah/ankql` | `ankql` | Hand-written recursive descent parser |
| `@ankurah/storage-common` | `ankurah-storage-common` | Traits + types |
| `@ankurah/connector-websocket` | `ankurah-websocket-client` | WebSocket client |
| `@ankurah/connector-local` | `ankurah-connector-local-process` | Local connector |

**TS-only packages:**

| TS Package | Purpose |
|-----------|---------|
| `@ankurah/react` | React hooks (from ankurah-react-hooks) |
| `@ankurah/storage-expo-sqlite` | Expo Go SQLite storage |
| `@ankurah/storage-better-sqlite3` | Node.js SQLite storage |
| `@ankurah/storage-memory` | In-memory storage (testing) |

## Rust Architecture (Verified Against Source)

These facts are verified against the actual Rust source code (2026-02-10). Detailed findings: `_agent-work/rust-architecture-findings.md`.

### Core Concepts

- **Entity**: `Arc<EntityInner>` with `RwLock<EntityInnerState>` holding `head: Clock` + `backends: BTreeMap<String, Arc<dyn PropertyBackend>>`. Has `EntityKind::Primary` or `EntityKind::Transacted { trx_alive, upstream }`.
- **Model/View/Mutable**: Derive-macro-generated typed wrappers. Model defines schema, View is read-only, Mutable provides CRDT access within a transaction.
- **PropertyBackend**: Trait. LWW stores `BTreeMap<PropertyName, ValueEntry>`, serializes via bincode. Yrs wraps `yrs::Doc`, serializes via native V2 encoding.
- **Transaction**: Owns `AppendOnlyVec<Entity>` of forked entities. `commit()` → `generate_commit_event()` per entity → validate → store → relay → notify reactor. `Drop` sets `alive = false`.
- **Node**: Generic over `StorageEngine` and `PolicyAgent`. Owns `WeakEntitySet`, `Reactor`, peer connections. Two modes: ephemeral (client) and durable (server).
- **Context**: Type-erased (`Arc<dyn TContext>`) interface to Node. Provides `begin()`, `get()`, `fetch()`, `query()`.
- **Reactor**: Complex subsystem (8+ files in `reactor/` directory). Manages WatcherSet with index, wildcard, and entity watchers. Three-phase notification: enumerate watchers → evaluate changes → notify.
- **Signals**: Independent reactive library (16+ files). Core traits: `Signal`, `Get<T>`, `Peek<T>`, `Subscribe<T>`. Types: `Broadcast`, `Mut<T>`, `Read<T>`, `Memo<T>`, `Map<S,T>`, `Calculated<T>`. `BroadcastId` derived from `Arc` pointer address.
- **AnkQL**: PEG grammar (`ankql.pest`). SQL-like predicates with AND/OR/NOT, comparisons, IN, BETWEEN, IS NULL, dot-paths, ORDER BY, LIMIT. AST is `Serialize + Deserialize` for wire transmission.

### Bincode Configuration (CRITICAL)

**bincode 1.3.x, legacy default format.** No varint, no custom options. Every call is bare `bincode::serialize()` / `bincode::deserialize()`.

| Type | Encoding |
|------|----------|
| bool | 1 byte (0x00 / 0x01) |
| u8/i8 | 1 byte |
| u16/i16 | 2 bytes LE |
| u32/i32 | 4 bytes LE |
| u64/i64 | 8 bytes LE |
| f64 | 8 bytes LE IEEE 754 |
| String | 8-byte LE u64 length + UTF-8 bytes |
| Vec\<T\> | 8-byte LE u64 length + elements |
| Option\<T\> | 1-byte tag (0=None, 1=Some) + value |
| enum variant | 4-byte LE u32 index + fields |
| BTreeMap\<K,V\> | 8-byte LE u64 length + sorted key-value pairs |
| [u8; N] fixed | N bytes raw, NO length prefix |
| struct | fields in declaration order, no delimiters |

**Special cases**:
- `EntityId`: custom serde — raw 16 bytes in bincode (no length prefix), base64url-no-pad in JSON
- `EventId`: custom serde — raw 32 bytes in bincode, base64url-no-pad in JSON
- `TransactionId`/`RequestId`/`QueryId`/`UpdateId`: derived serde on `Ulid` — 26-char string (34 bytes: u64 length + 26 ASCII)
- `Value::Json`: uses `json_as_bytes` — JSON → `serde_json::to_vec()` → bincode `Vec<u8>`

## Spec Files

| File | Contents | Status |
|------|----------|--------|
| `continue-implementation.md` | This file. Start here. | **Current** |
| `architectural-decisions.md` | All user-confirmed decisions | **Current** |
| `port-rules.md` | Bidirectional mapping rules, exceptions, validation | **Current** |
| `architecture.md` | Module mapping, package structure | Has errors (see cleanup tracker) |
| `ecosystem-research.md` | expo-sqlite, Yjs viability, Expo Go constraints | Has errors (V1 refs) |
| `structural-mapping-analysis.md` | 1:1 mapping analysis | Superseded by port-rules.md |
| `schema-registry-and-codegen.md` | PR #236, codegen flow | Valid but codegen deferred |
| `wire-format-interop.md` | Bincode strategy | Has errors (wrong Operation shape) |
| `yrs-yjs-interop-validation.md` | Yrs/Yjs compat plan | Has errors (says V1, should be V2) |
| `initial-porting-workflow.md` | 13-phase porting guide | Phase ordering issues noted |
| `ongoing-maintenance-workflow.md` | Drift detection CI | Valid |
| `ankurah-rs-spec-cleanup.md` | Checklist of spec errors to fix | Active |
| `_agent-work/*.md` | Detailed research outputs | Reference |

**Authoritative sources**: (1) `architectural-decisions.md` for decisions, (2) `port-rules.md` for structural rules, (3) the Rust source code for type definitions.

## Current Implementation Status

### Completed
- **Monorepo scaffolding** — All 11 packages created with correct dependencies, bun install works, tsc passes
- **Audit script** — `scripts/audit-port.ts` validates bidirectional mapping (runs via `bun run scripts/audit-port.ts`)
- **`@ankurah/proto` implementation** — 16 source files: bincode codec, all ID types, Clock, auth types, data types (Event, State, Operation, EntityState), sys, peering, request/response, update, message, human_id. Zero external deps. All annotations correct. Spot-checked against Rust: all field orders, enum variant orders, and encoding patterns match.
- **Rust worktree** — `ts-port-support` branch at `/Users/daniel/ak/ankurah-ts-support/`
- **Rust bincode fixtures** — 12 test functions in `proto/tests/bincode_fixtures.rs`, 12 `.bin` files in `proto/test_fixtures/`. Covers all proto types: IDs, Clock, auth, data, request/response, causal relations, deltas, updates, messages, presence, system items. Uses `OVERWRITE_FIXTURES` env var to toggle between compare and regenerate modes. All deterministic (no random ULIDs).
- **Proto fixture parity tests** — 24 TS tests in `packages/proto/__tests__/fixtures.test.ts` (decode + round-trip for all 12 `.bin` files). Validates TS codec reads Rust output correctly and encodes byte-identical bincode. 244 assertions. Fixture path resolves to `../ankurah-ts-support/proto/test_fixtures/`.
- **`@ankurah/signals`** — 10 source files + 4 stubs (deferred: Calculated, Map, Memo, Observer auto-tracking). Core types: Broadcast, BroadcastId, Mut, Read, Signal, Get, Peek, With, Subscribe, SubscriptionGuard, ListenerGuard, ValueCell, ReadValueCell. 45 tests passing. Zero external deps.
- **`@ankurah/ankql`** — 8 source files. Hand-written recursive descent parser with lexer, full AST types (Expr, Predicate, Selection, PathExpr, Literal, etc.), SQL generation, conversion utilities. 76 tests passing. Zero external deps.
- **Yrs↔Yjs V2 interop** — 6 Rust fixture tests in `proto/tests/yrs_v2_fixtures.rs`, 6 `.bin` files in `proto/test_fixtures/yrs_v2/`. 10 TS interop tests validate Rust Yrs 0.24.0 ↔ JS Yjs 13.6.x binary compatibility via V2 encoding. Semantic comparison (not byte-for-byte, since V2 encoding may differ across implementations). Uses `OVERWRITE_FIXTURES` env var.
- **`@ankurah/core` Layer 0 — foundational types** — Error types, Value subsystem (Value enum, casting, collation, predicate casting), PropertyBackend interface + traits. 9 source files.
- **`@ankurah/core` Layer 1 — backends + model** — LWWBackend (full implementation with bincode Value serialization, field broadcasts, fork, operations), YjsBackend (Yjs V2 encoding, state vector diffing, field change detection via observe/unobserve), YrsString active type, LWW<T> active type, Model/View/Mutable trait interfaces, MutableBorrow, defineModel() with typed field helpers (lww<T>(), yrsText(), ephemeral<T>()). 7 source files + 2 test files.
- **`@ankurah/core` Layer 2 — Entity** — Entity class, EntityKind (Primary | Transacted), WeakEntitySet (with WeakRef/FinalizationRegistry). All forward references replaced with real imports. 1 source file + 1 test file (30 tests).
- **`@ankurah/core` Layer 3 — Transaction + Context + Changes** — Transaction class (create/get/edit/commit/rollback), TContext interface, Context wrapper, EntityChange (validated), ItemChange discriminated union, ChangeKind. Also added Event.id() and EventId.fromParts() to proto. 3 source files + 1 test file (33 tests).
- **`@ankurah/core` Layer 4 — Node + Storage + Policy** — Node class, NodeAndContext (full TContext impl with 5-phase commit pipeline), MatchArgs, StorageEngine/StorageCollection interfaces, PolicyAgent interface, OpenPolicy. Fixed applyEvent() to use EventId.fromParts for head clock (was TODO). 3 source files + 1 test file (36 tests).

### Core source files (24 total in packages/core/src/)
```
src/error.ts                         — All error types (AccessDenied, MutationError, RetrievalError, StateError, etc.)
src/model.ts                         — Model/View/Mutable trait interfaces, MutableBorrow
src/define-model.ts                  — defineModel() + field helpers (lww, yrsText, ephemeral)
src/entity.ts                        — Entity, EntityKind, WeakEntitySet
src/transaction.ts                   — Transaction (create, get, edit, commit, rollback)
src/context.ts                       — TContext interface, Context wrapper
src/changes.ts                       — EntityChange, ItemChange, ChangeKind
src/node.ts                          — Node, NodeAndContext (TContext impl), MatchArgs
src/storage.ts                       — StorageEngine, StorageCollection interfaces
src/policy.ts                        — PolicyAgent interface, OpenPolicy
src/value/index.ts                   — Value enum, ValueType, comparison operators, extractAtPath
src/value/cast.ts                    — CastError, castTo, tryCastTo
src/value/cast_predicate.ts          — castPredicateTypes (AnkQL predicate casting)
src/value/collatable.ts              — Collation (toBytes, successor/predecessor, compare)
src/property/index.ts                — PropertyName, Property interface, type conversions
src/property/traits.ts               — PropertyError, InitializeWith, FromEntity, FromActiveType
src/property/backend/index.ts        — PropertyBackend interface, backendFromString factory
src/property/backend/lww.ts          — LWWBackend (bincode serialization, field broadcasts)
src/property/backend/yjs.ts          — YjsBackend (V2 encoding, state vector diffs)
src/property/value/lww.ts            — LWW<T> active type wrapper
src/property/value/yrs_string.ts     — YrsString active type wrapper
src/index.ts                         — Package entry with exports
```

### Test counts (latest run)
- tsc compiles all packages with zero errors
- **309 total tests passing**, 793 assertions across 15 test files
  - 24 proto fixture parity tests (244 assertions)
  - 45 signals tests
  - 76 ankql tests
  - 10 Yrs↔Yjs V2 interop tests (18 assertions)
  - 55+ core backend tests (LWW + Yjs backend + YrsString)
  - 30 entity tests (construction, state round-trip, backends, snapshot isolation, event generation)
  - 33 transaction tests (create, get, edit, commit, rollback, isolation, EntityChange, ItemChange, Event.id())
  - 36 node tests (Node, NodeAndContext, Context integration, full commit pipeline, OpenPolicy, StorageEngine mock)

### Next up
1. **`@ankurah/core` Layer 5 — Reactor** — 8 files: WatcherSet, ComparisonIndex, Subscription, CandidateChanges, three-phase notify.
3. **`@ankurah/core` supporting types** — Lineage, LiveQuery, ResultSet, Selection, PeerSubscription.
6. **Storage engines** — storage-common traits, then expo-sqlite and better-sqlite3
7. **Connectors** — WebSocket client, local connector
8. **`@ankurah/react`** — Absorb ankurah-react-hooks, wire to TS signals
9. **Integration tests** — Spawn Rust WS servers, test TS↔Rust interop

### Rust-side work (ts-port-support branch)
- ~~Bincode fixture generation tests~~ DONE (12 tests, 12 `.bin` files)
- ~~Yrs V2 fixture generation~~ DONE (6 tests, 6 `.bin` files in `proto/test_fixtures/yrs_v2/`)
- Integration test server binary
- Any spec cleanup PRs

### Core architecture notes (from research)
- **Entity**: `Arc<EntityInner>` → plain class in TS. `RwLock<EntityInnerState>` → plain mutable fields (JS single-threaded). `EntityKind.Primary | EntityKind.Transacted { trxAlive, upstream }`. `snapshot()` forks all backends for transaction isolation.
- **Transaction**: `AppendOnlyVec<Entity>` → regular array. `alive: Arc<AtomicBool>` → plain boolean. `commit()` generates events, validates via PolicyAgent, stores, relays to peers, notifies Reactor.
- **Reactor**: Three-phase notification. Phase 1: enumerate watchers (WatcherSet has index/wildcard/entity tiers). Phase 2: evaluate changes per subscription (filter entities against predicates, compute membership changes). Phase 3: broadcast ReactorUpdate to subscribers via signals.
- **defineModel() API**: Returns `{ View, Mutable, collection(), fields }`. View has typed getters returning projected types. Mutable has typed getters returning active type handles (LWW<T>, YrsString). Field helpers: `lww<T>()`, `yrsText()`, `ephemeral<T>()`. Placeholder wiring — actual backend access deferred to Entity implementation.

## Agent Working Notes

### Annotation format
- Line 1: `// MIRRORS: ankurah/<crate>/src/<path>.rs` — bare path only, NO extra text like `[E2]` or `(tests module)`
- Exception citations go on a SEPARATE line 2: `// Exception E12: file-with-submodules pattern`
- Test files: `// MIRRORS: ankurah/<crate>/src/<path>.rs` (same as source, bare path)
- TS-only files: `// TS-ONLY: <reason>`

### Fixture path convention
- Rust fixtures: `/Users/daniel/ak/ankurah-ts-support/proto/test_fixtures/`
- TS tests resolve via: `path.resolve(__dirname, '../../../../ankurah-ts-support/proto/test_fixtures')`
- Hard-fail with descriptive error if fixture dir missing

### ULID serialization (confirmed)
- `ulid` crate v1.2.1 serializes as 26-char Crockford Base32 string (always, no `is_human_readable` check)
- In bincode: 8-byte u64 length (=26) + 26 ASCII bytes = 34 bytes total
- Deterministic test ULIDs constructed via: `"0000000000000000000000" + decimal(seed).padStart(4, '0')`

### Sub-agent tips
- Grant `.cargo/registry` read permission proactively for Rust source exploration
- MIRRORS annotations must be bare paths — audit script validates
- After implementation, always run: `npx tsc --noEmit -p packages/<pkg>/tsconfig.json` and `bun test packages/<pkg>/`
- Run `bun run scripts/audit-port.ts` to verify structural compliance

## Instructions for a Continuing Agent

1. **Read `architectural-decisions.md`** — all decisions are finalized
2. **Read `port-rules.md`** — structural mapping rules are non-negotiable
3. **Check active work** — look at recent commits and open branches to understand what's in flight
4. **Surface uncertainties** — do not assume; ask the supervisor
5. **Follow port-rules.md strictly** — every file needs line 1 annotation, every exception needs a rule citation
6. **Port tests** — every test in every in-scope crate must have a TS equivalent
7. **Run the audit script** — `bun run scripts/audit-port.ts` to check compliance
8. **Work primarily through background agents** — You are the supervisor. Dispatch `Task` agents with `run_in_background: true` for ALL research and implementation work. Maximize parallelism within each layer. Keep your own context lean by delegating. See "Background Agent Pattern" section above for the full protocol.
9. **After context rewinds** — Re-read this file first. Check `/private/tmp/claude-501/-Users-daniel-ak/tasks/*.output` for recent agent outputs. Run `bun test` and `tsc` to verify current state. Then continue from the "Next up" list.
