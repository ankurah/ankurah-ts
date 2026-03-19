# RESUME PROMPT — paste after /clear or new session

Read these files first:
- /Users/daniel/ak/ankurah-ts/port/port-runbook.md
- /Users/daniel/ak/ankurah-ts/CLAUDE.md
- /Users/daniel/ak/ankurah-ts/port/transpiler-spec.md
- /Users/daniel/ak/ankurah-ts/port/translation-rules.md
- /Users/daniel/ak/ankurah-ts/port/ownership.md

## Where we are

Transpiler at `transpile/` is feature-complete. Next step: overwrite proto hand-port with transpiler output, audit the diff, then gradually expand to other crates.

### Transpiler (transpile/)

15 Rust source files, ~4000 lines. Generates 177 TS files / ~16K lines across all 10 crates with zero unhandled expressions.

```bash
cd /Users/daniel/ak/ankurah-ts/transpile
cargo run -- batch <rust_src_dir> <ts_out_dir> --crate-name <name>
cargo run -- skeleton <file.rs> --crate-path <crate/src/file.rs>
cargo run -- drop-analysis <rust_src_dir>
```

Config: `transpile/transpile.toml` — provided_impls, hardcoded files, excluded files, crate mapping.

**What it does:**
- Full struct/enum/trait/fn extraction and body translation via syn
- Bincode encode/decode from derive(Serialize, Deserialize)
- Derive method bodies (clone with null safety, equals with null guards)
- Match → .match({}), if-let → .is(), Option → null check
- Result<T,E> as Enum — Ok(x)→Result.Ok(x), Err(e)→Result.Err(e), ?→explicit isErr check
- Ownership-based .drop() at end of scope (inside if/else branches via pending_drops threading)
- Import resolution (cross-crate + intra-crate), cfg(test) → .test.ts splitting
- BodyTranslator struct (clean context, no thread_local)
- 50+ method call translations, format!/vec!/assert_eq! macros

**Architecture decisions:**
- "Write Rust in TypeScript" — structural 1:1 correspondence, not idiomatic TS
- Result<T,E> as Enum (not throw). throw reserved for panics only.
- No `using`. Manual .drop() calls for block-scoped ownership.
- All types extend AkObject. AkObject cascade for struct/enum fields.
- Tuple struct fields named `_0` (not semantic names)
- Code length restrictions don't apply to transpiled output

### Immediate next step

1. **Overwrite proto with transpiler output** — run batch into `packages/proto/src/`, audit `git diff`, check for lost implementation details, commit if acceptable
2. **Validate** — run `npx tsc --noEmit` and `bun test packages/proto` on the overwritten code
3. **Expand** — make a list of packages for transpiler management, gradually validate each

### Known transpiler limitations

- **Attested<T> generic callback** — types containing Attested<T> fields get `v.encode(w)` but need `v.encode(w, callback)`. Config-driven fix needed.
- **TS-only methods** — hand-port has methods not in Rust (Symbol.iterator, get length, entries, empty). These get erased on overwrite. Need preservation strategy.
- **Nested block drops** — for-loop bodies don't drop their own locals yet (function-level only)
- **Early return drops** — ? or return mid-function doesn't drop preceding locals

### Hand-port alignment done

Tuple struct fields renamed to `_0` in proto (Clock, AuthData, Attestation, AttestationSet, OperationSet, StateBuffers, TransactionId, RequestId, QueryId, UpdateId) and signals (BroadcastId, Broadcast, ValueCell, ReadValueCell).

### Port status

994 tests pass, 0 fail, 22 skip, tsc clean. Fatal leak severity set. Entity extends Struct.

### Process rules

Use named team agents, don't shut them down, check mailbox between steps, never skip or stub silently.
