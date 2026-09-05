// TS-ONLY: what may be a HashMap or HashSet key, and the bucket table both
// containers are built on.
//
// A JavaScript `Map` keys objects by identity, and Rust keys them by `Hash` and
// `Eq`: two `EntityId`s built from the same 16 bytes are one key in Rust and two
// in a `Map`, and the second lookup misses. So a key's `hash()` picks the bucket
// and its `equals()` decides which entry in that bucket is the one. The hash is
// only a label — a collision costs a comparison and nothing more — so a key type
// is free to hash coarsely.
//
// **What may be a key.** The same family Rust's `Hash + Eq` covers, and nothing
// else: a primitive and `null`; a sequence — an array or a typed array, which is
// how the port spells a tuple and a `Vec<u8>` — hashed element by element; and
// an object declaring `hash()` and `equals()`, which is what
// `#[derive(Hash, PartialEq, Eq)]` emits. Anything else is refused BY NAME
// rather than falling back to identity, because identity is the bug this file
// exists to fix and a map that silently keyed one type by value and another by
// identity would hide it where nobody looks.

import { dropOwned, isCopyLike } from '../object.ts';
import { fatalSelfAssignment } from '../drop_registry.ts';

export interface Hashable {
  hash(): string;
  equals(other: never): boolean;
}

/**
 * One value of a cloned container: its own `clone()` where it has one, and the
 * value itself where the port writes it as a primitive.
 *
 * `#[derive(Clone)]` on a map clones every key and every value; the port has no
 * type information here, so the value's own surface is what says which it is.
 */
export function cloned<T>(value: T): T {
  if (value === null || typeof value !== 'object') return value;
  const own = (value as { clone?: unknown }).clone;
  return typeof own === 'function' ? (own.call(value) as T) : value;
}

/** One entry. A plain record: it is the table's, and nothing else names it. */
export interface Entry<K, V> {
  key: K;
  value: V;
}

/** Where a lookup landed, and the bucket label it landed under. */
export interface Found<K, V> {
  hash: string;
  bucket: Entry<K, V>[];
  at: number;
}

function isSequence(value: object): value is ArrayLike<unknown> {
  return Array.isArray(value) || ArrayBuffer.isView(value);
}

/**
 * The bucket label for a key. Distinct types are tagged apart, so `1`, `"1"`
 * and `1n` do not share a bucket — not that sharing one would be wrong, only
 * slower.
 */
export function keyHash(key: unknown): string {
  switch (typeof key) {
    case 'string': return `s:${key}`;
    // String(-0) is "0", so -0 and 0 land together, which is what a JS Map does
    // and what Rust's integer keys mean. Rust has no f64 key: f64 is not Hash.
    case 'number': return `n:${key}`;
    case 'bigint': return `i:${key}`;
    case 'boolean': return `b:${key}`;
    case 'undefined': return 'u:';
    default: break;
  }
  if (key === null) return 'z:';
  const obj = key as object;
  if (isSequence(obj)) {
    const parts: string[] = [];
    for (let at = 0; at < obj.length; at++) parts.push(keyHash(obj[at]));
    return `[${parts.join(',')}]`;
  }
  const own = (obj as Partial<Hashable>).hash;
  if (typeof own === 'function') return `h:${String(own.call(obj))}`;
  throw new Error(
    `BUG: ${obj.constructor?.name ?? '(anonymous)'} was used as a HashMap or HashSet key,\n` +
    `and it declares no hash(). Rust would have needed a Hash impl to compile the\n` +
    `map at all. A ported type gets hash() and equals() from #[derive(Hash, Eq)];\n` +
    `keying it by identity instead is exactly the bug this map exists to prevent.`,
  );
}

/**
 * Are two keys the same key? `equals` where the type has one, and otherwise
 * SameValueZero — a JS `Map`'s rule, under which `NaN` is its own key and `-0`
 * and `0` are one.
 */
export function keysEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a !== a && b !== b) return true; // NaN
  if (a === null || b === null || typeof a !== 'object' || typeof b !== 'object') return false;
  if (isSequence(a) && isSequence(b)) {
    if (a.length !== b.length) return false;
    for (let at = 0; at < a.length; at++) {
      if (!keysEqual(a[at], b[at])) return false;
    }
    return true;
  }
  const own = (a as Partial<Hashable>).equals;
  if (typeof own === 'function') return own.call(a, b as never) === true;
  return false;
}

/**
 * The buckets, and nothing else. It owns nothing, releases nothing and is not
 * leak-tracked: `HashMap` and `HashSet` each hold one, and each of them is the
 * registered value with the label and the liveness checks. Keeping the
 * bookkeeping here is what lets the set be a table of its own rather than a map
 * wrapped in a set, which would register one leak twice.
 */
export class Table<K, V> {
  readonly #buckets = new Map<string, Entry<K, V>[]>();
  #size = 0;

  get size(): number { return this.#size; }

  find(key: K): Found<K, V> | null {
    const hash = keyHash(key);
    const bucket = this.#buckets.get(hash);
    if (bucket === undefined) return null;
    for (let at = 0; at < bucket.length; at++) {
      if (keysEqual((bucket[at] as Entry<K, V>).key, key)) return { hash, bucket, at };
    }
    return null;
  }

  /** Store a key the caller has established is absent. */
  add(key: K, value: V): void {
    const hash = keyHash(key);
    const bucket = this.#buckets.get(hash);
    if (bucket === undefined) this.#buckets.set(hash, [{ key, value }]);
    else bucket.push({ key, value });
    this.#size++;
  }

  /** Unhook an entry a lookup found, and hand it over. Releases nothing. */
  take(found: Found<K, V>): Entry<K, V> {
    const [entry] = found.bucket.splice(found.at, 1) as [Entry<K, V>];
    if (found.bucket.length === 0) this.#buckets.delete(found.hash);
    this.#size--;
    return entry;
  }

  /** Every entry, in bucket order, as a snapshot — so a loop may delete. */
  all(): Entry<K, V>[] {
    const entries: Entry<K, V>[] = [];
    for (const bucket of this.#buckets.values()) entries.push(...bucket);
    return entries;
  }

  /** Forget every entry. Releases nothing: the caller does that first. */
  reset(): void {
    this.#buckets.clear();
    this.#size = 0;
  }
}

/**
 * `std::collections::HashMap<K, V>`.
 *
 * A generic container rather than a ported Rust type, so it is leak-tracked
 * through a `DropGuard` the way `Mutex` and `RwLock` are, and does not extend
 * `AkObject`.
 */
