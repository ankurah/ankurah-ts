# ankurah-proto — Punchlist

**Rust crate**: `ankurah-proto` (`ankurah-ts-support/proto/`)
**TS package**: `@ankurah/proto` (`packages/proto/`)
**Dependencies**: none

## Source Files

| # | Rust file | TS target | Status |
|---|-----------|-----------|--------|
| 1 | proto/src/auth.rs | packages/proto/src/auth.ts | DONE |
| 2 | proto/src/clock.rs | packages/proto/src/clock.ts | DONE |
| 3 | proto/src/collection.rs | packages/proto/src/collection.ts | DONE |
| 4 | proto/src/data.rs | packages/proto/src/data.ts | DONE |
| 5 | proto/src/error.rs | packages/proto/src/error.ts | DONE |
| 6 | proto/src/human_id.rs | packages/proto/src/human_id.ts | DONE |
| 7 | proto/src/id.rs | packages/proto/src/id.ts | DONE |
| 8 | proto/src/lib.rs | packages/proto/src/index.ts | DONE |
| 9 | proto/src/message.rs | packages/proto/src/message.ts | DONE |
| 10 | proto/src/peering.rs | packages/proto/src/peering.ts | DONE |
| 11 | proto/src/postgres.rs | — | SKIP: Postgres-specific serde impls, not needed in TS |
| 12 | proto/src/request.rs | packages/proto/src/request.ts | DONE |
| 13 | proto/src/subscription.rs | packages/proto/src/subscription.ts | DONE |
| 14 | proto/src/sys.rs | packages/proto/src/sys.ts | DONE |
| 15 | proto/src/transaction.rs | packages/proto/src/transaction.ts | DONE |
| 16 | proto/src/update.rs | packages/proto/src/update.ts | DONE |
| 17 | proto/src/wasm.rs | — | SKIP: WASM bindings (E9) |

## Unit Tests (inline)

### proto/src/data.rs (2 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_event_id_json_serialization | DONE |
| 2 | test_event_id_bincode_serialization | DONE |

### proto/src/id.rs (2 tests)

| # | Rust test function | Status |
|---|-------------------|--------|
| 1 | test_entity_id_json_serialization | DONE |
| 2 | test_entity_id_bincode_serialization | DONE |

## Integration Tests (Rust-side fixture generators)

These Rust test files generate `.bin` fixture files that the TS tests read. The TS counterpart is `packages/proto/__tests__/fixtures.test.ts` and `packages/core/__tests__/yrs-yjs-interop.test.ts`.

### proto/tests/bincode_fixtures.rs (15 tests)

| # | Rust test function | TS counterpart | Status |
|---|-------------------|----------------|--------|
| 1 | test_ids_fixture | fixtures.test.ts | DONE |
| 2 | test_clock_fixture | fixtures.test.ts | DONE |
| 3 | test_auth_fixture | fixtures.test.ts | DONE |
| 4 | test_data_fixture | fixtures.test.ts | DONE |
| 5 | test_request_fixture | fixtures.test.ts | DONE |
| 6 | test_response_fixture | fixtures.test.ts | DONE |
| 7 | test_causal_fixture | fixtures.test.ts | DONE |
| 8 | test_delta_fixture | fixtures.test.ts | DONE |
| 9 | test_update_fixture | fixtures.test.ts | DONE |
| 10 | test_message_fixture | fixtures.test.ts | DONE |
| 11 | test_presence_fixture | fixtures.test.ts | DONE |
| 12 | test_system_fixture | fixtures.test.ts | DONE |
| 13 | test_causal_assertion_fixture | fixtures.test.ts | DONE |
| 14 | test_principal_fixture | fixtures.test.ts | DONE |
| 15 | test_attested_event_fixture | fixtures.test.ts | DONE |

### proto/tests/yrs_v2_fixtures.rs (7 tests)

| # | Rust test function | TS counterpart | Status |
|---|-------------------|----------------|--------|
| 1 | test_empty_doc | yrs-yjs-interop.test.ts | DONE |
| 2 | test_simple_text | yrs-yjs-interop.test.ts | DONE |
| 3 | test_multifield | yrs-yjs-interop.test.ts | DONE |
| 4 | test_text_with_edits | yrs-yjs-interop.test.ts | DONE |
| 5 | test_incremental_base | yrs-yjs-interop.test.ts | DONE |
| 6 | test_incremental_diff | yrs-yjs-interop.test.ts | DONE |
| 7 | test_concurrent_merge | yrs-yjs-interop.test.ts | DONE |

## Summary

- Source files: 17 (2 skip)
- Unit tests: 4
- Integration tests: 22 (Rust-side fixture generators; TS reads .bin files)
