# Status — 2026-09-02

Read these first:
- `/Users/daniel/ak/ankurah-ts/port/port-runbook.md` — scope, workflow, validation commands
- `/Users/daniel/ak/ankurah-ts/CLAUDE.md`
- `/Users/daniel/ak/ankurah-ts/port/ownership.md` — the ownership contract the runtime and the emitter share
- `/Users/daniel/ak/ankurah-ts/transpile/SYMBOL-TABLE-SPEC.md` — the type resolution engine
- `/Users/daniel/ak/ankurah-ts/port/retractions-2026-09-02.md` — what the older docs used to say and why it changed

The port is produced by the transpiler in `transpile/`, not by hand. The
hand-written TypeScript that predates it is still in `packages/` and still runs,
but it is output to be replaced, not a source to edit. The two things that are
genuinely hand-written are the ownership runtime in `packages/base/src` and the
provided files that stand in for code the transpiler will not translate.

**The runtime is finished and waiting to be read.** `packages/base` implements
Rust's drop semantics: `AkObject.drop()` marks the value, leaves the leak
registry, runs the type's own `onDrop()` while its fields are still alive, then
drops what it owns; containers drop their contents; a second drop, a use after
drop, a use after move, and dropping a container while a guard on it is
outstanding are all fatal, because Rust would not have compiled any of them.
`port/ownership.md` is the contract in full, and it is what the emitter has to
emit against. It is frozen for line-by-line staging in
`.claude/worktrees/agent-a23c104c7bbcb90b8`, with 149 tests passing, a clean
strict typecheck, and a clean lint run.

**The transpiler's type resolution engine is frozen at the end of its first
step.** It answers what Rust type an expression has — the question almost every
translation decision turns on — from a real module tree with per-module scopes,
and files a diagnostic naming the Rust location rather than guessing when it
cannot. Two independent reviews (Codex CLI and Opus) ran against it, every
finding was accepted, and the fix pass and close-out landed: 60 tests, proto and
ankql output byte-identical to the previous baseline, one deliberate change in
signals (`context/stack.ts`, which is the bug this step existed to fix), and the
core crate running to completion for the first time. Diagnostics are the
coverage metric and are pinned by a budget test: proto 87, ankql 8, signals 258,
core 2894. The frozen copy is `.claude/worktrees/engine-step1-final`.

**In flight**: the engine's second step, in `.claude/worktrees/engine` — an impl
table with real trait identity, unification, Rust method resolution through
deref chains, `Self` in trait default bodies, and pattern binding, which is the
largest remaining diagnostic class. Alongside it: Rust stub declarations for the
std and third-party surface the corpus touches (`.claude/worktrees/std-surface`)
and this doc and lint retraction (`.claude/worktrees/docs`).

**Fixtures** live on the `ts-port-support` branch in the sibling Rust checkout:
24 proto bincode fixtures with per-item offset and length sidecars, 12 yrs V2
documents, 10 core LWW cases, 91 ankql parse cases, and 26 planner cases
covering 48 plans. They pin the traps that would otherwise be found by a user —
a bare `u8` where the type says otherwise, `[u8;16]` against `Vec<u8>`, u64
values above 2^53, and yrs text indices being UTF-8 byte offsets rather than
UTF-16 units. TypeScript consumers for all of them now exist
(`.claude/worktrees/fixtures-ts`), and what they report is the port's honest
state: the Yrs interop passes in full, proto fails 83 of 249, ankql fails 108 of
303, and the core backend and planner suites cannot even load.

**Known defects, all awaiting the emission and macro steps, none to be fixed by
hand.** The transpiled packages throw where Rust returns a `Result`; a Rust
function returning `Result` must return a `Result` value. Drops are emitted
outside `try/finally`, a moved local is dropped twice, and a statement-position
`?` abandons the `Ok` value instead of dropping it. Transpiled tests never drop
what they allocate, which is why five suites in one run produce thousands of
leak reports. `i64` and `u64` reach TypeScript as `number`, so values above 2^53
round. The ankql AST has no bincode codec at all, because its file is on the
transpiler's hardcode list. The provided ankql parser accepts eleven queries the
Rust parser rejects. Ordered types get a `compareTo` that throws at runtime.
`#[error]`, the tracing macros and the `action_*!` family are emitted as
comments, so every `Display` impl and all logging are currently silently absent.

Working notes, reviews, staging instructions and commit messages live in
`~/.claude/handoffs/ankurah-ts/main/`.
