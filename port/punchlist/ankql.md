# ankql — Punchlist

**Rust crate**: `ankql` (`ankurah-ts-support/ankql/`)
**TS package**: `@ankurah/ankql` (`packages/ankql/`)
**Dependencies**: none

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | ankql/src/ast.rs | packages/ankql/src/ast.ts | DONE |
| 2 | ankql/src/conversion.rs | packages/ankql/src/conversion.ts | DONE |
| 3 | ankql/src/error.rs | packages/ankql/src/error.ts | DONE |
| 4 | ankql/src/grammar.rs | packages/ankql/src/grammar.ts | DONE |
| 5 | ankql/src/lib.rs | packages/ankql/src/index.ts | DONE |
| 6 | ankql/src/parser.rs | packages/ankql/src/parser.ts | DONE |
| 7 | ankql/src/selection.rs | packages/ankql/src/selection/index.ts | DONE |
| 8 | ankql/src/selection/sql.rs | packages/ankql/src/selection/sql.ts | DONE |

## Unit Tests

### ankql/src/ast.rs (9 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_single_comparison_null_handling | DONE |
| 2 | nested_predicate_null_handling | DONE |
| 3 | test_populate_single_placeholder | DONE |
| 4 | test_populate_multiple_placeholders | DONE |
| 5 | test_populate_in_clause | DONE |
| 6 | test_populate_mixed_types | DONE |
| 7 | test_populate_too_few_values | DONE |
| 8 | test_populate_too_many_values | DONE |
| 9 | test_populate_no_placeholders | DONE |

### ankql/src/grammar.rs (10 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_literal_comparison | DONE |
| 2 | test_path_comparison | DONE |
| 3 | test_boolean_expression | DONE |
| 4 | test_boolean_expression_parenthetical | DONE |
| 5 | test_order_by_clause_basic | DONE |
| 6 | test_order_by_clause_with_direction | DONE |
| 7 | test_limit_clause | DONE |
| 8 | test_order_by_and_limit | DONE |
| 9 | test_order_by_multiple_items | DONE |
| 10 | test_pathological_cases | DONE |

### ankql/src/parser.rs (26 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_parse_selection_status_active | DONE |
| 2 | test_parse_selection_user_and_status | DONE |
| 3 | test_parse_selection_user_or_and_status | DONE |
| 4 | test_parse_selection_status_is_null | DONE |
| 5 | test_parse_selection_status_is_not_null | DONE |
| 6 | unary_not_parenthesized | DONE |
| 7 | unary_not_unparenthesized | DONE |
| 8 | test_parse_empty_string | DONE |
| 9 | test_parse_true_literal | DONE |
| 10 | test_parse_selection_in_clause | DONE |
| 11 | test_parse_selection_in_clause_numbers | DONE |
| 12 | test_comparison_to_true | DONE |
| 13 | test_comparison_to_false | DONE |
| 14 | test_comparison_to_left_operand_boolean | DONE |
| 15 | test_placeholders | DONE |
| 16 | test_boolean_literals | DONE |
| 17 | test_order_by_basic | DONE |
| 18 | test_order_by_with_direction | DONE |
| 19 | test_order_by_dotted_identifier_not_supported | DONE |
| 20 | test_limit_basic | DONE |
| 21 | test_order_by_and_limit | DONE |
| 22 | test_limit_only | DONE |
| 23 | test_order_by_only | DONE |
| 24 | test_order_by_multiple_items | DONE |
| 25 | test_pathological_keyword_cases | DONE |
| 26 | test_raw_parsing | DONE |

### ankql/src/selection/sql.rs (14 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_simple_equality | DONE |
| 2 | test_and_condition | DONE |
| 3 | test_complex_condition | DONE |
| 4 | test_including_collection_identifier | DONE |
| 5 | test_in_operator | DONE |
| 6 | test_placeholder_with_none_count | DONE |
| 7 | test_placeholder_with_exact_count | DONE |
| 8 | test_placeholder_count_mismatch_too_few | DONE |
| 9 | test_placeholder_count_mismatch_too_many | DONE |
| 10 | test_placeholder_in_lists | DONE |
| 11 | test_placeholder_with_zero_count | DONE |
| 12 | test_string_escaping | DONE |
| 13 | test_null_byte_handling | DONE |
| 14 | test_placeholder_with_zero_count_but_has_placeholder | DONE |

## Summary

- Source files: 8
- Unit tests: 59
- Integration tests: 0
