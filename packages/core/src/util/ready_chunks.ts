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

  // Signal mechanism: the iterator awaits `signal`, and settling promises
  // call `notify()` to resolve it.
  private signalResolve: (() => void) | null = null;
  private signal: Promise<void> | null = null;

  constructor(promises: Promise<T>[]) {
    this.total = promises.length;
    this.remaining = promises.length;

    // Arm the first signal
    this.armSignal();

    // Attach callbacks that push into the ready queue and notify the iterator
    for (const p of promises) {
      p.then((value) => {
        this.ready.push(value);
        this.notify();
      });
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

      // Drain the ready queue
      const batch = this.ready.splice(0);
      if (batch.length === 0) {
        // Shouldn't happen, but guard against spurious wakes
        this.armSignal();
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
