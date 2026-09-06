// MIRRORS: ankurah/ankql/src/error.rs
import { Enum, debugString } from '@ankurah/base';

export type ParseErrorV = {
  SyntaxError: { _0: string };
  EmptyExpression: {};
  UnexpectedRule: { expected: string; got: string };
  InvalidPredicate: { _0: string };
  MissingOperand: { _0: string };
};

export class ParseError extends Enum<ParseErrorV> {

  debug(): string {
    return this.match({
      SyntaxError: (v) => `SyntaxError(${debugString(v._0)})`,
      EmptyExpression: () => 'EmptyExpression',
      UnexpectedRule: (v) => `UnexpectedRule { expected: ${debugString(v.expected)}, got: ${v.got} }`,
      InvalidPredicate: (v) => `InvalidPredicate(${debugString(v._0)})`,
      MissingOperand: (v) => `MissingOperand(${debugString(v._0)})`,
    });
  }

  override toString(): string {
    return this.match({
      SyntaxError: (v) => `Syntax error: ${v._0}`,
      EmptyExpression: () => 'Empty expression',
      UnexpectedRule: (v) => `Expected ${v.expected}, got ${v.got}`,
      InvalidPredicate: (v) => `Invalid predicate: ${v._0}`,
      MissingOperand: (v) => `Missing ${v._0} operand`,
    });
  }
}

export type SqlGenerationErrorV = {
  PlaceholderCountMismatch: { expected: number; found: number };
  InvalidExpression: { _0: string };
  UnsupportedOperator: { _0: string };
};

export class SqlGenerationError extends Enum<SqlGenerationErrorV> {

  debug(): string {
    return this.match({
      PlaceholderCountMismatch: (v) => `PlaceholderCountMismatch { expected: ${String(v.expected)}, found: ${String(v.found)} }`,
      InvalidExpression: (v) => `InvalidExpression(${debugString(v._0)})`,
      UnsupportedOperator: (v) => `UnsupportedOperator(${debugString(v._0)})`,
    });
  }

  override toString(): string {
    return this.match({
      PlaceholderCountMismatch: (v) => `Placeholder count mismatch: expected ${v.expected}, found ${v.found}`,
      InvalidExpression: (v) => `Invalid expression: ${v._0}`,
      UnsupportedOperator: (v) => `Unsupported operator: ${v._0}`,
    });
  }
}

