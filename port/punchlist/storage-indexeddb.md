# ankurah-storage-indexeddb-wasm — Punchlist

**Rust crate**: `ankurah-storage-indexeddb-wasm` (`ankurah-ts-support/storage/indexeddb-wasm/`)
**TS package**: `@ankurah/storage-indexeddb` (`packages/storage-indexeddb/`)
**Dependencies**: core, storage-common

Note: Rust crate targets WASM; TS port is pure TypeScript using browser IndexedDB API directly.

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | storage/indexeddb-wasm/src/collection.rs | packages/storage-indexeddb/src/collection.ts | TODO |
| 2 | storage/indexeddb-wasm/src/database.rs | packages/storage-indexeddb/src/database.ts | TODO |
| 3 | storage/indexeddb-wasm/src/engine.rs | packages/storage-indexeddb/src/engine.ts | TODO |
| 4 | storage/indexeddb-wasm/src/error.rs | packages/storage-indexeddb/src/error.ts | TODO |
| 5 | storage/indexeddb-wasm/src/idb_value.rs | packages/storage-indexeddb/src/idb_value.ts | TODO |
| 6 | storage/indexeddb-wasm/src/lib.rs | packages/storage-indexeddb/src/index.ts | TODO |
| 7 | storage/indexeddb-wasm/src/planner_integration.rs | packages/storage-indexeddb/src/planner_integration.ts | TODO |
| 8 | storage/indexeddb-wasm/src/scanner.rs | packages/storage-indexeddb/src/scanner.ts | TODO |
| 9 | storage/indexeddb-wasm/src/statics.rs | packages/storage-indexeddb/src/statics.ts | TODO |
| 10 | storage/indexeddb-wasm/src/util/cb_future.rs | packages/storage-indexeddb/src/util/cb_future.ts | TODO |
| 11 | storage/indexeddb-wasm/src/util/cb_race.rs | packages/storage-indexeddb/src/util/cb_race.ts | TODO |
| 12 | storage/indexeddb-wasm/src/util/cb_stream.rs | packages/storage-indexeddb/src/util/cb_stream.ts | TODO |
| 13 | storage/indexeddb-wasm/src/util/mod.rs | packages/storage-indexeddb/src/util/index.ts | TODO |
| 14 | storage/indexeddb-wasm/src/util/navigator_lock.rs | packages/storage-indexeddb/src/util/navigator_lock.ts | TODO |
| 15 | storage/indexeddb-wasm/src/util/object.rs | packages/storage-indexeddb/src/util/object.ts | TODO |
| 16 | storage/indexeddb-wasm/src/util/require.rs | packages/storage-indexeddb/src/util/require.ts | TODO |

## Unit Tests (inline)

### storage/indexeddb-wasm/src/idb_value.rs (2 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_safe_integer_range | TODO |
| 2 | test_timestamp_safety | TODO |

### storage/indexeddb-wasm/src/planner_integration.rs (8 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_plan_index_spec_name | TODO |
| 2 | test_scan_direction_to_cursor_direction | TODO |
| 3 | test_normalize_equality_only | TODO |
| 4 | test_normalize_with_inequality | TODO |
| 5 | test_plan_bounds_to_idb_range | TODO |
| 6 | test_plan_bounds_to_idb_range_syntax | TODO |
| 7 | test_plan_bounds_to_idb_range_syntax_equality_only | TODO |
| 8 | test_plan_bounds_to_idb_range_syntax_multi_equality | TODO |

## Integration Tests (WASM — requires browser/jsdom)

Note: These Rust tests run in a WASM environment. TS equivalents need jsdom or a browser test runner.

### storage/indexeddb-wasm/tests/common.rs

SKIP: Test helper module (IDB setup), not a test file.

### storage/indexeddb-wasm/tests/basic.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_indexeddb_basic_workflow | TODO |

### storage/indexeddb-wasm/tests/database_connection.rs (3 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_open_database | TODO |
| 2 | test_multi_connection_versionchange_reconnect | TODO |
| 3 | test_duplicate_index_creation_error_handling | TODO |

### storage/indexeddb-wasm/tests/desc_ordering.rs (8 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_desc_inequality_no_equality_prefix | TODO |
| 2 | test_desc_inequality_single_equality_prefix | TODO |
| 3 | test_desc_inequality_two_equality_prefix_lte | TODO |
| 4 | test_desc_inequality_two_equality_prefix_gte | TODO |
| 5 | test_range_inclusive_inclusive_desc | TODO |
| 6 | test_range_exclusive_exclusive_desc | TODO |
| 7 | test_limit_with_desc_inequality | TODO |
| 8 | test_limit_with_equality_prefix_desc | TODO |

### storage/indexeddb-wasm/tests/duplicate_ref.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_duplicate_ref_type_no_collision | TODO |

### storage/indexeddb-wasm/tests/edge_cases.rs (2 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_edge_cases | TODO |
| 2 | test_prefix_guard_collection_boundary | TODO |

### storage/indexeddb-wasm/tests/idb_value.rs (5 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_i64_positive_safe_range_as_number | TODO |
| 2 | test_i64_positive_beyond_safe_as_string | TODO |
| 3 | test_i64_negative_always_number | TODO |
| 4 | test_i64_string_roundtrip | TODO |
| 5 | test_i64_ordering_across_threshold | TODO |

### storage/indexeddb-wasm/tests/inclusion_and_ordering.rs (5 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_comprehensive_set_inclusion_and_ordering | TODO |
| 2 | test_room_filter_desc_limit | TODO |
| 3 | test_i64_bool_indexing | TODO |
| 4 | test_large_i64_timestamp | TODO |
| 5 | test_equality_prefix_edge_cases | TODO |

### storage/indexeddb-wasm/tests/index_creation.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_index_creation_and_reconnection | TODO |

### storage/indexeddb-wasm/tests/json_property.rs (7 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_json_property_storage_and_simple_query | TODO |
| 2 | test_json_path_query_string_equality | TODO |
| 3 | test_json_path_query_numeric_comparison | TODO |
| 4 | test_json_path_nested_query | TODO |
| 5 | test_json_path_combined_with_regular_field | TODO |
| 6 | test_json_path_missing_field | TODO |
| 7 | test_json_path_planner_generates_sub_path | TODO |

### storage/indexeddb-wasm/tests/multi_column_order_by.rs (21 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_secondary_sort_asc_asc | TODO |
| 2 | test_secondary_sort_desc_desc | TODO |
| 3 | test_secondary_sort_asc_desc | TODO |
| 4 | test_secondary_sort_desc_asc | TODO |
| 5 | test_three_column_order_by | TODO |
| 6 | test_three_column_desc_desc_asc | TODO |
| 7 | test_topk_desc_asc_limit | TODO |
| 8 | test_topk_three_column_asc_asc_desc_limit | TODO |
| 9 | test_topk_three_column_desc_desc_asc_limit | TODO |
| 10 | test_limit_respects_secondary_order_asc | TODO |
| 11 | test_limit_respects_secondary_order_desc | TODO |
| 12 | test_limit_at_category_boundary | TODO |
| 13 | test_inequality_with_secondary_sort | TODO |
| 14 | test_range_with_secondary_sort | TODO |
| 15 | test_equality_prefix_with_secondary_sort_asc | TODO |
| 16 | test_equality_prefix_with_secondary_sort_mixed | TODO |
| 17 | test_equality_prefix_with_duplicate_secondary | TODO |
| 18 | test_empty_result_multi_column_order | TODO |
| 19 | test_single_result_multi_column_order | TODO |
| 20 | test_all_duplicates_primary_same_direction | TODO |
| 21 | test_all_duplicates_primary_mixed_direction | TODO |

### storage/indexeddb-wasm/tests/predicate_checks.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_indexeddb_predicate_checks | TODO |

### storage/indexeddb-wasm/tests/ref_property.rs (2 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_ref_basic_creation_wasm | TODO |
| 2 | test_ref_traversal_wasm | TODO |

## Summary

- Source files: 16
- Unit tests: 10
- Integration tests: 57 (WASM tests — need browser/jsdom environment)
