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
//
// The AST values are built with the constructors ast.ts emits from Rust's own
// enums — `new Expr('Path', { _0: x })`, a struct variant by its field names — and
// every Rust `fn` that answers `Result` answers a `Result` here, so `?` is an
// `isErr()` test rather than a throw. `try_into()` on an Expr is the emitted
// `Predicate_fromExpr` out of conversion.ts, which is where the engine writes
// `impl TryFrom<ast::Expr> for Predicate`.
//
// Rust drops what a `?` abandons. Each early return below therefore releases the
// values that function still owns, and hands over the ones it has already moved
// into a callee — a leak and a double drop are both fatal under the registry.

import { Result, dropOwned } from '@ankurah/base';
import { type Pair, type Rule, parseSelectionRule } from './grammar.ts';
import { ComparisonOperator, Expr, Literal, OrderByItem, OrderDirection, PathExpr, Predicate, Selection } from './ast.ts';
import { Predicate_fromExpr } from './conversion.ts';
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
export function parseSelection(input: string): Result<Selection, ParseError> {
  // TODO: Improve grammar to handle these cases more elegantly
  if (input.trim() === '') {
    return Result.Ok(new Selection(new Predicate('True', {}), null, null));
  }

  const parsed = parseSelectionRule(input);
  if (!parsed.ok) return Result.Err(new ParseError('SyntaxError', { _0: parsed.message }));

  // `Selection = _{ SOI ~ Expr ~ OrderByClause? ~ LimitClause? ~ EOI }`, so each of
  // these is written at most once and no assignment ever drops an earlier value.
  let predicate: Predicate | null = null;
  let orderBy: OrderByItem[] | null = null;
  let limit: bigint | null = null;
  let moved = false;

  try {
    for (const pair of parsed.pairs) {
      switch (pair.rule) {
        case 'Expr': {
          const r = parseExpr(pair);
          if (r.isErr()) return Result.Err(r.unwrapErr());
          predicate = r.unwrap();
          break;
        }
        case 'OrderByClause': {
          const r = parseOrderByClause(pair);
          if (r.isErr()) return Result.Err(r.unwrapErr());
          orderBy = r.unwrap();
          break;
        }
        case 'LimitClause': {
          const r = parseLimitClause(pair);
          if (r.isErr()) return Result.Err(r.unwrapErr());
          limit = r.unwrap();
          break;
        }
        case 'EOI':
          break; // End of input, ignore
        default:
          return Result.Err(new ParseError('UnexpectedRule', { expected: 'Expr, OrderByClause, or LimitClause', got: pair.rule }));
      }
    }

    if (predicate === null) return Result.Err(new ParseError('EmptyExpression', {}));

    moved = true;
    return Result.Ok(new Selection(predicate, orderBy, limit));
  } finally {
    // Rust drops the half-built selection on every `?` above.
    if (!moved) {
      predicate?.drop();
      dropOwned(orderBy);
    }
  }
}

// Rust: fn parse_expr
/** Parse a boolean expression, which can be a comparison, AND, or OR expression */
function parseExpr(pair: Pair): Result<Predicate, ParseError> {
  // Rust: `assert_eq!` — a panic, not a ParseError, so this stays a throw.
  if (pair.rule !== 'Expr') throw new Error('Expected Expr rule');
  const pairs = new Pairs(pair.inner);

  // Parse the first value
  const first = pairs.next();
  if (first === undefined) return Result.Err(new ParseError('MissingOperand', { _0: 'first' }));

  // handle unary operators which have precedence over infix operators
  if (first.rule === 'UnaryNot') {
    const next = pairs.next();
    if (next === undefined) return Result.Err(new ParseError('EmptyExpression', {}));
    if (next.rule !== 'ExpressionInParentheses') {
      // NOT only works over parentheses: `NOT (a = 1)` parses, `NOT a = 1` lands here.
      return Result.Err(new ParseError('UnexpectedRule', { expected: 'ExpressionInParentheses', got: next.rule }));
    }
    const inner = next.inner[0];
    if (inner === undefined) return Result.Err(new ParseError('EmptyExpression', {}));
    const r = parseExpr(inner);
    if (r.isErr()) return Result.Err(r.unwrapErr());
    // Returning here abandons whatever follows the parenthesised group, exactly as
    // the Rust does.
    return Result.Ok(new Predicate('Not', { _0: r.unwrap() }));
  }

  const r0 = parseAtomicExpr(first);
  if (r0.isErr()) return Result.Err(r0.unwrapErr());
  let result = r0.unwrap();

  // Handle postfix and infix operators
  for (;;) {
    const op = pairs.next();
    if (op === undefined) break;

    if (op.rule === 'IsNullPostfix') {
      // Check if this is "IS NULL" or "IS NOT NULL" by examining the text
      const isNot = op.text.toUpperCase().includes('NOT');

      const isNull = new Expr('Predicate', { _0: new Predicate('IsNull', { _0: result }) });
      if (isNot) {
        // `Expr::Predicate(..)` always converts, so this never takes the Err path.
        const r = Predicate_fromExpr(isNull);
        if (r.isErr()) return Result.Err(r.unwrapErr());
        result = new Expr('Predicate', { _0: new Predicate('Not', { _0: r.unwrap() }) });
      } else {
        result = isNull;
      }
      continue;
    }

    // infix operators DO have a right operand
    const right = pairs.next();
    if (right === undefined) {
      result.drop();
      return Result.Err(new ParseError('MissingOperand', { _0: 'right' }));
    }

    switch (op.rule) {
      case 'Eq':
      case 'GtEq':
      case 'Gt':
      case 'LtEq':
      case 'Lt':
      case 'NotEq':
      case 'In': {
        // `result` belongs to the callee from here, on both of its answers.
        const r = createComparison(result, op.rule, right);
        if (r.isErr()) return Result.Err(r.unwrapErr());
        result = r.unwrap();
        break;
      }
      case 'And':
      case 'Or': {
        const r = createLogicalOp(op.rule, result, right, pairs);
        if (r.isErr()) return Result.Err(r.unwrapErr());
        result = r.unwrap();
        break;
      }
      default:
        // Between and the four arithmetic operators are in the grammar but have no
        // arm above, so every one of them is unreachable from query text.
        result.drop();
        return Result.Err(new ParseError('UnexpectedRule', { expected: 'comparison operator, And, or Or', got: op.rule }));
    }
  }

  return Predicate_fromExpr(result);
}

// Rust: fn create_comparison
/** Create a comparison predicate from a left expression and a right pair */
function createComparison(left: Expr, op: Rule, right: Pair): Result<Expr, ParseError> {
  const r = parseAtomicExpr(right);
  if (r.isErr()) {
    left.drop();
    return Result.Err(r.unwrapErr());
  }
  const rightExpr = r.unwrap();
  const operator = comparisonOperatorFor(op);
  if (operator === null) {
    left.drop();
    rightExpr.drop();
    return Result.Err(new ParseError('UnexpectedRule', { expected: 'comparison operator', got: op }));
  }
  const comparison = new Predicate('Comparison', { left, operator: new ComparisonOperator(operator, {}), right: rightExpr });
  return Result.Ok(new Expr('Predicate', { _0: comparison }));
}

/** Which `ComparisonOperator` variant a comparison rule names, or null for a rule
 *  that names none. There is deliberately no arm for Between: the grammar has the
 *  rule and the AST has the variant, but nothing joins them, so `a BETWEEN 1 AND
 *  10` is refused.
 *
 *  It answers the VARIANT NAME rather than a value, because both callers ask twice
 *  — once to decide whether the rule is a comparison at all, once to build the
 *  operator — and an operator built for the first question would be a value with
 *  no owner. Rust writes this match twice and answers the two null cases
 *  differently: an `UnexpectedRule` in create_comparison, an `unimplemented!()`
 *  panic in create_logical_op, so the decision stays with each caller. */
function comparisonOperatorFor(op: Rule): ComparisonOperator['type'] | null {
  switch (op) {
    case 'Eq': return 'Equal';
    case 'GtEq': return 'GreaterThanOrEqual';
    case 'Gt': return 'GreaterThan';
    case 'LtEq': return 'LessThanOrEqual';
    case 'Lt': return 'LessThan';
    case 'NotEq': return 'NotEqual';
    case 'In': return 'In';
    default: return null;
  }
}

// Rust: fn create_logical_op
/** Create a logical operation (AND/OR) from a left expression and a right pair */
function createLogicalOp(op: Rule, left: Expr, right: Pair, rest: Pairs): Result<Expr, ParseError> {
  const rl = Predicate_fromExpr(left);
  if (rl.isErr()) return Result.Err(rl.unwrapErr());
  const leftPred = rl.unwrap();

  // Parse the right side, which might be part of a comparison
  const rr = parseAtomicExpr(right);
  if (rr.isErr()) {
    leftPred.drop();
    return Result.Err(rr.unwrapErr());
  }
  const rightExpr = rr.unwrap();

  let rightPred: Predicate;
  // Pulling from `rest` here is what makes the fold left-associative: the operator
  // and operand this consumes belong to the right operand, and parse_expr's loop
  // resumes after them with the whole And/Or as its new left.
  const nextOp = rest.next();
  if (nextOp === undefined) {
    const rp = Predicate_fromExpr(rightExpr);
    if (rp.isErr()) {
      leftPred.drop();
      return Result.Err(rp.unwrapErr());
    }
    rightPred = rp.unwrap();
  } else if (comparisonOperatorFor(nextOp.rule) === null) {
    leftPred.drop();
    rightExpr.drop();
    return Result.Err(new ParseError('UnexpectedRule', { expected: 'comparison operator', got: nextOp.rule }));
  } else {
    const operator = comparisonOperatorFor(nextOp.rule)!;
    const nextRight = rest.next();
    if (nextRight === undefined) {
      leftPred.drop();
      rightExpr.drop();
      return Result.Err(new ParseError('MissingOperand', { _0: 'comparison right' }));
    }
    const rn = parseAtomicExpr(nextRight);
    if (rn.isErr()) {
      leftPred.drop();
      rightExpr.drop();
      return Result.Err(rn.unwrapErr());
    }
    // Rust's inner match ends in `unimplemented!()`; the arm above already proved
    // this rule names an operator, so that panic is unreachable and so is the `!`.
    rightPred = new Predicate('Comparison', { left: rightExpr, operator: new ComparisonOperator(operator, {}), right: rn.unwrap() });
  }

  // Rust: `match op { And => .., Or => .., _ => unimplemented!() }`. parse_expr only
  // ever calls this with And or Or.
  if (op !== 'And' && op !== 'Or') throw new Error(`rule not implemented: ${op}`);
  return Result.Ok(new Expr('Predicate', { _0: new Predicate(op, { _0: leftPred, _1: rightPred }) }));
}

// Rust: fn parse_atomic_expr
/** Parse an atomic expression, which can be a path, literal, or parenthesized expression */
function parseAtomicExpr(pair: Pair): Result<Expr, ParseError> {
  switch (pair.rule) {
    case 'PathExpr':
      return parsePathExpr(pair);
    case 'SingleQuotedString':
      return parseStringLiteral(pair);
    case 'True':
      return Result.Ok(new Expr('Literal', { _0: new Literal('Bool', { _0: true }) }));
    case 'False':
      return Result.Ok(new Expr('Literal', { _0: new Literal('Bool', { _0: false }) }));
    case 'Unsigned':
      return parseNumber(pair);
    case 'QuestionParameter':
      return Result.Ok(new Expr('Placeholder', {}));
    case 'ExpressionInParentheses': {
      const inner = pair.inner[0];
      if (inner === undefined) return Result.Err(new ParseError('EmptyExpression', {}));
      const r = parseExpr(inner);
      if (r.isErr()) return Result.Err(r.unwrapErr());
      return Result.Ok(new Expr('Predicate', { _0: r.unwrap() }));
    }
    case 'Row': {
      const exprs: Expr[] = [];
      for (const exprPair of pair.inner) {
        let r: Result<Expr, ParseError>;
        if (exprPair.rule === 'Expr') {
          // Only the first pair of each element is read, so anything the element
          // says after its first atom is discarded.
          const head = exprPair.inner[0];
          if (head === undefined) {
            dropOwned(exprs);
            return Result.Err(new ParseError('EmptyExpression', {}));
          }
          r = parseAtomicExpr(head);
        } else {
          r = parseAtomicExpr(exprPair);
        }
        if (r.isErr()) {
          dropOwned(exprs);
          return Result.Err(r.unwrapErr());
        }
        exprs.push(r.unwrap());
      }
      return Result.Ok(new Expr('ExprList', { _0: exprs }));
    }
    default:
      // Integer, Decimal, Double and Null all reach the grammar and stop here, which
      // is why negative, fractional and NULL literals cannot be written at all — and
      // why Literal::F64 is unreachable from query text.
      return Result.Err(new ParseError('UnexpectedRule', { expected: 'atomic expression', got: pair.rule }));
  }
}

// Rust: fn parse_path_expr
/** Parse a path expression (dot-separated identifiers like `name` or `licensing.territory`) */
function parsePathExpr(pair: Pair): Result<Expr, ParseError> {
  if (pair.rule !== 'PathExpr') {
    return Result.Err(new ParseError('UnexpectedRule', { expected: 'PathExpr', got: pair.rule }));
  }

  // A double-quoted identifier arrives with its quotes still on, and they are stored
  // in the step verbatim.
  const steps = pair.inner.filter((p) => p.rule === 'Identifier').map((p) => p.text.trim());

  if (steps.length === 0) {
    return Result.Err(new ParseError('InvalidPredicate', { _0: 'Empty path expression' }));
  }

  return Result.Ok(new Expr('Path', { _0: new PathExpr(steps) }));
}

// Rust: fn parse_string_literal
/** Parse a string literal, removing the surrounding quotes */
function parseStringLiteral(pair: Pair): Result<Expr, ParseError> {
  if (pair.rule !== 'SingleQuotedString') {
    return Result.Err(new ParseError('UnexpectedRule', { expected: 'SingleQuotedString', got: pair.rule }));
  }

  const s = pair.text;
  if (!s.startsWith("'") || !s.endsWith("'")) {
    return Result.Err(new ParseError('InvalidPredicate', { _0: 'String literal must be quoted' }));
  }

  return Result.Ok(new Expr('Literal', { _0: new Literal('String', { _0: s.slice(1, -1) }) }));
}

// Rust: fn parse_number
/** Parse a number literal */
function parseNumber(pair: Pair): Result<Expr, ParseError> {
  if (pair.rule !== 'Unsigned') {
    return Result.Err(new ParseError('UnexpectedRule', { expected: 'Unsigned', got: pair.rule }));
  }

  // Rust: `pair.as_str().trim().parse::<i64>()`. The rule matched digits, so the only
  // way this fails is overflow, and the text is ParseIntError's own.
  const num = BigInt(pair.text.trim());
  if (num > I64_MAX) {
    return Result.Err(new ParseError('InvalidPredicate', { _0: 'Failed to parse number: number too large to fit in target type' }));
  }

  // Strict inequalities, so i32::MAX itself falls through to I64 while i32::MAX - 1
  // does not. The port must not "fix" this to <=.
  if (num < I32_MAX && num > I32_MIN) {
    return Result.Ok(new Expr('Literal', { _0: new Literal('I32', { _0: Number(num) }) }));
  }

  return Result.Ok(new Expr('Literal', { _0: new Literal('I64', { _0: num }) }));
}

// Rust: fn parse_limit_clause
/** Parse a LIMIT clause */
function parseLimitClause(pair: Pair): Result<bigint, ParseError> {
  if (pair.rule !== 'LimitClause') {
    return Result.Err(new ParseError('UnexpectedRule', { expected: 'LimitClause', got: pair.rule }));
  }

  // Since LimitClause is compound atomic ($), we can access the inner Unsigned token directly
  const unsigned = pair.inner.find((p) => p.rule === 'Unsigned');
  if (unsigned === undefined) {
    return Result.Err(new ParseError('InvalidPredicate', { _0: 'Missing limit value' }));
  }

  // Rust: `parse::<u64>()` — the field is a u64, which in TS is a bigint.
  const limit = BigInt(unsigned.text.trim());
  if (limit > U64_MAX) {
    return Result.Err(new ParseError('InvalidPredicate', { _0: 'Failed to parse limit: number too large to fit in target type' }));
  }

  return Result.Ok(limit);
}

// Rust: fn parse_order_by_clause
function parseOrderByClause(pair: Pair): Result<OrderByItem[], ParseError> {
  if (pair.rule !== 'OrderByClause') {
    return Result.Err(new ParseError('UnexpectedRule', { expected: 'OrderByClause', got: pair.rule }));
  }

  const orderByItems: OrderByItem[] = [];

  // Parse each OrderByItem in the clause
  for (const inner of pair.inner) {
    if (inner.rule !== 'OrderByItem') continue;
    const r = parseOrderByItem(inner);
    if (r.isErr()) {
      dropOwned(orderByItems);
      return Result.Err(r.unwrapErr());
    }
    orderByItems.push(r.unwrap());
  }

  if (orderByItems.length === 0) {
    return Result.Err(new ParseError('InvalidPredicate', { _0: 'Missing ORDER BY items' }));
  }

  return Result.Ok(orderByItems);
}

// Rust: fn parse_order_by_item
function parseOrderByItem(pair: Pair): Result<OrderByItem, ParseError> {
  if (pair.rule !== 'OrderByItem') {
    return Result.Err(new ParseError('UnexpectedRule', { expected: 'OrderByItem', got: pair.rule }));
  }

  const identifier = pair.inner.find((p) => p.rule === 'Identifier');
  if (identifier === undefined) {
    return Result.Err(new ParseError('InvalidPredicate', { _0: 'Missing column name in ORDER BY item' }));
  }

  const identifierStr = identifier.text.trim();

  // Only simple identifiers are supported in ORDER BY (no dotted identifiers).
  // Unreachable from query text: OrderByItem takes an Identifier, so the grammar
  // stops at the dot and the selection fails at EOI before this check runs.
  if (identifierStr.includes('.')) {
    return Result.Err(new ParseError('InvalidPredicate', { _0: 'Dotted identifiers are not supported in ORDER BY clauses' }));
  }

  const path = PathExpr.simple(identifierStr);

  const directionPair = pair.inner.find((p) => p.rule === 'OrderDirection');
  let direction: OrderDirection;
  if (directionPair === undefined) {
    direction = new OrderDirection('Asc', {}); // Default
  } else {
    switch (directionPair.text.trim().toUpperCase()) {
      case 'ASC':
        direction = new OrderDirection('Asc', {});
        break;
      case 'DESC':
        direction = new OrderDirection('Desc', {});
        break;
      default:
        path.drop();
        return Result.Err(new ParseError('InvalidPredicate', { _0: `Invalid order direction: ${directionPair.text}` }));
    }
  }

  return Result.Ok(new OrderByItem(path, direction));
}
