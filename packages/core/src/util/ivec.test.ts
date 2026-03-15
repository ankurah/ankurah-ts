// MIRRORS: ankurah/core/src/util/ivec.rs #[cfg(test)]

import { describe, test, expect } from 'bun:test';
import { IVec } from './ivec.ts';

describe('IVec', () => {
  test('test_small_push', () => {
    const ivec = new IVec<number>();
    expect(ivec.len()).toBe(0);
    expect(ivec.isEmpty()).toBe(true);

    ivec.push(1);
    ivec.push(2);
    ivec.push(3);

    expect(ivec.len()).toBe(3);
    expect(ivec.isEmpty()).toBe(false);
  });

  test('test_transition_to_large', () => {
    // Divergence: No Small/Large transition in JS — test just verifies push works past N [E7].
    const ivec = new IVec<number>();
    ivec.push(1);
    ivec.push(2);
    ivec.push(3);
    expect(ivec.len()).toBe(3);
  });

  test('test_contains', () => {
    const ivec = new IVec<number>();
    ivec.push(1);
    ivec.push(2);
    ivec.push(3);

    expect(ivec.contains(1)).toBe(true);
    expect(ivec.contains(2)).toBe(true);
    expect(ivec.contains(3)).toBe(true);
    expect(ivec.contains(4)).toBe(false);
  });

  test('test_contains_large', () => {
    const ivec = new IVec<number>();
    ivec.push(1);
    ivec.push(2);
    ivec.push(3);
    ivec.push(4);

    expect(ivec.contains(1)).toBe(true);
    expect(ivec.contains(4)).toBe(true);
    expect(ivec.contains(5)).toBe(false);
  });

  test('test_iter', () => {
    const ivec = new IVec<number>();
    ivec.push(1);
    ivec.push(2);
    ivec.push(3);

    expect(ivec.iter()).toEqual([1, 2, 3]);
  });

  test('test_iter_large', () => {
    const ivec = new IVec<number>();
    ivec.push(1);
    ivec.push(2);
    ivec.push(3);
    ivec.push(4);

    expect(ivec.iter()).toEqual([1, 2, 3, 4]);
  });

  // test_drop: JS GC handles memory — no equivalent needed.

  test('test_add', () => {
    const ivec = new IVec<number>();

    expect(ivec.add(1)).toBe(true);
    expect(ivec.add(2)).toBe(true);
    expect(ivec.add(3)).toBe(true);
    expect(ivec.len()).toBe(3);

    // Adding duplicate should return false
    expect(ivec.add(2)).toBe(false);
    expect(ivec.len()).toBe(3);

    // Verify contents
    expect(ivec.iter()).toEqual([1, 2, 3]);
  });

  test('test_add_large', () => {
    const ivec = new IVec<number>();

    expect(ivec.add(1)).toBe(true);
    expect(ivec.add(2)).toBe(true);
    expect(ivec.add(3)).toBe(true);

    // Adding duplicate should return false
    expect(ivec.add(1)).toBe(false);
    expect(ivec.add(2)).toBe(false);
    expect(ivec.len()).toBe(3);

    // Can still add new items
    expect(ivec.add(4)).toBe(true);
    expect(ivec.len()).toBe(4);
  });
});
