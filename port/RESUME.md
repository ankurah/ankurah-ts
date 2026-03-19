# RESUME PROMPT — paste after /clear or new session

Read these files first:
- /Users/daniel/ak/ankurah-ts/port/port-runbook.md
- /Users/daniel/ak/ankurah-ts/CLAUDE.md
- /Users/daniel/ak/ankurah-ts/port/transpiler-spec.md
- /Users/daniel/ak/ankurah-ts/port/translation-rules.md
- /Users/daniel/ak/ankurah-ts/port/ownership.md

## Where we are

Building the ankurah Rust→TS transpiler at `transpile/`. Phases 1+2 complete. Generating real TS code with function bodies and ownership-based .drop() calls.

### Transpiler status

Working Rust binary at `transpile/` (15 source files, ~3900 lines):
```bash
cd /Users/daniel/ak/ankurah-ts/transpile
cargo run -- drop-analysis <src_dir>                              # transitive Drop ownership
cargo run -- skeleton <file.rs> --crate-path <crate/src/file.rs>  # single file
cargo run -- batch <src_dir> <out_dir> --crate-name <name>        # whole crate
```

Generates 177 TS files / ~16K lines across all 10 crates. Zero unhandled expressions.

**What the transpiler does:**
- Structs/enums/traits/fns with full body translation
- Bincode encode/decode from derive(Serialize, Deserialize)
- Derive method bodies (clone with null safety, equals with null guards)
- Match → .match({}), if-let → .is(), Option → null check, Result → unwrap
- 50+ method call translations, format!/vec!/assert_eq! macros
- Import resolution (cross-crate + intra-crate)
- cfg(test) → .test.ts splitting
- Config-driven provided_impls, hardcoded files, exclusions
- **Ownership-based .drop() insertion** — block-scoped variables not stored/passed/returned get .drop() at scope exit

**Current refactoring in progress:**
- Replace thread_local SELF_TYPE hack with proper TranslateCtx struct
- This will also hold ownership tracking state cleanly

### Hand-port alignment

Tuple struct fields renamed to `_0` in:
- proto: Clock, AuthData, Attestation, AttestationSet, OperationSet, StateBuffers, TransactionId, RequestId, QueryId, UpdateId
- signals: BroadcastId, Broadcast, ValueCell, ReadValueCell

### Discussion items (from yesterday)

1. **Attested generic callback pattern** — Attested<T> encode/decode need callback params. Transpiler doesn't handle generic codec delegation when OTHER types contain Attested<T> fields.

2. **TS-only additions** — Hand-port has methods not in Rust source (get length, Symbol.iterator, entries, empty). Need preservation strategy.

3. **GC disposal in transpiled code** — IMPLEMENTED. Transpiler emits .drop() for block-scoped variables using ownership analysis.

### Port status

994 tests pass, 0 fail, 22 skip, tsc clean. Fatal leak severity set. Entity extends Struct.

### Process rules

Use named team agents, don't shut them down, check mailbox between steps, never skip or stub silently.
