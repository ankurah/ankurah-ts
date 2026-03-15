# Port Punchlist

Every in-scope Rust source file and its target TS path.

**Rule**: Every TS file gets rewritten from its Rust source. Pure mechanical translation.


## proto → @ankurah/proto

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 1 | proto/src/auth.rs | packages/proto/src/auth.ts | YES |
| 2 | proto/src/clock.rs | packages/proto/src/clock.ts | YES |
| 3 | proto/src/collection.rs | packages/proto/src/collection.ts | YES |
| 4 | proto/src/data.rs | packages/proto/src/data.ts | YES |
| 5 | proto/src/error.rs | packages/proto/src/error.ts | YES |
| 6 | proto/src/human_id.rs | packages/proto/src/human_id.ts | YES |
| 7 | proto/src/id.rs | packages/proto/src/id.ts | YES |
| 8 | proto/src/lib.rs | packages/proto/src/index.ts | YES |
| 9 | proto/src/message.rs | packages/proto/src/message.ts | YES |
| 10 | proto/src/peering.rs | packages/proto/src/peering.ts | YES |
| - | proto/src/postgres.rs | SKIP | E10 |
| 11 | proto/src/request.rs | packages/proto/src/request.ts | YES |
| 12 | proto/src/subscription.rs | packages/proto/src/subscription.ts | YES |
| 13 | proto/src/sys.rs | packages/proto/src/sys.ts | YES |
| 14 | proto/src/transaction.rs | packages/proto/src/transaction.ts | YES |
| 15 | proto/src/update.rs | packages/proto/src/update.ts | YES |
| - | proto/src/wasm.rs | SKIP | E9 |

## core → @ankurah/core

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 16 | core/src/changes.rs | packages/core/src/changes.ts | YES |
| 17 | core/src/collation.rs | packages/core/src/collation.ts | NO |
| 18 | core/src/collectionset.rs | packages/core/src/collectionset.ts | YES |
| 19 | core/src/connector.rs | packages/core/src/connector.ts | YES |
| 20 | core/src/context.rs | packages/core/src/context.ts | YES |
| 21 | core/src/entity.rs | packages/core/src/entity.ts | YES |
| 22 | core/src/error.rs | packages/core/src/error.ts | YES |
| 23 | core/src/indexing/encoding.rs | packages/core/src/indexing/encoding.ts | YES |
| 24 | core/src/indexing/key_spec.rs | packages/core/src/indexing/key_spec.ts | YES |
| 25 | core/src/indexing/mod.rs | packages/core/src/indexing/index.ts | YES |
| 26 | core/src/lib.rs | packages/core/src/index.ts | YES |
| 27 | core/src/lineage.rs | packages/core/src/lineage.ts | YES |
| 28 | core/src/livequery.rs | packages/core/src/livequery.ts | YES |
| 29 | core/src/model.rs | packages/core/src/model/index.ts | NO |
| - | core/src/model/tsify.rs | SKIP | tsify |
| 30 | core/src/node.rs | packages/core/src/node.ts | YES |
| 31 | core/src/node_applier.rs | packages/core/src/node_applier.ts | YES |
| 32 | core/src/peer_subscription/client_relay.rs | packages/core/src/peer_subscription/client_relay.ts | NO |
| 33 | core/src/peer_subscription/mod.rs | packages/core/src/peer_subscription/index.ts | NO |
| 34 | core/src/peer_subscription/server.rs | packages/core/src/peer_subscription/server.ts | NO |
| 35 | core/src/policy.rs | packages/core/src/policy.ts | YES |
| 36 | core/src/property/backend/lww.rs | packages/core/src/property/backend/lww.ts | YES |
| 37 | core/src/property/backend/mod.rs | packages/core/src/property/backend/index.ts | YES |
| 38 | core/src/property/backend/pn_counter.rs | packages/core/src/property/backend/pn_counter.ts | NO |
| 39 | core/src/property/backend/yrs.rs | packages/core/src/property/backend/yjs.ts | YES |
| 40 | core/src/property/mod.rs | packages/core/src/property/index.ts | YES |
| 41 | core/src/property/traits.rs | packages/core/src/property/traits.ts | YES |
| 42 | core/src/property/value/entity_ref.rs | packages/core/src/property/value/entity_ref.ts | NO |
| 43 | core/src/property/value/json.rs | packages/core/src/property/value/json.ts | NO |
| 44 | core/src/property/value/lww.rs | packages/core/src/property/value/lww.ts | YES |
| 45 | core/src/property/value/mod.rs | packages/core/src/property/value/index.ts | NO |
| 46 | core/src/property/value/pn_counter.rs | packages/core/src/property/value/pn_counter.ts | NO |
| 47 | core/src/property/value/yrs.rs | packages/core/src/property/value/yjs.ts | NO |
| 48 | core/src/query_value.rs | packages/core/src/query_value.ts | YES |
| 49 | core/src/reactor.rs | packages/core/src/reactor/index.ts | YES |
| 50 | core/src/reactor/candidate_changes.rs | packages/core/src/reactor/candidate_changes.ts | NO |
| 51 | core/src/reactor/comparison_index.rs | packages/core/src/reactor/comparison_index.ts | NO |
| 52 | core/src/reactor/fetch_gap.rs | packages/core/src/reactor/fetch_gap.ts | YES |
| 53 | core/src/reactor/property_path.rs | packages/core/src/reactor/property_path.ts | NO |
| 54 | core/src/reactor/subscription.rs | packages/core/src/reactor/subscription.ts | YES |
| 55 | core/src/reactor/subscription_state.rs | packages/core/src/reactor/subscription_state.ts | YES |
| 56 | core/src/reactor/update.rs | packages/core/src/reactor/update.ts | YES |
| 57 | core/src/reactor/watcherset.rs | packages/core/src/reactor/watcherset.ts | NO |
| 58 | core/src/resultset.rs | packages/core/src/resultset.ts | YES |
| 59 | core/src/retrieval.rs | packages/core/src/retrieval.ts | YES |
| 60 | core/src/schema.rs | packages/core/src/schema.ts | YES |
| 61 | core/src/selection/filter.rs | packages/core/src/selection/filter.ts | YES |
| 62 | core/src/selection/mod.rs | packages/core/src/selection/index.ts | NO |
| 63 | core/src/storage.rs | packages/core/src/storage.ts | YES |
| 64 | core/src/system.rs | packages/core/src/system.ts | YES |
| 65 | core/src/task.rs | packages/core/src/task.ts | NO |
| 66 | core/src/traits.rs | packages/core/src/traits.ts | NO |
| 67 | core/src/transaction.rs | packages/core/src/transaction.ts | YES |
| 68 | core/src/type_resolver.rs | packages/core/src/type_resolver.ts | NO |
| 69 | core/src/util/cast.rs | packages/core/src/util/cast.ts | NO |
| 70 | core/src/util/expand_states.rs | packages/core/src/util/expand_states.ts | NO |
| 71 | core/src/util/iterable.rs | packages/core/src/util/iterable.ts | NO |
| 72 | core/src/util/ivec.rs | packages/core/src/util/ivec.ts | NO |
| 73 | core/src/util/mod.rs | packages/core/src/util/index.ts | NO |
| 74 | core/src/util/ready_chunks.rs | packages/core/src/util/ready_chunks.ts | YES |
| 75 | core/src/util/safemap.rs | packages/core/src/util/safemap.ts | NO |
| 76 | core/src/util/safeset.rs | packages/core/src/util/safeset.ts | NO |
| 77 | core/src/value/cast.rs | packages/core/src/value/cast.ts | YES |
| 78 | core/src/value/cast_predicate.rs | packages/core/src/value/cast_predicate.ts | YES |
| 79 | core/src/value/collatable.rs | packages/core/src/value/collatable.ts | YES |
| 80 | core/src/value/mod.rs | packages/core/src/value/index.ts | YES |
| - | core/src/value/wasm.rs | SKIP | E9 |

## signals → @ankurah/signals

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 81 | signals/src/broadcast.rs | packages/signals/src/broadcast.ts | YES |
| 82 | signals/src/context.rs | packages/signals/src/context.ts | YES |
| - | signals/src/jsvalue.rs | SKIP | jsvalue |
| 83 | signals/src/lib.rs | packages/signals/src/index.ts | YES |
| 84 | signals/src/observer.rs | packages/signals/src/observer/index.ts | YES |
| 85 | signals/src/observer/callback_observer.rs | packages/signals/src/observer/callback_observer.ts | YES |
| 86 | signals/src/porcelain.rs | packages/signals/src/porcelain/index.ts | YES |
| 87 | signals/src/porcelain/subscribe.rs | packages/signals/src/porcelain/subscribe.ts | YES |
| 88 | signals/src/porcelain/wait.rs | packages/signals/src/porcelain/wait.ts | NO |
| - | signals/src/react.rs | SKIP | E14/E15 |
| - | signals/src/react_native.rs | SKIP | E14/E15 |
| - | signals/src/reactive_graph.rs | SKIP | E14/E15 |
| 89 | signals/src/signal.rs | packages/signals/src/signal/index.ts | YES |
| 90 | signals/src/signal/calculated.rs | packages/signals/src/signal/calculated.ts | YES |
| 91 | signals/src/signal/map.rs | packages/signals/src/signal/map.ts | NO |
| 92 | signals/src/signal/memo.rs | packages/signals/src/signal/memo.ts | NO |
| 93 | signals/src/signal/mutable.rs | packages/signals/src/signal/mutable.ts | YES |
| 94 | signals/src/signal/read.rs | packages/signals/src/signal/read.ts | YES |
| 95 | signals/src/value.rs | packages/signals/src/value.ts | YES |

## ankql → @ankurah/ankql

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 96 | ankql/src/ast.rs | packages/ankql/src/ast.ts | YES |
| 97 | ankql/src/conversion.rs | packages/ankql/src/conversion.ts | YES |
| 98 | ankql/src/error.rs | packages/ankql/src/error.ts | YES |
| 99 | ankql/src/grammar.rs | packages/ankql/src/grammar.ts | YES |
| 100 | ankql/src/lib.rs | packages/ankql/src/index.ts | YES |
| 101 | ankql/src/parser.rs | packages/ankql/src/parser.ts | YES |
| 102 | ankql/src/selection.rs | packages/ankql/src/selection/index.ts | YES |
| 103 | ankql/src/selection/sql.rs | packages/ankql/src/selection/sql.ts | YES |

## storage/common → @ankurah/storage-common

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 104 | storage/common/src/bounds.rs | packages/storage-common/src/bounds.ts | YES |
| 105 | storage/common/src/filtering.rs | packages/storage-common/src/filtering.ts | YES |
| 106 | storage/common/src/lib.rs | packages/storage-common/src/index.ts | YES |
| 107 | storage/common/src/planner.rs | packages/storage-common/src/planner.ts | YES |
| 108 | storage/common/src/predicate.rs | packages/storage-common/src/predicate.ts | YES |
| 109 | storage/common/src/sorting.rs | packages/storage-common/src/sorting.ts | YES |
| 110 | storage/common/src/traits.rs | packages/storage-common/src/traits.ts | NO |
| 111 | storage/common/src/types.rs | packages/storage-common/src/types.ts | YES |

## storage/sqlite → @ankurah/storage-sqlite (split to expo-sqlite + better-sqlite3)

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 112 | storage/sqlite/src/connection.rs | packages/storage-sqlite/src/connection.ts | NO |
| 113 | storage/sqlite/src/engine.rs | packages/storage-sqlite/src/engine.ts | NO |
| 114 | storage/sqlite/src/error.rs | packages/storage-sqlite/src/error.ts | NO |
| 115 | storage/sqlite/src/lib.rs | packages/storage-sqlite/src/index.ts | NO |
| 116 | storage/sqlite/src/sql_builder.rs | packages/storage-sqlite/src/sql_builder.ts | NO |
| 117 | storage/sqlite/src/value.rs | packages/storage-sqlite/src/value.ts | NO |

## storage/postgres → @ankurah/storage-postgres (node target)

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 118 | storage/postgres/src/lib.rs | packages/storage-postgres/src/index.ts | NO |
| 119 | storage/postgres/src/sql_builder.rs | packages/storage-postgres/src/sql_builder.ts | NO |
| 120 | storage/postgres/src/value.rs | packages/storage-postgres/src/value.ts | NO |

## storage/indexeddb-wasm → @ankurah/storage-indexeddb (browser target (pure TS, no wasm))

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 121 | storage/indexeddb-wasm/src/collection.rs | packages/storage-indexeddb/src/collection.ts | NO |
| 122 | storage/indexeddb-wasm/src/database.rs | packages/storage-indexeddb/src/database.ts | NO |
| 123 | storage/indexeddb-wasm/src/engine.rs | packages/storage-indexeddb/src/engine.ts | NO |
| 124 | storage/indexeddb-wasm/src/error.rs | packages/storage-indexeddb/src/error.ts | NO |
| 125 | storage/indexeddb-wasm/src/idb_value.rs | packages/storage-indexeddb/src/idb_value.ts | NO |
| 126 | storage/indexeddb-wasm/src/lib.rs | packages/storage-indexeddb/src/index.ts | NO |
| 127 | storage/indexeddb-wasm/src/planner_integration.rs | packages/storage-indexeddb/src/planner_integration.ts | NO |
| 128 | storage/indexeddb-wasm/src/scanner.rs | packages/storage-indexeddb/src/scanner.ts | NO |
| 129 | storage/indexeddb-wasm/src/statics.rs | packages/storage-indexeddb/src/statics.ts | NO |
| 130 | storage/indexeddb-wasm/src/util/cb_future.rs | packages/storage-indexeddb/src/util/cb_future.ts | NO |
| 131 | storage/indexeddb-wasm/src/util/cb_race.rs | packages/storage-indexeddb/src/util/cb_race.ts | NO |
| 132 | storage/indexeddb-wasm/src/util/cb_stream.rs | packages/storage-indexeddb/src/util/cb_stream.ts | NO |
| 133 | storage/indexeddb-wasm/src/util/mod.rs | packages/storage-indexeddb/src/util/index.ts | NO |
| 134 | storage/indexeddb-wasm/src/util/navigator_lock.rs | packages/storage-indexeddb/src/util/navigator_lock.ts | NO |
| 135 | storage/indexeddb-wasm/src/util/object.rs | packages/storage-indexeddb/src/util/object.ts | NO |
| 136 | storage/indexeddb-wasm/src/util/require.rs | packages/storage-indexeddb/src/util/require.ts | NO |

## connectors/websocket-client → @ankurah/connector-websocket

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 137 | connectors/websocket-client/src/client.rs | packages/connector-websocket/src/client.ts | NO |
| 138 | connectors/websocket-client/src/lib.rs | packages/connector-websocket/src/index.ts | YES |
| 139 | connectors/websocket-client/src/sender.rs | packages/connector-websocket/src/sender.ts | NO |

## connectors/websocket-server → @ankurah/connector-websocket-server (node target)

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 140 | connectors/websocket-server/src/client_ip.rs | packages/connector-websocket-server/src/client_ip.ts | NO |
| 141 | connectors/websocket-server/src/lib.rs | packages/connector-websocket-server/src/index.ts | NO |
| 142 | connectors/websocket-server/src/sender.rs | packages/connector-websocket-server/src/sender.ts | NO |
| 143 | connectors/websocket-server/src/server.rs | packages/connector-websocket-server/src/server.ts | NO |
| 144 | connectors/websocket-server/src/state.rs | packages/connector-websocket-server/src/state.ts | NO |
| 145 | connectors/websocket-server/src/user_agent.rs | packages/connector-websocket-server/src/user_agent.ts | NO |

## connectors/local-process → @ankurah/connector-local

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 146 | connectors/local-process/src/lib.rs | packages/connector-local/src/index.ts | YES |

## ankurah → @ankurah/ankurah (facade crate)

| # | Rust file | TS target | Exists? |
|---|-----------|-----------|--------|
| 147 | ankurah/src/lib.rs | packages/ankurah/src/index.ts | NO |

## Summary

- **In-scope files**: 147
- **TS exists (rewrite)**: 83
- **TS missing (create)**: 64
- **Skipped**: 8

