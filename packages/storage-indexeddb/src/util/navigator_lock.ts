// MIRRORS: ankurah/storage/indexeddb-wasm/src/util/navigator_lock.rs

// Divergence: Rust uses wasm-bindgen + Reflect to access navigator.locks [E16]
// and wraps non-Send JS objects in SendWrapper. In TS, we call the Web Locks
// API directly since we're running in a native JS environment.

/**
 * Wraps the Web Locks API for exclusive lock acquisition.
 *
 * Mirrors Rust `NavigatorLock::with(lock_name, work)`.
 */
export class NavigatorLock {
  /**
   * Acquire an exclusive lock and run `work` while holding it.
   * Falls back to running without a lock if the Web Locks API is unavailable
   * (e.g., non-secure context).
   */
  static async with(lockName: string, work: () => Promise<void>): Promise<void> {
    // Check if locks API is available (requires secure context)
    if (typeof navigator === 'undefined' || !navigator.locks) {
      console.warn('Web Locks API not available (requires secure context), running without lock');
      return work();
    }

    return navigator.locks.request(lockName, { mode: 'exclusive' }, async () => {
      await work();
    });
  }
}
