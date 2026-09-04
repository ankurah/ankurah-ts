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
