# ankurah-storage-postgres — Punchlist

**Rust crate**: `ankurah-storage-postgres` (`ankurah-ts-support/storage/postgres/`)
**TS package**: `@ankurah/storage-postgres` (`packages/storage-postgres/`)
**Dependencies**: core, storage-common

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | storage/postgres/src/lib.rs | packages/storage-postgres/src/index.ts | TODO |
| 2 | storage/postgres/src/sql_builder.rs | packages/storage-postgres/src/sql_builder.ts | TODO |
| 3 | storage/postgres/src/value.rs | packages/storage-postgres/src/value.ts | TODO |

## Unit Tests (inline)

### storage/postgres/src/sql_builder.rs (26 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_simple_equality | TODO |
| 2 | test_and_condition | TODO |
| 3 | test_complex_condition | TODO |
| 4 | test_including_collection_identifier | TODO |
| 5 | test_false_predicate | TODO |
| 6 | test_in_operator | TODO |
| 7 | test_placeholder_error | TODO |
| 8 | test_selection_with_order_by | TODO |
| 9 | test_selection_with_limit | TODO |
| 10 | test_selection_with_order_by_and_limit | TODO |
| 11 | test_two_step_json_path | TODO |
| 12 | test_three_step_json_path | TODO |
| 13 | test_four_step_json_path | TODO |
| 14 | test_json_path_with_numeric_comparison | TODO |
| 15 | test_mixed_simple_and_json_paths | TODO |
| 16 | test_json_path_escaping | TODO |
| 17 | test_json_path_with_boolean | TODO |
| 18 | test_json_path_with_float | TODO |
| 19 | test_simple_predicate_fully_pushable | TODO |
| 20 | test_json_path_predicate_pushable | TODO |
| 21 | test_and_with_all_pushable | TODO |
| 22 | test_or_with_all_pushable | TODO |
| 23 | test_complex_nested_predicate | TODO |
| 24 | test_not_predicate_pushable | TODO |
| 25 | test_is_null_pushable | TODO |
| 26 | test_unpushable_predicate_goes_to_remaining | TODO |

## Integration Tests

### storage/postgres/tests/common/mod.rs

SKIP: Test helper module (Docker/connection setup), not a test file.

### storage/postgres/tests/add_event.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | add_event_postgres | TODO |

### storage/postgres/tests/basic.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_postgres | TODO |

### storage/postgres/tests/json_property.rs (7 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_json_property_storage_and_simple_query | TODO |
| 2 | test_bytea_jsonb_operator_behavior | TODO |
| 3 | test_json_path_pushdown_verification | TODO |
| 4 | test_json_path_query_string_equality | TODO |
| 5 | test_json_path_query_numeric_comparison | TODO |
| 6 | test_json_path_nested_query | TODO |
| 7 | test_json_path_combined_with_regular_field | TODO |

### storage/postgres/tests/jsonb_semantics.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_jsonb_numeric_comparison_is_numeric | TODO |
| 2 | test_jsonb_string_comparison_is_lexicographic | TODO |
| 3 | test_jsonb_cross_type_comparison_returns_false | TODO |
| 4 | test_jsonb_float_int_comparison | TODO |
| 5 | test_jsonb_null_comparison | TODO |
| 6 | test_jsonb_path_extraction_with_comparison | TODO |

### storage/postgres/tests/predicate_checks.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_postgres_predicate_checks | TODO |

### storage/postgres/tests/property_backends.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | pg_property_backends | TODO |

### storage/postgres/tests/repeatable_read.rs (2 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | pg_repeatable_read | TODO |
| 2 | pg_events | TODO |

### storage/postgres/tests/rt165.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | postgres_duplicate_event_idempotency | TODO |

### storage/postgres/tests/rt176.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | postgres_get_state_returns_entity_not_found | TODO |

### storage/postgres/tests/undefined_column.rs (5 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_undefined_column_in_where | TODO |
| 2 | test_undefined_column_in_order_by | TODO |
| 3 | test_undefined_columns_where_and_order_by | TODO |
| 4 | test_columns_exist_after_write | TODO |
| 5 | test_cache_refresh_after_column_creation | TODO |

### storage/postgres/tests/where_clause.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | pg_basic_where_clause | TODO |

## Summary

- Source files: 3
- Unit tests: 26
- Integration tests: 27
