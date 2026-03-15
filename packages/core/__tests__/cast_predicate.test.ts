// Tests for value/cast_predicate.ts — mirrors ankurah/core/src/value/cast_predicate.rs #[cfg(test)]

import { describe, test, expect } from 'bun:test';
import { EntityId } from '@ankurah/proto';
import { Expr, Literal, Predicate, PathExpr, ComparisonOperator } from '@ankurah/ankql';
import { ValueType } from '../src/value/index.ts';
import { castPredicateTypes } from '../src/value/cast_predicate.ts';
import type { CollectionSchema } from '../src/schema.ts';

// Test schema implementation — mirrors Rust TestSchema
const testSchema: CollectionSchema = {
  fieldType(path: PathExpr): ValueType {
    // Use property name (last step) for type lookup
    const propertyName = path.property();
    switch (propertyName) {
      case 'id': return ValueType.EntityId;
      default: return ValueType.String;
    }
  },
};

describe('value/cast_predicate', () => {
  test('cast_id_field_string_to_entity_id', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();

    // Create a predicate: id = "base64_string"
    const predicate = Predicate.Comparison(
      Expr.Path(PathExpr.simple('id')),
      ComparisonOperator.Equal(),
      Expr.Literal(Literal.String(base64Str)),
    );

    const castPred = castPredicateTypes(predicate, testSchema);

    // Verify the string literal was cast to EntityId
    expect(castPred.is('Comparison')).toBe(true);
    const comp = castPred.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(comp.right.is('Literal')).toBe(true);
    const lit = (comp.right.value as { literal: Literal }).literal;
    expect(lit.type).toBe('EntityId');
    const litBytes = (lit.value as { value: Uint8Array }).value;
    expect(EntityId.fromBytes(litBytes).equals(entityId)).toBe(true);
  });

  test('cast_literal_equals_field', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();

    // Create a predicate: "base64_string" = id (literal on left side)
    const predicate = Predicate.Comparison(
      Expr.Literal(Literal.String(base64Str)),
      ComparisonOperator.Equal(),
      Expr.Path(PathExpr.simple('id')),
    );

    const castPred = castPredicateTypes(predicate, testSchema);

    // Verify the string literal was cast to EntityId
    expect(castPred.is('Comparison')).toBe(true);
    const comp = castPred.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(comp.left.is('Literal')).toBe(true);
    const lit = (comp.left.value as { literal: Literal }).literal;
    expect(lit.type).toBe('EntityId');
    const litBytes = (lit.value as { value: Uint8Array }).value;
    expect(EntityId.fromBytes(litBytes).equals(entityId)).toBe(true);
  });

  test('cast_complex_predicate', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();

    // Create a complex predicate: id = "base64_string" AND name = "test"
    const predicate = Predicate.And(
      Predicate.Comparison(
        Expr.Path(PathExpr.simple('id')),
        ComparisonOperator.Equal(),
        Expr.Literal(Literal.String(base64Str)),
      ),
      Predicate.Comparison(
        Expr.Path(PathExpr.simple('name')),
        ComparisonOperator.Equal(),
        Expr.Literal(Literal.String('test')),
      ),
    );

    const castPred = castPredicateTypes(predicate, testSchema);

    // Verify the casting worked correctly
    expect(castPred.is('And')).toBe(true);
    const andVal = castPred.value as { left: Predicate; right: Predicate };

    // Check id field was cast to EntityId
    expect(andVal.left.is('Comparison')).toBe(true);
    const leftComp = andVal.left.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(leftComp.right.is('Literal')).toBe(true);
    const idLit = (leftComp.right.value as { literal: Literal }).literal;
    expect(idLit.type).toBe('EntityId');
    const idBytes = (idLit.value as { value: Uint8Array }).value;
    expect(EntityId.fromBytes(idBytes).equals(entityId)).toBe(true);

    // Check name field remained as String
    expect(andVal.right.is('Comparison')).toBe(true);
    const rightComp = andVal.right.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(rightComp.right.is('Literal')).toBe(true);
    const nameLit = (rightComp.right.value as { literal: Literal }).literal;
    expect(nameLit.type).toBe('String');
    expect((nameLit.value as { value: string }).value).toBe('test');
  });
});
