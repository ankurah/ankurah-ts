# ankurah-storage-common — Punchlist

**Rust crate**: `ankurah-storage-common` (`ankurah-ts-support/storage/common/`)
**TS package**: `@ankurah/storage-common` (`packages/storage-common/`)
**Dependencies**: ankql, proto

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | storage/common/src/bounds.rs | packages/storage-common/src/bounds.ts | TODO |
| 2 | storage/common/src/filtering.rs | packages/storage-common/src/filtering.ts | TODO |
| 3 | storage/common/src/lib.rs | packages/storage-common/src/index.ts | TODO |
| 4 | storage/common/src/planner.rs | packages/storage-common/src/planner.ts | TODO |
| 5 | storage/common/src/predicate.rs | packages/storage-common/src/predicate.ts | TODO |
| 6 | storage/common/src/sorting.rs | packages/storage-common/src/sorting.ts | TODO |
| 7 | storage/common/src/traits.rs | packages/storage-common/src/traits.ts | TODO |
| 8 | storage/common/src/types.rs | packages/storage-common/src/types.ts | TODO |

## Unit Tests

### storage/common/src/predicate.rs (8 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_single_comparison | TODO |
| 2 | test_simple_and | TODO |
| 3 | test_nested_and | TODO |
| 4 | test_or_blocks_conjunct_extraction | TODO |
| 5 | test_and_with_or_mixed | TODO |
| 6 | test_complex_nested_example | TODO |
| 7 | test_non_comparison_predicates | TODO |
| 8 | test_true_false_predicates | TODO |

### storage/common/src/planner.rs (71 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | basic_order_by | TODO |
| 2 | order_by_with_covered_inequality | TODO |
| 3 | no_collection_field | TODO |
| 4 | test_order_by_with_equality | TODO |
| 5 | test_order_by_desc_single_field | TODO |
| 6 | test_order_by_all_desc | TODO |
| 7 | test_order_by_mixed_directions_asc_first | TODO |
| 8 | test_order_by_mixed_directions_desc_first | TODO |
| 9 | test_order_by_three_asc_desc_desc | TODO |
| 10 | test_order_by_three_desc_desc_asc | TODO |
| 11 | test_order_by_three_asc_asc_asc | TODO |
| 12 | test_order_by_three_asc_asc_desc | TODO |
| 13 | test_order_by_three_asc_desc_asc | TODO |
| 14 | test_order_by_three_desc_asc_asc | TODO |
| 15 | test_order_by_three_desc_asc_desc | TODO |
| 16 | test_order_by_three_desc_desc_desc | TODO |
| 17 | test_order_by_with_equality_and_desc | TODO |
| 18 | test_order_by_with_inequality_and_desc | TODO |
| 19 | test_full_support_single_desc | TODO |
| 20 | test_full_support_mixed_directions | TODO |
| 21 | test_full_support_all_desc | TODO |
| 22 | test_full_support_with_equality_and_mixed_order | TODO |
| 23 | test_single_inequality_plan_structure | TODO |
| 24 | test_multiple_inequalities_same_field_plan_structure | TODO |
| 25 | test_multiple_inequalities_different_fields_plan_structures | TODO |
| 26 | test_greater_or_equal_inclusive_lower_bound | TODO |
| 27 | test_less_than_exclusive_upper_bound | TODO |
| 28 | test_less_or_equal_inclusive_upper_bound | TODO |
| 29 | test_range_inclusive_both | TODO |
| 30 | test_range_mixed_gte_lt | TODO |
| 31 | test_range_mixed_gt_lte | TODO |
| 32 | test_gte_with_desc_order_by | TODO |
| 33 | test_lte_with_desc_order_by | TODO |
| 34 | test_single_equality_plan_structure | TODO |
| 35 | test_multiple_equalities_plan_structure | TODO |
| 36 | test_four_column_equality_prefix | TODO |
| 37 | test_three_equality_with_order_by | TODO |
| 38 | test_three_equality_with_inequality | TODO |
| 39 | test_equality_with_inequality_plan_structure | TODO |
| 40 | test_equality_with_order_by_and_matching_inequality | TODO |
| 41 | test_collection_only_query | TODO |
| 42 | test_unsupported_operators | TODO |
| 43 | test_impossible_range | TODO |
| 44 | test_or_only_predicate | TODO |
| 45 | test_complex_nested_predicate | TODO |
| 46 | test_order_by_with_no_matching_predicate | TODO |
| 47 | test_inequality_different_field_than_order_by | TODO |
| 48 | test_multiple_inequalities_same_field_complex | TODO |
| 49 | test_large_numbers | TODO |
| 50 | test_empty_string_equality | TODO |
| 51 | test_empty_string_with_other_fields | TODO |
| 52 | test_primary_key_only_equality | TODO |
| 53 | test_primary_key_only_with_order_by | TODO |
| 54 | test_primary_key_with_non_primary_order_by | TODO |
| 55 | test_primary_key_not_equal | TODO |
| 56 | test_no_predicate_no_order_by | TODO |
| 57 | test_no_predicate_with_order_by | TODO |
| 58 | test_primary_key_range_intersection | TODO |
| 59 | test_mixed_primary_and_secondary_predicates | TODO |
| 60 | test_json_path_equality | TODO |
| 61 | test_json_path_with_order_by | TODO |
| 62 | test_deep_json_path | TODO |
| 63 | test_json_path_full_pushdown | TODO |
| 64 | test_json_path_inequality | TODO |
| 65 | test_json_path_mixed_predicates | TODO |
| 66 | test_spill_preserves_column_order | TODO |
| 67 | test_spill_preserves_directions | TODO |
| 68 | test_spill_with_limit | TODO |
| 69 | test_table_scan_spill_matches_full_order_by | TODO |
| 70 | test_no_spill_when_fully_satisfied | TODO |
| 71 | test_equality_prefix_affects_spill | TODO |

### storage/common/src/sorting.rs (27 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_limited_stream_basic | TODO |
| 2 | test_limited_stream_no_limit | TODO |
| 3 | test_limited_stream_limit_exceeds_items | TODO |
| 4 | test_limited_stream_zero_limit | TODO |
| 5 | test_limited_stream_empty_input | TODO |
| 6 | test_sorted_stream_global_sort_asc | TODO |
| 7 | test_sorted_stream_global_sort_desc | TODO |
| 8 | test_sorted_stream_global_sort_multi_column | TODO |
| 9 | test_sorted_stream_empty_input | TODO |
| 10 | test_sorted_stream_single_item | TODO |
| 11 | test_sorted_stream_partition_aware_basic | TODO |
| 12 | test_sorted_stream_partition_aware_mixed_directions | TODO |
| 13 | test_sorted_stream_partition_aware_single_partition | TODO |
| 14 | test_sorted_stream_partition_aware_single_item_partitions | TODO |
| 15 | test_sorted_stream_partition_aware_empty_spill | TODO |
| 16 | test_topk_stream_global_basic | TODO |
| 17 | test_topk_stream_global_desc | TODO |
| 18 | test_topk_stream_global_k_exceeds_items | TODO |
| 19 | test_topk_stream_global_k_zero | TODO |
| 20 | test_topk_stream_global_empty_input | TODO |
| 21 | test_topk_stream_partition_aware_basic | TODO |
| 22 | test_topk_stream_partition_aware_limit_within_partition | TODO |
| 23 | test_topk_stream_partition_aware_mixed_directions | TODO |
| 24 | test_sorted_stream_null_sorts_first_asc | TODO |
| 25 | test_sorted_stream_null_sorts_first_desc | TODO |
| 26 | test_sorted_stream_all_nulls | TODO |
| 27 | test_sorted_stream_multi_column_presort | TODO |

## Summary

- Source files: 8
- Unit tests: 106 (8 predicate + 71 planner + 27 sorting)
- Integration tests: 0
