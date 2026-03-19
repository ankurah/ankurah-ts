# RESUME PROMPT — paste after /clear or new session

Read these files first:
- /Users/daniel/ak/ankurah-ts/port/port-runbook.md
- /Users/daniel/ak/ankurah-ts/CLAUDE.md
- /Users/daniel/ak/ankurah-ts/port/transpiler-spec.md
- /Users/daniel/ak/ankurah-ts/port/translation-rules.md
- /Users/daniel/ak/ankurah-ts/port/ownership.md

## Where we are

Building the ankurah Rust→TS transpiler at `transpile/`. Generates real TS code with function bodies, ownership-based .drop() calls, and Result<T, E> type.

### Transpiler status

Working Rust binary at `transpile/` (15 source files, ~3900 lines):
```bash
cd /Users/daniel/ak/ankurah-ts/transpile
cargo run -- skeleton <file.rs> --crate-path <crate/src/file.rs>  # single file
cargo run -- batch <src_dir> <out_dir> --crate-name <name>        # whole crate
cargo run -- drop-analysis <src_dir>                              # transitive Drop ownership
```

Generates 177 TS files / ~16K lines across all 10 crates. Zero unhandled expressions.

### Key architecture decisions made

1. **Result<T, E> as Enum** — not throw-based. `Result.Ok(x)` / `Result.Err(e)`. Function signatures preserve `Result<T, E>`. `?` operator → explicit `isErr()` check + early return. `throw` is reserved for panics only.

2. **Ownership-based .drop()** — block-scoped variables that aren't returned get `.drop()` at end of scope. All locals drop (idempotent for moved values). No try/finally — drops go at end of block normally.

3. **BodyTranslator struct** — expression translation uses a struct with `self_type` field, not thread_local. Clean extensible design.

4. **"Write Rust in TypeScript"** — not idiomatic TS. Structural 1:1 correspondence with Rust source is the priority.

### What needs fixing (drop system)

**Must fix:**
1. **Drops inside if/else branches** — when last expression is if/else, drops must go inside each branch before the return. Currently the `_ret = if(...)` pattern generates invalid TS (if is a statement not expression). Fix: thread drops list into branch translation.
2. This is the ONLY blocking issue for drop correctness.

**Should fix later:**
3. Nested block drops (for-loop bodies drop their own locals)
4. Early return drops (`?`/`return` should drop locals declared before that point)

### Hand-port alignment done

Tuple struct fields renamed to `_0` in proto (Clock, AuthData, Attestation, AttestationSet, OperationSet, StateBuffers, TransactionId, RequestId, QueryId, UpdateId) and signals (BroadcastId, Broadcast, ValueCell, ReadValueCell). Method ordering aligned. Clone/equals derive bodies generated.

### Transpiler bugs fixed

- Bincode closure param capture (w/r inside callbacks, not outer writer/reader)
- Null-safe equals (null guards for nullable fields)
- Null-safe clone (?.clone() ?? null)
- Generic type stripping in decode calls
- Self resolution via BodyTranslator struct

### Discussion items

1. **Attested generic callback pattern** — Attested<T> encode/decode need callback params for generic payload. Types containing Attested<T> fields need special bincode handling.
2. **TS-only additions** — Hand-port has methods not in Rust (get length, Symbol.iterator, entries, empty). Need preservation/annotation strategy.
3. **When to start overwriting hand-port with transpiler output** — may differ per crate. Proto is closest. Requires careful diff auditing to preserve hand-port insights.

### Port status

994 tests pass, 0 fail, 22 skip, tsc clean. Fatal leak severity set. Entity extends Struct.

### Process rules

Use named team agents, don't shut them down, check mailbox between steps, never skip or stub silently. Code length restrictions don't apply to transpiled output.

### Source files (transpile/src/)

- body.rs (558) — BodyTranslator struct, expression/statement translation
- emit.rs (427) — TS emission (struct/enum/trait/fn)
- extract.rs (416) — syn parsing
- codegen.rs (321) — top-level generation orchestration
- name_map.rs (292) — identifier/type mapping
- drop_analysis.rs (264) — transitive Drop ownership
- bincode_module.rs (248) — bincode encode/decode generation
- match_expr.rs (243) — match expression translation
- macros.rs (196) — macro translation
- config.rs (181) — transpile.toml config parsing
- control_flow.rs (165) — if/if-let/return position
- ownership.rs (159) — ownership tracking, drop generation
- types.rs (110) — data structures
- imports.rs (94) — import resolution
