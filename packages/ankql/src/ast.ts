// MIRRORS: ankurah/ankql/src/ast.rs
// Rust: mod json_as_bytes — SKIP: serde-specific custom serializer, not needed in TS
// Rust: fn serialize (json_as_bytes) — SKIP: serde-specific
// Rust: fn deserialize (json_as_bytes) — SKIP: serde-specific

import { Struct, Enum } from '@ankurah/base';
import { ParseError } from './error.ts';

// ── Expr ──────────────────────────────────────────────────────────────

type ExprV = {
  Literal: { literal: Literal };
  Path: { path: PathExpr };
  Predicate: { predicate: Predicate };
  InfixExpr: { left: Expr; operator: InfixOperator; right: Expr };
  ExprList: { exprs: Expr[] };
  Placeholder: {};
};

export class Expr extends Enum<ExprV> {
  static Literal(literal: Literal): Expr { return new Expr('Literal', { literal }); }
  static Path(path: PathExpr): Expr { return new Expr('Path', { path }); }
  static Predicate(predicate: Predicate): Expr { return new Expr('Predicate', { predicate }); }
  static InfixExpr(left: Expr, operator: InfixOperator, right: Expr): Expr { return new Expr('InfixExpr', { left, operator, right }); }
  static ExprList(exprs: Expr[]): Expr { return new Expr('ExprList', { exprs }); }
  static Placeholder(): Expr { return new Expr('Placeholder', {}); }

  // Rust: fn populate_recursive
  populateRecursive(values: Iterator<Expr>): Expr {
    return this.match({
      Placeholder: () => {
        const next = values.next();
        if (next.done) {
          throw new ParseError('InvalidPredicate', { _0: 'Not enough values provided for placeholders' });
        }
        return next.value;
      },
      Literal: () => this,
      Path: () => this,
      Predicate: (v) => Expr.Predicate(v.predicate.populateRecursive(values)),
      InfixExpr: (v) => {
        const left = v.left.populateRecursive(values);
        try {
          const right = v.right.populateRecursive(values);
          return Expr.InfixExpr(left, v.operator, right);
        } catch (e) {
          left.drop();
          throw e;
        }
      },
      ExprList: (v) => {
        const results: Expr[] = [];
        try {
          for (const e of v.exprs) {
            results.push(e.populateRecursive(values));
          }
          return Expr.ExprList(results);
        } catch (e) {
          for (const r of results) r.drop();
          throw e;
        }
      },
    });
  }
}

// ── Literal ───────────────────────────────────────────────────────────

type LiteralV = {
  I16: { value: number };
  I32: { value: number };
  I64: { value: bigint };
  F64: { value: number };
  Bool: { value: boolean };
  String: { value: string };
  EntityId: { value: Uint8Array };
  Object: { value: Uint8Array };
  Binary: { value: Uint8Array };
  Json: { value: unknown };
};

export class Literal extends Enum<LiteralV> {
  static I16(value: number): Literal { return new Literal('I16', { value }); }
  static I32(value: number): Literal { return new Literal('I32', { value }); }
  static I64(value: bigint): Literal { return new Literal('I64', { value }); }
  static F64(value: number): Literal { return new Literal('F64', { value }); }
  static Bool(value: boolean): Literal { return new Literal('Bool', { value }); }
  static String(value: string): Literal { return new Literal('String', { value }); }
  static EntityId(value: Uint8Array): Literal { return new Literal('EntityId', { value }); }
  static Object(value: Uint8Array): Literal { return new Literal('Object', { value }); }
  static Binary(value: Uint8Array): Literal { return new Literal('Binary', { value }); }
  static Json(value: unknown): Literal { return new Literal('Json', { value }); }
}

// ── PathExpr ──────────────────────────────────────────────────────────

export class PathExpr extends Struct {
  steps: string[];

  constructor(steps: string[]) {
    super();
    this.steps = steps;
  }

  // Rust: fn simple
  /** Create a single-step path */
  static simple(name: string): PathExpr {
    return new PathExpr([name]);
  }

  // Rust: fn is_simple
  /** Check if this is a single-step path */
  isSimple(): boolean {
    return this.steps.length === 1;
  }

  // Rust: fn first
  /** Get the first step (always exists) */
  first(): string {
    return this.steps[0];
  }

  // Rust: fn property
  /** Get the property name (last step) */
  property(): string {
    return this.steps[this.steps.length - 1];
  }

  // Rust: fn fmt
  override toString(): string {
    return this.steps.join('.');
  }
}

// ── Selection ─────────────────────────────────────────────────────────

export class Selection extends Struct {
  predicate: Predicate;
  orderBy: OrderByItem[] | null;
  limit: number | null;

  constructor(
    predicate: Predicate,
    orderBy: OrderByItem[] | null = null,
    limit: number | null = null,
  ) {
    super();
    this.predicate = predicate;
    this.orderBy = orderBy;
    this.limit = limit;
  }

  // Rust: fn from
  /** Backward compatibility: From<Predicate> for Selection */
  static fromPredicate(predicate: Predicate): Selection {
    return new Selection(predicate, null, null);
  }

  // Rust: fn fmt
  override toString(): string {
    let result = predicateToString(this.predicate);
    if (this.orderBy) {
      result += ' ORDER BY ';
      result += this.orderBy
        .map((item) => item.toString())
        .join(', ');
    }
    if (this.limit !== null) {
      result += ` LIMIT ${this.limit}`;
    }
    return result;
  }

  // Rust: fn assume_null
  /**
   * Transform the selection to assume the given columns are NULL.
   * This filters out ORDER BY items that reference missing columns.
   */
  assumeNull(columns: string[]): Selection {
    let orderBy = this.orderBy
      ? this.orderBy.filter((item) => {
          const colName = item.path.property();
          return !columns.includes(colName);
        })
      : null;
    // If all ORDER BY items were filtered out, set to null
    if (orderBy && orderBy.length === 0) orderBy = null;

    return new Selection(
      assumeNull(this.predicate, columns),
      orderBy,
      this.limit,
    );
  }

  // Rust: fn referenced_columns
  /**
   * Collect all column names referenced in this selection (WHERE + ORDER BY).
   * For JSON paths like `licensing.territory`, returns the column name (`licensing`),
   * not the JSON path step (`territory`).
   */
  referencedColumns(): string[] {
    const columns = referencedColumns(this.predicate);
    if (this.orderBy) {
      for (const item of this.orderBy) {
        const col = item.path.first();
        if (!columns.includes(col)) {
          columns.push(col);
        }
      }
    }
    return columns;
  }

  // TODO: proper bincode serde when ankql is transpiled
  encode(writer: any): void {
    throw new Error('Selection.encode not yet implemented');
  }

  static decode(reader: any): Selection {
    throw new Error('Selection.decode not yet implemented');
  }
}

// ── OrderByItem ───────────────────────────────────────────────────────

export class OrderByItem extends Struct {
  path: PathExpr;
  direction: OrderDirection;

  constructor(path: PathExpr, direction: OrderDirection) {
    super();
    this.path = path;
    this.direction = direction;
  }

  // Rust: fn fmt
  override toString(): string {
    const dir = this.direction.is('Asc') ? 'ASC' : 'DESC';
    return `${this.path.toString()} ${dir}`;
  }
}

// ── OrderDirection ────────────────────────────────────────────────────

type OrderDirectionV = {
  Asc: {};
  Desc: {};
};

export class OrderDirection extends Enum<OrderDirectionV> {
  static Asc(): OrderDirection { return new OrderDirection('Asc', {}); }
  static Desc(): OrderDirection { return new OrderDirection('Desc', {}); }
}

// ── Predicate ─────────────────────────────────────────────────────────

type PredicateV = {
  Comparison: { left: Expr; operator: ComparisonOperator; right: Expr };
  IsNull: { expr: Expr };
  And: { left: Predicate; right: Predicate };
  Or: { left: Predicate; right: Predicate };
  Not: { predicate: Predicate };
  True: {};
  False: {};
  Placeholder: {};
};

export class Predicate extends Enum<PredicateV> {
  static Comparison(left: Expr, operator: ComparisonOperator, right: Expr): Predicate { return new Predicate('Comparison', { left, operator, right }); }
  static IsNull(expr: Expr): Predicate { return new Predicate('IsNull', { expr }); }
  static And(left: Predicate, right: Predicate): Predicate { return new Predicate('And', { left, right }); }
  static Or(left: Predicate, right: Predicate): Predicate { return new Predicate('Or', { left, right }); }
  static Not(predicate: Predicate): Predicate { return new Predicate('Not', { predicate }); }
  static True(): Predicate { return new Predicate('True', {}); }
  static False(): Predicate { return new Predicate('False', {}); }
  static Placeholder(): Predicate { return new Predicate('Placeholder', {}); }

  // Rust: fn walk
  /** Recursively walk a predicate tree and accumulate results using a closure */
  walk<T>(accumulator: T, visitor: (acc: T, pred: Predicate) => T): T {
    let result = visitor(accumulator, this);
    return this.match({
      And: (v) => {
        result = v.left.walk(result, visitor);
        return v.right.walk(result, visitor);
      },
      Or: (v) => {
        result = v.left.walk(result, visitor);
        return v.right.walk(result, visitor);
      },
      Not: (v) => v.predicate.walk(result, visitor),
      Comparison: () => result,
      IsNull: () => result,
      True: () => result,
      False: () => result,
      Placeholder: () => result,
    });
  }

  // Rust: fn referenced_columns
  /**
   * Collect all column names referenced in this predicate.
   * For JSON paths like `licensing.territory`, returns the column name (`licensing`),
   * not the JSON path step (`territory`).
   */
  referencedColumns(): string[] {
    return this.walk<string[]>([], (cols, pred) => {
      if (pred.is('Comparison')) {
        const v = pred.value as PredicateV['Comparison'];
        for (const expr of [v.left, v.right]) {
          if (expr.is('Path')) {
            const path = (expr.value as ExprV['Path']).path;
            const col = path.first();
            if (!cols.includes(col)) {
              cols.push(col);
            }
          }
        }
      } else if (pred.is('IsNull')) {
        const v = pred.value as PredicateV['IsNull'];
        if (v.expr.is('Path')) {
          const path = (v.expr.value as ExprV['Path']).path;
          const col = path.first();
          if (!cols.includes(col)) {
            cols.push(col);
          }
        }
      }
      return cols;
    });
  }

  // Rust: fn assume_null
  /** Clones the predicate tree and evaluates comparisons involving missing columns as if they were NULL */
  assumeNull(columns: string[]): Predicate {
    return assumeNull(this, columns);
  }

  // Rust: fn populate
  /** Populate placeholders in the predicate with actual values */
  populate(values: Iterable<Expr>): Predicate {
    const iter = values[Symbol.iterator]();
    const result = this.populateRecursive(iter);
    // Check if there are any unused values
    const next = iter.next();
    if (!next.done) {
      result.drop(); // Clean up the populated result before throwing
      throw new ParseError('InvalidPredicate', { _0: 'Too many values provided for placeholders' });
    }
    return result;
  }

  // Rust: fn populate_recursive
  populateRecursive(values: Iterator<Expr>): Predicate {
    return this.match({
      Comparison: (v) => {
        const left = v.left.populateRecursive(values);
        try {
          const right = v.right.populateRecursive(values);
          return Predicate.Comparison(left, v.operator, right);
        } catch (e) {
          left.drop();
          throw e;
        }
      },
      And: (v) => {
        const left = v.left.populateRecursive(values);
        try {
          const right = v.right.populateRecursive(values);
          return Predicate.And(left, right);
        } catch (e) {
          left.drop();
          throw e;
        }
      },
      Or: (v) => {
        const left = v.left.populateRecursive(values);
        try {
          const right = v.right.populateRecursive(values);
          return Predicate.Or(left, right);
        } catch (e) {
          left.drop();
          throw e;
        }
      },
      Not: (v) => Predicate.Not(v.predicate.populateRecursive(values)),
      IsNull: (v) => Predicate.IsNull(v.expr.populateRecursive(values)),
      True: () => Predicate.True(),
      False: () => Predicate.False(),
      Placeholder: () => { throw new ParseError('InvalidPredicate', { _0: 'Placeholder must be transformed before population' }); },
    });
  }
}

// ── ComparisonOperator ────────────────────────────────────────────────

type ComparisonOperatorV = {
  Equal: {};         // =
  NotEqual: {};      // <> or !=
  GreaterThan: {};   // >
  GreaterThanOrEqual: {}; // >=
  LessThan: {};      // <
  LessThanOrEqual: {}; // <=
  In: {};            // IN
  Between: {};       // BETWEEN
};

export class ComparisonOperator extends Enum<ComparisonOperatorV> {
  static Equal(): ComparisonOperator { return new ComparisonOperator('Equal', {}); }
  static NotEqual(): ComparisonOperator { return new ComparisonOperator('NotEqual', {}); }
  static GreaterThan(): ComparisonOperator { return new ComparisonOperator('GreaterThan', {}); }
  static GreaterThanOrEqual(): ComparisonOperator { return new ComparisonOperator('GreaterThanOrEqual', {}); }
  static LessThan(): ComparisonOperator { return new ComparisonOperator('LessThan', {}); }
  static LessThanOrEqual(): ComparisonOperator { return new ComparisonOperator('LessThanOrEqual', {}); }
  static In(): ComparisonOperator { return new ComparisonOperator('In', {}); }
  static Between(): ComparisonOperator { return new ComparisonOperator('Between', {}); }
}

// ── InfixOperator ─────────────────────────────────────────────────────

type InfixOperatorV = {
  Add: {};
  Subtract: {};
  Multiply: {};
  Divide: {};
};

export class InfixOperator extends Enum<InfixOperatorV> {
  static Add(): InfixOperator { return new InfixOperator('Add', {}); }
  static Subtract(): InfixOperator { return new InfixOperator('Subtract', {}); }
  static Multiply(): InfixOperator { return new InfixOperator('Multiply', {}); }
  static Divide(): InfixOperator { return new InfixOperator('Divide', {}); }
}

// ── Free functions (mirror Rust impl methods for external callers) ───

// Rust: fn walk — SKIP: free function wrapper, Rust fn is the impl method above
/** Recursively walk a predicate tree and accumulate results using a visitor */
export function walkPredicate<T>(
  pred: Predicate,
  acc: T,
  visitor: (acc: T, pred: Predicate) => T,
): T {
  return pred.walk(acc, visitor);
}

// Rust: fn referenced_columns — SKIP: free function wrapper, Rust fn is the impl method above
/**
 * Collect all column names referenced in this predicate.
 * For JSON paths like `licensing.territory`, returns the column name (`licensing`),
 * not the JSON path step (`territory`).
 */
export function referencedColumns(pred: Predicate): string[] {
  return pred.referencedColumns();
}

// Rust: fn assume_null
/**
 * Clones the predicate tree and evaluates comparisons involving missing columns
 * as if they were NULL.
 */
export function assumeNull(pred: Predicate, columns: string[]): Predicate {
  return pred.match({
    Comparison: (v) => {
      const hasNullPath = (() => {
        if (v.left.is('Path') && columns.includes((v.left.value as ExprV['Path']).path.property())) return true;
        if (v.right.is('Path') && columns.includes((v.right.value as ExprV['Path']).path.property())) return true;
        return false;
      })();

      if (hasNullPath) {
        // Any comparison with NULL is false in SQL semantics
        return Predicate.False();
      }
      return Predicate.Comparison(v.left, v.operator, v.right);
    },
    IsNull: (v) => {
      if (v.expr.is('Path')) {
        const path = (v.expr.value as ExprV['Path']).path;
        if (columns.includes(path.property())) {
          return Predicate.True();
        }
      }
      return Predicate.IsNull(v.expr);
    },
    And: (v) => {
      const left = assumeNull(v.left, columns);
      const right = assumeNull(v.right, columns);

      if (left.is('False') || right.is('False')) { left.drop(); right.drop(); return Predicate.False(); }
      if (left.is('True') && right.is('True')) { left.drop(); right.drop(); return Predicate.True(); }
      if (left.is('True')) { left.drop(); return right; }
      if (right.is('True')) { right.drop(); return left; }
      return Predicate.And(left, right);
    },
    Or: (v) => {
      const left = assumeNull(v.left, columns);
      const right = assumeNull(v.right, columns);

      if (left.is('True') || right.is('True')) { left.drop(); right.drop(); return Predicate.True(); }
      if (left.is('False') && right.is('False')) { left.drop(); right.drop(); return Predicate.False(); }
      if (left.is('False')) { left.drop(); return right; }
      if (right.is('False')) { right.drop(); return left; }
      return Predicate.Or(left, right);
    },
    Not: (v) => {
      const inner = assumeNull(v.predicate, columns);
      if (inner.is('True')) { inner.drop(); return Predicate.False(); }
      if (inner.is('False')) { inner.drop(); return Predicate.True(); }
      return Predicate.Not(inner);
    },
    True: () => Predicate.True(),
    False: () => Predicate.False(),
    Placeholder: () => Predicate.Placeholder(),
  });
}

// Rust: fn populate — SKIP: free function wrapper, Rust fn is the impl method above
/** Populate placeholders in the predicate with actual values */
export function populatePredicate(pred: Predicate, values: Iterable<Expr>): Predicate {
  return pred.populate(values);
}

// ── Expr conversion helpers (mirrors Rust From impls) ────────────────

// Rust: fn from (From<String> for Expr)
export function exprFromString(s: string): Expr {
  return Expr.Literal(Literal.String(s));
}

// Rust: fn from (From<i64> for Expr)
export function exprFromI64(i: bigint): Expr {
  return Expr.Literal(Literal.I64(i));
}

// Rust: fn from (From<f64> for Expr)
export function exprFromF64(f: number): Expr {
  return Expr.Literal(Literal.F64(f));
}

// Rust: fn from (From<bool> for Expr)
export function exprFromBool(b: boolean): Expr {
  return Expr.Literal(Literal.Bool(b));
}

// Rust: fn from (From<Literal> for Expr)
export function exprFromLiteral(lit: Literal): Expr {
  return Expr.Literal(lit);
}

// Rust: fn from (From<Vec<T>> for Expr, From<[T;N]> for Expr, From<&[T]> for Expr, From<&[T;N]> for Expr) — SKIP: generic From trait pattern not applicable in TS [E4]
// Divergence: In TS, use Expr.ExprList(items) directly.

// ── Expr to Predicate conversion (mirrors Rust TryFrom<Expr> for Predicate) ──

// Rust: fn try_from (TryFrom<Expr> for Predicate)
export function exprToPredicate(expr: Expr): Predicate {
  const result = expr.match({
    Predicate: (v) => {
      // Move inner predicate out before dropping the Expr wrapper
      const pred = v.predicate;
      (v as any).predicate = null; // detach so drop() won't cascade into it
      return pred;
    },
    Placeholder: () => Predicate.Placeholder(),
    Literal: (v) => {
      if (v.literal.is('Bool')) {
        return (v.literal.value as LiteralV['Bool']).value ? Predicate.True() : Predicate.False();
      }
      throw new ParseError('InvalidPredicate', { _0: 'Expression is not a predicate' });
    },
    Path: () => { throw new ParseError('InvalidPredicate', { _0: 'Expression is not a predicate' }); },
    InfixExpr: () => { throw new ParseError('InvalidPredicate', { _0: 'Expression is not a predicate' }); },
    ExprList: () => { throw new ParseError('InvalidPredicate', { _0: 'Expression is not a predicate' }); },
  });
  expr.drop(); // Drop the consumed Expr wrapper (Rust: implicit drop on move)
  return result;
}

// ── Display helpers ──────────────────────────────────────────────────

// Divergence: Rust Display impl for Predicate calls generate_selection_sql; TS uses a local
// predicateToString to avoid circular imports between ast.ts and selection/sql.ts [E4]

// Rust: fn fmt (Display for Predicate)
/** A minimal predicate-to-string that does NOT depend on selection/sql to avoid circular imports. */
function predicateToString(pred: Predicate): string {
  return pred.match({
    Comparison: (v) => {
      const opStr = comparisonOpToStr(v.operator);
      return `${exprToString(v.left)} ${opStr} ${exprToString(v.right)}`;
    },
    IsNull: (v) => `${exprToString(v.expr)} IS NULL`,
    And: (v) => `${predicateToString(v.left)} AND ${predicateToString(v.right)}`,
    Or: (v) => `(${predicateToString(v.left)} OR ${predicateToString(v.right)})`,
    Not: (v) => `NOT (${predicateToString(v.predicate)})`,
    True: () => 'TRUE',
    False: () => 'FALSE',
    Placeholder: () => '?',
  });
}

function comparisonOpToStr(op: ComparisonOperator): string {
  return op.match({
    Equal: () => '=',
    NotEqual: () => '<>',
    GreaterThan: () => '>',
    GreaterThanOrEqual: () => '>=',
    LessThan: () => '<',
    LessThanOrEqual: () => '<=',
    In: () => 'IN',
    Between: () => 'BETWEEN',
  });
}

function exprToString(expr: Expr): string {
  return expr.match({
    Literal: (v) => literalToString(v.literal),
    Path: (v) => v.path.steps.map((s) => `"${s}"`).join('.'),
    Predicate: (v) => predicateToString(v.predicate),
    InfixExpr: (v) => `${exprToString(v.left)} ${v.operator.type} ${exprToString(v.right)}`,
    ExprList: (v) => `(${v.exprs.map(exprToString).join(', ')})`,
    Placeholder: () => '?',
  });
}

function literalToString(lit: Literal): string {
  return lit.match({
    I16: (v) => String(v.value),
    I32: (v) => String(v.value),
    I64: (v) => String(v.value),
    F64: (v) => String(v.value),
    Bool: (v) => v.value ? 'true' : 'false',
    String: (v) => `'${v.value.replace(/'/g, "''")}'`,
    EntityId: () => '<bytes>',
    Object: () => '<bytes>',
    Binary: () => '<bytes>',
    Json: (v) => JSON.stringify(v.value),
  });
}
