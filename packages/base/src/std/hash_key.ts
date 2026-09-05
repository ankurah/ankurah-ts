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
 *
 * A SEQUENCE is walked. An array is what the port writes a `Vec<T>` and a tuple
 * as, and a typed array is what it writes a `Vec<u8>` as; neither has a
 * `clone()`, so both used to come back as the very same object — and a map that
 * handed its clone the same array then owned one set of elements twice, so
 * dropping both maps dropped each element twice.
 */
export function cloned<T>(value: T): T {
  if (value === null || typeof value !== 'object') return value;
  const own = (value as { clone?: unknown }).clone;
  if (typeof own === 'function') return own.call(value) as T;
  if (Array.isArray(value)) return clonedSequence(value) as T;
  // A typed array is copied through its own constructor, which is what the
  // emitter writes for a `Vec<u8>` field (`new Uint8Array(x)`).
  if (ArrayBuffer.isView(value)) {
    const make = (value as object).constructor as new (from: unknown) => T;
    return new make(value);
  }
  return value;
}

/**
 * A sequence cloned element by element, exception-safely.
 *
 * For: a clone can throw — a `clone()` that panics, a value the runtime refuses
 * to clone at all — and what has already been cloned belongs to nobody, because
 * the caller never received the array. `map` left every earlier element to the
 * garbage collector, which is what the leak check reports. The elements are
 * built into a local list and released together if one of them throws.
 */
function clonedSequence<T>(value: T[]): T[] {
  const made: T[] = [];
  try {
    for (const element of value) made.push(cloned(element));
  } catch (error) {
    dropOwned(made);
    throw error;
  }
  return made;
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
    // Each part carries its own LENGTH, so no separator can be forged out of
    // the parts themselves. Joining with a comma made `["a", "b"]` and
    // `["a,s:b"]` one label — a `Vec<String>` field of a derived key collided
    // with a single string that happened to spell the join — and two keys in
    // one bucket that `equals` then had to tell apart. The derived `hash()` the
    // emitter writes already length-prefixes its fields; this is the same rule
    // for the sequence a tuple and a `Vec` are written as.
    const parts: string[] = [];
    for (let at = 0; at < obj.length; at++) {
      const part = keyHash(obj[at]);
      parts.push(`${part.length}:${part}`);
    }
    return `[${parts.join('')}]`;
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

  /**
   * Store a key the caller has established is absent, and hand back the entry
   * that now holds it.
   *
   * The entry is the stable thing: a `&mut V` into the map has to keep reaching
   * the same storage, and holding the LOOKUP key instead meant re-hashing a key
   * the entry had already released.
   */
  add(key: K, value: V): Entry<K, V> {
    const hash = keyHash(key);
    const entry: Entry<K, V> = { key, value };
    const bucket = this.#buckets.get(hash);
    if (bucket === undefined) this.#buckets.set(hash, [entry]);
    else bucket.push(entry);
    this.#size++;
    return entry;
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

/**
 * Two values of a type PARAMETER, compared the way `#[derive(PartialEq)]`
 * compares them.
 *
 * For: a field written as `T` is one the emitter cannot compare — `T` is a
 * number in `Holder<u32>` and a class in `Holder<Item>`, and `.equals()` on a
 * number is a TypeError — so the decision is the value's own surface at run
 * time. `keysEqual` is that walk; what this adds is the REFUSAL, because Rust's
 * derive on a generic carries `T: PartialEq` and a value declaring no `equals`
 * is one that bound excludes. Answering `false` for it would turn a value the
 * port should not be holding into a quiet "not equal".
 */
export function derivedEquals(left: unknown, right: unknown): boolean {
  refuseWithout(left, 'equals', 'PartialEq');
  return keysEqual(left, right);
}

/**
 * One value of a type PARAMETER, copied the way `#[derive(Clone)]` copies it.
 *
 * `cloned` is the walk — the value itself where the port writes it as a
 * primitive, its own `clone()` where it has one — and this adds the refusal, for
 * the same reason: `#[derive(Clone)]` on a generic carries `T: Clone`, and
 * handing back a shared object for a value declaring no `clone()` would give one
 * value two owners and the second drop would be a fatal.
 */
export function derivedClone<T>(value: T): T {
  refuseWithout(value, 'clone', 'Clone');
  return cloned(value);
}

/** The refusal both of the above make, worded from what is missing. */
function refuseWithout(value: unknown, member: string, derive: string): void {
  if (value === null || typeof value !== 'object' || isSequence(value)) return;
  if (typeof (value as Record<string, unknown>)[member] === 'function') return;
  throw new Error(
    `BUG: ${(value as object).constructor?.name ?? '(anonymous)'} stands where a type ` +
    `parameter\nis declared, and it declares no ${member}(). Rust's #[derive(${derive})] on a ` +
    `generic\ncarries a ${derive} bound on that parameter, so this is a value the port put ` +
    `there\nand Rust would not have.`,
  );
}
