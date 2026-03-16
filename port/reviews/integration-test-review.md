# Integration Test Review

**Date**: 2026-03-15
**Scope**: All files in `packages/core/__tests__/integration/`
**Method**: Side-by-side comparison of every TS test against its Rust counterpart in `ankurah/tests/tests/`

---

## Summary

| Metric | Value |
|--------|-------|
| TS integration test files | 10 (8 `.test.ts` + 1 `_debug_observer.ts` + 1 `update_predicate.test.ts`) |
| Rust integration test files (portable) | 17 (excluding `common.rs`, sled-specific, and `policy_angent.rs` which is commented out) |
| Rust test functions (portable) | ~44 |
| TS test functions (active) | ~55 (across 10 files, some are TS-only split tests) |
| Currently passing | 29 |
| Currently failing | 18 (17 desc_inequality + 1 basic timeout) |
| Skipped | 8 (across multiple files, all justified) |

---

## Per-File Review

### 1. `desc_inequality.test.ts` vs `desc_inequality.rs`

**Parity**: PASS (19 Rust tests -> 19 TS tests, 1:1 mapping)

**Status**: ALL 17 non-trivial tests FAIL. 2 pass (single_equality_prefix, three_equality_prefix).

**Root Cause Analysis** (the 17 failures):

Two distinct bugs:

1. **Inequality filtering is completely broken**: Queries return ALL records or ZERO records instead of the filtered subset. For example, `timestamp <= mid` on 10 records returns 10 instead of 6; `timestamp > mid` returns 0 instead of 4. This means the inequality predicate evaluation in `ctx.fetch()` is not functioning for certain data types. The two passing tests (`single_equality_prefix` and `three_equality_prefix`) use small integer values (0-90), while all failing tests use `timestamp` values around 1,700,000,000,000.

2. **i64 truncation**: Timestamps are corrupted — `Expected: 1700000005000, Received: -807044216`. The value 1,700,000,005,000 requires 41 bits, but the received value (-807044216) is consistent with truncation to a signed 32-bit integer. This strongly suggests that somewhere in the LWW storage/retrieval path, `number` values above 2^31-1 are being stored or compared as 32-bit integers.

**Combined diagnosis**: The root cause is almost certainly a single bug — large `lww<number>()` values are being stored/compared as i32 instead of i64 (or f64). This corrupts both the stored values AND the inequality comparisons, since the comparisons operate on corrupted data. The two passing tests use small integers that fit in i32.

**Test quality**: The TS tests faithfully mirror the Rust tests. Setup helpers (`createMessages`, `createRoom`, `getTimestamps`, `assertDescOrder`, `assertAscOrder`) match their Rust counterparts. Constants match. Assertions match. The divergence annotations are correct (E1 for `Ref<TestRoom>` -> `lww<string>()`, E1 for `#[active_type(LWW)] bool` -> `lww<boolean>()`). No tests that pass for the wrong reason — the 2 passing tests genuinely exercise the same logic as Rust with values that happen to avoid the i32 truncation bug.

---

### 2. `basic.test.ts` vs `basic.rs`

**Parity**: PASS (1 Rust test -> 1 TS test)

**Status**: FAIL (timeout at 5000ms)

**Root Cause**: The test sets up a `viewWatcher` that listens on `album.entity().broadcast.reference().listen(...)` — this subscription mechanism may not be firing after `trx2.commit()`. The test hangs at `await viewWatcher.takeOne()` waiting for the first notification. This is a **real bug** in the subscription/broadcast machinery, not a test issue.

**Test quality**: Good faithful port. The Rust test uses `album.subscribe(&view_watcher)` which subscribes directly to the View. The TS divergence (`album.entity().broadcast.reference().listen(...)`) is annotated and justified — View doesn't implement Subscribe directly. However, this divergence IS the likely root cause of the timeout: the broadcast mechanism may not propagate commit notifications correctly to the listener.

**Concern**: The `_debug_observer.ts` file in the same directory is a standalone debug script that tests the same commit->notification flow. It's not a test file — it's a diagnostic artifact. Should be either removed or converted to a proper test.

---

### 3. `repeatable_read.test.ts` vs `repeatable_read.rs`

**Parity**: PASS (1 Rust test -> 1 TS test)

**Status**: Presumed passing (not in fail list).

**Test quality**: Good port. Key CRDT merge scenario ("I love cats" -> concurrent edits -> "I devour tofu") is faithfully reproduced. One concern: the TS test reads the read-only view value via `albumRo.entity().getPropertyValue('name')` and asserts on `(roVal as any).value`, which is fragile — it relies on the internal shape of the property value object. The Rust test uses `album_ro.name().unwrap()` which is a clean typed accessor. This is a **test fidelity issue** — if the internal property value shape changes, this test would break in a way the Rust test wouldn't.

---

### 4. `property_backends.test.ts` vs `property_backends.rs`

**Parity**: PASS (1 Rust test -> 1 TS test)

**Status**: Presumed passing (not in fail list).

**Test quality**: Good port. Creates a Video with mixed YrsString + LWW backends, modifies both, commits, and verifies. The divergence from `Visibility` enum to string literal is correctly annotated. The test correctly exercises both backend types.

---

### 5. `json_livequery.test.ts` vs `json_livequery.rs`

**Parity**: PASS (4 Rust tests -> 4 TS tests)

**Status**: Presumed passing (not in fail list).

**Test quality**: Faithful port. All 4 tests match: initial results, new entity notification, nested path query, and predicate re-evaluation. The `Json` property type mapping (`lww<unknown>()`) is correctly divergent from Rust's `Json` type. TestWatcher is simplified but functionally equivalent.

---

### 6. `nonexistent_entity.test.ts` vs `nonexistent_entity.rs`

**Parity**: PASS (4 Rust tests -> 4 TS tests, 3 correctly skipped)

**Status**: Presumed passing (not in fail list).

**Test quality**: Good. The 1 active test (`get_nonexistent_entity_errors`) correctly checks for `RetrievalError` with `kind === 'EntityNotFound'`. The 3 skipped tests are correctly annotated:
- `local_rejects_phantom_commit` — requires `conjure_evil_phantom` (not ported)
- `server_rejects_update_for_nonexistent` — requires `LocalProcessConnection` (not ported)
- `server_rejects_create_for_existing` — requires `LocalProcessConnection` (not ported)

---

### 7. `limit_gap_filling.test.ts` vs `limit_gap_filling.rs`

**Parity**: PASS (4 Rust tests -> 4 TS tests, 2 correctly skipped)

**Status**: The 2 active tests are presumed passing (not in fail list).

**Test quality**: Faithful port. `test_single_node_gap_filling` and `test_single_node_multiple_gap_filling` match exactly — same album creation, same predicate, same expected changeset notifications, same final state assertions. The `YrsString.replace()` call for mutations matches the Rust `year().replace()`. The 2 skipped inter-node tests are correctly annotated as requiring `LocalProcessConnection`.

**Note**: The `ChangesetWatcher` class is duplicated across `limit_gap_filling.test.ts`, `update_predicate.test.ts`, and `json_livequery.test.ts`. In Rust, this comes from `common::TestWatcher::changeset()`. Consider extracting to a shared test helper.

---

### 8. `selection_macro.test.ts` vs `selection_macro.rs`

**Parity**: PASS (9 Rust tests -> ~13 TS test cases across 8 describe blocks)

**Status**: Presumed passing (not in fail list).

**Test quality**: Good adaptation. The Rust `selection!` macro has no TS equivalent, so the TS tests correctly use `parseSelection()` + `populatePredicate()` as the runtime equivalent. The divergence is well-documented in the file header. All macro syntax forms (unquoted, quoted, shorthand, operator shorthand, list expansion, edge cases, syntax comparison, pure syntax forms) have corresponding TS tests.

**Minor concern**: The `test_selection_macro_in_clause` test (line 118-146) constructs an expected `Selection` with deeply nested `.match()` calls on `Expr` variants that are overly complex. The test then falls back to checking `comp.operator.type === 'In'` and `comp.right.type === 'ExprList'` — the complex expected construction is dead code. Not harmful but adds noise.

**Missing from Rust**: The Rust `test_selection_macro_operator_shorthand` has two additional sub-assertions:
- `selection!({=age}) == selection!({age})` — explicit equality equivalence
- `selection!({<>status})` — alternative not-equal operator

These are macro-specific and don't apply to TS (no macro system), so the omission is correct.

---

### 9. `policy_agent.test.ts` vs `policy_angent.rs`

**Parity**: PASS (0 active Rust tests -> 0 active TS tests, both fully commented/skipped)

**Status**: All tests skipped. Correct — the entire Rust file is commented out.

**Test quality**: Appropriate. The TS file correctly mirrors the Rust state and documents what each test would need.

---

### 10. `update_predicate.test.ts` vs `update_predicate.rs`

**Parity**: PASS (2 Rust tests -> 2 TS tests, 1 correctly skipped)

**Status**: Presumed passing for the active test (not in fail list).

**Test quality**: Faithful port. The active test (`test_predicate_update`) correctly creates albums, sets up a LiveQuery, updates the selection predicate, and verifies membership changes with sorted ID comparisons. The `drain_sorted` / `sorted![]` macros are correctly adapted. The inter-node test is correctly skipped.

**Minor concern**: Album creation uses `trx.create(Album, {})` followed by manual `YrsString.insert()` calls, rather than passing initial values to `create()`. The Rust test passes initial values directly: `trx.create(&Album { name: "Alpha", year: "2020" })`. This divergence is functional (works the same way) but differs from the Rust pattern — all other TS tests pass initial values to `create()`.

---

## Rust Integration Tests NOT Ported

The following Rust test files have NO TS counterpart:

| Rust file | # tests | Reason / Blocker |
|-----------|---------|-----------------|
| `concurrent_transactions.rs` | 3 | Portability: no dependencies beyond Node + MemoryStorage. **Should be ported.** |
| `predicate_checks.rs` | 1 | Portability: uses `predicate_cases.json` fixture. **Should be ported.** |
| `where_clause.rs` | 2 | Portability: basic predicate testing. **Should be ported.** |
| `check_request_error.rs` | 1 | Requires multi-node + connectors. Defer. |
| `local_subscription.rs` | 3 | Portability: may work with single-node. **Should be investigated.** |
| `pagination_cursor.rs` | 5 | Portability: may work with single-node queries. **Should be investigated.** |
| `rt106.rs` | 1 | Regression test — depends on multi-node. Defer. |
| `rt114.rs` | 2 | Regression test — depends on multi-node. Defer. |
| `system.rs` | 6 | Requires SystemManager + multi-node. Defer. |
| `websocket.rs` | ~5 | Requires WebSocket connectors. Defer. |
| `inter_node.rs` | many | Requires connectors. Defer. |
| `sled_*.rs` (3 files) | ~8 | Storage-engine-specific. Not applicable. |

**Total unported portable tests**: ~9 (concurrent_transactions: 3, predicate_checks: 1, where_clause: 2, local_subscription: up to 3)

---

## Critical Findings

### Finding 1: i64 truncation in LWW number storage (BLOCKING)

**Severity**: Critical
**Impact**: 17 test failures in desc_inequality, and potentially any production code using `lww<number>()` with values > 2^31-1

Large numbers stored via `lww<number>()` are being truncated to 32-bit signed integers. This affects:
- Storage: values are corrupted when saved
- Comparison: inequality predicates produce wrong results on corrupted values
- All 17 desc_inequality failures trace to this single root cause

**Where to look**: The LWW backend's storage path for number values — likely in the `Value` encoding/decoding, or in the comparison index key encoding. The `Collation` or `KeySpec` code that serializes numbers for index keys may be using 32-bit writes.

### Finding 2: Broadcast notification not firing on commit (BLOCKING for basic.test.ts)

**Severity**: High
**Impact**: 1 test failure (basic.test.ts timeout)

After `trx2.commit()`, the `viewWatcher` never receives a notification, causing the test to hang at `takeOne()` for 5000ms. The subscription path `album.entity().broadcast.reference().listen(...)` appears to not propagate commit events.

### Finding 3: Duplicate TestWatcher/ChangesetWatcher implementations (LOW)

**Severity**: Low (test hygiene)
**Impact**: Maintenance burden

The `TestWatcher`/`ChangesetWatcher` pattern is copy-pasted across `basic.test.ts`, `json_livequery.test.ts`, `limit_gap_filling.test.ts`, and `update_predicate.test.ts`. In Rust, this comes from `common::TestWatcher`. Consider creating `__tests__/integration/common.ts` to mirror the Rust shared helpers.

---

## Verdict

**Test parity for ported files**: GOOD. Every TS integration test file that exists faithfully mirrors its Rust counterpart. Test names, setup logic, assertion values, and edge cases all match. Divergences are correctly annotated.

**Test correctness**: 18 failures, all traceable to 2 bugs in the implementation (not in the tests themselves). No tests pass for the wrong reason.

**Coverage gap**: 9 portable Rust test functions have no TS counterpart (concurrent_transactions, predicate_checks, where_clause). These should be ported next.
