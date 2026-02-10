// MIRRORS: ankurah/ankql/src/selection/sql.rs

import type { Expr, Literal, Predicate, ComparisonOperator } from '../ast.ts';
import {
  PlaceholderCountMismatchError,
  InvalidExpressionError,
  UnsupportedOperatorError,
} from '../error.ts';

function generateExprSql(
  expr: Expr,
  expectedCount: number | null,
  foundPlaceholders: { count: number },
  buffer: string[],
): void {
  switch (expr.type) {
    case 'Placeholder': {
      foundPlaceholders.count++;
      if (expectedCount !== null && foundPlaceholders.count > expectedCount) {
        throw new PlaceholderCountMismatchError(expectedCount, foundPlaceholders.count);
      }
      buffer.push('?');
      break;
    }
    case 'Literal':
      buffer.push(literalToSql(expr.value));
      break;
    case 'Path':
      buffer.push(expr.value.steps.map((s) => `"${s}"`).join('.'));
      break;
    case 'ExprList': {
      buffer.push('(');
      for (let i = 0; i < expr.values.length; i++) {
        if (i > 0) buffer.push(', ');
        const item = expr.values[i];
        if (item.type === 'Placeholder') {
          foundPlaceholders.count++;
          if (expectedCount !== null && foundPlaceholders.count > expectedCount) {
            throw new PlaceholderCountMismatchError(expectedCount, foundPlaceholders.count);
          }
          buffer.push('?');
        } else if (item.type === 'Literal') {
          buffer.push(literalToSql(item.value));
        } else {
          throw new InvalidExpressionError(
            'Only literal expressions and placeholders are supported in IN lists',
          );
        }
      }
      buffer.push(')');
      break;
    }
    default:
      throw new InvalidExpressionError(
        'Only literal, identifier, and list expressions are supported',
      );
  }
}

function literalToSql(lit: Literal): string {
  switch (lit.type) {
    case 'I16':
    case 'I32':
      return String(lit.value);
    case 'I64':
      return String(lit.value);
    case 'F64':
      return String(lit.value);
    case 'Bool':
      return lit.value ? 'true' : 'false';
    case 'String': {
      let escaped = '';
      for (const ch of lit.value) {
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
    }
    case 'EntityId': {
      // Base64url encode the ULID bytes
      const bytes = lit.value;
      let binary = '';
      for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i]);
      }
      const b64 = btoa(binary)
        .replace(/\+/g, '-')
        .replace(/\//g, '_')
        .replace(/=+$/, '');
      return `'${b64}'`;
    }
    case 'Object':
    case 'Binary': {
      const decoded = new TextDecoder().decode(lit.value);
      return `'${decoded}'`;
    }
    case 'Json':
      return `'${JSON.stringify(lit.value)}'`;
  }
}

function comparisonOpToSql(op: ComparisonOperator): string {
  switch (op) {
    case 'Equal': return '=';
    case 'NotEqual': return '<>';
    case 'GreaterThan': return '>';
    case 'GreaterThanOrEqual': return '>=';
    case 'LessThan': return '<';
    case 'LessThanOrEqual': return '<=';
    case 'In': return 'IN';
    case 'Between':
      throw new UnsupportedOperatorError('BETWEEN operator is not yet supported');
  }
}

function generatePredicateSql(
  predicate: Predicate,
  expectedCount: number | null,
  foundPlaceholders: { count: number },
  buffer: string[],
): void {
  switch (predicate.type) {
    case 'Comparison': {
      generateExprSql(predicate.left, expectedCount, foundPlaceholders, buffer);
      buffer.push(' ');
      buffer.push(comparisonOpToSql(predicate.operator));
      buffer.push(' ');
      generateExprSql(predicate.right, expectedCount, foundPlaceholders, buffer);
      break;
    }
    case 'And': {
      generatePredicateSql(predicate.left, expectedCount, foundPlaceholders, buffer);
      buffer.push(' AND ');
      generatePredicateSql(predicate.right, expectedCount, foundPlaceholders, buffer);
      break;
    }
    case 'Or': {
      buffer.push('(');
      generatePredicateSql(predicate.left, expectedCount, foundPlaceholders, buffer);
      buffer.push(' OR ');
      generatePredicateSql(predicate.right, expectedCount, foundPlaceholders, buffer);
      buffer.push(')');
      break;
    }
    case 'Not': {
      buffer.push('NOT (');
      generatePredicateSql(predicate.predicate, expectedCount, foundPlaceholders, buffer);
      buffer.push(')');
      break;
    }
    case 'IsNull': {
      generateExprSql(predicate.expr, expectedCount, foundPlaceholders, buffer);
      buffer.push(' IS NULL');
      break;
    }
    case 'True':
      buffer.push('TRUE');
      break;
    case 'False':
      buffer.push('FALSE');
      break;
    case 'Placeholder':
      throw new InvalidExpressionError(
        'Placeholder must be transformed before SQL generation',
      );
  }
}

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
    throw new PlaceholderCountMismatchError(expectedCount, foundPlaceholders.count);
  }

  return buffer.join('');
}
