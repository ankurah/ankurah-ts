# Port Punchlist v2 — Index

**Source of truth**: `ankurah-ts-support/` (ts-port-support branch)

**Statuses**:
- `TODO` — needs porting or full re-audit
- `SKIP: <reason>` — intentionally excluded (file listed but not ported)
- `DONE` — ported, audited, reviewer-verified (nothing starts here)

Every `.rs` file in every included crate is listed. Every `#[test]`/`#[tokio::test]` function is listed individually.

## Dependency Order (port in this order)

```
1. ankql          (no internal deps)
2. proto          (no internal deps)
3. signals        (no internal deps)
4. storage-common (depends on: ankql, proto)
5. core           (depends on: ankql, proto, signals, storage-common)
6. storage-sqlite (depends on: core, storage-common)
7. storage-postgres (depends on: core, storage-common)
8. storage-indexeddb (depends on: core, storage-common)
9. connector-websocket (depends on: core, proto)
10. connector-websocket-server (depends on: core, proto)
11. connector-local (depends on: core)
12. ankurah        (facade — depends on all above)
13. tests          (integration — depends on all above)
```

## Per-Crate Punchlists

| # | Crate | TS Package | Punchlist | Source files | Unit tests | Integration tests |
|---|-------|------------|-----------|-------------|------------|-------------------|
| 1 | ankql | @ankurah/ankql | [ankql.md](ankql.md) | 8 | 59 | 0 |
| 2 | ankurah-proto | @ankurah/proto | [proto.md](proto.md) | 17 (2 skip) | 4 | 22 (Rust-side fixtures) |
| 3 | ankurah-signals | @ankurah/signals | [signals.md](signals.md) | 19 (4 skip) | 10 | 25 |
| 4 | ankurah-storage-common | @ankurah/storage-common | [storage-common.md](storage-common.md) | 8 | 79 | 0 |
| 5 | ankurah-core | @ankurah/core | [core.md](core.md) | 65 (3 skip) | 130 | 0 |
| 6 | ankurah-storage-sqlite | @ankurah/storage-sqlite | [storage-sqlite.md](storage-sqlite.md) | 7 (1 skip) | 11 | 24 |
| 7 | ankurah-storage-postgres | @ankurah/storage-postgres | [storage-postgres.md](storage-postgres.md) | 3 | 26 | 27 |
| 8 | ankurah-storage-indexeddb-wasm | @ankurah/storage-indexeddb | [storage-indexeddb.md](storage-indexeddb.md) | 16 | 10 | 57 (wasm — need browser/jsdom) |
| 9 | ankurah-websocket-client | @ankurah/connector-websocket | [connector-websocket.md](connector-websocket.md) | 3 | 0 | 0 |
| 10 | ankurah-websocket-server | @ankurah/connector-websocket-server | [connector-websocket-server.md](connector-websocket-server.md) | 6 | 0 | 0 |
| 11 | ankurah-connector-local-process | @ankurah/connector-local | [connector-local.md](connector-local.md) | 1 | 0 | 0 |
| 12 | ankurah (facade) | @ankurah/ankurah | [ankurah.md](ankurah.md) | 1 | 0 | 0 |
| 13 | ankurah-tests | @ankurah/core (integration) | [tests.md](tests.md) | 2 (1 skip) | 0 | 90 |

## Excluded Crates (with reason)

| Crate | Reason |
|-------|--------|
| storage/sled | Rust-specific embedded DB — no Node/browser equivalent |
| derive | Proc macro — replaced by `defineModel()` runtime (E12) |
| connectors/websocket-client-wasm | WASM variant — TS uses pure websocket-client |
| tests-wasm | WASM test bindings — not applicable to pure TS |
| examples/* | Example apps — not part of library |
| docs/example/* | Doc examples — not part of library |
| quarantine | Deprecated code |

## Totals

- **Source files**: 154 (+ 11 skips)
- **Unit tests**: 329
- **Integration tests**: 233
- **Grand total items**: 716
