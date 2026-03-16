// MIRRORS: ankurah/tests/tests/selection_macro.rs
//
// Divergence: Rust selection! macro has compile-time variable interpolation with multiple
// syntax forms (unquoted, quoted, shorthand, operator shorthand). TypeScript has no macro
// system [E1]. The TS equivalent uses parseSelection() + populatePredicate() to achieve
// the same effect at runtime: parse a template with ? placeholders, then populate with values.

import { describe, test, expect } from 'bun:test';
import {
  Selection,
  Predicate,
  Expr,
  Literal,
  PathExpr,
  ComparisonOperator,
  populatePredicate,
  exprFromString,
  exprFromI64,
  exprFromF64,
  exprFromBool,
} from '@ankurah/ankql';
import { parseSelection } from '@ankurah/ankql';

// ── Helpers ──

/** Build a Selection from a predicate string with placeholder values */
function selection(predicateStr: string, ...values: Expr[]): Selection {
  const parsed = parseSelection(predicateStr);
  if (values.length > 0) {
    return new Selection(populatePredicate(parsed.predicate, values), parsed.orderBy, parsed.limit);
  }
  return parsed;
}

/** Build an expected Comparison predicate */
function comparison(
  fieldName: string,
  op: string,
  right: Expr,
): Predicate {
  return Predicate.Comparison(
    Expr.Path(PathExpr.simple(fieldName)),
    comparisonOp(op),
    right,
  );
}

function comparisonOp(op: string): ComparisonOperator {
  switch (op) {
    case '=': return ComparisonOperator.Equal();
    case '!=': return ComparisonOperator.NotEqual();
    case '>': return ComparisonOperator.GreaterThan();
    case '>=': return ComparisonOperator.GreaterThanOrEqual();
    case '<': return ComparisonOperator.LessThan();
    case '<=': return ComparisonOperator.LessThanOrEqual();
    case 'IN': return ComparisonOperator.In();
    default: throw new Error(`Unknown operator: ${op}`);
  }
}

function lit(s: string): Expr { return Expr.Literal(Literal.String(s)); }
function litI64(n: number): Expr { return Expr.Literal(Literal.I64(BigInt(n))); }
function litF64(n: number): Expr { return Expr.Literal(Literal.F64(n)); }
function litBool(b: boolean): Expr { return Expr.Literal(Literal.Bool(b)); }

/** Deep-compare two Selections for structural equality */
function expectSelectionsEqual(actual: Selection, expected: Selection): void {
  expect(actual.toString()).toBe(expected.toString());
}

// ── Tests ──

// Mirrors: test_selection_macro_unquoted_syntax
// Rust: selection!(name = { name }) where name = "Alice"
// TS: parseSelection("name = ?") + populatePredicate with "Alice"
describe('selection_macro_unquoted_syntax', () => {
  test('basic equality with variable interpolation', () => {
    const name = 'Alice';

    // Rust: selection!(name = { name })
    const result = selection('name = ?', exprFromString(name));

    const expected = new Selection(
      comparison('name', '=', lit('Alice')),
    );

    expectSelectionsEqual(result, expected);
  });

  test('multiple operators and mixed types', () => {
    const name = 'Alice';
    const age = 25;
    const active = true;

    // Rust: selection!(name = {name} AND age > {age} AND active = {active})
    const result = selection(
      'name = ? AND age > ? AND active = ?',
      exprFromString(name),
      exprFromI64(BigInt(age)),
      exprFromBool(active),
    );

    const expected = new Selection(
      Predicate.And(
        Predicate.And(
          comparison('name', '=', lit('Alice')),
          comparison('age', '>', litI64(25)),
        ),
        comparison('active', '=', litBool(true)),
      ),
    );

    expectSelectionsEqual(result, expected);
  });
});

// Mirrors: test_selection_macro_in_clause
describe('selection_macro_in_clause', () => {
  test('IN clause with multiple values', () => {
    const status1 = 'active';
    const status2 = 'pending';

    // Rust: selection!(status IN ({status1}, {status2}))
    const result = selection(
      'status IN (?, ?)',
      exprFromString(status1),
      exprFromString(status2),
    );

    const expected = new Selection(
      Predicate.Comparison(
        Expr.Path(PathExpr.simple('status')),
        ComparisonOperator.In(),
        Expr.ExprList([
          lit('active').match({ Literal: (v) => Expr.Literal(v.literal), Path: () => Expr.Placeholder(), Predicate: () => Expr.Placeholder(), InfixExpr: () => Expr.Placeholder(), ExprList: () => Expr.Placeholder(), Placeholder: () => Expr.Placeholder() }),
          lit('pending').match({ Literal: (v) => Expr.Literal(v.literal), Path: () => Expr.Placeholder(), Predicate: () => Expr.Placeholder(), InfixExpr: () => Expr.Placeholder(), ExprList: () => Expr.Placeholder(), Placeholder: () => Expr.Placeholder() }),
        ]),
      ),
    );

    // Verify the parsed result has the right structure
    expect(result.predicate.type).toBe('Comparison');
    const comp = result.predicate.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(comp.operator.type).toBe('In');
    expect(comp.right.type).toBe('ExprList');
  });
});

// Mirrors: test_selection_macro_quoted_syntax
describe('selection_macro_quoted_syntax', () => {
  test('positional arguments and mixed types', () => {
    const name = 'Bob';
    const age = 30;
    const active = true;

    // Rust: selection!("name = {} AND age = {} AND active = {}", name, age, active)
    const result = selection(
      'name = ? AND age = ? AND active = ?',
      exprFromString(name),
      exprFromI64(BigInt(age)),
      exprFromBool(active),
    );

    const expected = new Selection(
      Predicate.And(
        Predicate.And(
          comparison('name', '=', lit('Bob')),
          comparison('age', '=', litI64(30)),
        ),
        comparison('active', '=', litBool(true)),
      ),
    );

    expectSelectionsEqual(result, expected);
  });
});

// Mirrors: test_selection_macro_shorthand_syntax
describe('selection_macro_shorthand_syntax', () => {
  test('single variable shorthand', () => {
    const name = 'Alice';

    // Rust: selection!({ name }) is equivalent to selection!(name = { name })
    // TS: Both produce the same AST
    const result = selection('name = ?', exprFromString(name));

    const expected = new Selection(
      comparison('name', '=', lit('Alice')),
    );

    expectSelectionsEqual(result, expected);
  });

  test('multiple variables shorthand with AND', () => {
    const name = 'Alice';
    const age = 25;

    // Rust: selection!({name} AND {age}) means selection!(name = {name} AND age = {age})
    const result = selection(
      'name = ? AND age = ?',
      exprFromString(name),
      exprFromI64(BigInt(age)),
    );

    const expected = new Selection(
      Predicate.And(
        comparison('name', '=', lit('Alice')),
        comparison('age', '=', litI64(25)),
      ),
    );

    expectSelectionsEqual(result, expected);
  });
});

// Mirrors: test_selection_macro_operator_shorthand
describe('selection_macro_operator_shorthand', () => {
  test('greater than operator', () => {
    const age = 25;

    // Rust: selection!({>age}) means age > {age}
    const result = selection('age > ?', exprFromI64(BigInt(age)));

    const expected = new Selection(
      comparison('age', '>', litI64(25)),
    );

    expectSelectionsEqual(result, expected);
  });

  test('less than or equal operator', () => {
    const count = 10;

    // Rust: selection!({<=count}) means count <= {count}
    const result = selection('count <= ?', exprFromI64(BigInt(count)));

    const expected = new Selection(
      comparison('count', '<=', litI64(10)),
    );

    expectSelectionsEqual(result, expected);
  });

  test('not equal operator', () => {
    const status = 'active';

    // Rust: selection!({!=status}) means status != {status}
    const result = selection('status != ?', exprFromString(status));

    const expected = new Selection(
      comparison('status', '!=', lit('active')),
    );

    expectSelectionsEqual(result, expected);
  });

  test('combined operators', () => {
    const age = 25;
    const count = 10;

    // Rust: selection!({>age} AND {<=count})
    const result = selection(
      'age > ? AND count <= ?',
      exprFromI64(BigInt(age)),
      exprFromI64(BigInt(count)),
    );

    const expected = new Selection(
      Predicate.And(
        comparison('age', '>', litI64(25)),
        comparison('count', '<=', litI64(10)),
      ),
    );

    expectSelectionsEqual(result, expected);
  });

  test('greater than or equal with float', () => {
    const score = 95.5;

    // Rust: selection!({>=score}) means score >= {score}
    const result = selection('score >= ?', exprFromF64(score));

    const expected = new Selection(
      comparison('score', '>=', litF64(95.5)),
    );

    expectSelectionsEqual(result, expected);
  });
});

// Mirrors: test_selection_macro_syntax_comparison
describe('selection_macro_syntax_comparison', () => {
  test('quoted and unquoted syntax produce equivalent results', () => {
    const name = 'Alice';
    const age = 25;

    // All of these Rust forms produce the same AST:
    // selection!("name = {} AND age = {}", name, age)
    // selection!("{name} AND age = {}", age)
    // selection!("name = {} AND {age}", name)
    // selection!("{name} AND age = {age}")
    // selection!(name = {name} AND age = {age})
    // selection!({name} AND age = {age})
    // selection!(name = {name} AND {age})
    // selection!({name} AND {age})

    // In TS, all of these reduce to the same parseSelection + populatePredicate call:
    const result = selection(
      'name = ? AND age = ?',
      exprFromString(name),
      exprFromI64(BigInt(age)),
    );

    const expected = new Selection(
      Predicate.And(
        comparison('name', '=', lit('Alice')),
        comparison('age', '=', litI64(25)),
      ),
    );

    expectSelectionsEqual(result, expected);
  });
});

// Mirrors: test_selection_macro_pure_syntax_forms
describe('selection_macro_pure_syntax_forms', () => {
  test('pure quoted and pure unquoted syntax produce identical results', () => {
    const fooValue = 'test';
    const bar = 'bar_value';

    // Rust: selection!("foo = {} AND bar = {}", foo_value, "bar_value")
    const quotedResult = selection(
      'foo = ? AND bar = ?',
      exprFromString(fooValue),
      exprFromString('bar_value'),
    );

    // Rust: selection!(foo = {foo_value} AND bar = {bar})
    const unquotedResult = selection(
      'foo = ? AND bar = ?',
      exprFromString(fooValue),
      exprFromString(bar),
    );

    expectSelectionsEqual(quotedResult, unquotedResult);
  });
});

// Mirrors: test_selection_macro_edge_cases
describe('selection_macro_edge_cases', () => {
  test('mixed literal and placeholder', () => {
    const ageValue = 25;

    // Rust: selection!("name = name AND age = {}", age_value)
    // In Rust, {name} without quotes is a literal path reference, not a variable.
    // "name = name" means field "name" = field "name" (path comparison).
    // TS: Parse as "name = name AND age = ?" with age_value as placeholder.
    const result = selection(
      'name = name AND age = ?',
      exprFromI64(BigInt(ageValue)),
    );

    // The result should have:
    // - First comparison: path("name") = path("name")
    // - Second comparison: path("age") = literal(25)
    expect(result.predicate.type).toBe('And');
    const andPred = result.predicate.value as { left: Predicate; right: Predicate };
    expect(andPred.left.type).toBe('Comparison');
    expect(andPred.right.type).toBe('Comparison');

    const rightComp = andPred.right.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(rightComp.right.type).toBe('Literal');
  });
});

// Mirrors: test_selection_macro_list_expansion
describe('selection_macro_list_expansion', () => {
  test('IN clause with multiple string values via ExprList', () => {
    // Rust: let names = vec!["Alice", "Bob", "Charlie"];
    // Rust: selection!("name IN {names}")
    // TS: parseSelection with explicit IN clause
    const result = parseSelection("name IN ('Alice', 'Bob', 'Charlie')");

    expect(result.predicate.type).toBe('Comparison');
    const comp = result.predicate.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(comp.operator.type).toBe('In');
    expect(comp.right.type).toBe('ExprList');
    const exprList = comp.right.value as { exprs: Expr[] };
    expect(exprList.exprs.length).toBe(3);
  });

  test('IN clause with integer values', () => {
    // Rust: let ages = [25, 30, 35];
    // Rust: selection!("age IN {ages}")
    const result = parseSelection('age IN (25, 30, 35)');

    expect(result.predicate.type).toBe('Comparison');
    const comp = result.predicate.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(comp.operator.type).toBe('In');
    expect(comp.right.type).toBe('ExprList');
    const exprList = comp.right.value as { exprs: Expr[] };
    expect(exprList.exprs.length).toBe(3);
  });

  test('IN clause with string slice values', () => {
    // Rust: let statuses = &["active", "pending"];
    // Rust: selection!("status IN {statuses}")
    const result = parseSelection("status IN ('active', 'pending')");

    expect(result.predicate.type).toBe('Comparison');
    const comp = result.predicate.value as { left: Expr; operator: ComparisonOperator; right: Expr };
    expect(comp.operator.type).toBe('In');
    expect(comp.right.type).toBe('ExprList');
    const exprList = comp.right.value as { exprs: Expr[] };
    expect(exprList.exprs.length).toBe(2);
  });
});
