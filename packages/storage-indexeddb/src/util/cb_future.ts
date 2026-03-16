// MIRRORS: ankurah/storage/indexeddb-wasm/src/util/cb_future.rs

// Divergence: Rust uses wasm-bindgen Closure + EventTarget + oneshot channel [E16]
// to bridge async WASM ↔ JS event listeners. In TS, we use native Promise +
// addEventListener directly since there's no WASM boundary.

import { extractMessage } from '../error.ts';

/**
 * Returns a Promise that resolves when the success event fires on the target,
 * or rejects when the error event fires.
 *
 * Mirrors Rust `CBFuture::new(target, success_events, error_events)`.
 */
export function cbFuture(
  target: EventTarget,
  successEvent: string | string[],
  errorEvent: string | string[],
): Promise<void> {
  const successEvents = Array.isArray(successEvent) ? successEvent : [successEvent];
  const errorEvents = Array.isArray(errorEvent) ? errorEvent : [errorEvent];

  return new Promise<void>((resolve, reject) => {
    const cleanup = () => {
      for (const evt of successEvents) {
        target.removeEventListener(evt, onSuccess);
      }
      for (const evt of errorEvents) {
        target.removeEventListener(evt, onError);
      }
    };

    const onSuccess = () => {
      cleanup();
      resolve();
    };

    const onError = (event: Event) => {
      cleanup();
      reject(new Error(`IDB error: ${extractMessage(event)}`));
    };

    for (const evt of successEvents) {
      target.addEventListener(evt, onSuccess, { once: false });
    }
    for (const evt of errorEvents) {
      target.addEventListener(evt, onError, { once: false });
    }
  });
}
