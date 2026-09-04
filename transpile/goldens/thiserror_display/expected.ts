// MIRRORS: ankurah/thiserror_display/src/input.rs
import { Struct, Enum, Result } from '@ankurah/base';

export class Rule extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }

  debug(): string {
    return `Rule { name: ${JSON.stringify(this.name)} }`;
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
      Syntax: (v) => `Syntax(${JSON.stringify(v._0)})`,
      Unexpected: (v) => `Unexpected { expected: ${JSON.stringify(v.expected)}, got: ${v.got.debug()} }`,
      Invalid: (v) => `Invalid(${JSON.stringify(v._0)})`,
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

  static fromIo(inner: Io): ParseError {
    return new ParseError('Read', { _0: inner });
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

