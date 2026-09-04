# CLAUDE.md

## Project

ankurah-ts is an ongoing, iterative TypeScript port of [ankurah](https://github.com/ankurah/ankurah), a Rust CRDT-backed reactive database. The port targets React Native / Expo Go.

## Start here

Read `/port/port-runbook.md` first. It contains orientation, workflows, current status, and outstanding tasks.

## Key rules

- **Port fidelity**: TS code should read as close to the Rust source as possible. Zero freestyling.
- **This is NOT a one-time port**: The Rust implementation continues to evolve. Port specs, translation rationale, and fixture infrastructure retain value indefinitely. Don't delete things just because the code is implemented.
- **Wire protocol**: Match Rust bincode encoding exactly. No reimagining. No JSON alternative.
- **Ownership**: Rust ownership patterns (Drop, Mutex, RwLock, RefCell, Arc) map to provided TS types 1:1, and the runtime enforces them while the program runs. The mechanism is `.drop()`, not `using`. See `port/ownership.md`.
- **The port is transpiled**: `transpile/` produces `packages/*/src`. Fix the transpiler, not its output. The exceptions are `packages/base/src` and `.provided.ts` files, which are written by hand.

## Validation

```bash
scripts/test-gate.sh                 # tests + typecheck per package; fails on failures, errors and type errors
npx eslint packages/                 # ownership compliance
bun run port/audit-port.ts           # structural compliance
cd transpile && cargo test           # the transpiler's own tests
```

Use the gate rather than a bare `bun test`: bun abandons the rest of a test file
at its first unhandled error and reports it separately, so "0 fail" can mean
"stopped reading". A root `npx tsc --noEmit` that reports only parse errors
inside `transpile/` is the masking bug fixed by excluding `transpile` from the
root tsconfig — the number it gives you is not the number of type errors.

## Agent behavior

- **Preserve team agents after they go idle**. Do not shut them down. Their accumulated context is valuable for follow-up questions and can be reused. Only shut down when explicitly asked.
- **Delegate implementation work to background agents**. The supervisor agent coordinates and reviews — it does not write implementation code directly.
- **Memory model spec is a rulebook**, not a code remediation plan. Type-specific adjudications belong as annotations in the source code, not in the spec.
