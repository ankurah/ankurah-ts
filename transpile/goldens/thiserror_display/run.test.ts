// Runs the emitted thiserror_display against the real runtime. What is under
// test is the text: thiserror builds each variant's message from its own
// `#[error("..")]`, and a port that prints anything else prints it into logs,
// into assertions and back to a user. The `#[from]` static is under test too —
// the class used to declare one name and the `?` site to call another.

import { expect, test } from 'bun:test';
import { Io, ParseError, Rule, Wrapped, parse, read } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a message with no fields is the string thiserror renders', () => {
  const e = new ParseError('Empty', {});
  expect(e.toString()).toBe('Empty expression');
  e.drop();
});

test('a positional placeholder reads the tuple field', () => {
  const e = new ParseError('Syntax', { _0: 'unexpected )' });
  expect(e.toString()).toBe('Syntax error: unexpected )');
  e.drop();
});

test('named placeholders read named fields, and {:?} goes through Debug', () => {
  const e = new ParseError('Unexpected', { expected: 'ident', got: new Rule('digit') });
  expect(e.toString()).toBe('Expected ident, got Rule { name: "digit" }');
  e.drop();
});

test('the variant name never appears in the text', () => {
  const e = new ParseError('Invalid', { _0: 'a = ' });
  expect(e.toString()).toBe('Invalid predicate: a = ');
  expect(e.toString()).not.toContain('Invalid(');
  e.drop();
});

test('the #[from] static builds the variant it was written on', () => {
  // `fromIo` takes the inner error by value: the variant owns it from here.
  const wrapped = ParseError.fromIo(new Io(5));
  expect(wrapped.type).toBe('Read');
  expect(wrapped.toString()).toBe('read failed');
  wrapped.drop();
});

test('a ? across the conversion calls that same static', () => {
  // `parse` and `read` borrow, so the driver still owns what it built.
  const source = new Io(7);
  const ok = parse(source);
  expect(ok.isOk()).toBe(true);
  expect(ok.unwrap()).toBe(7);
  expect(read(source).unwrap()).toBe(3 + 4);
  source.drop();
});

test('a transparent variant forwards its text to the error it wraps', () => {
  // The port used to write the variant's own name here, because the attribute
  // reader saw only the string form of `#[error]`.
  const wrapped = Wrapped.fromIo(new Io(7));
  const same = new Io(7);
  expect(wrapped.toString()).toBe(same.toString());
  same.drop();
  expect((wrapped.source() as Io).code).toBe(7);
  wrapped.drop();
  const said = new Wrapped('Said', { _0: 'so' });
  expect(said.toString()).toBe('said so');
  expect(said.source()).toBe(null);
  said.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
