// MIRRORS: ankurah/core/src/util/ivec.rs
//
// Rust's IVec is a small-vector optimization (inline array for small N, Vec for large).
// JS arrays are already dynamically sized and efficient for small collections.
// Divergence: No inline/heap distinction needed — JS arrays handle this natively [E7].
// We preserve the API surface for port fidelity.

/**
 * Inline vector — wraps a plain JS array.
 *
 * Rust: `pub enum IVec<T, const N: usize> { Small { data, len }, Large(Vec<T>) }`
 * Divergence: No const generic N or Small/Large distinction — JS arrays suffice [E7].
 */
export class IVec<T> {
  private data: T[] = [];

  constructor() {}

  len(): number {
    return this.data.length;
  }

  isEmpty(): boolean {
    return this.data.length === 0;
  }

  push(value: T): void {
    this.data.push(value);
  }

  contains(value: T): boolean {
    return this.data.includes(value);
  }

  /** Add a value if not already present. Returns true if added. */
  add(value: T): boolean {
    if (this.contains(value)) {
      return false;
    }
    this.push(value);
    return true;
  }

  iter(): T[] {
    return [...this.data];
  }

  asSlice(): readonly T[] {
    return this.data;
  }

  [Symbol.iterator](): Iterator<T> {
    return this.data[Symbol.iterator]();
  }
}
