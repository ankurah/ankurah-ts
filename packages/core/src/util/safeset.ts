// MIRRORS: ankurah/core/src/util/safeset.rs
//
// Rust's SafeSet wraps HashSet with RwLock for concurrent access safety.
// Divergence: JS is single-threaded — no lock needed. Plain Set suffices [E8].
// We preserve the API surface for port fidelity.

/**
 * A simple Set wrapper mirroring Rust's SafeSet API.
 *
 * Rust: `pub struct SafeSet<T>(RwLock<HashSet<T>>)`
 * Divergence: No RwLock — JS is single-threaded [E8].
 */
export class SafeSet<T> {
  private set: Set<T> = new Set();

  insert(value: T): boolean {
    if (this.set.has(value)) return false;
    this.set.add(value);
    return true;
  }

  remove(value: T): boolean {
    return this.set.delete(value);
  }

  contains(value: T): boolean {
    return this.set.has(value);
  }

  isEmpty(): boolean {
    return this.set.size === 0;
  }

  len(): number {
    return this.set.size;
  }

  toVec(): T[] {
    return [...this.set];
  }
}
