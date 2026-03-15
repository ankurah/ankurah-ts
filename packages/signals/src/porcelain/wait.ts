// MIRRORS: ankurah/signals/src/porcelain/wait.rs
import type { Signal, GetReadCell } from '../signal/index.ts';

/**
 * Helper trait for `waitFor` to allow flexible predicate return types.
 *
 * Rust: pub trait WaitResult { type Output; fn result(self) -> Option<Self::Output>; }
 *
 * In TS, we can't have blanket impls over primitives, so we use a function
 * that handles both boolean and T | null return types [E8].
 *
 * Semantics:
 * - boolean: true = stop with void, false = continue waiting
 * - T | null: non-null = stop with T, null = continue waiting
 */
export type WaitResultValue<R> = R extends boolean ? void : NonNullable<R>;

/** Convert a predicate result to a resolved value or null (continue waiting) */
function waitResult<R>(value: R): WaitResultValue<R> | null {
  if (typeof value === 'boolean') {
    return (value ? (undefined as any) : null);
  }
  // Option<T> pattern: null/undefined = continue, otherwise stop with value
  return (value != null ? value : null) as WaitResultValue<R> | null;
}

/**
 * Trait for waiting on signal values asynchronously.
 *
 * Rust: pub trait Wait<T: 'static>
 *
 * In TS, implemented as an interface. Since TS can't do blanket impls,
 * standalone helper functions are also provided below for use with any
 * Signal & GetReadCell<T> [E8].
 */
export interface Wait<T> {
  /** Wait for the signal to match a specific value */
  waitValue(targetValue: T): Promise<void>;

  /** Wait for the signal to reach a value matching the given predicate */
  waitFor<R>(predicate: (value: T) => R): Promise<WaitResultValue<R>>;
}

// ============================================================================
// Standalone implementations — mirror the Rust blanket impl
// Rust: #[cfg(feature = "tokio")] impl<T, S> Wait<T> for S where S: Signal + GetReadCell<T>
// Divergence: Rust uses tokio channels to bridge sync→async; TS uses
// Promise + listener callback since JS is single-threaded [E18].
// ============================================================================

/**
 * Wait for a signal to match a specific value.
 * Mirrors Rust's Wait::wait_value blanket impl.
 */
export function waitValue<T>(signal: Signal & GetReadCell<T>, targetValue: T): Promise<void> {
  // Check if current value already matches
  if (signal.getReadCell().with((v) => v === targetValue)) {
    return Promise.resolve();
  }

  return new Promise<void>((resolve) => {
    // Subscribe to change notifications
    const guard = signal.listen(() => {
      if (signal.getReadCell().with((v) => v === targetValue)) {
        guard.drop();
        resolve();
      }
    });
  });
}

/**
 * Wait for a signal to reach a value matching the given predicate.
 * Mirrors Rust's Wait::wait_for blanket impl.
 *
 * Predicate can return:
 * - boolean: true = stop with void, false = continue
 * - T | null: non-null = stop with value, null = continue
 */
export function waitFor<T, R>(
  signal: Signal & GetReadCell<T>,
  predicate: (value: T) => R,
): Promise<WaitResultValue<R>> {
  // Check current value first
  const immediate = signal.getReadCell().with((value) => waitResult(predicate(value)));
  if (immediate !== null) {
    return Promise.resolve(immediate);
  }

  return new Promise<WaitResultValue<R>>((resolve) => {
    // Subscribe to change notifications
    const guard = signal.listen(() => {
      const result = signal.getReadCell().with((value) => waitResult(predicate(value)));
      if (result !== null) {
        guard.drop();
        resolve(result);
      }
    });
  });
}
