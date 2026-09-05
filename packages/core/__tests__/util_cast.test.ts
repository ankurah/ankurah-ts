// MIRRORS: ankurah/core/src/util/cast.rs
//
// cast.rs has no tests — the macros are public API and every call site is in an
// application — so these pin what the ported functions promise instead: the
// reflexive pass-through, the one conversion hook, and create!'s delegation to
// `Transaction::create`. The two divergences the file's header names (positional
// fields, and a conversion picked by the source rather than the target) are what
// the first two tests are about.

import { describe, test, expect } from 'bun:test';
import { create, into } from '../src/util/cast.ts';

class Ref {
  constructor(readonly name: string) {}
}

/** A value that converts on the way into a field, the way a View does in Rust. */
class View {
  constructor(readonly name: string) {}
  into(): Ref { return new Ref(this.name); }
}

class ConnectionEvent {
  constructor(readonly user: Ref, readonly session: Ref, readonly timestamp: number) {}
}

describe('util/cast', () => {
  test('into converts each field through its own into()', () => {
    const event = into(ConnectionEvent, new View('alice'), new View('s1'), 17);
    expect(event.user).toBeInstanceOf(Ref);
    expect(event.user.name).toBe('alice');
    expect(event.session.name).toBe('s1');
  });

  test('a value with no into() is passed through — Rust\'s reflexive Into<Self>', () => {
    const event = into(ConnectionEvent, new Ref('alice'), new Ref('s1'), 17);
    expect(event.user.name).toBe('alice');
    expect(event.timestamp).toBe(17);
  });

  test('fields are positional, in the constructor\'s declaration order', () => {
    // The macro writes a struct literal and Rust checks each value against the
    // field beside it. Here the order is the whole check.
    const event = into(ConnectionEvent, new View('u'), new View('s'), 1);
    expect(event.user.name).toBe('u');
    expect(event.session.name).toBe('s');
  });

  test('create hands the built model to the transaction and passes its answer back', async () => {
    const seen: ConnectionEvent[] = [];
    const trx = {
      async create(model: ConnectionEvent) {
        seen.push(model);
        return 'created' as const;
      },
    };

    const answer = await create(trx, ConnectionEvent, new View('bob'), new View('s2'), 42);

    expect(answer).toBe('created');
    expect(seen.length).toBe(1);
    expect(seen[0].user).toBeInstanceOf(Ref);
    expect(seen[0].user.name).toBe('bob');
    expect(seen[0].timestamp).toBe(42);
  });
});
