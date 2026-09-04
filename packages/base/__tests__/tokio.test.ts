// TS-ONLY: Tests for the tokio stand-ins in @ankurah/base.
//
// Two things are being checked throughout. The first is that each primitive
// behaves the way the tokio type it stands in for behaves — a permit that is
// consumed at the first poll and not before, a channel that closes when its last
// sender goes, a join handle that reports a cancelled task. The second is that
// each one obeys the ownership contract in port/ownership.md: every handle is
// taken exactly once, a value that nobody can receive any more is released
// rather than stranded, and an instance that is simply forgotten is reported as
// a leak.
import { describe, test, expect } from 'bun:test';
import {
  Drop, Struct, clearFatalLatch, setOnDiagnostic,
  Notify, AsyncMutex, AsyncRwLock, spawn, select, sleep, timeout,
  oneshot, mpsc, tokio,
  Sender, Receiver, UnboundedSender, UnboundedReceiver,
} from '../src/index.ts';
import { installOwnershipTestHooks } from '../src/testing.ts';

installOwnershipTestHooks();

/** Assert a fatal, and clear the latch so the test can keep going. */
function expectFatal(body: () => unknown, message: string): void {
  expect(body).toThrow(message);
  clearFatalLatch();
}

/** Let queued microtasks and timers run. */
async function turns(count = 4): Promise<void> {
  for (let i = 0; i < count; i++) await new Promise((resolve) => { setTimeout(resolve, 0); });
}

/** A payload with drop glue, for proving who releases an undelivered value. */
class Payload extends Drop {
  dropCount = 0;
  protected override onDrop(): void { this.dropCount++; }
}

/**
 * Collect the fatals the async layer re-raises from a fresh host task.
 *
 * A fatal that surfaces inside a promise reaction cannot be thrown to a caller,
 * so drop_registry re-raises it through hostTask(), which is queueMicrotask
 * where the host has one. Bun's runner claims 'uncaughtException' first, so the
 * only place to intercept is queueMicrotask itself.
 */
async function fatalsRaisedDuring(body: () => Promise<void>): Promise<string[]> {
  const raised: string[] = [];
  const realQueueMicrotask = globalThis.queueMicrotask;
  globalThis.queueMicrotask = (cb: () => void) => {
    realQueueMicrotask(() => {
      try {
        cb();
      } catch (e) {
        raised.push(String((e as Error).message));
        clearFatalLatch();
      }
    });
  };
  try {
    await body();
    await turns(4);
  } finally {
    globalThis.queueMicrotask = realQueueMicrotask;
    clearFatalLatch();
  }
  return raised;
}

// ── Notify ──

describe('Notify', () => {
  test('notify_one with nobody waiting stores a permit', async () => {
    const notify = new Notify();
    notify.notify_one();
    const waiter = notify.notified();
    expect(waiter.enable()).toBe(true);
    await waiter;
    notify.drop();
  });

  test('a permit is worth exactly one notification', () => {
    const notify = new Notify();
    notify.notify_one();
    notify.notify_one(); // saturates at one, as tokio's does
    const first = notify.notified();
    const second = notify.notified();
    expect(first.enable()).toBe(true);
    expect(second.enable()).toBe(false);
    second.drop();
    first.drop();
    notify.drop();
  });

  test('a permit goes to the waiter polled first, not the one created first', () => {
    // The whole reason Notified has a state before "waiting": creating one
    // registers nothing and consumes nothing.
    const notify = new Notify();
    notify.notify_one();
    const created_first = notify.notified();
    const created_second = notify.notified();
    expect(created_second.enable()).toBe(true);
    expect(created_first.enable()).toBe(false);
    created_first.drop();
    created_second.drop();
    notify.drop();
  });

  test('notify_one stores a permit when the waiters that exist have not been polled', () => {
    const notify = new Notify();
    const first = notify.notified();
    const second = notify.notified();
    notify.notify_one(); // neither is in the queue, so this is stored
    expect(second.enable()).toBe(true);
    expect(first.enable()).toBe(false);
    first.drop();
    second.drop();
    notify.drop();
  });

  test('notify_waiters stores nothing', () => {
    const notify = new Notify();
    notify.notify_waiters();
    const waiter = notify.notified();
    expect(waiter.enable()).toBe(false);
    waiter.drop();
    notify.drop();
  });

  test('notify_one wakes the waiter that has been queued longest', () => {
    const notify = new Notify();
    const first = notify.notified();
    const second = notify.notified();
    first.enable();
    second.enable();
    notify.notify_one();
    expect(first.enable()).toBe(true);
    expect(second.enable()).toBe(false);
    second.drop();
    first.drop();
    notify.drop();
  });

  test('notify_last wakes the most recently queued waiter', () => {
    const notify = new Notify();
    const first = notify.notified();
    const second = notify.notified();
    first.enable();
    second.enable();
    notify.notify_last();
    expect(first.enable()).toBe(false);
    expect(second.enable()).toBe(true);
    second.drop();
    first.drop();
    notify.drop();
  });

  test('notify_waiters wakes everyone queued now', async () => {
    const notify = new Notify();
    const woken: string[] = [];
    const first = notify.notified();
    const second = notify.notified();
    const secondDone = (async () => { await second; woken.push('second'); })();
    const firstDone = (async () => { await first; woken.push('first'); })();
    await turns(1); // let both awaits poll, which is what queues them
    notify.notify_waiters();
    await Promise.all([firstDone, secondDone]);
    expect(woken.sort()).toEqual(['first', 'second']);
    notify.drop();
  });

  test('a broadcast between creating a waiter and awaiting it is not missed', async () => {
    // The generation recorded at construction is what catches this: the first
    // poll sees a notify_waiters it does not recognise and completes at once.
    const notify = new Notify();
    const waiter = notify.notified();
    await sleep(0);
    notify.notify_waiters();
    await waiter;
    notify.drop();
  });

  test('dropping a queued Notified takes it out of the queue', () => {
    const notify = new Notify();
    const waiter = notify.notified();
    waiter.enable();
    waiter.drop();
    notify.notify_one(); // nobody is queued any more, so this is stored
    const later = notify.notified();
    expect(later.enable()).toBe(true);
    later.drop();
    notify.drop();
  });

  test('a notify_one nobody received is handed to the next waiter', () => {
    const notify = new Notify();
    const first = notify.notified();
    const second = notify.notified();
    first.enable();
    second.enable();
    notify.notify_one(); // picks first
    first.drop();        // ... which never received it, so second gets it
    expect(second.enable()).toBe(true);
    second.drop();
    notify.drop();
  });

  test('a notify_last nobody received is handed on by the same strategy', () => {
    const notify = new Notify();
    const first = notify.notified();
    const second = notify.notified();
    first.enable();
    second.enable();
    notify.notify_last(); // picks second
    second.drop();
    expect(first.enable()).toBe(true);
    first.drop();
    notify.drop();
  });

  test('a notify_one nobody received becomes a permit when nobody is left', async () => {
    const notify = new Notify();
    const waiter = notify.notified();
    waiter.enable();
    notify.notify_one();
    waiter.drop();
    const later = notify.notified();
    expect(later.enable()).toBe(true);
    await later;
    notify.drop();
  });

  test('a broadcast wake is never handed on', () => {
    // notify_waiters was for everyone at once; there is no notification owed to
    // anybody after it, so dropping a woken waiter leaves nothing behind.
    const notify = new Notify();
    const waiter = notify.notified();
    waiter.enable();
    notify.notify_waiters();
    waiter.drop();
    const later = notify.notified();
    expect(later.enable()).toBe(false);
    later.drop();
    notify.drop();
  });

  test('dropping a Notify with a waiter queued on it is fatal', () => {
    const notify = new Notify('GateNotify');
    const waiter = notify.notified();
    waiter.enable();
    expectFatal(
      () => notify.drop(),
      'BUG: GateNotify was dropped while a Notified is still outstanding.',
    );
    waiter.drop();
    notify.drop();
  });
});

// ── Named futures: one consumer ──

describe('Named futures', () => {
  test('awaiting one moves it, so the emitter drops nothing afterwards', async () => {
    const [tx, rx] = oneshot.channel<number>();
    tx.send(1).drop();
    expect((await rx).unwrap()).toBe(1);
    expectFatal(() => rx.try_recv(), 'BUG: oneshot::Receiver was used after being moved');
    expectFatal(() => rx.drop(), 'BUG: oneshot::Receiver was used after being moved');
  });

  test('a second await is fatal, even before the first one settles', async () => {
    const [tx, rx] = oneshot.channel<number>();
    const first = (async () => await rx)();
    await turns(1); // let the await take the receiver
    expectFatal(() => rx.then(() => {}), 'BUG: oneshot::Receiver was used after being moved');
    tx.send(9).drop();
    expect((await first).unwrap()).toBe(9);
  });

  test('a JoinHandle is consumed by the await too', async () => {
    const handle = spawn(async () => 1);
    (await handle).unwrap();
    expectFatal(() => handle.is_finished(), 'BUG: JoinHandle was used after being moved');
  });
});

// ── oneshot ──

describe('oneshot', () => {
  test('a sent value arrives at the receiver', async () => {
    const [tx, rx] = oneshot.channel<number>();
    const sent = tx.send(7);
    expect(sent.isOk()).toBe(true);
    sent.drop();
    const received = await rx;
    expect(received.unwrap()).toBe(7);
  });

  test('send consumes the sender, so a second send is fatal', () => {
    const [tx, rx] = oneshot.channel<number>();
    tx.send(1).drop();
    expectFatal(() => tx.send(2), 'BUG: oneshot::Sender was used after being moved');
    rx.drop();
  });

  test('dropping the receiver makes send fail and hands the value back', () => {
    const [tx, rx] = oneshot.channel<number>();
    rx.drop();
    const sent = tx.send(7);
    expect(sent.isErr()).toBe(true);
    expect(sent.unwrapErr()).toBe(7);
  });

  test('dropping the sender resolves the receiver with RecvError', async () => {
    const [tx, rx] = oneshot.channel<number>();
    tx.drop();
    const received = await rx;
    expect(received.isErr()).toBe(true);
    received.unwrapErr().drop();
  });

  test('try_recv reports Empty while the sender lives and Closed once it is gone', () => {
    const [tx, rx] = oneshot.channel<number>();
    const empty = rx.try_recv().unwrapErr();
    expect(empty.match({ Empty: () => 'empty', Closed: () => 'closed' })).toBe('empty');
    empty.drop();
    tx.drop();
    const closed = rx.try_recv().unwrapErr();
    expect(closed.match({ Empty: () => 'empty', Closed: () => 'closed' })).toBe('closed');
    closed.drop();
    rx.drop();
  });

  test('try_recv takes a value the sender already put in', () => {
    const [tx, rx] = oneshot.channel<number>();
    tx.send(3).drop();
    expect(rx.try_recv().unwrap()).toBe(3);
    rx.drop();
  });

  test('dropping the receiver releases a value still in flight', () => {
    const [tx, rx] = oneshot.channel<Payload>();
    const payload = new Payload();
    tx.send(payload).drop();
    rx.drop();
    expect(payload.dropCount).toBe(1);
  });

  test('Sender.closed resolves when the receiving half goes away', async () => {
    const [tx, rx] = oneshot.channel<number>();
    let sawClose = false;
    const watching = tx.closed().then(() => { sawClose = true; });
    await turns(1);
    expect(sawClose).toBe(false);
    rx.drop();
    await watching;
    expect(sawClose).toBe(true);
    expect(tx.is_closed()).toBe(true);
    tx.drop();
  });
});

// ── mpsc ──

describe('mpsc', () => {
  test('recv resolves to null once every sender is dropped and the buffer is drained', async () => {
    const [tx, rx] = mpsc.channel<number>(4);
    (await tx.send(1)).drop();
    (await tx.send(2)).drop();
    expect(await rx.recv()).toBe(1);
    tx.drop();
    expect(await rx.recv()).toBe(2); // still receivable after the sender is gone
    expect(await rx.recv()).toBe(null);
    rx.drop();
  });

  test('a bounded send waits for capacity', async () => {
    const [tx, rx] = mpsc.channel<number>(1);
    (await tx.send(1)).drop();
    let landed = false;
    const second = tx.send(2).then((sent) => { landed = true; sent.drop(); });
    await turns(2);
    expect(landed).toBe(false);
    expect(await rx.recv()).toBe(1);
    await second;
    expect(landed).toBe(true);
    expect(await rx.recv()).toBe(2);
    tx.drop();
    rx.drop();
  });

  test('capacity is handed to the sender that has waited longest', async () => {
    const [tx, rx] = mpsc.channel<number>(1);
    const other = tx.clone();
    (await tx.send(1)).drop();
    const firstParked = tx.send(2);
    const secondParked = other.send(3);
    expect(await rx.recv()).toBe(1);
    (await firstParked).drop();
    expect(await rx.recv()).toBe(2);
    (await secondParked).drop();
    expect(await rx.recv()).toBe(3);
    tx.drop();
    other.drop();
    rx.drop();
  });

  test('try_send reports Full and Closed, with the value handed back', () => {
    const [tx, rx] = mpsc.channel<number>(1);
    tx.try_send(1).unwrap();
    const full = tx.try_send(2).unwrapErr();
    expect(full.match({ Full: (v) => v._0, Closed: () => -1 })).toBe(2);
    full.drop();
    rx.drop();
    const closed = tx.try_send(3).unwrapErr();
    expect(closed.match({ Full: () => -1, Closed: (v) => v._0 })).toBe(3);
    closed.drop();
    tx.drop();
  });

  test('the channel stays open until the last sender clone drops', async () => {
    const [tx, rx] = mpsc.channel<number>(4);
    const clone = tx.clone();
    tx.drop();
    (await clone.send(9)).drop();
    expect(await rx.recv()).toBe(9);
    clone.drop();
    expect(await rx.recv()).toBe(null);
    rx.drop();
  });

  test('dropping the receiver makes sends fail with the value returned', async () => {
    const [tx, rx] = mpsc.channel<number>(4);
    rx.drop();
    const failed = (await tx.send(5)).unwrapErr();
    expect(failed._0).toBe(5);
    failed.drop();
    expect(tx.is_closed()).toBe(true);
    tx.drop();
  });

  test('a send parked for capacity gets its value back when the receiver drops', async () => {
    const [tx, rx] = mpsc.channel<number>(1);
    (await tx.send(1)).drop();
    const parked = tx.send(2);
    rx.drop();
    const failed = (await parked).unwrapErr();
    expect(failed._0).toBe(2);
    failed.drop();
    tx.drop();
  });

  test('dropping the receiver releases what is still queued', () => {
    const [tx, rx] = mpsc.unbounded_channel<Payload>();
    const payload = new Payload();
    tx.send(payload).drop();
    rx.drop();
    expect(payload.dropCount).toBe(1);
    tx.drop();
  });

  test('an unbounded send never waits', async () => {
    const [tx, rx] = mpsc.unbounded_channel<number>();
    for (const value of [1, 2, 3]) tx.send(value).drop();
    expect(await rx.recv()).toBe(1);
    expect(await rx.recv()).toBe(2);
    tx.drop();
    expect(await rx.recv()).toBe(3);
    expect(await rx.recv()).toBe(null);
    rx.drop();
  });

  test('try_recv reports Empty then Disconnected', () => {
    const [tx, rx] = mpsc.unbounded_channel<number>();
    const empty = rx.try_recv().unwrapErr();
    expect(empty.match({ Empty: () => 'empty', Disconnected: () => 'gone' })).toBe('empty');
    empty.drop();
    tx.drop();
    const gone = rx.try_recv().unwrapErr();
    expect(gone.match({ Empty: () => 'empty', Disconnected: () => 'gone' })).toBe('gone');
    gone.drop();
    rx.drop();
  });

  test('close refuses new messages but leaves the queued ones receivable', async () => {
    const [tx, rx] = mpsc.channel<number>(4);
    (await tx.send(1)).drop();
    rx.close();
    const refused = (await tx.send(2)).unwrapErr();
    expect(refused._0).toBe(2);
    refused.drop();
    expect(await rx.recv()).toBe(1);
    expect(await rx.recv()).toBe(null);
    tx.drop();
    rx.drop();
  });

  test('a bounded channel needs a buffer, as tokio does', () => {
    expect(() => mpsc.channel<number>(0)).toThrow('mpsc bounded channel requires buffer > 0');
  });
});

// ── select ──

describe('select', () => {
  test('the first branch to finish is the one reported', async () => {
    const winner = await select([
      { tag: 'slow', promise: sleep(50) },
      { tag: 'fast', promise: sleep(0) },
    ]);
    expect(winner.tag).toBe('fast');
  });

  test('the value of the winning branch comes back with its tag', async () => {
    const winner = await select([
      { tag: 'answer', promise: sleep(0).then(() => 42) },
      { tag: 'never', promise: sleep(1000) },
    ]);
    expect(winner).toEqual({ tag: 'answer', value: 42 });
  });

  test('when two branches are ready at once, only one output is taken', async () => {
    // Promise.race would run both continuations, take both outputs, and abandon
    // the one it did not report. Arbitration is by source order.
    const [firstTx, firstRx] = oneshot.channel<Payload>();
    const [secondTx, secondRx] = oneshot.channel<Payload>();
    const first = new Payload();
    const second = new Payload();
    firstTx.send(first).drop();
    secondTx.send(second).drop();

    const winner = await select([
      { tag: 'first', promise: firstRx },
      { tag: 'second', promise: secondRx },
    ]);
    expect(winner.tag).toBe('first');
    winner.value.unwrap().drop();
    expect(first.dropCount).toBe(1);
    expect(second.dropCount).toBe(0); // still in the losing channel

    // The emitted scope drops every branch, and that is what cancels the loser.
    firstRx.drop();
    secondRx.drop();
    expect(second.dropCount).toBe(1);
  });

  test('a value that arrives after the decision is released', async () => {
    const payload = new Payload();
    const winner = await select([
      { tag: 'winner', promise: sleep(0) },
      { tag: 'late', promise: sleep(5).then(() => payload) },
    ]);
    expect(winner.tag).toBe('winner');
    await sleep(30);
    expect(payload.dropCount).toBe(1);
  });

  test('a losing lock guard is released, so the lock does not stay held', async () => {
    const lock = new AsyncRwLock(1, 'SelectLock');
    const winner = await select([
      { tag: 'tick', promise: sleep(0) },
      { tag: 'guard', promise: (async () => { await sleep(5); return await lock.write(); })() },
    ]);
    expect(winner.tag).toBe('tick');
    await sleep(30);
    lock.try_write().unwrap().drop(); // would fail if the losing guard still held it
    lock.drop();
  });

  test('a branch is claimed by the select, so awaiting it elsewhere is fatal', async () => {
    const notify = new Notify();
    const waiter = notify.notified();
    const raced = select([
      { tag: 'shutdown', promise: waiter },
      { tag: 'tick', promise: sleep(0) },
    ]);
    expectFatal(() => waiter.then(() => {}), 'BUG: Notified on Notify was used after being moved');
    expect((await raced).tag).toBe('tick');
    waiter.drop();
    notify.drop();
  });

  test('a losing branch keeps running — the one tokio semantic that does not carry over', async () => {
    // select! drops the losing futures, which cancels them. Nothing cancels a
    // Promise, so the loser runs to completion and everything it does on the
    // way still happens.
    let loserFinished = false;
    const loser = (async () => { await sleep(5); loserFinished = true; })();
    const winner = await select([
      { tag: 'loser', promise: loser },
      { tag: 'winner', promise: sleep(0) },
    ]);
    expect(winner.tag).toBe('winner');
    expect(loserFinished).toBe(false);
    await sleep(30);
    expect(loserFinished).toBe(true);
    await loser;
  });

  test('dropping a losing Notified is what gets the cancellation back', async () => {
    // Where the losing branch is one of the named futures, the drop the emitted
    // scope owes it does what select!'s drop does — here, giving up its place
    // in the waiter queue.
    const notify = new Notify();
    const waiter = notify.notified();
    const winner = await select([
      { tag: 'shutdown', promise: waiter },
      { tag: 'tick', promise: sleep(0) },
    ]);
    expect(winner.tag).toBe('tick');
    waiter.drop();
    notify.notify_one(); // nobody queued, so this is stored
    const later = notify.notified();
    expect(later.enable()).toBe(true);
    later.drop();
    notify.drop();
  });
});

// ── spawn / JoinHandle ──

describe('spawn', () => {
  test('the task does not run on the calling stack', async () => {
    // tokio never polls a spawned future on the thread that spawned it, and
    // code that spawns while holding a lock depends on that.
    const order: string[] = [];
    const handle = spawn(async () => { order.push('task'); return 1; });
    order.push('after spawn');
    (await handle).unwrap();
    expect(order).toEqual(['after spawn', 'task']);
  });

  test('awaiting a JoinHandle yields what the task returned', async () => {
    const handle = spawn(async () => 42);
    expect((await handle).unwrap()).toBe(42);
  });

  test('spawn also takes an already-running future', async () => {
    const handle = spawn(Promise.resolve('running'));
    expect((await handle).unwrap()).toBe('running');
  });

  test('is_finished reports the handle, which abort settles at once', async () => {
    const handle = spawn(async () => { await sleep(5); return 1; });
    expect(handle.is_finished()).toBe(false);
    handle.abort();
    expect(handle.is_finished()).toBe(true);
    (await handle).unwrapErr().drop();
  });

  test('abort resolves the handle as cancelled while the task body runs on', async () => {
    // tokio stops the task at its next await point. Nothing can stop a running
    // async function here, so the body finishes and its output is discarded.
    let bodyFinished = false;
    const handle = spawn(async () => { await sleep(5); bodyFinished = true; return 1; });
    handle.abort();
    const joined = (await handle).unwrapErr();
    expect(joined.is_cancelled()).toBe(true);
    expect(joined.is_panic()).toBe(false);
    joined.drop();
    expect(bodyFinished).toBe(false);
    await sleep(30);
    expect(bodyFinished).toBe(true);
  });

  test('a task that throws joins as a panic', async () => {
    const handle = spawn(async () => { throw new Error('boom'); });
    const joined = (await handle).unwrapErr();
    expect(joined.is_panic()).toBe(true);
    joined.drop();
  });

  test('into_panic moves the payload out and consumes the error', async () => {
    const handle = spawn(async () => { throw new Error('boom'); });
    const joined = (await handle).unwrapErr();
    expect((joined.into_panic() as Error).message).toBe('boom');
    expectFatal(() => joined.is_cancelled(), 'BUG: JoinError was used after being moved');
  });

  test('into_panic panics on a JoinError that reports a cancellation', async () => {
    const handle = spawn(async () => 1);
    handle.abort();
    const joined = (await handle).unwrapErr();
    expect(() => joined.into_panic()).toThrow('reports a cancellation');
  });

  test('try_into_panic hands a cancellation error back to its owner', async () => {
    const handle = spawn(async () => 1);
    handle.abort();
    const joined = (await handle).unwrapErr();
    const attempt = joined.try_into_panic();
    expect(attempt.isErr()).toBe(true);
    const returned = attempt.unwrapErr();
    expect(returned.is_cancelled()).toBe(true);
    returned.drop();
  });

  test('a JoinError releases a thrown payload that has drop glue', async () => {
    const payload = new Payload();
    const handle = spawn(async () => { throw payload; });
    (await handle).unwrapErr().drop();
    expect(payload.dropCount).toBe(1);
  });

  test('dropping a JoinHandle detaches the task, which runs to completion', async () => {
    let ran = false;
    const handle = spawn(async () => { await sleep(1); ran = true; return 1; });
    handle.drop();
    await sleep(30);
    expect(ran).toBe(true);
  });

  test("a detached task's output is released rather than stranded", async () => {
    const payload = new Payload();
    const handle = spawn(async () => { await sleep(1); return payload; });
    handle.drop();
    await sleep(30);
    expect(payload.dropCount).toBe(1);
  });

  test("an aborted task's output is released rather than stranded", async () => {
    const payload = new Payload();
    const handle = spawn(async () => { await sleep(1); return payload; });
    handle.abort();
    (await handle).unwrapErr().drop();
    await sleep(30);
    expect(payload.dropCount).toBe(1);
  });

  test('a detached failure goes to the diagnostic handler, which is silent by default', async () => {
    const seen: unknown[] = [];
    setOnDiagnostic((_message, detail) => { seen.push(detail); });
    try {
      const handle = spawn(async () => { await sleep(1); throw new Error('detached boom'); });
      handle.drop();
      await sleep(30);
      expect(seen.length).toBe(1);
      expect((seen[0] as Error).message).toBe('detached boom');
    } finally {
      setOnDiagnostic(() => {});
    }
  });
});

// ── Fatals raised where there is no caller ──

describe('Async fatal reporting', () => {
  test('an ownership fatal inside a task is re-raised, never wrapped in a JoinError', async () => {
    // A JoinError is a Rust error value the emitted code may handle. A fatal is
    // not one, so the handle must never settle with it.
    let joined = 'never joined';
    const raised = await fatalsRaisedDuring(async () => {
      const doomed = new Payload();
      doomed.drop();
      const handle = spawn(async () => { doomed.drop(); return 1; });
      handle.then(() => { joined = 'joined'; });
      await turns(4);
    });
    expect(raised.some((r) => r.startsWith('BUG: Payload was dropped twice'))).toBe(true);
    expect(joined).toBe('never joined');
  });

  test('a fatal while releasing a value that missed its deadline is re-raised', async () => {
    const raised = await fatalsRaisedDuring(async () => {
      const doomed = new Payload();
      const late = sleep(20).then(() => { doomed.drop(); return doomed; });
      (await timeout(1, late)).unwrapErr().drop();
      await sleep(40);
    });
    expect(raised.some((r) => r.startsWith('BUG: Payload was dropped twice'))).toBe(true);
  });

  test('a fatal while releasing a losing select branch is re-raised', async () => {
    const raised = await fatalsRaisedDuring(async () => {
      const doomed = new Payload();
      const loser = sleep(10).then(() => { doomed.drop(); return doomed; });
      const winner = await select([
        { tag: 'winner', promise: sleep(0) },
        { tag: 'loser', promise: loser },
      ]);
      expect(winner.tag).toBe('winner');
      await sleep(40);
    });
    expect(raised.some((r) => r.startsWith('BUG: Payload was dropped twice'))).toBe(true);
  });
});

// ── time ──

describe('time', () => {
  test('timeout returns the value when the future finishes first', async () => {
    const result = await timeout(200, sleep(0).then(() => 'done'));
    expect(result.unwrap()).toBe('done');
  });

  test('timeout returns Elapsed when the deadline comes first', async () => {
    const result = await timeout(1, sleep(100).then(() => 'late'));
    expect(result.isErr()).toBe(true);
    result.unwrapErr().drop();
  });

  test('a value that arrives after the deadline is released, not stranded', async () => {
    const payload = new Payload();
    const result = await timeout(1, sleep(20).then(() => payload));
    result.unwrapErr().drop();
    await sleep(50);
    expect(payload.dropCount).toBe(1);
  });

  test('a deadline beyond the host timer range does not elapse early', async () => {
    // setTimeout stores its delay in a signed 32-bit field, so a Duration of a
    // month would otherwise fire at once instead of in a month.
    const result = await timeout(2 ** 31 + 5000, sleep(5).then(() => 'in time'));
    expect(result.unwrap()).toBe('in time');
  });

  test('sleep does not finish before its deadline', async () => {
    const started = Date.now();
    await sleep(25);
    expect(Date.now() - started).toBeGreaterThanOrEqual(24);
  });
});

// ── AsyncRwLock ──

describe('AsyncRwLock', () => {
  test('readers share the lock and a writer waits for them', async () => {
    const lock = new AsyncRwLock(0, 'Counter');
    const firstRead = await lock.read();
    const secondRead = await lock.read();
    expect(firstRead.value).toBe(0);
    expect(secondRead.value).toBe(0);

    let wrote = false;
    const writing = (async () => {
      const write = await lock.write();
      write.value = 5;
      wrote = true;
      write.drop();
    })();
    await turns(2);
    expect(wrote).toBe(false);

    firstRead.drop();
    secondRead.drop();
    await writing;

    const afterWrite = await lock.read();
    expect(afterWrite.value).toBe(5);
    afterWrite.drop();
    lock.drop();
  });

  test('a waiting writer blocks the readers behind it', async () => {
    const order: string[] = [];
    const lock = new AsyncRwLock(0);
    const held = await lock.read();

    const writing = (async () => { const w = await lock.write(); order.push('write'); w.drop(); })();
    const readingLater = (async () => { const r = await lock.read(); order.push('read'); r.drop(); })();

    await turns(2);
    held.drop();
    await Promise.all([writing, readingLater]);
    expect(order).toEqual(['write', 'read']);
    lock.drop();
  });

  test('try_write fails while a reader holds the lock, and succeeds once free', async () => {
    const lock = new AsyncRwLock(1);
    const held = await lock.read();
    const refused = lock.try_write();
    expect(refused.isErr()).toBe(true);
    refused.unwrapErr().drop();
    held.drop();
    lock.try_write().unwrap().drop();
    lock.drop();
  });

  test('try_read fails while a writer is queued, so a writer cannot be starved', async () => {
    const lock = new AsyncRwLock(1);
    const held = await lock.read();
    const writing = (async () => { const w = await lock.write(); w.drop(); })();
    await turns(1);
    const refused = lock.try_read();
    expect(refused.isErr()).toBe(true);
    refused.unwrapErr().drop();
    held.drop();
    await writing;
    lock.try_read().unwrap().drop();
    lock.drop();
  });

  test('get_mut and into_inner answer to the names the Rust stub declares', async () => {
    const lock = new AsyncRwLock(7, 'StubRwLock');
    expect(lock.get_mut()).toBe(7);
    const held = await lock.read();
    expectFatal(
      () => lock.get_mut(),
      'BUG: StubRwLock was dropped while a AsyncRwLockReadGuard is still outstanding.',
    );
    held.drop();
    expect(lock.into_inner()).toBe(7);
    expectFatal(() => lock.get_mut(), 'BUG: StubRwLock was used after being dropped');
  });

  test('into_inner hands the value to its caller instead of releasing it', () => {
    const payload = new Payload();
    const lock = new AsyncRwLock(payload, 'HandOverLock');
    expect(lock.into_inner()).toBe(payload);
    expect(payload.dropCount).toBe(0);
    payload.drop();
  });

  test('dropping the lock under a guard is fatal', async () => {
    const lock = new AsyncRwLock(1, 'HeldLock');
    const held = await lock.read();
    expectFatal(
      () => lock.drop(),
      'BUG: HeldLock was dropped while a AsyncRwLockReadGuard is still outstanding.',
    );
    held.drop();
    lock.drop();
  });

  test('dropping the lock releases what it holds', async () => {
    const payload = new Payload();
    const lock = new AsyncRwLock(payload, 'PayloadLock');
    const held = await lock.write();
    expect(held.value.dropCount).toBe(0);
    held.drop();
    lock.drop();
    expect(payload.dropCount).toBe(1);
  });
});

// ── AsyncMutex, under tokio's names ──

describe('AsyncMutex', () => {
  test('lock is acquire under the name the Rust stub declares', async () => {
    const mutex = new AsyncMutex(1, 'StubMutex');
    const guard = await mutex.lock();
    expect(guard.value).toBe(1);
    guard.drop();
    mutex.drop();
  });

  test('try_lock fails while the lock is held and succeeds once free', async () => {
    const mutex = new AsyncMutex(1, 'TryMutex');
    const guard = await mutex.lock();
    const refused = mutex.try_lock();
    expect(refused.isErr()).toBe(true);
    refused.unwrapErr().drop();
    guard.drop();
    const taken = mutex.try_lock().unwrap();
    expect(taken.value).toBe(1);
    taken.drop();
    mutex.drop();
  });

  test('a lock taken by try_lock still makes the next acquire wait', async () => {
    const mutex = new AsyncMutex(1, 'QueuedMutex');
    const taken = mutex.try_lock().unwrap();
    let second = false;
    const queued = (async () => { const g = await mutex.acquire(); second = true; g.drop(); })();
    await turns(2);
    expect(second).toBe(false);
    taken.drop();
    await queued;
    expect(second).toBe(true);
    mutex.drop();
  });

  test('into_inner hands the value to its caller instead of releasing it', () => {
    const payload = new Payload();
    const mutex = new AsyncMutex(payload, 'HandOverMutex');
    expect(mutex.into_inner()).toBe(payload);
    expect(payload.dropCount).toBe(0);
    expectFatal(() => mutex.get_mut(), 'BUG: HandOverMutex was used after being dropped');
    payload.drop();
  });
});

// ── The namespace ──

describe('tokio namespace', () => {
  test('the module tree mirrors the crate, so a path rewrite is the whole mapping', async () => {
    const notify = new tokio.sync.Notify();
    notify.notify_one();
    await notify.notified();
    notify.drop();

    const [tx, rx] = tokio.sync.mpsc.channel<number>(1);
    (await tx.send(1)).drop();
    expect(await rx.recv()).toBe(1);
    tx.drop();
    rx.drop();

    const handle = tokio.spawn(async () => 'via the namespace');
    expect((await handle).unwrap()).toBe('via the namespace');

    const winner = await tokio.select([{ tag: 'only', promise: tokio.time.sleep(0) }]);
    expect(winner.tag).toBe('only');
  });

  test('tokio.sync.Mutex is the AsyncMutex the runtime already had', async () => {
    const mutex = new tokio.sync.Mutex(1, 'NamespacedMutex');
    const guard = await mutex.lock();
    expect(guard.value).toBe(1);
    guard.drop();
    mutex.drop();
  });

  test('the mpsc ends are also exported bare, for `use tokio::sync::mpsc::Sender`', () => {
    const [tx, rx] = mpsc.channel<number>(1);
    expect(tx).toBeInstanceOf(Sender);
    expect(rx).toBeInstanceOf(Receiver);
    tx.drop();
    rx.drop();

    const [unboundedTx, unboundedRx] = mpsc.unbounded_channel<number>();
    expect(unboundedTx).toBeInstanceOf(UnboundedSender);
    expect(unboundedRx).toBeInstanceOf(UnboundedReceiver);
    unboundedTx.drop();
    unboundedRx.drop();
  });

  test('tokio.sync.RwLock is AsyncRwLock', async () => {
    const lock = new tokio.sync.RwLock(2, 'NamespacedRwLock');
    const guard = await lock.read();
    expect(guard.value).toBe(2);
    guard.drop();
    lock.drop();
  });
});

// ── Leak registry ──

/** Deliberately leaked, to prove the registry actually fired during the window. */
class LeakProbe extends Struct {}

/**
 * Runs `body`, lets everything it started settle, forces a collection, and
 * returns the leak-registry messages raised while it ran.
 *
 * drop_registry.ts reports a fatal leak from a fresh host task, which is
 * queueMicrotask where the host has one. Bun's test runner claims
 * 'uncaughtException' first, so the only place to intercept is queueMicrotask.
 */
async function leakReportsDuring(body: () => Promise<void>): Promise<string[]> {
  const reports: string[] = [];
  const realQueueMicrotask = globalThis.queueMicrotask;
  globalThis.queueMicrotask = (cb: () => void) => {
    realQueueMicrotask(() => {
      try {
        cb();
      } catch (e) {
        reports.push(String((e as Error).message));
        // Acknowledge the latch here rather than in the finally below. Every
        // leak in this window is provoked on purpose, and the cases have await
        // points in them — so a report that lands mid-body would otherwise
        // leave the runtime poisoned and refuse the drops that come after it.
        clearFatalLatch();
      }
    });
  };
  try {
    await body();
    // A handle held by a pending promise reaction is not collectable yet, so
    // let the tasks these cases start finish before asking for a collection.
    await turns(5);
    Bun.gc(true);
    for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  } finally {
    globalThis.queueMicrotask = realQueueMicrotask;
    clearFatalLatch(); // the reports above poisoned the runtime on purpose
  }
  return reports;
}

// Every tracked type here, proved twice over in one collection window: an
// instance that was abandoned IS reported, and an otherwise identical instance
// that was dropped is NOT. Asserting only the first would pass while the
// registry reported everything indiscriminately.

describe('Leak registry', () => {
  test('every tokio type reports an abandoned instance and not a dropped one', async () => {
    const cases: Array<{
      kind: string;
      leaked: string;
      dropped: string;
      abandon: () => Promise<void>;
      release: () => Promise<void>;
    }> = [
      {
        kind: 'Notify', leaked: 'LeakedNotify', dropped: 'DroppedNotify',
        abandon: async () => { new Notify('LeakedNotify'); },
        release: async () => { new Notify('DroppedNotify').drop(); },
      },
      {
        // Unpolled, so the Notify is not holding it and the collector can see it.
        kind: 'Notified', leaked: 'Notified on LeakedWaiter', dropped: 'Notified on DroppedWaiter',
        abandon: async () => {
          const notify = new Notify('LeakedWaiter');
          notify.notified();
          notify.drop();
        },
        release: async () => {
          const notify = new Notify('DroppedWaiter');
          notify.notified().drop();
          notify.drop();
        },
      },
      {
        kind: 'oneshot::Sender', leaked: 'oneshot::Sender on LeakedOneshotTx', dropped: 'oneshot::Sender on DroppedOneshotTx',
        abandon: async () => { oneshot.channel<number>('LeakedOneshotTx')[1].drop(); },
        release: async () => {
          const pair = oneshot.channel<number>('DroppedOneshotTx');
          pair[0].drop();
          pair[1].drop();
        },
      },
      {
        kind: 'oneshot::Receiver', leaked: 'oneshot::Receiver on LeakedOneshotRx', dropped: 'oneshot::Receiver on DroppedOneshotRx',
        abandon: async () => { oneshot.channel<number>('LeakedOneshotRx')[0].drop(); },
        release: async () => {
          const pair = oneshot.channel<number>('DroppedOneshotRx');
          pair[0].drop();
          pair[1].drop();
        },
      },
      {
        kind: 'mpsc::Sender', leaked: 'mpsc::Sender on LeakedMpscTx', dropped: 'mpsc::Sender on DroppedMpscTx',
        abandon: async () => { mpsc.channel<number>(1, 'LeakedMpscTx')[1].drop(); },
        release: async () => {
          const pair = mpsc.channel<number>(1, 'DroppedMpscTx');
          pair[0].drop();
          pair[1].drop();
        },
      },
      {
        kind: 'mpsc::Receiver', leaked: 'mpsc::Receiver on LeakedMpscRx', dropped: 'mpsc::Receiver on DroppedMpscRx',
        abandon: async () => { mpsc.channel<number>(1, 'LeakedMpscRx')[0].drop(); },
        release: async () => {
          const pair = mpsc.channel<number>(1, 'DroppedMpscRx');
          pair[0].drop();
          pair[1].drop();
        },
      },
      {
        kind: 'mpsc::UnboundedSender',
        leaked: 'mpsc::UnboundedSender on LeakedUnboundedTx', dropped: 'mpsc::UnboundedSender on DroppedUnboundedTx',
        abandon: async () => { mpsc.unbounded_channel<number>('LeakedUnboundedTx')[1].drop(); },
        release: async () => {
          const pair = mpsc.unbounded_channel<number>('DroppedUnboundedTx');
          pair[0].drop();
          pair[1].drop();
        },
      },
      {
        kind: 'mpsc::UnboundedReceiver',
        leaked: 'mpsc::UnboundedReceiver on LeakedUnboundedRx', dropped: 'mpsc::UnboundedReceiver on DroppedUnboundedRx',
        abandon: async () => { mpsc.unbounded_channel<number>('LeakedUnboundedRx')[0].drop(); },
        release: async () => {
          const pair = mpsc.unbounded_channel<number>('DroppedUnboundedRx');
          pair[0].drop();
          pair[1].drop();
        },
      },
      {
        kind: 'JoinHandle', leaked: 'JoinHandle for LeakedTask', dropped: 'JoinHandle for DroppedTask',
        abandon: async () => { spawn(Promise.resolve(1), 'LeakedTask'); await turns(2); },
        release: async () => {
          const handle = spawn(Promise.resolve(1), 'DroppedTask');
          await turns(2);
          handle.drop();
        },
      },
      {
        kind: 'AsyncRwLock', leaked: 'LeakedAsyncRwLock', dropped: 'DroppedAsyncRwLock',
        abandon: async () => { new AsyncRwLock(1, 'LeakedAsyncRwLock'); },
        release: async () => { new AsyncRwLock(1, 'DroppedAsyncRwLock').drop(); },
      },
      {
        kind: 'AsyncRwLockReadGuard',
        leaked: 'AsyncRwLockReadGuard on LeakedRwRead', dropped: 'AsyncRwLockReadGuard on DroppedRwRead',
        abandon: async () => { await new AsyncRwLock(1, 'LeakedRwRead').read(); },
        release: async () => {
          const lock = new AsyncRwLock(1, 'DroppedRwRead');
          (await lock.read()).drop();
          lock.drop();
        },
      },
      {
        kind: 'AsyncRwLockWriteGuard',
        leaked: 'AsyncRwLockWriteGuard on LeakedRwWrite', dropped: 'AsyncRwLockWriteGuard on DroppedRwWrite',
        abandon: async () => { await new AsyncRwLock(1, 'LeakedRwWrite').write(); },
        release: async () => {
          const lock = new AsyncRwLock(1, 'DroppedRwWrite');
          (await lock.write()).drop();
          lock.drop();
        },
      },
    ];

    const reports = await leakReportsDuring(async () => {
      new LeakProbe(); // never dropped
      for (const each of cases) {
        await each.abandon();
        await each.release();
      }
    });

    // Without this the test would pass vacuously whenever the collection or the
    // registry callback did not run at all.
    expect(reports.some((r) => r.startsWith('BUG: LeakProbe was'))).toBe(true);

    const reported = (label: string) => reports.some((r) => r.startsWith(`BUG: ${label} was garbage collected`));
    for (const each of cases) {
      expect([each.kind, reported(each.leaked)]).toEqual([each.kind, true]);
      expect([each.kind, reported(each.dropped)]).toEqual([each.kind, false]);
    }
  });

  test('a oneshot Sender consumed by send is not reported as a leak', async () => {
    const reports = await leakReportsDuring(async () => {
      new LeakProbe();
      const pair = oneshot.channel<number>('MovedSender');
      pair[0].send(1).drop();
      pair[1].drop();
    });

    expect(reports.some((r) => r.startsWith('BUG: LeakProbe was'))).toBe(true);
    expect(reports.filter((r) => r.includes('MovedSender'))).toEqual([]);
  });

  test('a named future consumed by an await is not reported as a leak', async () => {
    const reports = await leakReportsDuring(async () => {
      new LeakProbe();
      const pair = oneshot.channel<number>('AwaitedReceiver');
      pair[0].send(1).drop();
      (await pair[1]).unwrap();
    });

    expect(reports.some((r) => r.startsWith('BUG: LeakProbe was'))).toBe(true);
    expect(reports.filter((r) => r.includes('AwaitedReceiver'))).toEqual([]);
  });
});
