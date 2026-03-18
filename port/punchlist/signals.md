# ankurah-signals — Punchlist

**Rust crate**: `ankurah-signals` (`ankurah-ts-support/signals/`)
**TS package**: `@ankurah/signals` (`packages/signals/`)
**Dependencies**: none

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | signals/src/broadcast.rs | packages/signals/src/broadcast.ts | DONE |
| 2 | signals/src/context.rs | packages/signals/src/context.ts | DONE |
| 3 | signals/src/jsvalue.rs | — | SKIP: JsValue WASM interop, not needed in pure TS |
| 4 | signals/src/lib.rs | packages/signals/src/index.ts | DONE |
| 5 | signals/src/observer.rs | packages/signals/src/observer/index.ts | DONE |
| 6 | signals/src/observer/callback_observer.rs | packages/signals/src/observer/callback_observer.ts | DONE |
| 7 | signals/src/porcelain.rs | packages/signals/src/porcelain/index.ts | DONE |
| 8 | signals/src/porcelain/subscribe.rs | packages/signals/src/porcelain/subscribe.ts | DONE |
| 9 | signals/src/porcelain/wait.rs | packages/signals/src/porcelain/wait.ts | DONE |
| 10 | signals/src/react.rs | — | SKIP: React WASM bindings (E14/E15) — TS @ankurah/react is separate |
| 11 | signals/src/react_native.rs | — | SKIP: React Native WASM bindings (E14/E15) |
| 12 | signals/src/reactive_graph.rs | — | SKIP: Leptos reactive_graph integration, not applicable |
| 13 | signals/src/signal.rs | packages/signals/src/signal/index.ts | DONE |
| 14 | signals/src/signal/calculated.rs | packages/signals/src/signal/calculated.ts | DONE |
| 15 | signals/src/signal/map.rs | packages/signals/src/signal/map.ts | DONE |
| 16 | signals/src/signal/memo.rs | packages/signals/src/signal/memo.ts | DONE |
| 17 | signals/src/signal/mutable.rs | packages/signals/src/signal/mutable.ts | DONE |
| 18 | signals/src/signal/read.rs | packages/signals/src/signal/read.ts | DONE |
| 19 | signals/src/value.rs | packages/signals/src/value.ts | DONE |

## Unit Tests (inline)

### signals/src/broadcast.rs (4 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_multiple_subscribers | DONE |
| 2 | test_channel_sender_subscriber | DONE |
| 3 | test_subscribe_trait | DONE |
| 4 | test_reentrant_subscription_during_send | DONE |

### signals/src/signal/calculated.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_basic_calculated | DONE |
| 2 | test_two_independent_inputs | DONE |
| 3 | test_calculated_with_closed_over_state | DONE |
| 4 | test_calculated_downstream_subscription | DONE |
| 5 | test_chained_calculated | DONE |
| 6 | test_listener_does_not_pollute_dependencies | DONE |

## Integration Tests

### signals/tests/basic.rs (15 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_basic_signal | DONE |
| 2 | test_basic_subscriber | DONE |
| 3 | test_wait_value | DONE |
| 4 | test_wait_predicate | DONE |
| 5 | test_wait_for_result | DONE |
| 6 | test_wait_for_boolean | DONE |
| 7 | test_wait_for_option | DONE |
| 8 | test_wait_for_immediate_match | DONE |
| 9 | test_map_signal | DONE |
| 10 | test_map_signal_string_transform | DONE |
| 11 | test_read_map_convenience_method | DONE |
| 12 | test_memo_caches_value | DONE |
| 13 | test_memo_invalidates_on_change | DONE |
| 14 | test_memo_subscription | DONE |
| 15 | test_memo_with_does_not_require_clone | DONE |

### signals/tests/common.rs

SKIP: Test helper module, not a test file.

### signals/tests/observer_context.rs (9 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_manual_subscription_works | DONE |
| 2 | test_basic_observer_subscription | DONE |
| 3 | test_multiple_signals_single_observer | DONE |
| 4 | test_nested_observer_contexts | DONE |
| 5 | test_deep_nested_context_restoration | DONE |
| 6 | test_observer_cleanup | DONE |
| 7 | test_context_subscription_clearing | DONE |
| 8 | test_react_style_try_finally_pattern | DONE |
| 9 | test_context_remove_pointer_equality | DONE |

### signals/tests/observer.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_observer | DONE |

## Summary

- Source files: 19 (4 skip)
- Unit tests: 10
- Integration tests: 25
