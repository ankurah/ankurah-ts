# RESUME PROMPT — paste after /clear or new session

Read these files first:
- /Users/daniel/ak/ankurah-ts/port/port-runbook.md
- /Users/daniel/ak/ankurah-ts/CLAUDE.md
- /Users/daniel/ak/ankurah-ts/port/transpiler-spec.md
- /Users/daniel/ak/ankurah-ts/port/translation-rules.md
- /Users/daniel/ak/ankurah-ts/port/ownership.md

## Where we are

Building the ankurah Rust→TS transpiler at `transpile/`. Phase 1 (skeleton generation) is near-complete. Phase 2 (body translation) is next.

### Transpiler status

Working Rust binary at `transpile/` with these commands:
```bash
cd /Users/daniel/ak/ankurah-ts/transpile
cargo run -- drop-analysis ../../ankurah-ts-support/proto/src     # transitive Drop ownership
cargo run -- skeleton <file.rs> --crate-path proto/src/file.rs    # single file skeleton
cargo run -- batch <src_dir> <out_dir> --crate-name proto         # whole crate batch
```

Source: 13 files, 3596 lines total. Generates 177 TS files / 16K lines from the full Rust codebase with zero unhandled expressions. Config reads from `transpile/transpile.toml`. Phases 1+2 complete. Derive method bodies (clone, equals) auto-generated. Constructor bodies auto-generated. Implicit return detection working.

**What Phase 1 handles:**
- Structs → classes extending Struct/Drop with fields, constructors, methods
- Enums → Enum<V> with variant type maps, bincode encode/decode
- Traits → interfaces / abstract classes
- impl blocks merged into classes, trait impls mapped (Display→toString, PartialEq→equals, From→from, etc.)
- From/TryFrom/TryInto disambiguation for multiple impls
- Generic Self resolution (Attested<T> not Attested)
- Derive-generated methods (Default, Clone, PartialEq, PartialOrd)
- Bincode encode/decode from derive(Serialize, Deserialize) field layout
- Import resolution: @ankurah/base, cross-crate (use statements), intra-crate (type→file map)
- cfg(test) splitting → .test.ts files with describe/test/expect
- Feature-gated file exclusion (wasm, uniffi, postgres)

**Validated via git diff against hand-ported code.** Output goes to /tmp/, never overwrites packages/.

### Systematic audit results (10 diff categories)

Audited every structural diff between transpiler output and hand-ported proto crate:
1. Tuple field naming (`_0` vs semantic) — cosmetic, skip
2. Extra clone() from derive — transpiler correct
3. Error types as Error subclass — porting divergence, needs provided_impl
4. Default param values — Phase 2
5. Generic encode/decode (Attested<T>) — needs provided_impl
6. TS-only convenience methods — not in Rust
7. Method ordering — cosmetic
8. EventId location divergence — porting choice
9. From naming (fromAttestedEvent) — config
10. Test file structure — fixed

### Ownership model

All types extend AkObject (Struct/Enum). Fatal leak detection enabled. The `using` vs `const` question for temporaries and function arguments is deferred to the transpiler — it can read Rust function signatures (move vs borrow) from syn AST.

Drop analysis (transpiler command) identified: 14 direct Drop, 105 transitive, 332 value types across the full Rust codebase.

### Transform module architecture

Three patterns for handling different kinds of code:
1. **Default transform** — syntactic 1:1 translation with stubs
2. **Rewrite module** — generates code from derives (bincode encode/decode)
3. **Provided impl** — hand-written TS preserved (custom serde, yrs→yjs compat, error types)

Config in transpile.toml (not yet parsed — hardcoded for now).

### Port status

994 tests pass, 0 fail, 22 skip (21 test.skip + 1 describe.skip), tsc clean. Fatal leak severity set but tests not yet fixed for it (gc-fixer work paused pending transpiler approach). Entity now extends Struct.

### What to do next

1. **Phase 2 body translation** — translate function bodies (syn::Expr → TS expressions)
2. **OR** implement provided_impl config parsing + error type handling
3. **OR** continue fixing GC ownership warnings with the transpiler informing using/const decisions

Process rules: use named team agents, don't shut them down, check mailbox between steps, never skip or stub silently.
