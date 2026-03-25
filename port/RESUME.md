# RESUME PROMPT — paste after /clear or new session

Read these files first:
- /Users/daniel/ak/ankurah-ts/port/port-runbook.md
- /Users/daniel/ak/ankurah-ts/CLAUDE.md
- /Users/daniel/ak/ankurah-ts/port/transpiler-spec.md
- /Users/daniel/ak/ankurah-ts/port/translation-rules.md
- /Users/daniel/ak/ankurah-ts/port/ownership.md

## Where we are

Transpiler at `transpile/` — signals crate being ported. All transpiled tests should pass.

### Architecture (recently refactored)

- **TypeContext** (`type_context.rs`, 284 lines) — comprehensive expression type resolution using ScopeStack + TypeRegistry. Single point of truth for all type decisions.
- **BodyTranslator** (`body.rs`, 1012 lines) — expression/statement translation. NO direct registry access — all type queries go through TypeContext.
- **control_flow.rs** — if/else/if-let translation. Now properly threaded with &BodyTranslator (type context preserved through branches).
- **native_types/arc.rs** — Arc/Weak method translations in dedicated module.
- Inline modules → separate files (context.rs + mod stack → context.ts + context/stack.ts).

### Immediate next bug

**`Ref` system type not found in registry** — `deref_accessor(Ref)` returns `None` even though `transpile.toml` has `[system_types.Ref]` with `deref_field = "value"`. `RefMut` works correctly. Debug shows `Ref { args: [...] }.last own=false deref=None`.

Likely cause: `Ref` may be conflicting with a TS/JS built-in name, or the config loader may be filtering it. Check `resolve.rs build_registry()` and the `config.rs` SystemTypeConfig parsing. Fixing this will make `stack.borrow().last()` → `stack.borrow().value.last()` and unblock the signals tests.

### Signals test status

- 0 tsc errors
- 18 transpiled files (17 source + 1 inline module)
- Runtime blocked by RefCell borrow state — `track()` calls `stack.borrow().last()` without releasing the Ref guard, then other functions try `borrowMut()` and fail
- Fix path: once `Ref` deref works, `borrow().value.last()` returns the value directly (Ref guard becomes a temporary that doesn't leak borrow state)... actually no, the Ref guard still leaks. The deeper issue is that temporary borrow guards in expression position aren't dropped.

### Transpiler architecture issues still open

1. **body.rs still 1012 lines** — expr() match is 300+ lines, translate_call() is 100+ lines. Could extract into focused modules.
2. **match_expr.rs** still uses free functions (no type context). Same issue as control_flow.rs had.
3. **Closure param type resolution** only handles ThreadLocal.with — should be extensible.
4. **String-matching** in body.rs closure body wrapping check (starts_with "if " etc.)

### Port status

- Proto: COMPLETE (0 tsc, 33 tests, 0 leaks)
- ankql: IN PROGRESS (0 tsc, 51 tests)
- Signals: IN PROGRESS (0 tsc, runtime blocked by RefCell/Ref deref)

### Process rules

Use named team agents, don't shut them down, check mailbox between steps, never skip or stub silently.
