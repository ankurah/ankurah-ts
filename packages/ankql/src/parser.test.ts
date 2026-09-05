// MIRRORS: ankurah/ankql/src/parser.rs (tests module)
//
// `parse_selection` answers `Result<Selection, ParseError>`, so every test here
// unwraps one, and the payload of a tuple variant is `_0` / `_1` — the names the
// emitted ast.ts gives what Rust wrote positionally. A struct variant keeps its
// field names, which is why a Comparison is still read as `.left` / `.right`.

import { describe, test, expect } from 'bun:test';
import { parseSelection } from './parser.ts';
import type { ComparisonOperator, Expr, Literal, OrderByItem, OrderDirection, Predicate, Selection } from './ast.ts';
import type { ParseError } from './error.ts';

// ── Helpers ──

/** Rust's `parse_selection(..).unwrap()`. */
function parse(query: string): Selection {
  const result = parseSelection(query);
  expect(result.isOk()).toBe(true);
  return result.unwrap();
}

/** Rust's `assert!(parse_selection(..).is_err())`. */
function parseErr(query: string): ParseError {
  const result = parseSelection(query);
  expect(result.isErr()).toBe(true);
  return result.unwrapErr();
}

/** The two operands of an And or an Or. */
function operands(pred: Predicate): [Predicate, Predicate] {
  expect(pred.is('And') || pred.is('Or')).toBe(true);
  const v = pred.value as { _0: Predicate; _1: Predicate };
  return [v._0, v._1];
}

/** Assert pred is Comparison(Path(pathName), op, Literal(litType, litValue)). */
function assertCmpLit(pred: Predicate, pathName: string, op: ComparisonOperator['type'], litType: Literal['type'], litValue: unknown): void {
  expect(pred.is('Comparison')).toBe(true);
  if (!pred.is('Comparison')) return;
  const v = pred.value;
  assertPath(v.left, pathName);
  expect(v.operator.type).toBe(op);
  assertLiteral(v.right, litType, litValue);
}

/** Assert pred is Comparison(Path(leftPath), op, Path(rightPath)). */
function assertCmpPaths(pred: Predicate, leftPath: string, op: ComparisonOperator['type'], rightPath: string): void {
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
  if (expr.is('Path')) expect(expr.value._0.toString()).toBe(pathStr);
}

/** Assert expr is Literal with given type and value. */
function assertLiteral(expr: Expr, litType: Literal['type'], litValue: unknown): void {
  expect(expr.is('Literal')).toBe(true);
  if (expr.is('Literal')) {
    expect(expr.value._0.type).toBe(litType);
    expect((expr.value._0.value as { _0: unknown })._0).toBe(litValue);
  }
}

/** Assert expr is Placeholder. */
function assertPlaceholder(expr: Expr): void {
  expect(expr.is('Placeholder')).toBe(true);
}

/** Assert orderBy item at index has given path and direction. */
function assertOrderBy(orderBy: OrderByItem[] | null, index: number, pathName: string, dir: OrderDirection['type'], length?: number): void {
  expect(orderBy).not.toBeNull();
  if (length !== undefined) expect(orderBy!.length).toBe(length);
  expect(orderBy![index].path.toString()).toBe(pathName);
  expect(orderBy![index].direction.type).toBe(dir);
}

// ── Tests ──

describe('parser', () => {
  // Rust: fn test_parse_selection_status_active
  test('parse selection: status = active', () => {
    using selection = parse("status = 'active'");
    assertCmpLit(selection.predicate, 'status', 'Equal', 'String', 'active');
    expect(selection.orderBy).toBeNull();
    expect(selection.limit).toBeNull();
  });

  // Rust: fn test_parse_selection_user_and_status
  test('parse selection: user AND status', () => {
    using selection = parse("user = 123 AND status = 'active'");
    expect(selection.predicate.is('And')).toBe(true);
    const [left, right] = operands(selection.predicate);
    assertCmpLit(left, 'user', 'Equal', 'I32', 123);
    assertCmpLit(right, 'status', 'Equal', 'String', 'active');
  });

  // Rust: fn test_parse_selection_user_or_and_status
  test('parse selection: (user OR user) AND status', () => {
    using selection = parse("(user = 123 OR user = 456) AND status = 'active'");
    expect(selection.predicate.is('And')).toBe(true);
    const [left, right] = operands(selection.predicate);
    expect(left.is('Or')).toBe(true);
    const [orLeft, orRight] = operands(left);
    assertCmpLit(orLeft, 'user', 'Equal', 'I32', 123);
    assertCmpLit(orRight, 'user', 'Equal', 'I32', 456);
    assertCmpLit(right, 'status', 'Equal', 'String', 'active');
  });

  // Rust: fn test_parse_selection_status_is_null
  test('parse selection: status IS NULL', () => {
    using selection = parse('status IS NULL');
    expect(selection.predicate.is('IsNull')).toBe(true);
    if (selection.predicate.is('IsNull')) assertPath(selection.predicate.value._0, 'status');
  });

  // Rust: fn test_parse_selection_status_is_not_null
  test('parse selection: status IS NOT NULL', () => {
    using selection = parse('status IS NOT NULL');
    expect(selection.predicate.is('Not')).toBe(true);
    if (!selection.predicate.is('Not')) return;
    const inner = selection.predicate.value._0 as Predicate;
    expect(inner.is('IsNull')).toBe(true);
    if (inner.is('IsNull')) assertPath(inner.value._0, 'status');
  });

  // Rust: fn unary_not_parenthesized
  test('unary NOT parenthesized', () => {
    using selection = parse("NOT (status = 'active')");
    expect(selection.predicate.is('Not')).toBe(true);
    if (selection.predicate.is('Not')) assertCmpLit(selection.predicate.value._0 as Predicate, 'status', 'Equal', 'String', 'active');
  });

  // Rust: fn unary_not_unparenthesized
  test('unary NOT unparenthesized fails', () => {
    using error = parseErr("NOT status = 'active'");
    expect(error.type).toBe('UnexpectedRule');
  });

  // Rust: fn test_parse_empty_string
  test('parse empty string', () => {
    using selection = parse('');
    expect(selection.predicate.is('True')).toBe(true);
  });

  // Rust: fn test_parse_true_literal
  test('parse true literal', () => {
    using selection = parse('true');
    expect(selection.predicate.is('True')).toBe(true);
  });

  // Rust: fn test_parse_true_literal (false branch)
  test('parse false literal', () => {
    using selection = parse('false');
    expect(selection.predicate.is('False')).toBe(true);
  });

  // Rust: fn test_parse_selection_in_clause
  test('parse selection: IN clause with strings', () => {
    using selection = parse("status IN ('active', 'pending')");
    expect(selection.predicate.is('Comparison')).toBe(true);
    if (!selection.predicate.is('Comparison')) return;
    assertPath(selection.predicate.value.left, 'status');
    expect(selection.predicate.value.operator.type).toBe('In');
    const right = selection.predicate.value.right;
    expect(right.is('ExprList')).toBe(true);
    if (!right.is('ExprList')) return;
    expect(right.value._0.length).toBe(2);
    assertLiteral(right.value._0[0], 'String', 'active');
    assertLiteral(right.value._0[1], 'String', 'pending');
  });

  // Rust: fn test_parse_selection_in_clause_numbers
  test('parse selection: IN clause with numbers', () => {
    using selection = parse('user_id IN (1, 2, 3)');
    expect(selection.predicate.is('Comparison')).toBe(true);
    if (!selection.predicate.is('Comparison')) return;
    assertPath(selection.predicate.value.left, 'user_id');
    expect(selection.predicate.value.operator.type).toBe('In');
    const right = selection.predicate.value.right;
    expect(right.is('ExprList')).toBe(true);
    if (!right.is('ExprList')) return;
    expect(right.value._0.length).toBe(3);
    assertLiteral(right.value._0[0], 'I32', 1);
    assertLiteral(right.value._0[1], 'I32', 2);
    assertLiteral(right.value._0[2], 'I32', 3);
  });

  // Rust: fn test_comparison_to_true
  test('comparison to true', () => {
    using selection = parse('bool_field = true');
    assertCmpLit(selection.predicate, 'bool_field', 'Equal', 'Bool', true);
  });

  // Rust: fn test_comparison_to_false
  test('comparison to false', () => {
    using selection = parse('bool_field <> false');
    assertCmpLit(selection.predicate, 'bool_field', 'NotEqual', 'Bool', false);
  });

  // Rust: fn test_comparison_to_left_operand_boolean
  test('comparison with left operand boolean', () => {
    using selection = parse('false <> bool_field');
    expect(selection.predicate.is('Comparison')).toBe(true);
    if (!selection.predicate.is('Comparison')) return;
    assertLiteral(selection.predicate.value.left, 'Bool', false);
    expect(selection.predicate.value.operator.type).toBe('NotEqual');
    assertPath(selection.predicate.value.right, 'bool_field');
  });

  // Rust: fn test_placeholders
  describe('placeholders', () => {
    test('single literal placeholder in comparison', () => {
      using selection = parse('user_id = ?');
      const pred = selection.predicate;
      expect(pred.is('Comparison')).toBe(true);
      if (!pred.is('Comparison')) return;
      assertPath(pred.value.left, 'user_id');
      expect(pred.value.operator.type).toBe('Equal');
      assertPlaceholder(pred.value.right);
    });

    test('multiple literal placeholders in AND expression', () => {
      using selection = parse('user_id = ? AND status = ?');
      expect(selection.predicate.is('And')).toBe(true);
      const [left, right] = operands(selection.predicate);
      for (const [pred, path] of [[left, 'user_id'], [right, 'status']] as const) {
        expect(pred.is('Comparison')).toBe(true);
        if (!pred.is('Comparison')) continue;
        assertPath(pred.value.left, path);
        expect(pred.value.operator.type).toBe('Equal');
        assertPlaceholder(pred.value.right);
      }
    });

    test('literal placeholders in IN clause', () => {
      using selection = parse('status IN (?, ?, ?)');
      const pred = selection.predicate;
      expect(pred.is('Comparison')).toBe(true);
      if (!pred.is('Comparison')) return;
      assertPath(pred.value.left, 'status');
      expect(pred.value.operator.type).toBe('In');
      expect(pred.value.right.is('ExprList')).toBe(true);
      if (!pred.value.right.is('ExprList')) return;
      const exprs = pred.value.right.value._0;
      expect(exprs.length).toBe(3);
      for (const expr of exprs) assertPlaceholder(expr);
    });

    test('predicate placeholders connected by AND', () => {
      using selection = parse('? AND ?');
      expect(selection.predicate.is('And')).toBe(true);
      const [left, right] = operands(selection.predicate);
      expect(left.is('Placeholder')).toBe(true);
      expect(right.is('Placeholder')).toBe(true);
    });

    test('predicate placeholders connected by OR', () => {
      using selection = parse('? OR ?');
      expect(selection.predicate.is('Or')).toBe(true);
      const [left, right] = operands(selection.predicate);
      expect(left.is('Placeholder')).toBe(true);
      expect(right.is('Placeholder')).toBe(true);
    });

    test('single predicate placeholder', () => {
      using selection = parse('?');
      expect(selection.predicate.is('Placeholder')).toBe(true);
    });

    test('mix of predicate and literal placeholders', () => {
      using selection = parse('? AND foo = ?');
      expect(selection.predicate.is('And')).toBe(true);
      const [left, right] = operands(selection.predicate);
      expect(left.is('Placeholder')).toBe(true);
      expect(right.is('Comparison')).toBe(true);
      if (!right.is('Comparison')) return;
      assertPath(right.value.left, 'foo');
      expect(right.value.operator.type).toBe('Equal');
      assertPlaceholder(right.value.right);
    });
  });

  describe('ORDER BY', () => {
    // Rust: fn test_order_by_basic
    test('basic ORDER BY', () => {
      using selection = parse("status = 'active' ORDER BY name");
      assertCmpLit(selection.predicate, 'status', 'Equal', 'String', 'active');
      assertOrderBy(selection.orderBy, 0, 'name', 'Asc', 1);
      expect(selection.limit).toBeNull();
    });

    // Rust: fn test_order_by_with_direction
    test('ORDER BY with direction', () => {
      using selection = parse('true ORDER BY created_at DESC');
      expect(selection.predicate.is('True')).toBe(true);
      assertOrderBy(selection.orderBy, 0, 'created_at', 'Desc', 1);
    });

    // Rust: fn test_order_by_dotted_identifier_not_supported
    test('ORDER BY dotted identifier not supported', () => {
      using error = parseErr('true ORDER BY user.name ASC');
      // The grammar stops at the dot, so this is refused before ORDER BY's own
      // "Dotted identifiers are not supported" check can ever run.
      expect(error.type).toBe('SyntaxError');
    });

    // Rust: fn test_order_by_only
    test('ORDER BY only', () => {
      using selection = parse('true ORDER BY score');
      expect(selection.predicate.is('True')).toBe(true);
      assertOrderBy(selection.orderBy, 0, 'score', 'Asc', 1);
      expect(selection.limit).toBeNull();
    });

    // Rust: fn test_order_by_multiple_items
    test('ORDER BY multiple items', () => {
      using selection = parse('true ORDER BY name ASC, created_at DESC, id');
      expect(selection.predicate.is('True')).toBe(true);
      assertOrderBy(selection.orderBy, 0, 'name', 'Asc', 3);
      assertOrderBy(selection.orderBy, 1, 'created_at', 'Desc');
      assertOrderBy(selection.orderBy, 2, 'id', 'Asc');
      expect(selection.limit).toBeNull();
    });
  });

  describe('LIMIT', () => {
    // Rust: fn test_limit_basic
    test('basic LIMIT', () => {
      using selection = parse("status = 'active' LIMIT 10");
      assertCmpLit(selection.predicate, 'status', 'Equal', 'String', 'active');
      expect(selection.orderBy).toBeNull();
      expect(selection.limit).toBe(10n);
    });

    // Rust: fn test_limit_only
    test('LIMIT only', () => {
      using selection = parse('true LIMIT 100');
      expect(selection.predicate.is('True')).toBe(true);
      expect(selection.orderBy).toBeNull();
      expect(selection.limit).toBe(100n);
    });
  });

  describe('ORDER BY and LIMIT combined', () => {
    // Rust: fn test_order_by_and_limit
    test('both ORDER BY and LIMIT', () => {
      using selection = parse('user_id > 100 ORDER BY created_at DESC LIMIT 5');
      assertCmpLit(selection.predicate, 'user_id', 'GreaterThan', 'I32', 100);
      assertOrderBy(selection.orderBy, 0, 'created_at', 'Desc', 1);
      expect(selection.limit).toBe(5n);
    });
  });

  // Rust: fn test_pathological_keyword_cases
  describe('pathological keyword cases', () => {
    test('limit as column name', () => {
      using selection = parse('limit = 1');
      assertCmpLit(selection.predicate, 'limit', 'Equal', 'I32', 1);
    });

    test('order as column name with ORDER BY', () => {
      using selection = parse('order = 2 ORDER BY name');
      assertCmpLit(selection.predicate, 'order', 'Equal', 'I32', 2);
      assertOrderBy(selection.orderBy, 0, 'name', 'Asc', 1);
    });
  });

  // Rust: fn test_boolean_literals
  describe('boolean literals', () => {
    test('true parses as Predicate.True', () => {
      using selection = parse('true');
      expect(selection.predicate.is('True')).toBe(true);
    });

    test('false parses as Predicate.False', () => {
      using selection = parse('false');
      expect(selection.predicate.is('False')).toBe(true);
    });
  });

  describe('path expressions', () => {
    test('dotted path in comparison', () => {
      using selection = parse("person.name = 'Alice'");
      assertCmpLit(selection.predicate, 'person.name', 'Equal', 'String', 'Alice');
    });

    test('dotted paths on both sides', () => {
      using selection = parse('a.foo = b.foo');
      assertCmpPaths(selection.predicate, 'a.foo', 'Equal', 'b.foo');
    });
  });

  describe('case insensitivity', () => {
    test('AND/and/And all work', () => {
      for (const query of ['a = 1 AND b = 2', 'a = 1 and b = 2', 'a = 1 And b = 2']) {
        using selection = parse(query);
        expect(selection.predicate.type).toBe('And');
      }
    });

    test('OR/or/Or all work', () => {
      for (const query of ['a = 1 OR b = 2', 'a = 1 or b = 2']) {
        using selection = parse(query);
        expect(selection.predicate.type).toBe('Or');
      }
    });

    test('IS NULL/is null', () => {
      for (const query of ['a IS NULL', 'a is null']) {
        using selection = parse(query);
        expect(selection.predicate.type).toBe('IsNull');
      }
    });

    test('TRUE/true/True', () => {
      for (const query of ['TRUE', 'true']) {
        using selection = parse(query);
        expect(selection.predicate.type).toBe('True');
      }
    });

    test('IN/in', () => {
      for (const query of ['x IN (1, 2)', 'x in (1, 2)']) {
        using selection = parse(query);
        const pred = selection.predicate;
        expect(pred.is('Comparison')).toBe(true);
        if (pred.is('Comparison')) expect(pred.value.operator.type).toBe('In');
      }
    });
  });

  test('!= as NotEqual', () => {
    using selection = parse('a != 1');
    assertCmpLit(selection.predicate, 'a', 'NotEqual', 'I32', 1);
  });
});
