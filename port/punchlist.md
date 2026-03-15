# Port Punchlist

Outstanding work for the ankurah-ts port. Updated 2026-03-15.

---

## Rust-side (ankurah-ts-support branch)

- [ ] Rebase ts-port-support onto main (main has new proto types: EntityIdRange, SubscribeEntity, EntitiesSubscribed, UnsubscribeEntities)
- [ ] Add fixtures for new proto types after rebase
- [ ] Verify all 22+ fixture tests pass after rebase

## Drift (8 Rust files changed since last port)

- [ ] `proto/src/message.rs` — new `UnsubscribeEntities` variant
- [ ] `proto/src/request.rs` — new `EntityIdRange`, `SubscribeEntity`, `EntitiesSubscribed`
- [ ] `core/src/context.rs` — changes unknown, diff needed
- [ ] `core/src/node.rs` — changes unknown, diff needed
- [ ] `core/src/reactor/subscription.rs` — changes unknown, diff needed
- [ ] `core/src/reactor/subscription_state.rs` — changes unknown, diff needed
- [ ] `tests/tests/inter_node.rs` — changes unknown, diff needed
- [ ] `tests/tests/system.rs` — changes unknown, diff needed

## Core port — remaining files (80 audit failures)

### Layer 7 — Networking
- [ ] `peer_subscription/mod.rs` → `peer_subscription/index.ts`
- [ ] `peer_subscription/client_relay.rs` → `peer_subscription/client_relay.ts` (971 lines, 5-state machine)
- [ ] `peer_subscription/server.rs` → `peer_subscription/server.ts`
- [ ] Complete `node.ts` networking (PeerState, register/deregister peer, handleMessage, RPC correlation)
- [ ] Complete `context.ts` (subscribe method)

### Missing core files
- [ ] `traits.rs` → `traits.ts` (21 lines)
- [ ] `task.rs` → `task.ts` (17 lines)
- [ ] `type_resolver.rs` → `type_resolver.ts` (240 lines, could defer)
- [ ] `collation.rs` → `collation.ts` (606 lines, may be covered by value/collatable.ts)
- [ ] `selection/mod.rs` → `selection/index.ts`
- [ ] `model.rs` → `model/index.ts` (file-with-submodules)

### Missing property value types
- [ ] `property/value/mod.rs` → `property/value/index.ts`
- [ ] `property/value/yrs.rs` → `property/value/yjs.ts` (already have yrs_string.ts?)
- [ ] `property/value/entity_ref.rs` → `property/value/entity_ref.ts`
- [ ] `property/value/json.rs` → `property/value/json.ts`

### Missing util files (may not need TS equivalents)
- [ ] `util/mod.rs` → `util/index.ts`
- [ ] `util/ivec.rs` — plain Array in TS
- [ ] `util/safeset.rs` — plain Set in TS
- [ ] `util/safemap.rs` — plain Map in TS
- [ ] `util/iterable.rs` — TS iterables
- [ ] `util/cast.rs` — covered by value/cast.ts?
- [ ] `util/expand_states.rs` — needs investigation

### Missing reactor files (naming mismatch?)
- [ ] `reactor/watcherset.rs` → `reactor/watcherset.ts` (have watcher_set.ts — name mismatch)
- [ ] `reactor/property_path.rs` → `reactor/property_path.ts` (have property-path.ts — name mismatch)
- [ ] `reactor/candidate_changes.rs` → `reactor/candidate_changes.ts` (have candidate-changes.ts — name mismatch)
- [ ] `reactor/comparison_index.rs` → `reactor/comparison_index.ts` (have comparison-index.ts — name mismatch)

### Missing signals files
- [ ] `signals/src/porcelain/wait.rs` → `porcelain/wait.ts`
- [ ] `signals/src/signal/memo.rs` → `signal/memo.ts`
- [ ] `signals/src/signal/map.rs` → `signal/map.ts`

### Missing storage files
- [ ] `storage/common/src/traits.rs` → `storage-common/src/traits.ts`
- [ ] `storage/sqlite/src/sql_builder.rs` → storage packages

## Storage engines (not yet started)

- [ ] `@ankurah/storage-better-sqlite3` — Node.js SQLite (for testing)
- [ ] `@ankurah/storage-expo-sqlite` — Expo Go SQLite (mobile)
- [ ] IndexedDB storage — browser target (may need new package)

## Layer 8 — Connectors
- [ ] `@ankurah/connector-local` — local in-process connector
- [ ] `@ankurah/connector-websocket` — WebSocket with reconnection

## Layer 9 — React
- [ ] `@ankurah/react` — hooks (useObserve, signalObserver HOC)

## Integration tests
- [ ] Port `tests/tests/basic.rs`
- [ ] Port `tests/tests/concurrent_transactions.rs`
- [ ] Port `tests/tests/inter_node.rs`
- [ ] Port `tests/tests/local_subscription.rs`
- [ ] Port `tests/tests/system.rs`
- [ ] Port remaining integration tests (15+ files)
- [ ] Rust WS server test harness (spawn, wait, test, kill)

## Infrastructure
- [ ] Enable eslint-plugin-ankurah in repo ESLint config
- [ ] Run linter on existing code and fix violations
- [ ] Add lint + audit to CI
- [ ] Fix audit script — reactor file naming mismatches (watcher_set vs watcherset, etc.)
- [ ] Spec cleanup per spec-reviewer recommendations

## Ownership spec conformance (mostly done)
- [x] Create @ankurah/std with Drop, RefCell, Mutex
- [x] Wire 7 types to Drop
- [x] ResultSetWrite → Drop guard with `using`
- [x] Transaction alive checks + Symbol.dispose
- [x] Rename Disposable → Drop
- [ ] Fix defineModel() to return guarded property instances (not raw handles)
- [ ] Add isWritable() checks to property value setters
