// MIRRORS: ankurah/ankql/src/error.rs
import { Enum } from '@ankurah/base';

export type ParseErrorV = {
  SyntaxError: { _0: string };
  EmptyExpression: {};
  UnexpectedRule: { expected: string; got: string };
  InvalidPredicate: { _0: string };
  MissingOperand: { _0: string };
};

export class ParseError extends Enum<ParseErrorV> {
  get message(): string {
    return this.match({
      SyntaxError: (v) => v._0,
      EmptyExpression: () => 'Empty expression',
      UnexpectedRule: (v) => `Expected ${v.expected}, got ${v.got}`,
      InvalidPredicate: (v) => v._0,
      MissingOperand: (v) => v._0,
    });
  }

  override toString(): string {
    return `${super.toString()}: ${this.message}`;
  }
}

export type SqlGenerationErrorV = {
  PlaceholderCountMismatch: { expected: number; found: number };
  InvalidExpression: { _0: string };
  UnsupportedOperator: { _0: string };
};

export class SqlGenerationError extends Enum<SqlGenerationErrorV> {
  get message(): string {
    return this.match({
      PlaceholderCountMismatch: (v) => `Expected ${v.expected} placeholders, found ${v.found}`,
      InvalidExpression: (v) => v._0,
      UnsupportedOperator: (v) => v._0,
    });
  }

  override toString(): string {
    return `${super.toString()}: ${this.message}`;
  }
}

