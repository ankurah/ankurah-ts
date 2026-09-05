// MIRRORS: ankurah/core/src/reactor/comparison_index.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { ComparisonIndex } from './comparison_index';
import { Value } from '../value/index';
import { ComparisonOperator, Literal } from '@ankurah/ankql';
import { QueryId } from '@ankurah/proto';

describe('comparison_index unit tests', () => {
  test('test_field_index', () => {
    let index = ComparisonIndex.new();
    const sub0 = proto.QueryId.test(0n);
    index.add(new ast.Literal('I64', { _0: 8n }), new ComparisonOperator('LessThan', {}), sub0);
    expect(index.findMatching(new Value('I64', { _0: 8n }))).toEqual([]);
    expect(index.findMatching(new Value('I64', { _0: 7n }))).toEqual([sub0]);
    const sub1 = proto.QueryId.test(1n);
    index.add(new ast.Literal('I64', { _0: 20n }), new ComparisonOperator('GreaterThan', {}), sub1);
    expect(index.findMatching(new Value('I64', { _0: 20n }))).toEqual([]);
    expect(index.findMatching(new Value('I64', { _0: 21n }))).toEqual([sub1]);
    index.add(new ast.Literal('I64', { _0: 5n }), new ComparisonOperator('Equal', {}), sub0);
    expect(index.findMatching(new Value('I64', { _0: 5n }))).toEqual([sub0]);
    index.add(new ast.Literal('I64', { _0: 25n }), new ComparisonOperator('LessThan', {}), sub0);
    expect(index.findMatching(new Value('I64', { _0: 22n }))).toEqual([sub0, sub1]);
    expect(index.findMatching(new Value('I64', { _0: 25n }))).toEqual([sub1]);
    expect(index.findMatching(new Value('I64', { _0: 26n }))).toEqual([sub1]);
  });

  test('test_field_index_not_equal', () => {
    let index = ComparisonIndex.new();
    try {
      const sub0 = proto.QueryId.test(0n);
      index.add(new ast.Literal('I64', { _0: 8n }), new ComparisonOperator('NotEqual', {}), sub0);
      expect(index.findMatching(new Value('I64', { _0: 8n }))).toEqual([]);
      expect(index.findMatching(new Value('I64', { _0: 9n }))).toEqual([sub0]);
    } finally {
      index.drop();
    }
  });

});
