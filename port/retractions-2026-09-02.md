# Retractions, 2026-09-02

Several premises the port docs and lint rules were written on were repealed
during 2026. The docs still stated them and the rules still enforced them. This
is the audit trail: every statement that a repealed premise made false, what is
true instead, and what was done about it.

**Dispositions**: *retracted* — the statement was wrong and is gone or struck
through; *re-justified* — the statement stays, with the reason it rests on
replaced; *historical* — the document is a dated record and is now marked as
one, unchanged inside; *retired* — a lint rule that no longer runs.

Line numbers are from the files as they stood before this pass.

## The repealed premises

1. **`using` declarations and `[Symbol.dispose]` are the ownership model.**
   Retired: Hermes refuses `using` outright (facebook/hermes,
   `lib/Sema/SemanticResolver.cpp` raises "using declarations are not yet
   supported", pinned by `test/Parser/using-declaration-error.js`), and Expo Go
   runs Hermes. The runtime dispatches on `.drop()`; the transpiler emits
   `.drop()` calls, `onDrop()` for `impl Drop`, `try/finally` for block-owned
   values, and statement-scoped guard temporaries. A guard's second drop is
   idempotent by design; every other second drop is fatal.
2. **"A Rust function returning `Result` throws in TypeScript."** Retired by
   Daniel's ruling of 2026-03-19: `Result` is a returned value, and a throw is a
   panic. The transpiled packages still throw, which is a known defect awaiting
   the emission step.
3. **Macro expansion.** Never: targeted handling per macro family, or a
   `.provided.ts` file. The criterion for providing a file rather than
   transpiling it is macros, out-of-family code, platform bindings, and files too
   awkward to transpile correctly — kept infrequent.
4. **Crate scope by phase.** Replaced by scope by target environment. In:
   proto, ankql, signals, core, storage-common, storage-sqlite (engine
   transpiled, the rusqlite binding provided as a driver interface),
   storage-indexeddb-wasm, websocket-client-wasm, connector-local-process, and
   the `ankurah` facade. Out: storage-postgres, websocket-server, the tokio
   websocket-client, sled, derive, tests-wasm, examples. The targets are the
   browser and React Native/Expo; Node is not primary.
5. **`npx tsc --noEmit` at the root tells you how many type errors there are.**
   It did not: two parse errors in `transpile/golden/proto/data.ts` made
   TypeScript report syntactic diagnostics only and skip the semantic pass over
   the whole tree, so it reported 4 errors while 349 existed. The root tsconfig
   now excludes `transpile`.
6. **A bun test summary counts the tests.** It does not: bun abandons the rest
   of a test file at its first unhandled error, so every count taken while fatal
   leak reports were firing is an undercount. `scripts/test-gate.sh` counts
   errors and type errors per package and fails on them.

## The table

| File | Line | Old claim | Disposition |
|---|---|---|---|
| `port/ownership.md` | whole file | `[Symbol.dispose]()` is the drop glue and the cascade; `RwLock` maps to `Mutex`; `Borrow`/`BorrowMut` are no-ops | retracted — rewritten to the current contract before this pass, during the base runtime work |
| `port/ownership/provided-types.md` | 19, 27 | `[Symbol.dispose]()` is the cascade and walks own properties | retracted — `AkObject.drop()` is the template; `dropOwned` is the cascade |
| `port/ownership/provided-types.md` | 99 | `Drop` forces you to implement `drop()` | retracted — it forces `onDrop()`; overriding `drop()` breaks the order |
| `port/ownership/provided-types.md` | 116-118, 194-197, 211-212 | `using trx = ...`, `using guard = ...` | retracted — `const` plus a `finally` that drops |
| `port/ownership/provided-types.md` | 212, 229 | `RefCell.borrow_mut()` | retracted — the method is `borrowMut()`, per the naming rule |
| `port/ownership/provided-types.md` | 245 | `AsyncMutex.run<T>(fn)` | retracted — never existed; the API is `acquire()` returning a guard |
| `port/ownership/provided-types.md` | 261, 268 | `Borrow`/`BorrowMut` have a no-op `[Symbol.dispose]()` | retracted — they have no `drop()` at all and carry the `nonOwning` marker |
| `port/ownership/provided-types.md` | 274-283 | the `Symbol.dispose` polyfill section, and `using` needing a Babel plugin | re-justified — the symbol still exists and delegates to `drop()`, but it is not the model and `using` is not coming back |
| `port/ownership/provided-types.md` | whole file | no `Result`, `RwLock`, `Guard`/`ReadGuard`/`WriteGuard`, `ThreadLocal`, fatal taxonomy or leak registry | re-justified — rewritten against `packages/base/src`, which is the API it claims to document |
| `port/port-runbook.md` | 26 | the port targets Expo Go, and is pure TS because Expo Go cannot use WASM | re-justified — the targets are the browser and React Native/Expo; the wasm-named crates are in scope precisely because they are ankurah's browser code |
| `port/port-runbook.md` | 59, 120 | `npx tsc --noEmit` type-checks the repository | retracted — premise 5 above; the gate script and the tsconfig exclusion replace it |
| `port/port-runbook.md` | 60 | `bun test` runs all tests | retracted — premise 6 above |
| `port/port-runbook.md` | 62 | eslint checks "Disposable, using, alive checks" | retracted — it checks drop, registration and guard escape |
| `port/port-runbook.md` | 75-94 | "How to Port a Rust File": read the Rust, write the TS by hand, one file per agent | retracted — the transpiler writes the TypeScript; hand edits to output are overwritten and hide the defect |
| `port/port-runbook.md` | 84 | `impl Drop for T` → override `drop()` | retracted — `onDrop()` |
| `port/port-runbook.md` | 86 | `RwLock<T>` → `Mutex<T>` | retracted — `RwLock` is its own type |
| `port/port-runbook.md` | 95-111 | "Agent Rules of Engagement": one file at a time, await confirmation, no tests | retracted — written for the hand-porting regime |
| `port/port-runbook.md` | 127-129 | 12 bincode fixtures, 24 TS tests, 244 assertions; 6 yrs fixtures | retracted — 24 bincode fixtures with sidecars, 12 yrs documents, plus LWW, parser and planner sets |
| `port/port-runbook.md` | 140 | derive replaced by `defineModel()` "(E12)" | retracted — E12 is the file-with-submodules rule; the derive rule is E1 |
| `port/port-runbook.md` | 141 | `connectors/websocket-client-wasm` excluded, "TS uses pure websocket-client" | retracted — backwards; the wasm client is the browser client and the tokio one is out |
| `port/port-runbook.md` | 135-144 | exclusion table omits postgres and sled from the reasons that now apply, and lists no in-scope table | re-justified — replaced by a crate scope table with both halves |
| `port/port-runbook.md` | 148-165 | package table with source-file counts, test counts and parity verdicts | retracted — the counts were of the hand port; the gate and the diagnostic budget are what report state now |
| `port/port-runbook.md` | 169 | "854 tests passing, 0 failures, tsc clean" | retracted — measured with the two lying commands above |
| `port/port-runbook.md` | 183 | 45 eslint errors from assert-not-disposed, dispose-owned-fields, require-using-for-guards | retracted — two of those three rules are retired; the rule names changed |
| `port/port-runbook.md` | 199 | "Wire `using` declarations where needed" | retracted — premise 1 |
| `port/RESUME.md` | whole file | the March transpiler architecture, the `Ref` registry bug as the next thing to fix, per-crate progress | retracted — rewritten as the current status |
| `port/decisions.md` | 3 | "Last updated: 2026-02-10" with no later corrections | re-justified — each correction now dated inline |
| `port/decisions.md` | 28 | error handling maps `Result` to a throw | retracted — premise 2 |
| `port/decisions.md` | 37-42 | Phase 1 scope; IndexedDB and WASM out of scope | retracted — premise 4 |
| `port/decisions.md` | 41 | PN Counter backend descoped | re-justified — `pn_counter.rs` is dead in Rust, so no rule is needed |
| `port/decisions.md` | 85 | two SQLite packages, `storage-expo-sqlite` and `storage-better-sqlite3`, each implementing the engine | re-justified — one transpiled engine, one thin driver package per environment, renamed `storage-sqlite-expo` and `storage-sqlite-node`. The rename has not happened |
| `port/decisions.md` | 95 | Transaction cleanup: `dispose()`; `Symbol.dispose` unverified on Hermes | retracted — `.drop()`, and the Hermes question is answered |
| `port/decisions.md` | 97 | WeakRef with a manual fallback if unavailable | re-justified — WeakRef is present; `FinalizationRegistry` is the one that needs the feature detect, and Hermes shipped it in `260318099.0.0` (2026-06-05) |
| `port/decisions.md` | 128-129 | the two SQLite packages as platform splits | re-justified — driver-only, and to be renamed |
| `port/decisions.md` | 133 | de-scoped crates list includes indexeddb-wasm and websocket-client-wasm | retracted — premise 4 |
| `port/decisions.md` | 140 | the audit script is `scripts/audit-port.ts` | retracted — it is `port/audit-port.ts` |
| `port/decisions.md` | 160 | the `using` escape hatch gotcha | re-justified — the escape is real, the mechanism is `const` plus `finally`, and the runtime and the lint rule both catch it |
| `port/translation-rules.md` | 66 | `impl Drop for T` → override `drop()`, call `super.drop()` | retracted — `onDrop()` |
| `port/translation-rules.md` | 72 | `Result<T, E>` → throw a typed Error subclass | retracted — premise 2 |
| `port/translation-rules.md` | 77-78 | `Borrow`/`BorrowMut` have a no-op dispose | retracted — no `drop()` at all |
| `port/translation-rules.md` | 79 | `using guard = mutex.lock()` | retracted — premise 1 |
| `port/translation-rules.md` | 80 | `RwLock<T>` → `Mutex<T>` | retracted — its own type |
| `port/translation-rules.md` | 82 | `RefCell.borrow_mut()` | retracted — `borrowMut()` |
| `port/translation-rules.md` | 154 | `#[derive(Model)]` → hand-written wrappers in Phase 1 | retracted — `defineModel()`, exception E1 |
| `port/translation-rules.md` | 157-163 (A8) | error handling by throw, `?` as try/catch | retracted — premise 2, and `?` must drop the `Ok` wrapper it does not return |
| `port/translation-rules.md` | 165-169 (A9) | tokio has no direct TS shape; channels become "event emitters or custom patterns" | re-justified — spawn is a Promise, `select!` is `Promise.race`, `Notify`/`oneshot` are a promise and its resolver, `mpsc` is an async queue. The one real difference is that `select!` cancels the losers |
| `port/translation-rules.md` | 197-213 (B1) | crate table by phase, indexeddb and websocket-client-wasm out of scope | retracted — premise 4 |
| `port/translation-rules.md` | 219 | `@ankurah/react-native` exists | retracted — the package is `@ankurah/react` |
| `port/translation-rules.md` | 261-273 (C3) | file counts as of 2026-02-10 | historical — marked as a dated count |
| `port/translation-rules.md` | 331-335 (E9) | wasm-gated modules are skipped because "the TS port IS the native JS target" | re-justified — true of the `JsValue` bridge modules, not of the browser crates, and the transpiler's `cfg` evaluation is moving to ankurah's wasm32 configuration |
| `port/translation-rules.md` | 343-347 (E11) | the cascade runs through `[Symbol.dispose]()`; `using` triggers cleanup at block exit | retracted — premise 1 |
| `port/translation-rules.md` | 355-359 (E13) | Rust macros become regular functions or inlined code | re-justified — targeted handling per family, plus the current state: every `Display` impl and all logging are missing from the output today |
| `port/translation-rules.md` | 552-571, 590 (G6) | the hash manifest is `scripts/rust-source-hashes.json` and the audit is `scripts/audit-port.ts` | retracted — `port/.rust-source-hashes.json` and `port/audit-port.ts` |
| `port/transpiler-spec.md` | 31 | decisions.md governs "error handling as throw" | retracted — premise 2 |
| `port/transpiler-spec.md` | 93 | `#[cfg(feature = "wasm")]` → skip entirely | re-justified — the bridge modules are skipped; the browser branch is the branch to follow |
| `port/transpiler-spec.md` | 105 | drop analysis could let the transpiler skip `using` for 332 value types | retracted — premise 1, and skipping tracking would lose leak detection. `Copy` types carry no drop glue, which is a rule, not an optimization |
| `port/transpiler-spec.md` | 181 | `impl Drop for T` → `drop()` override stub | retracted — `onDrop()` |
| `port/transpiler-spec.md` | 226 | `Result<T, E>` → `T` (throws on error) | retracted — premise 2 |
| `port/transpiler-spec.md` | 231 | `Mutex<T>` / `RwLock<T>` → `Mutex<T>` | retracted — two types |
| `port/transpiler-spec.md` | 349 | `Expr::Try` → `expr` (throws propagate) | retracted — check the `Result`, return the `Err`, drop the `Ok` wrapper |
| `port/transpiler-spec.md` | 363-365 | the transpiler generates `using` for block-scoped values | retracted — premise 1 |
| `port/transpiler-spec.md` | 396-410 | project structure listing `skeleton.rs` and `attestation.rs` | retracted — neither was ever written; replaced with the areas that exist |
| `port/transpiler-spec.md` | 431-444 | CLI with `transpile --crate`, `transpile --all`, `attest --check` | retracted — the commands are `drop-analysis`, `skeleton`, `batch` |
| `port/transpiler-spec.md` | 455-467 | `[crates]` config mapping postgres, websocket-server and the tokio websocket client | retracted — premise 4. The live `transpile.toml` still has the old mapping; changing it is a transpiler change |
| `port/transpiler-spec.md` | 26 | "the transpiler is NOT a generic Rust→TS tool", with no reason recorded for why no general tool exists | re-justified — the survey finding is now written down |
| `port/transpiler-spec.md` | 315-328 | the hardcode escape hatch, with no criterion for when to use it | re-justified — the criterion is written down, and hardcoded files are read for their declarations |
| `port/punchlist.md` | 1-5, whole table | `DONE` means ported and correct; postgres and IndexedDB dispositions | historical — banner added; the file-to-file mapping still stands |
| `port/punchlist/index.md` | 54 | `connectors/websocket-client-wasm` excluded, "TS uses pure websocket-client" | retracted — banner added; premise 4 |
| `port/compliance-assessment.md` | whole file | findings against `Disposable`, `Symbol.dispose`, `using`, `RefCell.withMut` | historical — a 2026-03-14 snapshot of hand-written code, marked as one |
| `port/ownership-conformance-changes.md` | whole file | a `@ankurah/std` package, `Disposable`, `onDispose()`, `using` call sites | historical — a 2026-03 changelog, marked as one |
| `port/fixture-assessment.md` | whole file | 15 bincode and 7 yrs fixtures, with coverage gaps | historical — the gaps it names were closed; marked as one |
| `port/spec-cleanup-recommendations.md` | 264, 326-328 | nine lint rules named `require-using`, `require-disposable`; `onDispose()` throw semantics to be specified | historical — a 2026-03-14 archive assessment, marked as one |
| `CLAUDE.md` | 20-25 | the four validation commands | retracted — the two lying commands replaced by the gate, with the caveats stated |

## Lint rules

| Rule | Disposition | Why |
|---|---|---|
| `require-using-for-guards` | **retired** — unregistered from the plugin, file kept for a staged deletion | It demanded `using`, which Hermes refuses to run. All nine of its findings asked for code the target runtime rejects. Nothing survives a rewrite: whether the emitter placed a `try/finally` correctly is the emitter's job, and a value nobody dropped is reported by the leak registry |
| `no-await-in-using-guard` | **retired** — same treatment | It only fired on `using` declarations, so it now reports nothing whatever the code does. Its invariant is enforced twice already and neither place is a lint rule: rustc rejects holding a `!Send` `MutexGuard` across an await in the source, and where ankurah does need a lock across an await it uses `tokio::sync::Mutex`, which maps to `AsyncMutex` |
| `dispose-owned-fields` | **retired** — same treatment | It told a `Drop` subclass to drop each owned field by hand in `onDrop()`. `AkObject.drop()` now does exactly that in a `finally`, so doing it by hand drops the field twice and a second drop is fatal — the rule asked for the one thing the runtime refuses. The inverse check is not worth writing, because dropping a field and then nulling it is a legitimate way to hand ownership away early |
| `dispose-requires-registration` → `drop-requires-registration` | **rewritten and renamed** | It recognised only `extends Drop` and a `DropGuard` field, so it flagged `AkObject` and `Arc`, which register by calling `leakRegistry.register()` themselves — they are the bottom of the hierarchy and have nothing to inherit from. It now accepts all three ways of registering, including every base that reaches `AkObject`'s constructor. The one remaining finding, `ReactorSubInner` in `packages/core/src/reactor/subscription.ts`, is a true positive: a plain class with a `drop()` method and no registration at all |
| `no-guard-escape` | **rewritten** | It required a `using` declaration to fire, so it could never report. It now recognises a guard by the call that produced it — `lock`, `borrow`, `borrowMut`, `read`, `write`, `acquire` — and flags assigning one to a variable declared outside the block that drops it. This is the one `using`-era rule with a live invariant behind it: Rust's borrow checker refuses the assignment outright, and here the escape only becomes visible when the dropped guard is used |
| `assert-not-disposed` → `assert-not-dropped` | **renamed** | The rule id was the last place the reader-facing surface said "dispose" while the runtime said "drop"; its own header comment already called it `assert-not-dropped`. Behaviour unchanged. Whether it should require the assertion on every public method or only on guards is still open |
| `no-type-laundering`, `no-unhandled-fire-and-forget`, `weakref-deref-null-check` | **unchanged** | Checked against the current runtime; none of them mentions `dispose` or `using`, and each still describes something true |

Six rules are registered after this pass, down from nine.

## Known to be stale, not fixed here

- The per-crate punchlists in `port/punchlist/` and the reviews in
  `port/reviews/` still describe the retired `using` and `Symbol.dispose` model.
  They are dated working documents rather than specifications; the index carries
  the scope correction.
- `package.json` points its `audit` script at `scripts/audit-port.ts`, which does
  not exist — the file is `port/audit-port.ts`. That is a code defect and belongs
  with the CI work.
- `packages/eslint-plugin-ankurah/src/rules/weakref-deref-null-check.ts` matches
  any method named `deref()`, and `RwLockReadGuard.deref()` in
  `packages/base/src/std/rwlock.ts` is one. Nothing trips it today; transpiled
  `RwLock` code will.
- `transpile/golden/` is the March capture whose unparseable TypeScript hid every
  type error in the repository. It is superseded by `transpile/goldens/` and
  `transpile/tests/snapshots/`. **It should be removed**, and that removal is a
  staging decision, not something to do in passing.
- The two retired rule files and their tests, and `dispose-owned-fields` with
  its tests, **should be deleted**. They are kept here so the deletion is a
  staged decision that can be read.
- `packages/storage-postgres` and `packages/connector-websocket-server` mirror
  crates that are no longer in scope, and `packages/storage-expo-sqlite` and
  `packages/storage-better-sqlite3` await the rename to `storage-sqlite-expo` and
  `storage-sqlite-node`. **All four should be dealt with**, and that too is a
  staging decision.
