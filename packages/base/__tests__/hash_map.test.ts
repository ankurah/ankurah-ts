// TS-ONLY: Tests for the value-keyed HashMap and HashSet (src/std/hash_map.ts).
import { describe, test, expect, afterEach } from 'bun:test';
import { HashMap, HashSet, Struct, Drop, clearFatalLatch, keyHash, derivedEquals, derivedClone, derivedHash } from '../src/index.ts';
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

/**
 * A key that reads its own bytes to hash and to compare, and reads them through
 * the liveness check.
 *
 * `Id` above answers both from fields it can still see after a drop, so a key
 * used after being released answers correctly by luck. This one does not: it is
 * how a `#[derive(Hash)]` on a type with a droppable field behaves, and it is
 * what caught an entry Slot keyed by the lookup key it had already released.
 */
class CheckedKey extends Struct {
  #bytes: Uint8Array;
  constructor(...bytes: number[]) {
    super('CheckedKey');
    this.#bytes = Uint8Array.from(bytes);
  }
  get bytes(): Uint8Array {
    this.assertNotDropped();
    return this.#bytes;
  }
  hash(): string { return this.bytes.join('.'); }
  equals(other: CheckedKey): boolean {
    const mine = this.bytes;
    const theirs = other.bytes;
    return mine.length === theirs.length && mine.every((b, at) => b === theirs[at]);
  }
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

// C18: `map.entry(k)` is the one place Rust's map API takes the key BEFORE it
// knows whether it needs it, and `*map.entry(k).or_insert(0) += 1` is how the
// corpus counts. Emitted against a map with no `entry`, that was a TypeError;
// emitted against a plain `BorrowMut`, the increment landed on a copy and the
// map never changed.
describe('entry', () => {
  test('or_insert puts a value there and hands back a slot into the map', () => {
    const map = new HashMap<string, number>();
    map.entry('a').orInsert(0).value += 1;
    map.entry('a').orInsert(0).value += 1;
    map.entry('b').orInsert(7);
    expect(map.get('a')).toBe(2);
    expect(map.get('b')).toBe(7);
    expect(map.size).toBe(2);
    map.drop();
  });

  test('an occupied entry releases the key it was handed and the value it did not use', () => {
    const map = new HashMap<Id, Held>();
    map.set(new Id([1]), new Held('first'));

    // The map keeps the key it already has, so the one passed in is released —
    // and so is the value `or_insert` was given and did not need.
    const surplusKey = new Id([1]);
    const surplusValue = new Held('unused');
    expect(map.entry(surplusKey).orInsert(surplusValue).value.tag).toBe('first');
    expect(surplusKey.isDropped).toBe(true);
    expect(surplusValue.dropCount).toBe(1);
    expect(map.size).toBe(1);
    map.drop();
  });

  test('or_insert_with calls the thunk only where there is nothing there', () => {
    const map = new HashMap<string, number>();
    let calls = 0;
    const make = () => {
      calls += 1;
      return 5;
    };
    expect(map.entry('a').orInsertWith(make).value).toBe(5);
    expect(map.entry('a').orInsertWith(make).value).toBe(5);
    expect(calls).toBe(1);
    // `or_default` is the same, with the thunk standing in for `V: Default`.
    expect(map.entry('b').orDefault(() => 0).value).toBe(0);
    map.drop();
  });

  // X4: the occupied path releases the LOOKUP key, and the Slot it handed back
  // held that same released key — so its first read hashed a dropped value.
  test('an occupied entry hands back a slot on the stored entry, not the released key', () => {
    const map = new HashMap<CheckedKey, Held>();
    map.set(new CheckedKey(1), new Held('first'));

    const lookup = new CheckedKey(1);
    const slot = map.entry(lookup).orInsert(new Held('unused'));
    expect(lookup.isDropped).toBe(true);
    // Reading and writing through the slot reach the map, and neither of them
    // touches the key the entry released.
    expect(slot.value.tag).toBe('first');
    slot.value = new Held('second');
    expect(map.get(probe(new CheckedKey(1)))?.tag).toBe('second');
    map.drop();
  });

  // X5: the vacant path called the factory before the map took the key, so a
  // factory that throws left the key with nobody. Rust's unwind drops it.
  test('a vacant orInsertWith releases its key when the factory throws', () => {
    const map = new HashMap<CheckedKey, Held>();
    const key = new CheckedKey(2);
    expect(() => map.entry(key).orInsertWith(() => { throw new Error('boom'); }))
      .toThrow('boom');
    expect(key.isDropped).toBe(true);
    expect(map.size).toBe(0);
    map.drop();
  });

  test('writing through the slot releases what the map held', () => {
    const map = new HashMap<string, Held>();
    const first = new Held('first');
    map.set('k', first);
    map.entry('k').orInsert(new Held('unused')).value = new Held('second');
    expect(first.dropCount).toBe(1);
    expect(map.get('k')?.tag).toBe('second');
    map.drop();
  });
});

// X6: `clone()` walked a value's own surface, and an array and a typed array
// have no `clone()` — so a cloned map handed its copy the very same array, both
// maps owned one set of elements, and dropping both dropped each element twice.
describe('clone walks what a container holds', () => {
  test('an array value is cloned element by element', () => {
    // The element carries a `clone()`, which is what `#[derive(Clone)]` on the
    // containing type requires of it; the ARRAY is what had none, and what both
    // maps used to share.
    class Piece extends Drop {
      dropCount = 0;
      constructor(readonly tag: string) { super(`Piece(${tag})`); }
      clone(): Piece { return new Piece(this.tag); }
      protected override onDrop(): void { this.dropCount++; }
    }
    const map = new HashMap<string, Piece[]>();
    const piece = new Piece('one');
    map.set('k', [piece]);

    const copy = map.clone();
    const theirs = copy.get('k') as Piece[];
    expect(theirs).not.toBe(map.get('k'));
    expect(theirs[0]).not.toBe(piece);
    map.drop();
    // The original's element went with it; the copy's is its own and is still
    // alive, so dropping the copy releases a different object.
    expect(piece.dropCount).toBe(1);
    expect((theirs[0] as Piece).dropCount).toBe(0);
    copy.drop();
    expect(piece.dropCount).toBe(1);
    expect((theirs[0] as Piece).dropCount).toBe(1);
  });

  test('a typed array value is copied, not shared', () => {
    const map = new HashMap<string, Uint8Array>();
    map.set('k', Uint8Array.from([1, 2, 3]));
    const copy = map.clone();
    const mine = map.get('k') as Uint8Array;
    const theirs = copy.get('k') as Uint8Array;
    expect(theirs).not.toBe(mine);
    theirs[0] = 9;
    expect(mine[0]).toBe(1);
    map.drop();
    copy.drop();
  });

  test('a set of arrays is cloned the same way', () => {
    const set = new HashSet<number[]>();
    set.insert([1, 2]);
    const copy = set.clone();
    expect(copy.has([1, 2])).toBe(true);
    set.drop();
    copy.drop();
  });
});

describe('from', () => {
  test('HashMap.from and HashSet.from build the keyed containers', () => {
    const map = HashMap.from([['a', 1], ['b', 2]] as const);
    expect(map.size).toBe(2);
    expect(map.get('b')).toBe(2);
    map.drop();

    const set = HashSet.from(['a', 'b', 'a']);
    expect(set.size).toBe(2);
    set.drop();
  });
});

describe('the bucket label of a sequence', () => {
  // A1.11's collision, one level down. Each part carries its own LENGTH, so no
  // separator can be forged out of the parts themselves: joining with a comma
  // made `['a', 'b']` and `['a,s:b']` one label, which is a `Vec<String>` field
  // of a derived key colliding with a single string that spells the join.
  test('two sequences are one label only when they hold the same things', () => {
    expect(keyHash(['a', 'b'])).not.toBe(keyHash(['a,s:b']));
    expect(keyHash(['a', 'b'])).toBe(keyHash(['a', 'b']));
    expect(keyHash(['ab'])).not.toBe(keyHash(['a', 'b']));
    expect(keyHash([1, 2])).not.toBe(keyHash(['n:1,n:2']));
    // A nested sequence is a part like any other, and carries its own length.
    expect(keyHash([['a'], ['b']])).not.toBe(keyHash([['a', 'b']]));
  });

  test('and a map keys by it, so two such keys are two entries', () => {
    const map = new HashMap<string[], number>();
    map.set(['a', 'b'], 1);
    map.set(['a,s:b'], 2);
    expect(map.size).toBe(2);
    expect(map.get(['a', 'b'])).toBe(1);
    expect(map.get(['a,s:b'])).toBe(2);
    map.drop();
  });
});

describe('a field written as a type parameter', () => {
  // `T` is a number in one instantiation and a class in another, and the
  // emitter cannot tell which: `.equals()` and `.clone()` on a number are both
  // TypeErrors, so the derived comparison and the derived copy decide by the
  // value's own surface at run time.
  test('a primitive is compared by identity and copied by being read', () => {
    expect(derivedEquals(1, 1)).toBe(true);
    expect(derivedEquals(1, 2)).toBe(false);
    expect(derivedEquals('a', 'a')).toBe(true);
    expect(derivedClone(1)).toBe(1);
    expect(derivedClone('a')).toBe('a');
  });

  test('a sequence is compared and copied element by element', () => {
    expect(derivedEquals([1, 2], [1, 2])).toBe(true);
    expect(derivedEquals([1, 2], [1, 3])).toBe(false);
    expect(derivedClone([1, 2])).toEqual([1, 2]);
  });

  test('an object answers its own equals and its own clone', () => {
    const a = new Id(new Uint8Array([1, 2]));
    const b = new Id(new Uint8Array([1, 2]));
    expect(derivedEquals(a, b)).toBe(true);
    a.drop();
    b.drop();

    class Counted extends Struct {
      constructor(readonly n: number) { super(); }
      clone(): Counted { return new Counted(this.n); }
      equals(other: Counted): boolean { return this.n === other.n; }
    }
    const one = new Counted(1);
    const copy = derivedClone(one);
    expect(copy).not.toBe(one);
    expect(derivedEquals(copy, one)).toBe(true);
    one.drop();
    copy.drop();
  });

  // The third half of the derive. `#[derive(Hash)]` on a generic carries
  // `T: Hash`, and the emitter cannot know which instantiation is in front of
  // it: `hash()` on a number is a TypeError, and on the very path a `HashMap`
  // takes to file a key.
  test('a value of a type parameter is hashed by its own surface', () => {
    expect(derivedHash(7)).toBe(keyHash(7));
    expect(derivedHash('a')).toBe(keyHash('a'));
    expect(derivedHash([1, 2])).toBe(keyHash([1, 2]));
    const a = new Id(new Uint8Array([1, 2]));
    const b = new Id(new Uint8Array([1, 2]));
    expect(derivedHash(a)).toBe(derivedHash(b));
    a.drop();
    b.drop();
  });

  test('and one declaring neither is REFUSED, because Rust\'s bound excludes it', () => {
    class Bare {}
    expect(() => derivedEquals(new Bare(), new Bare())).toThrow('declares no equals()');
    expect(() => derivedClone(new Bare())).toThrow('declares no clone()');
    expect(() => derivedHash(new Bare())).toThrow('declares no hash()');
  });
});

// Z9: a clone that throws part-way used to leave everything it had already
// cloned — and, for a map, the half-built destination too — owned by nobody.
// Nothing in the emitted code ever received them, so nothing releases them and
// the leak check reports each one.
// N6: `for (k, v) in map` moves the map into its `IntoIter`, which hands out an
// owned pair each turn and drops what it never handed out. Nothing in the
// runtime could do that: emptying a map released what was in it, so the emitted
// loop released every key and value and left the container to the collector.
describe('intoEntries hands the pairs over and consumes the map', () => {
  test('the pairs come out and the map is gone', () => {
    const map = new HashMap<Id, Id>();
    const key = new Id(new Uint8Array([1]));
    const value = new Id(new Uint8Array([2]));
    map.insert(key, value);
    const pairs = map.intoEntries();
    expect(pairs.length).toBe(1);
    expect(pairs[0]![0]).toBe(key);
    expect(pairs[0]![1]).toBe(value);
    // The map is moved: a use after this is the run-time spelling of the move
    // Rust refuses at compile time, and a second drop is a double drop.
    expectFatal(() => map.get(key), 'used after being dropped');
    expectFatal(() => map.drop(), 'dropped twice');
    // The pairs are the caller's now, and nothing else released them.
    key.drop();
    value.drop();
  });

  test('an empty map is consumed too', () => {
    const map = new HashMap<Id, Id>();
    expect(map.intoEntries()).toEqual([]);
    expectFatal(() => map.drop(), 'dropped twice');
  });
});

describe('a container clone that throws leaves nothing behind', () => {
  /** A value whose `clone()` throws once a set number of clones have been made. */
  class Fragile extends Struct {
    /** Every clone this class has handed out, so a test can ask what became of them. */
    static made: Fragile[] = [];
    static failAfter = Infinity;
    constructor(readonly n: number) { super(`Fragile(${n})`); }
    hash(): string { return String(this.n); }
    equals(other: Fragile): boolean { return other.n === this.n; }
    clone(): Fragile {
      if (Fragile.made.length >= Fragile.failAfter) throw new Error('clone failed');
      const copy = new Fragile(this.n);
      Fragile.made.push(copy);
      return copy;
    }
  }

  /** Every clone made before the throw was released, and there were some. */
  function everyCloneWasReleased(count: number): void {
    expect(Fragile.made.length).toBe(count);
    expect(Fragile.made.filter((c) => !c.isDropped)).toEqual([]);
  }

  test('a sequence releases the elements it had already cloned', () => {
    Fragile.made = [];
    Fragile.failAfter = 2;
    const source = [new Fragile(1), new Fragile(2), new Fragile(3)];
    expect(() => derivedClone(source)).toThrow('clone failed');
    everyCloneWasReleased(2);
    for (const element of source) element.drop();
  });

  test('a map releases the pairs it had already cloned, and builds no map at all', () => {
    Fragile.made = [];
    Fragile.failAfter = 3;
    const map = new HashMap<Fragile, Fragile>();
    map.set(new Fragile(1), new Fragile(10));
    map.set(new Fragile(2), new Fragile(20));
    expect(() => map.clone()).toThrow('clone failed');
    everyCloneWasReleased(3);
    map.drop();
  });

  test('a set releases the values it had already cloned', () => {
    Fragile.made = [];
    Fragile.failAfter = 1;
    const set = new HashSet<Fragile>();
    set.add(new Fragile(1));
    set.add(new Fragile(2));
    expect(() => set.clone()).toThrow('clone failed');
    everyCloneWasReleased(1);
    set.drop();
  });
});
