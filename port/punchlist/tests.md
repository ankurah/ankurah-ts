# ankurah-tests — Punchlist

**Rust crate**: `ankurah-tests` (`ankurah-ts-support/tests/`)
**TS location**: `packages/core/__tests__/integration/`
**Dependencies**: all crates (top-level integration tests)

Note: These are the top-level integration tests that exercise the full stack. In TS they live under `@ankurah/core` since there's no separate test package.

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | tests/src/lib.rs | — | SKIP: Test helper/model definitions. TS equivalent is inline in test files or in test helpers. |

## Test Helper Files

| # | Rust file | Status |
|---|-----------|--------|
| 1 | tests/tests/common.rs | SKIP: Test helper module (setup_tracing, model definitions) |

## Integration Tests

### tests/tests/basic.rs (1 test)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_sled | packages/core/__tests__/integration/basic.test.ts | TODO |

### tests/tests/check_request_error.rs (1 test)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | check_request_error_returns_to_client | packages/core/__tests__/integration/check_request_error.test.ts | TODO |

### tests/tests/concurrent_transactions.rs (3 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_concurrent_transactions_same_entity | packages/core/__tests__/integration/concurrent_transactions.test.ts | TODO |
| 2 | test_many_concurrent_transactions | packages/core/__tests__/integration/concurrent_transactions.test.ts | TODO |
| 3 | test_concurrent_transactions_long_lineage | packages/core/__tests__/integration/concurrent_transactions.test.ts | TODO |

### tests/tests/desc_inequality.rs (19 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_desc_inequality_no_equality_prefix | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 2 | test_desc_inequality_single_equality_prefix | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 3 | test_desc_inequality_two_equality_prefix | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 4 | test_desc_inequality_three_equality_prefix | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 5 | test_operator_less_than_desc | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 6 | test_operator_greater_than_desc | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 7 | test_range_inclusive_inclusive | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 8 | test_range_exclusive_exclusive | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 9 | test_range_inclusive_exclusive | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 10 | test_range_exclusive_inclusive | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 11 | test_empty_result_set | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 12 | test_single_result | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 13 | test_duplicate_timestamps | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 14 | test_boundary_at_minimum | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 15 | test_boundary_at_maximum | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 16 | test_asc_ordering_not_broken | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 17 | test_multi_column_order_by | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 18 | test_no_inequality_just_order_by | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |
| 19 | test_regression_pr212_desc_inequality_with_asc_prefix | packages/core/__tests__/integration/desc_inequality.test.ts | TODO |

### tests/tests/inter_node.rs (7 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | inter_node_fetch | packages/core/__tests__/integration/inter_node.test.ts | TODO |
| 2 | server_edits_subscription | packages/core/__tests__/integration/inter_node.test.ts | TODO |
| 3 | test_client_server_propagation | packages/core/__tests__/integration/inter_node.test.ts | TODO |
| 4 | test_client_server_subscription_propagation | packages/core/__tests__/integration/inter_node.test.ts | TODO |
| 5 | test_view_field_subscriptions_with_query_lifecycle | packages/core/__tests__/integration/inter_node.test.ts | TODO |
| 6 | test_lineage_event_bridge | packages/core/__tests__/integration/inter_node.test.ts | TODO |
| 7 | test_fetch_view_field_subscriptions_behavior | packages/core/__tests__/integration/inter_node.test.ts | TODO |

### tests/tests/json_livequery.rs (4 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_json_path_livequery_initial_results | packages/core/__tests__/integration/json_livequery.test.ts | TODO |
| 2 | test_json_path_livequery_with_new_entity | packages/core/__tests__/integration/json_livequery.test.ts | TODO |
| 3 | test_json_path_livequery_with_nested_path | packages/core/__tests__/integration/json_livequery.test.ts | TODO |
| 4 | test_json_path_predicate_reevaluation | packages/core/__tests__/integration/json_livequery.test.ts | TODO |

### tests/tests/limit_gap_filling.rs (4 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_single_node_gap_filling | packages/core/__tests__/integration/limit_gap_filling.test.ts | TODO |
| 2 | test_single_node_multiple_gap_filling | packages/core/__tests__/integration/limit_gap_filling.test.ts | TODO |
| 3 | test_inter_node_gap_filling | packages/core/__tests__/integration/limit_gap_filling.test.ts | TODO |
| 4 | test_inter_node_gap_filling_desc | packages/core/__tests__/integration/limit_gap_filling.test.ts | TODO |

### tests/tests/local_subscription.rs (3 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | basic_local_subscription | packages/core/__tests__/integration/local_subscription.test.ts | TODO |
| 2 | complex_local_subscription | packages/core/__tests__/integration/local_subscription.test.ts | TODO |
| 3 | resultset_vs_livequery_signal_semantics | packages/core/__tests__/integration/local_subscription.test.ts | TODO |

### tests/tests/nonexistent_entity.rs (4 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | get_nonexistent_entity_errors | packages/core/__tests__/integration/nonexistent_entity.test.ts | TODO |
| 2 | local_rejects_phantom_commit | packages/core/__tests__/integration/nonexistent_entity.test.ts | TODO |
| 3 | server_rejects_update_for_nonexistent | packages/core/__tests__/integration/nonexistent_entity.test.ts | TODO |
| 4 | server_rejects_create_for_existing | packages/core/__tests__/integration/nonexistent_entity.test.ts | TODO |

### tests/tests/pagination_cursor.rs (5 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_pagination_cursor_local | packages/core/__tests__/integration/pagination_cursor.test.ts | TODO |
| 2 | test_pagination_forward | packages/core/__tests__/integration/pagination_cursor.test.ts | TODO |
| 3 | test_pagination_inter_node | packages/core/__tests__/integration/pagination_cursor.test.ts | TODO |
| 4 | test_pagination_multi_column_order_by | packages/core/__tests__/integration/pagination_cursor.test.ts | TODO |
| 5 | test_pagination_multi_column_with_equality_prefix | packages/core/__tests__/integration/pagination_cursor.test.ts | TODO |

### tests/tests/policy_angent.rs (2 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | local_access_control | packages/core/__tests__/integration/policy_agent.test.ts | TODO |
| 2 | keeping_peers_honest | packages/core/__tests__/integration/policy_agent.test.ts | TODO |

### tests/tests/predicate_checks.rs (1 test)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_sled_predicate_checks | packages/core/__tests__/integration/predicate_checks.test.ts | TODO |

### tests/tests/property_backends.rs (1 test)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | property_backends | packages/core/__tests__/integration/property_backends.test.ts | TODO |

### tests/tests/repeatable_read.rs (1 test)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | repeatable_read | packages/core/__tests__/integration/repeatable_read.test.ts | TODO |

### tests/tests/rt106.rs (1 test)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | rt106 | packages/core/__tests__/integration/rt106.test.ts | TODO |

### tests/tests/rt114.rs (2 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | rt114 | packages/core/__tests__/integration/rt114.test.ts | TODO |
| 2 | rt114_b | packages/core/__tests__/integration/rt114.test.ts | TODO |

### tests/tests/selection_macro.rs (9 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_selection_macro_unquoted_syntax | packages/core/__tests__/integration/selection_macro.test.ts | TODO |
| 2 | test_selection_macro_in_clause | packages/core/__tests__/integration/selection_macro.test.ts | TODO |
| 3 | test_selection_macro_quoted_syntax | packages/core/__tests__/integration/selection_macro.test.ts | TODO |
| 4 | test_selection_macro_shorthand_syntax | packages/core/__tests__/integration/selection_macro.test.ts | TODO |
| 5 | test_selection_macro_operator_shorthand | packages/core/__tests__/integration/selection_macro.test.ts | TODO |
| 6 | test_selection_macro_syntax_comparison | packages/core/__tests__/integration/selection_macro.test.ts | TODO |
| 7 | test_selection_macro_pure_syntax_forms | packages/core/__tests__/integration/selection_macro.test.ts | TODO |
| 8 | test_selection_macro_edge_cases | packages/core/__tests__/integration/selection_macro.test.ts | TODO |
| 9 | test_selection_macro_list_expansion | packages/core/__tests__/integration/selection_macro.test.ts | TODO |

### tests/tests/system.rs (4 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_system | packages/core/__tests__/integration/system.test.ts | TODO |
| 2 | test_system_ready_behavior | packages/core/__tests__/integration/system.test.ts | TODO |
| 3 | test_system_persistence_across_reconstruction | packages/core/__tests__/integration/system.test.ts | TODO |
| 4 | test_system_root_change_behavior | packages/core/__tests__/integration/system.test.ts | TODO |

### tests/tests/update_predicate.rs (2 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_predicate_update | packages/core/__tests__/integration/update_predicate.test.ts | TODO |
| 2 | test_predicate_update_inter_node | packages/core/__tests__/integration/update_predicate.test.ts | TODO |

### tests/tests/websocket.rs (4 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | test_websocket_client_server_fetch | packages/core/__tests__/integration/websocket.test.ts | TODO |
| 2 | test_websocket_client_create_propagation | packages/core/__tests__/integration/websocket.test.ts | TODO |
| 3 | test_websocket_subscription_propagation | packages/core/__tests__/integration/websocket.test.ts | TODO |
| 4 | test_websocket_bidirectional_subscription | packages/core/__tests__/integration/websocket.test.ts | TODO |

### tests/tests/where_clause.rs (2 tests)

| # | Rust test function | TS target | Status |
|---|-------------------|-----------|--------|
| 1 | basic_where_clause | packages/core/__tests__/integration/where_clause.test.ts | TODO |
| 2 | test_where_clause_with_id | packages/core/__tests__/integration/where_clause.test.ts | TODO |

## Summary

- Source files: 2 (1 skip)
- Test helpers: 1 (skip)
- Integration tests: 90 across 22 test files
