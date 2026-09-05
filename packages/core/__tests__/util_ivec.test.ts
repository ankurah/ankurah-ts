// MIRRORS: ankurah/core/src/util/ivec.rs (tests module)
//
// The two Rust tests that assert the variant — `test_transition_to_large` and
// `test_contains_large` — assert the buffer's behaviour past N instead, because
// the port keeps no Small/Large split (see the header of util/ivec.ts). The last
// two tests have no Rust counterpart: they pin the ownership the port has to
// carry by hand, where Rust has `impl Drop` and a `false` branch that consumes.

import { describe, test, expect } from 'bun:test';
import { Arc, Struct } from '@ankurah/base';
import { IVec } from '../src/util/ivec.ts';

/** An element with drop glue, so the cascade has something to release. */
class Held extends Struct {
  constructor(readonly n: number) { super(); }
  equals(other: Held): boolean { return this.n === other.n; }
  clone(): Held { return new Held(this.n); }
}

describe('IVec', () => {
  test('test_small_push', () => {
    using ivec = IVec.new<number>();
    expect(ivec.len()).toBe(0);
    expect(ivec.isEmpty()).toBe(true);

    ivec.push(1);
    ivec.push(2);
    ivec.push(3);

    expect(ivec.len()).toBe(3);
    expect(ivec.isEmpty()).toBe(false);
  });

  test('test_transition_to_large', () => {
    using ivec = IVec.new<number>();
    for (const n of [1, 2, 3]) ivec.push(n);
    expect(ivec.len()).toBe(3);
    expect(ivec.asSlice()).toEqual([1, 2, 3]);
  });

  test('test_contains', () => {
    using ivec = IVec.new<number>();
    for (const n of [1, 2, 3]) ivec.push(n);

    expect(ivec.contains(1)).toBe(true);
    expect(ivec.contains(2)).toBe(true);
    expect(ivec.contains(3)).toBe(true);
    expect(ivec.contains(4)).toBe(false);
  });

  test('test_contains_large', () => {
    using ivec = IVec.new<number>();
    for (const n of [1, 2, 3, 4]) ivec.push(n);

    expect(ivec.contains(1)).toBe(true);
    expect(ivec.contains(4)).toBe(true);
    expect(ivec.contains(5)).toBe(false);
  });

  test('test_iter', () => {
    using ivec = IVec.new<number>();
    for (const n of [1, 2, 3]) ivec.push(n);
    expect([...ivec]).toEqual([1, 2, 3]);
    expect(ivec.iter()).toEqual([1, 2, 3]);
  });

  test('test_iter_large', () => {
    using ivec = IVec.new<number>();
    for (const n of [1, 2, 3, 4]) ivec.push(n);
    expect(ivec.iter()).toEqual([1, 2, 3, 4]);
  });

  test('test_drop', () => {
    // Rust: two Arcs pushed in, the buffer dropped, both counts back to 1.
    const ivec = IVec.new<Arc<number>>();
    const a1 = Arc.new(1);
    const a2 = Arc.new(2);

    ivec.push(a1.clone());
    ivec.push(a2.clone());

    expect(a1.strongCount).toBe(2);
    expect(a2.strongCount).toBe(2);

    ivec.drop();

    expect(a1.strongCount).toBe(1);
    expect(a2.strongCount).toBe(1);
    a1.drop();
    a2.drop();
  });

  test('test_add', () => {
    using ivec = IVec.new<number>();

    expect(ivec.add(1)).toBe(true);
    expect(ivec.add(2)).toBe(true);
    expect(ivec.add(3)).toBe(true);
    expect(ivec.len()).toBe(3);

    // Adding duplicate should return false
    expect(ivec.add(2)).toBe(false);
    expect(ivec.len()).toBe(3);

    expect(ivec.iter()).toEqual([1, 2, 3]);
  });

  test('test_add_large', () => {
    using ivec = IVec.new<number>();

    for (const n of [1, 2, 3]) expect(ivec.add(n)).toBe(true);

    // Adding a duplicate still returns false past N
    expect(ivec.add(1)).toBe(false);
    expect(ivec.add(2)).toBe(false);
    expect(ivec.len()).toBe(3);

    expect(ivec.add(4)).toBe(true);
    expect(ivec.len()).toBe(4);
  });

  // ── the port's own ownership, which Rust gets from the compiler ──

  test('dropping the buffer releases every element', () => {
    const ivec = IVec.new<Held>();
    const held = new Held(1);
    ivec.push(held);
    ivec.drop();
    expect(held.isDropped).toBe(true);
  });

  test('a refused add drops the value it refused', () => {
    using ivec = IVec.new<Held>();
    ivec.push(new Held(7));
    const duplicate = new Held(7);
    expect(ivec.add(duplicate)).toBe(false);
    // Rust's `false` branch takes `value` by value and lets it fall out of scope.
    expect(duplicate.isDropped).toBe(true);
    expect(ivec.len()).toBe(1);
  });

  test('clone copies the elements rather than sharing them', () => {
    const ivec = IVec.new<Held>();
    ivec.push(new Held(3));
    const copy = ivec.clone();
    expect(copy.len()).toBe(1);
    ivec.drop();
    // The clone's element is its own, so releasing the original left it alone.
    expect(copy.asSlice()[0].n).toBe(3);
    copy.drop();
  });
});
