// MIRRORS: ankurah/ankql/src/ast.rs

import { InvalidPredicateError, ParseError } from './error.ts';

// ── Expression types ─────────────────────────────────────────────────

export type Expr =
  | { type: 'Literal'; value: Literal }
  | { type: 'Path'; value: PathExpr }
  | { type: 'Predicate'; value: Predicate }
  | { type: 'InfixExpr'; left: Expr; operator: InfixOperator; right: Expr }
  | { type: 'ExprList'; values: Expr[] }
  | { type: 'Placeholder' };

export type Literal =
  | { type: 'I16'; value: number }
  | { type: 'I32'; value: number }
  | { type: 'I64'; value: bigint }
  | { type: 'F64'; value: number }
  | { type: 'Bool'; value: boolean }
  | { type: 'String'; value: string }
  | { type: 'EntityId'; value: Uint8Array }
  | { type: 'Object'; value: Uint8Array }
  | { type: 'Binary'; value: Uint8Array }
  | { type: 'Json'; value: unknown };

export type InfixOperator = 'Add' | 'Subtract' | 'Multiply' | 'Divide';

export type ComparisonOperator =
  | 'Equal'
  | 'NotEqual'
  | 'GreaterThan'
  | 'GreaterThanOrEqual'
  | 'LessThan'
  | 'LessThanOrEqual'
  | 'In'
  | 'Between';

export type Predicate =
  | { type: 'Comparison'; left: Expr; operator: ComparisonOperator; right: Expr }
  | { type: 'IsNull'; expr: Expr }
  | { type: 'And'; left: Predicate; right: Predicate }
  | { type: 'Or'; left: Predicate; right: Predicate }
  | { type: 'Not'; predicate: Predicate }
  | { type: 'True' }
  | { type: 'False' }
  | { type: 'Placeholder' };

export type OrderDirection = 'Asc' | 'Desc';

export interface OrderByItem {
  path: PathExpr;
  direction: OrderDirection;
}

// ── PathExpr ─────────────────────────────────────────────────────────

export class PathExpr {
  steps: string[];

  constructor(steps: string[]) {
    this.steps = steps;
  }

  static simple(name: string): PathExpr {
    return new PathExpr([name]);
  }

  isSimple(): boolean {
    return this.steps.length === 1;
  }

  first(): string {
    return this.steps[0];
  }

  property(): string {
    return this.steps[this.steps.length - 1];
  }

  toString(): string {
    return this.steps.join('.');
  }
}

// ── Selection ────────────────────────────────────────────────────────

export class Selection {
  predicate: Predicate;
  orderBy: OrderByItem[] | null;
  limit: number | null;

  constructor(
    predicate: Predicate,
    orderBy: OrderByItem[] | null = null,
    limit: number | null = null,
  ) {
    this.predicate = predicate;
    this.orderBy = orderBy;
    this.limit = limit;
  }

  /** Transform the selection to assume the given columns are NULL. */
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

  toString(): string {
    // Import would be circular, so we inline a basic representation
    let result = predicateToString(this.predicate);
    if (this.orderBy) {
      result += ' ORDER BY ';
      result += this.orderBy
        .map((item) => {
          const dir = item.direction === 'Asc' ? 'ASC' : 'DESC';
          return `${item.path.toString()} ${dir}`;
        })
        .join(', ');
    }
    if (this.limit !== null) {
      result += ` LIMIT ${this.limit}`;
    }
    return result;
  }
}

// ── Predicate functions ──────────────────────────────────────────────

/** Recursively walk a predicate tree and accumulate results using a visitor */
export function walkPredicate<T>(
  pred: Predicate,
  acc: T,
  visitor: (acc: T, pred: Predicate) => T,
): T {
  let result = visitor(acc, pred);
  switch (pred.type) {
    case 'And':
    case 'Or':
      result = walkPredicate(pred.left, result, visitor);
      result = walkPredicate(pred.right, result, visitor);
      break;
    case 'Not':
      result = walkPredicate(pred.predicate, result, visitor);
      break;
  }
  return result;
}

/**
 * Collect all column names referenced in this predicate.
 * For JSON paths like `licensing.territory`, returns the column name (`licensing`),
 * not the JSON path step (`territory`).
 */
export function referencedColumns(pred: Predicate): string[] {
  return walkPredicate<string[]>(pred, [], (cols, p) => {
    switch (p.type) {
      case 'Comparison': {
        for (const expr of [p.left, p.right]) {
          if (expr.type === 'Path') {
            const col = expr.value.first();
            if (!cols.includes(col)) {
              cols.push(col);
            }
          }
        }
        break;
      }
      case 'IsNull': {
        if (p.expr.type === 'Path') {
          const col = p.expr.value.first();
          if (!cols.includes(col)) {
            cols.push(col);
          }
        }
        break;
      }
    }
    return cols;
  });
}

/**
 * Clones the predicate tree and evaluates comparisons involving missing columns
 * as if they were NULL.
 */
export function assumeNull(pred: Predicate, columns: string[]): Predicate {
  switch (pred.type) {
    case 'Comparison': {
      const hasNullPath = (() => {
        if (pred.left.type === 'Path' && columns.includes(pred.left.value.property())) return true;
        if (pred.right.type === 'Path' && columns.includes(pred.right.value.property())) return true;
        return false;
      })();

      if (hasNullPath) {
        // Any comparison with NULL is false in SQL semantics
        return { type: 'False' };
      }
      return pred;
    }
    case 'IsNull': {
      if (pred.expr.type === 'Path') {
        const isNull = columns.includes(pred.expr.value.property());
        if (isNull) return { type: 'True' };
      }
      return pred;
    }
    case 'And': {
      const left = assumeNull(pred.left, columns);
      const right = assumeNull(pred.right, columns);

      if (left.type === 'False' || right.type === 'False') return { type: 'False' };
      if (left.type === 'True' && right.type === 'True') return { type: 'True' };
      if (left.type === 'True') return right;
      if (right.type === 'True') return left;
      return { type: 'And', left, right };
    }
    case 'Or': {
      const left = assumeNull(pred.left, columns);
      const right = assumeNull(pred.right, columns);

      if (left.type === 'True' || right.type === 'True') return { type: 'True' };
      if (left.type === 'False' && right.type === 'False') return { type: 'False' };
      if (left.type === 'False') return right;
      if (right.type === 'False') return left;
      return { type: 'Or', left, right };
    }
    case 'Not': {
      const inner = assumeNull(pred.predicate, columns);
      if (inner.type === 'True') return { type: 'False' };
      if (inner.type === 'False') return { type: 'True' };
      return { type: 'Not', predicate: inner };
    }
    case 'True':
    case 'False':
    case 'Placeholder':
      return pred;
    default:
      return pred;
  }
}

/** Populate placeholders in the predicate with actual values */
export function populatePredicate(pred: Predicate, values: Iterable<Expr>): Predicate {
  const iter = values[Symbol.iterator]();
  const state = { iter, done: false };
  const result = populatePredicateRecursive(pred, state);
  // Check if there are unused values
  const next = state.iter.next();
  if (!next.done) {
    throw new InvalidPredicateError('Too many values provided for placeholders');
  }
  return result;
}

interface IterState {
  iter: Iterator<Expr>;
  done: boolean;
}

function populatePredicateRecursive(pred: Predicate, state: IterState): Predicate {
  switch (pred.type) {
    case 'Comparison':
      return {
        type: 'Comparison',
        left: populateExprRecursive(pred.left, state),
        operator: pred.operator,
        right: populateExprRecursive(pred.right, state),
      };
    case 'And':
      return {
        type: 'And',
        left: populatePredicateRecursive(pred.left, state),
        right: populatePredicateRecursive(pred.right, state),
      };
    case 'Or':
      return {
        type: 'Or',
        left: populatePredicateRecursive(pred.left, state),
        right: populatePredicateRecursive(pred.right, state),
      };
    case 'Not':
      return {
        type: 'Not',
        predicate: populatePredicateRecursive(pred.predicate, state),
      };
    case 'IsNull':
      return {
        type: 'IsNull',
        expr: populateExprRecursive(pred.expr, state),
      };
    case 'True':
      return { type: 'True' };
    case 'False':
      return { type: 'False' };
    case 'Placeholder':
      throw new InvalidPredicateError('Placeholder must be transformed before population');
    default:
      return pred;
  }
}

function populateExprRecursive(expr: Expr, state: IterState): Expr {
  switch (expr.type) {
    case 'Placeholder': {
      const next = state.iter.next();
      if (next.done) {
        throw new InvalidPredicateError('Not enough values provided for placeholders');
      }
      return next.value;
    }
    case 'Literal':
      return expr;
    case 'Path':
      return expr;
    case 'Predicate':
      return { type: 'Predicate', value: populatePredicateRecursive(expr.value, state) };
    case 'InfixExpr':
      return {
        type: 'InfixExpr',
        left: populateExprRecursive(expr.left, state),
        operator: expr.operator,
        right: populateExprRecursive(expr.right, state),
      };
    case 'ExprList':
      return {
        type: 'ExprList',
        values: expr.values.map((e) => populateExprRecursive(e, state)),
      };
    default:
      return expr;
  }
}

// ── Expr conversion helpers (mirrors Rust From impls) ────────────────

export function exprFromString(s: string): Expr {
  return { type: 'Literal', value: { type: 'String', value: s } };
}

export function exprFromI64(i: bigint): Expr {
  return { type: 'Literal', value: { type: 'I64', value: i } };
}

export function exprFromF64(f: number): Expr {
  return { type: 'Literal', value: { type: 'F64', value: f } };
}

export function exprFromBool(b: boolean): Expr {
  return { type: 'Literal', value: { type: 'Bool', value: b } };
}

// ── Expr to Predicate conversion (mirrors Rust TryFrom<Expr> for Predicate) ──

export function exprToPredicate(expr: Expr): Predicate {
  switch (expr.type) {
    case 'Predicate':
      return expr.value;
    case 'Placeholder':
      return { type: 'Placeholder' };
    case 'Literal':
      if (expr.value.type === 'Bool') {
        return expr.value.value ? { type: 'True' } : { type: 'False' };
      }
      throw new InvalidPredicateError('Expression is not a predicate');
    default:
      throw new InvalidPredicateError('Expression is not a predicate');
  }
}

// ── Display helpers ──────────────────────────────────────────────────

/** A minimal predicate-to-string that does NOT depend on selection/sql to avoid circular imports.
 *  The full SQL generation is in selection/sql.ts.
 */
function predicateToString(pred: Predicate): string {
  switch (pred.type) {
    case 'Comparison': {
      const opStr = comparisonOpToStr(pred.operator);
      return `${exprToString(pred.left)} ${opStr} ${exprToString(pred.right)}`;
    }
    case 'IsNull':
      return `${exprToString(pred.expr)} IS NULL`;
    case 'And':
      return `${predicateToString(pred.left)} AND ${predicateToString(pred.right)}`;
    case 'Or':
      return `(${predicateToString(pred.left)} OR ${predicateToString(pred.right)})`;
    case 'Not':
      return `NOT (${predicateToString(pred.predicate)})`;
    case 'True':
      return 'TRUE';
    case 'False':
      return 'FALSE';
    case 'Placeholder':
      return '?';
  }
}

function comparisonOpToStr(op: ComparisonOperator): string {
  switch (op) {
    case 'Equal': return '=';
    case 'NotEqual': return '<>';
    case 'GreaterThan': return '>';
    case 'GreaterThanOrEqual': return '>=';
    case 'LessThan': return '<';
    case 'LessThanOrEqual': return '<=';
    case 'In': return 'IN';
    case 'Between': return 'BETWEEN';
  }
}

function exprToString(expr: Expr): string {
  switch (expr.type) {
    case 'Literal':
      return literalToString(expr.value);
    case 'Path':
      return expr.value.steps.map((s) => `"${s}"`).join('.');
    case 'Predicate':
      return predicateToString(expr.value);
    case 'InfixExpr':
      return `${exprToString(expr.left)} ${expr.operator} ${exprToString(expr.right)}`;
    case 'ExprList':
      return `(${expr.values.map(exprToString).join(', ')})`;
    case 'Placeholder':
      return '?';
  }
}

function literalToString(lit: Literal): string {
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
    case 'String':
      return `'${lit.value.replace(/'/g, "''")}'`;
    case 'EntityId':
    case 'Object':
    case 'Binary':
      return `<bytes>`;
    case 'Json':
      return JSON.stringify(lit.value);
  }
}
