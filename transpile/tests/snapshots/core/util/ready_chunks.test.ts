// MIRRORS: ankurah/core/src/util/ready_chunks.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { ReadyChunks } from './ready_chunks';
import { oneshot, tokio } from '@ankurah/base';

describe('ready_chunks unit tests', () => {
  test('drains_all_simultaneously_ready', async () => {
    const [tx1, rx1] = oneshot.channel();
    const [tx2, rx2] = oneshot.channel();
    const [tx3, rx3] = oneshot.channel();
    let stream = ReadyChunks.new([rx1, rx2, rx3]);
    tx1.send(1).unwrap();
    tx2.send(2).unwrap();
    const chunk = await stream.next();
    const values = [...chunk].map((r) => r);
    values.sort((a, b) => a.compareTo(b));
    expect(values).toEqual([1, 2]);
    tx3.send(3).unwrap();
    const chunk2 = await stream.next();
    const values2 = [...chunk2].map((r) => r);
    expect(values2).toEqual([3]);
    if (!(await stream.next() == null)) throw new Error('assertion failed');
  });

  test('yields_pending_until_first_ready_then_drains', async () => {
    const [tx1, rx1] = oneshot.channel();
    const [tx2, rx2] = oneshot.channel();
    let stream = ReadyChunks.new([rx1, rx2]);
    tokio.spawn((async () => {
      await tokio.time.sleep(time.Duration.fromMillis(10n));
      const _ = tx1.send(10);
    })());
    tokio.spawn((async () => {
      await tokio.time.sleep(time.Duration.fromMillis(30n));
      const _ = tx2.send(20);
    })());
    const first = await stream.next();
    const values = [...first].map((r) => r);
    expect(values).toEqual([10]);
    const second = await stream.next();
    const values2 = [...second].map((r) => r);
    expect(values2).toEqual([20]);
    if (!(await stream.next() == null)) throw new Error('assertion failed');
  });

  test('empty_stream_yields_none', async () => {
    const futs = [];
    let stream = ReadyChunks.new(futs);
    if (!(await stream.next() == null)) throw new Error('assertion failed');
  });

  test('includes_cancellations_in_chunk', async () => {
    const [tx, rx] = oneshot.channel();
    tx.drop();
    let stream = ReadyChunks.new([rx]);
    const chunk = await stream.next();
    expect(chunk.length).toEqual(1);
    if (!(chunk[0].isErr())) throw new Error('assertion failed');
    if (!(await stream.next() == null)) throw new Error('assertion failed');
  });

});
