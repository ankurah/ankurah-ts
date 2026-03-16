// MIRRORS: ankurah/core/src/selection/filter.rs #[cfg(test)] mod tests
import { describe, test, expect } from 'bun:test';
import { parseSelection } from '@ankurah/ankql';
import type { Value } from '../value/index.ts';
import type { Filterable, FilterResult } from './filter.ts';
import { filterIterator, FilterError } from './filter.ts';
import { TypeResolver } from '../type_resolver.ts';

// ── Test helpers ──

class TestItem implements Filterable {
  readonly name: string;
  readonly age: string;

  constructor(name: string, age: string) {
    this.name = name;
    this.age = age;
  }

  collection(): string { return 'users'; }

  value(name: string): Value | null {
    switch (name) {
      case 'name': return { type: 'String', value: this.name };
      case 'age': return { type: 'String', value: this.age };
      default: return null;
    }
  }
}

function collectResults<R extends Filterable>(items: R[], predicateStr: string): FilterResult<R>[] {
  const selection = parseSelection(predicateStr);
  return Array.from(filterIterator(items, selection.predicate));
}

// ── Tests ──

describe('filter', () => {
  // Rust: fn test_simple_equality()
  test('simple equality', () => {
    const items = [new TestItem('Alice', '30'), new TestItem('Bob', '25'), new TestItem('Charlie', '35')];
    const results = collectResults(items, "name = 'Alice'");

    expect(results.length).toBe(3);
    expect(results[0].type).toBe('Pass');
    expect(results[1].type).toBe('Skip');
    expect(results[2].type).toBe('Skip');
  });

  // Rust: fn test_and_condition()
  test('and condition', () => {
    const items = [new TestItem('Alice', '30'), new TestItem('Bob', '30'), new TestItem('Charlie', '35')];
    const results = collectResults(items, "name = 'Alice' AND age = '30'");

    expect(results.length).toBe(3);
    expect(results[0].type).toBe('Pass');
    expect(results[1].type).toBe('Skip');
    expect(results[2].type).toBe('Skip');
  });

  // Rust: fn test_complex_condition()
  test('complex condition', () => {
    const items = [
      new TestItem('Alice', '20'),
      new TestItem('Bob', '25'),
      new TestItem('Charlie', '30'),
      new TestItem('David', '35'),
      new TestItem('Eve', '40'),
    ];
    const results = collectResults(items, "(name = 'Alice' OR name = 'Charlie') AND age >= '30' AND age <= '40'");

    expect(results.length).toBe(5);
    expect(results[0].type).toBe('Skip');  // Alice age 20
    expect(results[1].type).toBe('Skip');  // Bob
    expect(results[2].type).toBe('Pass');  // Charlie age 30
    expect(results[3].type).toBe('Skip');  // David
    expect(results[4].type).toBe('Skip');  // Eve
  });

  // Rust: fn test_in_operator()
  test('in operator', () => {
    const items = [
      new TestItem('Alice', '20'),
      new TestItem('Bob', '25'),
      new TestItem('Charlie', '30'),
      new TestItem('David', '35'),
      new TestItem('Eve', '40'),
    ];

    // Test IN with names
    const results1 = collectResults(items, "name IN ('Alice', 'Charlie', 'Eve')");
    expect(results1[0].type).toBe('Pass');  // Alice
    expect(results1[1].type).toBe('Skip');  // Bob
    expect(results1[2].type).toBe('Pass');  // Charlie
    expect(results1[3].type).toBe('Skip');  // David
    expect(results1[4].type).toBe('Pass');  // Eve

    // Test IN with ages
    const results2 = collectResults(items, "age IN ('20', '30', '40')");
    expect(results2[0].type).toBe('Pass');  // 20
    expect(results2[1].type).toBe('Skip');  // 25
    expect(results2[2].type).toBe('Pass');  // 30
    expect(results2[3].type).toBe('Skip');  // 35
    expect(results2[4].type).toBe('Pass');  // 40
  });
});

// ── JSON path traversal tests ──

describe('filter json_tests', () => {
  class TrackItem implements Filterable {
    readonly name: string;
    readonly licensing: unknown; // JSON value

    constructor(name: string, licensing: unknown) {
      this.name = name;
      this.licensing = licensing;
    }

    collection(): string { return 'tracks'; }

    value(name: string): Value | null {
      switch (name) {
        case 'name': return { type: 'String', value: this.name };
        case 'licensing': {
          // Store as Binary (matching Rust which uses serde_json::to_vec)
          const bytes = new TextEncoder().encode(JSON.stringify(this.licensing));
          return { type: 'Binary', value: bytes };
        }
        default: return null;
      }
    }
  }

  // Rust: fn test_simple_json_path()
  test('simple json path', () => {
    const items = [
      new TrackItem('Track A', { territory: 'US', rights: 'exclusive' }),
      new TrackItem('Track B', { territory: 'UK', rights: 'non-exclusive' }),
      new TrackItem('Track C', { territory: 'US', rights: 'non-exclusive' }),
    ];

    const results = collectResults(items, "licensing.territory = 'US'");
    expect(results[0].type).toBe('Pass');
    expect(results[1].type).toBe('Skip');
    expect(results[2].type).toBe('Pass');
  });

  // Rust: fn test_nested_json_path()
  test('nested json path', () => {
    const items = [
      new TrackItem('Track A', { rights: { holder: 'Label A', type: 'exclusive' } }),
      new TrackItem('Track B', { rights: { holder: 'Label B', type: 'non-exclusive' } }),
    ];

    const results = collectResults(items, "licensing.rights.holder = 'Label A'");
    expect(results[0].type).toBe('Pass');
    expect(results[1].type).toBe('Skip');
  });

  // Rust: fn test_json_path_with_numeric_value()
  test('json path with numeric value', () => {
    const items = [
      new TrackItem('Track A', { duration: 180, bpm: 120 }),
      new TrackItem('Track B', { duration: 240, bpm: 140 }),
    ];

    const results = collectResults(items, 'licensing.duration > 200');
    expect(results[0].type).toBe('Skip');
    expect(results[1].type).toBe('Pass');
  });

  // Rust: fn test_json_path_with_boolean()
  test('json path with boolean', () => {
    const items = [
      new TrackItem('Track A', { active: true }),
      new TrackItem('Track B', { active: false }),
    ];

    const results = collectResults(items, 'licensing.active = true');
    expect(results[0].type).toBe('Pass');
    expect(results[1].type).toBe('Skip');
  });

  // Rust: fn test_json_path_not_found()
  test('json path not found', () => {
    const items = [new TrackItem('Track A', { territory: 'US' })];

    const results = collectResults(items, "licensing.nonexistent = 'value'");
    expect(results[0].type).toBe('Error');
  });

  // Rust: fn test_json_path_combined_with_regular_field()
  test('json path combined with regular field', () => {
    const items = [
      new TrackItem('Track A', { territory: 'US' }),
      new TrackItem('Track B', { territory: 'US' }),
      new TrackItem('Track C', { territory: 'UK' }),
    ];

    const results = collectResults(items, "name = 'Track A' AND licensing.territory = 'US'");
    expect(results[0].type).toBe('Pass');
    expect(results[1].type).toBe('Skip');
    expect(results[2].type).toBe('Skip');
  });

  // Rust: fn test_traverse_into_non_json_property_errors()
  test('traverse into non-json property errors', () => {
    const items = [new TestItem('Alice', '30')];

    const results = collectResults(items, "name.nested = 'value'");
    expect(results[0].type).toBe('Error');
    if (results[0].type === 'Error') {
      expect(results[0].error.kind).toBe('PropertyNotFound');
    }
  });

  // Rust: fn test_json_path_with_or()
  test('json path with or', () => {
    const items = [
      new TrackItem('Track A', { status: 'active', region: 'US' }),
      new TrackItem('Track B', { status: 'pending', region: 'UK' }),
      new TrackItem('Track C', { status: 'archived', region: 'US' }),
    ];

    const results = collectResults(items, "licensing.status = 'active' OR licensing.region = 'UK'");
    expect(results[0].type).toBe('Pass');  // active
    expect(results[1].type).toBe('Pass');  // UK
    expect(results[2].type).toBe('Skip');  // neither
  });

  // Rust: fn test_json_path_with_in_operator()
  test('json path with in operator', () => {
    const items = [
      new TrackItem('Track A', { status: 'active' }),
      new TrackItem('Track B', { status: 'pending' }),
      new TrackItem('Track C', { status: 'archived' }),
    ];

    const results = collectResults(items, "licensing.status IN ('active', 'pending')");
    expect(results[0].type).toBe('Pass');
    expect(results[1].type).toBe('Pass');
    expect(results[2].type).toBe('Skip');
  });

  // Rust: fn test_collection_qualified_json_path()
  test('collection qualified json path', () => {
    const items = [
      new TrackItem('Track A', { territory: 'US' }),
      new TrackItem('Track B', { territory: 'UK' }),
    ];

    const results = collectResults(items, "tracks.licensing.territory = 'US'");
    expect(results[0].type).toBe('Pass');
    expect(results[1].type).toBe('Skip');
  });

  // Rust: fn test_regular_field_still_casts_string_to_number()
  test('regular field still casts string to number', () => {
    const items = [new TestItem('Alice', '30')];

    // Regular field with string value, queried with number
    const results = collectResults(items, 'age = 30');

    // Should pass - general casting allows string '30' to match integer 30
    expect(results[0].type).toBe('Pass');
  });
});

// ── JSON type casting tests (require TypeResolver) ──

describe('filter json_type_casting', () => {
  class TrackItem implements Filterable {
    readonly name: string;
    readonly licensing: unknown;

    constructor(name: string, licensing: unknown) {
      this.name = name;
      this.licensing = licensing;
    }

    collection(): string { return 'tracks'; }

    value(name: string): Value | null {
      switch (name) {
        case 'name': return { type: 'String', value: this.name };
        case 'licensing': {
          const bytes = new TextEncoder().encode(JSON.stringify(this.licensing));
          return { type: 'Binary', value: bytes };
        }
        default: return null;
      }
    }
  }

  // Rust: fn parse_with_types(query: &str) -> Selection
  function parseWithTypes(query: string) {
    const selection = parseSelection(query);
    return new TypeResolver().resolveSelectionTypes(selection);
  }

  function collectResultsTyped<R extends Filterable>(items: R[], predicateStr: string): FilterResult<R>[] {
    const selection = parseWithTypes(predicateStr);
    return Array.from(filterIterator(items, selection.predicate));
  }

  // Rust: fn test_json_numeric_casting_same_type()
  test('json numeric casting same type', () => {
    // JSON numbers matching literal numbers should work
    const items = [new TrackItem('Track A', { count: 42 })];

    const results = collectResultsTyped(items, 'licensing.count = 42');
    expect(results[0].type).toBe('Pass');
  });

  // Rust: fn test_json_numeric_casting_float_to_int()
  test('json numeric casting float to int', () => {
    // JSON float should match integer literal (numeric family casting)
    const items = [new TrackItem('Track A', { count: 42.5 })]; // Float in JSON

    // Query with integer - should match via numeric casting
    const results = collectResultsTyped(items, 'licensing.count > 42');
    expect(results[0].type).toBe('Pass'); // 42.5 > 42
  });

  // Rust: fn test_json_string_to_number_no_cast()
  test('json string to number no cast', () => {
    // JSON string "42" should NOT match integer literal 42
    // (JSON-aware casting only allows numeric family, not string->number)
    const items = [new TrackItem('Track A', { count: '42' })]; // String, not number

    // Type resolver converts literal 42 to Json(42) for JSON path comparison
    const results = collectResultsTyped(items, 'licensing.count = 42');

    // Should NOT pass - no string->number casting for JSON
    expect(results[0].type).toBe('Skip');
  });

  // Rust: fn test_json_number_to_string_no_cast()
  test('json number to string no cast', () => {
    // JSON number 42 should NOT match string literal '42'
    const items = [new TrackItem('Track A', { count: 42 })]; // Number, not string

    // Type resolver converts literal '42' to Json("42") for JSON path comparison
    const results = collectResultsTyped(items, "licensing.count = '42'");

    // Should NOT pass - no number->string casting for JSON
    expect(results[0].type).toBe('Skip');
  });

  // Rust: fn test_json_string_equality_works()
  test('json string equality works', () => {
    // JSON string matching string literal should work
    const items = [new TrackItem('Track A', { status: 'active' })];

    // Type resolver converts literal to Json for JSON path comparison
    const results = collectResultsTyped(items, "licensing.status = 'active'");
    expect(results[0].type).toBe('Pass');
  });

  // Rust: fn test_json_comparison_operators()
  test('json comparison operators', () => {
    // Test numeric comparisons work correctly
    const items = [
      new TrackItem('A', { score: 50 }),
      new TrackItem('B', { score: 100 }),
      new TrackItem('C', { score: 150 }),
    ];

    // > operator (type resolver converts literals to Json)
    let results = collectResultsTyped(items, 'licensing.score > 100');
    expect(results[0].type).toBe('Skip');
    expect(results[1].type).toBe('Skip');
    expect(results[2].type).toBe('Pass');

    // >= operator
    results = collectResultsTyped(items, 'licensing.score >= 100');
    expect(results[0].type).toBe('Skip');
    expect(results[1].type).toBe('Pass');
    expect(results[2].type).toBe('Pass');

    // < operator
    results = collectResultsTyped(items, 'licensing.score < 100');
    expect(results[0].type).toBe('Pass');
    expect(results[1].type).toBe('Skip');
    expect(results[2].type).toBe('Skip');
  });
});
