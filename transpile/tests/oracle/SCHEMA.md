# rust-analyzer oracle — schema

What this is for: when the engine starts resolving method calls itself, we need
an answer key that was produced by something that already knows the answers.
On 2026-09-02 a throwaway spike loaded the ankurah workspace with
`ra_ap_*` 0.0.349 and wrote down what rust-analyzer inferred at a sample of
sites. These files are that text converted to JSON.

The oracle is **test data, never a build dependency**. The transpiler does not
link rust-analyzer, does not shell out to it, and does not read these files.
Only `transpile/tests/oracle.rs` reads them.

## Files

Every file is an object with two keys: `source`, naming the spike output it came
from, and `sites`, an array of records. `crate_counts.json` is the exception; it
has `source` and `crates`.

| File | Records | From | One record says |
| --- | --- | --- | --- |
| `method_calls.json` | 14 | `spike-run3.txt` | receiver type before and after adjustment, the resolved callee, and the call's result type |
| `adjustment_chains.json` | 6 | `spike-run3.txt` | the ordered deref/borrow/unsize steps inserted at one site |
| `closure_types.json` | 6 | `spike-run3.txt` | a closure's type and inferred return |
| `try_sites.json` | 0 | `spike-run3.txt` | a `?` and its conversion — empty, see the gap below |
| `trait_generic_calls.json` | 76 | `inventory4.txt` | a call that resolved to a trait's own declaration, not to an impl |
| `overloaded_derefs.json` | 27 | `inventory4.txt` | a receiver that went through a `Deref` impl before the call |
| `closure_returns.json` | 52 | `inventory4.txt` | a closure's inferred return type |
| `try_conversions.json` | 57 | `inventory4.txt` | a `?` whose error type changed, and the function that changes it |
| `untyped_expressions.json` | 188 | `inventory4.txt` | an expression rust-analyzer could not type |
| `crate_counts.json` | 4 crates | `inventory4.txt` | per-crate totals for every construct the spike counted |

The two sources differ in granularity. `spike-run3.txt` walked two files with the
proc-macro server on and recorded full detail including a column; `inventory4.txt`
walked all four corpus crates and recorded one line per interesting site, keyed by
file and line only. Both are kept: the first is precise, the second is broad.

## Fields

Locations. `file` is relative to the support checkout root, e.g.
`proto/src/clock.rs`. `line` is 1-based. `col` is 1-based and present only on
records from `spike-run3.txt`.

Expressions. `expr` is the source text of the site, and `truncated` is `true`
when the spike cut it off for display — those strings are for a human reading a
failure, never for matching.

Types. Every type is rust-analyzer's own rendering, lifetimes, `Global`
allocator parameter and all: `Vec<EventId, Global>`, `&mut DebugStruct<'_, '_>`,
`<D as Deserializer<'de>>::Error`. A comparison against the engine has to
normalise, or compare structurally, rather than expect string equality.

Per record kind:

- **method call** — `receiver_type` is the receiver's type as written,
  `receiver_type_adjusted` is what it became after auto-deref and auto-ref,
  `callee` is the resolved function as `Type::method` or `Trait::method`,
  `callee_kind` is `inherent impl` or `trait method, defining trait`, and
  `result_type` is the call's type.
- **adjustment chain** — `steps` in application order; each has `adjustment`
  (rust-analyzer's word: `Deref(None)`, `Deref(Some(OverloadedDeref(Shared)))`,
  `Borrow(Ref(Shared))`, `Borrow(Ref(Mut))`, `Pointer(Unsize)`), `from`, and `to`.
- **overloaded deref** — `steps` with `from` and `to` only; the coarse form of the
  same fact from the whole-corpus pass.
- **closure type / closure return** — `closure_type` is the `impl Fn(..)` shape;
  `inferred_return` and `return_type` are the return.
- **try conversion** — `from_error` is the error the operand carries, `to_error`
  is the error the enclosing function returns, `conversion` is the function
  rust-analyzer resolved for the step (usually `<Result<T, E> as Try>::branch`).
- **crate counts** — a flat map of count name to number, per crate. The names are
  renamed from the spike's display labels, e.g. `typed by RA` →
  `expressions_typed`, `>=2 deref steps` → `receiver_deref_steps_ge2`. The
  mapping lives in `convert_spike_txt.py`.

## The known gap

The spike typed 88% of the corpus's expressions. Items under `#[async_trait]`,
`wasm_bindgen`, and `uniffi::export` came back untyped, which is why
`try_sites.json` is empty: the only `?` sites the detailed run visited were in
`core/src/context.rs`, an `#[async_trait]` file. Records where every answer was
`<none>` or `<UNRESOLVED>` were dropped in conversion — 24 of them — rather than
checked in as false answers. Sites the oracle does not cover are covered by the
corpus inventory test instead (`transpile/tests/corpus_inventory.rs`).

`untyped_expressions.json` is the honest record of that gap: 188 expressions the
spike saw and could not type.

## Regenerating

Deliberately, and rarely. Rebuild the spike out of tree — its source is in the
handoff directory `2026-09-02-1149-ra-spike/`, `cargo +nightly build --release`
(the `ra_ap_*` crates do not build on stable) — run it over the support
checkout, then convert its text output:

```
python3 transpile/tests/oracle/convert_spike_txt.py transpile/tests/oracle [spike-dir]
```

The spike directory defaults to the 2026-09-02 handoff copy; `ANKURAH_SUPPORT_PATH`
names the Rust checkout it ran over, so its absolute paths can be made relative.
Re-running it over the same text reproduces the checked-in JSON byte for byte.

The converter fails loudly on any line or label it does not recognise. After
regenerating, the record counts asserted in `tests/oracle.rs::oracle_loads` will
need updating, which is the intended speed bump.
