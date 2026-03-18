# ankurah-core — Punchlist

**Rust crate**: `ankurah-core` (`ankurah-ts-support/core/`)
**TS package**: `@ankurah/core` (`packages/core/`)
**Dependencies**: ankql, proto, signals, storage-common

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | core/src/changes.rs | packages/core/src/changes.ts | TODO |
| 2 | core/src/collation.rs | packages/core/src/collation.ts | TODO |
| 3 | core/src/collectionset.rs | packages/core/src/collectionset.ts | TODO |
| 4 | core/src/connector.rs | packages/core/src/connector.ts | TODO |
| 5 | core/src/context.rs | packages/core/src/context.ts | TODO |
| 6 | core/src/entity.rs | packages/core/src/entity.ts | TODO |
| 7 | core/src/error.rs | packages/core/src/error.ts | TODO |
| 8 | core/src/indexing/encoding.rs | packages/core/src/indexing/encoding.ts | TODO |
| 9 | core/src/indexing/key_spec.rs | packages/core/src/indexing/key_spec.ts | TODO |
| 10 | core/src/indexing/mod.rs | packages/core/src/indexing/index.ts | TODO |
| 11 | core/src/lib.rs | packages/core/src/index.ts | TODO |
| 12 | core/src/lineage.rs | packages/core/src/lineage.ts | TODO |
| 13 | core/src/livequery.rs | packages/core/src/livequery.ts | TODO |
| 14 | core/src/model.rs | packages/core/src/model.ts | TODO |
| 15 | core/src/model/tsify.rs | — | SKIP: Rust proc-macro tsify integration, replaced by defineModel() (E12) |
| 16 | core/src/node.rs | packages/core/src/node.ts | TODO |
| 17 | core/src/node_applier.rs | packages/core/src/node_applier.ts | TODO |
| 18 | core/src/peer_subscription/client_relay.rs | packages/core/src/peer_subscription/client_relay.ts | TODO |
| 19 | core/src/peer_subscription/mod.rs | packages/core/src/peer_subscription/index.ts | TODO |
| 20 | core/src/peer_subscription/server.rs | packages/core/src/peer_subscription/server.ts | TODO |
| 21 | core/src/policy.rs | packages/core/src/policy.ts | TODO |
| 22 | core/src/property/backend/lww.rs | packages/core/src/property/backend/lww.ts | TODO |
| 23 | core/src/property/backend/mod.rs | packages/core/src/property/backend/index.ts | TODO |
| 24 | core/src/property/backend/pn_counter.rs | packages/core/src/property/backend/pn_counter.ts | TODO |
| 25 | core/src/property/backend/yrs.rs | packages/core/src/property/backend/yjs.ts | TODO |
| 26 | core/src/property/mod.rs | packages/core/src/property/index.ts | TODO |
| 27 | core/src/property/traits.rs | packages/core/src/property/traits.ts | TODO |
| 28 | core/src/property/value/entity_ref.rs | packages/core/src/property/value/entity_ref.ts | TODO |
| 29 | core/src/property/value/json.rs | packages/core/src/property/value/json.ts | TODO |
| 30 | core/src/property/value/lww.rs | packages/core/src/property/value/lww.ts | TODO |
| 31 | core/src/property/value/mod.rs | packages/core/src/property/value/index.ts | TODO |
| 32 | core/src/property/value/pn_counter.rs | packages/core/src/property/value/pn_counter.ts | TODO |
| 33 | core/src/property/value/yrs.rs | packages/core/src/property/value/yrs_string.ts | TODO |
| 34 | core/src/query_value.rs | packages/core/src/query_value.ts | TODO |
| 35 | core/src/reactor.rs | packages/core/src/reactor/index.ts | TODO |
| 36 | core/src/reactor/candidate_changes.rs | packages/core/src/reactor/candidate_changes.ts | TODO |
| 37 | core/src/reactor/comparison_index.rs | packages/core/src/reactor/comparison_index.ts | TODO |
| 38 | core/src/reactor/fetch_gap.rs | packages/core/src/reactor/fetch_gap.ts | TODO |
| 39 | core/src/reactor/property_path.rs | packages/core/src/reactor/property_path.ts | TODO |
| 40 | core/src/reactor/subscription.rs | packages/core/src/reactor/subscription.ts | TODO |
| 41 | core/src/reactor/subscription_state.rs | packages/core/src/reactor/subscription_state.ts | TODO |
| 42 | core/src/reactor/update.rs | packages/core/src/reactor/update.ts | TODO |
| 43 | core/src/reactor/watcherset.rs | packages/core/src/reactor/watcherset.ts | TODO |
| 44 | core/src/resultset.rs | packages/core/src/resultset.ts | TODO |
| 45 | core/src/retrieval.rs | packages/core/src/retrieval.ts | TODO |
| 46 | core/src/schema.rs | packages/core/src/schema.ts | TODO |
| 47 | core/src/selection/filter.rs | packages/core/src/selection/filter.ts | TODO |
| 48 | core/src/selection/mod.rs | packages/core/src/selection/index.ts | TODO |
| 49 | core/src/storage.rs | packages/core/src/storage.ts | TODO |
| 50 | core/src/system.rs | packages/core/src/system.ts | TODO |
| 51 | core/src/task.rs | packages/core/src/task.ts | TODO |
| 52 | core/src/traits.rs | packages/core/src/traits.ts | TODO |
| 53 | core/src/transaction.rs | packages/core/src/transaction.ts | TODO |
| 54 | core/src/type_resolver.rs | packages/core/src/type_resolver.ts | TODO |
| 55 | core/src/util/cast.rs | packages/core/src/util/cast.ts | TODO |
| 56 | core/src/util/expand_states.rs | packages/core/src/util/expand_states.ts | TODO |
| 57 | core/src/util/iterable.rs | packages/core/src/util/iterable.ts | TODO |
| 58 | core/src/util/ivec.rs | packages/core/src/util/ivec.ts | TODO |
| 59 | core/src/util/mod.rs | packages/core/src/util/index.ts | TODO |
| 60 | core/src/util/ready_chunks.rs | packages/core/src/util/ready_chunks.ts | TODO |
| 61 | core/src/util/safemap.rs | packages/core/src/util/safemap.ts | TODO |
| 62 | core/src/util/safeset.rs | packages/core/src/util/safeset.ts | TODO |
| 63 | core/src/value/cast.rs | packages/core/src/value/cast.ts | TODO |
| 64 | core/src/value/cast_predicate.rs | packages/core/src/value/cast_predicate.ts | TODO |
| 65 | core/src/value/collatable.rs | packages/core/src/value/collatable.ts | TODO |
| 66 | core/src/value/mod.rs | packages/core/src/value/index.ts | TODO |
| 67 | core/src/value/wasm.rs | — | SKIP: WASM bindings (E9) |

## Unit Tests

### core/src/collation.rs (9 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_string_collation | TODO |
| 2 | test_integer_collation | TODO |
| 3 | test_float_collation | TODO |
| 4 | test_range_bounds | TODO |
| 5 | test_literal_i16_collation | TODO |
| 6 | test_literal_i32_collation | TODO |
| 7 | test_literal_entity_id_collation | TODO |
| 8 | test_literal_binary_collation | TODO |
| 9 | test_literal_object_collation | TODO |

### core/src/indexing/encoding.rs (2 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_desc_ordering | TODO |
| 2 | test_asc_ordering | TODO |

### core/src/indexing/key_spec.rs (13 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_exact_match | TODO |
| 2 | test_prefix_match | TODO |
| 3 | test_inverse_exact_match | TODO |
| 4 | test_inverse_prefix_match | TODO |
| 5 | test_user_example | TODO |
| 6 | test_no_match_different_fields | TODO |
| 7 | test_no_match_partial_field_overlap | TODO |
| 8 | test_no_match_query_longer_than_index | TODO |
| 9 | test_empty_specs | TODO |
| 10 | test_single_field_cases | TODO |
| 11 | test_complex_multi_field_scenarios | TODO |
| 12 | test_helper_methods | TODO |
| 13 | test_edge_case_behaviors | TODO |

### core/src/lineage.rs (13 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_linear_history | TODO |
| 2 | test_concurrent_history | TODO |
| 3 | test_incomparable | TODO |
| 4 | test_empty_clocks | TODO |
| 5 | test_budget_exceeded | TODO |
| 6 | test_self_comparison | TODO |
| 7 | multiple_roots | TODO |
| 8 | test_compare_event_unstored | TODO |
| 9 | test_compare_event_redundant_delivery | TODO |
| 10 | test_event_accumulator | TODO |
| 11 | test_event_accumulator_with_concurrent_history | TODO |
| 12 | test_event_accumulator_equal_clocks | TODO |
| 13 | test_event_accumulator_only_subject_side | TODO |

### core/src/peer_subscription/client_relay.rs (8 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_new_subscription_setup | TODO |
| 2 | test_peer_disconnection_orphans_subscriptions | TODO |
| 3 | test_peer_connection_triggers_setup | TODO |
| 4 | test_failed_subscription_retry | TODO |
| 5 | test_retryable_vs_non_retryable_failures | TODO |
| 6 | test_subscription_removal | TODO |
| 7 | test_edge_cases | TODO |
| 8 | test_notify_unsubscribe_with_no_established_subscription | TODO |

### core/src/property/value/entity_ref.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_ref_roundtrip | TODO |
| 2 | test_ref_from_entity_id | TODO |
| 3 | test_ref_into_entity_id | TODO |
| 4 | test_ref_missing | TODO |
| 5 | test_ref_invalid_string | TODO |
| 6 | test_ref_invalid_variant | TODO |

### core/src/property/value/json.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_json_roundtrip | TODO |
| 2 | test_json_get_path | TODO |
| 3 | test_json_null | TODO |
| 4 | test_json_missing | TODO |
| 5 | test_json_invalid_variant | TODO |
| 6 | test_json_deref | TODO |

### core/src/reactor.rs (1 test)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_entity_remains_watched_after_predicate_stops_matching | TODO |

### core/src/reactor/candidate_changes.rs (3 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_candidate_changes_empty | TODO |
| 2 | test_candidate_changes_add_query | TODO |
| 3 | test_candidate_changes_entity_level | TODO |

### core/src/reactor/comparison_index.rs (2 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_field_index | TODO |
| 2 | test_field_index_not_equal | TODO |

### core/src/reactor/fetch_gap.rs (3 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_build_gap_predicate_single_column_asc | TODO |
| 2 | test_build_gap_predicate_multi_column | TODO |
| 3 | test_infer_value_type_for_field | TODO |

### core/src/resultset.rs (8 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_entity_id_ordering | TODO |
| 2 | test_order_by_with_tie_breaking | TODO |
| 3 | test_limit_functionality | TODO |
| 4 | test_dirty_tracking | TODO |
| 5 | test_write_guard_atomic_operations | TODO |
| 6 | test_ivec_small_keys | SKIP: Rust-internal IVec Small/Large enum optimization; TS uses plain Uint8Array |
| 7 | test_ivec_large_keys | SKIP: same as above |
| 8 | test_ivec_boundary | SKIP: same as above |

### core/src/selection/filter.rs (21 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_simple_equality | TODO |
| 2 | test_and_condition | TODO |
| 3 | test_complex_condition | TODO |
| 4 | test_in_operator | TODO |
| 5 | test_simple_json_path | TODO |
| 6 | test_nested_json_path | TODO |
| 7 | test_json_path_with_numeric_value | TODO |
| 8 | test_json_path_with_boolean | TODO |
| 9 | test_json_path_not_found | TODO |
| 10 | test_json_path_combined_with_regular_field | TODO |
| 11 | test_traverse_into_non_json_property_errors | TODO |
| 12 | test_json_path_with_or | TODO |
| 13 | test_json_path_with_in_operator | TODO |
| 14 | test_collection_qualified_json_path | TODO |
| 15 | test_json_numeric_casting_same_type | TODO |
| 16 | test_json_numeric_casting_float_to_int | TODO |
| 17 | test_json_string_to_number_no_cast | TODO |
| 18 | test_json_number_to_string_no_cast | TODO |
| 19 | test_json_string_equality_works | TODO |
| 20 | test_json_comparison_operators | TODO |
| 21 | test_regular_field_still_casts_string_to_number | TODO |

### core/src/type_resolver.rs (7 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_resolve_simple_path | TODO |
| 2 | test_resolve_id_path | TODO |
| 3 | test_resolve_json_path | TODO |
| 4 | test_literal_to_json_string | TODO |
| 5 | test_literal_to_json_number | TODO |
| 6 | test_resolve_types_converts_literal_for_json_path | TODO |
| 7 | test_resolve_types_leaves_simple_path_literal_alone | TODO |

### core/src/util/ivec.rs (9 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_small_push | TODO |
| 2 | test_transition_to_large | TODO |
| 3 | test_contains | TODO |
| 4 | test_contains_large | TODO |
| 5 | test_iter | TODO |
| 6 | test_iter_large | TODO |
| 7 | test_drop | TODO |
| 8 | test_add | TODO |
| 9 | test_add_large | TODO |

### core/src/value/cast.rs (9 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_string_to_entity_id | TODO |
| 2 | test_entity_id_to_string | TODO |
| 3 | test_invalid_entity_id_string | TODO |
| 4 | test_numeric_upcasting | TODO |
| 5 | test_numeric_downcasting | TODO |
| 6 | test_string_to_numeric | TODO |
| 7 | test_string_to_bool | TODO |
| 8 | test_incompatible_types | TODO |
| 9 | test_same_type_cast | TODO |

### core/src/value/cast_predicate.rs (3 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_cast_id_field_string_to_entity_id | TODO |
| 2 | test_cast_literal_equals_field | TODO |
| 3 | test_cast_complex_predicate | TODO |

### core/src/value/mod.rs (6 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_extract_at_path_empty | TODO |
| 2 | test_extract_at_path_json_string | TODO |
| 3 | test_extract_at_path_json_number | TODO |
| 4 | test_extract_at_path_json_nested | TODO |
| 5 | test_extract_at_path_missing | TODO |
| 6 | test_extract_at_path_non_json | TODO |

## Summary

- Source files: 67 (3 skip)
- Unit tests: 130 (3 skip: ivec internal tests)
- Integration tests: 0 (core integration tests are in the `tests` crate — see tests.md)
