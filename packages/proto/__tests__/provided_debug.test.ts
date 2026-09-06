// TS-ONLY: the `debug()` the provided proto types declare.
//
// `#[derive(Debug)]` on a type holding one of these — `Event`, `EventFragment`,
// every request and update body — prints the field through the field type's own
// Debug. The engine never reads TypeScript it did not write, so it could not
// call one until `[provided_impls]` said `has_debug = true`, and printed the
// field through `toString` instead: `[object Object]` for a class. Forty-five
// emitted fields in proto alone.
//
// Each method here matches the Rust exactly, which is what these check:
//   - `EntityId`   `impl Debug` writes `self.to_base64()`
//   - `EventId`    `impl Debug` writes `EventId({to_base64})`
//   - the four ULID wrappers  `#[derive(Debug)]` over `ulid::Ulid`, itself
//     `#[derive(Debug)] pub struct Ulid(pub u128)`, so `Wrapper(Ulid(<decimal>))`
//   - `Clock`      `#[derive(Debug)]` over `Vec<EventId>`
//   - `Attested`   `#[derive(Debug)]` over named fields

import { expect, test } from 'bun:test';
import { Attested } from '../src/auth.provided.ts';
import { Attestation, AttestationSet } from '../src/auth.ts';
import { Clock } from '../src/clock.provided.ts';
import { EntityId, EventId, QueryId, RequestId, TransactionId, UpdateId } from '../src/id.provided.ts';

/** The 16 bytes of a ULID whose `u128` is 1. */
const ONE = new Uint8Array(16);
ONE[15] = 1;

test('EntityId prints its base64, as its own `impl Debug` writes it', () => {
  const id = EntityId.fromBytes(ONE);
  expect(id.debug()).toBe(id.toBase64());
  // and not the class's default, which is what `toString` on an object gives.
  expect(id.debug()).not.toContain('object');
  id.drop();
});

test('EventId prints its name around its base64', () => {
  const id = EventId.fromBytes(new Uint8Array(32));
  expect(id.debug()).toBe(`EventId(${id.toBase64()})`);
  id.drop();
});

test('the four ULID wrappers print the derived shape, over the inner u128', () => {
  const built = [
    [TransactionId.fromBytes(ONE), 'TransactionId(Ulid(1))'] as const,
    [RequestId.fromBytes(ONE), 'RequestId(Ulid(1))'] as const,
    [QueryId.fromBytes(ONE), 'QueryId(Ulid(1))'] as const,
    [UpdateId.fromBytes(ONE), 'UpdateId(Ulid(1))'] as const,
  ];
  for (const [id, written] of built) expect(id.debug()).toBe(written);

  // The whole 128 bits, most significant byte first.
  const big = TransactionId.fromBytes(new Uint8Array(16).fill(0xff));
  expect(big.debug()).toBe(`TransactionId(Ulid(${2n ** 128n - 1n}))`);

  for (const [id] of built) id.drop();
  big.drop();
});

test('Clock prints its sequence, each event through its own Debug', () => {
  const a = EventId.fromBytes(new Uint8Array(32));
  const clock = new Clock([a]);
  expect(clock.debug()).toBe(`Clock([${a.debug()}])`);
  const empty = new Clock([]);
  expect(empty.debug()).toBe('Clock([])');
  // `Clock` owns its ids, so dropping it drops `a` too.
  clock.drop();
  empty.drop();
});

test('Attested prints its named fields, and a payload with no debug() by its value', () => {
  const attested = new Attested(7, new AttestationSet([new Attestation(new Uint8Array([1]))]));
  expect(attested.debug()).toBe('Attested { payload: 7, attestations: AttestationSet([Attestation([1])]) }');
  attested.drop();
});
