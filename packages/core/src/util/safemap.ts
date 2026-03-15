// MIRRORS: ankurah/core/src/util/safemap.rs
//
// Rust's SafeMap wraps HashMap with RwLock for concurrent access safety.
// Divergence: JS is single-threaded — no lock needed. Plain Map suffices [E8].
// We preserve the API surface for port fidelity.

/**
 * A simple Map wrapper mirroring Rust's SafeMap API.
 *
 * Rust: `pub struct SafeMap<K, V>(RwLock<HashMap<K, V>>)`
 * Divergence: No RwLock — JS is single-threaded [E8].
 */
export class SafeMap<K, V> {
  private map: Map<K, V> = new Map();

  insert(key: K, value: V): void {
    this.map.set(key, value);
  }

  remove(key: K): V | undefined {
    const value = this.map.get(key);
    this.map.delete(key);
    return value;
  }

  retain(cb: (key: K, value: V) => boolean): void {
    for (const [key, value] of this.map) {
      if (!cb(key, value)) {
        this.map.delete(key);
      }
    }
  }

  isEmpty(): boolean {
    return this.map.size === 0;
  }

  len(): number {
    return this.map.size;
  }

  clear(): void {
    this.map.clear();
  }

  containsKey(key: K): boolean {
    return this.map.has(key);
  }

  get(key: K): V | undefined {
    return this.map.get(key);
  }

  getOrDefault(key: K, defaultValue: V): V {
    if (this.map.has(key)) {
      return this.map.get(key)!;
    }
    this.map.set(key, defaultValue);
    return defaultValue;
  }

  toVec(): [K, V][] {
    return [...this.map.entries()];
  }

  keys(): K[] {
    return [...this.map.keys()];
  }

  values(): V[] {
    return [...this.map.values()];
  }

  /** For SafeMap<K, V[]>: push a value into the array at key. */
  push(key: K, value: V extends Array<infer H> ? H : never): void {
    let arr = this.map.get(key) as V | undefined;
    if (!arr) {
      arr = [] as unknown as V;
      this.map.set(key, arr);
    }
    (arr as unknown as unknown[]).push(value);
  }

  /** For SafeMap<K, V[]>: remove a value by equality from the array at key. */
  removeEq(key: K, value: V extends Array<infer H> ? H : never): void {
    const arr = this.map.get(key);
    if (Array.isArray(arr)) {
      const idx = arr.indexOf(value);
      if (idx !== -1) arr.splice(idx, 1);
    }
  }

  /** For SafeMap<K, Set<H>>: insert a value into the set at key. */
  setInsert(key: K, value: V extends Set<infer H> ? H : never): void {
    let set = this.map.get(key) as V | undefined;
    if (!set) {
      set = new Set() as unknown as V;
      this.map.set(key, set);
    }
    (set as unknown as Set<unknown>).add(value);
  }

  /** For SafeMap<K, Set<H>>: remove a value from the set at key. */
  setRemove(key: K, value: V extends Set<infer H> ? H : never): boolean {
    const set = this.map.get(key);
    if (set instanceof Set) {
      return set.delete(value);
    }
    return false;
  }
}
