// TS-ONLY: the shape tokio's three named futures share.
//
// `Notified`, `oneshot::Receiver` and `JoinHandle` are values in Rust, not
// `async fn` calls: each is a struct with `impl Future`, so the source can hold
// one, hand it to `select!`, and drop it. They are here because the port has to
// be able to do the same three things.
//
// **One consumer.** A future has exactly one owner, so the first thing that
// takes it takes it for good. Awaiting one moves it — `.await` takes a future
// by value, the same way `Result`'s self-taking methods take a Result — so the
// emitter emits no drop for an awaited future, and a second await, or any other
// use, is fatal. `select` takes a different claim: it borrows the future for the
// duration of the race and leaves ownership with the emitted scope, which drops
// every branch afterwards exactly as `select!` does.
//
// **Cancelling** one is dropping it before it completes, and each subclass's
// onDrop() does what tokio does there: unregister the waiter and hand its
// notification on, close the channel end, detach the task.
//
// **The output never lives here.** Each subclass parks it where it belongs — in
// the channel, in the join handle — so a future that is cancelled before anyone
// takes delivery releases the payload from the one place that owns it.

import { Drop } from '../std/drop.ts';
import { fatalUseAfterMove } from '../drop_registry.ts';

/** Who took the future. Rust allows exactly one of these to happen. */
type Claim = 'none' | 'await' | 'select';

export abstract class NamedFuture<T> extends Drop implements PromiseLike<T> {
  #settled = false;
  #polled = false;
  #claim: Claim = 'none';
  readonly #listeners: Array<() => void> = [];

  /**
   * The output is ready where this future parked it. Settling one that already
   * settled, or one the emitted code cancelled, changes nothing: the payload
   * stays with whoever parked it, and that owner releases it.
   */
  protected settle(): void {
    if (this.#settled || this.isDropped) return;
    this.#settled = true;
    const waiting = this.#listeners.splice(0, this.#listeners.length);
    for (const notify of waiting) notify();
  }

  /** Whether the output has arrived. The future may still be untaken. */
  protected isSettled(): boolean { return this.#settled; }

  /**
   * Rust's first poll, which for some futures is when the real work of
   * registering starts. Run once, whichever claim comes first.
   */
  protected poll(): void {
    if (this.#polled) return;
    this.#polled = true;
    this.pollOnce();
  }

  /** What the first poll does beyond waiting. Nothing, for most futures. */
  protected pollOnce(): void {}

  /** Move the output out. Called once, by whoever takes delivery. */
  protected abstract takeOutput(): T;

  then<R1 = T, R2 = never>(
    onfulfilled?: ((value: T) => R1 | PromiseLike<R1>) | undefined | null,
    onrejected?: ((reason: any) => R2 | PromiseLike<R2>) | undefined | null,
  ): PromiseLike<R1 | R2> {
    this.assertNotDropped();
    // `.await` takes the future by value. Marking it moved here rather than at
    // delivery is what makes a second await — or any later use of the binding —
    // fatal from the moment the first one starts, which is when Rust's move
    // happens.
    this.#claimBy('await');
    return this.#deliver().then(onfulfilled, onrejected);
  }

  /**
   * @internal — select's claim. It blocks a competing await for the length of
   * the race but does not move the future, because `select!`'s branches are
   * dropped by the emitted scope once the select returns.
   */
  claimForSelect(onSettled: () => void): void {
    this.assertNotDropped();
    this.#claimBy('select');
    this.#onSettled(onSettled);
  }

  /** @internal — select taking the output of the branch that won. */
  takeSelectOutput(): T {
    this.assertNotDropped();
    return this.takeOutput();
  }

  #claimBy(kind: Exclude<Claim, 'none'>): void {
    // An await has already marked this moved, so assertNotDropped catches that
    // case first; this catches a second select, and a select after an await
    // that has not yet been observed.
    if (this.#claim !== 'none') fatalUseAfterMove(this.$label);
    this.#claim = kind;
    if (kind === 'await') this.markMoved();
    this.poll();
  }

  #onSettled(cb: () => void): void {
    if (this.#settled) {
      cb();
      return;
    }
    this.#listeners.push(cb);
  }

  #deliver(): Promise<T> {
    return new Promise<T>((resolve) => {
      this.#onSettled(() => { resolve(this.takeOutput()); });
    });
  }
}
