# Ownership Conformance Changes

Summary of changes to bring existing TS code into conformance with the finalized ownership spec (`port/ownership.md`, `port/ownership/provided-types.md`).

## Priority 1: New `@ankurah/std` package + updated provided types

Created `packages/std/` as a standalone workspace package (`@ankurah/std`) containing Rust std-equivalent types, organized to mirror the Rust module structure:

| File | Rust module | Types |
|------|-------------|-------|
| `src/std/dispose.ts` | `std::ops::Drop` | `Disposable`, `DisposeGuard`, `disposeSymbol`, `leakRegistry` |
| `src/std/cell.ts` | `std::cell` | `RefCell`, `Ref`, `RefMut` |
| `src/std/sync.ts` | `std::sync` | `Mutex`, `MutexGuard` |

Key changes to existing types:
- **Disposable**: Added `severity` parameter (`'fatal' | 'warning'`). Fatal uses `queueMicrotask(() => { throw })`, warning uses `console.error`.
- **RefCell**: Replaced `withMut`/`withRef` closure pattern with `borrow()` / `borrow_mut()` returning `Ref<T>` / `RefMut<T>` Disposable guards (used with `using`).
- **Mutex + MutexGuard**: New. `Mutex.lock()` returns a `MutexGuard<T>` Disposable guard with `.value` get/set.

Both `@ankurah/core` and `@ankurah/signals` now depend on `@ankurah/std`. The old `core/src/disposable.ts` is a thin re-export shim.

## Priority 2: Wire existing types to Disposable

| Type | Package | Change |
|------|---------|--------|
| `ReactorSubscription` | core | `extends Disposable`, `onDispose()` replaces manual `dispose()` + `[Symbol.dispose]()` |
| `EntityLiveQuery` | core | `extends Disposable`, `onDispose()` replaces manual `dispose()` + `[Symbol.dispose]()`, removed standalone `FinalizationRegistry` |
| `LiveQuery<V>` | core | `extends Disposable`, `onDispose()` replaces manual `dispose()` + `[Symbol.dispose]()` |
| `SubscriptionGuard` | signals | `extends Disposable`, `onDispose()` replaces manual `dispose()` |
| `ListenerGuard` (broadcast) | signals | `extends Disposable`, `onDispose()` replaces manual `dispose()` |
| `ListenerGuard` (signal) | signals | `extends Disposable`, `onDispose()` replaces manual `dispose()` |
| `Transaction` | core | Added `[Symbol.dispose]()` for auto-rollback on scope exit |

## Priority 3: Alive checks

- `Transaction.create()`, `Transaction.get()`, `Transaction.edit()` now check `this.alive.value` and throw `MutationError('General', 'Transaction has been consumed')` if false.
- Property value setters (`LWW.set`, `YrsString.insert/delete/overwrite/replace`) already had `entity.isWritable()` checks — no changes needed.

## Priority 4: ResultSetWrite migration

- `ResultSetWrite` now `extends Disposable`. `onDispose()` fires the broadcast (previously `done()`).
- `done()` kept as a deprecated compatibility alias that calls `dispose()`.
- All call sites in `subscription_state.ts` converted to use `using` blocks:
  - `updateQuery()`: wrapped in `{ using rwResultset = ... }` block
  - `evaluateChanges()`: inline `{ using rw = ...; rw.add(entity); }`
  - `processGapFillEntities()`: wrapped in `{ using rw = ... }` block
  - `processGapFill()`: wrapped in `{ using rw = ... }` block

## Validation

- `npx tsc --noEmit` passes (zero errors)
- `bun test` passes (440 tests, 1016 assertions, zero failures)
