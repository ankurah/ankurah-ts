# ankurah-ts Design Resume

**Purpose**: This document provides sufficient context for a fresh agent to continue the ankurah-ts design and implementation work. Read this first, then `architectural-decisions.md`, then refer to other specs as needed.

**Last updated**: 2026-02-10

## Project Goal

Create a **fully faithful TypeScript port** of [ankurah](https://github.com/ankurah/ankurah) (a Rust CRDT-backed reactive database), targeting **React Native / Expo Go** as the primary runtime. Expo Go cannot use WASM or native modules, so this must be pure TypeScript.

## Repository Layout

```
/Users/daniel/ak/
├── ankurah/                 # Rust implementation (source of truth)
├── ankurah-ts/              # TypeScript port (this project)
│   └── specs/               # Design documents (this directory)
│       └── _agent-work/     # Detailed agent research outputs
/Users/daniel/code/
└── domcorder/               # Reference project for bincode TS patterns
    └── proto-ts/            # Bincode codec reference implementation
```

ankurah-ts assumes a sibling checkout of `ankurah/` for test fixtures and structural comparison.

## Key Architectural Decisions (User-Confirmed)

See `architectural-decisions.md` for the full authoritative list. Summary:

1. **Fully faithful port** — Mirror Rust 1:1 for agentic maintainability. Error types, file structure, naming all track Rust.
2. **Bincode wire format only** — No JSON alternative. Reference implementation: `/Users/daniel/code/domcorder/proto-ts/`.
3. **ankurah/proto structs are authoritative** — Specs must not duplicate or paraphrase proto type definitions. The Rust source is the single source of truth.
4. **Yrs V2 encoding** — Rust uses V2 exclusively. TS must use `Y.applyUpdateV2()` / `Y.encodeStateAsUpdateV2()`. NOT V1.
5. **Phase 1 scope — nearly everything**:
   - **In scope**: proto, core, signals, ankql, Entity/Model/View/Mutable, LWW + Yjs backends, Transaction, Node, Context, Reactor, LiveQuery, AnkQL parser, expo-sqlite, better-sqlite3 (Node testing), in-memory storage, WebSocket client, local connector, React Native hooks, `Attested<T>`, `CausalRelation`, lineage types.
   - **Out of scope**: WebSocket server, PostgreSQL storage, Sled storage, PN Counter backend only.
6. **CLI codegen dropped from Phase 1** — Hand-write model wrappers. Advantage over WASM: no macro monomorphization needed.
7. **Error handling: parity with Rust** — Mirror `MutationError`, `RetrievalError`, `StateError`, `PropertyError`, `DecodeError` as TS Error subclasses.
8. **Monorepo tooling: exploring bun** — pnpm+Turborepo is fallback. domcorder already uses bun workspaces. Decision pending research.
9. **Reference fixtures** at `ankurah/proto/test/fixtures/` — Rust tests generate bincode fixtures, TS tests read from sibling checkout.
10. **Polyfills**: `expo-crypto` (crypto.getRandomValues) + `fast-text-encoding` (TextDecoder). Import before any ankurah/Yjs code.

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

### Key File Locations in ankurah/ (Corrected)

- Derive macro: `derive/src/model/{view,mutable,model,description,backend_registry,backend}.rs`
- Entity: `core/src/entity.rs`
- Model traits: `core/src/model.rs`
- Backends: `core/src/property/backend/{lww,yrs}.rs`
- Active types: `core/src/property/value/{lww,yrs,entity_ref,json,pn_counter}.rs`
- Transaction: `core/src/transaction.rs`
- Node: `core/src/node.rs`
- Context: `core/src/context.rs`
- Reactor: `core/src/reactor.rs` + `core/src/reactor/{watcherset,property_path,update,candidate_changes,comparison_index,fetch_gap,subscription,subscription_state}.rs`
- Signals: `signals/src/{signal,broadcast,observer,context,reactive_graph,porcelain,value}.rs` + subtypes
- Proto: `proto/src/{data,message,request,update,clock,id,sys,auth,peering,subscription,transaction,collection,human_id,error}.rs` (14 modules)
- Storage traits: `storage/common/src/{traits,types,bounds,filtering,planner,predicate,sorting}.rs` (7 modules, NOT "just traits")
- SQLite storage: `storage/sqlite/src/`
- WS client: `connectors/websocket-client/src/`
- Local connector: `connectors/local-process/src/`
- AnkQL: `ankql/src/{grammar,ast,conversion,parser,selection,error}.rs` + `ankql.pest`

### PR #236 (Property Registration)

WIP. Current `proto/src/sys.rs` only has `SysRoot`/`Collection`/`Other`. No `BackendKind`/`ValueType` enums yet. NOT a blocker for Phase 1.

## Answers to Previously Open Questions

1. **Yrs client IDs**: Random. `yrs::Doc::new()` with no explicit assignment. TS: use default `new Y.Doc()`.
2. **Bincode configuration**: Legacy bincode 1.3.x default. Fixed u64 lengths, u32 enum variants, LE. See table above.
3. **domcorder bincode patterns**: Access granted. Agent investigation in progress. See `_agent-work/domcorder-analysis.md`.
4. **Yrs/Yjs version compat**: Yrs 0.24.0 ↔ Yjs 13.6.x. V2 encoding is compatible. See `_agent-work/yrs-yjs-interop-findings.md`.

## Remaining Open Questions

See `_agent-work/remaining-questions.md` for the full list (~30 items). Key clusters:

- **Bincode details**: ULID string serialization for non-EntityId types, BTreeMap sort order, BigInt vs Number API
- **AnkQL parser**: hand-written vs Peggy vs Chevrotain; parse-only vs parse+evaluate
- **Entity lifecycle in TS**: identity without pointer equality, transaction cleanup without Drop, WeakRef on Hermes
- **Signals**: which types needed for Phase 1 minimum, BroadcastId without pointer identity
- **Storage interface**: sync vs async, in-memory engine, how much of storage-common's 7 modules needed client-side
- **Reactor scope**: how much of the 8+ file reactor is client-relevant vs server-only
- **React hooks**: API design, field-level vs entity-level subscriptions
- **Monorepo**: bun vs pnpm (research in progress), build tooling, test framework
- **Spec cleanup**: correct vs delete stale specs (tracked in `ankurah-rs-spec-cleanup.md`)

## Spec Files

| # | File | Contents | Status |
|---|------|----------|--------|
| 1 | `design-resume.md` | This file. Start here. | **Updated 2026-02-10** |
| 2 | `architectural-decisions.md` | All user-confirmed decisions | **Updated 2026-02-10** |
| 3 | `architecture.md` | Module mapping, package structure | Has errors (see cleanup tracker) |
| 4 | `ecosystem-research.md` | expo-sqlite, Yjs viability, Expo Go constraints | Has errors (V1 refs) |
| 5 | `structural-mapping-analysis.md` | 1:1 mapping analysis | Undercounts files |
| 6 | `schema-registry-and-codegen.md` | PR #236, codegen flow | Valid but codegen deferred |
| 7 | `wire-format-interop.md` | Bincode strategy | Has errors (wrong Operation shape) |
| 8 | `yrs-yjs-interop-validation.md` | Yrs/Yjs compat plan | Has errors (says V1, should be V2) |
| 9 | `initial-porting-workflow.md` | 13-phase porting guide | Phase ordering issues noted |
| 10 | `ongoing-maintenance-workflow.md` | Drift detection CI | Valid |
| - | `ankurah-rs-spec-cleanup.md` | Checklist of spec errors to fix | **New 2026-02-10** |
| - | `_agent-work/*.md` | Detailed research outputs | **New 2026-02-10** |

**Note**: Several specs contain factual errors identified by cross-checking against the actual Rust source. See `ankurah-rs-spec-cleanup.md` for the full list. The authoritative sources are: (1) `architectural-decisions.md` for decisions, (2) the Rust source code for type definitions.

## Next Steps for a Continuing Agent

### Priority 1: Resolve remaining open questions

Present the remaining questions from `_agent-work/remaining-questions.md` to the user for decisions. Key blockers:
- AnkQL parser strategy
- Storage interface (sync vs async)
- Monorepo tooling (bun research pending)
- Reactor scope for client

### Priority 2: Scaffold and begin implementation

1. **Scaffold the monorepo** — packages per `architectural-decisions.md`
2. **Start with @ankurah/proto** — type definitions + bincode codec, following domcorder patterns
3. **Generate Rust-side fixtures** — add test to `ankurah/proto` that writes reference .bin files to `proto/test/fixtures/`
4. **Validate Yrs↔Yjs V2 interop** — generate Yrs fixtures, load in Yjs, verify round-trip

### Priority 3: Clean up specs

Apply fixes from `ankurah-rs-spec-cleanup.md`. Delete sections that duplicate proto struct definitions.
