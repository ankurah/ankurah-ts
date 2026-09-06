import { describe, expect, test } from 'bun:test';
import { HashMap, HashSet, keysEqual, valueEquals, valueNotEquals } from '../src/index.ts';

class Tag {
  constructor(readonly n: number) {}
  equals(other: Tag): boolean {
    return this.n === other.n;
  }
  hash(): string {
    return `Tag:${this.n}`;
  }
  drop(): void {}
}

class Opaque {
  constructor(readonly n: number) {}
  drop(): void {}
}

describe('== compares contents, as Rust does', () => {
  test('primitives keep ===', () => {
    expect(valueEquals(1, 1)).toBe(true);
    expect(valueEquals(1, 2)).toBe(false);
    expect(valueEquals('a', 'a')).toBe(true);
    expect(valueEquals(null, null)).toBe(true);
    expect(valueEquals(null, 1)).toBe(false);
    expect(valueNotEquals(1, 2)).toBe(true);
  });

  // `bytes == [0u8; 16]` in `collatable` was `===` between two arrays:
  // identity, so the branch could never be taken.
  test('two sequences compare element by element, bytes included', () => {
    expect(valueEquals([1, 2, 3], [1, 2, 3])).toBe(true);
    expect(valueEquals([1, 2], [1, 2, 3])).toBe(false);
    expect(valueEquals(new Uint8Array(16), new Array(16).fill(0))).toBe(true);
    expect(valueEquals(new Uint8Array([1]), new Uint8Array([2]))).toBe(false);
  });

  test('a value that declares equals() is asked', () => {
    expect(valueEquals(new Tag(1), new Tag(1))).toBe(true);
    expect(valueEquals(new Tag(1), new Tag(2))).toBe(false);
    expect(valueEquals([new Tag(1)], [new Tag(1)])).toBe(true);
  });

  // Rust's `==` needs a PartialEq impl, so this is a comparison the port wrote
  // where Rust would not have compiled one. Answering `false` would hide it.
  test('two values that declare no equals() raise rather than answering false', () => {
    expect(() => valueEquals(new Opaque(1), new Opaque(1))).toThrow(/declares an equals/);
    // One comparable side settles it without reaching a member.
    expect(valueEquals(new Opaque(1), null)).toBe(false);
    expect(valueEquals(new Opaque(1), 5)).toBe(false);
    expect(valueEquals(null, new Opaque(1))).toBe(false);
  });

  // E5. The refusal used to be skipped where EITHER side could be compared, so
  // one comparable operand hid the other: both of these answered `false` and
  // neither raised — exactly the shape the refusal exists to catch.
  test('a pair with only one comparable side raises, in both orders', () => {
    expect(() => valueEquals(new Opaque(1), new Tag(1))).toThrow(/declares an equals/);
    expect(() => valueEquals(new Tag(1), new Opaque(1))).toThrow(/declares an equals/);
  });

  // E5's other half. The map LOOKUP walk is a different question from `==`: it
  // asks "is this the key I stored", and a key that answers no is an absent key
  // rather than a defect. It consulted only the LEFT operand's `equals`, so
  // which side of the call a value arrived on decided the answer. Both sides
  // are the same key type in any real map, so nothing live changes — the
  // order-dependence goes.
  test('the lookup walk asks either side, so the order does not decide it', () => {
    expect(keysEqual(new Opaque(1), new Tag(1))).toBe(true);
    expect(keysEqual(new Tag(1), new Opaque(1))).toBe(true);
    expect(keysEqual(new Opaque(1), new Tag(2))).toBe(false);
  });

  // F5. The refusal reached the two OUTER operands only: the element-wise step
  // went through the lookup walk, so `valueEquals({}, {})` raised and
  // `valueEquals([{}], [{}])` answered `false`.
  test('the refusal reaches every pair the walk compares', () => {
    expect(() => valueEquals([new Opaque(1)], [new Opaque(1)])).toThrow(/declares an equals/);
    expect(() => valueEquals([[new Opaque(1)]], [[new Opaque(1)]])).toThrow(/declares an equals/);
    expect(() => valueEquals([new Tag(1), new Opaque(1)], [new Tag(1), new Opaque(1)])).toThrow(
      /declares an equals/,
    );
    // A length difference is answered before any element is reached.
    expect(valueEquals([new Opaque(1)], [])).toBe(false);
    // And a sequence of comparable values still compares.
    expect(valueEquals([[new Tag(1)]], [[new Tag(1)]])).toBe(true);
  });
});

describe('the runtime containers compare by contents', () => {
  test('two sets are equal when they hold the same elements, in any order', () => {
    const a = HashSet.from(['x', 'y']);
    const b = HashSet.from(['y', 'x']);
    expect(valueEquals(a, b)).toBe(true);
    b.add('z');
    expect(valueEquals(a, b)).toBe(false);
    a.drop();
    b.drop();
  });

  test('two maps are equal when every key maps to an equal value', () => {
    const a = HashMap.from<string, Tag>([['k', new Tag(1)]]);
    const b = HashMap.from<string, Tag>([['k', new Tag(1)]]);
    expect(valueEquals(a, b)).toBe(true);
    const c = HashMap.from<string, Tag>([['k', new Tag(2)]]);
    expect(valueEquals(a, c)).toBe(false);
    const d = HashMap.from<string, Tag>([['other', new Tag(1)]]);
    expect(valueEquals(a, d)).toBe(false);
    for (const m of [a, b, c, d]) m.drop();
  });

  // E19. `HashMap.equals` compared its VALUES with the lookup walk, which
  // answers `false` for a value declaring no equals() where `==` on the same
  // pair raises. `impl PartialEq for HashMap<K, V>` carries `V: PartialEq`.
  test('a map value that declares no equals() raises rather than differing quietly', () => {
    const a = HashMap.from<string, Opaque>([['k', new Opaque(1)]]);
    const b = HashMap.from<string, Opaque>([['k', new Opaque(1)]]);
    expect(() => valueEquals(a, b)).toThrow(/declares an equals/);
    a.drop();
    b.drop();
  });

  test('a set and a map of different kinds are not equal', () => {
    const s = HashSet.from(['x']);
    const m = HashMap.from<string, number>([['x', 1]]);
    expect(valueEquals(s, m)).toBe(false);
    s.drop();
    m.drop();
  });
});
