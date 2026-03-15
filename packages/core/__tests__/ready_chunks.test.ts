// MIRRORS: ankurah/core/src/util/ready_chunks.rs
import { describe, test, expect } from 'bun:test';
import { ReadyChunks } from '../src/util/ready_chunks.ts';

function delay<T>(ms: number, value: T): Promise<T> {
  return new Promise((resolve) => setTimeout(resolve, ms, value));
}

describe('ReadyChunks', () => {
  test('drains_all_simultaneously_ready', async () => {
    // All three promises are already resolved before iteration begins
    const promises = [Promise.resolve(1), Promise.resolve(2), Promise.resolve(3)];
    const chunks = new ReadyChunks(promises);

    const batches: number[][] = [];
    for await (const batch of chunks) {
      batches.push(batch);
    }

    // All pre-resolved promises should come in a single batch
    expect(batches.length).toBe(1);
    const values = batches[0].slice().sort();
    expect(values).toEqual([1, 2, 3]);
    expect(chunks.isEmpty()).toBe(true);
    expect(chunks.len()).toBe(0);
  });

  test('yields_pending_until_first_ready_then_drains', async () => {
    // Two promises that resolve at different times
    const p1 = delay(10, 10);
    const p2 = delay(80, 20);

    const chunks = new ReadyChunks([p1, p2]);

    const batches: number[][] = [];
    for await (const batch of chunks) {
      batches.push(batch);
    }

    // They should arrive in separate batches since their timings differ significantly
    expect(batches.length).toBe(2);
    expect(batches[0]).toEqual([10]);
    expect(batches[1]).toEqual([20]);
    expect(chunks.isEmpty()).toBe(true);
  });

  test('empty_stream_yields_none', async () => {
    const chunks = new ReadyChunks<number>([]);

    const batches: number[][] = [];
    for await (const batch of chunks) {
      batches.push(batch);
    }

    expect(batches.length).toBe(0);
    expect(chunks.isEmpty()).toBe(true);
    expect(chunks.len()).toBe(0);
  });

  test('mixed_immediate_and_delayed', async () => {
    // Some promises resolve immediately, some are delayed
    const immediate1 = Promise.resolve('a');
    const immediate2 = Promise.resolve('b');
    const delayed = delay(50, 'c');

    const chunks = new ReadyChunks([immediate1, immediate2, delayed]);

    const batches: string[][] = [];
    for await (const batch of chunks) {
      batches.push(batch);
    }

    // The two immediate ones should batch together, the delayed one separately
    expect(batches.length).toBe(2);

    const firstBatch = batches[0].slice().sort();
    expect(firstBatch).toEqual(['a', 'b']);
    expect(batches[1]).toEqual(['c']);
    expect(chunks.isEmpty()).toBe(true);
  });

  // Rust: fn includes_cancellations_in_chunk()
  // Divergence: Rust Canceled futures yield Result::Err which is still an output item;
  // TS rejected promises are silently consumed (no value pushed) so the iterator
  // terminates without hanging [E8].
  test('rejected_promises_dont_hang_iterator', async () => {
    const rejected = Promise.reject(new Error('canceled'));
    const chunks = new ReadyChunks<number>([rejected]);

    const batches: number[][] = [];
    for await (const batch of chunks) {
      batches.push(batch);
    }

    // Rejected promise produces no value — iterator terminates with no batches
    expect(batches.length).toBe(0);
    expect(chunks.isEmpty()).toBe(true);
  });

  // Rust: (no direct equivalent — tests mixed resolve+reject)
  test('mixed_resolve_and_reject', async () => {
    const resolved = Promise.resolve(42);
    const rejected = Promise.reject(new Error('canceled'));

    const chunks = new ReadyChunks<number>([resolved, rejected]);

    const batches: number[][] = [];
    for await (const batch of chunks) {
      batches.push(batch);
    }

    // Only the resolved promise produces a value
    expect(batches.length).toBe(1);
    expect(batches[0]).toEqual([42]);
    expect(chunks.isEmpty()).toBe(true);
  });

  test('len_and_isEmpty_track_remaining', async () => {
    const p1 = delay(10, 1);
    const p2 = delay(80, 2);

    const chunks = new ReadyChunks([p1, p2]);

    expect(chunks.len()).toBe(2);
    expect(chunks.isEmpty()).toBe(false);

    const iter = chunks[Symbol.asyncIterator]();

    const first = await iter.next();
    expect(first.done).toBe(false);
    expect(first.value).toEqual([1]);
    expect(chunks.len()).toBe(1);
    expect(chunks.isEmpty()).toBe(false);

    const second = await iter.next();
    expect(second.done).toBe(false);
    expect(second.value).toEqual([2]);
    expect(chunks.len()).toBe(0);
    expect(chunks.isEmpty()).toBe(true);

    const done = await iter.next();
    expect(done.done).toBe(true);
  });
});
