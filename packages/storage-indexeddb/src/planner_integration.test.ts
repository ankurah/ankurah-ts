// MIRRORS: ankurah/storage/indexeddb-wasm/src/planner_integration.rs #[cfg(test)]

import { describe, expect, test } from 'bun:test';
import { KeyBounds, KeyBoundComponent, Endpoint, ScanDirection, Plan, OrderByComponents } from '@ankurah/storage-common';
import { ValueType, keySpecNew, indexKeyPartAsc, keySpecNameWith } from '@ankurah/core';
import type { Value, KeySpec } from '@ankurah/core';
import { Predicate } from '@ankurah/ankql';
import { normalize, planBoundsToIdbRangeSyntax, scanDirectionToCursorDirection } from './planner_integration.ts';

describe('PlannerIntegration', () => {
  test('test_plan_index_spec_name', () => {
    const indexSpec = keySpecNew([
      indexKeyPartAsc('__collection', ValueType.String),
      indexKeyPartAsc('age', ValueType.I32),
      indexKeyPartAsc('score', ValueType.I32),
    ]);

    const plan = Plan.Index(
      indexSpec,
      ScanDirection.Forward(),
      new KeyBounds([]),
      Predicate.True(),
      OrderByComponents.default(),
    );

    plan.match({
      Index: (data) => {
        const indexName = keySpecNameWith(data.indexSpec, '', '__');
        expect(indexName).toBe('__collection asc__age asc__score asc');
      },
      TableScan: () => { throw new Error('unexpected'); },
      EmptyScan: () => { throw new Error('unexpected'); },
    });
  });

  test('test_scan_direction_to_cursor_direction', () => {
    const ascDirection = scanDirectionToCursorDirection(ScanDirection.Forward());
    const descDirection = scanDirectionToCursorDirection(ScanDirection.Reverse());

    // Verify the directions are different and correct
    expect(ascDirection).not.toBe(descDirection);
    expect(ascDirection).toBe('next');
    expect(descDirection).toBe('prev');
  });

  test('test_normalize_equality_only', () => {
    // Test normalization of equality-only bounds: __collection = 'album' AND age = 30
    // With the new bounded range logic, this should create [album, 30] to [album, 31)
    const bounds = new KeyBounds([
      new KeyBoundComponent(
        '__collection',
        Endpoint.incl({ type: 'String', value: 'album' }),
        Endpoint.incl({ type: 'String', value: 'album' }),
      ),
      new KeyBoundComponent(
        'age',
        Endpoint.incl({ type: 'I32', value: 30 }),
        Endpoint.incl({ type: 'I32', value: 30 }),
      ),
    ]);

    const [canonicalRange, eqPrefixLen, eqPrefixValues] = normalize(bounds);

    // Should have both values in equality prefix
    expect(eqPrefixLen).toBe(2);
    expect(eqPrefixValues).toEqual([
      { type: 'String', value: 'album' },
      { type: 'I32', value: 30 },
    ]);

    // Lower bound is inclusive [album, 30], upper bound is exclusive [album, 31)
    expect(canonicalRange.lower).not.toBeNull();
    expect(canonicalRange.lower![0]).toEqual([
      { type: 'String', value: 'album' },
      { type: 'I32', value: 30 },
    ]);
    expect(canonicalRange.lower![1]).toBe(false); // closed lower

    expect(canonicalRange.upper).not.toBeNull();
    expect(canonicalRange.upper![0]).toEqual([
      { type: 'String', value: 'album' },
      { type: 'I32', value: 31 },
    ]);
    expect(canonicalRange.upper![1]).toBe(true); // open upper (exclusive)
  });

  test('test_normalize_with_inequality', () => {
    // Test normalization with inequality: __collection = 'album' AND age > 25
    const bounds = new KeyBounds([
      new KeyBoundComponent(
        '__collection',
        Endpoint.incl({ type: 'String', value: 'album' }),
        Endpoint.incl({ type: 'String', value: 'album' }),
      ),
      new KeyBoundComponent(
        'age',
        Endpoint.excl({ type: 'I32', value: 25 }),
        Endpoint.UnboundedHigh(ValueType.I32),
      ),
    ]);

    const [canonicalRange, eqPrefixLen, eqPrefixValues] = normalize(bounds);

    // Should have one equality in prefix
    expect(eqPrefixLen).toBe(1);
    expect(eqPrefixValues).toEqual([
      { type: 'String', value: 'album' },
    ]);

    // Lower bound should include equality + inequality
    expect(canonicalRange.lower).not.toBeNull();
    expect(canonicalRange.lower![0]).toEqual([
      { type: 'String', value: 'album' },
      { type: 'I32', value: 25 },
    ]);
    expect(canonicalRange.lower![1]).toBe(true); // open because > 25

    // Upper bound should be null (open-ended)
    expect(canonicalRange.upper).toBeNull();
  });

  // test_plan_bounds_to_idb_range is gated with #[cfg(target_arch = "wasm32")]
  // and requires IDBKeyRange which needs a browser environment.
  // The normalize logic it tests is covered by the above tests.

  test('test_plan_bounds_to_idb_range_syntax', () => {
    // Test the syntax generation for bounds from the debug print
    const bounds = new KeyBounds([
      new KeyBoundComponent(
        '__collection',
        Endpoint.incl({ type: 'String', value: 'connectionevent' }),
        Endpoint.incl({ type: 'String', value: 'connectionevent' }),
      ),
      new KeyBoundComponent(
        'user_id',
        Endpoint.incl({ type: 'String', value: 'AZoegTHj_4vcBoJ5FfY-Xw' }),
        Endpoint.incl({ type: 'String', value: 'AZoegTHj_4vcBoJ5FfY-Xw' }),
      ),
      new KeyBoundComponent(
        'timestamp',
        Endpoint.excl({ type: 'I64', value: 1761455267792 }),
        Endpoint.excl({ type: 'I64', value: 1761456167793 }),
      ),
    ]);

    const jsSyntax = planBoundsToIdbRangeSyntax(bounds);

    // Should generate IDBKeyRange.bound with raw i64 numbers (matches From<&Value>)
    expect(jsSyntax).toContain('IDBKeyRange.bound');
    expect(jsSyntax).toContain('"connectionevent"');
    expect(jsSyntax).toContain('"AZoegTHj_4vcBoJ5FfY-Xw"');
    // i64 values as raw numbers
    expect(jsSyntax).toContain('1761455267792');
    expect(jsSyntax).toContain('1761456167793');
    expect(jsSyntax).toContain('true, true'); // Both bounds are exclusive
  });

  test('test_plan_bounds_to_idb_range_syntax_equality_only', () => {
    // Test with equality-only bounds on a single column
    // With the new bounded range logic, single equality becomes bound(["album"], ["album\u0000"], false, true)
    const bounds = new KeyBounds([
      new KeyBoundComponent(
        '__collection',
        Endpoint.incl({ type: 'String', value: 'album' }),
        Endpoint.incl({ type: 'String', value: 'album' }),
      ),
    ]);

    const jsSyntax = planBoundsToIdbRangeSyntax(bounds);

    // Single equality now becomes a bounded range with next_upper_bound
    expect(jsSyntax).toContain('IDBKeyRange.bound');
    expect(jsSyntax).toContain('"album"');
  });

  test('test_plan_bounds_to_idb_range_syntax_multi_equality', () => {
    // Test with equality on multiple columns
    // With the new bounded range logic, this becomes bound(["album", "2000"], ["album", "2000\u0000"], false, true)
    const bounds = new KeyBounds([
      new KeyBoundComponent(
        '__collection',
        Endpoint.incl({ type: 'String', value: 'album' }),
        Endpoint.incl({ type: 'String', value: 'album' }),
      ),
      new KeyBoundComponent(
        'year',
        Endpoint.incl({ type: 'String', value: '2000' }),
        Endpoint.incl({ type: 'String', value: '2000' }),
      ),
    ]);

    const jsSyntax = planBoundsToIdbRangeSyntax(bounds);

    // Multiple equalities now use bound() with next_upper_bound on the last column
    expect(jsSyntax).toContain('IDBKeyRange.bound');
    expect(jsSyntax).toContain('"album"');
    expect(jsSyntax).toContain('"2000"');
  });
});
