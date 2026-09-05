// TS-ONLY: Tests for the serde_json::Error stand-in (src/serde_json.ts).
import { describe, test, expect } from 'bun:test';
import { JsonError, serde_json, Result, clearFatalLatch } from '../src/index.ts';
import { installOwnershipTestHooks } from '../src/testing.ts';

installOwnershipTestHooks();

/** Assert a fatal, and clear the latch so the test can keep going. */
function expectFatal(body: () => unknown, message: string): void {
  expect(body).toThrow(message);
  clearFatalLatch();
}

describe('serde_json::Error', () => {
  test('a custom error renders as the message alone', () => {
    // serde::de::Error::custom builds one with no position, and serde_json
    // prints the position only when it has one.
    const error = JsonError.custom('Invalid Base64: invalid symbol 45');
    expect(error.toString()).toBe('Invalid Base64: invalid symbol 45');
    expect(error.message).toBe('Invalid Base64: invalid symbol 45');
    expect(error.line).toBe(0);
    expect(error.column).toBe(0);
    error.drop();
  });

  test('a positioned error renders the message, the line and the column', () => {
    const error = JsonError.syntax('expected value', 3, 17);
    expect(error.toString()).toBe('expected value at line 3 column 17');
    expect(error.line).toBe(3);
    expect(error.column).toBe(17);
    error.drop();
  });

  test('a syntax error with no position renders like a custom one', () => {
    // serde_json decides on the line alone, so line 0 means "no position" and
    // the column is never printed on its own.
    const error = JsonError.syntax('trailing characters');
    expect(error.toString()).toBe('trailing characters');
    error.drop();
  });

  test('fromException carries the host message and no position', () => {
    const error = JsonError.fromException(new SyntaxError('Unexpected end of JSON input'));
    expect(error.message).toBe('Unexpected end of JSON input');
    expect(error.toString()).toBe('Unexpected end of JSON input');
    expect([error.line, error.column]).toEqual([0, 0]);
    error.drop();
  });

  test('fromException wraps what JSON.parse actually throws', () => {
    let error: JsonError | null = null;
    try {
      JSON.parse('{ not json');
    } catch (thrown) {
      error = JsonError.fromException(thrown);
    }
    expect(error).not.toBeNull();
    // The text is the host parser's and differs between engines, so the test
    // pins that it arrived rather than what it says.
    expect((error as JsonError).message.length).toBeGreaterThan(0);
    (error as JsonError).drop();
  });

  test('fromException renders a thrown value that is not an Error', () => {
    const fromString = JsonError.fromException('just a string');
    expect(fromString.message).toBe('just a string');
    fromString.drop();
    const fromRecord = JsonError.fromException({ code: 7 });
    expect(fromRecord.message).toBe('{"code":7}');
    fromRecord.drop();
    const fromNothing = JsonError.fromException(undefined);
    expect(fromNothing.message).toBe('undefined');
    fromNothing.drop();
  });

  test('it is the Err half of a Result, and unwrapErr hands it over', () => {
    const parsed: serde_json.Result<number> = Result.Err(JsonError.custom('invalid type'));
    expect(parsed.isErr()).toBe(true);
    const error = parsed.unwrapErr();
    expect(error.toString()).toBe('invalid type');
    error.drop();
  });

  test('serde_json.Error is JsonError under the name Rust uses', () => {
    const error = serde_json.Error.custom('named the way `use serde_json::Error` names it');
    expect(error).toBeInstanceOf(JsonError);
    error.drop();
  });

  test('reading a dropped error is fatal', () => {
    const error = JsonError.custom('gone');
    error.drop();
    expectFatal(() => error.message, 'BUG: serde_json::Error was used after being dropped');
    expectFatal(() => error.line, 'BUG: serde_json::Error was used after being dropped');
  });

  test('dropping one twice is fatal', () => {
    const error = JsonError.custom('once');
    error.drop();
    expectFatal(() => error.drop(), 'BUG: serde_json::Error was dropped twice');
  });

  test('rendering a dropped error says so instead of failing', () => {
    // toString runs on the panic path and in a debugger, which is precisely
    // when something has already gone wrong; it must never be the second fault.
    const error = JsonError.custom('gone');
    error.drop();
    expect(error.toString()).toBe('serde_json::Error (dropped)');
  });
});

// ── The lossless integer layer (R3) ──────────────────────────────
//
// serde_json keeps an integer token exactly. JSON.parse does not: it reads
// every number as a double, so a `u64` above 2^53 comes back rounded and cannot
// be recovered, and JSON.stringify writes a rounded token Rust then refuses.
// These pin both directions against what the Rust reference prints.
//
// Both answer a `Result`, as `serde_json::from_str` and `to_string` do, so a
// failure is a value the caller owns rather than an exception — and the
// `JsonError` in it is a tracked value, dropped here as Rust drops it.

import { parse, stringify } from '../src/serde_json.ts';

/** The value a successful parse produced. */
function parsed(text: string): unknown {
  const answer = parse(text);
  expect(answer.isOk()).toBe(true);
  return answer.unwrap();
}

/** That a parse failed, with the error released the way its owner would. */
function refused(text: string): void {
  const answer = parse(text);
  expect(answer.isErr()).toBe(true);
  answer.unwrapErr().drop();
}

/** The text a successful stringify produced. */
function written(value: unknown): string {
  const answer = stringify(value);
  expect(answer.isOk()).toBe(true);
  return answer.unwrap();
}

function unwritable(value: unknown): void {
  const answer = stringify(value);
  expect(answer.isErr()).toBe(true);
  answer.unwrapErr().drop();
}

describe('serde_json.parse keeps an integer token', () => {
  test('an integer beyond the safe range comes back as a bigint', () => {
    expect(parsed('9007199254740993')).toBe(9007199254740993n);
    expect(parsed('-9007199254740993')).toBe(-9007199254740993n);
    expect(parsed('18446744073709551615')).toBe(18446744073709551615n);
  });

  test('an integer a number holds exactly stays a number', () => {
    expect(parsed('0')).toBe(0);
    expect(parsed('1')).toBe(1);
    expect(parsed('-42')).toBe(-42);
    expect(parsed('9007199254740991')).toBe(9007199254740991);
  });

  test('a fractional or exponential number is a number', () => {
    expect(parsed('1.5')).toBe(1.5);
    expect(parsed('-0.25')).toBe(-0.25);
    expect(parsed('1e30')).toBe(1e30);
    expect(parsed('9007199254740993.0')).toBe(9007199254740992);
  });

  // serde_json refuses a float the format cannot hold rather than reading it as
  // an infinity, and `Infinity` is not a value it can write back out.
  test('an exponent past what a double holds is out of range', () => {
    refused('1e400');
    refused('1e999');
    refused('-1e400');
  });

  // JSON's number grammar, which `Number()` is looser than: `Number('01')` is 1,
  // `Number('1.')` is 1 and `Number('.5')` is 0.5, and none of the three is a
  // JSON number.
  test('a malformed number token is refused', () => {
    refused('01');
    refused('-01');
    refused('1.');
    refused('.5');
    refused('1e');
    refused('1e+');
  });

  // A raw control character inside a string is not JSON. `JSON.parse` says so by
  // THROWING, and the throw used to travel out of `parse` — an exception where
  // `from_str` answers `Err`, at seven live boundaries.
  test('a control character inside a string is an Err, not a throw', () => {
    refused(`"a${String.fromCharCode(1)}b"`);
    refused(`{"k": "a${String.fromCharCode(0)}b"}`);
    refused(String.raw`"\uZZZZ"`);
    // The escaped spelling is the legal one.
    expect(parsed(String.raw`"a\u0001b"`)).toBe(`a${String.fromCharCode(1)}b`);
  });

  test('everything else reads as JSON.parse reads it', () => {
    expect(parsed('null')).toBe(null);
    expect(parsed('true')).toBe(true);
    expect(parsed('"a\\nb"')).toBe('a\nb');
    expect(parsed('"\\ud83d\\ude80"')).toBe('🚀');
    expect(parsed('[]')).toEqual([]);
    expect(parsed('{}')).toEqual({});
    expect(parsed('  [1, "two", {"k": [null, false]}]  ')).toEqual([1, 'two', { k: [null, false] }]);
  });

  test('a wide integer inside a container is kept too', () => {
    expect(parsed('{"unsigned":9007199254740993,"signed":-9007199254740993}')).toEqual({
      unsigned: 9007199254740993n,
      signed: -9007199254740993n,
    });
    expect(parsed('[9007199254740993]')).toEqual([9007199254740993n]);
  });

  test('malformed input is an Err, not a guess and not a throw', () => {
    refused('');
    refused('{');
    refused('[1,]');
    refused('{"a" 1}');
    refused('1 2');
    refused('"unterminated');
    refused('tru');
  });
});

describe('serde_json.stringify writes a bigint as a bare integer token', () => {
  test('a bigint is the digits, not a string and not a throw', () => {
    expect(written(9007199254740993n)).toBe('9007199254740993');
    expect(written(-9007199254740993n)).toBe('-9007199254740993');
    expect(written(18446744073709551615n)).toBe('18446744073709551615');
  });

  test('the round trip through both is exact', () => {
    const text = '{"unsigned":9007199254740993,"signed":-9007199254740993}';
    expect(written(parsed(text))).toBe(text);
  });

  test('a value Rust could not read back is an Err', () => {
    // Above u64::MAX and below i64::MIN.
    unwritable(18446744073709551616n);
    unwritable(-9223372036854775809n);
    // serde_json refuses to write a non-finite float.
    unwritable(Number.NaN);
    unwritable(Number.POSITIVE_INFINITY);
  });

  test('everything else writes as JSON.stringify writes it', () => {
    expect(written(null)).toBe('null');
    expect(written(true)).toBe('true');
    expect(written(1.5)).toBe('1.5');
    expect(written('a\nb')).toBe('"a\\nb"');
    expect(written(['x', 1, null])).toBe('["x",1,null]');
    expect(written({ a: 1, b: [true] })).toBe('{"a":1,"b":[true]}');
    // An absent field is absent, which is what `Option::None` writes.
    expect(written({ a: 1, b: undefined })).toBe('{"a":1}');
  });
});

// X15: `out['__proto__'] = value` sets the object's PROTOTYPE instead of
// creating the member, so a document with that key parsed to an object that
// did not hold it — `hasOwnProperty('__proto__')` was false and `stringify`
// wrote the document back without it. serde_json treats `__proto__` as an
// ordinary key.
describe('a key named __proto__ is an ordinary key', () => {
  test('it becomes an own member, and the prototype is untouched', () => {
    const value = parsed('{"__proto__":{"polluted":true},"a":1}') as Record<string, unknown>;
    expect(Object.prototype.hasOwnProperty.call(value, '__proto__')).toBe(true);
    expect((value['__proto__'] as Record<string, unknown>)['polluted']).toBe(true);
    // Nothing was written to the prototype: an ordinary object still has none
    // of it.
    expect(({} as Record<string, unknown>)['polluted']).toBe(undefined);
    expect(Object.getPrototypeOf(value)).toBe(Object.prototype);
  });

  test('and it survives a round trip', () => {
    const value = parsed('{"__proto__":1}');
    expect(written(value)).toBe('{"__proto__":1}');
  });
});

// M: `JSON.parse` accepts a lone `\uD800` and hands back a string no UTF-8
// encoder can write out again; serde_json answers
// `Err(unexpected end of hex escape)`. Fifty documents through both agreed 49
// times, and this was the one that did not.
describe('an unpaired surrogate escape is refused', () => {
  test('a lone high or low surrogate is an error', () => {
    refused('"\\ud800"');
    refused('"\\udc00"');
    refused('"\\ud800a"');
    refused('"\\ud800\\u0041"');
  });

  test('a well-formed pair is one code point', () => {
    expect(parsed('"\\ud800\\udc00"')).toBe('\u{10000}');
    expect(parsed('"\\ud83d\\ude80"')).toBe('\u{1F680}');
  });

  test('and an escaped backslash is not the start of an escape', () => {
    expect(parsed('"\\\\ud800"')).toBe('\\ud800');
    expect(parsed('"a\\\\b"')).toBe('a\\b');
  });
});
