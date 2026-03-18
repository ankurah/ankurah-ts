// MIRRORS: ankurah/storage/common/src/predicate.rs #[cfg(test)]

import { describe, expect, test } from 'bun:test';
import { parseSelection, Predicate, Selection } from '@ankurah/ankql';
import { ConjunctFinder } from './predicate.ts';

/** Parse a selection string and return the predicate. Equivalent of Rust `selection!("...")`. */
function sel(input: string): Predicate {
  return parseSelection(input).predicate;
}

/** Compare two predicates via their canonical string form. */
function predStr(p: Predicate): string {
  return Selection.fromPredicate(p).toString();
}

describe('ConjunctFinder', () => {
  // Rust: fn test_single_comparison
  test('test_single_comparison', () => {
    const predicate = sel('age > 25');
    const conjuncts = ConjunctFinder.find(predicate);
    expect(conjuncts.length).toBe(1);
    expect(predStr(conjuncts[0])).toBe(predStr(predicate));
  });

  // Rust: fn test_simple_and
  test('test_simple_and', () => {
    const predicate = sel("age > 25 AND name = 'Alice'");
    const conjuncts = ConjunctFinder.find(predicate);
    expect(conjuncts.length).toBe(2);

    // Should extract both comparisons
    expect(predStr(conjuncts[0])).toBe(predStr(sel('age > 25')));
    expect(predStr(conjuncts[1])).toBe(predStr(sel("name = 'Alice'")));
  });

  // Rust: fn test_nested_and
  test('test_nested_and', () => {
    const predicate = sel("(age > 25 AND name = 'Alice') AND score < 100");
    const conjuncts = ConjunctFinder.find(predicate);
    expect(conjuncts.length).toBe(3);

    // Should flatten all AND operations
    expect(predStr(conjuncts[0])).toBe(predStr(sel('age > 25')));
    expect(predStr(conjuncts[1])).toBe(predStr(sel("name = 'Alice'")));
    expect(predStr(conjuncts[2])).toBe(predStr(sel('score < 100')));
  });

  // Rust: fn test_or_blocks_conjunct_extraction
  test('test_or_blocks_conjunct_extraction', () => {
    const predicate = sel("age > 25 OR name = 'Alice'");
    const conjuncts = ConjunctFinder.find(predicate);
    expect(conjuncts.length).toBe(1);

    // The entire OR should be treated as a single conjunct
    expect(predStr(conjuncts[0])).toBe(predStr(predicate));
  });

  // Rust: fn test_and_with_or_mixed
  test('test_and_with_or_mixed', () => {
    const predicate = sel("score = 100 AND (age > 25 OR name = 'Alice')");
    const conjuncts = ConjunctFinder.find(predicate);
    expect(conjuncts.length).toBe(2);

    // Should extract the equality and treat the OR as a single conjunct
    expect(predStr(conjuncts[0])).toBe(predStr(sel('score = 100')));
    expect(predStr(conjuncts[1])).toBe(predStr(sel("age > 25 OR name = 'Alice'")));
  });

  // Rust: fn test_complex_nested_example
  test('test_complex_nested_example', () => {
    // Example from documentation: (foo = 1 AND bar = 2) AND (baz = 3 OR zed = 4)
    const predicate = sel('(foo = 1 AND bar = 2) AND (baz = 3 OR zed = 4)');
    const conjuncts = ConjunctFinder.find(predicate);
    expect(conjuncts.length).toBe(3);

    // foo = 1 and bar = 2 should be extracted as conjuncts
    expect(predStr(conjuncts[0])).toBe(predStr(sel('foo = 1')));
    expect(predStr(conjuncts[1])).toBe(predStr(sel('bar = 2')));
    // The OR should remain as a single conjunct
    expect(predStr(conjuncts[2])).toBe(predStr(sel('baz = 3 OR zed = 4')));
  });

  // Rust: fn test_non_comparison_predicates
  test('test_non_comparison_predicates', () => {
    // Test with a simple AND of two comparisons since IS NULL isn't supported by selection! macro
    const predicate = sel('age > 25 AND score = 100');
    const conjuncts = ConjunctFinder.find(predicate);
    expect(conjuncts.length).toBe(2);

    expect(predStr(conjuncts[0])).toBe(predStr(sel('age > 25')));
    expect(predStr(conjuncts[1])).toBe(predStr(sel('score = 100')));
  });

  // Rust: fn test_true_false_predicates
  test('test_true_false_predicates', () => {
    // Test with Predicate::True and Predicate::False
    let conjuncts = ConjunctFinder.find(Predicate.True());
    expect(conjuncts.length).toBe(1);
    expect(conjuncts[0].is('True')).toBe(true);

    conjuncts = ConjunctFinder.find(Predicate.False());
    expect(conjuncts.length).toBe(1);
    expect(conjuncts[0].is('False')).toBe(true);
  });
});
