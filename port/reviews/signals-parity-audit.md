# Signals Parity Audit (v2)

**Date**: 2026-03-15
**Auditor**: signals-auditor agent
**Verdict**: **PASS** — all Rust tests ported; minor source export nits remain

---

## 1. Test Parity: Rust Test -> TS Test Mapping

### 1a. Inline Tests (`src/*.rs`)

#### `broadcast.rs` (4 tests)

| # | Rust test | TS test | Status |
|---|---|---|---|
| 1 | `test_multiple_subscribers` | `broadcast.test.ts` "multiple subscribers" | PASS |
| 2 | `test_channel_sender_subscriber` | N/A (tokio feature) | SKIP |
| 3 | `test_subscribe_trait` | `mutable.test.ts` "subscribe receives updated values" | PASS |
| 4 | `test_reentrant_subscription_during_send` | `broadcast.test.ts` "reentrant subscription during send" | PASS |

#### `signal/calculated.rs` (6 tests)

| # | Rust test | TS test | Status |
|---|---|---|---|
| 1 | `test_basic_calculated` | `calculated.test.ts` "basic calculated" | PASS |
| 2 | `test_two_independent_inputs` | `calculated.test.ts` "two independent inputs" | PASS |
| 3 | `test_calculated_with_closed_over_state` | `calculated.test.ts` "calculated with closed-over state" | PASS |
| 4 | `test_calculated_downstream_subscription` | `calculated.test.ts` "calculated downstream subscription" | PASS |
| 5 | `test_chained_calculated` | `calculated.test.ts` "chained calculated" | PASS |
| 6 | `test_listener_does_not_pollute_dependencies` | `calculated.test.ts` "listener does not pollute dependencies" | PASS |

### 1b. Integration Tests (`signals/tests/`)

#### `tests/basic.rs` (15 tests)

| # | Rust test | TS test | Status |
|---|---|---|---|
| 1 | `test_basic_signal` | `basic.test.ts` "test_basic_signal" | PASS |
| 2 | `test_basic_subscriber` | `basic.test.ts` "test_basic_subscriber" | PASS |
| 3 | `test_wait_value` | `basic.test.ts` "test_wait_value" | PASS |
| 4 | `test_wait_predicate` | `basic.test.ts` "test_wait_predicate" | PASS |
| 5 | `test_wait_for_result` | `basic.test.ts` "test_wait_for_result" | PASS |
| 6 | `test_wait_for_boolean` | `basic.test.ts` "test_wait_for_boolean" | PASS |
| 7 | `test_wait_for_option` | `basic.test.ts` "test_wait_for_option" | PASS |
| 8 | `test_wait_for_immediate_match` | `basic.test.ts` "test_wait_for_immediate_match" | PASS |
| 9 | `test_map_signal` | `basic.test.ts` "test_map_signal" | PASS |
| 10 | `test_map_signal_string_transform` | `basic.test.ts` "test_map_signal_string_transform" | PASS |
| 11 | `test_read_map_convenience_method` | `basic.test.ts` "test_read_map_convenience_method" | PASS |
| 12 | `test_memo_caches_value` | `basic.test.ts` "test_memo_caches_value" | PASS |
| 13 | `test_memo_invalidates_on_change` | `basic.test.ts` "test_memo_invalidates_on_change" | PASS |
| 14 | `test_memo_subscription` | `basic.test.ts` "test_memo_subscription" | PASS |
| 15 | `test_memo_with_does_not_require_clone` | `basic.test.ts` "test_memo_with_does_not_require_clone" | PASS |

#### `tests/observer.rs` (1 test)

| # | Rust test | TS test | Status |
|---|---|---|---|
| 1 | `test_observer` | `observer.test.ts` "test_observer" | PASS |

#### `tests/observer_context.rs` (9 tests)

| # | Rust test | TS test | Status |
|---|---|---|---|
| 1 | `test_manual_subscription_works` | `observer_context.test.ts` "test_manual_subscription_works" | PASS |
| 2 | `test_basic_observer_subscription` | `observer_context.test.ts` "test_basic_observer_subscription" | PASS |
| 3 | `test_multiple_signals_single_observer` | `observer_context.test.ts` "test_multiple_signals_single_observer" | PASS |
| 4 | `test_nested_observer_contexts` | `observer_context.test.ts` "test_nested_observer_contexts" | PASS |
| 5 | `test_deep_nested_context_restoration` | `observer_context.test.ts` "test_deep_nested_context_restoration" | PASS |
| 6 | `test_observer_cleanup` | `observer_context.test.ts` "test_observer_cleanup" | PASS |
| 7 | `test_context_subscription_clearing` | `observer_context.test.ts` "test_context_subscription_clearing" | PASS |
| 8 | `test_react_style_try_finally_pattern` | `observer_context.test.ts` "test_react_style_try_finally_pattern" | PASS |
| 9 | `test_context_remove_pointer_equality` | `observer_context.test.ts` "test_context_remove_pointer_equality" | PASS |

### 1c. Score

| Category | Rust tests | TS ported | Status |
|---|---|---|---|
| `broadcast.rs` inline | 3 applicable | 3 | PASS |
| `calculated.rs` inline | 6 | 6 | PASS |
| `tests/basic.rs` | 15 | 15 | PASS |
| `tests/observer.rs` | 1 | 1 | PASS |
| `tests/observer_context.rs` | 9 | 9 | PASS |
| **Total** | **34 applicable** | **34** | **PASS** |

---

## 2. Source Parity

No changes from v1 audit — all Rust public types have TS equivalents. **PASS**.

---

## 3. Remaining Nits (non-blocking)

1. **`signal/index.ts`** has stale comments: `Calculated` listed as "a stub (deferred)", `Map`/`Memo` as "not yet ported". Re-exports are missing from this barrel file (they ARE correctly exported from `src/index.ts`, so consumers are unaffected).

2. **`porcelain/index.ts`** does not re-export `Wait`/`waitValue`/`waitFor`. Rust `porcelain.rs` re-exports `Wait`. These are exported from `src/index.ts` so consumers are unaffected.

---

## 4. Verdict

**PASS**

All 34 applicable Rust test functions now have TS equivalents across 7 test files. Source API parity is complete. The two barrel-file export nits are cosmetic and do not affect external consumers.
