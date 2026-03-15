# CLAUDE.md

## Project

ankurah-ts is an ongoing, iterative TypeScript port of [ankurah](https://github.com/ankurah/ankurah), a Rust CRDT-backed reactive database. The port targets React Native / Expo Go.

## Start here

Read `/port/port-runbook.md` first. It contains orientation, workflows, current status, and outstanding tasks.

## Key rules

- **Port fidelity**: TS code should read as close to the Rust source as possible. Zero freestyling.
- **This is NOT a one-time port**: The Rust implementation continues to evolve. Port specs, translation rationale, and fixture infrastructure retain value indefinitely. Don't delete things just because the code is implemented.
- **Wire protocol**: Match Rust bincode encoding exactly. No reimagining. No JSON alternative.
- **Ownership**: Rust ownership patterns (Drop, Mutex, RefCell) map to provided TS types 1:1. See `port/ownership.md`.

## Validation

```bash
npx tsc --noEmit                    # type-check
bun test                             # all tests
bun run port/audit-port.ts           # structural compliance
npx eslint packages/                 # ownership compliance
```

## Agent behavior

- **Preserve team agents after they go idle**. Do not shut them down. Their accumulated context is valuable for follow-up questions and can be reused. Only shut down when explicitly asked.
- **Delegate implementation work to background agents**. The supervisor agent coordinates and reviews — it does not write implementation code directly.
- **Memory model spec is a rulebook**, not a code remediation plan. Type-specific adjudications belong as annotations in the source code, not in the spec.
