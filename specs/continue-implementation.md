# ankurah-ts Implementation Continuation

**Purpose**: This document provides sufficient context for a fresh agent to continue the ankurah-ts implementation. Read this first, then `architectural-decisions.md`, then `port-rules.md`, then refer to other specs as needed.

**Last updated**: 2026-02-10

**Status**: Proto, signals, ankql fully done. Yrs↔Yjs V2 interop validated. Core Layers 0-4 fully implemented. Layer 5a (Reactor sub-layer: foundational types) implemented — update.ts, property-path.ts, comparison-index.ts, candidate-changes.ts, selection/filter.ts. Next: Layer 5b (WatcherSet, fetch-gap, EntityResultSet), then 5c (Subscription, Reactor main), then supporting types + storage engines/connectors.

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
- **`@ankurah/core` Layer 5a — Reactor foundational types** — ReactorUpdate/ReactorUpdateItem/MembershipChange (reactor/update.ts), PropertyPath with value extraction (reactor/property-path.ts), ComparisonIndex with sorted-array BTreeMap substitute for gt/lt range queries (reactor/comparison-index.ts), CandidateChanges zero-copy wrapper with per-query and entity offset maps (reactor/candidate-changes.ts), Filterable interface + evaluatePredicate recursive predicate evaluator (selection/filter.ts). 5 source files. Also fixed tsc errors in node.test.ts (CollectionId branded type + OpenPolicy arg count).

### Core source files (30 total in packages/core/src/)
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
src/reactor/update.ts                — MembershipChange, ReactorUpdate, ReactorUpdateItem
src/reactor/property-path.ts         — PropertyPath (field path with JSON sub-path support)
src/reactor/comparison-index.ts      — ComparisonIndex<T> (field-level index for watcher matching)
src/reactor/candidate-changes.ts     — CandidateChanges<C>, QueryCandidate<C> (zero-copy change wrapper)
src/selection/filter.ts              — Filterable interface, evaluatePredicate()
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
1. **`@ankurah/core` Layer 5b — Reactor mid-layer** — 3 files needed next:
   - `reactor/watcherset.ts` — WatcherSet (three registries: index/wildcard/entity watchers, uses ComparisonIndex + PropertyPath). Central dispatch for ENUMERATE phase.
   - `reactor/fetch-gap.ts` — GapFetcher trait + QueryGapFetcher, build_continuation_predicate, infer_value_type_for_field.
   - `resultset.ts` — EntityResultSet (sorted order + index + key_spec + limit + gap_dirty, write/read guards).
2. **`@ankurah/core` Layer 5c — Reactor top-layer** — 3 files:
   - `reactor/subscription-state.ts` — Subscription internal state (queries, entity_subscriptions, entities cache, evaluate_changes).
   - `reactor/subscription.ts` — ReactorSubscription public handle (subscribe/unsubscribe, auto-cleanup).
   - `reactor/reactor.ts` — Reactor main coordinator (three-phase notify_change pipeline, add_query, update_query).
3. **`@ankurah/core` Layer 5 tests** — Port 9 unit tests: 2 comparison-index, 3 candidate-changes, 3 fetch-gap, 1 reactor end-to-end.
4. **`@ankurah/core` supporting types** — Lineage, LiveQuery, ResultSet (typed wrapper), ChangeSet, PeerSubscription.
5. **Node + Context integration** — Add `reactor` field to Node, wire Phase 7 in commitLocalTrx(), add `query()` method to Context.
6. **Storage engines** — storage-common traits, then expo-sqlite and better-sqlite3
7. **Connectors** — WebSocket client, local connector
8. **`@ankurah/react`** — Absorb ankurah-react-hooks, wire to TS signals
9. **Integration tests** — Spawn Rust WS servers, test TS↔Rust interop

### Reactor research completed
All research for the Reactor subsystem has been done. Key findings:
- **Reactor structure**: 9 Rust files (reactor.rs + 7 in reactor/ + resultset.rs + livequery.rs). Three-phase notification: enumerate (WatcherSet) → evaluate (Subscription) → notify (broadcast ReactorUpdate).
- **WatcherSet**: Three registries — index watchers (field-specific via ComparisonIndex), wildcard watchers (collection-wide for Predicate::True), entity watchers (explicit entity subscriptions). `accumulate_interested_watchers()` is the hot path.
- **ComparisonIndex**: Sorted-array substitute for BTreeMap. Handles eq/ne/gt/lt range queries on collatable bytes. Already ported.
- **EntityResultSet**: Ordered entity list with HashMap index. Supports ORDER BY sort keys, LIMIT, gap_dirty flag. Write guard does sorted insert/remove. Read guard for consistent reads. Broadcasts on mutation.
- **Subscription internal state**: Holds queries map, entity_subscriptions set, entities cache. `evaluate_changes()` is the core — for each candidate, evaluate predicate, compare to resultset membership, compute MembershipChange (Initial/Add/Remove), update resultset, accumulate ReactorUpdateItems, generate deferred WatcherChanges.
- **GapFetcher**: Fills gaps when LIMIT-constrained queries lose members. Builds continuation predicates from ORDER BY.
- **Node integration**: Node needs `reactor` field. `commitLocalTrx()` Phase 7 calls `reactor.notifyChange(entityChanges)`. Context needs `query()` method returning LiveQuery.
- **TS simplifications**: No Arc/Mutex (JS single-threaded). notify_lock → async queue/promise chain. BTreeMap → sorted array. IVec → plain array. Drop → dispose()/Symbol.dispose. tokio::spawn → fire-and-forget async.

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

### Known patterns and workarounds
- **Circular dependency**: `context.ts` ↔ `transaction.ts` — resolved via `require('./transaction.ts')` in `context.ts:begin()` (inline import). This is the only circular dep in the codebase.
- **Rust→TS concurrency mapping**: `Arc<AtomicBool>` → `{ value: boolean }` (shared reference), `RwLock` → plain fields, `AppendOnlyVec` → array, `DashMap` → Map. All documented as Exception E8 in individual files.
- **`require('@ankurah/proto')` in node.ts**: Used for Attested constructor in commit pipeline to avoid top-level import issues. Should be cleaned up eventually.

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
