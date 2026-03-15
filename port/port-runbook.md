# ankurah-ts Port Runbook

**Start here.** This is the entry point for any agent or human doing port work on ankurah-ts — an ongoing, iterative TypeScript port of [ankurah](https://github.com/ankurah/ankurah), a Rust CRDT-backed reactive database.

This is NOT a one-time port. The Rust implementation continues to evolve. This runbook and its referenced docs define how to translate Rust code to TS, how to update the TS when Rust changes, and how to validate correctness.

---

## What This Project Is

A **fully faithful TypeScript port** of ankurah, targeting **React Native / Expo Go** as the primary runtime. Expo Go cannot use WASM or native modules, so this must be pure TypeScript. The TS code should read as close to the Rust source as possible.

## Repository Layout

```
/Users/daniel/ak/
├── ankurah/                 # Rust implementation (source of truth, main branch)
├── ankurah-ts-support/      # Rust worktree on ts-port-support branch (fixtures, integration tests)
├── ankurah-ts/              # TypeScript port (this project)
│   ├── packages/            # bun workspace packages
│   └── port/                # Port specs, audit scripts, hash manifest (this directory)
```

**Sibling Rust checkout required**: `ANKURAH_RS_PATH` env var (default `../ankurah`). Tests hard-fail if missing.

## Port Docs (read in this order)

| Doc | What it covers |
|-----|---------------|
| **This file** | Orientation, workflows, current status, outstanding tasks |
| [translation-rules.md](translation-rules.md) | How to translate any Rust construct to TS (file naming, types, imports, annotations, exceptions) |
| [ownership.md](ownership.md) | How Rust ownership (Drop, Mutex, RefCell, Arc, Weak, lifetimes) maps to TS |
| [ownership/provided-types.md](ownership/provided-types.md) | API reference for Disposable, Mutex, RefCell, AsyncMutex |
| [decisions.md](decisions.md) | Confirmed architectural decisions (wire format, CRDT, tooling, scope, async serialization, gotchas) |

## Validation (run these to check your work)

```bash
npx tsc --noEmit                              # type-check
bun test                                       # all tests
bun run port/audit-port.ts                     # structural compliance (MIRRORS, coverage, drift)
npx eslint packages/                           # ownership compliance (Disposable, using, alive checks)
```

Port scripts live in `port/` and are self-documenting (`--help` or read the source):

- `audit-port.ts` — validates bidirectional Rust↔TS mapping, annotations, test coverage, drift detection
- `audit-port.ts --backpopulate` — bootstraps hash manifest from MIRRORS annotations
- `audit-port.ts --update-manifest` — updates hashes after porting Rust changes

## How to Port a Rust File

1. **Read the Rust file** — understand every line
2. **Create/overwrite the TS file** — same path under `packages/<pkg>/src/`, same snake_case filename
3. **Add line 1 annotation** — `// MIRRORS: ankurah/<crate>/src/<path>.rs`
4. **Translate mechanically** — apply [translation-rules.md](translation-rules.md):
   - `struct Foo` → `class Foo extends Struct` (from `@ankurah/base`)
   - `enum Foo` → `class Foo extends Enum<FooV>` with static variant constructors
   - `impl Drop for T` → `class T extends Drop`, override `drop()`
   - `Arc<T>` → `Arc<T>`, `Weak<T>` → `Weak<T>` (from `@ankurah/base`)
   - `&T` in fields → `Borrow<T>`, `&mut T` → `BorrowMut<T>`
   - `Mutex<T>` / `RwLock<T>` → `Mutex<T>`, `RefCell<T>` → `RefCell<T>`
   - `snake_case` → `camelCase` for functions/variables
   - `match` → `.match({})`, `if let` → `.is()`
   - Cite divergences: `// Divergence: <what> [E8]`
5. **Match the Rust code as closely as possible** — same declaration order, same `use` statement order, same structure. The goal is line-for-line correspondence.
6. **Bincode**: For proto types, encode/decode as methods on the class. Reference `/Users/daniel/code/domcorder/proto-ts/` for bincode codec patterns.
7. **Do NOT run tests** — a separate test agent handles that
8. **Do NOT edit files outside your assigned file**

## Agent Rules of Engagement

When porting agents are dispatched to do the work:

1. Each agent works on **ONE file at a time**
2. The task is: read ONE Rust file, write/overwrite ONE TS file
3. Agents must **await positive confirmation from the team lead** before proceeding to the next file
4. **No editing outside the assigned file** — no touching index.ts, no touching other source files
5. **No tests run by porting agents** — a separate test agent runs tsc + bun test when dependencies are met
6. **No splitting or merging** — one Rust file maps to one TS file, period
7. **Audit every line** — the goal is for the TS to match the Rust as closely as possible, right down to declaration order and use statement order. This will be closely reviewed.
8. **Do not modify spec files** (anything in `port/`) — ever

## How to Update When Rust Changes

1. **Detect drift** — `bun run port/audit-port.ts` flags changed Rust files
2. **Diff the Rust change** — understand what changed
3. **Find the TS file** — MIRRORS annotation tells you which TS file corresponds
4. **Apply the delta** — translate the Rust diff using the same rules as a fresh port
5. **Preserve intentional divergences** — check for `// Divergence:` comments; preserve unless the Rust change makes them obsolete
6. **Run tests** — `npx tsc --noEmit && bun test`
7. **Update manifest** — `bun run port/audit-port.ts --update-manifest`

## Wire Protocol Validation

The TS port must be byte-compatible with the Rust implementation. This is validated via fixtures:

**Bincode fixtures**: Rust-side tests in `ankurah-ts-support/proto/tests/bincode_fixtures.rs` generate `.bin` files. TS tests in `packages/proto/__tests__/fixtures.test.ts` decode them and verify round-trip byte equality. 12 fixtures, 24 TS tests, 244 assertions.

**Yrs↔Yjs V2 interop**: Rust-side tests in `ankurah-ts-support/proto/tests/yrs_v2_fixtures.rs` generate Yrs V2 encoded docs. TS tests in `packages/core/__tests__/yrs-yjs-interop.test.ts` load them into Yjs and verify content. 6 fixtures, 10 TS tests.

**Regenerating fixtures**: `cd ankurah-ts-support && OVERWRITE_FIXTURES=1 cargo test -p ankurah-proto --test bincode_fixtures`

**The rule**: We match the Rust wire protocol exactly. Bincode only. No reimagining. No JSON alternative. The fixture tests are the proof.

## Package Structure

| TS Package | Rust Crate | Status |
|-----------|------------|--------|
| `@ankurah/proto` | `ankurah-proto` | Done |
| `@ankurah/core` | `ankurah-core` | Layers 0-6b done, Layer 7 partial |
| `@ankurah/signals` | `ankurah-signals` | Done (core types) |
| `@ankurah/ankql` | `ankql` | Done |
| `@ankurah/storage-common` | `ankurah-storage-common` | Done |
| `@ankurah/storage-memory` | TS-only | Done |
| `@ankurah/connector-websocket` | `ankurah-websocket-client` | Not started |
| `@ankurah/connector-local` | `ankurah-connector-local-process` | Not started |
| `@ankurah/react` | TS-only (replaces Rust UniFFI bridge) | Not started |
| `@ankurah/eslint-plugin` | TS-only | Done (8 rules, 64 tests) |

## Current Status

**707 tests passing, 0 failures**, tsc clean across all packages. Last commit: `edfd9a0`.

### What's done
- **@ankurah/base**: AkObject, Struct, Enum<V>, Drop, Arc, Weak, Borrow, BorrowMut, Mutex, RefCell, AsyncMutex — 68 tests
- **@ankurah/ankql**: All 8 source files ported, all 76 tests passing. Parity audit PASS (59/59 Rust tests).
- **@ankurah/proto**: All 15 source files ported, 29 fixture tests + 12 Yrs V2 interop tests passing. Parity audit PASS.
- **@ankurah/signals**: All 15 source files ported, 68 tests passing. Parity audit PASS (34/34 Rust tests).
- **@ankurah/storage-common**: All 8 source files ported, 104 tests passing. Parity audit pending (2 planner tests short of Rust's 71).
- **@ankurah/core**: ~44/65 source files ported, 285 tests (6 skip). tsc clean.
- **@ankurah/storage-memory**: 15 tests passing. TS-only package.
- **@ankurah/eslint-plugin**: 8 ownership rules, 64 tests.
- **Ownership model**: AkObject→Struct/Enum/Drop hierarchy, Arc/Weak refcounting, Borrow non-propagation. Fully tested.
- **Rust fixtures**: Yrs V2 committed, 4 new bincode fixtures added (CausalAssertion, Principal, Attested<Event>, concurrent_merge).

### Outstanding Tasks

#### Core — remaining source files (~21)
- `peer_subscription/mod.rs`, `client_relay.rs`, `server.rs` — networking (Layer 7)
- `property/value/pn_counter.rs` — deferred (dead code in Rust)
- `property/backend/pn_counter.rs` — deferred (dead code)
- `traits.rs`, `task.rs` — small utility files (task.ts created)
- `type_resolver.rs` — 240 lines, 6 tests depend on it
- `collation.rs` — created but needs integration
- `util/mod.rs`, `util/cast.rs`, `util/expand_states.rs`, `util/iterable.rs`, `util/ivec.rs`, `util/safemap.rs`, `util/safeset.rs` — most map to JS primitives
- `model.rs` as `model/index.ts` (E12) — done
- `lib.rs` → `index.ts` — needs re-export updates

#### Storage engines (0/25 files)
- `@ankurah/storage-sqlite` — 6 files + 5 integration tests
- `@ankurah/storage-postgres` — 3 files + 12 integration tests
- `@ankurah/storage-indexeddb` — 16 files + 13 integration tests

#### Connectors (0/10 files)
- `@ankurah/connector-websocket` — 3 files
- `@ankurah/connector-websocket-server` — 6 files
- `@ankurah/connector-local` — 1 file

#### Facade (0/1)
- `@ankurah/ankurah` — re-exports

#### Integration tests (0/52)
- See punchlist.md T1-T52

#### Parity audits needed
- storage-common (2 planner tests short)
- core (once remaining files ported)

#### Known bugs
- 2 LiveQuery gap-filling test failures (pre-existing logic bug, not Enum migration)
- 6 skipped tests depend on TypeResolver (not yet ported)

#### Rust-side
- Support branch needs rebase onto main for new proto types
- Fixtures need regenerating after rebase

#### Port infrastructure
14. **Enable eslint-plugin-ankurah** — configure ESLint in the repo to actually run the ownership rules
15. **Run linter on existing code** — will surface all the ownership violations that need fixing (items 4-8 above)
16. **Add lint + audit to CI** — automated compliance checking

### Known Issues
- ResultSetWrite uses old `write()/done()` pattern — most critical ownership violation
- WatcherSet gap-fill has async interleaving risk (fire-and-forget outside AsyncMutex)
- LiveQuery activation race (same bug in Rust, issue #146)
- `defineModel()` returns raw `{backend, fieldName, entity}` handles — alive checks can be bypassed
