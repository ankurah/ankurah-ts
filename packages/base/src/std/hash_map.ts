// TS-ONLY: Rust's std::collections::HashMap<K, V> and HashSet<T>, keyed by value.
//
// A JavaScript `Map` keys objects by identity, and Rust keys them by `Hash` and
// `Eq`. So `HashMap<EntityId, Peer>` cannot be a `Map`: two `EntityId`s built
// from the same 16 bytes are one key in Rust and two keys in a `Map`, and the
// second lookup misses. The port has been getting by on stringified keys —
// `peerConnections.get(nodeId.toBase64())` — which loses the key's type and
// makes the map own a string instead of the id.
//
// So the map here does what Rust does: a key's `hash()` picks the bucket and
// its `equals()` decides which entry in that bucket is the one. The hash is
// only a label — collisions cost a comparison and nothing more — so a key type
// is free to hash coarsely.
//
// **What may be a key.** The same family of types Rust's `Hash + Eq` covers,
// and nothing else:
//
//   - a primitive (string, number, bigint, boolean) and `null`;
//   - a sequence — an array or a typed array, which is how the port spells a
//     tuple and a `Vec<u8>` — hashed and compared element by element, which is
//     what Rust's `impl Hash for (A, B)` and `for [T]` do;
//   - an object declaring `hash(): string` and `equals(other): boolean`, which
//     is what `#[derive(Hash, PartialEq, Eq)]` emits.
//
// Anything else is refused by name rather than falling back to identity.
// Identity is precisely the bug this file exists to fix, and a map that
// silently keyed one type by value and another by identity would hide it in the
// one place nobody looks. The refusal is a plain throw, not an ownership fatal:
// the insert did not happen, so nothing is corrupted and there is nothing for
// the poison latch to protect.
//
// **Ownership.** The map owns its keys and its values, as Rust's does, and
// dropping it releases both. The methods that remove an entry release what they
// remove, except where Rust hands the value back to the caller: `remove` and
// `insert` return a value that becomes the caller's.

import { DropGuard } from './drop.ts';
import { dropContainer } from './guard.ts';
import { dropOwned, isCopyLike } from '../object.ts';
import { fatalSelfAssignment } from '../drop_registry.ts';

/**
 * What `#[derive(Hash, PartialEq, Eq)]` gives a type, in the spelling this map
 * asks for.
 *
 * `hash()` returns a string because the bucket is a string: a JavaScript number
 * cannot hold Rust's `u64` hash, and nothing here needs a number. Two keys that
 * are `equals` MUST return the same `hash()` — Rust's own `Hash`/`Eq` contract,
 * and the same thing goes wrong if it is broken: the table keeps two entries
 * for one key.
 */
export interface Hashable {
  hash(): string;
  equals(other: never): boolean;
}

/** One entry. A plain record: it is the table's, and nothing else names it. */
interface Entry<K, V> {
  key: K;
  value: V;
}

/** Where a lookup landed, and the bucket label it landed under. */
interface Found<K, V> {
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
class Table<K, V> {
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
export class HashMap<K, V> {
  readonly #table = new Table<K, V>();
  readonly #guard: DropGuard;
  readonly #label: string;

  /**
   * @param entries — pairs to fill the map with, as `new Map(pairs)` takes them.
   * @param label — TS-only: what to call this map in a leak report.
   */
  constructor(entries?: Iterable<readonly [K, V]> | null, label?: string) {
    this.#label = label ?? 'HashMap';
    this.#guard = new DropGuard(this, this.#label);
    if (entries) for (const [key, value] of entries) this.set(key, value);
  }

  /**
   * `get(&k)` — Rust returns `Option<&V>`, so this borrows: what comes back is
   * still the map's, and the caller must not drop it.
   */
  get(key: K): V | null {
    this.#guard.assertNotDropped();
    const found = this.#table.find(key);
    return found === null ? null : (found.bucket[found.at] as Entry<K, V>).value;
  }

  /** `contains_key(&k)`. */
  has(key: K): boolean {
    this.#guard.assertNotDropped();
    return this.#table.find(key) !== null;
  }

  /**
   * `insert(k, v)` — the map takes both, and hands back the value it displaced,
   * which is `null` when the key is new.
   *
   * Rust keeps the key it already has and drops the one it was handed, so an
   * insert over a present key releases the key passed in. That is the half of
   * `insert` a `Map.set` loses, and losing it leaks a key on every overwrite.
   */
  insert(key: K, value: V): V | null {
    this.#guard.assertNotDropped();
    const found = this.#table.find(key);
    if (found === null) {
      this.#table.add(key, value);
      return null;
    }
    const entry = found.bucket[found.at] as Entry<K, V>;
    // Handed back what it already holds: one value with two owners, and the
    // release below would drop what the map is keeping.
    if (entry.key === key && !isCopyLike(key)) fatalSelfAssignment(`${this.#label} key`);
    if (entry.value === value && !isCopyLike(value)) fatalSelfAssignment(this.#label);
    const displaced = entry.value;
    entry.value = value;
    dropOwned(key); // the surplus key, which Rust drops when insert returns
    return displaced;
  }

  /**
   * `Map.prototype.set`, which is what the emitter writes for Rust's `insert`
   * where the source discards the displaced value — `map.insert(k, v);` as a
   * statement. It is `insert` with that value released, because in Rust it is
   * released at the end of that statement and nobody else can.
   */
  set(key: K, value: V): this {
    this.#guard.assertNotDropped();
    dropOwned(this.insert(key, value));
    return this;
  }

  /**
   * `remove(&k)` — the value becomes the caller's; the key the map was storing
   * is dropped, as Rust's `remove` drops it. `null` when the key was absent.
   *
   * `HashMap<K, Option<X>>` cannot tell an absent key from a stored `None`
   * here, because this port spells both `null`. Ask `has` first where the
   * difference matters — the same limitation `Option<T>` has everywhere else.
   */
  remove(key: K): V | null {
    this.#guard.assertNotDropped();
    const found = this.#table.find(key);
    if (found === null) return null;
    const entry = this.#table.take(found);
    dropOwned(entry.key);
    return entry.value;
  }

  /**
   * `Map.prototype.delete`, which is what the emitter writes for Rust's
   * `remove` where the source discards the value — `map.remove(&k);` as a
   * statement. Key and value are both released, and the answer is whether there
   * was an entry at all.
   */
  delete(key: K): boolean {
    this.#guard.assertNotDropped();
    const found = this.#table.find(key);
    if (found === null) return false;
    const entry = this.#table.take(found);
    dropOwned(entry.key);
    dropOwned(entry.value);
    return true;
  }

  /** `clear()` — every key and every value is released, and the map lives on. */
  clear(): void {
    this.#guard.assertNotDropped();
    const entries = this.#table.all();
    this.#table.reset();
    for (const entry of entries) {
      dropOwned(entry.key);
      dropOwned(entry.value);
    }
  }

  /** `len()`, under the name the emitter writes it as. */
  get size(): number {
    this.#guard.assertNotDropped();
    return this.#table.size;
  }

  // ── Iteration. Every one of these borrows: Rust's `iter()` yields `(&K, &V)`
  //    and the map is still the owner, so nothing an iterator hands out may be
  //    dropped by whoever received it. Each takes a snapshot first, so a loop
  //    that deletes as it goes — which is what `retain` becomes — is safe.

  entries(): IterableIterator<[K, V]> {
    this.#guard.assertNotDropped();
    return this.#table.all().map((entry): [K, V] => [entry.key, entry.value])[Symbol.iterator]();
  }

  keys(): IterableIterator<K> {
    this.#guard.assertNotDropped();
    return this.#table.all().map((entry) => entry.key)[Symbol.iterator]();
  }

  values(): IterableIterator<V> {
    this.#guard.assertNotDropped();
    return this.#table.all().map((entry) => entry.value)[Symbol.iterator]();
  }

  [Symbol.iterator](): IterableIterator<[K, V]> {
    this.#guard.assertNotDropped();
    return this.entries();
  }

  /**
   * Dropping a `HashMap<K, V>` in Rust drops every key and every value in it.
   * They sit in `#private` state the owning object's cascade cannot see, so the
   * map releases them itself.
   */
  drop(): void {
    dropContainer(this, this.#guard, this.#label, () => null, () => {
      const owned: unknown[] = [];
      for (const entry of this.#table.all()) owned.push(entry.key, entry.value);
      return owned;
    });
  }
}

/**
 * `std::collections::HashSet<T>`.
 *
 * Rust's `HashSet<T>` is a `HashMap<T, ()>`, and this one is the same table
 * with the value half unused. It holds its own table rather than a `HashMap`,
 * so that a forgotten set is one registered value and one leak report.
 */
export class HashSet<T> {
  readonly #table = new Table<T, null>();
  readonly #guard: DropGuard;
  readonly #label: string;

  /**
   * @param values — values to fill the set with, as `new Set(values)` takes them.
   * @param label — TS-only: what to call this set in a leak report.
   */
  constructor(values?: Iterable<T> | null, label?: string) {
    this.#label = label ?? 'HashSet';
    this.#guard = new DropGuard(this, this.#label);
    if (values) for (const value of values) this.insert(value);
  }

  /**
   * `insert(v)` — true when the value was not there before.
   *
   * Rust keeps the value it already has and drops the one it was handed, the
   * way the map keeps its key.
   */
  insert(value: T): boolean {
    this.#guard.assertNotDropped();
    const found = this.#table.find(value);
    if (found === null) {
      this.#table.add(value, null);
      return true;
    }
    const held = (found.bucket[found.at] as Entry<T, null>).key;
    if (held === value && !isCopyLike(value)) fatalSelfAssignment(this.#label);
    dropOwned(value); // the surplus value, which Rust drops when insert returns
    return false;
  }

  /** `Set.prototype.add`, which is what the emitter writes for `insert`. */
  add(value: T): this {
    this.#guard.assertNotDropped();
    this.insert(value);
    return this;
  }

  /** `contains(&v)`. */
  has(value: T): boolean {
    this.#guard.assertNotDropped();
    return this.#table.find(value) !== null;
  }

  /**
   * `remove(&v)` — the stored value is released, and the answer is whether one
   * was there. `Set.prototype.delete` is the same call under the name the
   * emitter writes it as.
   */
  remove(value: T): boolean {
    this.#guard.assertNotDropped();
    const found = this.#table.find(value);
    if (found === null) return false;
    dropOwned(this.#table.take(found).key);
    return true;
  }

  /** `Set.prototype.delete`, which is what the emitter writes for `remove`. */
  delete(value: T): boolean {
    this.#guard.assertNotDropped();
    return this.remove(value);
  }

  /** `clear()` — every value is released, and the set lives on. */
  clear(): void {
    this.#guard.assertNotDropped();
    const entries = this.#table.all();
    this.#table.reset();
    for (const entry of entries) dropOwned(entry.key);
  }

  /** `len()`, under the name the emitter writes it as. */
  get size(): number {
    this.#guard.assertNotDropped();
    return this.#table.size;
  }

  // Borrowing, like the map's: Rust's `iter()` yields `&T`.

  values(): IterableIterator<T> {
    this.#guard.assertNotDropped();
    return this.#table.all().map((entry) => entry.key)[Symbol.iterator]();
  }

  keys(): IterableIterator<T> {
    this.#guard.assertNotDropped();
    return this.values();
  }

  [Symbol.iterator](): IterableIterator<T> {
    this.#guard.assertNotDropped();
    return this.values();
  }

  /** Dropping a `HashSet<T>` drops every value in it, as Rust's does. */
  drop(): void {
    dropContainer(this, this.#guard, this.#label, () => null, () => this.#table.all().map((e) => e.key));
  }
}
