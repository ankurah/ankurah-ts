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
      return Result.Err(ParseError.Empty);
    }
    return Result.Ok(new Header(raw.toString()));
  }

  static parseTwice(raw: string): Result<[Header, Header], ParseError> {
    const _r0 = Header.parse(raw);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const first = _r0.unwrap();
    const _r1 = Header.parse(raw);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    const second = _r1.unwrap();
    return Result.Ok([first, second]);
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
    return true;
  }
}

