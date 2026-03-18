// MIRRORS: ankurah/storage/postgres/src/sql_builder.rs #[cfg(test)]

import { describe, expect, test } from 'bun:test';
import {
  ComparisonOperator,
  Expr,
  Literal,
  OrderByItem,
  OrderDirection,
  PathExpr,
  Predicate,
  Selection,
  parseSelection,
} from '@ankurah/ankql';
import { SqlBuilder, SqlGenerationError, splitPredicateForPostgres } from './sql_builder.ts';

// ── Helpers ──────────────────────────────────────────────────────────

function assertArgs(args: unknown[], expected: unknown[]): void {
  // Mirrors Rust: `assert_eq!(format!("{:?}", args), format!("{:?}", expected));`
  // We compare debug-formatted strings since TS args may be mixed types.
  // Divergence: JSON.stringify can't handle BigInt, so we use a replacer [E8].
  const replacer = (_key: string, value: unknown) =>
    typeof value === 'bigint' ? `BigInt(${value})` : value;
  expect(JSON.stringify(args, replacer)).toEqual(JSON.stringify(expected, replacer));
}

// =========================================================================
// mod tests
// =========================================================================

describe('SqlBuilder', () => {
  test('test_simple_equality', () => {
    const selection = parseSelection("name = 'Alice'");
    const sql = new SqlBuilder();
    sql.selection(selection);

    const result = sql.buildWhereClause();
    expect(result.sql).toBe('"name" = $1');
    assertArgs(result.args, ['Alice']);
  });

  test('test_and_condition', () => {
    const selection = parseSelection("name = 'Alice' AND age = 30");
    const sql = SqlBuilder.withFields(['id', 'name', 'age']);
    sql.tableName('users');
    sql.selection(selection);
    const result = sql.build();

    expect(result.sql).toBe('SELECT "id", "name", "age" FROM "users" WHERE "name" = $1 AND "age" = $2');
    assertArgs(result.args, ['Alice', 30]);
  });

  test('test_complex_condition', () => {
    const selection = parseSelection("(name = 'Alice' OR name = 'Charlie') AND age >= 30 AND age <= 40");

    const sql = SqlBuilder.withFields(['id', 'name', 'age']);
    sql.tableName('users');
    sql.selection(selection);
    const result = sql.build();

    expect(result.sql).toBe(
      'SELECT "id", "name", "age" FROM "users" WHERE ("name" = $1 OR "name" = $2) AND "age" >= $3 AND "age" <= $4',
    );
    assertArgs(result.args, ['Alice', 'Charlie', 30, 40]);
  });

  test('test_including_collection_identifier', () => {
    // Tests multi-step path SQL generation using JSONB operators.
    const selection = parseSelection("person.name = 'Alice'");

    const sql = SqlBuilder.withFields(['id', 'name']);
    sql.tableName('people');
    sql.selection(selection);
    const result = sql.build();

    // Multi-step paths generate JSONB syntax: -> with ::jsonb cast for proper comparison
    expect(result.sql).toBe(
      `SELECT "id", "name" FROM "people" WHERE "person"->'name' = '"Alice"'::jsonb`,
    );
    // No args - the value is inlined as ::jsonb cast
    assertArgs(result.args, []);
  });

  test('test_false_predicate', () => {
    const sql = SqlBuilder.withFields(['id']);
    sql.tableName('test');
    sql.predicate(Predicate.False());
    const result = sql.build();

    expect(result.sql).toBe('SELECT "id" FROM "test" WHERE FALSE');
    assertArgs(result.args, []);
  });

  test('test_in_operator', () => {
    const selection = parseSelection("name IN ('Alice', 'Bob', 'Charlie')");
    const sql = SqlBuilder.withFields(['id', 'name']);
    sql.tableName('users');
    sql.selection(selection);
    const result = sql.build();

    expect(result.sql).toBe('SELECT "id", "name" FROM "users" WHERE "name" IN ($1, $2, $3)');
    assertArgs(result.args, ['Alice', 'Bob', 'Charlie']);
  });

  test('test_placeholder_error', () => {
    const sql = SqlBuilder.withFields(['id']);
    sql.tableName('test');
    expect(() => sql.predicate(Predicate.Placeholder())).toThrow(SqlGenerationError);
  });

  test('test_selection_with_order_by', () => {
    const baseSelection = parseSelection("name = 'Alice'");
    const selection = new Selection(
      baseSelection.predicate,
      [new OrderByItem(PathExpr.simple('created_at'), OrderDirection.Desc())],
      null,
    );

    const sql = SqlBuilder.withFields(['id', 'name', 'created_at']);
    sql.tableName('users');
    sql.selection(selection);
    const result = sql.build();

    expect(result.sql).toBe(
      'SELECT "id", "name", "created_at" FROM "users" WHERE "name" = $1 ORDER BY "created_at" DESC',
    );
    assertArgs(result.args, ['Alice']);
  });

  test('test_selection_with_limit', () => {
    const baseSelection = parseSelection('age > 18');
    const selection = new Selection(baseSelection.predicate, null, 10);

    const sql = SqlBuilder.withFields(['id', 'name', 'age']);
    sql.tableName('users');
    sql.selection(selection);
    const result = sql.build();

    expect(result.sql).toBe('SELECT "id", "name", "age" FROM "users" WHERE "age" > $1 LIMIT $2');
    assertArgs(result.args, [18, 10n]);
  });

  test('test_selection_with_order_by_and_limit', () => {
    const baseSelection = parseSelection("status = 'active'");
    const selection = new Selection(
      baseSelection.predicate,
      [
        new OrderByItem(PathExpr.simple('priority'), OrderDirection.Desc()),
        new OrderByItem(PathExpr.simple('created_at'), OrderDirection.Asc()),
      ],
      5,
    );

    const sql = SqlBuilder.withFields(['id', 'status', 'priority', 'created_at']);
    sql.tableName('tasks');
    sql.selection(selection);
    const result = sql.build();

    expect(result.sql).toBe(
      'SELECT "id", "status", "priority", "created_at" FROM "tasks" WHERE "status" = $1 ORDER BY "priority" DESC, "created_at" ASC LIMIT $2',
    );
    assertArgs(result.args, ['active', 5n]);
  });
});

// =========================================================================
// mod jsonb_sql_tests
// =========================================================================

describe('JSONB SQL Generation', () => {
  test('test_two_step_json_path', () => {
    // licensing.territory = 'US' should use -> and ::jsonb cast
    const selection = parseSelection("licensing.territory = 'US'");
    const sql = new SqlBuilder();
    sql.selection(selection);
    const result = sql.buildWhereClause();

    // String literal becomes '"US"'::jsonb (JSON string)
    expect(result.sql).toBe(`"licensing"->'territory' = '"US"'::jsonb`);
  });

  test('test_three_step_json_path', () => {
    // licensing.rights.holder should become "licensing"->'rights'->'holder'
    const selection = parseSelection("licensing.rights.holder = 'Label'");
    const sql = new SqlBuilder();
    sql.selection(selection);
    const result = sql.buildWhereClause();

    expect(result.sql).toBe(`"licensing"->'rights'->'holder' = '"Label"'::jsonb`);
  });

  test('test_four_step_json_path', () => {
    // a.b.c.d should become "a"->'b'->'c'->'d'
    const selection = parseSelection("a.b.c.d = 'value'");
    const sql = new SqlBuilder();
    sql.selection(selection);
    const result = sql.buildWhereClause();

    expect(result.sql).toBe(`"a"->'b'->'c'->'d' = '"value"'::jsonb`);
  });

  test('test_json_path_with_numeric_comparison', () => {
    // Using -> with ::jsonb ensures proper numeric comparison
    const selection = parseSelection('data.count > 10');
    const sql = new SqlBuilder();
    sql.selection(selection);
    const result = sql.buildWhereClause();

    expect(result.sql).toBe(`"data"->'count' > '10'::jsonb`);
  });

  test('test_mixed_simple_and_json_paths', () => {
    // name = 'test' AND data.status = 'active'
    // Simple path uses $1, JSON path uses ::jsonb cast
    const selection = parseSelection("name = 'test' AND data.status = 'active'");
    const sql = new SqlBuilder();
    sql.selection(selection);
    const result = sql.buildWhereClause();

    expect(result.sql).toBe(`"name" = $1 AND "data"->'status' = '"active"'::jsonb`);
  });

  test('test_json_path_escaping', () => {
    // Field with quote in path step - should escape properly
    const sql = new SqlBuilder();
    const path = new PathExpr(['data', "it's"]);
    sql.expr(Expr.Path(path));
    const result = sql.buildWhereClause();

    // Just the path, no comparison - still uses ->
    expect(result.sql).toBe(`"data"->'it''s'`);
  });

  test('test_json_path_with_boolean', () => {
    const selection = parseSelection('data.active = true');
    const sql = new SqlBuilder();
    sql.selection(selection);
    const result = sql.buildWhereClause();

    expect(result.sql).toBe(`"data"->'active' = 'true'::jsonb`);
  });

  test('test_json_path_with_float', () => {
    // Note: AnkQL parser may parse this as i64, but the principle stands
    const selection = parseSelection('data.score >= 95');
    const sql = new SqlBuilder();
    sql.selection(selection);
    const result = sql.buildWhereClause();

    expect(result.sql).toBe(`"data"->'score' >= '95'::jsonb`);
  });
});

// =========================================================================
// mod predicate_split_tests
// =========================================================================

describe('Predicate Split', () => {
  test('test_simple_predicate_fully_pushable', () => {
    const selection = parseSelection("name = 'Alice'");
    const split = splitPredicateForPostgres(selection.predicate);

    // Simple predicate should be fully pushable
    expect(split.needsPostFilter()).toBe(false);
    expect(split.remainingPredicate.is('True')).toBe(true);
  });

  test('test_json_path_predicate_pushable', () => {
    // Multi-step paths ARE pushed down using JSONB operators.
    const selection = parseSelection("licensing.territory = 'US'");
    const split = splitPredicateForPostgres(selection.predicate);

    // JSON path IS pushable via JSONB syntax
    expect(split.needsPostFilter()).toBe(false);
  });

  test('test_and_with_all_pushable', () => {
    const selection = parseSelection("name = 'test' AND licensing.status = 'active'");
    const split = splitPredicateForPostgres(selection.predicate);

    // Both parts pushable (simple path + JSON path) = whole thing pushable
    expect(split.needsPostFilter()).toBe(false);
  });

  test('test_or_with_all_pushable', () => {
    const selection = parseSelection("name = 'a' OR name = 'b'");
    const split = splitPredicateForPostgres(selection.predicate);

    // Both branches pushable = whole OR pushable
    expect(split.needsPostFilter()).toBe(false);
  });

  test('test_complex_nested_predicate', () => {
    const selection = parseSelection("(name = 'test' OR data.type = 'special') AND status = 'active'");
    const split = splitPredicateForPostgres(selection.predicate);

    // All parts are pushable (simple paths + JSON paths)
    expect(split.needsPostFilter()).toBe(false);
  });

  test('test_not_predicate_pushable', () => {
    const selection = parseSelection("NOT (status = 'deleted')");
    const split = splitPredicateForPostgres(selection.predicate);

    expect(split.needsPostFilter()).toBe(false);
  });

  test('test_is_null_pushable', () => {
    const selection = parseSelection('name IS NULL');
    const split = splitPredicateForPostgres(selection.predicate);

    expect(split.needsPostFilter()).toBe(false);
  });

  // Test for future: when we have unpushable predicates (e.g., Ref traversal)
  // test('test_unpushable_predicate_goes_to_remaining', () => {
  //   // When we add Ref traversal, this test would verify:
  //   // const selection = parseSelection("artist.name = 'Radiohead'");
  //   // const split = splitPredicateForPostgres(selection.predicate);
  //   // expect(split.needsPostFilter()).toBe(true);
  //   // expect(split.sqlPredicate.is('True')).toBe(true);
  // });
});
