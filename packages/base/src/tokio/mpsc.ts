// TS-ONLY: Maps tokio::sync::mpsc to an async queue.
//
// Many senders, one receiver, and the receiver learns that the conversation is
// over by getting `None` — that last part is what the corpus leans on.
// `local-process` runs `while let Some(m) = rx.recv().await` and stops when the
// peer's senders are gone; `client_relay` holds a Sender it never sends on, so
// dropping the relay closes the channel and its retry task falls out of its
// loop. Both work because a receive resolves to None once every sender has been
// dropped and the buffer is drained, so that is the rule this file is built
// around.
//
// Buffered messages belong to the channel. The receiving end releases whatever
// is still queued when it drops, and a send that fails hands its value back to
// the caller, because at that point nobody else can own it.

import { Drop } from '../std/drop.ts';
import { Enum } from '../enum.ts';
import { Result } from '../result.ts';
import { Struct } from '../struct.ts';
import { dropOwned, nonOwning } from '../object.ts';

/** A send that parked for capacity and then went through carries nothing back. */
const SENT: unique symbol = Symbol('ankurah.tokio.mpsc.sent');

/** The receiving half is gone; the value it could not take comes back here. */
export class SendError<T> extends Struct {
  readonly _0: T;

  constructor(value: T) {
    super('mpsc::SendError');
    this._0 = value;
  }

  toString(): string { return 'channel closed'; }
}

type TrySendErrorV<T> = {
  Full: { _0: T };
  Closed: { _0: T };
};

/** Why a non-blocking send could not go through. Either way, the value is here. */
export class TrySendError<T> extends Enum<TrySendErrorV<T>> {
  static Full = <T>(value: T): TrySendError<T> => new TrySendError<T>('Full', { _0: value });
  static Closed = <T>(value: T): TrySendError<T> => new TrySendError<T>('Closed', { _0: value });
}

type TryRecvErrorV = {
  Empty: {};
  Disconnected: {};
};

/** Why a non-blocking receive found nothing. */
export class TryRecvError extends Enum<TryRecvErrorV> {
  static Empty = (): TryRecvError => new TryRecvError('Empty', {});
  static Disconnected = (): TryRecvError => new TryRecvError('Disconnected', {});
}

/**
 * What the two ends share: the queue, the population count on each side, and
 * whoever is parked waiting for room or for a message.
 *
 * It is marked nonOwning so that a cascade reaching it through a channel end
 * steps over it. A channel end does not own the queue — dropping one Sender out
 * of five must not release messages the others put there — so the receiving end
 * releases what is still buffered, and it does that from its own onDrop().
 */
class ChannelCore<T> {
  readonly [nonOwning] = true;
  readonly buffer: T[] = [];
  readonly capacity: number;
  senders = 1;
  /** Set by Receiver.close(), and by dropping the receiver. Sends fail after it. */
  closed = false;
  readonly recvWaiters: Array<(value: T | null) => void> = [];
  readonly sendWaiters: Array<{ value: T; resolve: (outcome: T | typeof SENT) => void }> = [];

  constructor(capacity: number) {
    this.capacity = capacity;
  }

  /** Whether a message can go in right now — handed straight to a parked
   *  receiver, or into a buffer with room left. */
  canAccept(): boolean {
    if (this.closed) return false;
    return this.recvWaiters.length > 0 || this.buffer.length < this.capacity;
  }

  /** Put a message in. A receiver already parked takes it without it ever
   *  being buffered, which is what keeps a zero-length queue moving. */
  deposit(value: T): void {
    const waiting = this.recvWaiters.shift();
    if (waiting !== undefined) {
      waiting(value);
      return;
    }
    this.buffer.push(value);
  }

  /** Give freed capacity to the senders that have been waiting longest. */
  pumpSenders(): void {
    while (this.sendWaiters.length > 0 && this.canAccept()) {
      const waiting = this.sendWaiters.shift() as { value: T; resolve: (outcome: T | typeof SENT) => void };
      this.deposit(waiting.value);
      waiting.resolve(SENT);
    }
  }

  /** The channel closed under the senders parked on it: each gets its value back. */
  failParkedSends(): void {
    const parked = this.sendWaiters.splice(0, this.sendWaiters.length);
    for (const waiting of parked) waiting.resolve(waiting.value);
  }

  /** Tell whoever is waiting for a message that no more are coming. */
  endReceives(): void {
    const parked = this.recvWaiters.splice(0, this.recvWaiters.length);
    for (const wake of parked) wake(null);
  }

  /** Take the oldest message, and let the capacity it freed through. */
  takeBuffered(): T {
    const value = this.buffer.shift() as T;
    this.pumpSenders();
    return value;
  }

  /** No senders left and nothing queued: every future receive is None. */
  get drained(): boolean {
    return this.buffer.length === 0 && (this.senders === 0 || this.closed);
  }
}

/** What both sending halves do, minus the send itself — bounded send waits for
 *  capacity and unbounded send cannot. */
abstract class ChannelSender<T> extends Drop {
  protected readonly core: ChannelCore<T>;

  constructor(core: ChannelCore<T>, label: string) {
    super(label);
    this.core = core;
  }

  /** Whether the receiving half is gone, so a send would fail. */
  is_closed(): boolean {
    this.assertNotDropped();
    return this.core.closed;
  }

  /**
   * The last sender to drop is what tells a parked receiver the conversation
   * is over. Everything still buffered stays receivable until the receiver
   * takes it or drops.
   */
  protected override onDrop(): void {
    this.core.senders--;
    if (this.core.senders > 0) return;
    if (this.core.buffer.length > 0) return;
    this.core.endReceives();
  }

  // A channel end owns nothing: the queue is the channel's, and the receiving
  // end releases it. Spelled out rather than left to the nonOwning marker so
  // that adding a field here is a deliberate decision about who drops it.
  protected override ownedFields(): unknown[] {
    return [];
  }
}

/** What both receiving halves do. */
abstract class ChannelReceiver<T> extends Drop {
  protected readonly core: ChannelCore<T>;

  constructor(core: ChannelCore<T>, label: string) {
    super(label);
    this.core = core;
  }

  /** The next message, or null (Rust's `None`) once every sender is gone and
   *  the buffer is drained. */
  async recv(): Promise<T | null> {
    this.assertNotDropped();
    if (this.core.buffer.length > 0) return this.core.takeBuffered();
    if (this.core.drained) return null;
    return await new Promise<T | null>((resolve) => { this.core.recvWaiters.push(resolve); });
  }

  /**
   * The next message without waiting. `Empty` means a sender is still out
   * there and may yet send; `Disconnected` means none will.
   */
  try_recv(): Result<T, TryRecvError> {
    this.assertNotDropped();
    if (this.core.buffer.length > 0) return Result.Ok(this.core.takeBuffered());
    if (this.core.drained) return Result.Err(TryRecvError.Disconnected());
    return Result.Err(TryRecvError.Empty());
  }

  /**
   * Refuse new messages while leaving the queued ones receivable. Senders
   * parked for capacity get their values back, since no room is ever coming.
   */
  close(): void {
    this.assertNotDropped();
    if (this.core.closed) return;
    this.core.closed = true;
    this.core.failParkedSends();
    if (this.core.buffer.length === 0) this.core.endReceives();
  }

  /**
   * Dropping the receiver closes the channel. Whatever is still queued belongs
   * to nobody now, so it is released here, and every parked send fails with its
   * value handed back to its caller.
   */
  protected override onDrop(): void {
    this.core.closed = true;
    this.core.failParkedSends();
    this.core.endReceives();
    const abandoned = this.core.buffer.splice(0, this.core.buffer.length);
    for (const value of abandoned) dropOwned(value);
  }

  // See ChannelSender.ownedFields: the queue is released above, by hand.
  protected override ownedFields(): unknown[] {
    return [];
  }
}

/** `tokio::sync::mpsc::Sender<T>` — bounded, so sending waits for room. */
export class Sender<T> extends ChannelSender<T> {
  /** @internal — only channel() creates these. */
  constructor(core: ChannelCore<T>, label: string) {
    super(core, label);
  }

  /** Send, waiting for capacity if the buffer is full. */
  async send(value: T): Promise<Result<undefined, SendError<T>>> {
    this.assertNotDropped();
    if (this.core.closed) return Result.Err(new SendError(value));
    if (this.core.canAccept()) {
      this.core.deposit(value);
      return Result.Ok(undefined);
    }
    const outcome = await new Promise<T | typeof SENT>((resolve) => {
      this.core.sendWaiters.push({ value, resolve });
    });
    if (outcome !== SENT) return Result.Err(new SendError(outcome));
    return Result.Ok(undefined);
  }

  /** Send only if there is room right now. */
  try_send(value: T): Result<undefined, TrySendError<T>> {
    this.assertNotDropped();
    if (this.core.closed) return Result.Err(TrySendError.Closed(value));
    if (!this.core.canAccept()) return Result.Err(TrySendError.Full(value));
    this.core.deposit(value);
    return Result.Ok(undefined);
  }

  /** Another sending half. Each clone is its own value and must be dropped;
   *  the channel closes when the last of them goes. */
  clone(): Sender<T> {
    this.assertNotDropped();
    this.core.senders++;
    return new Sender<T>(this.core, this.$label);
  }
}

/** `tokio::sync::mpsc::Receiver<T>` — the bounded receiving half. */
export class Receiver<T> extends ChannelReceiver<T> {
  /** @internal — only channel() creates these. */
  constructor(core: ChannelCore<T>, label: string) {
    super(core, label);
  }
}

/** `tokio::sync::mpsc::UnboundedSender<T>` — no capacity, so sending never waits. */
export class UnboundedSender<T> extends ChannelSender<T> {
  /** @internal — only unbounded_channel() creates these. */
  constructor(core: ChannelCore<T>, label: string) {
    super(core, label);
  }

  /** Send. Synchronous, because an unbounded queue is never full. */
  send(value: T): Result<undefined, SendError<T>> {
    this.assertNotDropped();
    if (this.core.closed) return Result.Err(new SendError(value));
    this.core.deposit(value);
    return Result.Ok(undefined);
  }

  clone(): UnboundedSender<T> {
    this.assertNotDropped();
    this.core.senders++;
    return new UnboundedSender<T>(this.core, this.$label);
  }
}

/** `tokio::sync::mpsc::UnboundedReceiver<T>`. */
export class UnboundedReceiver<T> extends ChannelReceiver<T> {
  /** @internal — only unbounded_channel() creates these. */
  constructor(core: ChannelCore<T>, label: string) {
    super(core, label);
  }
}

/**
 * `tokio::sync::mpsc::channel(buffer)` — the pair, in Rust's order. tokio
 * panics on a buffer of 0, and so does this.
 *
 * @param label — TS-only, like the one Mutex and RwLock take: what to call this
 * channel's ends in a leak report, so the report names the site rather than
 * just the type.
 */
export function channel<T>(buffer: number, label?: string): [Sender<T>, Receiver<T>] {
  if (!Number.isInteger(buffer) || buffer < 1) {
    throw new Error('mpsc bounded channel requires buffer > 0');
  }
  const core = new ChannelCore<T>(buffer);
  const site = label === undefined ? '' : ` on ${label}`;
  return [new Sender<T>(core, `mpsc::Sender${site}`), new Receiver<T>(core, `mpsc::Receiver${site}`)];
}

/**
 * `tokio::sync::mpsc::unbounded_channel()` — the pair, in Rust's order.
 *
 * @param label — TS-only; see channel().
 */
export function unbounded_channel<T>(label?: string): [UnboundedSender<T>, UnboundedReceiver<T>] {
  const core = new ChannelCore<T>(Number.POSITIVE_INFINITY);
  const site = label === undefined ? '' : ` on ${label}`;
  return [
    new UnboundedSender<T>(core, `mpsc::UnboundedSender${site}`),
    new UnboundedReceiver<T>(core, `mpsc::UnboundedReceiver${site}`),
  ];
}
