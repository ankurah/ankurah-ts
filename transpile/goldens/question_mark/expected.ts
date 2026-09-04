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
    const _r_first = Header.parse(raw);
    if (_r_first.isErr()) return _r_first as any;
    const first = _r_first.unwrap();
    const _r_second = Header.parse(raw);
    if (_r_second.isErr()) return _r_second as any;
    const second = _r_second.unwrap();
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

