// MIRRORS: ankurah/signals/src/context.rs
import type { Signal } from './signal/index.ts';

/**
 * Observer interface - mirrors Rust Observer trait from observer.rs.
 * Defined here locally to avoid circular dependency with observer/index.ts
 * which is still a stub. When observer is fully ported, this should be
 * imported from there instead.
 */
export interface Observer {
  /** Observe a signal - implement to handle subscriptions */
  observe(signal: Signal): void;
  /** Get a unique identifier for this observer (for equality comparison) */
  observerId(): number;
}

// ============================================================================
// Module-level stack - single-threaded JS equivalent of Rust's thread_local!
// Divergence: Rust has two feature-gated implementations (singlethread vs multithread).
// TS uses only the singlethread variant since JS is single-threaded [E8].
// Divergence: Rust uses Arc<dyn Observer>; TS uses Observer directly since
// JS has no need for Arc around trait objects in a single-threaded context [E8].
// ============================================================================

const OBSERVER_STACK: Observer[] = [];

function track(signal: Signal): void {
  const observer = OBSERVER_STACK.at(-1);
  if (observer != null) {
    observer.observe(signal);
  }
}

function set(observer: Observer): void {
  OBSERVER_STACK.push(observer);
}

function pop(): void {
  OBSERVER_STACK.pop();
}

function remove(observer: Observer): void {
  const targetId = observer.observerId();
  const last = OBSERVER_STACK.at(-1);
  if (last != null && last.observerId() === targetId) {
    OBSERVER_STACK.pop();
    return;
  }
  // Retain only observers whose id does not match
  for (let i = OBSERVER_STACK.length - 1; i >= 0; i--) {
    if (OBSERVER_STACK[i].observerId() === targetId) {
      OBSERVER_STACK.splice(i, 1);
    }
  }
}

function current(): Observer | null {
  return OBSERVER_STACK.at(-1) ?? null;
}

/**
 * Manages the current observer stack
 * and provides a way to subscribe the current observer to a given signal.
 *
 * In Rust this is `pub struct CurrentObserver {}` with static-like associated functions.
 * In TS this is a class with static methods (never instantiated).
 */
export class CurrentObserver {
  /** Subscribes the current context to a signal */
  static track(signal: Signal): void {
    track(signal);
  }

  /** Sets an observer as the current context, pushing it onto the stack */
  static set(observer: Observer): void {
    set(observer);
  }

  /** Removes the current observer from the stack, restoring the previous one */
  static pop(): void {
    pop();
  }

  /** Removes a specific observer from the stack */
  static remove(observer: Observer): void {
    remove(observer);
  }

  /** Get a copy of the current observer context (for testing/debugging) */
  // Divergence: Rust returns Option<Arc<dyn Observer>>; TS returns Observer | null [E8]
  static current(): Observer | null {
    return current();
  }
}
