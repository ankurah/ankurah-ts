// MIRRORS: ankurah/thiserror_display/src/input.rs
import { Struct, Enum, Result, debugString } from '@ankurah/base';

export class Rule extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }

  debug(): string {
    return `Rule { name: ${debugString(this.name)} }`;
  }
}

export class Io extends Struct {
  readonly code: number;

  constructor(code: number) {
    super();
    this.code = code;
  }

  debug(): string {
    return `Io { code: ${String(this.code)} }`;
  }
}

export type ParseErrorV = {
  Empty: {};
  Syntax: { _0: string };
  Unexpected: { expected: string; got: Rule };
  Invalid: { _0: string };
  Read: { _0: Io };
};

export class ParseError extends Enum<ParseErrorV> {

  debug(): string {
    return this.match({
      Empty: () => 'Empty',
      Syntax: (v) => `Syntax(${debugString(v._0)})`,
      Unexpected: (v) => `Unexpected { expected: ${debugString(v.expected)}, got: ${v.got.debug()} }`,
      Invalid: (v) => `Invalid(${debugString(v._0)})`,
      Read: (v) => `Read(${v._0.debug()})`,
    });
  }

  override toString(): string {
    return this.match({
      Empty: () => 'Empty expression',
      Syntax: (v) => `Syntax error: ${v._0}`,
      Unexpected: (v) => `Expected ${v.expected}, got ${v.got.debug()}`,
      Invalid: (v) => `Invalid predicate: ${v._0}`,
      Read: () => 'read failed',
    });
  }

  /** The error this one wraps: Rust's `Error::source`. */
  source(): unknown {
    switch (this.type) {
      case 'Read': return (this.value as any)._0;
      default: return null;
    }
  }

  static fromIo(inner: Io): ParseError {
    return new ParseError('Read', { _0: inner });
  }
}

export type WrappedV = {
  Passed: { _0: Io };
  Said: { _0: string };
};

export class Wrapped extends Enum<WrappedV> {

  debug(): string {
    return this.match({
      Passed: (v) => `Passed(${v._0.debug()})`,
      Said: (v) => `Said(${debugString(v._0)})`,
    });
  }

  override toString(): string {
    return this.match({
      Passed: (v) => v._0.toString(),
      Said: (v) => `said ${v._0}`,
    });
  }

  /** The error this one wraps: Rust's `Error::source`. */
  source(): unknown {
    switch (this.type) {
      case 'Passed': return (this.value as any)._0;
      default: return null;
    }
  }

  static fromIo(inner: Io): Wrapped {
    return new Wrapped('Passed', { _0: inner });
  }
}

export function parse(source: Io): Result<number, ParseError> {
  const _r0 = read(source);
  if (_r0.isErr()) return Result.Err(ParseError.fromIo(_r0.unwrapErr()));
  const n = _r0.unwrap();
  return Result.Ok(n);
}

export function read(source: Io): Result<number, Io> {
  return Result.Ok(source.code);
}

