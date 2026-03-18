# ankql — Punchlist

**Rust crate**: `ankql` (`ankurah-ts-support/ankql/`)
**TS package**: `@ankurah/ankql` (`packages/ankql/`)
**Dependencies**: none

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | ankql/src/ast.rs | packages/ankql/src/ast.ts | TODO |
| 2 | ankql/src/conversion.rs | packages/ankql/src/conversion.ts | TODO |
| 3 | ankql/src/error.rs | packages/ankql/src/error.ts | TODO |
| 4 | ankql/src/grammar.rs | packages/ankql/src/grammar.ts | TODO |
| 5 | ankql/src/lib.rs | packages/ankql/src/index.ts | TODO |
| 6 | ankql/src/parser.rs | packages/ankql/src/parser.ts | TODO |
| 7 | ankql/src/selection.rs | packages/ankql/src/selection/index.ts | TODO |
| 8 | ankql/src/selection/sql.rs | packages/ankql/src/selection/sql.ts | TODO |

## Unit Tests

### ankql/src/ast.rs (9 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_single_comparison_null_handling | TODO |
| 2 | nested_predicate_null_handling | TODO |
| 3 | test_populate_single_placeholder | TODO |
| 4 | test_populate_multiple_placeholders | TODO |
| 5 | test_populate_in_clause | TODO |
| 6 | test_populate_mixed_types | TODO |
| 7 | test_populate_too_few_values | TODO |
| 8 | test_populate_too_many_values | TODO |
| 9 | test_populate_no_placeholders | TODO |

### ankql/src/grammar.rs (10 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_literal_comparison | TODO |
| 2 | test_path_comparison | TODO |
| 3 | test_boolean_expression | TODO |
| 4 | test_boolean_expression_parenthetical | TODO |
| 5 | test_order_by_clause_basic | TODO |
| 6 | test_order_by_clause_with_direction | TODO |
| 7 | test_limit_clause | TODO |
| 8 | test_order_by_and_limit | TODO |
| 9 | test_order_by_multiple_items | TODO |
| 10 | test_pathological_cases | TODO |

### ankql/src/parser.rs (26 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_parse_selection_status_active | TODO |
| 2 | test_parse_selection_user_and_status | TODO |
| 3 | test_parse_selection_user_or_and_status | TODO |
| 4 | test_parse_selection_status_is_null | TODO |
| 5 | test_parse_selection_status_is_not_null | TODO |
| 6 | unary_not_parenthesized | TODO |
| 7 | unary_not_unparenthesized | TODO |
| 8 | test_parse_empty_string | TODO |
| 9 | test_parse_true_literal | TODO |
| 10 | test_parse_selection_in_clause | TODO |
| 11 | test_parse_selection_in_clause_numbers | TODO |
| 12 | test_comparison_to_true | TODO |
| 13 | test_comparison_to_false | TODO |
| 14 | test_comparison_to_left_operand_boolean | TODO |
| 15 | test_placeholders | TODO |
| 16 | test_boolean_literals | TODO |
| 17 | test_order_by_basic | TODO |
| 18 | test_order_by_with_direction | TODO |
| 19 | test_order_by_dotted_identifier_not_supported | TODO |
| 20 | test_limit_basic | TODO |
| 21 | test_order_by_and_limit | TODO |
| 22 | test_limit_only | TODO |
| 23 | test_order_by_only | TODO |
| 24 | test_order_by_multiple_items | TODO |
| 25 | test_pathological_keyword_cases | TODO |
| 26 | test_raw_parsing | TODO |

### ankql/src/selection/sql.rs (14 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_simple_equality | TODO |
| 2 | test_and_condition | TODO |
| 3 | test_complex_condition | TODO |
| 4 | test_including_collection_identifier | TODO |
| 5 | test_in_operator | TODO |
| 6 | test_placeholder_with_none_count | TODO |
| 7 | test_placeholder_with_exact_count | TODO |
| 8 | test_placeholder_count_mismatch_too_few | TODO |
| 9 | test_placeholder_count_mismatch_too_many | TODO |
| 10 | test_placeholder_in_lists | TODO |
| 11 | test_placeholder_with_zero_count | TODO |
| 12 | test_string_escaping | TODO |
| 13 | test_null_byte_handling | TODO |
| 14 | test_placeholder_with_zero_count_but_has_placeholder | TODO |

## Summary

- Source files: 8
- Unit tests: 59
- Integration tests: 0
