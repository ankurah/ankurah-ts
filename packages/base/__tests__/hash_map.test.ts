// TS-ONLY: Tests for the value-keyed HashMap and HashSet (src/std/hash_map.ts).
import { describe, test, expect, afterEach } from 'bun:test';
import { HashMap, HashSet, Struct, Drop, clearFatalLatch } from '../src/index.ts';
import { installOwnershipTestHooks } from '../src/testing.ts';

installOwnershipTestHooks();

/** Assert a fatal, and clear the latch so the test can keep going. */
function expectFatal(body: () => unknown, message: string): void {
  expect(body).toThrow(message);
  clearFatalLatch();
}

/**
 * The shape `#[derive(Hash, PartialEq, Eq)]` gives a ported id: bytes, a hash
 * over them, and an equality that reads them rather than comparing identities.
 * Two of these built from the same bytes are one key in Rust and two keys in a
 * JavaScript Map, which is the whole reason this container exists.
 */
class Id extends Struct {
  readonly bytes: Uint8Array;
  constructor(...bytes: number[]) {
    super('Id');
    this.bytes = Uint8Array.from(bytes);
  }
  hash(): string { return this.bytes.join('.'); }
  equals(other: Id): boolean {
    if (this.bytes.length !== other.bytes.length) return false;
    return this.bytes.every((b, at) => b === other.bytes[at]);
  }
}

/** Two of these are never equal, and every one of them hashes the same — so
 *  every key lands in one bucket and only `equals` can tell them apart. */
class Colliding extends Struct {
  constructor(readonly n: number) { super('Colliding'); }
  hash(): string { return 'one bucket for all of them'; }
  equals(other: Colliding): boolean { return this.n === other.n; }
}

/** A droppable value, so a test can see what the map released. */
class Held extends Drop {
  dropCount = 0;
  constructor(readonly tag: string) { super(`Held(${tag})`); }
  protected override onDrop(): void { this.dropCount++; }
}

// A key built only to look an entry up is a temporary, and Rust drops one at
// the end of the statement that made it. Collecting them and releasing them at
// the end of the test is what keeps a lookup key from surfacing as a leak —
// and the guard is there because a temporary handed to `insert` over a present
// key is dropped by the map, which is the behaviour under test.
const temporaries: Array<{ drop(): void; isDropped: boolean }> = [];
afterEach(() => {
  for (const temp of temporaries.splice(0)) if (!temp.isDropped) temp.drop();
});
function probe<T extends { drop(): void; isDropped: boolean }>(key: T): T {
  temporaries.push(key);
  return key;
}
const anId = (...bytes: number[]): Id => probe(new Id(...bytes));
const aColliding = (n: number): Colliding => probe(new Colliding(n));

describe('HashMap keying', () => {
  test('two distinct keys equal by value find the same entry', () => {
    const map = new HashMap<Id, string>();
    map.set(new Id(1, 2, 3), 'peer A');
    // A second Id over the same bytes: a different object, and the same key.
    const lookup = anId(1, 2, 3);
    expect(map.has(lookup)).toBe(true);
    expect(map.get(lookup)).toBe('peer A');
    expect(map.size).toBe(1);
    // Overwriting through the second one replaces the value and keeps one entry.
    map.set(lookup, 'peer B');
    expect(map.size).toBe(1);
    expect(map.get(anId(1, 2, 3))).toBe('peer B');
    map.drop();
  });

  test('a key that differs by one byte is a different entry', () => {
    const map = new HashMap<Id, number>();
    map.set(new Id(1, 2, 3), 1);
    map.set(new Id(1, 2, 4), 2);
    expect(map.size).toBe(2);
    expect(map.get(anId(1, 2, 3))).toBe(1);
    expect(map.get(anId(1, 2, 4))).toBe(2);
    expect(map.get(anId(9, 9, 9))).toBeNull();
    map.drop();
  });

  test('keys sharing a hash are told apart by equals', () => {
    const map = new HashMap<Colliding, string>();
    for (let n = 0; n < 5; n++) map.set(new Colliding(n), `v${n}`);
    expect(map.size).toBe(5);
    for (let n = 0; n < 5; n++) expect(map.get(aColliding(n))).toBe(`v${n}`);
    expect(map.delete(aColliding(2))).toBe(true);
    expect(map.size).toBe(4);
    expect(map.get(aColliding(2))).toBeNull();
    expect(map.get(aColliding(3))).toBe('v3');
    map.drop();
  });

  test('primitive keys work unchanged, and the types do not share a key', () => {
    const map = new HashMap<unknown, string>();
    map.set('1', 'string');
    map.set(1, 'number');
    map.set(1n, 'bigint');
    map.set(true, 'boolean');
    map.set(null, 'null');
    expect(map.size).toBe(5);
    expect(map.get('1')).toBe('string');
    expect(map.get(1)).toBe('number');
    expect(map.get(1n)).toBe('bigint');
    expect(map.get(true)).toBe('boolean');
    expect(map.get(null)).toBe('null');
    expect(map.get(false)).toBeNull();
    map.drop();
  });

  test('a sequence key is hashed and compared element by element', () => {
    // How the port spells a tuple key — HashSet<(ReactorSubscriptionId, QueryId)>
    // — and a Vec<u8> key.
    const map = new HashMap<unknown, string>();
    map.set(['a', 1], 'tuple');
    map.set(Uint8Array.from([1, 2, 3]), 'bytes');
    expect(map.get(['a', 1])).toBe('tuple');
    expect(map.get(['a', 2])).toBeNull();
    expect(map.get(['a'])).toBeNull();
    expect(map.get(Uint8Array.from([1, 2, 3]))).toBe('bytes');
    expect(map.get(Uint8Array.from([1, 2]))).toBeNull();
    map.drop();
  });

  test('a nested key of ported ids compares through their equals', () => {
    const map = new HashMap<Id[], string>();
    map.set([new Id(1), new Id(2)], 'pair');
    expect(map.get([anId(1), anId(2)])).toBe('pair');
    expect(map.get([anId(2), anId(1)])).toBeNull();
    map.drop();
  });

  test('a key with no hash() is refused by name rather than keyed by identity', () => {
    class NoHash extends Struct {}
    const map = new HashMap<NoHash, number>();
    const key = probe(new NoHash());
    expect(() => map.set(key, 1)).toThrow('NoHash was used as a HashMap or HashSet key');
    // A refusal is not an ownership fatal: nothing was stored, so nothing is
    // corrupted and the map is still usable.
    expect(map.size).toBe(0);
    map.drop();
  });
});

describe('HashMap ownership', () => {
  test('dropping the map releases every key and every value', () => {
    const key = new Id(1);
    const value = new Held('v');
    const map = new HashMap<Id, Held>();
    map.set(key, value);
    map.drop();
    expect(value.dropCount).toBe(1);
    expect(key.isDropped).toBe(true);
  });

  test('insert hands back the displaced value and drops the surplus key', () => {
    const first = new Id(1);
    const second = new Id(1); // equal by value, so the map keeps the first
    const oldValue = new Held('old');
    const newValue = new Held('new');
    const map = new HashMap<Id, Held>();
    map.set(first, oldValue);

    const displaced = map.insert(second, newValue);
    // Rust's insert keeps the key it has and drops the one it was handed.
    expect(second.isDropped).toBe(true);
    expect(first.isDropped).toBe(false);
    // The displaced value is the caller's now — the map did not release it.
    expect(displaced).toBe(oldValue);
    expect(oldValue.dropCount).toBe(0);
    (displaced as Held).drop();

    map.drop();
    expect(newValue.dropCount).toBe(1);
    expect(first.isDropped).toBe(true);
  });

  test('set releases the value insert would have handed back', () => {
    // `map.insert(k, v);` as a statement in Rust drops the returned Option at
    // the end of that statement, which is what the emitted `.set(k, v)` means.
    const oldValue = new Held('old');
    const newValue = new Held('new');
    const map = new HashMap<string, Held>();
    map.set('k', oldValue);
    map.set('k', newValue);
    expect(oldValue.dropCount).toBe(1);
    expect(newValue.dropCount).toBe(0);
    map.drop();
    expect(newValue.dropCount).toBe(1);
  });

  test('remove hands back the value and drops the stored key', () => {
    const key = new Id(7);
    const value = new Held('v');
    const map = new HashMap<Id, Held>();
    map.set(key, value);

    const taken = map.remove(anId(7));
    expect(taken).toBe(value);
    expect(value.dropCount).toBe(0);
    expect(key.isDropped).toBe(true);
    expect(map.size).toBe(0);
    expect(map.remove(anId(7))).toBeNull();

    (taken as Held).drop();
    map.drop();
  });

  test('delete releases the key and the value both', () => {
    const key = new Id(7);
    const value = new Held('v');
    const map = new HashMap<Id, Held>();
    map.set(key, value);
    expect(map.delete(anId(7))).toBe(true);
    expect(key.isDropped).toBe(true);
    expect(value.dropCount).toBe(1);
    expect(map.delete(anId(7))).toBe(false);
    map.drop();
  });

  test('clear releases everything and leaves the map usable', () => {
    const values = [new Held('a'), new Held('b')];
    const map = new HashMap<string, Held>();
    map.set('a', values[0] as Held);
    map.set('b', values[1] as Held);
    map.clear();
    expect(values.map((v) => v.dropCount)).toEqual([1, 1]);
    expect(map.size).toBe(0);
    const after = new Held('c');
    map.set('c', after);
    map.drop();
    expect(after.dropCount).toBe(1);
  });

  test('handing the map back the value it already holds is fatal', () => {
    const value = new Held('v');
    const map = new HashMap<string, Held>();
    map.set('k', value);
    expectFatal(
      () => map.insert('k', value),
      'BUG: HashMap was assigned the value it already holds',
    );
    map.drop();
  });

  test('handing the map back the key it already holds is fatal', () => {
    const key = new Id(1);
    const map = new HashMap<Id, number>();
    map.set(key, 1);
    expectFatal(
      () => map.insert(key, 2),
      'BUG: HashMap key was assigned the value it already holds',
    );
    map.drop();
  });

  test('re-storing a Copy value over itself is legal', () => {
    // A Copy type has no drop glue, so re-storing one releases nothing — the
    // runtime tests for drop glue, not for "is it a primitive".
    const map = new HashMap<string, number>();
    map.set('k', 5);
    expect(map.insert('k', 5)).toBe(5);
    expect(map.get('k')).toBe(5);
    map.drop();
  });

  test('a second drop is fatal, and so is using a dropped map', () => {
    const map = new HashMap<string, number>(null, 'LabelledMap');
    map.drop();
    expectFatal(() => map.drop(), 'BUG: LabelledMap was dropped twice');
    expectFatal(() => map.get('k'), 'BUG: LabelledMap was used after being dropped');
  });

  test('the map is filled from an iterable of pairs, as new Map is', () => {
    const map = new HashMap<Id, number>([[new Id(1), 1], [new Id(2), 2]]);
    expect(map.size).toBe(2);
    expect(map.get(anId(2))).toBe(2);
    map.drop();
  });
});

describe('HashMap iteration', () => {
  test('entries, keys, values and spread all borrow', () => {
    const map = new HashMap<string, Held>();
    const a = new Held('a');
    const b = new Held('b');
    map.set('a', a);
    map.set('b', b);

    expect([...map.keys()].sort()).toEqual(['a', 'b']);
    expect([...map.values()]).toEqual([a, b]);
    expect([...map.entries()]).toEqual([['a', a], ['b', b]]);
    expect([...map]).toEqual([['a', a], ['b', b]]);
    // Iterating handed nothing over: the map still owns both values.
    expect([a.dropCount, b.dropCount]).toEqual([0, 0]);
    map.drop();
    expect([a.dropCount, b.dropCount]).toEqual([1, 1]);
  });

  test('deleting while iterating is safe — the shape retain is emitted as', () => {
    const map = new HashMap<number, Held>();
    const held = [0, 1, 2, 3].map((n) => new Held(String(n)));
    for (const [n, value] of held.entries()) map.set(n, value);
    // `map.retain(|k, _| k % 2 == 0)` becomes exactly this loop.
    for (const [k] of map) {
      if (k % 2 !== 0) map.delete(k);
    }
    expect(map.size).toBe(2);
    expect(held.map((h) => h.dropCount)).toEqual([0, 1, 0, 1]);
    map.drop();
    expect(held.map((h) => h.dropCount)).toEqual([1, 1, 1, 1]);
  });
});

describe('HashSet', () => {
  test('two distinct values equal by value are one member', () => {
    const set = new HashSet<Id>();
    expect(set.insert(new Id(1, 2))).toBe(true);
    expect(set.has(anId(1, 2))).toBe(true);
    expect(set.size).toBe(1);
    set.drop();
  });

  test('insert answers whether the value was new, and drops the surplus one', () => {
    const first = new Id(1);
    const duplicate = new Id(1);
    const set = new HashSet<Id>();
    expect(set.insert(first)).toBe(true);
    expect(set.insert(duplicate)).toBe(false);
    // Rust keeps the value it has and drops the one it was handed.
    expect(duplicate.isDropped).toBe(true);
    expect(first.isDropped).toBe(false);
    expect(set.size).toBe(1);
    set.drop();
    expect(first.isDropped).toBe(true);
  });

  test('add, delete, clear and size read the way the emitter writes them', () => {
    const set = new HashSet<string>();
    set.add('a').add('b');
    expect(set.size).toBe(2);
    expect(set.has('a')).toBe(true);
    expect(set.delete('a')).toBe(true);
    expect(set.delete('a')).toBe(false);
    expect([...set]).toEqual(['b']);
    set.clear();
    expect(set.size).toBe(0);
    set.drop();
  });

  test('remove releases the stored value', () => {
    const value = new Id(3);
    const set = new HashSet<Id>();
    set.add(value);
    expect(set.remove(anId(3))).toBe(true);
    expect(value.isDropped).toBe(true);
    set.drop();
  });

  test('dropping the set releases every value in it', () => {
    const values = [new Id(1), new Id(2)];
    const set = new HashSet<Id>(values);
    set.drop();
    expect(values.map((v) => v.isDropped)).toEqual([true, true]);
  });

  test('values, keys and spread all borrow', () => {
    const values = [new Id(1), new Id(2)];
    const set = new HashSet<Id>(values);
    expect([...set.values()]).toEqual(values);
    expect([...set.keys()]).toEqual(values);
    expect([...set]).toEqual(values);
    expect(values.map((v) => v.isDropped)).toEqual([false, false]);
    set.drop();
  });
});
