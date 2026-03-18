# ankurah-storage-common — Punchlist

**Rust crate**: `ankurah-storage-common` (`ankurah-ts-support/storage/common/`)
**TS package**: `@ankurah/storage-common` (`packages/storage-common/`)
**Dependencies**: ankql, proto

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | storage/common/src/bounds.rs | packages/storage-common/src/bounds.ts | DONE |
| 2 | storage/common/src/filtering.rs | packages/storage-common/src/filtering.ts | DONE |
| 3 | storage/common/src/lib.rs | packages/storage-common/src/index.ts | DONE |
| 4 | storage/common/src/planner.rs | packages/storage-common/src/planner.ts | DONE |
| 5 | storage/common/src/predicate.rs | packages/storage-common/src/predicate.ts | DONE |
| 6 | storage/common/src/sorting.rs | packages/storage-common/src/sorting.ts | DONE |
| 7 | storage/common/src/traits.rs | packages/storage-common/src/traits.ts | DONE |
| 8 | storage/common/src/types.rs | packages/storage-common/src/types.ts | DONE |

## Unit Tests

### storage/common/src/predicate.rs (8 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_single_comparison | DONE |
| 2 | test_simple_and | DONE |
| 3 | test_nested_and | DONE |
| 4 | test_or_blocks_conjunct_extraction | DONE |
| 5 | test_and_with_or_mixed | DONE |
| 6 | test_complex_nested_example | DONE |
| 7 | test_non_comparison_predicates | DONE |
| 8 | test_true_false_predicates | DONE |

### storage/common/src/planner.rs (71 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | basic_order_by | DONE |
| 2 | order_by_with_covered_inequality | DONE |
| 3 | no_collection_field | DONE |
| 4 | test_order_by_with_equality | DONE |
| 5 | test_order_by_desc_single_field | DONE |
| 6 | test_order_by_all_desc | DONE |
| 7 | test_order_by_mixed_directions_asc_first | DONE |
| 8 | test_order_by_mixed_directions_desc_first | DONE |
| 9 | test_order_by_three_asc_desc_desc | DONE |
| 10 | test_order_by_three_desc_desc_asc | DONE |
| 11 | test_order_by_three_asc_asc_asc | DONE |
| 12 | test_order_by_three_asc_asc_desc | DONE |
| 13 | test_order_by_three_asc_desc_asc | DONE |
| 14 | test_order_by_three_desc_asc_asc | DONE |
| 15 | test_order_by_three_desc_asc_desc | DONE |
| 16 | test_order_by_three_desc_desc_desc | DONE |
| 17 | test_order_by_with_equality_and_desc | DONE |
| 18 | test_order_by_with_inequality_and_desc | DONE |
| 19 | test_full_support_single_desc | DONE |
| 20 | test_full_support_mixed_directions | DONE |
| 21 | test_full_support_all_desc | DONE |
| 22 | test_full_support_with_equality_and_mixed_order | DONE |
| 23 | test_single_inequality_plan_structure | DONE |
| 24 | test_multiple_inequalities_same_field_plan_structure | DONE |
| 25 | test_multiple_inequalities_different_fields_plan_structures | DONE |
| 26 | test_greater_or_equal_inclusive_lower_bound | DONE |
| 27 | test_less_than_exclusive_upper_bound | DONE |
| 28 | test_less_or_equal_inclusive_upper_bound | DONE |
| 29 | test_range_inclusive_both | DONE |
| 30 | test_range_mixed_gte_lt | DONE |
| 31 | test_range_mixed_gt_lte | DONE |
| 32 | test_gte_with_desc_order_by | DONE |
| 33 | test_lte_with_desc_order_by | DONE |
| 34 | test_single_equality_plan_structure | DONE |
| 35 | test_multiple_equalities_plan_structure | DONE |
| 36 | test_four_column_equality_prefix | DONE |
| 37 | test_three_equality_with_order_by | DONE |
| 38 | test_three_equality_with_inequality | DONE |
| 39 | test_equality_with_inequality_plan_structure | DONE |
| 40 | test_equality_with_order_by_and_matching_inequality | DONE |
| 41 | test_collection_only_query | DONE |
| 42 | test_unsupported_operators | DONE |
| 43 | test_impossible_range | DONE |
| 44 | test_or_only_predicate | DONE |
| 45 | test_complex_nested_predicate | DONE |
| 46 | test_order_by_with_no_matching_predicate | DONE |
| 47 | test_inequality_different_field_than_order_by | DONE |
| 48 | test_multiple_inequalities_same_field_complex | DONE |
| 49 | test_large_numbers | DONE |
| 50 | test_empty_string_equality | DONE |
| 51 | test_empty_string_with_other_fields | DONE |
| 52 | test_primary_key_only_equality | DONE |
| 53 | test_primary_key_only_with_order_by | DONE |
| 54 | test_primary_key_with_non_primary_order_by | DONE |
| 55 | test_primary_key_not_equal | DONE |
| 56 | test_no_predicate_no_order_by | DONE |
| 57 | test_no_predicate_with_order_by | DONE |
| 58 | test_primary_key_range_intersection | DONE |
| 59 | test_mixed_primary_and_secondary_predicates | DONE |
| 60 | test_json_path_equality | DONE |
| 61 | test_json_path_with_order_by | DONE |
| 62 | test_deep_json_path | DONE |
| 63 | test_json_path_full_pushdown | DONE |
| 64 | test_json_path_inequality | DONE |
| 65 | test_json_path_mixed_predicates | DONE |
| 66 | test_spill_preserves_column_order | DONE |
| 67 | test_spill_preserves_directions | DONE |
| 68 | test_spill_with_limit | DONE |
| 69 | test_table_scan_spill_matches_full_order_by | DONE |
| 70 | test_no_spill_when_fully_satisfied | DONE |
| 71 | test_equality_prefix_affects_spill | DONE |

### storage/common/src/sorting.rs (27 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_limited_stream_basic | DONE |
| 2 | test_limited_stream_no_limit | DONE |
| 3 | test_limited_stream_limit_exceeds_items | DONE |
| 4 | test_limited_stream_zero_limit | DONE |
| 5 | test_limited_stream_empty_input | DONE |
| 6 | test_sorted_stream_global_sort_asc | DONE |
| 7 | test_sorted_stream_global_sort_desc | DONE |
| 8 | test_sorted_stream_global_sort_multi_column | DONE |
| 9 | test_sorted_stream_empty_input | DONE |
| 10 | test_sorted_stream_single_item | DONE |
| 11 | test_sorted_stream_partition_aware_basic | DONE |
| 12 | test_sorted_stream_partition_aware_mixed_directions | DONE |
| 13 | test_sorted_stream_partition_aware_single_partition | DONE |
| 14 | test_sorted_stream_partition_aware_single_item_partitions | DONE |
| 15 | test_sorted_stream_partition_aware_empty_spill | DONE |
| 16 | test_topk_stream_global_basic | DONE |
| 17 | test_topk_stream_global_desc | DONE |
| 18 | test_topk_stream_global_k_exceeds_items | DONE |
| 19 | test_topk_stream_global_k_zero | DONE |
| 20 | test_topk_stream_global_empty_input | DONE |
| 21 | test_topk_stream_partition_aware_basic | DONE |
| 22 | test_topk_stream_partition_aware_limit_within_partition | DONE |
| 23 | test_topk_stream_partition_aware_mixed_directions | DONE |
| 24 | test_sorted_stream_null_sorts_first_asc | DONE |
| 25 | test_sorted_stream_null_sorts_first_desc | DONE |
| 26 | test_sorted_stream_all_nulls | DONE |
| 27 | test_sorted_stream_multi_column_presort | DONE |

## Summary

- Source files: 8
- Unit tests: 106 (8 predicate + 71 planner + 27 sorting)
- Integration tests: 0
