// TS-ONLY: Maps tokio::select! to an arbitrated race between tagged branches.
//
// `select!` is a macro over syntax; a function cannot be. So the emitter turns
// each arm into a `{ tag, promise }` pair and matches on the tag it gets back,
// which puts the arm bodies where the emitter can already put them — in a
// switch — instead of inside the primitive.
//
// This is not `Promise.race`. A race subscribes every branch and lets each one
// run its continuation, so two branches that are ready at once both take their
// outputs and only one of them is reported — the other is silently abandoned.
// Exactly one branch's output is taken here, and everything that arrives after
// the decision is released.
//
// **Arbitration is by source order**, decided at one checkpoint after the
// branches are subscribed: whatever is ready by then, the earliest branch in the
// list wins; after that it is first past the post. tokio's unbiased `select!`
// picks a random polling order among ready branches, so a fixed order is one of
// its permitted outcomes — and it is exactly what `biased;` asks for. Being
// deterministic is worth more to a port than reproducing the randomness.
//
// **The one semantic that does not carry over.** `select!` drops the futures
// that lost, which cancels them: a `sleep` stops, a `recv` gives up its place in
// the queue, a spawned task's work stops at its next await point. A losing
// Promise here keeps running to completion, and whatever it was going to do it
// still does. What the emitter can get back: `select!` drops every branch —
// winner and losers alike — when it returns, so the emitted scope drops all of
// them in a `finally`, including on the exceptional path where a branch threw
// and there is no tag to route to. For a `Notified`, a `oneshot::Receiver` or a
// `JoinHandle` that drop is the real cancellation, which is why this function
// leaves the branch futures un-moved and does not take a loser's output: the
// emitter's drop is what releases it, and for a `Notified` that is also what
// hands its notification on to the next waiter.

import { NamedFuture } from './future.ts';
import { discardValue, reportFault } from './discard.ts';
import { hostTask } from '../drop_registry.ts';

/** One arm of a select: a name to match on, and the future it waits for. */
export interface SelectBranch<Tag extends string, T> {
  readonly tag: Tag;
  readonly promise: PromiseLike<T>;
}

/** What the arm that won produced. */
export type SelectOutcome<B> = B extends SelectBranch<infer Tag, infer T> ? { tag: Tag; value: T } : never;

/**
 * A branch that has produced something. A named future keeps its output until
 * somebody takes it, so this records the future rather than a value; a plain
 * promise has already handed its value over by the time we hear about it.
 */
interface Arrival {
  readonly index: number;
  readonly ok: boolean;
  readonly named: NamedFuture<unknown> | null;
  readonly value?: unknown;
  readonly thrown?: unknown;
}

/**
 * Wait for the first branch to produce a value, and report which one it was.
 *
 * A branch that throws makes the whole select throw, because there is no arm to
 * route it to — `select!` has no such case, since a Rust future does not fail
 * on its own.
 */
export function select<const B extends readonly SelectBranch<string, any>[]>(
  branches: B,
): Promise<SelectOutcome<B[number]>> {
  const ready: Arrival[] = [];
  let decided = false;
  let arbitrated = false;
  let deliver!: (outcome: unknown) => void;
  let fail!: (thrown: unknown) => void;
  const outcome = new Promise<unknown>((resolve, reject) => {
    deliver = resolve;
    fail = reject;
  });

  /** This branch won: its output is transferred to the caller, and only its. */
  const win = (arrival: Arrival): void => {
    decided = true;
    if (!arrival.ok) {
      fail(arrival.thrown);
      return;
    }
    const value = arrival.named === null ? arrival.value : arrival.named.takeSelectOutput();
    deliver({ tag: (branches[arrival.index] as SelectBranch<string, unknown>).tag, value });
  };

  /**
   * This branch lost, or arrived after the decision. A named future keeps its
   * output: the emitted scope drops that future, and its drop glue is what both
   * releases the output and cancels the future the way `select!` would have.
   * Everything else is released here, which is what stops a losing lock guard
   * from holding its lock forever.
   */
  const lose = (arrival: Arrival): void => {
    if (arrival.named !== null) return;
    if (arrival.ok) {
      discardValue(arrival.value);
      return;
    }
    reportFault(arrival.thrown, 'ankurah: a losing select branch failed after the select had returned.');
  };

  const record = (arrival: Arrival): void => {
    if (decided) {
      lose(arrival);
      return;
    }
    if (!arbitrated) {
      ready.push(arrival);
      return;
    }
    win(arrival);
  };

  branches.forEach((branch, index) => {
    const awaited = branch.promise;
    if (awaited instanceof NamedFuture) {
      // The privileged claim: it blocks a competing await but leaves the future
      // where the emitted scope can still drop it.
      awaited.claimForSelect(() => { record({ index, ok: true, named: awaited }); });
      return;
    }
    // fire-and-forget: a branch reports through record(), which arbitrates. The
    // select's own promise is what the caller awaits.
    Promise.resolve(awaited).then(
      (value) => { record({ index, ok: true, named: null, value }); },
      (thrown: unknown) => { record({ index, ok: false, named: null, thrown }); },
    );
  });

  hostTask(() => {
    arbitrated = true;
    if (decided || ready.length === 0) return;
    ready.sort((a, b) => a.index - b.index);
    const winner = ready.shift() as Arrival;
    const losers = ready.splice(0, ready.length);
    win(winner);
    for (const loser of losers) lose(loser);
  });

  return outcome as Promise<SelectOutcome<B[number]>>;
}
