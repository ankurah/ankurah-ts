# ankurah-storage-sqlite — Punchlist

**Rust crate**: `ankurah-storage-sqlite` (`ankurah-ts-support/storage/sqlite/`)
**TS package**: `@ankurah/storage-sqlite` (`packages/storage-sqlite/`)
**Dependencies**: core, storage-common

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | storage/sqlite/src/connection.rs | packages/storage-sqlite/src/connection.ts | TODO |
| 2 | storage/sqlite/src/engine.rs | packages/storage-sqlite/src/engine.ts | TODO |
| 3 | storage/sqlite/src/error.rs | packages/storage-sqlite/src/error.ts | TODO |
| 4 | storage/sqlite/src/lib.rs | packages/storage-sqlite/src/index.ts | TODO |
| 5 | storage/sqlite/src/sql_builder.rs | packages/storage-sqlite/src/sql_builder.ts | TODO |
| 6 | storage/sqlite/src/value.rs | packages/storage-sqlite/src/value.ts | TODO |
| 7 | storage/sqlite/examples/basic.rs | — | SKIP: Example binary, not library code |

## Unit Tests (inline)

### storage/sqlite/src/engine.rs (5 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_open_in_memory | TODO |
| 2 | test_sane_name | TODO |
| 3 | test_jsonb_function_availability | TODO |
| 4 | test_json_path_query | TODO |
| 5 | test_jsonb_storage_and_parameterized_query | TODO |

### storage/sqlite/src/sql_builder.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_simple_equality | TODO |
| 2 | test_and_condition | TODO |
| 3 | test_json_path | TODO |
| 4 | test_json_nested_path | TODO |
| 5 | test_json_numeric_comparison | TODO |
| 6 | test_in_operator | TODO |

## Integration Tests

### storage/sqlite/tests/common/mod.rs

SKIP: Test helper module, not a test file.

### storage/sqlite/tests/basic.rs (5 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_sqlite_create_and_query | TODO |
| 2 | test_sqlite_update_entity | TODO |
| 3 | test_sqlite_state_change_detection | TODO |
| 4 | test_sqlite_multiple_updates | TODO |
| 5 | test_sqlite_query_with_subscription | TODO |

### storage/sqlite/tests/json_property.rs (8 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_json_property_storage_and_simple_query | TODO |
| 2 | test_json_path_pushdown_verification | TODO |
| 3 | test_json_path_query_string_equality | TODO |
| 4 | test_json_path_query_numeric_comparison | TODO |
| 5 | test_json_path_nested_query | TODO |
| 6 | test_json_path_combined_with_regular_field | TODO |
| 7 | test_json_path_query_with_or | TODO |
| 8 | test_json_path_query_numeric_ordering | TODO |

### storage/sqlite/tests/sqlite_json_semantics.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_json_extract_numeric_comparison_is_numeric | TODO |
| 2 | test_json_extract_string_comparison_is_lexicographic | TODO |
| 3 | test_json_extract_cross_type_comparison | TODO |
| 4 | test_json_extract_float_int_comparison | TODO |
| 5 | test_json_extract_null_comparison | TODO |
| 6 | test_json_extract_path_with_comparison | TODO |

### storage/sqlite/tests/sqlite_undefined_column.rs (5 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_undefined_column_in_where | TODO |
| 2 | test_undefined_column_in_order_by | TODO |
| 3 | test_undefined_columns_where_and_order_by | TODO |
| 4 | test_columns_exist_after_write | TODO |
| 5 | test_cache_refresh_after_column_creation | TODO |

## Summary

- Source files: 7 (1 skip)
- Unit tests: 11
- Integration tests: 24
