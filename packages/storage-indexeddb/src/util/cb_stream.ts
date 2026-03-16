// MIRRORS: ankurah/storage/indexeddb-wasm/src/util/cb_stream.rs

// Divergence: Rust uses wasm-bindgen Closure + mpsc channel to bridge IndexedDB [E16]
// cursor iteration (repeated success events) into a Rust Stream. In TS, we use
// an AsyncGenerator that wraps the IDBRequest success/error events directly.

import { extractMessage } from '../error.ts';

/**
 * Yields successive cursor results from an IDBRequest.
 *
 * IndexedDB cursors fire repeated "success" events (one per cursor.continue()),
 * each time populating request.result with the cursor (or null at end).
 *
 * Mirrors Rust `cb_stream(target, "success", "error")` which returns a Stream<Item = Result<JsValue, Event>>.
 */
export async function* cbStream(
  request: IDBRequest,
  _successEvent: string = 'success',
  _errorEvent: string = 'error',
): AsyncGenerator<IDBCursorWithValue | null> {
  while (true) {
    const result = await new Promise<IDBCursorWithValue | null>((resolve, reject) => {
      request.onsuccess = () => {
        resolve(request.result as IDBCursorWithValue | null);
      };
      request.onerror = (event) => {
        reject(new Error(`Cursor error: ${extractMessage(event)}`));
      };
    });

    yield result;

    // null means end of cursor
    if (result === null) {
      break;
    }
  }
}
