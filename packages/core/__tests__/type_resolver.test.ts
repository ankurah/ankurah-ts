// MIRRORS: ankurah/core/src/type_resolver.rs #[cfg(test)]

import { describe, test, expect } from 'bun:test';
import { Expr, Literal, PathExpr, Predicate, ComparisonOperator } from '@ankurah/ankql';
import { ValueType } from '../src/value/index.ts';
import { TypeResolver } from '../src/type_resolver.ts';

describe('TypeResolver', () => {
  test('resolve_simple_path', () => {
    const resolver = TypeResolver.new();
    const path = PathExpr.simple('name');
    expect(resolver.resolvePath(path)).toBe(null);
  });

  test('resolve_id_path', () => {
    const resolver = TypeResolver.new();
    const path = PathExpr.simple('id');
    expect(resolver.resolvePath(path)).toBe(ValueType.EntityId);
  });

  test('resolve_json_path', () => {
    const resolver = TypeResolver.new();
    const path = new PathExpr(['data', 'number']);
    expect(resolver.resolvePath(path)).toBe(ValueType.Json);
  });

  test('literal_to_json_string', () => {
    const lit = Literal.String('hello');
    const jsonLit = TypeResolver.literalToJson(lit);
    expect(jsonLit.type).toBe('Json');
    const jsonVal = (jsonLit.value as { value: unknown }).value;
    expect(jsonVal).toBe('hello');
  });

  test('literal_to_json_number', () => {
    const lit = Literal.I64(BigInt(42));
    const jsonLit = TypeResolver.literalToJson(lit);
    expect(jsonLit.type).toBe('Json');
    const jsonVal = (jsonLit.value as { value: unknown }).value;
    expect(jsonVal).toBe(42);
  });

  test('resolve_types_converts_literal_for_json_path', () => {
    const resolver = TypeResolver.new();

    // data.number = 9 → literal should be converted to Json
    const predicate = Predicate.Comparison(
      Expr.Path(new PathExpr(['data', 'number'])),
      ComparisonOperator.Equal(),
      Expr.Literal(Literal.I64(BigInt(9))),
    );

    const resolved = resolver.resolveTypes(predicate);

    // Check that the literal was converted to Json
    expect(resolved.is('Comparison')).toBe(true);
    const comp = resolved.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(comp.right.is('Literal')).toBe(true);
    const rightLit = (comp.right.value as { literal: Literal }).literal;
    expect(rightLit.type).toBe('Json');
    const jsonVal = (rightLit.value as { value: unknown }).value;
    expect(jsonVal).toBe(9);
  });

  test('resolve_types_leaves_simple_path_literal_alone', () => {
    const resolver = TypeResolver.new();

    // name = "test" → literal should NOT be converted (simple path)
    const predicate = Predicate.Comparison(
      Expr.Path(PathExpr.simple('name')),
      ComparisonOperator.Equal(),
      Expr.Literal(Literal.String('test')),
    );

    const resolved = resolver.resolveTypes(predicate);

    // Check that the literal was NOT converted
    expect(resolved.is('Comparison')).toBe(true);
    const comp = resolved.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(comp.right.is('Literal')).toBe(true);
    const rightLit = (comp.right.value as { literal: Literal }).literal;
    expect(rightLit.type).toBe('String');
    expect((rightLit.value as { value: string }).value).toBe('test');
  });
});
