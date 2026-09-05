// Runs the emitted lifted_loop_jump against the real runtime. What is under
// test is where a jump written inside a LIFTED body lands. An arm of `match` is
// an arrow function, so a `return` in one has to travel out as a sentinel — but
// a `continue` naming a `for` written INSIDE that arm is an ordinary continue,
// because the loop is in the same arrow. Handed back as a sentinel it left the
// arm on the first NUL byte: ankql's `generate_expr_sql` wrote an unterminated
// SQL literal, and `sql.test.ts test_null_byte_handling` failed on it.

import { expect, test } from 'bun:test';
import { BorrowMut } from '@ankurah/base';
import { Lit, firstOver, quote, quoteAll } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

const NUL = String.fromCharCode(0);

test('the NUL byte is skipped and the literal is still closed', () => {
  const out = new BorrowMut('');
  const lit = new Lit('Text', { _0: `a${NUL}b` });
  const answer = quote(lit, out);
  lit.drop(); // `quote` takes `&Lit`; the temporary is the caller's
  expect(answer.isOk()).toBe(true);
  expect(answer.unwrap()).toBe(4);
  expect(out.value).toBe("'ab'");
});

test('the arm that returns still leaves the whole function', () => {
  const out = new BorrowMut('');
  const lit = new Lit('Count', { _0: 0 });
  const answer = quote(lit, out);
  lit.drop();
  expect(answer.isErr()).toBe(true);
  const refusal = answer.unwrapErr();
  expect(refusal.is('Empty')).toBe(true);
  refusal.drop();
  expect(out.value).toBe('');
});

test('the other arm falls through to the value after the match', () => {
  const out = new BorrowMut('');
  const lit = new Lit('Count', { _0: 3 });
  const answer = quote(lit, out);
  lit.drop();
  expect(answer.isOk()).toBe(true);
  answer.drop();
  expect(out.value).toBe('n');
});

test('a labelled break written past an inner loop leaves the outer loop', () => {
  const out = new BorrowMut('');
  const lits = [new Lit('Text', { _0: 'ab!cd' }), new Lit('Count', { _0: 5 })];
  const answer = quoteAll(lits, out);
  expect(answer.isOk()).toBe(true);
  answer.drop();
  // `break 'rows` left the `for` over `lits`, so the Count arm never ran.
  expect(out.value).toBe('ab');
  for (const lit of lits) lit.drop();
});

test('and the return inside that loop still leaves the function', () => {
  const out = new BorrowMut('');
  const lits = [new Lit('Text', { _0: 'ab' }), new Lit('Count', { _0: 0 })];
  const answer = quoteAll(lits, out);
  expect(answer.isErr()).toBe(true);
  answer.drop();
  expect(out.value).toBe('ab');
  for (const lit of lits) lit.drop();
});

// Z4: the payload a labelled `break` carries is what the loop produces. The
// marker used to be handed back before the payload was translated, so the loop
// answered whatever it had been initialised with — `undefined` here.
test('a labelled break carries its payload out of the lift', () => {
  expect(firstOver([[1, 2], [3, 9], [4]], 5)).toBe(9);
  // Nothing over the limit: the loop's own `break 0` answers.
  expect(firstOver([[1, 2], [3]], 5)).toBe(0);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
