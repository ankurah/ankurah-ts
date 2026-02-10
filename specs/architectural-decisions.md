# Architectural Decisions

Last updated: 2026-02-10

---

## Wire Format

- Bincode only. No JSON wire format alternative.
- TS must implement bincode reader/writer for all proto types.
- **ankurah/proto structs are authoritative** -- specs must not duplicate or paraphrase the structs defined in `ankurah/proto/src/`. Any spec that overlaps with proto definitions should delete the overlapping descriptions (tracked in `ankurah-rs-spec-cleanup.md`).
- **domcorder as reference implementation** -- `/Users/daniel/code/domcorder/proto-ts/` is the authoritative reference for how to implement a bincode codec in TypeScript. Patterns from domcorder (reader/writer structure, enum variant encoding, Vec lengths, Option encoding) should be followed in ankurah-ts.
- **Reference fixtures** -- Rust-side tests generate bincode reference fixtures at `ankurah/proto/test/fixtures/` for cross-language validation. ankurah-ts tests read fixtures from the sibling `../ankurah/` checkout (required).

## CRDT Backend

- Yjs (pure JS) replaces Yrs (Rust) for collaborative text fields.
- **Yrs V2 encoding confirmed** -- The Rust code uses V2 exclusively (`encode_state_as_update_v2`, `decode_v2`, `encode_diff_v2`). The TS port must use `Y.applyUpdateV2()` / `Y.encodeStateAsUpdateV2()`. V1 encoding is NOT used.
- Yrs/Yjs interop must be validated carefully before proceeding (state buffers, operations, merge behavior).
- **Yrs client ID: random** -- Use default `Y.Doc()` with random client ID, matching Rust behavior.

## Port Fidelity

- Fully faithful port. No corners cut.
- Mirror Rust file/struct/impl/test structure 1:1 wherever possible.
- The closer the structural mapping, the easier agentic validation and ongoing maintenance.
- camelCase for TS names, but preserve the same logical names.
- **Error handling: parity with Rust** -- Mirror Rust error types (`MutationError`, `RetrievalError`, `StateError`, `PropertyError`, `DecodeError`) as TS `Error` subclasses. Goal is the simplest, most consistent mapping between Rust and TS for port maintenance.

## Codebase Organization

- Three clear zones in the TS codebase:
  - **1:1 mapped**: Files mirroring a specific Rust file. Annotate with `// MIRRORS: ankurah/core/src/entity.rs`
  - **Completely different**: Files with no Rust counterpart. Annotate with `// TS-ONLY: no Rust counterpart`
  - **Bridge code**: Files translating between Rust-compatible and TS-native patterns. Annotate inline where mapping diverges.

## Phase 1 Scope

- **No descoping of core libraries** -- `proto`, `core`, `signals`, `ankql` are ALL fully in scope for Phase 1. This includes `Attested<T>`, `CausalRelation`, and all lineage types (they appear in the wire protocol and must be ported).
- **In scope**: Entity, Model/View/Mutable, LWW backend, Yjs backend, Transaction, Node, Context, Reactor, LiveQuery, Signals, AnkQL parser, expo-sqlite storage, better-sqlite3 storage (Node testing), in-memory storage, WebSocket client, local connector, React Native hooks, Attested<T>, CausalRelation, lineage types.
- **Out of scope**: WebSocket server (use Rust server), PostgreSQL storage, Sled storage, PN Counter backend.
- IndexedDB storage, WASM build target, and policy agent (beyond PermissiveAgent) remain out of scope for Phase 1 but are not permanently descoped.

## Model Wrappers and Codegen

- **Phase 1**: Hand-write TypeScript model wrappers (View, Mutable, Input, Ops) to match Rust structs.
- **CLI codegen dropped from Phase 1** -- Focus on hand-written example views/mutables. Notable advantage of the pure-TS port over WASM: no macro monomorphization needed. Codegen is a later optimization.
- The derive macro in Rust has no direct TS equivalent; the codegen CLI is its eventual replacement.

## Monorepo Tooling

- **pnpm + Turborepo** is the default recommendation.
- **Exploring bun workspaces** -- User wants to evaluate bun workspaces as an alternative (domcorder already uses bun workspaces successfully). Decision pending.

## Polyfills and Runtime

- **Polyfill boot sequence** -- `expo-crypto` for `crypto.getRandomValues`, `fast-text-encoding` for `TextDecoder`. Import before any ankurah/Yjs code. Use whatever approach is most elegant.
- **Primary target**: React Native / Expo Go (no WASM, no native modules).
- **Secondary target**: Node.js (for testing with better-sqlite3).
- expo-sqlite for mobile storage, standard WebSocket API for sync.

## Minimal Rust Changes Required

- Complete PR #236 (property registration / schema registry).
- Add bincode reference fixture generation to `ankurah/proto`.
- Add schema export capability (derive macro emits `schema_json()` method on Model trait).
- These benefit the Rust codebase regardless of the TS port.
