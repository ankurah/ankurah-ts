// MIRRORS: ankurah/question_mark/src/input.rs
import { Struct, Enum, Result } from '@ankurah/base';

export class Header extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }

  static parse(raw: string): Result<Header, ParseError> {
    if (raw.length === 0) {
      return Result.Err(new ParseError('Empty', {}));
    }
    return Result.Ok(new Header(raw));
  }

  static parseTwice(raw: string): Result<[Header, Header], ParseError> {
    const _r0 = Header.parse(raw);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const first = _r0.unwrap();
    try {
      const _r2 = Header.parse(raw);
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      let _moved3 = false;
      const second = _r2.unwrap();
      try {
        _moved1 = true;
        _moved3 = true;
        return Result.Ok([first, second]);
      } finally {
        if (!_moved3) second.drop();
      }
    } finally {
      if (!_moved1) first.drop();
    }
  }
}

export type ParseErrorV = {
  Empty: {};
};

export class ParseError extends Enum<ParseErrorV> {

  clone(): ParseError {
    return new ParseError(this.type, { ...this.value });
  }

  equals(other: ParseError): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  debug(): string {
    return this.match({
      Empty: () => 'Empty',
    });
  }
}

