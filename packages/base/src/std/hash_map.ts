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
import { Table, cloned, valueEquals, type Entry } from './hash_key.ts';
import { fatalSelfAssignment } from '../drop_registry.ts';
export { keyHash, keysEqual, cloned, derivedEquals, derivedClone, derivedHash, valueEquals, valueNotEquals, type Hashable } from './hash_key.ts';
import { BorrowMut } from './borrow.ts';
import { invoke, type Invocable } from '../closure.ts';

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

  /**
   * TS-only: the stored entry for a key, or `null` where the map has none.
   *
   * `map.entry(k)` hands back a `&mut V` INTO the map, and the entry is what
   * that reference points at: it is the same object for as long as the map
   * holds it, and it does not have to be looked up again. `$`-namespaced
   * because it is the runtime's own seam and no Rust field name can collide
   * with one.
   */
  $findEntry(key: K): Entry<K, V> | null {
    this.#guard.assertNotDropped();
    const found = this.#table.find(key);
    return found === null ? null : (found.bucket[found.at] as Entry<K, V>);
  }

  /** TS-only: store a key the caller has established is absent, and hand back
   * the entry that now holds it. */
  $addEntry(key: K, value: V): Entry<K, V> {
    this.#guard.assertNotDropped();
    return this.#table.add(key, value);
  }

  /** TS-only: read through a `&mut V` this map handed out. */
  $readEntry(entry: Entry<K, V>): V {
    this.#guard.assertNotDropped();
    return entry.value;
  }

  /**
   * TS-only: write through a `&mut V` this map handed out.
   *
   * `*map.entry(k).or_insert(0) += 1` writes the VALUE and leaves the key
   * alone; going back through `set` handed the map the key it is already
   * storing, which is a self-assignment and a fatal.
   */
  $writeEntry(entry: Entry<K, V>, value: V): void {
    this.#guard.assertNotDropped();
    if (entry.value === value) return;
    // Rust's `*slot = v` drops what was there.
    dropOwned(entry.value);
    entry.value = value;
  }

  /**
   * `impl PartialEq for HashMap`: the same size, and every key mapping to a
   * value the other's does too.
   *
   * Order is not part of it — Rust's is a hash map and so is this — and the
   * lookup goes through the table, so two maps whose keys were inserted in
   * different orders still compare equal. `==` between two maps had been
   * `===`, which compares identity and was false for every pair of distinct
   * maps.
   *
   * The KEYS are matched by the table's own lookup, which is a `Map`'s rule: a
   * key that answers no is an absent key. The VALUES go through `valueEquals`,
   * which is Rust's rule and refuses a pair it cannot compare — `impl PartialEq
   * for HashMap<K, V>` carries `V: PartialEq`, so a value declaring no
   * `equals()` is one the bound excludes, and answering `false` for it turned
   * that into a quiet "these maps differ".
   */
  equals(other: HashMap<K, V>): boolean {
    this.#guard.assertNotDropped();
    if (other === this) return true;
    if (!(other instanceof HashMap) || other.size !== this.size) return false;
    for (const entry of this.#table.all()) {
      const found = other.#table.find(entry.key);
      if (found === null) return false;
      if (!valueEquals(entry.value, (found.bucket[found.at] as Entry<K, V>).value)) return false;
    }
    return true;
  }

  /**
   * `#[derive(Clone)]` on a type holding one: a NEW map with a clone of every
   * key and every value, so the two maps own separate values.
   *
   * A shallow copy handed both maps one set of values, and the second drop of
   * the pair released each of them twice.
   */
  clone(): HashMap<K, V> {
    this.#guard.assertNotDropped();
    // The destination used to be built first and filled as the walk went, so a
    // key or a value whose `clone()` throws left a registered half-built map,
    // and every pair already in it, to nobody. The pairs are cloned into a
    // local list, released together if one of them throws, and only a complete
    // list becomes a map.
    const pairs: [K, V][] = [];
    try {
      for (const entry of this.#table.all()) {
        // The key goes into the list before its value is cloned, so a throwing
        // value clone does not orphan the key beside it.
        const pair: [K, V] = [cloned(entry.key), undefined as never];
        pairs.push(pair);
        pair[1] = cloned(entry.value);
      }
    } catch (error) {
      dropOwned(pairs);
      throw error;
    }
    // J5: the INSERTION phase is exception-unsafe too. `set` hashes the key,
    // and a key whose `hash()` throws left a registered half-built map to
    // nobody and gave every pair already in it two owners — the map and the
    // list. So the walk is guarded: what has not been handed over is still the
    // list's, and what has is the map's, which its own `drop()` releases.
    const copy = new HashMap<K, V>(null, this.#label);
    let at = 0;
    try {
      for (; at < pairs.length; at++) copy.set(pairs[at]![0], pairs[at]![1]);
    } catch (error) {
      dropOwned(pairs.slice(at));
      copy.drop();
      throw error;
    }
    return copy;
  }

  /**
   * `HashMap::from([(k, v), ..])`, and what `collect()` into one becomes.
   *
   * The emitter wrote this call before anything declared it, so every
   * `HashMap::from` in the corpus named a static that does not exist.
   */
  static from<K, V>(entries: Iterable<readonly [K, V]>): HashMap<K, V> {
    return new HashMap<K, V>(entries);
  }

  /**
   * `map.entry(k)` — the one place in Rust's map API that takes the key BEFORE
   * deciding whether it needs it.
   *
   * The entry owns the key it was handed, as Rust's does: an occupied entry
   * releases it, because the map keeps the key it already has.
   */
  entry(key: K): MapEntry<K, V> {
    this.#guard.assertNotDropped();
    return new MapEntry(this, key);
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
   * Every pair, with the map CONSUMED — Rust's `IntoIterator for HashMap`.
   *
   * `for (k, v) in map` moves the map into its `IntoIter`, which hands out an
   * OWNED pair each turn and drops whatever it has not handed out when the loop
   * ends, however it ends. So this empties the map and marks it dropped without
   * releasing what was in it: from here every pair belongs to whoever walks the
   * array, and the tail nobody reached is that caller's to release. The emitted
   * loop is the same index walk a `Vec` gets, with `dropOwned` over the tail in
   * its `finally`.
   *
   * A map used after this reports a use after drop, which is the run-time
   * spelling of the move Rust refuses at compile time.
   */
  intoEntries(): [K, V][] {
    this.#guard.assertNotDropped();
    const pairs = this.#table.all().map((entry): [K, V] => [entry.key, entry.value]);
    this.#table.reset();
    this.#guard.markDropped(this);
    return pairs;
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
/**
 * What `HashMap::entry` hands back: a key the map may or may not need, and the
 * three ways Rust says what to do when it does not have one.
 *
 * Each of them CONSUMES the entry, so each releases the key where the map keeps
 * the one it already has — and releases the value, or the closure, it did not
 * need. Rust's `or_insert(v)` evaluates `v` eagerly and drops it on the occupied
 * path; `or_insert_with(f)` does not call `f` at all there, and `f` is dropped
 * unused.
 */
export class MapEntry<K, V> {
  readonly #map: HashMap<K, V>;
  readonly #key: K;

  constructor(map: HashMap<K, V>, key: K) {
    this.#map = map;
    this.#key = key;
  }

  /** Is there already a value under this key? */
  get occupied(): boolean {
    return this.#map.has(this.#key);
  }

  /** `or_insert(v)`: the value there, or `v` put there. */
  orInsert(value: V): BorrowMut<V> {
    const found = this.#map.$findEntry(this.#key);
    if (found !== null) {
      dropOwned(this.#key);
      dropOwned(value);
      // The STORED entry, never the lookup key that was just released: a Slot
      // holding the released key raised `used after being dropped` on its first
      // read.
      return new Slot(this.#map, found);
    }
    return new Slot(this.#map, this.#map.$addEntry(this.#key, value));
  }

  /** `or_insert_with(f)`: `f` is called only where there is nothing there. */
  orInsertWith(make: Invocable<[], V>): BorrowMut<V> {
    const found = this.#map.$findEntry(this.#key);
    if (found !== null) {
      dropOwned(this.#key);
      // Rust consumes `f` whether or not it calls it, and a closure that owns
      // captures has to be released here or they leak.
      dropOwned(make);
      return new Slot(this.#map, found);
    }
    // The key is the entry's until the map takes it. A factory that throws
    // leaves it with nobody, which Rust's unwind does not: the entry owns the
    // key from `entry(k)` onwards and drops it on the way out.
    let value: V;
    try {
      value = invoke(make);
    } catch (thrown) {
      dropOwned(this.#key);
      throw thrown;
    }
    return new Slot(this.#map, this.#map.$addEntry(this.#key, value));
  }

  /**
   * `or_default()`: the same, with the thunk standing in for `V: Default`.
   *
   * Rust reads the default off the type; the port has no such thing for an
   * arbitrary `V`, so the caller supplies it and the emitter writes the
   * `default()` of the value type there.
   */
  orDefault(make: Invocable<[], V>): BorrowMut<V> {
    return this.orInsertWith(make);
  }

  /**
   * `and_modify(f)`: `f` sees the value only where there is one, and the entry
   * comes back either way so a finisher can follow it.
   *
   * What `f` is handed is a `&mut V` into the MAP — the same write-through slot
   * `or_insert` answers — so `entry(k).and_modify(|n| *n += 1).or_insert(1)`
   * counts. Rust takes `f` by value and drops it whether or not it calls it;
   * `invoke` marks it moved where it runs, so only the untaken path releases it
   * here. The key stays the entry's: `and_modify` does not consume it.
   */
  andModify(change: Invocable<[BorrowMut<V>], void>): MapEntry<K, V> {
    const found = this.#map.$findEntry(this.#key);
    if (found === null) {
      dropOwned(change);
      return this;
    }
    invoke(change, new Slot(this.#map, found));
    return this;
  }
}

/**
 * A `&mut V` into a map: reading it reads the map, and writing it writes the
 * map — which is what makes `*counts.entry(k).or_insert(0) += 1` count.
 *
 * A plain `BorrowMut` holds a COPY, so the increment landed on the copy and the
 * map never changed.
 */
class Slot<K, V> extends BorrowMut<V> {
  readonly #map: HashMap<K, V>;
  readonly #entry: Entry<K, V>;

  constructor(map: HashMap<K, V>, entry: Entry<K, V>) {
    super(undefined as never);
    this.#map = map;
    this.#entry = entry;
  }

  override get value(): V {
    return this.#map.$readEntry(this.#entry);
  }

  override set value(v: V) {
    // Through the map's own seam: `set` would hand it the key it is already
    // storing, which is a self-assignment and a fatal.
    this.#map.$writeEntry(this.#entry, v);
  }
}

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

  /**
   * `impl PartialEq for HashSet`: the same size, and every element of one in
   * the other. Order is not part of it, and membership is the table's own
   * lookup — a set has no values beside its keys, so there is nothing here for
   * the strict comparison to reach.
   */
  equals(other: HashSet<T>): boolean {
    this.#guard.assertNotDropped();
    if (other === this) return true;
    if (!(other instanceof HashSet) || other.size !== this.size) return false;
    for (const entry of this.#table.all()) {
      if (!other.has(entry.key)) return false;
    }
    return true;
  }

  /** `#[derive(Clone)]`: a new set holding a clone of every value. */
  clone(): HashSet<T> {
    this.#guard.assertNotDropped();
    // Exception-safe for the same reason `HashMap::clone` is.
    const values: T[] = [];
    try {
      for (const entry of this.#table.all()) values.push(cloned(entry.key));
    } catch (error) {
      dropOwned(values);
      throw error;
    }
    // Guarded for the same reason `HashMap::clone`'s insertion phase is: `add`
    // hashes the value, and a value whose `hash()` throws part way through left
    // a registered half-built set and two owners for everything already in it.
    const copy = new HashSet<T>(null, this.#label);
    let at = 0;
    try {
      for (; at < values.length; at++) copy.add(values[at]!);
    } catch (error) {
      dropOwned(values.slice(at));
      copy.drop();
      throw error;
    }
    return copy;
  }

  /** `HashSet::from([a, b])`, and what `collect()` into one becomes. */
  static from<T>(values: Iterable<T>): HashSet<T> {
    return new HashSet<T>(values);
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
