# ankurah-ts Port Runbook

**Start here.** This is the entry point for any agent or human doing port work on ankurah-ts — an ongoing, iterative TypeScript port of [ankurah](https://github.com/ankurah/ankurah), a Rust CRDT-backed reactive database.

This is NOT a one-time port. The Rust implementation continues to evolve. This runbook and its referenced docs define how the Rust source becomes TypeScript, how the TypeScript follows the Rust when it changes, and how to validate correctness.

**The port is produced by the transpiler in `transpile/`.** The hand-written TypeScript in `packages/` predates the transpiler and is being replaced by it: treat it as output, not as source. The two things genuinely written by hand are the ownership runtime in `packages/base/src` and the provided files that stand in for code the transpiler will not translate.

Statements in this runbook that were true of the earlier hand-porting regime and are not true now are listed, with what replaced them, in [retractions-2026-09-02.md](retractions-2026-09-02.md).

---

## Definition of Done (per package)

A package is NOT complete until ALL of these are true:
1. Every in-scope Rust source file has been transpiled, or is covered by a `.provided.ts` file or a stub declaration
2. `npx tsc --noEmit` clean for that package (zero errors)
3. Every Rust test transpiled — unit tests (`#[cfg(test)]`), integration tests (`tests/`), and the fixture consumers for anything on the wire
4. `scripts/test-gate.sh <package>` green: zero failures, zero errors, zero type errors
5. Parity audit passed (every Rust item has a TS counterpart)
6. Line 1 `// MIRRORS:` annotations on every file
7. Divergences annotated with `// Divergence: <what> [E#]`

Do NOT call a package "complete" until all 7 criteria are met. Criterion 4 says "errors" as well as "failures" for a reason — see the validation section below.

---

## What This Project Is

A **fully faithful TypeScript port** of ankurah. The target environments are **the browser** and **React Native / Expo**; Node is supported for testing but is not a primary target. The port ships as pure TypeScript — Expo Go loads neither WASM nor native modules, and being TypeScript is the point of the port in the browser — and the TS reads as close to the Rust source as possible.

Where ankurah already maintains a browser code path in Rust, that path is the source of truth for the port, because it is the path ankurah itself tests. So `storage/indexeddb-wasm` and `connectors/websocket-client-wasm` are what get transpiled, and their `web-sys` calls resolve to the browser APIs TypeScript already has. Nothing ships as WASM — the crate names say wasm because that is the Rust target where those browser APIs are reachable. (The transpiler's `cfg` evaluation is being moved to ankurah's wasm32 configuration for the same reason; today it evaluates every non-feature predicate to false, which keeps the wrong branch. See the `cfg` note in [transpiler-spec.md](transpiler-spec.md).)

## Repository Layout

```
/Users/daniel/ak/
├── ankurah/                 # Rust implementation (source of truth, main branch)
├── ankurah-ts-support/      # Rust worktree on ts-port-support branch (fixtures, integration tests)
├── ankurah-ts/              # TypeScript port (this project)
│   ├── packages/            # bun workspace packages
│   ├── transpile/           # the Rust→TS transpiler (a Rust binary) and its specs
│   └── port/                # Port specs, audit scripts, hash manifest (this directory)
```

**Sibling Rust checkouts required**:
- `../ankurah` — Rust implementation (main branch, source of truth for reference)
- `../ankurah-ts-support` — Rust worktree on `ts-port-support` branch (fixtures, integration tests, the branch the transpiler reads from)

`ANKURAH_RS_PATH` env var (default `../ankurah`). Tests hard-fail if missing.

## Port Docs (read in this order)

| Doc | What it covers |
|-----|---------------|
| **This file** | Orientation, scope, workflow, validation |
| [RESUME.md](RESUME.md) | Current status: what is finished, what is frozen for review, what is in flight |
| [ownership.md](ownership.md) | The ownership contract: how Rust's drop, moves, borrows, `Arc`, `Mutex` and lifetimes behave in TypeScript, and what the transpiler must emit against |
| [ownership/provided-types.md](ownership/provided-types.md) | API reference for the ownership types in `packages/base/src` |
| [translation-rules.md](translation-rules.md) | The mechanical translation rule set: file naming, types, imports, annotations, and the numbered exception rules |
| [../transpile/SYMBOL-TABLE-SPEC.md](../transpile/SYMBOL-TABLE-SPEC.md) | How the transpiler decides what Rust type an expression has, and what it refuses to guess |
| [transpiler-spec.md](transpiler-spec.md) | The transpiler's pipeline, transform modules, and configuration |
| [decisions.md](decisions.md) | Confirmed architectural decisions (wire format, CRDT, tooling, scope, async, gotchas) |
| [retractions-2026-09-02.md](retractions-2026-09-02.md) | Every statement in these docs that a repealed premise made false, and its replacement |
| [punchlist/index.md](punchlist/index.md) | Per-crate punchlists: every .rs file and every test function |

## Validation (run these to check your work)

```bash
scripts/test-gate.sh                           # tests + typecheck, per package; fails on any of them
npx tsc --noEmit                               # type-check (root; see the caveat below)
npx eslint packages/                           # ownership compliance (drop, registration, guard escape)
bun run port/audit-port.ts                     # structural compliance (MIRRORS, coverage, drift)
cd transpile && cargo test                     # the transpiler's own unit, snapshot and budget tests
```

Both `scripts/test-gate.sh` and the `transpile` exclusion in the root `tsconfig.json` arrive with the transpiler branch; if neither is present in your checkout, you are behind it.

**Use the gate, not a bare `bun test`.** Bun abandons the rest of a test *file* at its first unhandled error — a throw at module scope, a fatal from the ownership runtime — and reports it on a separate ` N error` line. The tests after it in that file never run and never appear in any count, so a run that says "0 fail" while reporting errors has not told you the tests passed; it has told you where it stopped reading. `scripts/test-gate.sh` counts pass, fail, error and type errors per package and refuses to pass on any of them. Every test count recorded in this repository before 2026-09-02 was measured without that distinction and undercounts the tests that exist.

**The root typecheck lied until 2026-09-02.** `npx tsc --noEmit` at the repository root reported a handful of errors while hundreds existed: two parse errors in the captured TypeScript under `transpile/golden/` made TypeScript report syntactic diagnostics only and skip the semantic pass over the whole tree. The root `tsconfig.json` now excludes `transpile`. If a root typecheck reports only parse errors inside `transpile/`, you are looking at that bug and the number you have is not the number of type errors.

Port scripts live in `port/` and are self-documenting (`--help` or read the source):

- `audit-port.ts` — validates bidirectional Rust↔TS mapping, annotations, test coverage, drift detection
- `audit-port.ts --backpopulate` — bootstraps hash manifest from MIRRORS annotations
- `audit-port.ts --update-manifest` — updates hashes after porting Rust changes

## Port Order

Crates are transpiled in **reverse-dependency order** (leaves first), because the transpiler resolves types across crate boundaries and needs the dependency's declarations before it can type the dependent's expressions. See [punchlist/index.md](punchlist/index.md) for the dependency graph.

## How a Rust File Becomes TypeScript

The transpiler does this; no one writes the TypeScript by hand.

1. **Run the transpiler over the crate** — `cd transpile && cargo run -- batch <rust-crate-src> <out-dir> --crate-name <crate>`, where `<rust-crate-src>` is that crate's `src/` in the `ankurah-ts-support` checkout. It parses each `.rs` file with `syn`, builds a module tree and type registry across the whole crate, translates bodies, and writes TypeScript.
2. **Read the diagnostics.** The transpiler refuses to guess: a construct it cannot type produces a diagnostic naming the Rust file and line, and the diagnostic count per crate is pinned by `transpile/tests/diagnostics_budget.toml`. A number that goes up is a regression, and one that goes down is progress that must be explained.
3. **Diff the output.** `git diff` against the previous output is the validation tool — the transpiler does no diffing itself. A changed file is either a transpiler fix or a transpiler regression, and you have to say which.
4. **Fix the transpiler, never the output.** Hand edits to `packages/*/src` are overwritten on the next run and hide the defect that produced them.
5. **Check the snapshots and goldens.** `transpile/tests/snapshots/` holds the current output for proto, ankql and signals; `transpile/goldens/` holds hand-vetted idiom translations. A snapshot moves only with a written explanation of why the new output is more correct.

What the emitted TypeScript looks like is set by [translation-rules.md](translation-rules.md) and [ownership.md](ownership.md). In outline: a Rust struct becomes a class extending `Struct`, an enum a class extending `Enum<V>`, `impl Drop for T` a `protected override onDrop()`, a block-owned value a `try/finally` that drops it, `Result<T, E>` a returned `Result` value, `snake_case` becomes `camelCase`, and `RwLock`, `RefCell`, `Arc`, `Weak` and the rest come from `@ankurah/base` with their Rust semantics intact.

### When the transpiler should not translate a file

Some files get a hand-written `.provided.ts` instead. The criterion is narrow and the count is meant to stay low: **macros, out-of-family code, platform bindings, and files that are simply too awkward to get right through the transpiler.** A provided file still participates in drift detection — the transpiler knows the Rust source it stands for and flags when that source changes — and it still has to obey the ownership contract, because the cascade will walk whatever it hands back.

## Agent Rules of Engagement

1. **Do not hand-edit transpiled output.** Every defect in `packages/*/src` is a transpiler defect; fix it in `transpile/src/` and re-run. The exceptions are `packages/base/src` (the hand-written ownership runtime) and `.provided.ts` files.
2. **Do not modify spec files** (anything in `port/`) unless that is your assignment.
3. **NEVER skip or stub silently.** A construct the transpiler cannot handle files a diagnostic; it does not emit a guess. In hand-written code, `test.skip()` and `throw Error('TODO')` are only acceptable for genuine platform gaps, and every skip must be justified against the Rust source. If you are stuck, **STOP and ask.** Do not substitute empty arrays, hardcoded booleans, or type casts to make it compile.
4. **`as unknown as` requires justification** — every `as unknown as` cast needs a `// Divergence: <reason> [E#]` comment on the same or preceding line.
5. **Use named team agents** — always pass the `name` parameter when spawning agents so the user can see individual agent status.
6. **Don't shut down agents on interrupt** — when the user interrupts, they are talking to the agent, not killing it. Their accumulated context is worth keeping.
7. **Check mailbox between major steps** — check for messages from the supervisor between units of work, so a redirected task does not get finished anyway.

## How to Update When Rust Changes

1. **Detect drift** — `bun run port/audit-port.ts` flags changed Rust files
2. **Diff the Rust change** — understand what changed
3. **Re-run the transpiler** on the affected crates
4. **Diff the TypeScript** — the output delta should correspond to the Rust delta and nothing else
5. **Read the diagnostics** — new Rust constructs surface as new diagnostics, which is the transpiler telling you it needs work before this change can be translated
6. **Run the gate** — `scripts/test-gate.sh`
7. **Update manifest** — `bun run port/audit-port.ts --update-manifest`

## Wire Protocol Validation

The TS port must be byte-compatible with the Rust implementation. Rust-side tests on the `ts-port-support` branch generate fixtures; TS tests read them back.

**Bincode fixtures**: `ankurah-ts-support/proto/tests/bincode_fixtures.rs` generates 24 `.bin` files, each with a sidecar recording the offset and length of every item inside it, so a decode failure names the field rather than the file.

**Yrs↔Yjs V2 interop**: `ankurah-ts-support/proto/tests/yrs_v2_fixtures.rs` generates 12 Yrs V2 encoded documents. The TS tests load them into Yjs and verify content.

**Core LWW**: 10 cases covering the last-write-wins property backend.

**AnkQL and the planner**: `ankql/test_fixtures/parse_cases.json` holds 91 parse cases (66 accepted, 25 rejected) and `storage/common/test_fixtures/plans.json` holds 26 planner cases covering 48 plans. These pin behaviors of the current Rust parser and planner that the port must reproduce exactly, including the ones that look like bugs.

**Regenerating fixtures**: `cd ankurah-ts-support && OVERWRITE_FIXTURES=1 cargo test -p ankurah-proto --test bincode_fixtures`

**The rule**: We match the Rust wire protocol exactly. Bincode only. No reimagining. No JSON alternative. The fixture tests are the proof.

## Crate Scope

Transpiled:

| Crate | Notes |
|-------|-------|
| `proto`, `ankql`, `signals`, `core` | The measured corpus |
| `storage/common` | Planner, predicates, bounds, sorting; no platform dependencies |
| `storage/sqlite` | SQL builder, value, error and engine transpiled. The `rusqlite` binding in `connection.rs` becomes a provided file exposing a small driver interface |
| `storage/indexeddb-wasm` | For the browser; the `web-sys` glue resolves to the IndexedDB API |
| `connectors/websocket-client-wasm` | The `web-sys` WebSocket client is the browser and React Native client |
| `connectors/local-process` | |
| `ankurah` (facade) | |

Not transpiled:

| Crate | Reason |
|-------|--------|
| `storage/postgres` | Server-side database; neither primary target can reach it |
| `storage/sled` | Rust-specific embedded DB with no browser or React Native equivalent |
| `connectors/websocket-server` | The Rust server stays the server |
| `connectors/websocket-client` | The tokio-tungstenite client; the browser client is the wasm one above |
| `derive` | Proc macro — replaced by the `defineModel()` runtime (translation-rules.md exception E1, the derive-crate rule) |
| `tests-wasm` | WASM test bindings |
| `examples/*`, `docs/example/*` | Not part of the library |

**Where a backend differs by environment**, one transpiled package holds all of the ankurah logic and one thin hand-written package per environment provides only the driver, named crate first and environment second: `storage-sqlite-expo` over expo-sqlite and `storage-sqlite-node` over better-sqlite3. Both drivers are synchronous, so the transpiled engine stays synchronous. SQLite is the only crate split this way. The existing `packages/storage-expo-sqlite` and `packages/storage-better-sqlite3` are the packages to be renamed; that rename has not happened yet.

## Package Structure

The parity and completeness columns this table used to carry described the hand-porting regime and are gone: a package's real state is what `scripts/test-gate.sh` reports today, and the transpiler's diagnostic budget is what says how much of a crate it can translate. See [RESUME.md](RESUME.md) for status.

| TS Package | Rust Crate |
|-----------|------------|
| `@ankurah/base` | TS-only — the hand-written ownership runtime |
| `@ankurah/ankql` | `ankql` |
| `@ankurah/proto` | `ankurah-proto` |
| `@ankurah/signals` | `ankurah-signals` |
| `@ankurah/storage-common` | `ankurah-storage-common` |
| `@ankurah/core` | `ankurah-core` |
| `@ankurah/storage-sqlite` | `ankurah-storage-sqlite` |
| `@ankurah/storage-indexeddb` | `ankurah-storage-indexeddb-wasm` |
| `@ankurah/connector-websocket` | `ankurah-websocket-client-wasm` |
| `@ankurah/connector-local` | `ankurah-connector-local-process` |
| `@ankurah/ankurah` | `ankurah` (facade) |
| `@ankurah/storage-memory` | TS-only test utility |
| `@ankurah/eslint-plugin` | TS-only |
| `@ankurah/react` | TS-only (React hooks; replaces the feature-gated Rust module) |

Three packages in `packages/` are out of the transpiled scope above and are left in place rather than deleted: `storage-postgres` and `connector-websocket-server` mirror crates that are no longer in scope, and `storage-better-sqlite3` and `storage-expo-sqlite` are the driver packages awaiting the rename. Removing or renaming any of them is a staging decision, not something to do in passing.

## Current Status

See [RESUME.md](RESUME.md). It is the status file and is kept current; this runbook is not.
