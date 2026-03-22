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
}

export type SqlGenerationErrorV = {
  PlaceholderCountMismatch: { expected: number; found: number };
  InvalidExpression: { _0: string };
  UnsupportedOperator: { _0: string };
};

export class SqlGenerationError extends Enum<SqlGenerationErrorV> {
}

