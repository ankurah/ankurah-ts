// MIRRORS: ankurah/ankql/src/parser.rs
// Rust: fn print_tree — SKIP: test-only debug helper
// Rust: fn debug_print_pairs — SKIP: test-only debug helper
//
// This file walks the pair tree grammar.ts produces and builds the AST, the same
// way parser.rs walks pest's. The split matters for more than tidiness: it is the
// line the fixtures pin. A `SyntaxError` means the grammar refused the text; every
// other ParseError variant means the grammar accepted it and AST construction then
// refused it. Keeping the grammar in grammar.ts and the AST construction here is
// what keeps a port on the same side of that line as the Rust parser.

import { type Pair, type Rule, parseSelectionRule } from './grammar.ts';
import {
  Expr,
  Literal,
  Predicate,
  ComparisonOperator,
  OrderByItem,
  OrderDirection,
  PathExpr,
  Selection,
  exprToPredicate,
} from './ast.ts';
import { ParseError } from './error.ts';

const I32_MIN = -2147483648n;
const I32_MAX = 2147483647n;
const I64_MAX = 9223372036854775807n;
const U64_MAX = 18446744073709551615n;

/**
 * `Pairs` — pest's iterator over a rule's children. It is an iterator rather than an
 * array on purpose: parse_expr hands it to create_logical_op, which pulls a further
 * operator and operand out of it, and the loop in parse_expr then carries on from
 * wherever create_logical_op left off. That shared cursor is how `a = 1 OR b = 2 AND
 * c = 3` folds left to right.
 */
class Pairs {
  private index = 0;

  constructor(private readonly pairs: readonly Pair[]) {}

  next(): Pair | undefined {
    return this.index < this.pairs.length ? this.pairs[this.index++] : undefined;
  }
}

// Rust: fn parse_selection
/**
 * Parse a selection expression into a Selection AST.
 * The selection includes a predicate and optional ORDER BY and LIMIT clauses.
 */
export function parseSelection(input: string): Selection {
  // TODO: Improve grammar to handle these cases more elegantly
  if (input.trim() === '') {
    return new Selection(Predicate.True(), null, null);
  }

  const parsed = parseSelectionRule(input);
  if (!parsed.ok) throw new ParseError('SyntaxError', { _0: parsed.message });

  let predicate: Predicate | null = null;
  let orderBy: OrderByItem[] | null = null;
  let limit: bigint | null = null;

  try {
    for (const pair of parsed.pairs) {
      switch (pair.rule) {
        case 'Expr':
          predicate = parseExpr(pair);
          break;
        case 'OrderByClause':
          orderBy = parseOrderByClause(pair);
          break;
        case 'LimitClause':
          limit = parseLimitClause(pair);
          break;
        case 'EOI':
          break; // End of input, ignore
        default:
          throw new ParseError('UnexpectedRule', { expected: 'Expr, OrderByClause, or LimitClause', got: pair.rule });
      }
    }
  } catch (e) {
    // Rust drops the half-built Selection here; TS has to say so.
    predicate?.drop();
    if (orderBy) for (const item of orderBy) item.drop();
    throw e;
  }

  if (predicate === null) throw new ParseError('EmptyExpression', {});

  return new Selection(predicate, orderBy, limit);
}

// Rust: fn parse_expr
/** Parse a boolean expression, which can be a comparison, AND, or OR expression */
function parseExpr(pair: Pair): Predicate {
  if (pair.rule !== 'Expr') throw new Error('Expected Expr rule');
  const pairs = new Pairs(pair.inner);

  // Parse the first value
  const first = pairs.next();
  if (first === undefined) throw new ParseError('MissingOperand', { _0: 'first' });

  // handle unary operators which have precedence over infix operators
  if (first.rule === 'UnaryNot') {
    const next = pairs.next();
    if (next === undefined) throw new ParseError('EmptyExpression', {});
    if (next.rule !== 'ExpressionInParentheses') {
      // NOT only works over parentheses: `NOT (a = 1)` parses, `NOT a = 1` lands here.
      throw new ParseError('UnexpectedRule', { expected: 'ExpressionInParentheses', got: next.rule });
    }
    const inner = next.inner[0];
    if (inner === undefined) throw new ParseError('EmptyExpression', {});
    // Returning here abandons whatever follows the parenthesised group, exactly as
    // the Rust does.
    return Predicate.Not(parseExpr(inner));
  }

  let result = parseAtomicExpr(first);

  try {
    // Handle postfix and infix operators
    for (;;) {
      const op = pairs.next();
      if (op === undefined) break;

      if (op.rule === 'IsNullPostfix') {
        // Check if this is "IS NULL" or "IS NOT NULL" by examining the text
        const isNot = op.text.toUpperCase().includes('NOT');

        const isNull = Expr.Predicate(Predicate.IsNull(result));
        result = isNot ? Expr.Predicate(Predicate.Not(exprToPredicate(isNull))) : isNull;
        continue;
      }

      // infix operators DO have a right operand
      const right = pairs.next();
      if (right === undefined) throw new ParseError('MissingOperand', { _0: 'right' });

      switch (op.rule) {
        case 'Eq':
        case 'GtEq':
        case 'Gt':
        case 'LtEq':
        case 'Lt':
        case 'NotEq':
        case 'In':
          result = createComparison(result, op.rule, right);
          break;
        case 'And':
        case 'Or':
          result = createLogicalOp(op.rule, result, right, pairs);
          break;
        default:
          // Between and the four arithmetic operators are in the grammar but have no
          // arm above, so every one of them is unreachable from query text.
          throw new ParseError('UnexpectedRule', { expected: 'comparison operator, And, or Or', got: op.rule });
      }
    }
  } catch (e) {
    result.drop();
    throw e;
  }

  return exprToPredicate(result);
}

// Rust: fn create_comparison
/** Create a comparison predicate from a left expression and a right pair */
function createComparison(left: Expr, op: Rule, right: Pair): Expr {
  const rightExpr = parseAtomicExpr(right);
  const operator = comparisonOperatorFor(op);
  return Expr.Predicate(Predicate.Comparison(left, operator, rightExpr));
}

/** The `ComparisonOperator` a comparison rule names. There is deliberately no arm for
 *  Between: the grammar has the rule and the AST has the variant, but nothing joins
 *  them, so `a BETWEEN 1 AND 10` is refused. */
function comparisonOperatorFor(op: Rule): ComparisonOperator {
  switch (op) {
    case 'Eq': return ComparisonOperator.Equal();
    case 'GtEq': return ComparisonOperator.GreaterThanOrEqual();
    case 'Gt': return ComparisonOperator.GreaterThan();
    case 'LtEq': return ComparisonOperator.LessThanOrEqual();
    case 'Lt': return ComparisonOperator.LessThan();
    case 'NotEq': return ComparisonOperator.NotEqual();
    case 'In': return ComparisonOperator.In();
    default:
      throw new ParseError('UnexpectedRule', { expected: 'comparison operator', got: op });
  }
}

// Rust: fn create_logical_op
/** Create a logical operation (AND/OR) from a left expression and a right pair */
function createLogicalOp(op: Rule, left: Expr, right: Pair, rest: Pairs): Expr {
  const leftPred = exprToPredicate(left);

  let rightExpr: Expr | null = null;
  let rightPred: Predicate;
  try {
    // Parse the right side, which might be part of a comparison
    rightExpr = parseAtomicExpr(right);
    // Pulling from `rest` here is what makes the fold left-associative: the operator
    // and operand this consumes belong to the right operand, and parse_expr's loop
    // resumes after them with the whole And/Or as its new left.
    const nextOp = rest.next();
    if (nextOp === undefined) {
      rightPred = exprToPredicate(rightExpr);
    } else {
      switch (nextOp.rule) {
        case 'Eq':
        case 'GtEq':
        case 'Gt':
        case 'LtEq':
        case 'Lt':
        case 'NotEq':
        case 'In': {
          const nextRight = rest.next();
          if (nextRight === undefined) throw new ParseError('MissingOperand', { _0: 'comparison right' });
          const nextRightExpr = parseAtomicExpr(nextRight);
          rightPred = Predicate.Comparison(rightExpr, comparisonOperatorFor(nextOp.rule), nextRightExpr);
          break;
        }
        default:
          throw new ParseError('UnexpectedRule', { expected: 'comparison operator', got: nextOp.rule });
      }
    }
  } catch (e) {
    // Rust drops both half-built operands here.
    leftPred.drop();
    rightExpr?.drop();
    throw e;
  }

  return Expr.Predicate(op === 'And' ? Predicate.And(leftPred, rightPred) : Predicate.Or(leftPred, rightPred));
}

// Rust: fn parse_atomic_expr
/** Parse an atomic expression, which can be a path, literal, or parenthesized expression */
function parseAtomicExpr(pair: Pair): Expr {
  switch (pair.rule) {
    case 'PathExpr':
      return parsePathExpr(pair);
    case 'SingleQuotedString':
      return parseStringLiteral(pair);
    case 'True':
      return Expr.Literal(Literal.Bool(true));
    case 'False':
      return Expr.Literal(Literal.Bool(false));
    case 'Unsigned':
      return parseNumber(pair);
    case 'QuestionParameter':
      return Expr.Placeholder();
    case 'ExpressionInParentheses': {
      const inner = pair.inner[0];
      if (inner === undefined) throw new ParseError('EmptyExpression', {});
      return Expr.Predicate(parseExpr(inner));
    }
    case 'Row': {
      const exprs: Expr[] = [];
      try {
        for (const exprPair of pair.inner) {
          if (exprPair.rule === 'Expr') {
            // Only the first pair of each element is read, so anything the element
            // says after its first atom is discarded.
            const head = exprPair.inner[0];
            if (head === undefined) throw new ParseError('EmptyExpression', {});
            exprs.push(parseAtomicExpr(head));
          } else {
            exprs.push(parseAtomicExpr(exprPair));
          }
        }
      } catch (e) {
        for (const expr of exprs) expr.drop();
        throw e;
      }
      return Expr.ExprList(exprs);
    }
    default:
      // Integer, Decimal, Double and Null all reach the grammar and stop here, which
      // is why negative, fractional and NULL literals cannot be written at all — and
      // why Literal::F64 is unreachable from query text.
      throw new ParseError('UnexpectedRule', { expected: 'atomic expression', got: pair.rule });
  }
}

// Rust: fn parse_path_expr
/** Parse a path expression (dot-separated identifiers like `name` or `licensing.territory`) */
function parsePathExpr(pair: Pair): Expr {
  if (pair.rule !== 'PathExpr') {
    throw new ParseError('UnexpectedRule', { expected: 'PathExpr', got: pair.rule });
  }

  // A double-quoted identifier arrives with its quotes still on, and they are stored
  // in the step verbatim.
  const steps = pair.inner.filter((p) => p.rule === 'Identifier').map((p) => p.text.trim());

  if (steps.length === 0) {
    throw new ParseError('InvalidPredicate', { _0: 'Empty path expression' });
  }

  return Expr.Path(new PathExpr(steps));
}

// Rust: fn parse_string_literal
/** Parse a string literal, removing the surrounding quotes */
function parseStringLiteral(pair: Pair): Expr {
  if (pair.rule !== 'SingleQuotedString') {
    throw new ParseError('UnexpectedRule', { expected: 'SingleQuotedString', got: pair.rule });
  }

  const s = pair.text;
  if (!s.startsWith("'") || !s.endsWith("'")) {
    throw new ParseError('InvalidPredicate', { _0: 'String literal must be quoted' });
  }

  return Expr.Literal(Literal.String(s.slice(1, -1)));
}

// Rust: fn parse_number
/** Parse a number literal */
function parseNumber(pair: Pair): Expr {
  if (pair.rule !== 'Unsigned') {
    throw new ParseError('UnexpectedRule', { expected: 'Unsigned', got: pair.rule });
  }

  // Rust: `pair.as_str().trim().parse::<i64>()`. The rule matched digits, so the only
  // way this fails is overflow, and the text is ParseIntError's own.
  const num = BigInt(pair.text.trim());
  if (num > I64_MAX) {
    throw new ParseError('InvalidPredicate', { _0: 'Failed to parse number: number too large to fit in target type' });
  }

  // Strict inequalities, so i32::MAX itself falls through to I64 while i32::MAX - 1
  // does not. The port must not "fix" this to <=.
  if (num < I32_MAX && num > I32_MIN) {
    return Expr.Literal(Literal.I32(Number(num)));
  }

  return Expr.Literal(Literal.I64(num));
}

// Rust: fn parse_limit_clause
/** Parse a LIMIT clause */
function parseLimitClause(pair: Pair): bigint {
  if (pair.rule !== 'LimitClause') {
    throw new ParseError('UnexpectedRule', { expected: 'LimitClause', got: pair.rule });
  }

  // Since LimitClause is compound atomic ($), we can access the inner Unsigned token directly
  const unsigned = pair.inner.find((p) => p.rule === 'Unsigned');
  if (unsigned === undefined) {
    throw new ParseError('InvalidPredicate', { _0: 'Missing limit value' });
  }

  // Rust: `parse::<u64>()` — the field is a u64, which in TS is a bigint.
  const limit = BigInt(unsigned.text.trim());
  if (limit > U64_MAX) {
    throw new ParseError('InvalidPredicate', { _0: 'Failed to parse limit: number too large to fit in target type' });
  }

  return limit;
}

// Rust: fn parse_order_by_clause
function parseOrderByClause(pair: Pair): OrderByItem[] {
  if (pair.rule !== 'OrderByClause') {
    throw new ParseError('UnexpectedRule', { expected: 'OrderByClause', got: pair.rule });
  }

  const orderByItems: OrderByItem[] = [];

  // Parse each OrderByItem in the clause
  try {
    for (const inner of pair.inner) {
      if (inner.rule === 'OrderByItem') {
        orderByItems.push(parseOrderByItem(inner));
      }
    }
  } catch (e) {
    for (const item of orderByItems) item.drop();
    throw e;
  }

  if (orderByItems.length === 0) {
    throw new ParseError('InvalidPredicate', { _0: 'Missing ORDER BY items' });
  }

  return orderByItems;
}

// Rust: fn parse_order_by_item
function parseOrderByItem(pair: Pair): OrderByItem {
  if (pair.rule !== 'OrderByItem') {
    throw new ParseError('UnexpectedRule', { expected: 'OrderByItem', got: pair.rule });
  }

  const identifier = pair.inner.find((p) => p.rule === 'Identifier');
  if (identifier === undefined) {
    throw new ParseError('InvalidPredicate', { _0: 'Missing column name in ORDER BY item' });
  }

  const identifierStr = identifier.text.trim();

  // Only simple identifiers are supported in ORDER BY (no dotted identifiers).
  // Unreachable from query text: OrderByItem takes an Identifier, so the grammar
  // stops at the dot and the selection fails at EOI before this check runs.
  if (identifierStr.includes('.')) {
    throw new ParseError('InvalidPredicate', { _0: 'Dotted identifiers are not supported in ORDER BY clauses' });
  }

  const path = PathExpr.simple(identifierStr);

  const directionPair = pair.inner.find((p) => p.rule === 'OrderDirection');
  let direction: OrderDirection;
  if (directionPair === undefined) {
    direction = OrderDirection.Asc(); // Default
  } else {
    switch (directionPair.text.trim().toUpperCase()) {
      case 'ASC':
        direction = OrderDirection.Asc();
        break;
      case 'DESC':
        direction = OrderDirection.Desc();
        break;
      default:
        throw new ParseError('InvalidPredicate', { _0: `Invalid order direction: ${directionPair.text}` });
    }
  }

  return new OrderByItem(path, direction);
}
