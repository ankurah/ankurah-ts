// MIRRORS: ankurah/ankql/src/parser.rs
// Hand-written recursive descent parser (E6: no Pest equivalent in TS).

import {
  type Token,
  type TokenType,
  isWhitespace,
  isDigit,
  isIdentStart,
  isIdentCont,
} from './grammar.ts';
import {
  type Expr,
  type Predicate,
  type ComparisonOperator,
  type InfixOperator,
  type OrderByItem,
  type OrderDirection,
  PathExpr,
  Selection,
  exprToPredicate,
} from './ast.ts';
import {
  SyntaxError,
  InvalidPredicateError,
} from './error.ts';

// ── Tokenizer / Lexer ────────────────────────────────────────────────

class Lexer {
  private input: string;
  private pos: number;
  private tokens: Token[];

  constructor(input: string) {
    this.input = input;
    this.pos = 0;
    this.tokens = [];
    this.tokenize();
  }

  private peek(): string {
    return this.pos < this.input.length ? this.input[this.pos] : '';
  }

  private advance(): string {
    return this.input[this.pos++];
  }

  private remaining(): string {
    return this.input.slice(this.pos);
  }

  private skipWhitespace(): void {
    while (this.pos < this.input.length && isWhitespace(this.input[this.pos])) {
      this.pos++;
    }
  }

  private peekAt(offset: number): string {
    const idx = this.pos + offset;
    return idx < this.input.length ? this.input[idx] : '';
  }

  /** Check if "ORDER BY" (case-insensitive) starts at position afterOrderPos */
  private isOrderByAhead(afterOrderPos: number): boolean {
    let p = afterOrderPos;
    while (p < this.input.length && isWhitespace(this.input[p])) p++;
    const rest = this.input.slice(p).toLowerCase();
    if (rest.startsWith('by')) {
      const afterBy = p + 2;
      if (afterBy >= this.input.length || !isIdentCont(this.input[afterBy])) {
        return true;
      }
    }
    return false;
  }

  /** Check if "LIMIT <digit>" starts at position afterLimitPos */
  private isLimitAhead(afterLimitPos: number): boolean {
    let p = afterLimitPos;
    while (p < this.input.length && isWhitespace(this.input[p])) p++;
    if (p < this.input.length && isDigit(this.input[p])) {
      return true;
    }
    return false;
  }

  private tokenize(): void {
    while (this.pos < this.input.length) {
      this.skipWhitespace();
      if (this.pos >= this.input.length) break;

      const startPos = this.pos;
      const ch = this.peek();

      // Single-character tokens
      if (ch === '(') { this.advance(); this.tokens.push({ type: 'LParen', value: '(', pos: startPos }); continue; }
      if (ch === ')') { this.advance(); this.tokens.push({ type: 'RParen', value: ')', pos: startPos }); continue; }
      if (ch === ',') { this.advance(); this.tokens.push({ type: 'Comma', value: ',', pos: startPos }); continue; }
      if (ch === '?') { this.advance(); this.tokens.push({ type: 'Question', value: '?', pos: startPos }); continue; }
      if (ch === '+') { this.advance(); this.tokens.push({ type: 'Add', value: '+', pos: startPos }); continue; }
      if (ch === '*') { this.advance(); this.tokens.push({ type: 'Multiply', value: '*', pos: startPos }); continue; }
      if (ch === '/') { this.advance(); this.tokens.push({ type: 'Divide', value: '/', pos: startPos }); continue; }

      // Dot — must distinguish from decimal numbers. A dot is a path separator
      // ONLY if the previous token is an Identifier and the next char is an ident-start.
      // Decimal dots are handled by the number parser.
      if (ch === '.') {
        this.advance();
        this.tokens.push({ type: 'Dot', value: '.', pos: startPos });
        continue;
      }

      // Multi-char operators
      if (ch === '=') {
        this.advance();
        this.tokens.push({ type: 'Eq', value: '=', pos: startPos });
        continue;
      }
      if (ch === '!' && this.peekAt(1) === '=') {
        this.advance(); this.advance();
        this.tokens.push({ type: 'NotEq', value: '!=', pos: startPos });
        continue;
      }
      if (ch === '<' && this.peekAt(1) === '>') {
        this.advance(); this.advance();
        this.tokens.push({ type: 'NotEq', value: '<>', pos: startPos });
        continue;
      }
      if (ch === '<' && this.peekAt(1) === '=') {
        this.advance(); this.advance();
        this.tokens.push({ type: 'LtEq', value: '<=', pos: startPos });
        continue;
      }
      if (ch === '<') {
        this.advance();
        this.tokens.push({ type: 'Lt', value: '<', pos: startPos });
        continue;
      }
      if (ch === '>' && this.peekAt(1) === '=') {
        this.advance(); this.advance();
        this.tokens.push({ type: 'GtEq', value: '>=', pos: startPos });
        continue;
      }
      if (ch === '>') {
        this.advance();
        this.tokens.push({ type: 'Gt', value: '>', pos: startPos });
        continue;
      }

      // Subtraction operator vs negative number
      if (ch === '-') {
        const prevType = this.tokens.length > 0 ? this.tokens[this.tokens.length - 1].type : null;
        const isAfterOperand =
          prevType === 'Identifier' || prevType === 'Unsigned' || prevType === 'Integer' ||
          prevType === 'Decimal' || prevType === 'Double' || prevType === 'String' ||
          prevType === 'RParen' || prevType === 'True' || prevType === 'False' || prevType === 'Question';
        if (isAfterOperand) {
          this.advance();
          this.tokens.push({ type: 'Subtract', value: '-', pos: startPos });
          continue;
        }
        // Fall through to number parsing for negative numbers
      }

      // Single-quoted string
      if (ch === "'") {
        const str = this.readSingleQuotedString();
        this.tokens.push({ type: 'String', value: str, pos: startPos });
        continue;
      }

      // Double-quoted identifier
      if (ch === '"') {
        const ident = this.readDoubleQuotedIdentifier();
        this.tokens.push({ type: 'Identifier', value: ident, pos: startPos });
        continue;
      }

      // Number
      if (isDigit(ch) || (ch === '-' && this.pos + 1 < this.input.length && isDigit(this.input[this.pos + 1]))) {
        const { tokenType, value } = this.readNumber();
        this.tokens.push({ type: tokenType, value, pos: startPos });
        continue;
      }

      // Keywords and identifiers
      if (isIdentStart(ch)) {
        const word = this.readWord();
        const lower = word.toLowerCase();
        const afterWord = this.pos;

        // Keywords — they are only keywords if NOT followed by ident-cont char
        // (which readWord already guarantees by consuming all ident-cont chars)
        if (lower === 'and') { this.tokens.push({ type: 'And', value: word, pos: startPos }); continue; }
        if (lower === 'or') { this.tokens.push({ type: 'Or', value: word, pos: startPos }); continue; }
        if (lower === 'not') { this.tokens.push({ type: 'Not', value: word, pos: startPos }); continue; }
        if (lower === 'in') { this.tokens.push({ type: 'In', value: word, pos: startPos }); continue; }
        if (lower === 'between') { this.tokens.push({ type: 'Between', value: word, pos: startPos }); continue; }
        if (lower === 'is') { this.tokens.push({ type: 'Is', value: word, pos: startPos }); continue; }
        if (lower === 'null') { this.tokens.push({ type: 'Null', value: word, pos: startPos }); continue; }
        if (lower === 'true') { this.tokens.push({ type: 'True', value: word, pos: startPos }); continue; }
        if (lower === 'false') { this.tokens.push({ type: 'False', value: word, pos: startPos }); continue; }
        if (lower === 'asc') { this.tokens.push({ type: 'Asc', value: word, pos: startPos }); continue; }
        if (lower === 'desc') { this.tokens.push({ type: 'Desc', value: word, pos: startPos }); continue; }

        // ORDER BY (compound keyword)
        if (lower === 'order' && this.isOrderByAhead(afterWord)) {
          let p = afterWord;
          while (p < this.input.length && isWhitespace(this.input[p])) p++;
          p += 2; // skip "by"
          this.pos = p;
          this.tokens.push({ type: 'OrderBy', value: 'ORDER BY', pos: startPos });
          continue;
        }

        // LIMIT keyword — only when followed by whitespace then digit
        if (lower === 'limit' && this.isLimitAhead(afterWord)) {
          this.tokens.push({ type: 'Limit', value: word, pos: startPos });
          continue;
        }

        // Plain identifier
        this.tokens.push({ type: 'Identifier', value: word, pos: startPos });
        continue;
      }

      // Semicolon ends input
      if (ch === ';') {
        break;
      }

      throw new SyntaxError(`Unexpected character '${ch}' at position ${this.pos}`);
    }

    this.tokens.push({ type: 'EOF', value: '', pos: this.pos });
  }

  private readSingleQuotedString(): string {
    this.advance(); // consume opening '
    let result = '';
    while (this.pos < this.input.length) {
      const ch = this.peek();
      if (ch === "'") {
        this.advance();
        // Check for escaped quote ''
        if (this.peek() === "'") {
          result += "'";
          this.advance();
        } else {
          return result;
        }
      } else {
        result += ch;
        this.advance();
      }
    }
    throw new SyntaxError('Unterminated string literal');
  }

  private readDoubleQuotedIdentifier(): string {
    this.advance(); // consume opening "
    let result = '';
    while (this.pos < this.input.length) {
      const ch = this.peek();
      if (ch === '"') {
        this.advance();
        return result;
      }
      result += ch;
      this.advance();
    }
    throw new SyntaxError('Unterminated double-quoted identifier');
  }

  private readWord(): string {
    let result = '';
    while (this.pos < this.input.length && isIdentCont(this.input[this.pos])) {
      result += this.input[this.pos];
      this.pos++;
    }
    return result;
  }

  private readNumber(): { tokenType: TokenType; value: string } {
    let result = '';

    // Optional sign
    if (this.peek() === '-' || this.peek() === '+') {
      result += this.advance();
    }

    // Integer part
    while (this.pos < this.input.length && isDigit(this.input[this.pos])) {
      result += this.advance();
    }

    // Check for decimal point — only if followed by a digit
    if (this.peek() === '.' && this.pos + 1 < this.input.length && isDigit(this.input[this.pos + 1])) {
      result += this.advance(); // consume '.'
      while (this.pos < this.input.length && isDigit(this.input[this.pos])) {
        result += this.advance();
      }

      // Check for exponent
      if (this.peek().toLowerCase() === 'e') {
        result += this.advance();
        if (this.peek() === '+' || this.peek() === '-') {
          result += this.advance();
        }
        while (this.pos < this.input.length && isDigit(this.input[this.pos])) {
          result += this.advance();
        }
        return { tokenType: 'Double', value: result };
      }

      return { tokenType: 'Decimal', value: result };
    }

    // Check for exponent without decimal point
    if (this.peek().toLowerCase() === 'e') {
      result += this.advance();
      if (this.peek() === '+' || this.peek() === '-') {
        result += this.advance();
      }
      while (this.pos < this.input.length && isDigit(this.input[this.pos])) {
        result += this.advance();
      }
      return { tokenType: 'Double', value: result };
    }

    if (result.startsWith('-') || result.startsWith('+')) {
      return { tokenType: 'Integer', value: result };
    }
    return { tokenType: 'Unsigned', value: result };
  }

  getTokens(): Token[] {
    return this.tokens;
  }
}

// ── Parser ───────────────────────────────────────────────────────────

class Parser {
  private tokens: Token[];
  private pos: number;

  constructor(tokens: Token[]) {
    this.tokens = tokens;
    this.pos = 0;
  }

  private peek(): Token {
    return this.tokens[this.pos];
  }

  private advance(): Token {
    const token = this.tokens[this.pos];
    this.pos++;
    return token;
  }

  private expect(type: TokenType): Token {
    const token = this.peek();
    if (token.type !== type) {
      throw new SyntaxError(
        `Expected ${type}, got ${token.type} ('${token.value}') at position ${token.pos}`,
      );
    }
    return this.advance();
  }

  private isAtEnd(): boolean {
    return this.peek().type === 'EOF';
  }

  /** Parse a full selection: predicate [ORDER BY ...] [LIMIT ...] */
  parseSelection(): Selection {
    if (this.isAtEnd()) {
      return new Selection({ type: 'True' });
    }

    const predicate = this.parseOr();

    let orderBy: OrderByItem[] | null = null;
    let limit: number | null = null;

    if (this.peek().type === 'OrderBy') {
      this.advance();
      orderBy = this.parseOrderByItems();
    }

    if (this.peek().type === 'Limit') {
      this.advance();
      const tok = this.expect('Unsigned');
      limit = parseInt(tok.value, 10);
    }

    if (!this.isAtEnd()) {
      throw new SyntaxError(
        `Unexpected token '${this.peek().value}' (${this.peek().type}) at position ${this.peek().pos}`,
      );
    }

    return new Selection(predicate, orderBy, limit);
  }

  // ── Precedence climbing ──

  /** OR — lowest precedence */
  private parseOr(): Predicate {
    let left = this.parseAnd();
    while (this.peek().type === 'Or') {
      this.advance();
      const right = this.parseAnd();
      left = { type: 'Or', left, right };
    }
    return left;
  }

  /** AND */
  private parseAnd(): Predicate {
    let left = this.parseNotOrComparison();
    while (this.peek().type === 'And') {
      this.advance();
      const right = this.parseNotOrComparison();
      left = { type: 'And', left, right };
    }
    return left;
  }

  /** NOT (unary prefix) or comparison */
  private parseNotOrComparison(): Predicate {
    if (this.peek().type === 'Not') {
      this.advance();
      if (this.peek().type !== 'LParen') {
        // NOT without parens is not supported (matches Rust behavior)
        throw new SyntaxError(`Expected '(' after NOT at position ${this.peek().pos}`);
      }
      const inner = this.parseNotOrComparison();
      return { type: 'Not', predicate: inner };
    }
    return this.parseComparison();
  }

  /** Comparison: expr (op expr | IS [NOT] NULL)? */
  private parseComparison(): Predicate {
    const left = this.parseArithExpr();

    // IS [NOT] NULL (postfix)
    if (this.peek().type === 'Is') {
      this.advance();
      const hasNot = this.peek().type === 'Not';
      if (hasNot) this.advance();
      this.expect('Null');
      const isNull: Predicate = { type: 'IsNull', expr: left };
      return hasNot ? { type: 'Not', predicate: isNull } : isNull;
    }

    // Comparison operators
    const opType = this.peek().type;
    if (isComparisonOp(opType)) {
      const op = this.advance();
      const compOp = tokenToComparisonOp(op.type);
      const right = this.parseArithExpr();
      return { type: 'Comparison', left, operator: compOp, right };
    }

    // No operator → convert expression to predicate
    return exprToPredicate(left);
  }

  /** Arithmetic expressions (+, -, *, /) */
  private parseArithExpr(): Expr {
    let left = this.parsePrimaryExpr();

    while (
      this.peek().type === 'Add' ||
      this.peek().type === 'Subtract' ||
      this.peek().type === 'Multiply' ||
      this.peek().type === 'Divide'
    ) {
      const op = this.advance();
      const right = this.parsePrimaryExpr();
      const infixOp = tokenToInfixOp(op.type);
      left = { type: 'InfixExpr', left, operator: infixOp, right };
    }

    return left;
  }

  /** Primary expressions (atoms) */
  private parsePrimaryExpr(): Expr {
    const tok = this.peek();

    switch (tok.type) {
      case 'Unsigned':
      case 'Integer':
        return this.parseNumber();
      case 'Decimal':
      case 'Double':
        return this.parseFloat();
      case 'String':
        return this.parseStringLiteral();
      case 'True':
        this.advance();
        return { type: 'Literal', value: { type: 'Bool', value: true } };
      case 'False':
        this.advance();
        return { type: 'Literal', value: { type: 'Bool', value: false } };
      case 'Question':
        this.advance();
        return { type: 'Placeholder' };
      case 'Identifier':
        return this.parsePathExpr();
      case 'LParen':
        return this.parseParenExpr();
      default:
        throw new SyntaxError(
          `Unexpected token '${tok.value}' (${tok.type}) at position ${tok.pos}`,
        );
    }
  }

  private parseNumber(): Expr {
    const tok = this.advance();
    const numStr = tok.value;

    // Try BigInt for very large numbers, otherwise use regular number
    try {
      const num = Number(numStr);
      if (!isFinite(num)) {
        throw new InvalidPredicateError(`Failed to parse number: ${numStr}`);
      }
      // Check i32 range: -2147483648 to 2147483647
      if (num > -(2 ** 31) && num < 2 ** 31 - 1) {
        return { type: 'Literal', value: { type: 'I32', value: num } };
      }
      // For large numbers, use BigInt
      return { type: 'Literal', value: { type: 'I64', value: BigInt(numStr) } };
    } catch {
      // If Number fails, try BigInt
      return { type: 'Literal', value: { type: 'I64', value: BigInt(numStr) } };
    }
  }

  private parseFloat(): Expr {
    const tok = this.advance();
    const num = parseFloat(tok.value);
    return { type: 'Literal', value: { type: 'F64', value: num } };
  }

  private parseStringLiteral(): Expr {
    const tok = this.advance();
    return { type: 'Literal', value: { type: 'String', value: tok.value } };
  }

  private parsePathExpr(): Expr {
    const steps: string[] = [];
    const first = this.expect('Identifier');
    steps.push(first.value);

    while (this.peek().type === 'Dot') {
      this.advance(); // consume '.'
      // Next must be an identifier (but could also be a keyword used as field name)
      const next = this.peek();
      if (next.type === 'Identifier') {
        steps.push(this.advance().value);
      } else {
        // Allow keywords as path steps after a dot
        // (e.g., user.order, user.limit, etc.)
        if (isKeywordTokenType(next.type)) {
          steps.push(this.advance().value);
        } else {
          throw new SyntaxError(
            `Expected identifier after '.' at position ${next.pos}, got ${next.type}`,
          );
        }
      }
    }

    return { type: 'Path', value: new PathExpr(steps) };
  }

  /**
   * Parse parenthesized expression: either (predicate) or (expr, expr, ...) for IN rows.
   */
  private parseParenExpr(): Expr {
    this.expect('LParen');

    // Parse the content inside parentheses as a full predicate/expression
    // First, try to parse the first atom
    const firstAtom = this.parseArithExpr();

    // Check for comma → Row/ExprList
    if (this.peek().type === 'Comma') {
      const items: Expr[] = [firstAtom];
      while (this.peek().type === 'Comma') {
        this.advance();
        items.push(this.parseArithExpr());
      }
      this.expect('RParen');
      return { type: 'ExprList', values: items };
    }

    // Check for comparison/logical operators → this is a parenthesized predicate
    if (isComparisonOp(this.peek().type) || this.peek().type === 'Is') {
      const pred = this.finishPredicateFromExpr(firstAtom);
      // Now check for AND/OR within the parens
      const fullPred = this.finishLogicalChain(pred);
      this.expect('RParen');
      return { type: 'Predicate', value: fullPred };
    }

    // Check for AND/OR → the first atom should be convertible to predicate
    if (this.peek().type === 'And' || this.peek().type === 'Or') {
      const firstPred = exprToPredicate(firstAtom);
      const fullPred = this.finishLogicalChain(firstPred);
      this.expect('RParen');
      return { type: 'Predicate', value: fullPred };
    }

    // Just a parenthesized expression
    this.expect('RParen');
    return firstAtom;
  }

  /** Given a left expression, parse the comparison operator and right side */
  private finishPredicateFromExpr(left: Expr): Predicate {
    if (this.peek().type === 'Is') {
      this.advance();
      const hasNot = this.peek().type === 'Not';
      if (hasNot) this.advance();
      this.expect('Null');
      const isNull: Predicate = { type: 'IsNull', expr: left };
      return hasNot ? { type: 'Not', predicate: isNull } : isNull;
    }

    if (isComparisonOp(this.peek().type)) {
      const op = this.advance();
      const compOp = tokenToComparisonOp(op.type);
      const right = this.parseArithExpr();
      return { type: 'Comparison', left, operator: compOp, right };
    }

    return exprToPredicate(left);
  }

  /**
   * Given an already-parsed predicate, continue parsing AND/OR chains
   * within parentheses.
   */
  private finishLogicalChain(left: Predicate): Predicate {
    let result = left;

    while (this.peek().type === 'And' || this.peek().type === 'Or') {
      const logicType = this.advance().type;

      // Parse the right side (comparison or atom → predicate)
      const rightAtom = this.parseArithExpr();
      let rightPred: Predicate;

      if (isComparisonOp(this.peek().type) || this.peek().type === 'Is') {
        rightPred = this.finishPredicateFromExpr(rightAtom);
      } else {
        rightPred = exprToPredicate(rightAtom);
      }

      if (logicType === 'And') {
        result = { type: 'And', left: result, right: rightPred };
      } else {
        result = { type: 'Or', left: result, right: rightPred };
      }
    }

    return result;
  }

  private parseOrderByItems(): OrderByItem[] {
    const items: OrderByItem[] = [];
    items.push(this.parseOrderByItem());
    while (this.peek().type === 'Comma') {
      this.advance();
      items.push(this.parseOrderByItem());
    }
    return items;
  }

  private parseOrderByItem(): OrderByItem {
    const tok = this.expect('Identifier');

    if (this.peek().type === 'Dot') {
      throw new InvalidPredicateError('Dotted identifiers are not supported in ORDER BY clauses');
    }

    const path = PathExpr.simple(tok.value);

    let direction: OrderDirection = 'Asc';
    if (this.peek().type === 'Asc') {
      this.advance();
      direction = 'Asc';
    } else if (this.peek().type === 'Desc') {
      this.advance();
      direction = 'Desc';
    }

    return { path, direction };
  }
}

// ── Helper functions ─────────────────────────────────────────────────

function isComparisonOp(type: TokenType): boolean {
  return (
    type === 'Eq' || type === 'NotEq' ||
    type === 'Gt' || type === 'GtEq' ||
    type === 'Lt' || type === 'LtEq' ||
    type === 'In'
  );
}

function isKeywordTokenType(type: TokenType): boolean {
  return (
    type === 'And' || type === 'Or' || type === 'Not' ||
    type === 'In' || type === 'Between' || type === 'Is' ||
    type === 'Null' || type === 'True' || type === 'False' ||
    type === 'Asc' || type === 'Desc' || type === 'Limit'
  );
}

function tokenToComparisonOp(type: TokenType): ComparisonOperator {
  switch (type) {
    case 'Eq': return 'Equal';
    case 'NotEq': return 'NotEqual';
    case 'Gt': return 'GreaterThan';
    case 'GtEq': return 'GreaterThanOrEqual';
    case 'Lt': return 'LessThan';
    case 'LtEq': return 'LessThanOrEqual';
    case 'In': return 'In';
    default:
      throw new SyntaxError(`'${type}' is not a comparison operator`);
  }
}

function tokenToInfixOp(type: TokenType): InfixOperator {
  switch (type) {
    case 'Add': return 'Add';
    case 'Subtract': return 'Subtract';
    case 'Multiply': return 'Multiply';
    case 'Divide': return 'Divide';
    default:
      throw new SyntaxError(`'${type}' is not an infix operator`);
  }
}

// ── Public API ───────────────────────────────────────────────────────

/**
 * Parse a selection expression into a Selection AST.
 * The selection includes a predicate and optional ORDER BY and LIMIT clauses.
 */
export function parseSelection(input: string): Selection {
  if (input.trim() === '') {
    return new Selection({ type: 'True' });
  }

  const lexer = new Lexer(input);
  const tokens = lexer.getTokens();
  const parser = new Parser(tokens);
  return parser.parseSelection();
}
