// MIRRORS: ankurah/ankql/src/error.rs

/** Custom error type for parsing errors */
export class ParseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ParseError';
  }
}

export class SyntaxError extends ParseError {
  constructor(message: string) {
    super(`Syntax error: ${message}`);
    this.name = 'SyntaxError';
  }
}

export class EmptyExpressionError extends ParseError {
  constructor() {
    super('Empty expression');
    this.name = 'EmptyExpressionError';
  }
}

export class UnexpectedTokenError extends ParseError {
  constructor(expected: string, got: string) {
    super(`Expected ${expected}, got ${got}`);
    this.name = 'UnexpectedTokenError';
  }
}

export class InvalidPredicateError extends ParseError {
  constructor(message: string) {
    super(`Invalid predicate: ${message}`);
    this.name = 'InvalidPredicateError';
  }
}

export class MissingOperandError extends ParseError {
  constructor(operand: string) {
    super(`Missing ${operand} operand`);
    this.name = 'MissingOperandError';
  }
}

/** Custom error type for SQL generation errors */
export class SqlGenerationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'SqlGenerationError';
  }
}

export class PlaceholderCountMismatchError extends SqlGenerationError {
  expected: number;
  found: number;
  constructor(expected: number, found: number) {
    super(`Placeholder count mismatch: expected ${expected}, found ${found}`);
    this.name = 'PlaceholderCountMismatchError';
    this.expected = expected;
    this.found = found;
  }
}

export class InvalidExpressionError extends SqlGenerationError {
  constructor(message: string) {
    super(`Invalid expression: ${message}`);
    this.name = 'InvalidExpressionError';
  }
}

export class UnsupportedOperatorError extends SqlGenerationError {
  constructor(message: string) {
    super(`Unsupported operator: ${message}`);
    this.name = 'UnsupportedOperatorError';
  }
}
