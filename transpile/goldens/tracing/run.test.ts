// Runs the emitted tracing golden against the real runtime. What is under test
// is that the five macros reach a sink at all: the port used to write them as
// comments, so a ported program ran silently where the Rust one narrated
// itself. The driver installs a sink of its own, calls the two functions, and
// reads back the level and the rendered line each macro produced.

import { expect, test } from 'bun:test';
import { tracing } from '@ankurah/base';
import { Peer, connect, lost } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

/** What the sink recorded, as `level: message`. */
const recorded: string[] = [];

function capture<T>(run: () => T): string[] {
  recorded.length = 0;
  tracing.setSink((level: tracing.Level, message: string) => {
    recorded.push(`${level}: ${message}`);
  });
  try {
    run();
  } finally {
    tracing.setSink(tracing.consoleSink);
  }
  return [...recorded];
}

test('an info and a debug macro reach the sink with their arguments rendered', () => {
  const peer = new Peer(7);
  expect(capture(() => connect(peer))).toEqual([
    'info: connecting to 7',
    'debug: peer 7 state ready',
  ]);
  peer.drop();
});

test('a bare `warn!` behind a use, an error and a trace are the same three levels', () => {
  const peer = new Peer(3);
  expect(capture(() => lost(peer, 'socket closed'))).toEqual([
    'warn: lost 3: socket closed',
    'error: giving up on 3',
    'trace: done',
  ]);
  peer.drop();
});

test('the namespace the emitted calls are written against is the one base exports', () => {
  // The emitted module writes `tracing.info(..)`, so the five names have to be
  // on that namespace and callable. A missing one is a TypeError at the call.
  for (const level of ['trace', 'debug', 'info', 'warn', 'error'] as const) {
    expect(typeof tracing[level]).toBe('function');
  }
});

test('nothing leaked', async () => {
  await expectNoOwnershipReports();
});
