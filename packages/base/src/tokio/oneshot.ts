// TS-ONLY: Maps tokio::sync::oneshot to a promise and its resolver.
//
// One value travels from one place to one other place, once. `node.rs` parks a
// Sender in its pending-request map and awaits the Receiver until the peer
// answers; `cb_race.rs` hands the Sender to a pile of JS callbacks and keeps
// whichever fires first. Both need the same thing from the channel: an answer
// that cannot arrive twice, and a definite failure when the other end is gone.
//
// The value in flight belongs to the channel. Whichever end is dropped while it
// is still in there releases it, which is why the payload lives in the shared
// state rather than in either handle.

import { Drop } from '../std/drop.ts';
import { Enum } from '../enum.ts';
import { Result } from '../result.ts';
import { Struct } from '../struct.ts';
import { dropOwned } from '../object.ts';
import { NamedFuture } from './future.ts';

/** The sending half went away before it sent anything. */
export class RecvError extends Struct {
  toString(): string { return 'channel closed'; }
}

type TryRecvErrorV = {
  Empty: {};
  Closed: {};
};

/** Why a non-blocking receive found nothing. */
export class TryRecvError extends Enum<TryRecvErrorV> {
  static Empty = (): TryRecvError => new TryRecvError('Empty', {});
  static Closed = (): TryRecvError => new TryRecvError('Closed', {});
}

interface ChannelState<T> {
  value: T | undefined;
  hasValue: boolean;
  senderGone: boolean;
  /** The receiving half is dropped or was closed by hand; sends fail. */
  receiverClosed: boolean;
  receiver: Receiver<T> | null;
  /** Callers parked in `Sender.closed()`. */
  closedWaiters: Array<() => void>;
}

function closeReceivingHalf<T>(state: ChannelState<T>): void {
  state.receiverClosed = true;
  const waiting = state.closedWaiters.splice(0, state.closedWaiters.length);
  for (const wake of waiting) wake();
}

/**
 * The sending half. `send` takes `self` in Rust, so it consumes this handle:
 * afterwards the Sender is moved, not dropped, and using it again is fatal.
 */
export class Sender<T> extends Drop {
  readonly #state: ChannelState<T>;

  /** @internal — only channel() creates these. */
  constructor(state: ChannelState<T>, label: string) {
    super(label);
    this.#state = state;
  }

  /**
   * Send the value and consume this handle. `Err(value)` hands the value back
   * when the receiving half is already gone, because at that point nobody else
   * can own it.
   */
  send(value: T): Result<undefined, T> {
    this.assertNotDropped();
    const state = this.#state;
    state.senderGone = true;
    if (state.receiverClosed) {
      this.markMoved();
      return Result.Err(value);
    }
    state.value = value;
    state.hasValue = true;
    state.receiver?.ready();
    this.markMoved();
    return Result.Ok(undefined);
  }

  /** Whether the receiving half is gone, so a send would fail. */
  is_closed(): boolean {
    this.assertNotDropped();
    return this.#state.receiverClosed;
  }

  /** Resolves once the receiving half is dropped or closed. */
  closed(): Promise<void> {
    this.assertNotDropped();
    const state = this.#state;
    if (state.receiverClosed) return Promise.resolve();
    return new Promise<void>((resolve) => { state.closedWaiters.push(resolve); });
  }

  /** Dropping the sender without sending closes the channel: the receiver fails. */
  protected override onDrop(): void {
    const state = this.#state;
    state.senderGone = true;
    state.receiver?.ready();
  }
}

/**
 * The receiving half. Awaiting it consumes it, the way `.await` consumes any
 * future; dropping it first makes the pending `send` fail and releases a value
 * already in flight.
 */
export class Receiver<T> extends NamedFuture<Result<T, RecvError>> {
  readonly #state: ChannelState<T>;

  /** @internal — only channel() creates these. */
  constructor(state: ChannelState<T>, label: string) {
    super(label);
    this.#state = state;
  }

  /** @internal — a value arrived, or the sending half went away. */
  ready(): void {
    this.settle();
  }

  /**
   * Take the value without waiting. `Empty` means the sender still exists and
   * has not sent; `Closed` means it never will.
   */
  try_recv(): Result<T, TryRecvError> {
    this.assertNotDropped();
    const state = this.#state;
    if (state.hasValue) {
      const value = state.value as T;
      state.value = undefined;
      state.hasValue = false;
      return Result.Ok(value);
    }
    if (state.senderGone || state.receiverClosed) return Result.Err(TryRecvError.Closed());
    return Result.Err(TryRecvError.Empty());
  }

  /**
   * Refuse any value the sender has not sent yet. A value already in flight is
   * still there to be taken, which is why this does not settle a full channel.
   */
  close(): void {
    this.assertNotDropped();
    closeReceivingHalf(this.#state);
    if (!this.#state.hasValue) this.settle();
  }

  protected override takeOutput(): Result<T, RecvError> {
    const state = this.#state;
    if (!state.hasValue) return Result.Err(new RecvError());
    const value = state.value as T;
    state.value = undefined;
    state.hasValue = false;
    return Result.Ok(value);
  }

  /**
   * Dropping the receiver closes the channel, so a later `send` hands the value
   * back to its caller — and a value already in flight belongs to nobody now,
   * so this releases it.
   */
  protected override onDrop(): void {
    const state = this.#state;
    state.receiver = null;
    closeReceivingHalf(state);
    if (!state.hasValue) return;
    const value = state.value as T;
    state.value = undefined;
    state.hasValue = false;
    dropOwned(value);
  }
}

/**
 * `tokio::sync::oneshot::channel()` — the pair, in Rust's order.
 *
 * @param label — TS-only, like the one Mutex and RwLock take: what to call this
 * channel's two ends in a leak report, so the report names the site rather than
 * just the type.
 */
export function channel<T>(label?: string): [Sender<T>, Receiver<T>] {
  const state: ChannelState<T> = {
    value: undefined,
    hasValue: false,
    senderGone: false,
    receiverClosed: false,
    receiver: null,
    closedWaiters: [],
  };
  const site = label === undefined ? '' : ` on ${label}`;
  const receiver = new Receiver<T>(state, `oneshot::Receiver${site}`);
  state.receiver = receiver;
  return [new Sender<T>(state, `oneshot::Sender${site}`), receiver];
}
