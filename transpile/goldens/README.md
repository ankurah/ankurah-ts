# Idiom goldens

Each directory here pins one Rust idiom to the TypeScript the transpiler must
produce for it. `input.rs` is transpiled as a one-file crate; `expected.ts` is
the whole of the output, compared line for line after trailing whitespace is
squared up. The runner is `transpile/tests/idiom_goldens.rs`:

```
cd transpile && cargo test --test idiom_goldens
```

## A golden changes only by a deliberate, reviewed edit

There is no `UPDATE_GOLDENS=1`. When the transpiler's output for an idiom
changes, the test prints a unified diff and fails; somebody reads the new output,
decides whether it is better Rust-faithful TypeScript than what is written down,
and edits `expected.ts` by hand. That editing step is the whole point — it is
where a fix gets told apart from a regression. The corpus snapshots under
`transpile/tests/snapshots/` are the opposite kind of artefact: bulk recordings,
refreshable in one command.

## `run.test.ts` runs the emitted golden

`expected.ts` pins the text. A `run.test.ts` beside it makes something execute
that text, so "the emitted code leaks nothing and provokes no fatal" is a checked
property rather than a claim about output nobody ever ran. The runner is
`transpile/tests/golden_run.rs`:

```
cd transpile && cargo test --test golden_run
```

For each golden that has a driver it builds a scratch package: `batch` writes
`input.ts` into a temp directory, the driver and the shared leak check
(`goldens/_driver/leaks.ts`) are copied in beside it, `@ankurah/base` is linked
into `node_modules`, and a `bunfig.toml` preloads the package's `src/testing.ts`
so the ownership test hooks are on. Then `bun test` runs there. A golden passes
only if bun exits zero *and* nothing in its output reads like an ownership
report. A failure prints bun's output and the emitted `input.ts`, because the
temp directory is gone by the time anybody reads the message.

The base package comes from `ANKURAH_BASE_PATH`, or from
`.claude/worktrees/async-layer/packages/base` above the checkout. If it is not
there the run fails and says so; it never skips. Nor does the run go quiet when
every driver disappears: no driver at all is a failure too.

A golden opts in by having a driver, and one whose point is the shape of the text
alone — `struct_bincode` and its kind — never gets one. A driver owes three
things: call every function the golden emitted, drop everything the driver itself
constructs so the only leak that can surface is the transpiler's, and end with
`expectNoOwnershipReports()`.

That last check is one-sided by construction, and worth reading as such. A leak is
found by a `FinalizationRegistry` callback, and a garbage collector is never
obliged to collect anything: a report that fires proves a value was collected
without being dropped, while silence proves only that nothing was collected. The
check forces a collection and gives the loop a turn to deliver what it found,
which is what makes the silence worth something — not what makes it certain.

## These five seeds are captured output, not yet vetted

They were produced by running `batch` at commit f602831 and saving what came out.
Daniel has not read them line by line yet, so treat them as "what the transpiler
does today", not as "what is right". Until he vets them, a diff against one of
these files means *something moved*, not *something broke*.

| Golden | Idiom |
| --- | --- |
| `struct_bincode` | named-field struct and a byte newtype, with the derived `encode`/`decode` pair |
| `enum_payload` | enum with a unit, a tuple, and a named-field variant, plus its variant-tagged codec |
| `option_result_fields` | `Option<T>` fields and a method returning `Result<T, E>` |
| `question_mark` | `?` on a call whose error type already matches the function's |
| `arc_mutex_field` | a field read through `Arc<Inner>` and then a `Mutex` guard |

What I already doubt in the seeds, for whoever vets them:

- `arc_mutex_field/expected.ts` reads `this._0.value.label.lock().value.length`
  and never drops the guard. Rust drops the temporary at the end of the
  statement; the emitted TypeScript holds it forever. Compare with the guard in
  `tests/snapshots/signals/broadcast.ts`, where a guard bound to a `let` does get
  a `.drop()` through an IIFE. If the temporary-guard case gets fixed, this
  golden changes.
- `option_result_fields/expected.ts` builds an error value as `SlotError.Missing`
  while every other construction of that enum in the file goes through
  `new SlotError('Missing', {})`. One of the two is wrong.
- `struct_bincode/expected.ts` types `u64` as `bigint | number`, so `writeU64`
  accepts either. Whether that union is the intended `u64` mapping is a wire
  protocol question, not a transpiler question.

Two shapes I tried and deliberately kept out of the seeds, because their output
is plainly wrong and a golden should not enshrine it: a method whose name equals
a field's name (`pub fn label(&self) -> &String` next to a `label` field) emits a
TypeScript class with a method and a property of the same name, and
`Mutex<Vec<u64>>` emits `Mutex<bigint | number[]>`, which parses as a `Mutex` of
an array of `number` unioned with `bigint`. Both belong in the engine's own unit
tests as failing cases, not here.

## Adding a golden

Make a directory with an `input.rs`, run the test once, and it prints the output
with a note telling you to save it as `expected.ts` if it is right. Keep inputs
small enough to read in one screen and close to idioms the corpus actually uses.
