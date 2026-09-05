// MIRRORS: ankurah/core/src/util/mod.rs (the five #[macro_export] macros)
//
// The macros have no Rust tests — they expand to `tracing::*` calls — so what is
// pinned here is the text each one writes, escape for escape, because the escapes
// are the whole point of the macro: an action line is the actor in bold blue, an
// arrow, the action in green, and the rest dimmed; a notice is bold yellow.

import { describe, test, expect, beforeEach, afterEach } from 'bun:test';
import { tracing } from '@ankurah/base';
import { actionDebug, actionError, actionInfo, actionWarn, noticeInfo } from '../src/util/index.ts';

const written: [tracing.Level, string][] = [];

beforeEach(() => {
  written.length = 0;
  tracing.setSink((level, message) => { written.push([level, message]); });
});

afterEach(() => {
  tracing.setSink(tracing.consoleSink);
});

describe('util action macros', () => {
  test('action_info! with a thing and an action', () => {
    actionInfo('Node(a1)', 'register_peer');
    expect(written).toEqual([['info', '\x1b[1;34mNode(a1)\x1b[0m → \x1b[32mregister_peer\x1b[0m']]);
  });

  test('action_info! with a thing, an action and context', () => {
    actionInfo('Node(a1)', 'register_peer', 'presence(b2)');
    expect(written).toEqual([
      ['info', '\x1b[1;34mNode(a1)\x1b[0m → \x1b[32mregister_peer\x1b[0m \x1b[2mpresence(b2)\x1b[0m'],
    ]);
  });

  test('a thing is rendered by Display, so a ported value writes its toString', () => {
    actionInfo({ toString: () => 'Node(a1)' }, 'unsubscribing');
    expect(written[0][1]).toBe('\x1b[1;34mNode(a1)\x1b[0m → \x1b[32munsubscribing\x1b[0m');
  });

  test('the four action macros differ only in level', () => {
    actionInfo('t', 'a');
    actionDebug('t', 'a');
    actionWarn('t', 'a');
    actionError('t', 'a');
    expect(written.map(([level]) => level)).toEqual(['info', 'debug', 'warn', 'error']);
    expect(new Set(written.map(([, message]) => message)).size).toBe(1);
  });

  test('notice_info! writes one message in bold yellow, at info', () => {
    noticeInfo('Node(a1) created as durable');
    expect(written).toEqual([['info', '\x1b[1;33mNode(a1) created as durable\x1b[0m']]);
  });
});
