// MIRRORS: ankurah/ankql/src/parser.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { parseSelection } from './parser.ts';
import { PathExpr } from './ast.ts';
import type { Predicate, Expr, Literal, ComparisonOperator } from './ast.ts';

// ── Helpers ──

/** Assert pred is Comparison(Path(pathName), op, Literal(litType, litValue)). */
function assertCmpLit(
  pred: Predicate,
  pathName: string,
  op: ComparisonOperator['type'],
  litType: Literal['type'],
  litValue: unknown,
): void {
  expect(pred.is('Comparison')).toBe(true);
  if (!pred.is('Comparison')) return;
  const v = pred.value;
  assertPath(v.left, pathName);
  expect(v.operator.type).toBe(op);
  assertLiteral(v.right, litType, litValue);
}

/** Assert pred is Comparison(Path(leftPath), op, Path(rightPath)). */
function assertCmpPaths(
  pred: Predicate,
  leftPath: string,
  op: ComparisonOperator['type'],
  rightPath: string,
): void {
  expect(pred.is('Comparison')).toBe(true);
  if (!pred.is('Comparison')) return;
  const v = pred.value;
  assertPath(v.left, leftPath);
  expect(v.operator.type).toBe(op);
  assertPath(v.right, rightPath);
}

/** Assert expr is Path with given string representation. */
function assertPath(expr: Expr, pathStr: string): void {
  expect(expr.is('Path')).toBe(true);
  if (expr.is('Path')) {
    expect(expr.value.path.toString()).toBe(pathStr);
  }
}

/** Assert expr is Literal with given type and value. */
function assertLiteral(expr: Expr, litType: Literal['type'], litValue: unknown): void {
  expect(expr.is('Literal')).toBe(true);
  if (expr.is('Literal')) {
    expect(expr.value.literal.type).toBe(litType);
    expect(expr.value.literal.value.value).toBe(litValue);
  }
}

/** Assert expr is Placeholder. */
function assertPlaceholder(expr: Expr): void {
  expect(expr.is('Placeholder')).toBe(true);
}

/** Assert orderBy item at index has given path and direction. */
function assertOrderBy(
  orderBy: any[] | null,
  index: number,
  pathName: string,
  dir: string,
): void {
  expect(orderBy).not.toBeNull();
  expect(orderBy!.length).toBeGreaterThan(index);
  expect(orderBy![index].path.toString()).toBe(pathName);
  expect(orderBy![index].direction.type).toBe(dir);
}

// ── Tests ──

describe('parser', () => {
  // Rust: fn test_parse_selection_status_active
  test('parse selection: status = active', () => {
    using selection = parseSelection("status = 'active'");
    assertCmpLit(selection.predicate, 'status', 'Equal', 'String', 'active');
    expect(selection.orderBy).toBeNull();
    expect(selection.limit).toBeNull();
  });

  // Rust: fn test_parse_selection_user_and_status
  test('parse selection: user AND status', () => {
    using selection = parseSelection("user = 123 AND status = 'active'");
    expect(selection.predicate.is('And')).toBe(true);
    if (selection.predicate.is('And')) {
      assertCmpLit(selection.predicate.value.left, 'user', 'Equal', 'I32', 123);
      assertCmpLit(selection.predicate.value.right, 'status', 'Equal', 'String', 'active');
    }
  });

  // Rust: fn test_parse_selection_user_or_and_status
  test('parse selection: (user OR user) AND status', () => {
    using selection = parseSelection("(user = 123 OR user = 456) AND status = 'active'");
    expect(selection.predicate.is('And')).toBe(true);
    if (selection.predicate.is('And')) {
      const left = selection.predicate.value.left;
      expect(left.is('Or')).toBe(true);
      if (left.is('Or')) {
        assertCmpLit(left.value.left, 'user', 'Equal', 'I32', 123);
        assertCmpLit(left.value.right, 'user', 'Equal', 'I32', 456);
      }
      assertCmpLit(selection.predicate.value.right, 'status', 'Equal', 'String', 'active');
    }
  });

  // Rust: fn test_parse_selection_status_is_null
  test('parse selection: status IS NULL', () => {
    using selection = parseSelection('status IS NULL');
    expect(selection.predicate.is('IsNull')).toBe(true);
    if (selection.predicate.is('IsNull')) {
      assertPath(selection.predicate.value.expr, 'status');
    }
  });

  // Rust: fn test_parse_selection_status_is_not_null
  test('parse selection: status IS NOT NULL', () => {
    using selection = parseSelection('status IS NOT NULL');
    expect(selection.predicate.is('Not')).toBe(true);
    if (selection.predicate.is('Not')) {
      const inner = selection.predicate.value.predicate;
      expect(inner.is('IsNull')).toBe(true);
      if (inner.is('IsNull')) {
        assertPath(inner.value.expr, 'status');
      }
    }
  });

  // Rust: fn unary_not_parenthesized
  test('unary NOT parenthesized', () => {
    using selection = parseSelection("NOT (status = 'active')");
    expect(selection.predicate.is('Not')).toBe(true);
    if (selection.predicate.is('Not')) {
      assertCmpLit(selection.predicate.value.predicate, 'status', 'Equal', 'String', 'active');
    }
  });

  // Rust: fn unary_not_unparenthesized
  test('unary NOT unparenthesized fails', () => {
    try {
      parseSelection("NOT status = 'active'");
      expect(true).toBe(false); // should not reach here
    } catch (e: any) {
      if (typeof e?.drop === 'function') e.drop();
    }
  });

  // Rust: fn test_parse_empty_string
  test('parse empty string', () => {
    using selection = parseSelection('');
    expect(selection.predicate.is('True')).toBe(true);
  });

  // Rust: fn test_parse_true_literal
  test('parse true literal', () => {
    using selection = parseSelection('true');
    expect(selection.predicate.is('True')).toBe(true);
  });

  // Rust: fn test_parse_true_literal (false branch)
  test('parse false literal', () => {
    using selection = parseSelection('false');
    expect(selection.predicate.is('False')).toBe(true);
  });

  // Rust: fn test_parse_selection_in_clause
  test('parse selection: IN clause with strings', () => {
    using selection = parseSelection("status IN ('active', 'pending')");
    expect(selection.predicate.is('Comparison')).toBe(true);
    if (selection.predicate.is('Comparison')) {
      assertPath(selection.predicate.value.left, 'status');
      expect(selection.predicate.value.operator.type).toBe('In');
      const right = selection.predicate.value.right;
      expect(right.is('ExprList')).toBe(true);
      if (right.is('ExprList')) {
        expect(right.value.exprs.length).toBe(2);
        assertLiteral(right.value.exprs[0], 'String', 'active');
        assertLiteral(right.value.exprs[1], 'String', 'pending');
      }
    }
  });

  // Rust: fn test_parse_selection_in_clause_numbers
  test('parse selection: IN clause with numbers', () => {
    using selection = parseSelection('user_id IN (1, 2, 3)');
    expect(selection.predicate.is('Comparison')).toBe(true);
    if (selection.predicate.is('Comparison')) {
      assertPath(selection.predicate.value.left, 'user_id');
      expect(selection.predicate.value.operator.type).toBe('In');
      const right = selection.predicate.value.right;
      expect(right.is('ExprList')).toBe(true);
      if (right.is('ExprList')) {
        expect(right.value.exprs.length).toBe(3);
        assertLiteral(right.value.exprs[0], 'I32', 1);
        assertLiteral(right.value.exprs[1], 'I32', 2);
        assertLiteral(right.value.exprs[2], 'I32', 3);
      }
    }
  });

  // Rust: fn test_comparison_to_true
  test('comparison to true', () => {
    using selection = parseSelection('bool_field = true');
    assertCmpLit(selection.predicate, 'bool_field', 'Equal', 'Bool', true);
  });

  // Rust: fn test_comparison_to_false
  test('comparison to false', () => {
    using selection = parseSelection('bool_field <> false');
    assertCmpLit(selection.predicate, 'bool_field', 'NotEqual', 'Bool', false);
  });

  // Rust: fn test_comparison_to_left_operand_boolean
  test('comparison with left operand boolean', () => {
    using selection = parseSelection('false <> bool_field');
    expect(selection.predicate.is('Comparison')).toBe(true);
    if (selection.predicate.is('Comparison')) {
      assertLiteral(selection.predicate.value.left, 'Bool', false);
      expect(selection.predicate.value.operator.type).toBe('NotEqual');
      assertPath(selection.predicate.value.right, 'bool_field');
    }
  });

  // Rust: fn test_placeholders
  describe('placeholders', () => {
    test('single literal placeholder in comparison', () => {
      using selection = parseSelection('user_id = ?');
      const pred = selection.predicate;
      expect(pred.is('Comparison')).toBe(true);
      if (pred.is('Comparison')) {
        assertPath(pred.value.left, 'user_id');
        expect(pred.value.operator.type).toBe('Equal');
        assertPlaceholder(pred.value.right);
      }
    });

    test('multiple literal placeholders in AND expression', () => {
      using selection = parseSelection('user_id = ? AND status = ?');
      const pred = selection.predicate;
      expect(pred.is('And')).toBe(true);
      if (pred.is('And')) {
        expect(pred.value.left.is('Comparison')).toBe(true);
        if (pred.value.left.is('Comparison')) {
          assertPath(pred.value.left.value.left, 'user_id');
          expect(pred.value.left.value.operator.type).toBe('Equal');
          assertPlaceholder(pred.value.left.value.right);
        }
        expect(pred.value.right.is('Comparison')).toBe(true);
        if (pred.value.right.is('Comparison')) {
          assertPath(pred.value.right.value.left, 'status');
          expect(pred.value.right.value.operator.type).toBe('Equal');
          assertPlaceholder(pred.value.right.value.right);
        }
      }
    });

    test('literal placeholders in IN clause', () => {
      using selection = parseSelection('status IN (?, ?, ?)');
      const pred = selection.predicate;
      expect(pred.is('Comparison')).toBe(true);
      if (pred.is('Comparison')) {
        assertPath(pred.value.left, 'status');
        expect(pred.value.operator.type).toBe('In');
        expect(pred.value.right.is('ExprList')).toBe(true);
        if (pred.value.right.is('ExprList')) {
          const exprs = pred.value.right.value.exprs;
          expect(exprs.length).toBe(3);
          assertPlaceholder(exprs[0]);
          assertPlaceholder(exprs[1]);
          assertPlaceholder(exprs[2]);
        }
      }
    });

    test('predicate placeholders connected by AND', () => {
      using selection = parseSelection('? AND ?');
      const pred = selection.predicate;
      expect(pred.is('And')).toBe(true);
      if (pred.is('And')) {
        expect(pred.value.left.is('Placeholder')).toBe(true);
        expect(pred.value.right.is('Placeholder')).toBe(true);
      }
    });

    test('predicate placeholders connected by OR', () => {
      using selection = parseSelection('? OR ?');
      const pred = selection.predicate;
      expect(pred.is('Or')).toBe(true);
      if (pred.is('Or')) {
        expect(pred.value.left.is('Placeholder')).toBe(true);
        expect(pred.value.right.is('Placeholder')).toBe(true);
      }
    });

    test('single predicate placeholder', () => {
      using selection = parseSelection('?');
      expect(selection.predicate.is('Placeholder')).toBe(true);
    });

    test('mix of predicate and literal placeholders', () => {
      using selection = parseSelection('? AND foo = ?');
      const pred = selection.predicate;
      expect(pred.is('And')).toBe(true);
      if (pred.is('And')) {
        expect(pred.value.left.is('Placeholder')).toBe(true);
        expect(pred.value.right.is('Comparison')).toBe(true);
        if (pred.value.right.is('Comparison')) {
          assertPath(pred.value.right.value.left, 'foo');
          expect(pred.value.right.value.operator.type).toBe('Equal');
          assertPlaceholder(pred.value.right.value.right);
        }
      }
    });
  });

  describe('ORDER BY', () => {
    // Rust: fn test_order_by_basic
    test('basic ORDER BY', () => {
      using selection = parseSelection("status = 'active' ORDER BY name");
      assertCmpLit(selection.predicate, 'status', 'Equal', 'String', 'active');
      assertOrderBy(selection.orderBy, 0, 'name', 'Asc');
      expect(selection.orderBy!.length).toBe(1);
      expect(selection.limit).toBeNull();
    });

    // Rust: fn test_order_by_with_direction
    test('ORDER BY with direction', () => {
      using selection = parseSelection('true ORDER BY created_at DESC');
      expect(selection.predicate.is('True')).toBe(true);
      assertOrderBy(selection.orderBy, 0, 'created_at', 'Desc');
      expect(selection.orderBy!.length).toBe(1);
    });

    // Rust: fn test_order_by_dotted_identifier_not_supported
    test('ORDER BY dotted identifier not supported', () => {
      try {
        parseSelection('true ORDER BY user.name ASC');
        expect(true).toBe(false); // should not reach here
      } catch (e: any) {
        if (typeof e?.drop === 'function') e.drop();
      }
    });

    // Rust: fn test_order_by_only
    test('ORDER BY only', () => {
      using selection = parseSelection('true ORDER BY score');
      expect(selection.predicate.is('True')).toBe(true);
      assertOrderBy(selection.orderBy, 0, 'score', 'Asc');
      expect(selection.orderBy!.length).toBe(1);
      expect(selection.limit).toBeNull();
    });

    // Rust: fn test_order_by_multiple_items
    test('ORDER BY multiple items', () => {
      using selection = parseSelection('true ORDER BY name ASC, created_at DESC, id');
      expect(selection.predicate.is('True')).toBe(true);
      expect(selection.orderBy!.length).toBe(3);
      assertOrderBy(selection.orderBy, 0, 'name', 'Asc');
      assertOrderBy(selection.orderBy, 1, 'created_at', 'Desc');
      assertOrderBy(selection.orderBy, 2, 'id', 'Asc');
      expect(selection.limit).toBeNull();
    });
  });

  describe('LIMIT', () => {
    // Rust: fn test_limit_basic
    test('basic LIMIT', () => {
      using selection = parseSelection("status = 'active' LIMIT 10");
      assertCmpLit(selection.predicate, 'status', 'Equal', 'String', 'active');
      expect(selection.orderBy).toBeNull();
      expect(selection.limit).toBe(10n);
    });

    // Rust: fn test_limit_only
    test('LIMIT only', () => {
      using selection = parseSelection('true LIMIT 100');
      expect(selection.predicate.is('True')).toBe(true);
      expect(selection.orderBy).toBeNull();
      expect(selection.limit).toBe(100n);
    });
  });

  describe('ORDER BY and LIMIT combined', () => {
    // Rust: fn test_order_by_and_limit
    test('both ORDER BY and LIMIT', () => {
      using selection = parseSelection('user_id > 100 ORDER BY created_at DESC LIMIT 5');
      assertCmpLit(selection.predicate, 'user_id', 'GreaterThan', 'I32', 100);
      assertOrderBy(selection.orderBy, 0, 'created_at', 'Desc');
      expect(selection.orderBy!.length).toBe(1);
      expect(selection.limit).toBe(5n);
    });
  });

  // Rust: fn test_pathological_keyword_cases
  describe('pathological keyword cases', () => {
    test('limit as column name', () => {
      using selection = parseSelection('limit = 1');
      assertCmpLit(selection.predicate, 'limit', 'Equal', 'I32', 1);
    });

    test('order as column name with ORDER BY', () => {
      using selection = parseSelection('order = 2 ORDER BY name');
      assertCmpLit(selection.predicate, 'order', 'Equal', 'I32', 2);
      assertOrderBy(selection.orderBy, 0, 'name', 'Asc');
      expect(selection.orderBy!.length).toBe(1);
    });
  });

  // Rust: fn test_boolean_literals
  describe('boolean literals', () => {
    test('true parses as Predicate.True', () => {
      using selection = parseSelection('true');
      expect(selection.predicate.is('True')).toBe(true);
    });

    test('false parses as Predicate.False', () => {
      using selection = parseSelection('false');
      expect(selection.predicate.is('False')).toBe(true);
    });
  });

  describe('path expressions', () => {
    test('dotted path in comparison', () => {
      using selection = parseSelection("person.name = 'Alice'");
      assertCmpLit(selection.predicate, 'person.name', 'Equal', 'String', 'Alice');
    });

    test('dotted paths on both sides', () => {
      using selection = parseSelection('a.foo = b.foo');
      assertCmpPaths(selection.predicate, 'a.foo', 'Equal', 'b.foo');
    });
  });

  describe('case insensitivity', () => {
    test('AND/and/And all work', () => {
      using s1 = parseSelection("a = 1 AND b = 2");
      using s2 = parseSelection("a = 1 and b = 2");
      using s3 = parseSelection("a = 1 And b = 2");
      expect(s1.predicate.type).toBe('And');
      expect(s2.predicate.type).toBe('And');
      expect(s3.predicate.type).toBe('And');
    });

    test('OR/or/Or all work', () => {
      using s1 = parseSelection("a = 1 OR b = 2");
      using s2 = parseSelection("a = 1 or b = 2");
      expect(s1.predicate.type).toBe('Or');
      expect(s2.predicate.type).toBe('Or');
    });

    test('IS NULL/is null', () => {
      using s1 = parseSelection("a IS NULL");
      using s2 = parseSelection("a is null");
      expect(s1.predicate.type).toBe('IsNull');
      expect(s2.predicate.type).toBe('IsNull');
    });

    test('TRUE/true/True', () => {
      using s1 = parseSelection("TRUE");
      using s2 = parseSelection("true");
      expect(s1.predicate.type).toBe('True');
      expect(s2.predicate.type).toBe('True');
    });

    test('IN/in', () => {
      using s1 = parseSelection("x IN (1, 2)");
      using s2 = parseSelection("x in (1, 2)");
      const p1 = s1.predicate;
      const p2 = s2.predicate;
      expect(p1.is('Comparison')).toBe(true);
      expect(p2.is('Comparison')).toBe(true);
      if (p1.is('Comparison')) expect(p1.value.operator.type).toBe('In');
      if (p2.is('Comparison')) expect(p2.value.operator.type).toBe('In');
    });
  });

  test('!= as NotEqual', () => {
    using selection = parseSelection('a != 1');
    assertCmpLit(selection.predicate, 'a', 'NotEqual', 'I32', 1);
  });
});
