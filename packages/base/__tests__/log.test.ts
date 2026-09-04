// TS-ONLY: Tests for the `tracing` stand-in (src/log.ts).
import { describe, test, expect, afterEach } from 'bun:test';
import { tracing } from '../src/index.ts';
import { installOwnershipTestHooks } from '../src/testing.ts';

installOwnershipTestHooks();

/** Capture what the five level functions write, and hand back the record. */
function captured(): Array<[tracing.Level, string]> {
  const record: Array<[tracing.Level, string]> = [];
  tracing.setSink((level, message) => { record.push([level, message]); });
  return record;
}

// Every test replaces the sink, so put the shipped one back or the next suite
// in the run inherits a sink that writes into a dead array.
afterEach(() => { tracing.setSink(tracing.consoleSink); });

describe('tracing', () => {
  test('each level reaches the sink under its own name', () => {
    const record = captured();
    tracing.trace('t');
    tracing.debug('d');
    tracing.info('i');
    tracing.warn('w');
    tracing.error('e');
    expect(record).toEqual([
      ['trace', 't'],
      ['debug', 'd'],
      ['info', 'i'],
      ['warn', 'w'],
      ['error', 'e'],
    ]);
  });

  test('the message arrives already rendered, exactly as it was passed', () => {
    const record = captured();
    // What the emitter writes for `tracing::info!("peer {} sent {} events", id, n)`.
    const id = 'p1';
    const n = 3;
    tracing.info(`peer ${id} sent ${n} events`);
    expect(record).toEqual([['info', 'peer p1 sent 3 events']]);
  });

  test('a replacement sink takes over from the one before it', () => {
    const first: string[] = [];
    const second: string[] = [];
    tracing.setSink((_level, message) => { first.push(message); });
    tracing.info('to the first');
    tracing.setSink((_level, message) => { second.push(message); });
    tracing.info('to the second');
    expect(first).toEqual(['to the first']);
    expect(second).toEqual(['to the second']);
  });

  test('consoleSink puts the shipped behaviour back', () => {
    const record = captured();
    tracing.setSink(tracing.consoleSink);
    // Reinstalled, so the capture stops receiving. Nothing asserts the console
    // here: what matters is that the export is the default and restoring it
    // works, which is what every other test in this file relies on.
    expect(record).toEqual([]);
  });

  test('consoleSink writes one call per event on the level-carrying method', () => {
    const calls: Array<[string, unknown[]]> = [];
    const console_ = globalThis.console;
    const record = (name: string) => (...args: unknown[]) => { calls.push([name, args]); };
    globalThis.console = {
      ...console_, debug: record('debug'), info: record('info'),
      warn: record('warn'), error: record('error'), trace: record('trace'),
    } as Console;
    try {
      tracing.setSink(tracing.consoleSink);
      tracing.trace('t');
      tracing.debug('d');
      tracing.info('i');
      tracing.warn('w');
      tracing.error('e');
    } finally {
      globalThis.console = console_;
    }
    // trace goes to console.debug, not console.trace: console.trace prints a
    // stack trace and `tracing::trace!` does not.
    expect(calls).toEqual([
      ['debug', ['t']],
      ['debug', ['d']],
      ['info', ['i']],
      ['warn', ['w']],
      ['error', ['e']],
    ]);
  });
});
