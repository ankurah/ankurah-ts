// TS-ONLY: Maps tokio::time onto setTimeout.
//
// `Duration` is Copy in Rust and has no drop glue, so it crosses as a plain
// number of milliseconds: `Duration::from_millis(n)` is `n`, and
// `Duration::from_secs(n)` is `n * 1000`.

import { Result } from '../result.ts';
import { Struct } from '../struct.ts';
import { discardValue, reportFault } from './discard.ts';

/** The deadline passed before the future produced anything. */
export class Elapsed extends Struct {
  toString(): string { return 'deadline has elapsed'; }
}

/** Marks the deadline winning. A symbol, so no `T` can be mistaken for it. */
const EXPIRED: unique symbol = Symbol('ankurah.tokio.time.expired');

/**
 * The longest delay a host timer represents. `setTimeout` stores its delay in a
 * signed 32-bit field, and anything larger wraps — so a `Duration` of a month
 * would fire immediately rather than in a month.
 */
const LONGEST_TIMER = 2 ** 31 - 1;

/** A clock that only goes forwards, where the host has one. */
function now(): number {
  return typeof performance !== 'undefined' && typeof performance.now === 'function'
    ? performance.now()
    : Date.now();
}

/**
 * Wait until `millis` from now, however far away that is, and return a way to
 * call it off.
 *
 * A single `setTimeout` cannot express an arbitrary Rust `Duration`, so this
 * chases a deadline in hops the host can hold: each hop re-reads the clock and
 * schedules the rest, which also stops the wait from finishing early when the
 * host coalesces or delays a timer. Anything under a millisecond rounds up,
 * because a host timer has no finer resolution and rounding down would fire
 * before the deadline.
 */
function armTimer(millis: number, done: () => void): () => void {
  const wait = Number.isFinite(millis) ? Math.max(0, Math.ceil(millis)) : LONGEST_TIMER;
  const deadline = now() + wait;
  let timer = setTimeout(function step(): void {
    const remaining = deadline - now();
    if (remaining <= 0) {
      done();
      return;
    }
    timer = setTimeout(step, Math.min(remaining, LONGEST_TIMER));
  }, Math.min(wait, LONGEST_TIMER));
  return () => { clearTimeout(timer); };
}

/** `tokio::time::sleep` — resolves after `millis` milliseconds. */
export function sleep(millis: number): Promise<void> {
  return new Promise<void>((resolve) => { armTimer(millis, resolve); });
}

/**
 * `tokio::time::timeout` — the future's value, or `Elapsed` if the deadline
 * came first.
 *
 * tokio drops the future when the deadline wins, which cancels it. Nothing can
 * cancel a Promise, so the future here runs on; what it eventually produces
 * belongs to nobody, so this releases it rather than leaving it to be reported
 * as a leak. Whatever else the future does before it finishes, it still does.
 */
export async function timeout<T>(millis: number, future: PromiseLike<T>): Promise<Result<T, Elapsed>> {
  let expired = false;
  let cancelTimer: () => void = () => {};
  const deadline = new Promise<typeof EXPIRED>((resolve) => {
    cancelTimer = armTimer(millis, () => {
      expired = true;
      resolve(EXPIRED);
    });
  });
  const settled: Promise<{ value: T } | typeof EXPIRED> = Promise.resolve(future).then(
    (value) => {
      if (!expired) return { value };
      discardValue(value);
      return EXPIRED;
    },
    (thrown: unknown) => {
      // Before the deadline this is the caller's failure to handle. After it,
      // there is nobody to hand it to — and an ownership fatal must not be lost
      // in a rejection the settled race is no longer listening to.
      if (!expired) throw thrown;
      reportFault(thrown, 'ankurah: a timed-out future failed after its deadline.');
      return EXPIRED;
    },
  );
  try {
    const winner = await Promise.race([settled, deadline]);
    if (winner === EXPIRED) return Result.Err<T, Elapsed>(new Elapsed());
    return Result.Ok<T, Elapsed>(winner.value);
  } finally {
    cancelTimer();
  }
}
