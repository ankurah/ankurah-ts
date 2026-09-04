// TS-ONLY: Maps tokio::sync::Notify to a waiter list, a single permit, and a
// broadcast generation.
//
// Notify is how the port's readiness gates are written: `system.rs` and
// `livequery.rs` both park on `notified()` until whoever finishes the work calls
// `notify_waiters()`. Everything here exists to make that handshake impossible
// to lose.
//
// The three states a `Notified` moves through are tokio's, and the distinction
// between them is the whole design:
//
//   created — it has recorded the broadcast generation and nothing else. It is
//             not in the queue, so `notify_one()` does not see it.
//   waiting — its first poll found no broadcast and no permit, so it joined the
//             queue and can be picked by `notify_one()` / `notify_last()`.
//   done    — it has its notification.
//
// A `Notified` therefore does not consume a stored permit when it is created,
// only when it is first polled or enabled. `notify_one(); const a = notified();
// const b = notified(); b.enable()` gives the permit to `b`, because `a` was
// never polled — and wake order among queued waiters is poll order, not
// creation order.

import { DropGuard } from '../std/drop.ts';
import { dropContainer } from '../std/guard.ts';
import { NamedFuture } from './future.ts';

/**
 * How a wake reached a waiter. The distinction matters on drop: tokio hands an
 * unreceived `notify_one` / `notify_last` on to the next waiter, because that
 * notification was for exactly one task and dropping it would lose it. A
 * `notify_waiters` broadcast was for everyone at once and is never forwarded.
 */
type WakeKind = 'one-fifo' | 'one-lifo' | 'all';

/**
 * The future `Notify::notified()` hands back.
 *
 * It records the broadcast generation at construction, so a `notify_waiters()`
 * between creating it and awaiting it still completes it — that is what makes
 * `const n = notify.notified(); await doThing(); await n;` safe. It joins the
 * `notify_one` queue only when first polled.
 */
export class Notified extends NamedFuture<void> {
  readonly #notify: Notify;
  readonly #generation: number;
  #stage: 'created' | 'waiting' | 'done' = 'created';
  /** A one-at-a-time notification this waiter was given and never received. */
  #unreceived: 'one-fifo' | 'one-lifo' | null = null;

  /** @internal — only Notify creates these. */
  constructor(notify: Notify, notifyLabel: string, generation: number) {
    // Named after the Notify it is parked on, the way a guard is named after
    // its container: a leak report then points at the handshake that went
    // wrong rather than at the type.
    super(`Notified on ${notifyLabel}`);
    this.#notify = notify;
    this.#generation = generation;
  }

  /**
   * tokio's `Notified::enable`: do the registration the first poll would do,
   * and report whether that alone completed the future. This is how a caller
   * arms a waiter before doing something that may notify it.
   */
  enable(): boolean {
    this.assertNotDropped();
    this.poll();
    return this.isSettled();
  }

  /** The first poll: a broadcast since creation, then a permit, then the queue. */
  protected override pollOnce(): void {
    if (this.#notify.enlist(this, this.#generation)) {
      this.#stage = 'done';
      this.settle();
      return;
    }
    this.#stage = 'waiting';
  }

  /** @internal — picked out of the queue by one of the notify methods. */
  wake(kind: WakeKind): void {
    this.#stage = 'done';
    this.#unreceived = kind === 'all' ? null : kind;
    this.settle();
  }

  /** A notification carries nothing: tokio's `Notified` has `Output = ()`. */
  protected override takeOutput(): void {
    // Received, so there is nothing left to hand on if this is dropped.
    this.#unreceived = null;
  }

  protected override onDrop(): void {
    if (this.#stage === 'waiting') {
      this.#stage = 'done';
      this.#notify.unpark(this);
      return;
    }
    const forward = this.#unreceived;
    if (forward === null) return;
    this.#unreceived = null;
    // This waiter was given a one-at-a-time notification and never received it.
    // Dropping it here would lose the notification outright, so it goes on to
    // the next waiter by the same strategy, or becomes a stored permit.
    if (forward === 'one-fifo') this.#notify.notify_one();
    else this.#notify.notify_last();
  }
}

/**
 * tokio::sync::Notify — a wake-up with no payload.
 *
 * It holds at most one permit: `notify_one()` with nobody in the queue stores
 * it, so the next waiter to be polled completes immediately instead of parking
 * forever. `notify_waiters()` is the other half of the pair — it wakes everyone
 * queued right now, bumps the generation so a waiter created before it but not
 * yet polled also completes, and stores nothing.
 */
export class Notify {
  readonly #waiters: Notified[] = [];
  #permit = false;
  #generation = 0;
  readonly #guard: DropGuard;
  readonly #label: string;

  constructor(label?: string) {
    this.#label = label ?? 'Notify';
    this.#guard = new DropGuard(this, this.#label);
  }

  /**
   * A future that waits for a notification. Nothing is consumed here: it takes
   * the permit, or joins the queue, when it is first polled or enabled.
   */
  notified(): Notified {
    this.#guard.assertNotDropped();
    return new Notified(this, this.#label, this.#generation);
  }

  /** Wake the waiter that has been queued longest, or store a permit. */
  notify_one(): void {
    this.#guard.assertNotDropped();
    const waiter = this.#waiters.shift();
    if (waiter === undefined) {
      this.#permit = true;
      return;
    }
    waiter.wake('one-fifo');
  }

  /** Wake the most recently queued waiter, or store a permit. */
  notify_last(): void {
    this.#guard.assertNotDropped();
    const waiter = this.#waiters.pop();
    if (waiter === undefined) {
      this.#permit = true;
      return;
    }
    waiter.wake('one-lifo');
  }

  /**
   * Wake everyone waiting now, and store nothing. The generation bump is what
   * reaches a waiter that was created before this call but has not been polled
   * yet: its first poll sees a generation it does not recognise and completes.
   * A waiter created after this call waits for the next notification.
   */
  notify_waiters(): void {
    this.#guard.assertNotDropped();
    this.#generation++;
    const woken = this.#waiters.splice(0, this.#waiters.length);
    for (const waiter of woken) waiter.wake('all');
  }

  /**
   * @internal — a waiter's first poll. Returns whether that alone completed it:
   * a broadcast since it was created, or a permit it could take. Otherwise it
   * joins the queue and waits.
   */
  enlist(waiter: Notified, generation: number): boolean {
    this.#guard.assertNotDropped();
    if (generation !== this.#generation) return true;
    if (this.#permit) {
      this.#permit = false;
      return true;
    }
    this.#waiters.push(waiter);
    return false;
  }

  /** @internal — a queued Notified that was dropped before it was woken. */
  unpark(waiter: Notified): void {
    this.#guard.assertNotDropped();
    const at = this.#waiters.indexOf(waiter);
    if (at !== -1) this.#waiters.splice(at, 1);
  }

  /**
   * A Notify owns nothing, so dropping one releases only itself — but a waiter
   * still queued on it borrows it, and Rust's borrow checker makes dropping it
   * out from under that waiter impossible. Reaching here with waiters means the
   * emitted drop scope is wrong, and the parked task would wait forever.
   */
  drop(): void {
    dropContainer(
      this,
      this.#guard,
      this.#label,
      () => (this.#waiters.length > 0 ? 'Notified' : null),
      () => undefined,
    );
  }
}
