# Memory Model: Lint Rules

**Custom lint rules that enforce Rust-like ownership semantics at the TS level.** These replace compile-time guarantees from Rust's borrow checker, lifetime system, and auto-Drop with static analysis at dev time.

---

## Tier 1 — Must Have

### 1. `assertNotDisposed` at public method entry

**Rust equivalent**: Lifetime enforcement — compiler prevents use-after-free.

**Rule**: Every public method on a `Disposable` subclass (or a class with a `DisposeGuard`) must call `this.assertNotDisposed()` (or `this.#guard.assertNotDisposed()`) as its first statement. Excludes `dispose()`, `[Symbol.dispose]()`, `isDisposed`.

**Why**: Without this, the `using` escape hatch (`let leaked; { using g = ...; leaked = g; }`) produces silent use-after-dispose bugs. This is the primary runtime defense replacing Rust's lifetime enforcement.

### 2. `onDispose` must cascade to all owned Disposable fields

**Rust equivalent**: Auto-Drop cascade through owned fields.

**Rule**: If a class extends `Disposable` and has fields whose types extend `Disposable` (or have a `dispose()` method), its `onDispose()` must call `.dispose()` on each of them.

**Why**: JS has no auto-cascade. Missing a single field silently breaks the vicarious RAII chain (e.g., LiveQuery -> EntityLiveQuery -> ReactorSubscription). Rust does this automatically; we must enforce it.

### 3. `using` required at guard creation sites

**Rust equivalent**: Guard values are always dropped at scope exit.

**Rule**: Methods annotated as returning a Disposable guard (or returning a type extending `Disposable` that has a short intended lifetime) must be called with `using`, not bare `const`/`let`.

**Why**: A bare `const rw = resultset.write()` without `using` means `dispose()` (and its side effects like broadcasts) might never fire. For correctness-critical guards, this is a silent data staleness bug. The FinalizationRegistry crash catches it eventually, but a lint catches it at write time.

---

## Tier 2 — Should Have

### 4. No fire-and-forget async calls that mutate shared state

**Rust equivalent**: `task::spawn` makes async fire-and-forget explicit; `std::sync::Mutex` protects shared state.

**Rule**: Flag un-awaited calls to async methods unless annotated with `// fire-and-forget: <justification>`. Especially flag cases where the un-awaited method mutates state also accessed by other async paths.

**Why**: This is the exact pattern that caused the WatcherSet gap-fill race — the highest-severity async interleaving bug in the codebase.

### 5. Alive flag check at mutation points

**Rust equivalent**: Lifetime parameters (`'trx`) prevent writes outside a valid scope.

**Rule**: Property value setters/mutators on entity types must check `entity.isWritable()` (or equivalent alive flag) before performing the mutation. Transaction methods (`create`, `get`, `edit`) must check `this.alive.value`.

**Why**: Without this, mutations through a stale handle after `commit()` or `rollback()` succeed silently. Rust prevents this at compile time via lifetimes.

### 6. `WeakRef.deref()` result must be null-checked

**Rust equivalent**: `Weak::upgrade()` returns `Option<Arc<T>>`, forcing the caller to handle `None`.

**Rule**: Flag any `.deref()` call on a `WeakRef` whose result is used without a null/undefined check.

**Why**: If the strong reference holder has been GC'd, `deref()` returns `undefined`. Using it without checking is a null-pointer-equivalent bug.

---

## Tier 3 — Nice to Have

### 7. Every class with `dispose()` must have FR registration

**Rule**: Any class that has a `dispose()` method must either extend `Disposable` or use a `DisposeGuard`. Ad-hoc `dispose()` without FR registration means leaked instances produce zero diagnostics.

### 8. No `await` inside `using` blocks with side-effect guards

**Rule**: Flag `await` expressions inside `using` blocks where the guard type is correctness-critical (has side effects on dispose). An `await` means other code can interleave while the guard is active.

### 9. Guard escape detection

**Rule**: Flag assignments of Disposable-typed values to variables declared in an outer scope from inside a `using` block. This is the `using` escape hatch — the bug pattern the user identified as "the real concern."

**Note**: This is the hardest rule to implement reliably but catches the most insidious bug class.

---

## Implementation Notes

These rules can be implemented as custom ESLint rules (via `@typescript-eslint` for type-aware linting). Rules 1-3 and 6-7 are straightforward AST checks. Rules 4-5 require type information. Rules 8-9 require scope analysis.

For the ongoing automatic port, these lints serve as a compile-time approximation of Rust's ownership guarantees — catching at dev time what Rust catches at compile time.
