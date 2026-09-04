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
