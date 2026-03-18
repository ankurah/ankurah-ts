# ankurah-signals — Punchlist

**Rust crate**: `ankurah-signals` (`ankurah-ts-support/signals/`)
**TS package**: `@ankurah/signals` (`packages/signals/`)
**Dependencies**: none

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | signals/src/broadcast.rs | packages/signals/src/broadcast.ts | TODO |
| 2 | signals/src/context.rs | packages/signals/src/context.ts | TODO |
| 3 | signals/src/jsvalue.rs | — | SKIP: JsValue WASM interop, not needed in pure TS |
| 4 | signals/src/lib.rs | packages/signals/src/index.ts | TODO |
| 5 | signals/src/observer.rs | packages/signals/src/observer/index.ts | TODO |
| 6 | signals/src/observer/callback_observer.rs | packages/signals/src/observer/callback_observer.ts | TODO |
| 7 | signals/src/porcelain.rs | packages/signals/src/porcelain/index.ts | TODO |
| 8 | signals/src/porcelain/subscribe.rs | packages/signals/src/porcelain/subscribe.ts | TODO |
| 9 | signals/src/porcelain/wait.rs | packages/signals/src/porcelain/wait.ts | TODO |
| 10 | signals/src/react.rs | — | SKIP: React WASM bindings (E14/E15) — TS @ankurah/react is separate |
| 11 | signals/src/react_native.rs | — | SKIP: React Native WASM bindings (E14/E15) |
| 12 | signals/src/reactive_graph.rs | — | SKIP: Leptos reactive_graph integration, not applicable |
| 13 | signals/src/signal.rs | packages/signals/src/signal/index.ts | TODO |
| 14 | signals/src/signal/calculated.rs | packages/signals/src/signal/calculated.ts | TODO |
| 15 | signals/src/signal/map.rs | packages/signals/src/signal/map.ts | TODO |
| 16 | signals/src/signal/memo.rs | packages/signals/src/signal/memo.ts | TODO |
| 17 | signals/src/signal/mutable.rs | packages/signals/src/signal/mutable.ts | TODO |
| 18 | signals/src/signal/read.rs | packages/signals/src/signal/read.ts | TODO |
| 19 | signals/src/value.rs | packages/signals/src/value.ts | TODO |

## Unit Tests (inline)

### signals/src/broadcast.rs (4 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_multiple_subscribers | TODO |
| 2 | test_channel_sender_subscriber | TODO |
| 3 | test_subscribe_trait | TODO |
| 4 | test_reentrant_subscription_during_send | TODO |

### signals/src/signal/calculated.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_basic_calculated | TODO |
| 2 | test_two_independent_inputs | TODO |
| 3 | test_calculated_with_closed_over_state | TODO |
| 4 | test_calculated_downstream_subscription | TODO |
| 5 | test_chained_calculated | TODO |
| 6 | test_listener_does_not_pollute_dependencies | TODO |

## Integration Tests

### signals/tests/basic.rs (15 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_basic_signal | TODO |
| 2 | test_basic_subscriber | TODO |
| 3 | test_wait_value | TODO |
| 4 | test_wait_predicate | TODO |
| 5 | test_wait_for_result | TODO |
| 6 | test_wait_for_boolean | TODO |
| 7 | test_wait_for_option | TODO |
| 8 | test_wait_for_immediate_match | TODO |
| 9 | test_map_signal | TODO |
| 10 | test_map_signal_string_transform | TODO |
| 11 | test_read_map_convenience_method | TODO |
| 12 | test_memo_caches_value | TODO |
| 13 | test_memo_invalidates_on_change | TODO |
| 14 | test_memo_subscription | TODO |
| 15 | test_memo_with_does_not_require_clone | TODO |

### signals/tests/common.rs

SKIP: Test helper module, not a test file.

### signals/tests/observer_context.rs (9 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_manual_subscription_works | TODO |
| 2 | test_basic_observer_subscription | TODO |
| 3 | test_multiple_signals_single_observer | TODO |
| 4 | test_nested_observer_contexts | TODO |
| 5 | test_deep_nested_context_restoration | TODO |
| 6 | test_observer_cleanup | TODO |
| 7 | test_context_subscription_clearing | TODO |
| 8 | test_react_style_try_finally_pattern | TODO |
| 9 | test_context_remove_pointer_equality | TODO |

### signals/tests/observer.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_observer | TODO |

## Summary

- Source files: 19 (4 skip)
- Unit tests: 10
- Integration tests: 25
