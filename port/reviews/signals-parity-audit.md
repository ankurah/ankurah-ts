# Signals Parity Audit

**Date**: 2026-03-15
**Auditor**: signals-auditor agent
**Verdict**: **FAIL** — missing integration test files, missing test functions, and source export gaps

---

## 1. Test Parity: Rust Test -> TS Test Mapping

### 1a. Inline Tests (inside `src/*.rs`)

#### `broadcast.rs` inline tests

| # | Rust test function | TS test | Status |
|---|---|---|---|
| 1 | `test_multiple_subscribers` | `broadcast.test.ts` → "multiple subscribers" | PASS |
| 2 | `test_channel_sender_subscriber` (tokio feature) | No TS equivalent | SKIP (tokio feature, N/A in TS) |
| 3 | `test_subscribe_trait` | `mutable.test.ts` → "subscribe receives updated values" | PASS (tested via Mut, equivalent) |
| 4 | `test_reentrant_subscription_during_send` | `broadcast.test.ts` → "reentrant subscription during send" | PASS |

#### `signal/calculated.rs` inline tests

| # | Rust test function | TS test | Status |
|---|---|---|---|
| 1 | `test_basic_calculated` | **MISSING** | FAIL |
| 2 | `test_two_independent_inputs` | **MISSING** | FAIL |
| 3 | `test_calculated_with_closed_over_state` | **MISSING** | FAIL |
| 4 | `test_calculated_downstream_subscription` | **MISSING** | FAIL |
| 5 | `test_chained_calculated` | **MISSING** | FAIL |
| 6 | `test_listener_does_not_pollute_dependencies` | **MISSING** | FAIL |

**Note**: There are NO test files for `Calculated`, `Map`, or `Memo` in the TS codebase despite the implementations being present.

### 1b. Integration Tests (`signals/tests/`)

#### `tests/basic.rs`

| # | Rust test function | TS test | Status |
|---|---|---|---|
| 1 | `test_basic_signal` | `basic.test.ts` → "test_basic_signal" | PASS (sync variant, no channel test) |
| 2 | `test_basic_subscriber` | `basic.test.ts` → "test_basic_subscriber" | PASS |
| 3 | `test_wait_value` (tokio feature) | **MISSING** | FAIL — `wait.ts` is implemented but has no tests |
| 4 | `test_wait_predicate` | **MISSING** | FAIL |
| 5 | `test_wait_for_result` | **MISSING** | FAIL |
| 6 | `test_wait_for_boolean` | **MISSING** | FAIL |
| 7 | `test_wait_for_option` | **MISSING** | FAIL |
| 8 | `test_wait_for_immediate_match` | **MISSING** | FAIL |
| 9 | `test_map_signal` | **MISSING** | FAIL — `Map` class exists but has no tests |
| 10 | `test_map_signal_string_transform` | **MISSING** | FAIL |
| 11 | `test_read_map_convenience_method` | **MISSING** | FAIL |
| 12 | `test_memo_caches_value` | **MISSING** | FAIL — `Memo` class exists but has no tests |
| 13 | `test_memo_invalidates_on_change` | **MISSING** | FAIL |
| 14 | `test_memo_subscription` | **MISSING** | FAIL |
| 15 | `test_memo_with_does_not_require_clone` | **MISSING** | FAIL (N/A in TS — no Clone bound distinction) |

**Note**: `basic.test.ts` lists these as "Deferred" but the implementations (`Map`, `Memo`, `wait.ts`) now exist in TS. Tests should have been added.

#### `tests/observer.rs`

| # | Rust test function | TS test | Status |
|---|---|---|---|
| 1 | `test_observer` | **MISSING** — No `observer.test.ts` exists | FAIL |

#### `tests/observer_context.rs`

| # | Rust test function | TS test | Status |
|---|---|---|---|
| 1 | `test_manual_subscription_works` | **MISSING** — No `observer_context.test.ts` exists | FAIL |
| 2 | `test_basic_observer_subscription` | **MISSING** | FAIL |
| 3 | `test_multiple_signals_single_observer` | **MISSING** | FAIL |
| 4 | `test_nested_observer_contexts` | **MISSING** | FAIL |
| 5 | `test_deep_nested_context_restoration` | **MISSING** | FAIL |
| 6 | `test_observer_cleanup` | **MISSING** | FAIL |
| 7 | `test_context_subscription_clearing` | **MISSING** | FAIL |
| 8 | `test_react_style_try_finally_pattern` | **MISSING** | FAIL |
| 9 | `test_context_remove_pointer_equality` | **MISSING** | FAIL |

### 1c. Summary — Tests that exist in TS but have no Rust counterpart

These are TS-only tests (not a parity issue, just extra coverage):

- `basic.test.ts` → "signal Mut subscribe/set integration" (extra)
- `basic.test.ts` → "multiple subscriptions on same signal" (extra)
- `basic.test.ts` → "signal with complex types" (extra)
- `basic.test.ts` → "read signal reflects mutations through get/peek/with" (extra)
- `basic.test.ts` → "listener guard drop is idempotent" (extra)
- `basic.test.ts` → "subscription guard drop is idempotent" (extra)
- `broadcast.test.ts` → Multiple BroadcastId tests, "notify-only listeners", "payload listeners receive value", "broadcast ID is consistent", "reference broadcast ID matches sender ID", "listener guard broadcast ID matches sender ID", "drop is idempotent" (extra)
- `mutable.test.ts` → All 16 tests (extra, Rust has no inline Mut tests)
- `read.test.ts` → All 12 tests (extra, Rust has no inline Read tests)

---

## 2. Source Parity

### 2a. File-by-file comparison

| Rust file | TS file | Status |
|---|---|---|
| `broadcast.rs` | `broadcast.ts` | PASS — all public types present: `BroadcastId`, `BroadcastListener`, `Broadcast`, `Ref` (→ `BroadcastRef`), `ListenerGuard`, `TListenerGuard`, `IntoBroadcastListener` (adapted) |
| `signal.rs` | `signal/index.ts` | PASS — `Signal`, `Get`, `Peek`, `With`, `GetReadCell`, `Listener`, `ListenerGuard` all present |
| `signal/mutable.rs` | `signal/mutable.ts` | PASS — `Mut<T>` with `new`, `set`, `get`, `peek`, `with`, `value`, `read`, `listen`, `broadcastId`, `subscribe` |
| `signal/read.rs` | `signal/read.ts` | PASS — `Read<T>` with `value`, `map`, `memo`, `get`, `peek`, `with`, `getReadCell`, `equals`, `toString`, `listen`, `broadcastId`, `subscribe` |
| `signal/map.rs` | `signal/map.ts` | PASS — `Map` with `new`, `listen`, `broadcastId`, `with`, `get`, `peek`, `subscribe` |
| `signal/memo.rs` | `signal/memo.ts` | PASS — `Memo` with `new`, `listen`, `broadcastId`, `with`, `get`, `peek`, `subscribe`, private `withCached` |
| `signal/calculated.rs` | `signal/calculated.ts` | PASS — `Calculated` with `new`, `clone`, `listen`, `broadcastId`, `get`, `peek`, `with`, `getReadCell`, `subscribe`. Observer impl via `InnerObserver`. |
| `observer.rs` | `observer/index.ts` | PASS — `Observer` interface with `observe`, `observerId`. `ObserverBounds` correctly omitted (TS has no threading). |
| `observer/callback_observer.rs` | `observer/callback_observer.ts` | PASS — `CallbackObserver` with `new`, `clone`, `trigger`, `withContext`, `clear`, `observe`, `observerId`, private `markAllForRemoval`, `sweepMarkedListeners` |
| `context.rs` | `context.ts` | PASS — `CurrentObserver` with `track`, `set`, `pop`, `remove`, `current`. Correctly uses module-level stack (singlethread variant). |
| `value.rs` | `value.ts` | PASS — `ValueCell`, `ReadValueCell` with `new`, `clone`, `set`, `with`, `setWith`, `readvalue`, `value` |
| `porcelain/subscribe.rs` | `porcelain/subscribe.ts` | PASS — `Subscribe<T>`, `SubscriptionGuard`. `DynSubscribe`/`GetAndDynSubscribe` correctly omitted (TS doesn't need the distinction). |
| `porcelain/wait.rs` | `porcelain/wait.ts` | PASS — `Wait<T>`, `WaitResult` (adapted to TS), `waitValue`, `waitFor` standalone functions |
| `lib.rs` | `index.ts` | **PARTIAL** — see gaps below |
| `porcelain.rs` | `porcelain/index.ts` | **GAP** — `Wait` is not re-exported from porcelain/index.ts (exported only from index.ts directly) |

### 2b. Export gaps in `signal/index.ts`

The file has comments saying `Calculated` is "a stub (deferred)" and `Map`/`Memo` are "not yet ported", but the implementations exist in their respective `.ts` files. The re-exports are missing:

```typescript
// Currently:
export { } from './calculated.ts';  // empty re-export
// map.ts not yet ported
// memo.ts not yet ported

// Should be:
export { Calculated } from './calculated.ts';
export { Map } from './map.ts';
export { Memo } from './memo.ts';
```

These **are** re-exported from `src/index.ts` (the package entry point), so external consumers can import them, but `signal/index.ts` is out of date with the actual codebase.

### 2c. Feature-gated modules (correctly skipped in TS)

| Rust module | Feature | TS status |
|---|---|---|
| `reactive_graph.rs` | reactive-graph | Skipped (correct) |
| `react.rs` | react | Skipped (correct) |
| `react_native.rs` | react-native | Skipped (correct) |
| `jsvalue.rs` | jsvalue | Skipped (correct) |

### 2d. Missing public types/traits in TS

| Rust type | Status |
|---|---|
| `IntoSubscribeListener<T>` | Not needed — TS subscribe takes `(value: T) => void` directly |
| `SubscribeListener<T>` | Not needed — same reason |
| `IntoBroadcastListener<T>` | Not needed — TS listen takes `BroadcastListener<T>` directly |
| `DynSubscribe<T>` | Not needed — TS `Subscribe` is already dynamically dispatched |
| `GetAndDynSubscribe<T>` | Not needed — use intersection type at call sites |

All omissions are justified by TS's type system differences.

---

## 3. Test Score

| Category | Rust test count | TS test count | Missing |
|---|---|---|---|
| `broadcast.rs` inline | 4 (1 tokio-only) | 3 of 3 applicable | 0 |
| `calculated.rs` inline | 6 | 0 | **6** |
| `tests/basic.rs` | 15 (1 tokio-only) | 2 of 14 applicable | **12** |
| `tests/observer.rs` | 1 | 0 | **1** |
| `tests/observer_context.rs` | 9 | 0 | **9** |
| **Total** | **35** | **5** | **28** |

**Test parity: 5/35 = 14%**

---

## 4. Critical Gaps

### 4a. Missing test files (need to be created)

1. **`__tests__/observer.test.ts`** — Port of `tests/observer.rs` (1 test)
2. **`__tests__/observer_context.test.ts`** — Port of `tests/observer_context.rs` (9 tests)
3. **`src/signal/calculated.test.ts`** — Port of `calculated.rs` inline tests (6 tests)
4. **`src/signal/map.test.ts`** — Tests for Map (port `test_map_signal`, `test_map_signal_string_transform`, `test_read_map_convenience_method`)
5. **`src/signal/memo.test.ts`** — Tests for Memo (port `test_memo_caches_value`, `test_memo_invalidates_on_change`, `test_memo_subscription`, `test_memo_with_does_not_require_clone`)
6. **`__tests__/wait.test.ts`** — Tests for wait functionality (port `test_wait_value`, `test_wait_predicate`, `test_wait_for_result`, `test_wait_for_boolean`, `test_wait_for_option`, `test_wait_for_immediate_match`)

### 4b. Source export fix needed

- `signal/index.ts` needs to re-export `Calculated`, `Map`, `Memo` instead of stale comments saying they're deferred/not ported.

### 4c. `porcelain/index.ts` should re-export `Wait`

Currently the `Wait` type and `waitValue`/`waitFor` functions are only exported from `src/index.ts`, not from `porcelain/index.ts`. The Rust `porcelain.rs` re-exports `Wait` from its submodule.

---

## 5. Verdict

**FAIL**

The source implementations are in good shape — all Rust public types have TS equivalents with correct API shapes. However:

- **28 of 35 Rust test functions have no TS equivalent** (80% missing)
- **No test files exist** for `Calculated`, `Map`, `Memo`, `Observer`, `ObserverContext`, or `Wait` despite the implementations being present
- The `basic.test.ts` file still has a deferred-items comment listing Map/Memo/Wait as unimplemented, but they are now implemented
- `signal/index.ts` re-exports are stale

The source parity is strong. The test parity is critically lacking.
