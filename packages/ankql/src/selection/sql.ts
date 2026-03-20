// MIRRORS: ankurah/ankql/src/selection/sql.rs

import { Expr, Literal, Predicate, ComparisonOperator } from '../ast.ts';
import {
  SqlGenerationError,
} from '../error.ts';

// Rust: fn generate_expr_sql
function generateExprSql(
  expr: Expr,
  expectedCount: number | null,
  foundPlaceholders: { count: number },
  buffer: string[],
): void {
  expr.match({
    Placeholder: () => {
      foundPlaceholders.count++;
      if (expectedCount !== null && foundPlaceholders.count > expectedCount) {
        throw new SqlGenerationError('PlaceholderCountMismatch', { expected: expectedCount, found: foundPlaceholders.count });
      }
      buffer.push('?');
    },
    Literal: (v) => {
      buffer.push(literalToSql(v.literal));
    },
    Path: (v) => {
      buffer.push(v.path.steps.map((s) => `"${s}"`).join('.'));
    },
    ExprList: (v) => {
      buffer.push('(');
      for (let i = 0; i < v.exprs.length; i++) {
        if (i > 0) buffer.push(', ');
        const item = v.exprs[i];
        if (item.is('Placeholder')) {
          foundPlaceholders.count++;
          if (expectedCount !== null && foundPlaceholders.count > expectedCount) {
            throw new SqlGenerationError('PlaceholderCountMismatch', { expected: expectedCount, found: foundPlaceholders.count });
          }
          buffer.push('?');
        } else if (item.is('Literal')) {
          buffer.push(literalToSql((item.value as { literal: Literal }).literal));
        } else {
          throw new SqlGenerationError('InvalidExpression', {
            _0: 'Only literal expressions and placeholders are supported in IN lists',
          });
        }
      }
      buffer.push(')');
    },
    Predicate: () => {
      throw new SqlGenerationError('InvalidExpression', {
        _0: 'Only literal, identifier, and list expressions are supported',
      });
    },
    InfixExpr: () => {
      throw new SqlGenerationError('InvalidExpression', {
        _0: 'Only literal, identifier, and list expressions are supported',
      });
    },
  });
}

function literalToSql(lit: Literal): string {
  return lit.match({
    I16: (v) => String(v.value),
    I32: (v) => String(v.value),
    I64: (v) => String(v.value),
    F64: (v) => String(v.value),
    Bool: (v) => v.value ? 'true' : 'false',
    String: (v) => {
      let escaped = '';
      for (const ch of v.value) {
        if (ch === "'") {
          escaped += "''";
        } else if (ch === '\0') {
          // Skip null bytes for safety
          continue;
        } else {
          escaped += ch;
        }
      }
      return `'${escaped}'`;
    },
    EntityId: (v) => {
      // Base64url encode the ULID bytes
      const bytes = v.value;
      let binary = '';
      for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i]);
      }
      const b64 = btoa(binary)
        .replace(/\+/g, '-')
        .replace(/\//g, '_')
        .replace(/=+$/, '');
      return `'${b64}'`;
    },
    Object: (v) => {
      const decoded = new TextDecoder().decode(v.value);
      return `'${decoded}'`;
    },
    Binary: (v) => {
      const decoded = new TextDecoder().decode(v.value);
      return `'${decoded}'`;
    },
    Json: (v) => `'${JSON.stringify(v.value)}'`,
  });
}

// Rust: fn comparison_op_to_sql
function comparisonOpToSql(op: ComparisonOperator): string {
  return op.match({
    Equal: () => '=',
    NotEqual: () => '<>',
    GreaterThan: () => '>',
    GreaterThanOrEqual: () => '>=',
    LessThan: () => '<',
    LessThanOrEqual: () => '<=',
    In: () => 'IN',
    Between: () => { throw new SqlGenerationError('UnsupportedOperator', { _0: 'BETWEEN operator is not yet supported' }); },
  });
}

// Rust: fn generate_selection_sql_inner
function generatePredicateSql(
  predicate: Predicate,
  expectedCount: number | null,
  foundPlaceholders: { count: number },
  buffer: string[],
): void {
  predicate.match({
    Comparison: (v) => {
      generateExprSql(v.left, expectedCount, foundPlaceholders, buffer);
      buffer.push(' ');
      buffer.push(comparisonOpToSql(v.operator));
      buffer.push(' ');
      generateExprSql(v.right, expectedCount, foundPlaceholders, buffer);
    },
    And: (v) => {
      generatePredicateSql(v.left, expectedCount, foundPlaceholders, buffer);
      buffer.push(' AND ');
      generatePredicateSql(v.right, expectedCount, foundPlaceholders, buffer);
    },
    Or: (v) => {
      buffer.push('(');
      generatePredicateSql(v.left, expectedCount, foundPlaceholders, buffer);
      buffer.push(' OR ');
      generatePredicateSql(v.right, expectedCount, foundPlaceholders, buffer);
      buffer.push(')');
    },
    Not: (v) => {
      buffer.push('NOT (');
      generatePredicateSql(v.predicate, expectedCount, foundPlaceholders, buffer);
      buffer.push(')');
    },
    IsNull: (v) => {
      generateExprSql(v.expr, expectedCount, foundPlaceholders, buffer);
      buffer.push(' IS NULL');
    },
    True: () => {
      buffer.push('TRUE');
    },
    False: () => {
      buffer.push('FALSE');
    },
    Placeholder: () => {
      throw new SqlGenerationError('InvalidExpression', {
        _0: 'Placeholder must be transformed before SQL generation',
      });
    },
  });
}

// Rust: fn generate_selection_sql
/**
 * Generate SQL from a Predicate AST.
 * @param predicate The predicate to convert to SQL
 * @param expectedPlaceholders If provided, validates the number of ? placeholders matches
 */
export function generateSelectionSql(
  predicate: Predicate,
  expectedPlaceholders?: number,
): string {
  const expectedCount = expectedPlaceholders ?? null;
  const foundPlaceholders = { count: 0 };
  const buffer: string[] = [];

  generatePredicateSql(predicate, expectedCount, foundPlaceholders, buffer);

  // Verify placeholder count matches expected
  if (expectedCount !== null && foundPlaceholders.count !== expectedCount) {
    throw new SqlGenerationError('PlaceholderCountMismatch', { expected: expectedCount, found: foundPlaceholders.count });
  }

  return buffer.join('');
}
