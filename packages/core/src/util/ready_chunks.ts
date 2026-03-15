// MIRRORS: ankurah/core/src/util/ready_chunks.rs

/**
 * Yields batches (arrays) of all items that are ready at the time of a wake.
 * Analogous to Rust's ReadyChunks backed by FuturesUnordered.
 *
 * Behavioral contract:
 * 1. Wait for at least one Promise to settle
 * 2. Synchronously collect all other Promises that have already settled
 * 3. Yield the batch as an array
 * 4. Repeat until all Promises consumed
 */
export class ReadyChunks<T> implements AsyncIterable<T[]> {
  private remaining: number;
  private readonly total: number;
  private readonly ready: T[] = [];
  private rejectedCount: number = 0;

  // Signal mechanism: the iterator awaits `signal`, and settling promises
  // call `notify()` to resolve it.
  private signalResolve: (() => void) | null = null;
  private signal: Promise<void> | null = null;

  constructor(promises: Promise<T>[]) {
    this.total = promises.length;
    this.remaining = promises.length;

    // Arm the first signal
    this.armSignal();

    // Attach callbacks that push into the ready queue and notify the iterator.
    // Divergence: Rust futures yield Result<T, E> on cancellation; TS callers
    // encode errors as values (e.g. ApplyErrorItem) so rejections shouldn't
    // occur in practice. We still attach a no-op rejection handler to prevent
    // unhandled-rejection crashes, but rejected promises silently reduce
    // remaining so the iterator terminates [E8].
    for (const p of promises) {
      p.then(
        (value) => {
          this.ready.push(value);
          this.notify();
        },
        (_err) => {
          // Promise rejected — still need to count it as consumed so the
          // iterator doesn't hang. We push nothing to `ready`; the iterator
          // handles empty drains via the batch.length === 0 guard.
          this.rejectedCount++;
          this.notify();
        },
      );
    }
  }

  /** Returns true if all promises have been yielded. */
  isEmpty(): boolean {
    return this.remaining === 0;
  }

  /** Returns the number of promises not yet yielded in a batch. */
  len(): number {
    return this.remaining;
  }

  private armSignal(): void {
    this.signal = new Promise<void>((resolve) => {
      this.signalResolve = resolve;
    });
  }

  private notify(): void {
    if (this.signalResolve) {
      const resolve = this.signalResolve;
      this.signalResolve = null;
      resolve();
    }
  }

  async *[Symbol.asyncIterator](): AsyncIterableIterator<T[]> {
    while (this.remaining > 0) {
      // Wait for at least one promise to settle
      await this.signal;

      // Let microtasks flush so that all synchronously-settled promises
      // have had their .then() callbacks run.
      await Promise.resolve();

      // Account for any rejected promises (they don't produce values)
      if (this.rejectedCount > 0) {
        this.remaining -= this.rejectedCount;
        this.rejectedCount = 0;
      }

      // Drain the ready queue
      const batch = this.ready.splice(0);
      if (batch.length === 0) {
        // Woken by rejection(s) only — no values to yield. Re-arm if work remains.
        if (this.remaining > 0) {
          this.armSignal();
        }
        continue;
      }

      this.remaining -= batch.length;

      // Re-arm the signal for the next iteration (if promises remain)
      if (this.remaining > 0) {
        this.armSignal();
      }

      yield batch;
    }
  }
}
