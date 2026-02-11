# ankurah-ts Progress & Parallelism Review

**Date**: 2026-02-11
**Reviewer**: Automated analysis of project state

---

## 1. Current Progress Summary

### Completed Packages

| Package | TS Files | Rust Files | Status | Notes |
|---------|----------|------------|--------|-------|
| `@ankurah/proto` | 16 source + 1 test | N/A (proto crate) | **100% done** | 24 fixture parity tests, 244 assertions |
| `@ankurah/signals` | 10 source + 4 stubs + 4 tests | ~16 files | **~85% done** | Calculated, Map, Memo, Observer auto-tracking deferred |
| `@ankurah/ankql` | 8 source + 4 tests | PEG grammar crate | **100% done** | 76 tests, hand-written recursive descent parser |
| `@ankurah/core` | 29 source + 1 in-progress + 6 tests | 63 Rust files | **~46% done** | Layers 0-5a complete, Layer 5b started |
| `@ankurah/storage-common` | 1 stub | 8 Rust files | **0% done** | Stub index.ts only |
| `@ankurah/storage-memory` | 1 stub | No Rust equivalent | **0% done** | TS-only package, stub only |
| `@ankurah/connector-websocket` | 1 stub | 3 Rust files | **0% done** | Stub index.ts only |
| `@ankurah/connector-local` | 1 stub | 1 Rust file (107 lines) | **0% done** | Stub index.ts only |
| `@ankurah/react` | 1 stub | N/A (TS-only) | **0% done** | Reference code exists in ankurah-react-hooks |
| `@ankurah/storage-better-sqlite3` | Exists (not checked) | N/A (port of sqlite crate) | **0% done** | Stub only |
| `@ankurah/storage-expo-sqlite` | Exists (not checked) | N/A (TS-only) | **0% done** | Stub only |

### Overall Progress: ~45% of core, ~35% of total project

### Test Counts
- **309 total tests passing**, 793 assertions across 15 test files
- Proto: 24 tests (244 assertions)
- Signals: 45 tests
- AnkQL: 76 tests
- Yrs/Yjs V2 interop: 10 tests (18 assertions)
- Core backends: 55+ tests
- Core entity: 30 tests
- Core transaction: 33 tests
- Core node: 36 tests

---

## 2. Layer 5b Status (Currently In Progress)

Three files are part of Layer 5b. Current state:

| File | Exists? | Status | Rust Source Lines |
|------|---------|--------|-------------------|
| `reactor/watcherset.ts` | **No** | Not started | 266 lines |
| `reactor/fetch_gap.ts` | **Yes** (218 lines) | **Complete** | ~200 lines |
| `resultset.ts` | **No** | Not started | 918 lines |
| `indexing/key_spec.ts` | **No** (empty dir exists) | Not started | 383 lines |
| `indexing/encoding.ts` | **No** | Not started | 221 lines |

**Summary**: `fetch_gap.ts` is fully implemented. `watcherset.ts`, `resultset.ts`, and the `indexing/` module still need porting. The indexing module is a dependency of `resultset.ts` (EntityResultSet uses `encode_tuple_values_with_key_spec` and `KeySpec`).

### Dependency chain within 5b:
- `indexing/encoding.ts` + `indexing/key_spec.ts` -- no internal deps beyond core L0 types
- `resultset.ts` -- depends on `indexing/` (KeySpec, encode), Entity, Signals (Broadcast)
- `reactor/watcherset.ts` -- depends on ComparisonIndex, PropertyPath, CandidateChanges (all Layer 5a, already done)

---

## 3. Remaining Core Work (File-by-File)

### Already Ported (29 TS files mapping to Rust files):

| TS File | Rust Counterpart | Layer |
|---------|-----------------|-------|
| `error.ts` | `error.rs` | L0 |
| `value/index.ts` | `value/mod.rs` | L0 |
| `value/cast.ts` | `value/cast.rs` | L0 |
| `value/cast_predicate.ts` | `value/cast_predicate.rs` | L0 |
| `value/collatable.ts` | `value/collatable.rs` | L0 |
| `property/index.ts` | `property/mod.rs` | L0 |
| `property/traits.ts` | `property/traits.rs` | L0 |
| `property/backend/index.ts` | `property/backend/mod.rs` | L0 |
| `property/backend/lww.ts` | `property/backend/lww.rs` | L1 |
| `property/backend/yjs.ts` | `property/backend/yrs.rs` | L1 |
| `property/value/lww.ts` | `property/value/lww.rs` | L1 |
| `property/value/yrs_string.ts` | `property/value/yrs.rs` | L1 |
| `model.ts` | `model.rs` | L1 |
| `define-model.ts` | Derive macro equivalent | L1 |
| `entity.ts` | `entity.rs` | L2 |
| `transaction.ts` | `transaction.rs` | L3 |
| `context.ts` | `context.rs` | L3 |
| `changes.ts` | `changes.rs` | L3 |
| `node.ts` | `node.rs` | L4 |
| `storage.ts` | `storage.rs` | L4 |
| `policy.ts` | `policy.rs` | L4 |
| `reactor/update.ts` | `reactor/update.rs` | L5a |
| `reactor/property-path.ts` | `reactor/property_path.rs` | L5a |
| `reactor/comparison-index.ts` | `reactor/comparison_index.rs` | L5a |
| `reactor/candidate-changes.ts` | `reactor/candidate_changes.rs` | L5a |
| `selection/filter.ts` | `selection/filter.rs` | L5a |
| `reactor/fetch_gap.ts` | `reactor/fetch_gap.rs` | L5b (done) |
| `index.ts` | `lib.rs` | All |

### Not Yet Ported (needs implementation):

#### Layer 5b (Reactor mid-layer) -- 4 files remaining

| Target TS File | Rust Source | Lines | Complexity | Notes |
|----------------|------------|-------|------------|-------|
| `indexing/encoding.ts` | `indexing/encoding.rs` | 221 | Medium | Tuple value encoding for sort keys |
| `indexing/key_spec.ts` | `indexing/key_spec.rs` | 383 | Medium | KeySpec, IndexKeyPart, IndexDirection, NullsOrder |
| `reactor/watcherset.ts` | `reactor/watcherset.rs` | 266 | Medium | Three registries, accumulate_interested_watchers hot path |
| `resultset.ts` | `resultset.rs` | 918 | **High** | EntityResultSet + ResultSet, sorted insert, IVec, Broadcast, guards |

#### Layer 5c (Reactor top-layer) -- 3 files

| Target TS File | Rust Source | Lines | Complexity | Notes |
|----------------|------------|-------|------------|-------|
| `reactor/subscription_state.ts` | `reactor/subscription_state.rs` | 721 | **High** | QueryState, evaluate_changes core logic, MembershipChange |
| `reactor/subscription.ts` | `reactor/subscription.rs` | 108 | Low | ReactorSubscription handle, ReactorSubscriptionId, Drop cleanup |
| `reactor/index.ts` (reactor main) | `reactor.rs` | 629 | **High** | Reactor coordinator, three-phase notify_change, add_query, subscribe |

#### Supporting types -- 8 files

| Target TS File | Rust Source | Lines | Complexity | Notes |
|----------------|------------|-------|------------|-------|
| `connector.ts` | `connector.rs` | 60 | Low | PeerSender, NodeComms, SendError |
| `collectionset.ts` | `collectionset.rs` | 64 | Low | CollectionSet cache with lazy creation |
| `livequery.ts` | `livequery.rs` | 399 | **High** | EntityLiveQuery, typed LiveQuery<V>, async init, gap fetching |
| `lineage.ts` | `lineage.rs` | 1004 | **Very High** | EventAccumulator, DAG ordering, lineage traversal, Retrieve |
| `retrieval.ts` | `retrieval.rs` | 325 | High | TEvent, TClock, GetEvents, LocalRetriever, Retrieve impl |
| `schema.ts` | `schema.rs` | 9 | Trivial | CollectionSchema trait |
| `query_value.ts` | `query_value.rs` | 79 | Low | QueryValue enum for FFI parameter substitution |
| `system.ts` | `system.rs` | 316 | High | SystemManager, catalog management, root clock |

#### Deferred / Possibly out of scope -- 6 files

| Target TS File | Rust Source | Lines | Complexity | Notes |
|----------------|------------|-------|------------|-------|
| `node_applier.ts` | `node_applier.rs` | 296 | High | Remote update application, needs connector infra |
| `peer_subscription/index.ts` | `peer_subscription/mod.rs` | 5 | Trivial | Module re-exports |
| `peer_subscription/client_relay.ts` | `peer_subscription/client_relay.rs` | 971 | **Very High** | Client-side subscription relay, complex async state machine |
| `peer_subscription/server.ts` | `peer_subscription/server.rs` | 175 | Medium | Server-side subscription handling |
| `type_resolver.ts` | `type_resolver.rs` | 240 | Medium | Runtime type resolution |
| `property/value/json.ts` | `property/value/json.rs` | (small) | Low | Json active type |

#### Rust-only / Not applicable to TS port

| Rust File | Lines | Reason |
|-----------|-------|--------|
| `value/wasm.rs` | (small) | WASM-specific, not needed |
| `model/tsify.rs` | (small) | Tsify derive, not needed |
| `property/backend/pn_counter.rs` | (commented out) | Not in scope (commented out in Rust) |
| `property/value/pn_counter.rs` | (commented out) | Not in scope |
| `property/value/entity_ref.rs` | (small) | Ref active type -- may be deferred |
| `task.rs` | 17 | Just `tokio::spawn` wrapper -- TS uses `queueMicrotask`/`setTimeout` |
| `traits.rs` | 21 | Namespace trait -- may not be needed for Phase 1 |
| `util/*.rs` | ~250 total | Rust-specific utils (IVec, SafeMap, ReadyChunks) -- not 1:1 needed |

---

## 4. Dependency Graph

```
Layer 0: error, value/*, property/traits, property/backend/index
    |
Layer 1: backend/lww, backend/yjs, property/value/*, model, define-model
    |
Layer 2: entity (EntityKind, WeakEntitySet)
    |
Layer 3: transaction, context, changes
    |
Layer 4: node, storage, policy
    |
Layer 5a: reactor/update, reactor/property-path, reactor/comparison-index,
          reactor/candidate-changes, selection/filter
    |
Layer 5b: indexing/encoding + indexing/key_spec (independent)
          reactor/watcherset (needs 5a)
          resultset (needs indexing/*, signals, entity)
          reactor/fetch_gap (DONE, needs entity, ankql, value)
    |
Layer 5c: reactor/subscription_state (needs 5b: watcherset, resultset, candidate-changes, filter)
          reactor/subscription (needs subscription_state, Reactor ref, signals)
          reactor/index (Reactor main) (needs subscription_state, subscription, watcherset)
    |
Layer 6: Supporting types
          schema (trivial, needs value)
          query_value (needs ankql, proto)
          connector (needs proto, policy, storage, node)
          collectionset (needs storage, error)
          lineage (needs retrieval, proto) -- **complex DAG logic**
          retrieval (needs proto, storage, error)
          system (needs collectionset, entity, WeakEntitySet, reactor, property)
          livequery (needs reactor/subscription, resultset, fetch_gap, signals, node, model)
    |
Layer 7: Node + Context integration
          Add reactor field to Node
          Wire Phase 7 in commitLocalTrx() (notify reactor)
          Add query() method to Context
          Add node_applier (remote update handling)
    |
Layer 8: Storage engines
          storage-common (needs proto, core error types, ankql)
          storage-memory (needs storage-common)
          storage-better-sqlite3 (needs storage-common)
          storage-expo-sqlite (needs storage-common)
    |
Layer 9: Connectors
          connector-local (needs core connector, node, proto)
          connector-websocket (needs core connector, proto, bincode)
    |
Layer 10: React
          react (needs signals, core)
    |
Layer 11: Integration tests
          End-to-end with Rust WS server
```

---

## 5. Parallelization Opportunities

### Wave 1 -- Currently in progress / immediate (Layer 5b completion)

These have no dependencies on each other and can run in parallel:

| Agent | Task | Depends On | Blocks |
|-------|------|------------|--------|
| A1 | `indexing/encoding.ts` + `indexing/key_spec.ts` | L0 Value, collation | resultset.ts |
| A2 | `reactor/watcherset.ts` | L5a (done) | subscription_state, reactor main |
| A3 | `resultset.ts` | indexing/* (wait for A1), Entity, Signals | subscription_state, livequery |

**Parallelism**: A1 and A2 can run simultaneously. A3 must wait for A1 to complete.

### Wave 2 -- Layer 5c (after Wave 1)

| Agent | Task | Depends On | Blocks |
|-------|------|------------|--------|
| B1 | `reactor/subscription.ts` | Signals (done), needs Reactor type ref | reactor main |
| B2 | `reactor/subscription_state.ts` | watcherset (A2), resultset (A3), filter (done) | reactor main |
| B3 | `schema.ts` + `query_value.ts` | Value (done), ankql (done) | Nothing critical |
| B4 | `connector.ts` + `collectionset.ts` | Storage interfaces (done), proto (done) | Node integration, connectors |

**Parallelism**: B1, B3, B4 can start immediately after Wave 1. B2 needs A2 + A3 output. B1 is small (108 lines Rust) and can be fast-tracked.

### Wave 3 -- Reactor main + supporting types

| Agent | Task | Depends On | Blocks |
|-------|------|------------|--------|
| C1 | `reactor/index.ts` (Reactor main) | B1 + B2 | Node integration, livequery |
| C2 | `retrieval.ts` | Proto, storage (done) | lineage |
| C3 | `storage-common` package (8 Rust files) | Proto, core errors, ankql | storage-memory, sqlite engines |

**Parallelism**: C2 and C3 are independent of reactor work and can run in parallel with Wave 2. C1 must wait for Wave 2.

### Wave 4 -- Integration and remaining types

| Agent | Task | Depends On | Blocks |
|-------|------|------------|--------|
| D1 | `lineage.ts` | C2 (retrieval) | System manager integration |
| D2 | `system.ts` | Collectionset (B4), entity, reactor (C1) | Full Node integration |
| D3 | `livequery.ts` | Reactor (C1), resultset (A3), fetch_gap (done), signals | Context.query() |
| D4 | `storage-memory` package | C3 (storage-common) | Testing infrastructure |
| D5 | Node + Context integration (add reactor, wire Phase 7, add query()) | C1, D3 | End-to-end |
| D6 | `connector-local` package | B4 (connector.ts) | Integration tests |

**Parallelism**: D1 and D4 can start as soon as their deps land. D2, D3, D5 form the critical path.

### Wave 5 -- Connectors, React, advanced features

| Agent | Task | Depends On | Blocks |
|-------|------|------------|--------|
| E1 | `connector-websocket` | B4 (connector), proto bincode | Cross-node tests |
| E2 | `@ankurah/react` package | Signals (done), core types | React app integration |
| E3 | `node_applier.ts` | Connector infra, lineage, Node | Remote sync |
| E4 | `peer_subscription/` | Connector, reactor, node_applier | Full P2P sync |
| E5 | `storage-better-sqlite3` | C3 (storage-common) | Node.js persistence |
| E6 | `storage-expo-sqlite` | C3 (storage-common) | Expo Go persistence |
| E7 | Layer 5 tests (9 unit tests from Rust) | Waves 1-3 complete | Quality gate |

**Parallelism**: E1, E2, E5, E6 are independent and can run in parallel. E2 can start at any time (only needs signals + core types). E7 can start once Wave 3 completes.

### Wave 6 -- Integration tests

| Agent | Task | Depends On | Blocks |
|-------|------|------------|--------|
| F1 | Integration tests (TS <-> Rust WS server) | E1, E3, E4, D5 | Release |
| F2 | End-to-end test suite | All above | Release |

---

## 6. Non-Core Packages

### `@ankurah/storage-common` (8 Rust files -> ~5-6 TS files)

| Rust File | Lines | TS Target | Complexity |
|-----------|-------|-----------|------------|
| `lib.rs` | 13 | `index.ts` | Trivial (re-exports) |
| `traits.rs` | 67 | `traits.ts` | Medium (stream traits -> async iterators) |
| `types.rs` | ? | `types.ts` | Low |
| `bounds.rs` | ? | `bounds.ts` | Medium |
| `filtering.rs` | ? | `filtering.ts` | Medium |
| `sorting.rs` | ? | `sorting.ts` | Medium |
| `planner.rs` | ? | `planner.ts` | High (query planning) |
| `predicate.rs` | ? | `predicate.ts` | Medium |

**Key adaptation**: Rust uses `Stream` (async iterator). TS will use `AsyncIterable` or plain `Promise<T[]>` arrays. The stream-oriented API may simplify significantly in TS.

**Dependencies**: Needs `@ankurah/proto`, `@ankurah/core` (EntityId, error types), `@ankurah/ankql` (Predicate, OrderByItem).

### `@ankurah/storage-memory` (TS-only, no Rust counterpart)

A simple in-memory implementation of `StorageEngine` / `StorageCollection`. Needs:
- Implement `StorageEngine` interface with `Map<CollectionId, MemoryCollection>`
- Each `MemoryCollection` stores `Map<EntityId, EntityState>` (or equivalent)
- Implement query evaluation via `evaluatePredicate` from core
- Estimated: 1-2 files, ~200-300 lines

**Dependencies**: `@ankurah/storage-common`, `@ankurah/core` (storage interfaces).

### `@ankurah/connector-websocket` (3 Rust files -> ~2-3 TS files)

| Rust File | Lines | Notes |
|-----------|-------|-------|
| `lib.rs` | (small) | Re-exports |
| `client.rs` | ? | WebSocket client with reconnection |
| `sender.rs` | ? | PeerSender implementation |

**Key adaptation**: Rust uses `tokio-tungstenite`. TS uses browser/RN `WebSocket` API. Reconnection with exponential backoff. Bincode encode/decode for messages.

**Dependencies**: `@ankurah/core` (connector.ts, PeerSender), `@ankurah/proto` (NodeMessage bincode).

### `@ankurah/connector-local` (1 Rust file, 107 lines -> ~1 TS file)

Simple in-process connector using direct function calls or event emitters instead of Tokio channels. For testing multi-node scenarios.

**Dependencies**: `@ankurah/core` (connector.ts, Node).

### `@ankurah/react` (TS-only, reference from ankurah-react-hooks)

The existing `ankurah-react-hooks` package (194 lines) provides the template. Needs adaptation:
- Replace `ReactObserverInterface` (UniFFI/WASM) with TS signals observer
- Keep `createAnkurahReactHooks` factory pattern
- Keep `useObserve` + `signalObserver` HOC
- Wire to `@ankurah/signals` instead of foreign bindings

**Dependencies**: `@ankurah/signals`, `react` (peer dep). Core types needed for typed query hooks.

### `@ankurah/storage-better-sqlite3` + `@ankurah/storage-expo-sqlite`

Both port from the Rust `storage/sqlite` crate (6 files: connection, engine, error, lib, sql_builder, value). The SQL builder and query planning logic is the complex part.

**Dependencies**: `@ankurah/storage-common`, respective SQLite libraries.

---

## 7. Critical Path

The critical path to a working end-to-end system (local, single-node with queries):

```
Current state (Layers 0-5a done, fetch_gap done)
    |
[1] indexing/encoding.ts + indexing/key_spec.ts  (~1 agent session)
    |
[2] resultset.ts  (~1 agent session, complex)
    +-- reactor/watcherset.ts  (parallel with [1], ~1 agent session)
    |
[3] reactor/subscription_state.ts  (~1 agent session, complex)
    +-- reactor/subscription.ts  (fast, ~0.5 agent session)
    |
[4] reactor/index.ts (Reactor main)  (~1 agent session, complex)
    |
[5] livequery.ts  (~1 agent session, complex)
    +-- connector.ts + collectionset.ts  (parallel, fast)
    |
[6] Node + Context integration (add reactor, wire Phase 7, add query())  (~1 agent session)
    |
[7] storage-common  (~1-2 agent sessions)
    |
[8] storage-memory  (~1 agent session)
    |
=== MILESTONE: Single-node local queries working ===
    |
[9] connector-local  (~1 agent session)
    |
[10] connector-websocket  (~1-2 agent sessions)
    +-- node_applier + peer_subscription  (~2-3 agent sessions, complex)
    |
=== MILESTONE: Multi-node sync working ===
    |
[11] @ankurah/react  (~1 agent session)
    |
=== MILESTONE: React integration working ===
```

**Critical path length**: ~10-12 agent sessions in sequence (steps 1-8), but with parallelism this can be compressed to ~7-8 elapsed sessions.

---

## 8. Estimated Agent Dispatches

### Core package completion

| Wave | Parallel Agents | Tasks | Sessions |
|------|----------------|-------|----------|
| Wave 1 | 3 | indexing (2 files), watcherset, resultset | 2-3 |
| Wave 2 | 4 | subscription_state, subscription, schema+query_value, connector+collectionset | 2-3 |
| Wave 3 | 3 | reactor main, retrieval, storage-common | 2-3 |
| Wave 4 | 4-5 | lineage, system, livequery, storage-memory, Node integration | 3-4 |
| Wave 5 | 4-5 | reactor tests, connector-local, connector-websocket, react, node_applier | 3-4 |
| Wave 6 | 2-3 | peer_subscription, SQLite engines, integration tests | 3-4 |

### Summary

| Category | Estimated Agent Sessions |
|----------|------------------------|
| Core reactor completion (5b + 5c) | 6-8 |
| Core supporting types | 5-7 |
| Node/Context integration | 2-3 |
| Storage engines | 4-6 |
| Connectors | 3-4 |
| React | 1-2 |
| Tests (reactor + integration) | 3-5 |
| **Total** | **24-35 agent sessions** |

With aggressive parallelism (3-5 agents per wave), the elapsed calendar time is approximately **6-8 wave cycles** of supervisor orchestration.

---

## 9. Risk Areas

### Very High Risk (>500 lines Rust, complex logic)

1. **`lineage.ts`** (1004 lines Rust) -- Complex DAG traversal with EventAccumulator, partial descent detection, meet computation. The most algorithmic file in the codebase. Needs careful porting of the ordering logic (Equal, Descends, NotDescends, Incomparable, Partial).

2. **`resultset.ts`** (918 lines Rust) -- EntityResultSet with sorted insert/remove, IVec optimization, write/read guards (RwLock -> simplified in TS but still complex), Broadcast integration, LIMIT enforcement, gap_dirty tracking. The `ResultSet<R: View>` wrapper adds typed access.

3. **`peer_subscription/client_relay.ts`** (971 lines Rust) -- Complex async state machine for managing subscriptions across nodes. Uses Tokio channels, JoinHandles, and intricate lifecycle management. The TS adaptation (Tokio -> async/await + event emitters) is non-trivial.

### High Risk (300-700 lines, moderate complexity)

4. **`reactor/subscription_state.ts`** (721 lines) -- The `evaluate_changes()` method is the core of the reactor. Complex iteration over candidate changes, predicate evaluation, membership comparison, resultset mutation, and deferred watcher changes.

5. **`reactor/index.ts` (reactor.rs)** (629 lines) -- The `notify_change()` three-phase pipeline with Mutex → async queue adaptation. `add_query()` / `update_query()` with predicate index management.

6. **`livequery.ts`** (399 lines) -- Bridges reactor subscriptions to user-facing LiveQuery API. Uses `tokio::sync::Notify` (-> Promise/event), atomic version tracking, selection mutation. Generic over View type.

7. **`node_applier.ts`** (296 lines) -- Remote update application with event attestation checking. Needs ReadyChunks stream utility adaptation.

### Medium Risk (potential gotchas)

8. **`indexing/key_spec.ts`** (383 lines) -- KeySpec parsing from ORDER BY, IndexDirection, NullsOrder. Feeds into resultset sorted operations. Must match Rust encoding exactly for cross-node compatibility.

9. **`system.ts`** (316 lines) -- SystemManager for catalog management. Uses OnceLock (-> lazy init), RwLock (-> plain Map), CollectionSet. Durable vs ephemeral behavior.

10. **Reactor test porting** -- 9 Rust tests to port. Tests exercise the three-phase notification pipeline end-to-end. May need mock infrastructure not yet built.

### Low Risk (but easy to forget)

11. **`property/value/json.ts`** -- Json active type not yet ported. Needed for JSON sub-path support in reactor property paths.
12. **Re-exports / index.ts updates** -- Every new file needs index.ts export entries. Concurrent agents may conflict on this file.
13. **Circular dependency management** -- The existing context.ts <-> transaction.ts circular dep is handled. New files (livequery, system) may introduce new circular deps.

---

## Appendix: Complete Rust Core Files Not Yet Ported

```
NOT PORTED (needs implementation):
  core/src/indexing/encoding.rs         221 lines  [Wave 1]
  core/src/indexing/key_spec.rs         383 lines  [Wave 1]
  core/src/indexing/mod.rs                5 lines  [Wave 1]
  core/src/reactor/watcherset.rs        266 lines  [Wave 1]
  core/src/resultset.rs                 918 lines  [Wave 1]
  core/src/reactor/subscription_state.rs 721 lines [Wave 2]
  core/src/reactor/subscription.rs      108 lines  [Wave 2]
  core/src/reactor.rs                   629 lines  [Wave 3]
  core/src/connector.rs                  60 lines  [Wave 2]
  core/src/collectionset.rs              64 lines  [Wave 2]
  core/src/schema.rs                      9 lines  [Wave 2]
  core/src/query_value.rs                79 lines  [Wave 2]
  core/src/retrieval.rs                 325 lines  [Wave 3]
  core/src/lineage.rs                  1004 lines  [Wave 4]
  core/src/system.rs                    316 lines  [Wave 4]
  core/src/livequery.rs                 399 lines  [Wave 4]
  core/src/node_applier.rs              296 lines  [Wave 5]
  core/src/peer_subscription/mod.rs       5 lines  [Wave 5]
  core/src/peer_subscription/client_relay.rs 971 lines [Wave 5]
  core/src/peer_subscription/server.rs  175 lines  [Wave 5]
  core/src/type_resolver.rs             240 lines  [Wave 5]

DEFERRED / NOT APPLICABLE:
  core/src/value/wasm.rs                (WASM-specific)
  core/src/model/tsify.rs              (Tsify derive)
  core/src/property/backend/pn_counter.rs (commented out in Rust)
  core/src/property/value/pn_counter.rs (commented out)
  core/src/property/value/entity_ref.rs (may defer to later phase)
  core/src/property/value/json.rs       (needed but lower priority)
  core/src/task.rs                      (trivial, TS uses queueMicrotask)
  core/src/traits.rs                    (Namespace trait, may not need)
  core/src/util/*.rs                    (Rust-specific utilities)

TOTAL UNPORTED: ~6,020 lines of Rust across 21 files
TOTAL DEFERRED: ~8 files (Rust-specific or commented out)
```
