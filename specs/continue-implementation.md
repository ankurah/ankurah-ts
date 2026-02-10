# ankurah-ts Implementation Continuation

**Purpose**: This document provides sufficient context for a fresh agent to continue the ankurah-ts implementation. Read this first, then `architectural-decisions.md`, then `port-rules.md`, then refer to other specs as needed.

**Last updated**: 2026-02-10

**Status**: Proto (with cross-language fixture parity), signals, and ankql fully implemented and tested. Next: Yrs↔Yjs V2 interop, then core.

## Supervision Model

A human supervisor remains in the loop throughout implementation. Agents are dispatched to perform implementation work in parallel, but the supervisor:
- Reviews agent output before committing
- Makes all architectural and design decisions
- Resolves conflicts between agent work products
- Manages the overall implementation sequence

Agents should surface uncertainties, ambiguities, and potential conflicts to the supervisor rather than making assumptions.

## Project Goal

Create a **fully faithful TypeScript port** of [ankurah](https://github.com/ankurah/ankurah) (a Rust CRDT-backed reactive database), targeting **React Native / Expo Go** as the primary runtime. Expo Go cannot use WASM or native modules, so this must be pure TypeScript.

## Repository Layout

```
/Users/daniel/ak/
├── ankurah/                 # Rust implementation (source of truth, main branch)
├── ankurah-ts-support/      # Rust worktree on ts-port-support branch (fixtures, integration test infra)
├── ankurah-ts/              # TypeScript port (this project)
│   ├── packages/            # bun workspace packages
│   ├── scripts/             # Audit and tooling scripts
│   └── specs/               # Design documents (this directory)
│       └── _agent-work/     # Detailed agent research outputs
├── ankurah-react-hooks/     # Existing React hooks package (absorbed into @ankurah/react)
/Users/daniel/code/
└── domcorder/               # Reference project for bincode TS patterns
    └── proto-ts/            # Bincode codec reference implementation
```

**Sibling Rust checkout required**: `ANKURAH_RS_PATH` env var (default `../ankurah`). Defined in `.env` (gitignored), documented in `.env.example`. Tests hard-fail if missing.

## Key Architectural Decisions (All Resolved)

See `architectural-decisions.md` for the full authoritative list. Summary:

1. **Fully faithful port** — Mirror Rust 1:1. Zero freestyling. See `port-rules.md` for exhaustive mapping rules and exception citations.
2. **Bincode wire format only** — No JSON alternative. Reference: `/Users/daniel/code/domcorder/proto-ts/`.
3. **ankurah/proto structs are authoritative** — Rust source is the single source of truth.
4. **Yrs V2 encoding** — `Y.applyUpdateV2()` / `Y.encodeStateAsUpdateV2()`. NOT V1.
5. **Phase 1 scope — nearly everything** (see architectural-decisions.md for full list).
6. **`defineModel()` API from day one** — TS equivalent of `#[derive(Model)]`. No decorator requirement. Optional legacy decorator sugar for non-RN environments.
7. **No TC39 decorators** — Broken on Hermes/Expo Go (`_initClass` runtime error, unresolved). `Symbol.metadata` unavailable.
8. **bun workspaces + bun test + tsc** — No turborepo, no separate test framework.
9. **Fully async StorageEngine** — All methods return Promises. Matches Rust `async_trait`.
10. **Two separate SQLite packages** — `@ankurah/storage-expo-sqlite` + `@ankurah/storage-better-sqlite3`.
11. **Hand-written recursive descent AnkQL parser** — Parse + local predicate evaluation.
12. **Signals-first React integration** — `useObserve()` + `signalObserver()` HOC via `useSyncExternalStore`. Absorbed from existing `ankurah-react-hooks` into `@ankurah/react`.
13. **Every test must be ported** — All unit/integration tests in every in-scope crate. No exceptions except de-scoped crates.
14. **Programmatic audit script** — `scripts/audit-port.ts` validates bidirectional mapping. Runs in CI.
15. **Co-development branch** — `ts-port-support` branch/worktree in Rust repo for fixtures and integration test infra.

## Package Structure

**Mirroring Rust crates:**

| TS Package | Rust Crate | Notes |
|-----------|------------|-------|
| `@ankurah/proto` | `ankurah-proto` | Types + bincode codec |
| `@ankurah/core` | `ankurah-core` | Entity, Transaction, Node, Reactor, etc. |
| `@ankurah/signals` | `ankurah-signals` | Reactive signal library |
| `@ankurah/ankql` | `ankql` | Hand-written recursive descent parser |
| `@ankurah/storage-common` | `ankurah-storage-common` | Traits + types |
| `@ankurah/connector-websocket` | `ankurah-websocket-client` | WebSocket client |
| `@ankurah/connector-local` | `ankurah-connector-local-process` | Local connector |

**TS-only packages:**

| TS Package | Purpose |
|-----------|---------|
| `@ankurah/react` | React hooks (from ankurah-react-hooks) |
| `@ankurah/storage-expo-sqlite` | Expo Go SQLite storage |
| `@ankurah/storage-better-sqlite3` | Node.js SQLite storage |
| `@ankurah/storage-memory` | In-memory storage (testing) |

## Rust Architecture (Verified Against Source)

These facts are verified against the actual Rust source code (2026-02-10). Detailed findings: `_agent-work/rust-architecture-findings.md`.

### Core Concepts

- **Entity**: `Arc<EntityInner>` with `RwLock<EntityInnerState>` holding `head: Clock` + `backends: BTreeMap<String, Arc<dyn PropertyBackend>>`. Has `EntityKind::Primary` or `EntityKind::Transacted { trx_alive, upstream }`.
- **Model/View/Mutable**: Derive-macro-generated typed wrappers. Model defines schema, View is read-only, Mutable provides CRDT access within a transaction.
- **PropertyBackend**: Trait. LWW stores `BTreeMap<PropertyName, ValueEntry>`, serializes via bincode. Yrs wraps `yrs::Doc`, serializes via native V2 encoding.
- **Transaction**: Owns `AppendOnlyVec<Entity>` of forked entities. `commit()` → `generate_commit_event()` per entity → validate → store → relay → notify reactor. `Drop` sets `alive = false`.
- **Node**: Generic over `StorageEngine` and `PolicyAgent`. Owns `WeakEntitySet`, `Reactor`, peer connections. Two modes: ephemeral (client) and durable (server).
- **Context**: Type-erased (`Arc<dyn TContext>`) interface to Node. Provides `begin()`, `get()`, `fetch()`, `query()`.
- **Reactor**: Complex subsystem (8+ files in `reactor/` directory). Manages WatcherSet with index, wildcard, and entity watchers. Three-phase notification: enumerate watchers → evaluate changes → notify.
- **Signals**: Independent reactive library (16+ files). Core traits: `Signal`, `Get<T>`, `Peek<T>`, `Subscribe<T>`. Types: `Broadcast`, `Mut<T>`, `Read<T>`, `Memo<T>`, `Map<S,T>`, `Calculated<T>`. `BroadcastId` derived from `Arc` pointer address.
- **AnkQL**: PEG grammar (`ankql.pest`). SQL-like predicates with AND/OR/NOT, comparisons, IN, BETWEEN, IS NULL, dot-paths, ORDER BY, LIMIT. AST is `Serialize + Deserialize` for wire transmission.

### Bincode Configuration (CRITICAL)

**bincode 1.3.x, legacy default format.** No varint, no custom options. Every call is bare `bincode::serialize()` / `bincode::deserialize()`.

| Type | Encoding |
|------|----------|
| bool | 1 byte (0x00 / 0x01) |
| u8/i8 | 1 byte |
| u16/i16 | 2 bytes LE |
| u32/i32 | 4 bytes LE |
| u64/i64 | 8 bytes LE |
| f64 | 8 bytes LE IEEE 754 |
| String | 8-byte LE u64 length + UTF-8 bytes |
| Vec\<T\> | 8-byte LE u64 length + elements |
| Option\<T\> | 1-byte tag (0=None, 1=Some) + value |
| enum variant | 4-byte LE u32 index + fields |
| BTreeMap\<K,V\> | 8-byte LE u64 length + sorted key-value pairs |
| [u8; N] fixed | N bytes raw, NO length prefix |
| struct | fields in declaration order, no delimiters |

**Special cases**:
- `EntityId`: custom serde — raw 16 bytes in bincode (no length prefix), base64url-no-pad in JSON
- `EventId`: custom serde — raw 32 bytes in bincode, base64url-no-pad in JSON
- `TransactionId`/`RequestId`/`QueryId`/`UpdateId`: derived serde on `Ulid` — 26-char string (34 bytes: u64 length + 26 ASCII)
- `Value::Json`: uses `json_as_bytes` — JSON → `serde_json::to_vec()` → bincode `Vec<u8>`

## Spec Files

| File | Contents | Status |
|------|----------|--------|
| `continue-implementation.md` | This file. Start here. | **Current** |
| `architectural-decisions.md` | All user-confirmed decisions | **Current** |
| `port-rules.md` | Bidirectional mapping rules, exceptions, validation | **Current** |
| `architecture.md` | Module mapping, package structure | Has errors (see cleanup tracker) |
| `ecosystem-research.md` | expo-sqlite, Yjs viability, Expo Go constraints | Has errors (V1 refs) |
| `structural-mapping-analysis.md` | 1:1 mapping analysis | Superseded by port-rules.md |
| `schema-registry-and-codegen.md` | PR #236, codegen flow | Valid but codegen deferred |
| `wire-format-interop.md` | Bincode strategy | Has errors (wrong Operation shape) |
| `yrs-yjs-interop-validation.md` | Yrs/Yjs compat plan | Has errors (says V1, should be V2) |
| `initial-porting-workflow.md` | 13-phase porting guide | Phase ordering issues noted |
| `ongoing-maintenance-workflow.md` | Drift detection CI | Valid |
| `ankurah-rs-spec-cleanup.md` | Checklist of spec errors to fix | Active |
| `_agent-work/*.md` | Detailed research outputs | Reference |

**Authoritative sources**: (1) `architectural-decisions.md` for decisions, (2) `port-rules.md` for structural rules, (3) the Rust source code for type definitions.

## Current Implementation Status

### Completed
- **Monorepo scaffolding** — All 11 packages created with correct dependencies, bun install works, tsc passes
- **Audit script** — `scripts/audit-port.ts` validates bidirectional mapping (runs via `bun run scripts/audit-port.ts`)
- **`@ankurah/proto` implementation** — 16 source files: bincode codec, all ID types, Clock, auth types, data types (Event, State, Operation, EntityState), sys, peering, request/response, update, message, human_id. Zero external deps. All annotations correct. Spot-checked against Rust: all field orders, enum variant orders, and encoding patterns match.
- **Rust worktree** — `ts-port-support` branch at `/Users/daniel/ak/ankurah-ts-support/`
- **Rust bincode fixtures** — 12 test functions in `proto/tests/bincode_fixtures.rs`, 12 `.bin` files in `proto/test_fixtures/`. Covers all proto types: IDs, Clock, auth, data, request/response, causal relations, deltas, updates, messages, presence, system items. Uses `OVERWRITE_FIXTURES` env var to toggle between compare and regenerate modes. All deterministic (no random ULIDs).
- **Proto fixture parity tests** — 24 TS tests in `packages/proto/__tests__/fixtures.test.ts` (decode + round-trip for all 12 `.bin` files). Validates TS codec reads Rust output correctly and encodes byte-identical bincode. 244 assertions. Fixture path resolves to `../ankurah-ts-support/proto/test_fixtures/`.
- **`@ankurah/signals`** — 10 source files + 4 stubs (deferred: Calculated, Map, Memo, Observer auto-tracking). Core types: Broadcast, BroadcastId, Mut, Read, Signal, Get, Peek, With, Subscribe, SubscriptionGuard, ListenerGuard, ValueCell, ReadValueCell. 45 tests passing. Zero external deps.
- **`@ankurah/ankql`** — 8 source files. Hand-written recursive descent parser with lexer, full AST types (Expr, Predicate, Selection, PathExpr, Literal, etc.), SQL generation, conversion utilities. 76 tests passing. Zero external deps.

### Audit results (latest run)
- **4 PASS**: All TS annotations valid (52 files), all MIRRORS point to real Rust files (47), no orphans, all exception citations present (4 E12 citations)
- **115 FAIL**: Expected — missing source files and test files for packages not yet implemented (core, storage, connectors, react)
- tsc compiles proto, signals, and ankql with zero errors
- **145 total tests passing** (24 proto fixture + 45 signals + 76 ankql), 463 assertions

### Next up
1. **Yrs↔Yjs V2 interop** — Generate Yrs fixtures in Rust, load in Yjs, verify round-trip
2. **`@ankurah/core`** — Entity, Transaction, Node, Context, Reactor (biggest package ~63 files)
3. **Storage engines** — storage-common traits, then expo-sqlite and better-sqlite3
4. **Connectors** — WebSocket client, local connector
5. **`@ankurah/react`** — Absorb ankurah-react-hooks, wire to TS signals
6. **Integration tests** — Spawn Rust WS servers, test TS↔Rust interop

### Rust-side work (ts-port-support branch)
- ~~Bincode fixture generation tests~~ DONE (12 tests, 12 `.bin` files)
- Integration test server binary
- Yrs V2 fixture generation
- Any spec cleanup PRs

## Agent Working Notes

### Annotation format
- Line 1: `// MIRRORS: ankurah/<crate>/src/<path>.rs` — bare path only, NO extra text like `[E2]` or `(tests module)`
- Exception citations go on a SEPARATE line 2: `// Exception E12: file-with-submodules pattern`
- Test files: `// MIRRORS: ankurah/<crate>/src/<path>.rs` (same as source, bare path)
- TS-only files: `// TS-ONLY: <reason>`

### Fixture path convention
- Rust fixtures: `/Users/daniel/ak/ankurah-ts-support/proto/test_fixtures/`
- TS tests resolve via: `path.resolve(__dirname, '../../../../ankurah-ts-support/proto/test_fixtures')`
- Hard-fail with descriptive error if fixture dir missing

### ULID serialization (confirmed)
- `ulid` crate v1.2.1 serializes as 26-char Crockford Base32 string (always, no `is_human_readable` check)
- In bincode: 8-byte u64 length (=26) + 26 ASCII bytes = 34 bytes total
- Deterministic test ULIDs constructed via: `"0000000000000000000000" + decimal(seed).padStart(4, '0')`

### Sub-agent tips
- Grant `.cargo/registry` read permission proactively for Rust source exploration
- MIRRORS annotations must be bare paths — audit script validates
- After implementation, always run: `npx tsc --noEmit -p packages/<pkg>/tsconfig.json` and `bun test packages/<pkg>/`
- Run `bun run scripts/audit-port.ts` to verify structural compliance

## Instructions for a Continuing Agent

1. **Read `architectural-decisions.md`** — all decisions are finalized
2. **Read `port-rules.md`** — structural mapping rules are non-negotiable
3. **Check active work** — look at recent commits and open branches to understand what's in flight
4. **Surface uncertainties** — do not assume; ask the supervisor
5. **Follow port-rules.md strictly** — every file needs line 1 annotation, every exception needs a rule citation
6. **Port tests** — every test in every in-scope crate must have a TS equivalent
7. **Run the audit script** — `bun run scripts/audit-port.ts` to check compliance
8. **Delegate to sub-agents** — preserve supervisor context by dispatching implementation work to background agents
