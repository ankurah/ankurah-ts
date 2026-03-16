// MIRRORS: ankurah/storage/indexeddb-wasm/src/scanner.rs

// Divergence: Rust uses futures::stream::unfold + Pin<Box<CBStream>> for [E16]
// cursor iteration with prefix guard. In TS, we use an AsyncGenerator which
// naturally handles the cursor lifecycle with native IndexedDB API.

import type { Value } from '@ankurah/core';
import { IdbObject } from './util/object.ts';
import { valueToIdb } from './idb_value.ts';

/**
 * Configuration for an IndexedDB index scan.
 *
 * Mirrors Rust `IdbIndexScanner` which wraps index + key range + direction + prefix guard.
 */
export class IdbIndexScanner {
  private readonly index: IDBIndex;
  private readonly keyRange: IDBKeyRange | null;
  private readonly cursorDirection: IDBCursorDirection;
  private readonly eqPrefixLen: number;
  private readonly eqPrefixIdb: unknown[];

  constructor(
    index: IDBIndex,
    keyRange: IDBKeyRange | null,
    cursorDirection: IDBCursorDirection,
    eqPrefixLen: number,
    eqPrefixValues: Value[],
  ) {
    // Convert equality prefix values to IDB-compatible values for comparison
    this.eqPrefixIdb = eqPrefixValues.map(v => valueToIdb(v));
    this.index = index;
    this.keyRange = keyRange;
    this.cursorDirection = cursorDirection;
    this.eqPrefixLen = eqPrefixLen;
  }

  /**
   * Scan the index, yielding IdbObject for each record.
   *
   * The scan handles:
   * - Opening the cursor with the configured range and direction
   * - Prefix guard termination for open-ended ranges
   * - Cursor advancement via continue()
   *
   * Mirrors Rust `IdbIndexScanner::scan(self) -> impl Stream<Item = Result<Object, RetrievalError>>`.
   */
  async *scan(): AsyncGenerator<IdbObject> {
    // Open the cursor
    const request = this.keyRange
      ? this.index.openCursor(this.keyRange, this.cursorDirection)
      : this.index.openCursor(null, this.cursorDirection);

    while (true) {
      const cursor = await new Promise<IDBCursorWithValue | null>((resolve, reject) => {
        request.onsuccess = () => {
          resolve(request.result as IDBCursorWithValue | null);
        };
        request.onerror = () => {
          reject(new Error(`Failed to open cursor: ${request.error?.message ?? 'unknown error'}`));
        };
      });

      // End of cursor
      if (cursor === null) {
        return;
      }

      // Apply prefix guard if needed
      if (this.eqPrefixLen > 0) {
        const key = cursor.key;
        if (key !== undefined && key !== null && Array.isArray(key)) {
          for (let i = 0; i < this.eqPrefixLen; i++) {
            const lhs = key[i];
            const rhs = this.eqPrefixIdb[i];
            // Use strict equality for IDB key comparison
            // (IDB keys from the same encoding will be identical types)
            if (lhs !== rhs) {
              // For objects (ArrayBuffer), need deeper comparison
              if (typeof lhs !== typeof rhs || lhs !== rhs) {
                // Prefix doesn't match - end of range
                return;
              }
            }
          }
        }
      }

      // Get the record value
      yield new IdbObject(cursor.value);

      // Advance cursor for next iteration
      cursor.continue();
    }
  }
}
