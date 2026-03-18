# ankurah-core — Punchlist

**Rust crate**: `ankurah-core` (`ankurah-ts-support/core/`)
**TS package**: `@ankurah/core` (`packages/core/`)
**Dependencies**: ankql, proto, signals, storage-common

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | core/src/changes.rs | packages/core/src/changes.ts | DONE |
| 2 | core/src/collation.rs | packages/core/src/collation.ts | DONE |
| 3 | core/src/collectionset.rs | packages/core/src/collectionset.ts | DONE |
| 4 | core/src/connector.rs | packages/core/src/connector.ts | DONE |
| 5 | core/src/context.rs | packages/core/src/context.ts | DONE |
| 6 | core/src/entity.rs | packages/core/src/entity.ts | DONE |
| 7 | core/src/error.rs | packages/core/src/error.ts | DONE |
| 8 | core/src/indexing/encoding.rs | packages/core/src/indexing/encoding.ts | DONE |
| 9 | core/src/indexing/key_spec.rs | packages/core/src/indexing/key_spec.ts | DONE |
| 10 | core/src/indexing/mod.rs | packages/core/src/indexing/index.ts | DONE |
| 11 | core/src/lib.rs | packages/core/src/index.ts | DONE |
| 12 | core/src/lineage.rs | packages/core/src/lineage.ts | DONE |
| 13 | core/src/livequery.rs | packages/core/src/livequery.ts | DONE |
| 14 | core/src/model.rs | packages/core/src/model.ts | DONE |
| 15 | core/src/model/tsify.rs | — | SKIP: Rust proc-macro tsify integration, replaced by defineModel() (E12) |
| 16 | core/src/node.rs | packages/core/src/node.ts | DONE |
| 17 | core/src/node_applier.rs | packages/core/src/node_applier.ts | DONE |
| 18 | core/src/peer_subscription/client_relay.rs | packages/core/src/peer_subscription/client_relay.ts | DONE |
| 19 | core/src/peer_subscription/mod.rs | packages/core/src/peer_subscription/index.ts | DONE |
| 20 | core/src/peer_subscription/server.rs | packages/core/src/peer_subscription/server.ts | DONE |
| 21 | core/src/policy.rs | packages/core/src/policy.ts | DONE |
| 22 | core/src/property/backend/lww.rs | packages/core/src/property/backend/lww.ts | DONE |
| 23 | core/src/property/backend/mod.rs | packages/core/src/property/backend/index.ts | DONE |
| 24 | core/src/property/backend/pn_counter.rs | packages/core/src/property/backend/pn_counter.ts | DONE |
| 25 | core/src/property/backend/yrs.rs | packages/core/src/property/backend/yjs.ts | DONE |
| 26 | core/src/property/mod.rs | packages/core/src/property/index.ts | DONE |
| 27 | core/src/property/traits.rs | packages/core/src/property/traits.ts | DONE |
| 28 | core/src/property/value/entity_ref.rs | packages/core/src/property/value/entity_ref.ts | DONE |
| 29 | core/src/property/value/json.rs | packages/core/src/property/value/json.ts | DONE |
| 30 | core/src/property/value/lww.rs | packages/core/src/property/value/lww.ts | DONE |
| 31 | core/src/property/value/mod.rs | packages/core/src/property/value/index.ts | DONE |
| 32 | core/src/property/value/pn_counter.rs | packages/core/src/property/value/pn_counter.ts | DONE |
| 33 | core/src/property/value/yrs.rs | packages/core/src/property/value/yrs_string.ts | DONE |
| 34 | core/src/query_value.rs | packages/core/src/query_value.ts | DONE |
| 35 | core/src/reactor.rs | packages/core/src/reactor/index.ts | DONE |
| 36 | core/src/reactor/candidate_changes.rs | packages/core/src/reactor/candidate_changes.ts | DONE |
| 37 | core/src/reactor/comparison_index.rs | packages/core/src/reactor/comparison_index.ts | DONE |
| 38 | core/src/reactor/fetch_gap.rs | packages/core/src/reactor/fetch_gap.ts | DONE |
| 39 | core/src/reactor/property_path.rs | packages/core/src/reactor/property_path.ts | DONE |
| 40 | core/src/reactor/subscription.rs | packages/core/src/reactor/subscription.ts | DONE |
| 41 | core/src/reactor/subscription_state.rs | packages/core/src/reactor/subscription_state.ts | DONE |
| 42 | core/src/reactor/update.rs | packages/core/src/reactor/update.ts | DONE |
| 43 | core/src/reactor/watcherset.rs | packages/core/src/reactor/watcherset.ts | DONE |
| 44 | core/src/resultset.rs | packages/core/src/resultset.ts | DONE |
| 45 | core/src/retrieval.rs | packages/core/src/retrieval.ts | DONE |
| 46 | core/src/schema.rs | packages/core/src/schema.ts | DONE |
| 47 | core/src/selection/filter.rs | packages/core/src/selection/filter.ts | DONE |
| 48 | core/src/selection/mod.rs | packages/core/src/selection/index.ts | DONE |
| 49 | core/src/storage.rs | packages/core/src/storage.ts | DONE |
| 50 | core/src/system.rs | packages/core/src/system.ts | DONE |
| 51 | core/src/task.rs | packages/core/src/task.ts | DONE |
| 52 | core/src/traits.rs | packages/core/src/traits.ts | DONE |
| 53 | core/src/transaction.rs | packages/core/src/transaction.ts | DONE |
| 54 | core/src/type_resolver.rs | packages/core/src/type_resolver.ts | DONE |
| 55 | core/src/util/cast.rs | packages/core/src/util/cast.ts | DONE |
| 56 | core/src/util/expand_states.rs | packages/core/src/util/expand_states.ts | DONE |
| 57 | core/src/util/iterable.rs | packages/core/src/util/iterable.ts | DONE |
| 58 | core/src/util/ivec.rs | packages/core/src/util/ivec.ts | DONE |
| 59 | core/src/util/mod.rs | packages/core/src/util/index.ts | DONE |
| 60 | core/src/util/ready_chunks.rs | packages/core/src/util/ready_chunks.ts | DONE |
| 61 | core/src/util/safemap.rs | packages/core/src/util/safemap.ts | DONE |
| 62 | core/src/util/safeset.rs | packages/core/src/util/safeset.ts | DONE |
| 63 | core/src/value/cast.rs | packages/core/src/value/cast.ts | DONE |
| 64 | core/src/value/cast_predicate.rs | packages/core/src/value/cast_predicate.ts | DONE |
| 65 | core/src/value/collatable.rs | packages/core/src/value/collatable.ts | DONE |
| 66 | core/src/value/mod.rs | packages/core/src/value/index.ts | DONE |
| 67 | core/src/value/wasm.rs | — | SKIP: WASM bindings (E9) |

## Unit Tests

### core/src/collation.rs (9 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_string_collation | DONE |
| 2 | test_integer_collation | DONE |
| 3 | test_float_collation | DONE |
| 4 | test_range_bounds | DONE |
| 5 | test_literal_i16_collation | DONE |
| 6 | test_literal_i32_collation | DONE |
| 7 | test_literal_entity_id_collation | DONE |
| 8 | test_literal_binary_collation | DONE |
| 9 | test_literal_object_collation | DONE |

### core/src/indexing/encoding.rs (2 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_desc_ordering | DONE |
| 2 | test_asc_ordering | DONE |

### core/src/indexing/key_spec.rs (13 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_exact_match | DONE |
| 2 | test_prefix_match | DONE |
| 3 | test_inverse_exact_match | DONE |
| 4 | test_inverse_prefix_match | DONE |
| 5 | test_user_example | DONE |
| 6 | test_no_match_different_fields | DONE |
| 7 | test_no_match_partial_field_overlap | DONE |
| 8 | test_no_match_query_longer_than_index | DONE |
| 9 | test_empty_specs | DONE |
| 10 | test_single_field_cases | DONE |
| 11 | test_complex_multi_field_scenarios | DONE |
| 12 | test_helper_methods | DONE |
| 13 | test_edge_case_behaviors | DONE |

### core/src/lineage.rs (13 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_linear_history | DONE |
| 2 | test_concurrent_history | DONE |
| 3 | test_incomparable | DONE |
| 4 | test_empty_clocks | DONE |
| 5 | test_budget_exceeded | DONE |
| 6 | test_self_comparison | DONE |
| 7 | multiple_roots | DONE |
| 8 | test_compare_event_unstored | DONE |
| 9 | test_compare_event_redundant_delivery | DONE |
| 10 | test_event_accumulator | DONE |
| 11 | test_event_accumulator_with_concurrent_history | DONE |
| 12 | test_event_accumulator_equal_clocks | DONE |
| 13 | test_event_accumulator_only_subject_side | DONE |

### core/src/peer_subscription/client_relay.rs (8 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_new_subscription_setup | DONE |
| 2 | test_peer_disconnection_orphans_subscriptions | DONE |
| 3 | test_peer_connection_triggers_setup | DONE |
| 4 | test_failed_subscription_retry | DONE |
| 5 | test_retryable_vs_non_retryable_failures | DONE |
| 6 | test_subscription_removal | DONE |
| 7 | test_edge_cases | DONE |
| 8 | test_notify_unsubscribe_with_no_established_subscription | DONE |

### core/src/property/value/entity_ref.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_ref_roundtrip | DONE |
| 2 | test_ref_from_entity_id | DONE |
| 3 | test_ref_into_entity_id | DONE |
| 4 | test_ref_missing | DONE |
| 5 | test_ref_invalid_string | DONE |
| 6 | test_ref_invalid_variant | DONE |

### core/src/property/value/json.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_json_roundtrip | DONE |
| 2 | test_json_get_path | DONE |
| 3 | test_json_null | DONE |
| 4 | test_json_missing | DONE |
| 5 | test_json_invalid_variant | DONE |
| 6 | test_json_deref | DONE |

### core/src/reactor.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_entity_remains_watched_after_predicate_stops_matching | DONE |

### core/src/reactor/candidate_changes.rs (3 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_candidate_changes_empty | DONE |
| 2 | test_candidate_changes_add_query | DONE |
| 3 | test_candidate_changes_entity_level | DONE |

### core/src/reactor/comparison_index.rs (2 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_field_index | DONE |
| 2 | test_field_index_not_equal | DONE |

### core/src/reactor/fetch_gap.rs (3 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_build_gap_predicate_single_column_asc | DONE |
| 2 | test_build_gap_predicate_multi_column | DONE |
| 3 | test_infer_value_type_for_field | DONE |

### core/src/resultset.rs (8 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_entity_id_ordering | DONE |
| 2 | test_order_by_with_tie_breaking | DONE |
| 3 | test_limit_functionality | DONE |
| 4 | test_dirty_tracking | DONE |
| 5 | test_write_guard_atomic_operations | DONE |
| 6 | test_ivec_small_keys | SKIP: Rust-internal IVec Small/Large enum optimization; TS uses plain Uint8Array |
| 7 | test_ivec_large_keys | SKIP: same as above |
| 8 | test_ivec_boundary | SKIP: same as above |

### core/src/selection/filter.rs (21 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_simple_equality | DONE |
| 2 | test_and_condition | DONE |
| 3 | test_complex_condition | DONE |
| 4 | test_in_operator | DONE |
| 5 | test_simple_json_path | DONE |
| 6 | test_nested_json_path | DONE |
| 7 | test_json_path_with_numeric_value | DONE |
| 8 | test_json_path_with_boolean | DONE |
| 9 | test_json_path_not_found | DONE |
| 10 | test_json_path_combined_with_regular_field | DONE |
| 11 | test_traverse_into_non_json_property_errors | DONE |
| 12 | test_json_path_with_or | DONE |
| 13 | test_json_path_with_in_operator | DONE |
| 14 | test_collection_qualified_json_path | DONE |
| 15 | test_json_numeric_casting_same_type | DONE |
| 16 | test_json_numeric_casting_float_to_int | DONE |
| 17 | test_json_string_to_number_no_cast | DONE |
| 18 | test_json_number_to_string_no_cast | DONE |
| 19 | test_json_string_equality_works | DONE |
| 20 | test_json_comparison_operators | DONE |
| 21 | test_regular_field_still_casts_string_to_number | DONE |

### core/src/type_resolver.rs (7 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_resolve_simple_path | DONE |
| 2 | test_resolve_id_path | DONE |
| 3 | test_resolve_json_path | DONE |
| 4 | test_literal_to_json_string | DONE |
| 5 | test_literal_to_json_number | DONE |
| 6 | test_resolve_types_converts_literal_for_json_path | DONE |
| 7 | test_resolve_types_leaves_simple_path_literal_alone | DONE |

### core/src/util/ivec.rs (9 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_small_push | DONE |
| 2 | test_transition_to_large | DONE |
| 3 | test_contains | DONE |
| 4 | test_contains_large | DONE |
| 5 | test_iter | DONE |
| 6 | test_iter_large | DONE |
| 7 | test_drop | DONE |
| 8 | test_add | DONE |
| 9 | test_add_large | DONE |

### core/src/value/cast.rs (9 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_string_to_entity_id | DONE |
| 2 | test_entity_id_to_string | DONE |
| 3 | test_invalid_entity_id_string | DONE |
| 4 | test_numeric_upcasting | DONE |
| 5 | test_numeric_downcasting | DONE |
| 6 | test_string_to_numeric | DONE |
| 7 | test_string_to_bool | DONE |
| 8 | test_incompatible_types | DONE |
| 9 | test_same_type_cast | DONE |

### core/src/value/cast_predicate.rs (3 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_cast_id_field_string_to_entity_id | DONE |
| 2 | test_cast_literal_equals_field | DONE |
| 3 | test_cast_complex_predicate | DONE |

### core/src/value/mod.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_extract_at_path_empty | DONE |
| 2 | test_extract_at_path_json_string | DONE |
| 3 | test_extract_at_path_json_number | DONE |
| 4 | test_extract_at_path_json_nested | DONE |
| 5 | test_extract_at_path_missing | DONE |
| 6 | test_extract_at_path_non_json | DONE |

## Summary

- Source files: 67 (3 skip)
- Unit tests: 130 (3 skip: ivec internal tests)
- Integration tests: 0 (core integration tests are in the `tests` crate — see tests.md)
