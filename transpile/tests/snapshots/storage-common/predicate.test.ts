// MIRRORS: ankurah/storage/common/src/predicate.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { ConjunctFinder } from './predicate';
import { dropOwned } from '@ankurah/base';
import { Predicate } from '@ankurah/ankql';

describe('predicate unit tests', () => {
  test('test_single_comparison', () => {
    const predicate = undefined /* selection!("age > 25") */.predicate;
    const conjuncts = ConjunctFinder.find(predicate);
    try {
      expect(conjuncts.length).toEqual(1);
      expect(conjuncts[0]).toEqual(predicate);
    } finally {
      dropOwned(conjuncts);
    }
  });

  test('test_simple_and', () => {
    const predicate = undefined /* selection!("age > 25 AND name = 'Alice'") */.predicate;
    const conjuncts = ConjunctFinder.find(predicate);
    try {
      expect(conjuncts.length).toEqual(2);
      expect(conjuncts[0]).toEqual(undefined /* selection!("age > 25") */.predicate);
      expect(conjuncts[1]).toEqual(undefined /* selection!("name = 'Alice'") */.predicate);
    } finally {
      dropOwned(conjuncts);
    }
  });

  test('test_nested_and', () => {
    const predicate = undefined /* selection!("(age > 25 AND name = 'Alice') AND score < 100") */.predicate;
    const conjuncts = ConjunctFinder.find(predicate);
    try {
      expect(conjuncts.length).toEqual(3);
      expect(conjuncts[0]).toEqual(undefined /* selection!("age > 25") */.predicate);
      expect(conjuncts[1]).toEqual(undefined /* selection!("name = 'Alice'") */.predicate);
      expect(conjuncts[2]).toEqual(undefined /* selection!("score < 100") */.predicate);
    } finally {
      dropOwned(conjuncts);
    }
  });

  test('test_or_blocks_conjunct_extraction', () => {
    const predicate = undefined /* selection!("age > 25 OR name = 'Alice'") */.predicate;
    const conjuncts = ConjunctFinder.find(predicate);
    try {
      expect(conjuncts.length).toEqual(1);
      expect(conjuncts[0]).toEqual(predicate);
    } finally {
      dropOwned(conjuncts);
    }
  });

  test('test_and_with_or_mixed', () => {
    const predicate = undefined /* selection!("score = 100 AND (age > 25 OR name = 'Alice')") */.predicate;
    const conjuncts = ConjunctFinder.find(predicate);
    try {
      expect(conjuncts.length).toEqual(2);
      expect(conjuncts[0]).toEqual(undefined /* selection!("score = 100") */.predicate);
      expect(conjuncts[1]).toEqual(undefined /* selection!("age > 25 OR name = 'Alice'") */.predicate);
    } finally {
      dropOwned(conjuncts);
    }
  });

  test('test_complex_nested_example', () => {
    const predicate = undefined /* selection!("(foo = 1 AND bar = 2) AND (baz = 3 OR zed = 4)") */.predicate;
    const conjuncts = ConjunctFinder.find(predicate);
    try {
      expect(conjuncts.length).toEqual(3);
      expect(conjuncts[0]).toEqual(undefined /* selection!("foo = 1") */.predicate);
      expect(conjuncts[1]).toEqual(undefined /* selection!("bar = 2") */.predicate);
      expect(conjuncts[2]).toEqual(undefined /* selection!("baz = 3 OR zed = 4") */.predicate);
    } finally {
      dropOwned(conjuncts);
    }
  });

  test('test_non_comparison_predicates', () => {
    const predicate = undefined /* selection!("age > 25 AND score = 100") */.predicate;
    const conjuncts = ConjunctFinder.find(predicate);
    try {
      expect(conjuncts.length).toEqual(2);
      expect(conjuncts[0]).toEqual(undefined /* selection!("age > 25") */.predicate);
      expect(conjuncts[1]).toEqual(undefined /* selection!("score = 100") */.predicate);
    } finally {
      dropOwned(conjuncts);
    }
  });

  test('test_true_false_predicates', () => {
    const conjuncts = ConjunctFinder.find(new Predicate('True', {}));
    try {
      expect(conjuncts.length).toEqual(1);
      expect(conjuncts[0]).toEqual(new Predicate('True', {}));
      const conjuncts_1 = ConjunctFinder.find(new Predicate('False', {}));
      try {
        expect(conjuncts_1.length).toEqual(1);
        expect(conjuncts_1[0]).toEqual(new Predicate('False', {}));
      } finally {
        dropOwned(conjuncts_1);
      }
    } finally {
      dropOwned(conjuncts);
    }
  });

});
