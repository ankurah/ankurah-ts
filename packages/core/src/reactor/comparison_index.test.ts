// MIRRORS: ankurah/core/src/reactor/comparison_index.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { ComparisonIndex } from './comparison_index';
import { dropOwned } from '@ankurah/base';
import { Value } from '../value/index';
import { ComparisonOperator, Literal } from '@ankurah/ankql';
import { QueryId } from '@ankurah/proto';

describe('comparison_index unit tests', () => {
  test('test_field_index', () => {
    let index = ComparisonIndex.new();
    try {
      const sub0 = QueryId.test(0n);
      let _moved1 = false;
      const _b0 = new ast.Literal('I64', { _0: 8n });
      try {
        const _b2 = new ComparisonOperator('LessThan', {});
        _moved1 = true;
        index.add(_b0, _b2, sub0);
      } finally {
        if (!_moved1) dropOwned(_b0);
      }
      expect(index.findMatching(new Value('I64', { _0: 8n }))).toEqual([]);
      expect(index.findMatching(new Value('I64', { _0: 7n }))).toEqual([sub0]);
      const sub1 = QueryId.test(1n);
      let _moved4 = false;
      const _b3 = new ast.Literal('I64', { _0: 20n });
      try {
        const _b5 = new ComparisonOperator('GreaterThan', {});
        _moved4 = true;
        index.add(_b3, _b5, sub1);
      } finally {
        if (!_moved4) dropOwned(_b3);
      }
      expect(index.findMatching(new Value('I64', { _0: 20n }))).toEqual([]);
      expect(index.findMatching(new Value('I64', { _0: 21n }))).toEqual([sub1]);
      let _moved7 = false;
      const _b6 = new ast.Literal('I64', { _0: 5n });
      try {
        const _b8 = new ComparisonOperator('Equal', {});
        _moved7 = true;
        index.add(_b6, _b8, sub0);
      } finally {
        if (!_moved7) dropOwned(_b6);
      }
      expect(index.findMatching(new Value('I64', { _0: 5n }))).toEqual([sub0]);
      let _moved10 = false;
      const _b9 = new ast.Literal('I64', { _0: 25n });
      try {
        const _b11 = new ComparisonOperator('LessThan', {});
        _moved10 = true;
        index.add(_b9, _b11, sub0);
      } finally {
        if (!_moved10) dropOwned(_b9);
      }
      expect(index.findMatching(new Value('I64', { _0: 22n }))).toEqual([sub0, sub1]);
      expect(index.findMatching(new Value('I64', { _0: 25n }))).toEqual([sub1]);
      expect(index.findMatching(new Value('I64', { _0: 26n }))).toEqual([sub1]);
    } finally {
      index.drop();
    }
  });

  test('test_field_index_not_equal', () => {
    let index = ComparisonIndex.new();
    try {
      const sub0 = QueryId.test(0n);
      let _moved1 = false;
      const _b0 = new ast.Literal('I64', { _0: 8n });
      try {
        const _b2 = new ComparisonOperator('NotEqual', {});
        _moved1 = true;
        index.add(_b0, _b2, sub0);
      } finally {
        if (!_moved1) dropOwned(_b0);
      }
      expect(index.findMatching(new Value('I64', { _0: 8n }))).toEqual([]);
      expect(index.findMatching(new Value('I64', { _0: 9n }))).toEqual([sub0]);
    } finally {
      index.drop();
    }
  });

});
