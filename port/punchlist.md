# Port Punchlist

Every in-scope Rust source file and its target TS path. Generated from `find`, not memory.

**Rule**: Every existing TS file gets rewritten from its Rust source using the translation rules. No preserving old work. Pure mechanical Rust→TS translation.

**Naming**: TS files use snake_case matching Rust (per translation-rules.md A1). Existing files with hyphens (e.g. `candidate-changes.ts`) must be renamed to underscores (`candidate_changes.ts`).

## Files to rename (existing TS files with wrong names)

| Current TS name | Correct TS name |
|----------------|-----------------|
| `core/src/reactor/candidate-changes.ts` | `core/src/reactor/candidate_changes.ts` |
| `core/src/reactor/comparison-index.ts` | `core/src/reactor/comparison_index.ts` |
| `core/src/reactor/property-path.ts` | `core/src/reactor/property_path.ts` |
| `core/src/reactor/watcher_set.ts` | `core/src/reactor/watcherset.ts` |

## proto (16 files, 2 skipped)

| # | Rust file | TS target | Exists? | Notes |
|---|-----------|-----------|---------|-------|
| 1 | proto/src/lib.rs | packages/proto/src/index.ts | YES | |
| 2 | proto/src/auth.rs | packages/proto/src/auth.ts | YES | |
| 3 | proto/src/clock.rs | packages/proto/src/clock.ts | YES | |
| 4 | proto/src/collection.rs | packages/proto/src/collection.ts | YES | |
| 5 | proto/src/data.rs | packages/proto/src/data.ts | YES | |
| 6 | proto/src/error.rs | packages/proto/src/error.ts | YES | |
| 7 | proto/src/human_id.rs | packages/proto/src/human_id.ts | YES | |
| 8 | proto/src/id.rs | packages/proto/src/id.ts | YES | |
| 9 | proto/src/message.rs | packages/proto/src/message.ts | YES | |
| 10 | proto/src/peering.rs | packages/proto/src/peering.ts | YES | |
| 11 | proto/src/request.rs | packages/proto/src/request.ts | YES | |
| 12 | proto/src/subscription.rs | packages/proto/src/subscription.ts | YES | |
| 13 | proto/src/sys.rs | packages/proto/src/sys.ts | YES | |
| 14 | proto/src/transaction.rs | packages/proto/src/transaction.ts | YES | |
| 15 | proto/src/update.rs | packages/proto/src/update.ts | YES | |
| - | proto/src/postgres.rs | SKIP | | E10: feature-gated |
| - | proto/src/wasm.rs | SKIP | | E9: WASM-only |

## core (67 files, 4 skipped)

| # | Rust file | TS target | Exists? | Notes |
|---|-----------|-----------|---------|-------|
| 16 | core/src/lib.rs | packages/core/src/index.ts | YES | |
| 17 | core/src/changes.rs | packages/core/src/changes.ts | YES | |
| 18 | core/src/collation.rs | packages/core/src/collation.ts | NO | |
| 19 | core/src/collectionset.rs | packages/core/src/collectionset.ts | YES | |
| 20 | core/src/connector.rs | packages/core/src/connector.ts | YES | |
| 21 | core/src/context.rs | packages/core/src/context.ts | YES | |
| 22 | core/src/entity.rs | packages/core/src/entity.ts | YES | |
| 23 | core/src/error.rs | packages/core/src/error.ts | YES | |
| 24 | core/src/lineage.rs | packages/core/src/lineage.ts | YES | |
| 25 | core/src/livequery.rs | packages/core/src/livequery.ts | YES | |
| 26 | core/src/model.rs | packages/core/src/model.ts | YES | E12: file-with-submodules |
| 27 | core/src/node.rs | packages/core/src/node.ts | YES | |
| 28 | core/src/node_applier.rs | packages/core/src/node_applier.ts | YES | |
| 29 | core/src/peer_subscription/mod.rs | packages/core/src/peer_subscription/index.ts | NO | |
| 30 | core/src/peer_subscription/client_relay.rs | packages/core/src/peer_subscription/client_relay.ts | NO | |
| 31 | core/src/peer_subscription/server.rs | packages/core/src/peer_subscription/server.ts | NO | |
| 32 | core/src/policy.rs | packages/core/src/policy.ts | YES | |
| 33 | core/src/property/mod.rs | packages/core/src/property/index.ts | YES | |
| 34 | core/src/property/traits.rs | packages/core/src/property/traits.ts | YES | |
| 35 | core/src/property/backend/mod.rs | packages/core/src/property/backend/index.ts | YES | |
| 36 | core/src/property/backend/lww.rs | packages/core/src/property/backend/lww.ts | YES | |
| 37 | core/src/property/backend/yrs.rs | packages/core/src/property/backend/yjs.ts | YES | E5: yrs→yjs |
| 38 | core/src/property/value/mod.rs | packages/core/src/property/value/index.ts | NO | |
| 39 | core/src/property/value/lww.rs | packages/core/src/property/value/lww.ts | YES | |
| 40 | core/src/property/value/yrs.rs | packages/core/src/property/value/yjs.ts | NO | E5: yrs→yjs |
| 41 | core/src/property/value/entity_ref.rs | packages/core/src/property/value/entity_ref.ts | NO | |
| 42 | core/src/property/value/json.rs | packages/core/src/property/value/json.ts | NO | |
| 43 | core/src/query_value.rs | packages/core/src/query_value.ts | YES | |
| 44 | core/src/reactor.rs | packages/core/src/reactor/index.ts | YES | E12: file-with-submodules |
| 45 | core/src/reactor/candidate_changes.rs | packages/core/src/reactor/candidate_changes.ts | NO | rename from candidate-changes.ts |
| 46 | core/src/reactor/comparison_index.rs | packages/core/src/reactor/comparison_index.ts | NO | rename from comparison-index.ts |
| 47 | core/src/reactor/fetch_gap.rs | packages/core/src/reactor/fetch_gap.ts | YES | |
| 48 | core/src/reactor/property_path.rs | packages/core/src/reactor/property_path.ts | NO | rename from property-path.ts |
| 49 | core/src/reactor/subscription.rs | packages/core/src/reactor/subscription.ts | YES | |
| 50 | core/src/reactor/subscription_state.rs | packages/core/src/reactor/subscription_state.ts | YES | |
| 51 | core/src/reactor/update.rs | packages/core/src/reactor/update.ts | YES | |
| 52 | core/src/reactor/watcherset.rs | packages/core/src/reactor/watcherset.ts | NO | rename from watcher_set.ts |
| 53 | core/src/resultset.rs | packages/core/src/resultset.ts | YES | |
| 54 | core/src/retrieval.rs | packages/core/src/retrieval.ts | YES | |
| 55 | core/src/schema.rs | packages/core/src/schema.ts | YES | |
| 56 | core/src/selection/filter.rs | packages/core/src/selection/filter.ts | YES | |
| 57 | core/src/selection/mod.rs | packages/core/src/selection/index.ts | NO | |
| 58 | core/src/storage.rs | packages/core/src/storage.ts | YES | |
| 59 | core/src/system.rs | packages/core/src/system.ts | YES | |
| 60 | core/src/task.rs | packages/core/src/task.ts | NO | |
| 61 | core/src/traits.rs | packages/core/src/traits.ts | NO | |
| 62 | core/src/transaction.rs | packages/core/src/transaction.ts | YES | |
| 63 | core/src/type_resolver.rs | packages/core/src/type_resolver.ts | NO | |
| 64 | core/src/util/mod.rs | packages/core/src/util/index.ts | NO | |
| 65 | core/src/util/cast.rs | packages/core/src/util/cast.ts | NO | |
| 66 | core/src/util/expand_states.rs | packages/core/src/util/expand_states.ts | NO | |
| 67 | core/src/util/iterable.rs | packages/core/src/util/iterable.ts | NO | |
| 68 | core/src/util/ivec.rs | packages/core/src/util/ivec.ts | NO | maps to plain Array |
| 69 | core/src/util/ready_chunks.rs | packages/core/src/util/ready_chunks.ts | YES | |
| 70 | core/src/util/safemap.rs | packages/core/src/util/safemap.ts | NO | maps to plain Map |
| 71 | core/src/util/safeset.rs | packages/core/src/util/safeset.ts | NO | maps to plain Set |
| 72 | core/src/value/mod.rs | packages/core/src/value/index.ts | YES | |
| 73 | core/src/value/cast.rs | packages/core/src/value/cast.ts | YES | |
| 74 | core/src/value/cast_predicate.rs | packages/core/src/value/cast_predicate.ts | YES | |
| 75 | core/src/value/collatable.rs | packages/core/src/value/collatable.ts | YES | |
| - | core/src/model/tsify.rs | SKIP | | E9: WASM-only |
| - | core/src/property/backend/pn_counter.rs | SKIP | | deferred |
| - | core/src/property/value/pn_counter.rs | SKIP | | deferred |
| - | core/src/value/wasm.rs | SKIP | | E9: WASM-only |

## signals (19 files, 4 skipped)

| # | Rust file | TS target | Exists? | Notes |
|---|-----------|-----------|---------|-------|
| 76 | signals/src/lib.rs | packages/signals/src/index.ts | YES | |
| 77 | signals/src/broadcast.rs | packages/signals/src/broadcast.ts | YES | |
| 78 | signals/src/context.rs | packages/signals/src/context.ts | YES | |
| 79 | signals/src/observer.rs | packages/signals/src/observer/index.ts | YES | E12 |
| 80 | signals/src/observer/callback_observer.rs | packages/signals/src/observer/callback_observer.ts | YES | |
| 81 | signals/src/porcelain.rs | packages/signals/src/porcelain/index.ts | YES | E12 |
| 82 | signals/src/porcelain/subscribe.rs | packages/signals/src/porcelain/subscribe.ts | YES | |
| 83 | signals/src/porcelain/wait.rs | packages/signals/src/porcelain/wait.ts | NO | |
| 84 | signals/src/signal.rs | packages/signals/src/signal/index.ts | YES | E12 |
| 85 | signals/src/signal/calculated.rs | packages/signals/src/signal/calculated.ts | YES | |
| 86 | signals/src/signal/map.rs | packages/signals/src/signal/map.ts | NO | |
| 87 | signals/src/signal/memo.rs | packages/signals/src/signal/memo.ts | NO | |
| 88 | signals/src/signal/mutable.rs | packages/signals/src/signal/mutable.ts | YES | |
| 89 | signals/src/signal/read.rs | packages/signals/src/signal/read.ts | YES | |
| 90 | signals/src/value.rs | packages/signals/src/value.ts | YES | |
| - | signals/src/jsvalue.rs | SKIP | | E9: WASM-only |
| - | signals/src/react.rs | SKIP | | E15: replaced by @ankurah/react |
| - | signals/src/react_native.rs | SKIP | | E15: replaced by @ankurah/react |
| - | signals/src/reactive_graph.rs | SKIP | | E14: Rust-only integration |

## ankql (8 files, 0 skipped)

| # | Rust file | TS target | Exists? | Notes |
|---|-----------|-----------|---------|-------|
| 91 | ankql/src/lib.rs | packages/ankql/src/index.ts | YES | |
| 92 | ankql/src/ast.rs | packages/ankql/src/ast.ts | YES | |
| 93 | ankql/src/conversion.rs | packages/ankql/src/conversion.ts | YES | |
| 94 | ankql/src/error.rs | packages/ankql/src/error.ts | YES | |
| 95 | ankql/src/grammar.rs | packages/ankql/src/grammar.ts | YES | |
| 96 | ankql/src/parser.rs | packages/ankql/src/parser.ts | YES | |
| 97 | ankql/src/selection.rs | packages/ankql/src/selection/index.ts | YES | E12 |
| 98 | ankql/src/selection/sql.rs | packages/ankql/src/selection/sql.ts | YES | |

## storage-common (8 files, 0 skipped)

| # | Rust file | TS target | Exists? | Notes |
|---|-----------|-----------|---------|-------|
| 99 | storage/common/src/lib.rs | packages/storage-common/src/index.ts | YES | |
| 100 | storage/common/src/bounds.rs | packages/storage-common/src/bounds.ts | YES | |
| 101 | storage/common/src/filtering.rs | packages/storage-common/src/filtering.ts | YES | |
| 102 | storage/common/src/planner.rs | packages/storage-common/src/planner.ts | YES | |
| 103 | storage/common/src/predicate.rs | packages/storage-common/src/predicate.ts | YES | |
| 104 | storage/common/src/sorting.rs | packages/storage-common/src/sorting.ts | YES | |
| 105 | storage/common/src/traits.rs | packages/storage-common/src/traits.ts | NO | |
| 106 | storage/common/src/types.rs | packages/storage-common/src/types.ts | YES | |

## TS-only files (no Rust counterpart)

| TS file | Purpose |
|---------|---------|
| packages/proto/src/codec.ts | Bincode reader/writer |
| packages/core/src/define-model.ts | defineModel() API (replaces derive macro, E1) |
| packages/core/src/property/value/yrs_string.ts | YrsString active type (may merge with yjs.ts) |

## Summary

- **Total in-scope Rust files**: 106
- **TS exists (needs rewrite)**: 75
- **TS missing (needs creation)**: 31
- **Files to rename**: 4 (reactor hyphen→underscore)
- **Skipped**: 11 (wasm, postgres, pn_counter, react, reactive_graph)
