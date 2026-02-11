# ankurah-ts Implementation Continuation

**Purpose**: This document provides sufficient context for a fresh agent to continue the ankurah-ts implementation. Read this first, then `architectural-decisions.md`, then `port-rules.md`, then refer to other specs as needed.

**Last updated**: 2026-02-11 (Layer 7 in progress, last commit e86d01e)

**Status**: Proto, signals, ankql fully done. Yrs↔Yjs V2 interop validated. Core Layers 0-6b fully implemented. Reactor complete (8 files + 9 tests). Supporting types done. storage-common fully ported. lineage.ts + system.ts done with tests. Layer 6a+6b committed: livequery, storage-memory, integration tests, GC bug fix. **Layer 7 in progress**: ReadyChunks utility (5 tests), PolicyAgent validation methods added, NodeApplier implemented (422 lines). Peer subscription and node networking methods remain. **376 tests passing**, 1016 assertions, 22 test files. **Next: peer_subscription/ → node networking methods → Layer 8 (connectors + React).**

## Supervision Model

**You (the Claude Code agent) are a SUPERVISOR, not an implementor.** Your role is to orchestrate background agents that do the actual work. A human supervisor remains in the loop above you, reviewing output and making decisions.

### Your Role as Supervisor Agent

**DO:**
- Dispatch background `Task` agents for ALL research and implementation work
- Read files to understand structure and verify agent output
- Run `tsc` and `bun test` to verify correctness after agents complete
- Fix small integration issues (import conflicts, type mismatches between agent outputs)
- Commit checkpoint commits after each verified layer
- Surface uncertainties to the human supervisor

**DO NOT:**
- Write implementation code yourself (this burns context that should be preserved for orchestration)
- Read large Rust source files inline (dispatch an Explore agent instead)
- Hold detailed implementation knowledge in your context (delegate it to agents)

The reason for this discipline is **context management**. This project regularly overflows context windows. Every line of code you write inline, every Rust file you read directly, consumes context that could instead be used for 2-3 more rounds of agent orchestration. Background agents get their own fresh context windows — use them.

### Background Agent Pattern (CRITICAL)

This project uses **aggressive background agent parallelism**. As supervisor you should:

1. **Dispatch research agents first** — Before implementing anything, send Task agents (subagent_type: "general-purpose" or "Explore") to read Rust source files and produce detailed specs. Launch 3-6 research agents in parallel for independent subsystems. Use `run_in_background: true`.
2. **Dispatch implementation agents** — After research completes, send Task agents to write the TS files. Each agent gets a self-contained prompt with: (a) which Rust files to read, (b) which TS files to create/modify, (c) the annotation format from port-rules.md, (d) what to import from which packages, (e) verification commands to run. Use `run_in_background: true`.
3. **Implement in layers** — Identify dependency order, then dispatch implementation agents for each independent file simultaneously. Example: Layer 0 (errors, Value, traits) has no internal deps → 3 parallel agents. Layer 1 (LWWBackend, YjsBackend, defineModel) depends on L0 but not on each other → 3 parallel agents after L0 lands.
4. **Verify after each layer** — Run `npx tsc --noEmit` and `bun test` after each layer's agents complete. Fix conflicts from concurrent agents editing shared files (e.g., `index.ts` exports).
5. **Commit after each layer** — Create a checkpoint commit so work is preserved across context rewinds.
6. **Context management** — This project regularly overflows context. When that happens, the human supervisor rewinds (`/clear`), you re-read this file, recover agent outputs from `/private/tmp/claude-501/-Users-daniel-ak/tasks/*.output`, and continue from the "Next up" list.

### Proven layer pattern for @ankurah/core

```
Layer 0 (no deps):     error.ts, value/*.ts, property/traits.ts, property/backend/index.ts
Layer 1 (needs L0):    backend/lww.ts, backend/yjs.ts, property/value/*.ts, model.ts, define-model.ts
Layer 2 (needs L1):    entity.ts (the central type)
Layer 3 (needs L2):    transaction.ts
Layer 4 (needs L2-3):  context.ts, node.ts, storage.ts, policy.ts
Layer 5 (needs L2-4):  reactor/*.ts (8 files)
Supporting (any time):  changes.ts, lineage.ts, selection.ts, etc.
Post-core:             storage-memory, storage engines, connectors, react
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
| `port-maintainability-analysis.md` | Assessment of port's automated update readiness | **Current** (score: 7.5/10) |
| `progress-and-parallelism-review.md` | Remaining work breakdown + parallelization plan | **Current** |
| `_agent-work/*.md` | Detailed research outputs (incl. reactor-main-spec.md) | Reference |

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
- **`@ankurah/core` Layer 5a — Reactor foundational types** — ReactorUpdate/ReactorUpdateItem/MembershipChange (reactor/update.ts), PropertyPath with value extraction (reactor/property-path.ts), ComparisonIndex with sorted-array BTreeMap substitute for gt/lt range queries (reactor/comparison-index.ts), CandidateChanges zero-copy wrapper with per-query and entity offset maps (reactor/candidate-changes.ts), Filterable interface + evaluatePredicate recursive predicate evaluator (selection/filter.ts). 5 source files. Also fixed tsc errors in node.test.ts (CollectionId branded type + OpenPolicy arg count).
- **`@ankurah/core` Layer 5b — Reactor mid-layer** — WatcherSet (reactor/watcher_set.ts), fetch-gap (reactor/fetch_gap.ts), EntityResultSet (resultset.ts), indexing types (indexing/key_spec.ts, indexing/encoding.ts). WatcherSet implements three registries (index/wildcard/entity watchers) with accumulate_interested_watchers as the hot path. EntityResultSet provides ordered entity list with HashMap index, ORDER BY sort keys, LIMIT, gap_dirty flag, and write/read guard semantics. Indexing types provide KeySpec, IndexKeyPart, encoding for tuple values. GapFetcher trait + QueryGapFetcher build continuation predicates from ORDER BY for LIMIT-constrained queries. 6 source files (including indexing/index.ts barrel).
- **`@ankurah/core` Layer 5c — Reactor top-layer** — Subscription state (reactor/subscription_state.ts: Subscription class with evaluateChanges, QueryState, gap filling, 815 lines), ReactorSubscription (reactor/subscription.ts: public handle with Signal/Subscribe + dispose), Reactor main (reactor/index.ts: three-phase notifyChange pipeline, subscribe/unsubscribe, addQueryAndNotify/updateQueryAndNotify, PromiseMutex for async serialization, 499 lines). 3 source files.
- **`@ankurah/core` supporting types** — schema.ts (CollectionSchema interface), query_value.ts (QueryValue discriminated union), connector.ts (PeerSender, NodeComms interfaces), collectionset.ts (CollectionSet lazy-init cache), retrieval.ts (TEvent, TClock, GetEvents, LocalRetriever). 5 source files.
- **`@ankurah/storage-common` implementation** — 6 source files: types.ts (Plan, KeyBounds, OrderByComponents), predicate.ts (ConjunctFinder), bounds.ts (normalize KeyBounds to CanonicalRange), sorting.ts (async generators: sortedIterable, limitedIterable, topKIterable), filtering.ts (filterPredicate, sortBy, limit, topK), planner.ts (full query planner with ORDER-FIRST and INEQ-FIRST strategies). Rust Stream to TS AsyncIterable.
- **Drift detection** — SHA-256 hash manifest in scripts/rust-source-hashes.json tracking 155 files (85 source + 70 test). audit-port.ts supports --backpopulate and --update-manifest commands. port-rules.md updated with section G6.
- **`@ankurah/core` lineage.ts** — Bidirectional BFS DAG comparison engine (~600 lines TS from ~1004 lines Rust). Generic interfaces (LClock, LEvent, LGetEvents, LAttested) for flexible ID types. EventAccumulator, Ordering discriminated union (6 variants), compare/compareUnstoredEvent/compareWithAccumulator. 13 tests (linear history, concurrent, incomparable, empty clocks, budget exceeded, self comparison, multiple roots, unstored events, redundant delivery, event accumulator variants). Rust SmallVec→Array, BTreeSet→Set, saturating_sub→Math.max(0, x-y).
- **`@ankurah/core` system.ts** — SystemManager (~325 lines TS from ~316 lines Rust). Constants SYSTEM_COLLECTION_ID/_PROTECTED_COLLECTIONS. Methods: root(), getItems(), isLoaded/isSystemReady/waitLoaded/waitSystemReady, collection(), create(), joinSystem(), hardReset(), loadSystemCatalog(). sysItemToValue/sysItemFromValue for cross-language JSON interop. Deferred Promise pattern replacing Rust tokio::sync::Notify. 23 tests (round-trip, construction, lifecycle, joins, reset).
- **Layer 5 reactor tests** — 9 unit tests ported from Rust: ComparisonIndex (2: field_index, not_equal), CandidateChanges (3: empty, add_query, entity_level), FetchGap (3: single_column_asc, multi_column, infer_value_type), Reactor end-to-end (1: entity_remains_watched_after_predicate_stops_matching). Uses sortQueryIds() helper for Rust BTreeSet→TS ordering parity.

### Layer 6a — COMMITTED (commits 46ba45b, 01a2c55)

**New files:**
- **`@ankurah/core` livequery.ts** (726 lines) — EntityLiveQuery, WeakEntityLiveQuery, LiveQuery<V> generic typed wrapper, RemoteQuerySubscriber interface (stub), ChangeSet<V>, liveQueryChangeSetFrom(). Async init via fire-and-forget (`void me.activate(1).then(...)`). Promise-based Notify replacement for `wait_initialized()`. Signal/Get/Peek/Subscribe trait impls for reactive integration. Mirrors `ankurah/core/src/livequery.rs` (~399 lines Rust).
- **`@ankurah/storage-memory` src/index.ts** (188 lines, was 6-line stub) — MemoryStorageEngine (lazy-creates collections), MemoryStorageCollection (Map-backed in-memory storage). `entityStateAsFilterable()` helper reconstructs PropertyBackends from StateBuffers for predicate evaluation. `compareForSort()` helper for ORDER BY with valuePartialCmp. TS-only package (no Rust counterpart).

**Modified files:**
- **node.ts** (now 360 lines, was 327) — Added `readonly reactor: Reactor` field (default-constructed if not provided in options). Added `readonly subscriptionRelay: null = null` Phase 1 stub. Added Phase 7 reactor notification in `commitLocalTrx()`: collects `EntityChange` per entity, calls `reactor.notifyChange(entityChanges)`. Added `NodeAndContext.query()` method wired to `EntityLiveQuery.create()`.
- **context.ts** (now 156 lines, was 128) — Added `query(collectionId, args)` to TContext interface. Added `query()` to Context class delegating to `this.inner.query()`.
- **changes.ts** — Added ChangeSet<V> interface (batch of changes for LiveQuery subscriptions).
- **index.ts** — Added exports for livequery types, valuePartialCmp, backendFromString.
- **storage-memory/package.json** — Added `@ankurah/core` and `@ankurah/ankql` dependencies.

**New spec files:**
- `specs/livequery-port-spec.md` — Comprehensive port spec for livequery.ts (21 sections)
- `specs/storage-memory-impl-spec.md` — Implementation spec for storage-memory package
- `specs/_agent-work/node-context-gap-analysis.md` — Gap analysis for node.ts + context.ts reactor integration

### Layer 6b — COMMITTED (commits bb58066, 7294de3)

**New test files:**
- **`@ankurah/storage-memory` __tests__/storage-memory.test.ts** — 11 tests: set/get/list/filter/sort/limit/missing-collection/concurrent-writes/state-round-trip/delete/empty-filter.
- **`@ankurah/core` __tests__/livequery.test.ts** — 6 LiveQuery integration tests: basic subscription, complex transitions, signal semantics, predicate update, single-node gap filling, multi-gap filling.

**Test infrastructure:**
- **TestWatcher utility** — Reusable watcher for tracking LiveQuery change events in tests.
- **queryWait helper** — Async helper that creates a LiveQuery and waits for initialization before returning.
- **Test model definitions** — Shared defineModel() definitions for use across LiveQuery tests.

**Bug fixes:**
- **ReactorUpdate barrel export fix** — Added type-only re-export of ReactorUpdate from reactor barrel (was missing, caused import errors in tests).
- **NodeLikeAdapter GC bug fix in EntityLiveQuery** — Fixed issue where FinalizationRegistry/WeakRef GC in NodeLikeAdapter caused premature cleanup of entities referenced by live queries.

**Drift detection:**
- Hash manifest expanded to 155 files (85 source + 70 test).

### Core source files (45 total in packages/core/src/)
```
src/error.ts                         — All error types (AccessDenied, MutationError, RetrievalError, StateError, etc.)
src/model.ts                         — Model/View/Mutable trait interfaces, MutableBorrow
src/define-model.ts                  — defineModel() + field helpers (lww, yrsText, ephemeral)
src/entity.ts                        — Entity, EntityKind, WeakEntitySet
src/transaction.ts                   — Transaction (create, get, edit, commit, rollback)
src/context.ts                       — TContext interface, Context wrapper
src/changes.ts                       — EntityChange, ItemChange, ChangeKind, ChangeSet
src/node.ts                          — Node, NodeAndContext (TContext impl), MatchArgs
src/storage.ts                       — StorageEngine, StorageCollection interfaces
src/policy.ts                        — PolicyAgent interface, OpenPolicy
src/schema.ts                        — CollectionSchema interface
src/query_value.ts                   — QueryValue discriminated union
src/connector.ts                     — PeerSender, NodeComms interfaces
src/collectionset.ts                 — CollectionSet lazy-init cache
src/retrieval.ts                     — TEvent, TClock, GetEvents, LocalRetriever
src/resultset.ts                     — EntityResultSet (ordered entity list, sort keys, LIMIT, gap_dirty)
src/livequery.ts                     — EntityLiveQuery, LiveQuery<V>, WeakEntityLiveQuery, RemoteQuerySubscriber (Layer 6a)
src/lineage.ts                       — EventAccumulator, Ordering, compare/compareUnstoredEvent (BFS DAG comparison)
src/system.ts                        — SystemManager, SYSTEM_COLLECTION_ID, sysItemToValue/sysItemFromValue
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
src/reactor/update.ts                — MembershipChange, ReactorUpdate, ReactorUpdateItem
src/reactor/property-path.ts         — PropertyPath (field path with JSON sub-path support)
src/reactor/comparison-index.ts      — ComparisonIndex<T> (field-level index for watcher matching)
src/reactor/candidate-changes.ts     — CandidateChanges<C>, QueryCandidate<C> (zero-copy change wrapper)
src/reactor/watcher_set.ts           — WatcherSet (three registries: index/wildcard/entity watchers)
src/reactor/fetch_gap.ts             — GapFetcher trait, QueryGapFetcher, build_continuation_predicate
src/reactor/subscription_state.ts    — Subscription class (evaluateChanges, QueryState, gap filling, 815 lines)
src/reactor/subscription.ts          — ReactorSubscription public handle (Signal/Subscribe + dispose)
src/reactor/index.ts                 — Reactor main (three-phase notifyChange, PromiseMutex, Exception E12)
src/indexing/key_spec.ts             — KeySpec, IndexKeyPart, IndexDirection, NullsOrder, IndexSpecMatch
src/indexing/encoding.ts             — IndexError, encodeTupleValuesWithKeySpec
src/indexing/index.ts                — Barrel export for indexing module
src/selection/filter.ts              — Filterable interface, evaluatePredicate()
src/node_applier.ts                  — NodeApplier (remote update/delta application, ReadyChunks batching)
src/util/ready_chunks.ts             — ReadyChunks<T> async iterable (batched concurrent promise resolution)
src/index.ts                         — Package entry with exports
```

### Test counts (latest stable: commit e86d01e)
- At commit e86d01e: tsc zero errors, **376 tests passing**, 1016 assertions across 22 test files
  - 24 proto fixture parity tests (244 assertions)
  - 45 signals tests
  - 76 ankql tests
  - 10 Yrs↔Yjs V2 interop tests (18 assertions)
  - 55+ core backend tests (LWW + Yjs backend + YrsString)
  - 30 entity tests (construction, state round-trip, backends, snapshot isolation, event generation)
  - 33 transaction tests (create, get, edit, commit, rollback, isolation, EntityChange, ItemChange, Event.id())
  - 36 node tests (construction, commit pipeline, isolation, query wiring)
  - 13 lineage tests (linear/concurrent/incomparable history, budget, accumulator, unstored events)
  - 23 system tests (sysItem round-trip, construction, create, join, reset lifecycle)
  - 9 reactor tests (ComparisonIndex, CandidateChanges, FetchGap, end-to-end watcher)
  - 11 storage-memory tests (set/get/list/filter/sort/limit/missing-collection/concurrent-writes/state-round-trip/delete/empty-filter)
  - 6 livequery tests (basic subscription, complex transitions, signal semantics, predicate update, single-node gap filling, multi-gap filling)
  - 5 ready-chunks tests (simultaneous drain, pending-until-ready, empty, mixed, len/isEmpty tracking)

### Next up

**Layer 6b remaining (deferred until property types ported):**
1. **JSON path tests** — Needs `Json` property type (currently deferred in property/value/json.ts).
2. **Pagination tests** — Needs `Ref` property type (currently deferred in property/value/entity_ref.ts).

**Layer 7 — Networking (in progress):**
3. ~~**`@ankurah/core` node_applier.ts**~~ — DONE (commit e86d01e). 422 lines. applyUpdates (guarded until SubscriptionRelay), applyDeltas (ReadyChunks concurrent), DeltaContent/UpdateContent handlers.
4. ~~**ReadyChunks utility**~~ — DONE (commit a1c6780). Async iterable batching concurrent promises. 5 tests.
5. ~~**PolicyAgent validation methods**~~ — DONE (commit a1c6780). validateReceivedEvent/validateReceivedState added.
6. **`@ankurah/core` peer_subscription/** — Client relay (SubscriptionRelay: 5-state machine, 10 public methods, retry timer), server (SubscriptionHandler: per-peer reactor subscription). Detailed spec available in agent research output.
7. **Complete node.rs networking** — PeerState struct, register_peer/deregister_peer, request/sendUpdate (RPC correlation), handleMessage dispatcher, handleRequest/handleUpdate, relayToRequiredPeers, commitRemoteTransaction, TNodeErased full impl.

**Layer 8 — Connectors + React (needs 7):**
6. **Connectors** — connector-local (simple), connector-websocket (with reconnection).
7. **`@ankurah/react`** — Absorb ankurah-react-hooks, wire to TS signals.

**Layer 9 — Integration:**
8. **Integration tests** — Spawn Rust WS servers, test TS↔Rust interop.

**Out of scope / deferred:**
- `type_resolver.rs` (240 lines) — Type inference for JSON/collection schema. Phase 3 placeholder.
- `collation.rs` (606 lines) — Already covered by value/collatable.ts.
- `util/*` — IVec (plain arrays), SafeMap/SafeSet (plain Map/Set), ReadyChunks (simple async batching), Cast (already in value/cast.ts). Port as-needed.
- `model/tsify.rs` — WASM-specific, not needed for TS port.
- `property/value/pn_counter.rs`, `property/value/entity_ref.rs`, `property/value/json.rs` — Additional property types, can be ported later.
- `property/backend/pn_counter.rs` — PNCounter backend, not yet needed.
- `task.rs` (17 lines), `traits.rs` (21 lines) — Minimal utility types.

### Reactor — fully ported and tested, Node integration partially wired
The entire Reactor subsystem is complete: 8 source files + 9 tests. All three phases work end-to-end.
- **All 8 files ported**: update.ts, property-path.ts, comparison-index.ts, candidate-changes.ts, watcher_set.ts, fetch_gap.ts, subscription_state.ts (815 lines), subscription.ts, index.ts (499 lines)
- **All key methods present**: subscribe/unsubscribe, addQueryAndNotify/updateQueryAndNotify, notifyChange (3-phase), systemReset
- **9 tests passing**: ComparisonIndex (2), CandidateChanges (3), FetchGap (3), Reactor end-to-end (1)
- **Node integration status**: `reactor` field added to Node ✓. Phase 7 `reactor.notifyChange()` added ✓. `query()` added to TContext ✓, wired to `EntityLiveQuery.create()` ✓. All integration bugs fixed and committed.

### LiveQuery — ported and tested (committed, see Layer 6a/6b sections above)
The livequery.ts file (726 lines) has been created. It mirrors `ankurah/core/src/livequery.rs` (399 lines Rust). Key patterns:
- `EntityLiveQuery` (Arc<Inner>) → class EntityLiveQuery (plain instance)
- `LiveQuery<R>` wraps EntityLiveQuery + generic R type. Implements Signal, Get, Peek, Subscribe via delegation.
- `new()` spawns async init via fire-and-forget: `void me.activate(1).then(...)`
- `wait_initialized()` uses Promise-based Notify replacement (stored resolver pattern)
- RemoteQuerySubscriber stubbedDetailed spec: `specs/livequery-port-spec.md`

### Rust↔TS file mapping (remaining)
Files in ankurah/core/src/ that still need work:

| Rust file | Lines | TS status | Notes |
|-----------|-------|-----------|-------|
| livequery.rs | 399 | PORTED + TESTED (6 tests) | EntityLiveQuery, LiveQuery<V>, WeakEntityLiveQuery. Committed. |
| node.rs | 889 | PARTIAL (360 lines, committed) | Reactor field + Phase 7 + query() wired. No peers/networking. |
| context.rs | 389 | PARTIAL (156 lines, committed) | query() added to TContext. No subscribe(). |
| node_applier.rs | 296 | PORTED (422 lines, commit e86d01e) | applyUpdates (guarded), applyDeltas (ReadyChunks). |
| peer_subscription/client_relay.rs | 971 | **NOT PORTED** | Complex async state machine |
| peer_subscription/server.rs | 175 | **NOT PORTED** | Subscription handler |
| peer_subscription/mod.rs | ~50 | **NOT PORTED** | Module re-exports |
| type_resolver.rs | 240 | DEFERRED | Type inference, Phase 3 tech debt |
| task.rs | 17 | DEFERRED | Trivial utility |
| traits.rs | 21 | DEFERRED | Minimal traits (already mapped) |
| model/tsify.rs | N/A | OUT OF SCOPE | WASM-specific |
| value/wasm.rs | N/A | OUT OF SCOPE | WASM-specific |
| property/value/pn_counter.rs | N/A | DEFERRED | PNCounter property type |
| property/value/entity_ref.rs | N/A | DEFERRED | EntityRef property type |
| property/value/json.rs | N/A | DEFERRED | JSON property type |
| property/backend/pn_counter.rs | N/A | DEFERRED | PNCounter backend |
| util/ready_chunks.rs | N/A | NEEDED for node_applier | Async batching |
| util/* (rest) | N/A | NOT NEEDED | Already covered by TS primitives |

### Rust-side work (ts-port-support branch)
- ~~Bincode fixture generation tests~~ DONE (12 tests, 12 `.bin` files)
- ~~Yrs V2 fixture generation~~ DONE (6 tests, 6 `.bin` files in `proto/test_fixtures/yrs_v2/`)
- Integration test server binary
- Any spec cleanup PRs

### Core architecture notes (from research)
- **Entity**: `Arc<EntityInner>` → plain class in TS. `RwLock<EntityInnerState>` → plain mutable fields (JS single-threaded). `EntityKind.Primary | EntityKind.Transacted { trxAlive, upstream }`. `snapshot()` forks all backends for transaction isolation.
- **Transaction**: `AppendOnlyVec<Entity>` → regular array. `alive: Arc<AtomicBool>` → plain boolean. `commit()` generates events, validates via PolicyAgent, stores, relays to peers, notifies Reactor.
- **Reactor**: Three-phase notification. Phase 1: enumerate watchers (WatcherSet has index/wildcard/entity tiers). Phase 2: evaluate changes per subscription (filter entities against predicates, compute membership changes). Phase 3: broadcast ReactorUpdate to subscribers via signals.
- **defineModel() API**: Returns `{ View, Mutable, collection(), fields }`. View has typed getters returning projected types (via entity.getPropertyValue). Mutable has typed getters returning active type handles (via entity.getActiveHandle). Field helpers: `lww<T>()`, `yrsText()`, `ephemeral<T>()`. initializeNewEntity() delegates to entity.initializeProperty() per field.

## Rust Ownership → JS GC Translation (CRITICAL)

This section documents how Rust's reference-counted ownership paradigm (Arc, Weak, Drop) is translated to JavaScript's garbage-collected paradigm. Two formal exception rules govern all divergences:

- **E8 (Concurrency Primitives Eliminated)**: `Arc<T>`, `Rc<T>`, `RwLock<T>`, `Mutex<T>`, `AtomicBool`, `Send + Sync` → plain references, plain properties, plain booleans. JS is single-threaded.
- **E11 (Drop Semantics → Dispose Pattern)**: Rust `impl Drop` → explicit `dispose()` methods + `Symbol.dispose` (ES2023 `using` declarations). JS has no deterministic destructors.

### Pattern-by-Pattern Mapping

| Rust Construct | TS Equivalent | Deterministic? | Primary Risk |
|---|---|---|---|
| `Arc<T>` / `Rc<T>` | Plain reference | N/A (GC handles) | None |
| `Weak<T>` | `WeakRef<T>` | No (GC timing) | **Must ensure a strong ref exists elsewhere** |
| `impl Drop` | `dispose()` + `[Symbol.dispose]()` | Only if called | Leaks if caller forgets |
| `FinalizationRegistry` | Safety-net destructor | No (best-effort) | May never fire per spec |
| `RwLock<T>` / `Mutex<T>` | Plain property | N/A | Write guards need manual `done()` |
| `AtomicBool/U32` | `boolean` / `number` | N/A | None in single-threaded |
| Lifetime params (`'a`) | Runtime `alive` flag check | No (no auto-drop) | Dangling mutable if Transaction not committed/rolled back |

### Layered Defense Strategy

The codebase uses four layers of defense for lifetime management:

1. **Primary: explicit `dispose()` calls.** Every resource-owning type has a `dispose()` method. Tests always call it. API supports `[Symbol.dispose]()` for `using` declarations.
2. **Secondary: `FinalizationRegistry` safety net.** Used for `WeakEntitySet` (cleans stale map entries) and `liveQueryRegistry` (eventually calls `unsubscribe_remote_predicate`). These catch cases where dispose was forgotten.
3. **Structural: `WeakRef<T>`** for non-owning references that mirror Rust's `Weak<T>`. Prevents cycles and allows natural GC of unused objects.
4. **Runtime guards: boolean flags** (`disposed`, `alive.value`) checked at mutation points to catch use-after-dispose. These replace Rust's compile-time lifetime enforcement.

### Known Gotchas and Bugs Encountered

**1. WeakRef without strong holder (NodeLikeAdapter GC bug — fixed in bb58066)**
In `EntityLiveQuery`, a `NodeLikeAdapter` bridge object was created and passed to `QueryGapFetcher` which held only a `WeakRef` to it. Since no strong reference existed anywhere, the GC could collect it immediately, causing "Node has been dropped" errors during gap filling. Fix: added `private readonly _nodeLikeAdapter` field to `EntityLiveQuery` to hold a strong reference. **Lesson: in Rust, the `Arc` ownership graph guarantees the strong ref exists. In JS, you must manually ensure someone holds a strong reference for every WeakRef target.**

**2. Transaction alive flag — no auto-Drop**
Rust's `impl Drop for Transaction` automatically sets `alive = false` when the transaction goes out of scope. In TS, the `alive` flag is a shared `{ value: boolean }` object that only gets set to `false` when `commit()` or `rollback()` is explicitly called. If neither is called, forked entities remain writable indefinitely. **This is a genuine semantic gap.** Mitigated by test discipline (always commit/rollback) and documented in transaction.ts.

**3. ResultSetWrite.done() — must be called manually**
Rust's `MutexGuard` broadcasts changes on `Drop` when the write guard goes out of scope. TS uses an explicit `done()` method. Forgetting to call `done()` silently drops change notifications — no error, just silent data loss. Every call site in `subscription_state.ts` shows the pattern: `const rw = queryState.resultset.write(); rw.add(entity); rw.done();`

**4. ListenerGuard / SubscriptionGuard leak risk**
In Rust, dropping a `ListenerGuard` auto-unsubscribes the listener. In TS, if `dispose()` is never called, the listener stays registered forever — a classic JS memory leak. Tests explicitly verify idempotent dispose (signals/basic.test.ts).

**5. WeakRef timing non-determinism**
`WeakRef<T>.deref()` in JS is not deterministic about *when* the referent becomes unreachable. The spec allows engines to return a live reference even after all strong references are gone (within the same microtask turn). In Rust, `Weak::upgrade()` returns `None` the instant the last `Arc` is dropped. In practice this rarely causes bugs but means JS WeakRef is "weaker" in its guarantees.

### Rules for New Code

1. **Every `WeakRef` must have a corresponding strong reference holder.** Before creating a `WeakRef`, identify which object holds the strong reference and for how long. Document it with a comment.
2. **Every `dispose()` method must be idempotent.** Use a `disposed` boolean flag. Test idempotency.
3. **Every write guard (`ResultSetWrite`, etc.) must have `done()` called.** Consider `try/finally` patterns.
4. **Prefer `using` declarations** where the scope is clear: `using sub = reactor.subscribe(...)`. This provides Rust-like scope-based cleanup.
5. **`FinalizationRegistry` is a safety net, never a primary mechanism.** Always pair with explicit `dispose()`.
6. **Test lifecycle behavior explicitly.** Port all Rust Drop-related tests. Add tests that verify dispose prevents further notifications.

## Agent Working Notes

### Annotation format
- Line 1: `// MIRRORS: ankurah/<crate>/src/<path>.rs` — bare path only, NO extra text like `[E2]` or `(tests module)`
- Exception citations go on a SEPARATE line 2: `// Exception E12: file-with-submodules pattern`
- Test files: `// MIRRORS: ankurah/<crate>/src/<path>.rs` (same as source, bare path)
- TS-only files: `// TS-ONLY: <reason>`

### SOURCE-HASH annotations
New TS files should include `// SOURCE-HASH: <sha256>` on line 2 (after the MIRRORS annotation) with the SHA-256 hash of the Rust source file used as the basis for the port. This enables automated drift detection.

### Fixture path convention
- Rust fixtures: `/Users/daniel/ak/ankurah-ts-support/proto/test_fixtures/`
- TS tests resolve via: `path.resolve(__dirname, '../../../../ankurah-ts-support/proto/test_fixtures')`
- Hard-fail with descriptive error if fixture dir missing

### ULID serialization (confirmed)
- `ulid` crate v1.2.1 serializes as 26-char Crockford Base32 string (always, no `is_human_readable` check)
- In bincode: 8-byte u64 length (=26) + 26 ASCII bytes = 34 bytes total
- Deterministic test ULIDs constructed via: `"0000000000000000000000" + decimal(seed).padStart(4, '0')`

### Known patterns and workarounds
- **Circular dependency**: `context.ts` ↔ `transaction.ts` — resolved via `require('./transaction.ts')` in `context.ts:begin()` (inline import). This is the only circular dep in the codebase.
- **Rust→TS concurrency mapping**: `Arc<AtomicBool>` → `{ value: boolean }` (shared reference), `RwLock` → plain fields, `AppendOnlyVec` → array, `DashMap` → Map. All documented as Exception E8 in individual files.
- **`require('@ankurah/proto')` in node.ts**: Used for Attested constructor in commit pipeline to avoid top-level import issues. Should be cleaned up eventually.

### Rust source hash manifest (drift detection)
- A hash manifest at `scripts/rust-source-hashes.json` tracks SHA-256 hashes of 155 files (85 source + 70 test)
- The audit script now checks for drift: if a Rust file changes since last port, the audit warns
- **After porting Rust changes to TS**: run `bun run scripts/audit-port.ts --update-manifest` to record the new hashes
- **To bootstrap from scratch**: run `bun run scripts/audit-port.ts --backpopulate` to scan all MIRRORS annotations and compute current hashes
- The manifest file should be committed alongside TS changes

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
8. **You are the SUPERVISOR, not the implementor** — **THIS IS NON-NEGOTIABLE.** Never write implementation code yourself. Never run `tsc` or `bun test` yourself. Never read large Rust source files yourself. Dispatch `Task` agents with `run_in_background: true` for ALL of these. Your job is ONLY: dispatch agents → wait for results → fix small integration conflicts → commit. See "Supervision Model" section above. This is not a suggestion — every line of code you write inline, every test you run directly, every file you read consumes context that burns through the window and forces a `/clear`. Background agents get their own fresh context windows — use them aggressively.
9. **Dispatch agents in parallel** — Always launch 3-6 independent agents simultaneously. Never wait for one agent to finish before dispatching unrelated work. The proven pattern is: research agents first (parallel) → implementation agents (parallel per sub-layer) → verification agent (background) → commit.
10. **After context rewinds** — Re-read this file first. Dispatch a background agent to run `bun test` and `tsc` to verify current state. Check `/private/tmp/claude-501/-Users-daniel-ak/tasks/*.output` for recent agent outputs if needed. Then continue from the "Next up" list.
