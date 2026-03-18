# ankurah-ts Port Runbook

**Start here.** This is the entry point for any agent or human doing port work on ankurah-ts — an ongoing, iterative TypeScript port of [ankurah](https://github.com/ankurah/ankurah), a Rust CRDT-backed reactive database.

This is NOT a one-time port. The Rust implementation continues to evolve. This runbook and its referenced docs define how to translate Rust code to TS, how to update the TS when Rust changes, and how to validate correctness.

---

## Definition of Done (per package)

A package is NOT complete until ALL of these are true:
1. All source files ported from Rust
2. `npx tsc --noEmit` clean (zero errors)
3. ALL tests ported — unit tests (`#[cfg(test)]`), integration tests (`tests/`), doc tests
4. ALL tests passing (`bun test` zero failures)
5. Parity audit passed (every Rust test has a matching TS test)
6. Line 1 `// MIRRORS:` annotations on every file
7. Divergences annotated with `// Divergence: <what> [E#]`

Do NOT call a package "complete" until all 7 criteria are met.

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

**Sibling Rust checkouts required**:
- `../ankurah` — Rust implementation (main branch, source of truth for reference)
- `../ankurah-ts-support` — Rust worktree on `ts-port-support` branch (fixtures, integration tests, the branch porters read from)

`ANKURAH_RS_PATH` env var (default `../ankurah`). Tests hard-fail if missing.

## Port Docs (read in this order)

| Doc | What it covers |
|-----|---------------|
| **This file** | Orientation, workflows, current status, outstanding tasks |
| [translation-rules.md](translation-rules.md) | How to translate any Rust construct to TS (file naming, types, imports, annotations, exceptions) |
| [ownership.md](ownership.md) | How Rust ownership (Drop, Mutex, RefCell, Arc, Weak, lifetimes) maps to TS |
| [ownership/provided-types.md](ownership/provided-types.md) | API reference for Disposable, Mutex, RefCell, AsyncMutex |
| [decisions.md](decisions.md) | Confirmed architectural decisions (wire format, CRDT, tooling, scope, async serialization, gotchas) |
| [punchlist/index.md](punchlist/index.md) | Per-crate punchlists: every .rs file and every test function, with status tracking |

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

## Port Order

Port in **reverse-dependency order** (leaves first). See [punchlist/index.md](punchlist/index.md) for the full dependency graph and per-crate punchlists. Nothing skips ahead — a crate is not started until all its dependencies are fully complete (all 7 DoD criteria).

## How to Port a Rust File

1. **Read the Rust file** from `ankurah-ts-support/` — understand every line
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

## Excluded Crates

| Crate | Reason |
|-------|--------|
| `storage/sled` | Rust-specific embedded DB — no Node/browser equivalent |
| `derive` | Proc macro — replaced by `defineModel()` runtime (E12) |
| `connectors/websocket-client-wasm` | WASM variant — TS uses pure websocket-client |
| `tests-wasm` | WASM test bindings — not applicable to pure TS |
| `examples/*` | Example apps — not part of library |
| `docs/example/*` | Doc examples — not part of library |

## Package Structure

| TS Package | Rust Crate | Source files | Tests | Parity audit |
|-----------|------------|-------------|-------|-------------|
| `@ankurah/base` | TS-only | Done | 68 | n/a |
| `@ankurah/ankql` | `ankql` | 8/8 | 76 | PASS |
| `@ankurah/proto` | `ankurah-proto` | 15/15 | 41 | PASS |
| `@ankurah/signals` | `ankurah-signals` | 15/15 | 68 | PASS |
| `@ankurah/storage-common` | `ankurah-storage-common` | 8/8 | 104 | pending |
| `@ankurah/core` | `ankurah-core` | 63/65 | ~400 | pending |
| `@ankurah/storage-memory` | TS-only | Done | 15 | n/a |
| `@ankurah/storage-sqlite` | `ankurah-storage-sqlite` | 6/6 | 6 (sql_builder only) | engine stubs |
| `@ankurah/storage-postgres` | `ankurah-storage-postgres` | 3/3 | 25 (sql_builder only) | engine stubs |
| `@ankurah/storage-indexeddb` | `ankurah-storage-indexeddb-wasm` | 16/16 | 8 | engine stubs |
| `@ankurah/connector-websocket` | `ankurah-websocket-client` | 3/3 | 0 | no Rust tests |
| `@ankurah/connector-websocket-server` | `ankurah-websocket-server` | 6/6 | 0 | no Rust tests |
| `@ankurah/connector-local` | `ankurah-connector-local-process` | 1/1 | 0 | no Rust tests |
| `@ankurah/ankurah` | `ankurah` (facade) | 1/1 | 0 | n/a |
| `@ankurah/eslint-plugin` | TS-only | Done | 64 | n/a |
| `@ankurah/react` | TS-only | Not started | 0 | n/a |

## Current Status

**854 tests passing, 0 failures, 31 skip**, tsc clean. ESLint ownership rules enabled (45 errors, 41 warnings).

Full re-audit in progress. Per-crate punchlists track every file and every test function: [punchlist/index.md](punchlist/index.md).

### What's working
- All source files have initial TS counterparts across all 13 crates
- ankql, proto, signals, storage-common passed prior parity audits
- Full re-audit underway (assume nothing is done — verify everything)

### Outstanding

See per-crate punchlists for detailed status. High-level gaps:

#### ESLint ownership violations
- 45 errors (assert-not-disposed, dispose-owned-fields, require-using-for-guards)
- 41 warnings (fire-and-forget, dispose-requires-registration)
- Hotspots: livequery.ts, connectors, entity.ts

#### Storage engine implementations
- SQLite: sql_builder ported, engine needs real driver (24 integration tests)
- Postgres: sql_builder ported, engine needs real driver + Docker (27 integration tests)
- IndexedDB: unit tests pass, engine needs browser/jsdom (57 integration tests)

#### Connector/networking integration
- 25 inter-node tests skipped (need LocalProcessConnection wired to Node)
- 4 websocket tests skipped (need WS client+server integration)
- 3 local_subscription tests not ported yet

#### Infrastructure
- Add lint + audit to CI
- Wire `using` declarations where needed (GC warnings in test output)

#### Rust-side
- Support branch needs rebase onto main for new proto types
- Fixtures need regenerating after rebase

### Known Issues
- ResultSetWrite uses old `write()/done()` pattern — most critical ownership violation
- WatcherSet gap-fill has async interleaving risk (fire-and-forget outside AsyncMutex)
- LiveQuery activation race (same bug in Rust, issue #146)
- `defineModel()` returns raw `{backend, fieldName, entity}` handles — alive checks can be bypassed
