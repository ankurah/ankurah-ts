// TS-ONLY: Maps tokio::spawn and tokio::task::JoinHandle to a promise plus a
// handle over it.
//
// A spawned task runs on its own from here; the JoinHandle is the only way back
// to it. `core/src/task.rs` throws the handle away and never joins, while
// `local-process` keeps two of them in a struct and aborts both when the
// connection drops — so a handle has to be droppable as a field, abortable, and
// awaitable, and this file provides those three.
//
// Cancellation is where JavaScript stops matching Rust. `abort()` in tokio stops
// the task at its next await point; here nothing can stop a running async
// function. The handle resolves to a cancelled JoinError and the task's eventual
// output is dropped, but the body keeps running to its end. Anything it does on
// the way — a write, a message — still happens.

import { Result } from '../result.ts';
import { Struct } from '../struct.ts';
import { diagnostic, reportAsyncFatal } from '../drop_registry.ts';
import { discardValue } from './discard.ts';
import { NamedFuture } from './future.ts';

/** Why a join produced no value: the task was aborted, or it panicked. */
export class JoinError extends Struct {
  readonly #cancelled: boolean;
  #panic: unknown;

  private constructor(cancelled: boolean, panic: unknown) {
    super('JoinError');
    this.#cancelled = cancelled;
    this.#panic = panic;
  }

  static cancelled(): JoinError { return new JoinError(true, undefined); }
  static panicked(thrown: unknown): JoinError { return new JoinError(false, thrown); }

  // `&self` in Rust, so these borrow — and reading one that into_panic() has
  // already taken the payload out of is a use of a moved value.
  is_cancelled(): boolean {
    this.assertNotDropped();
    return this.#cancelled;
  }

  is_panic(): boolean {
    this.assertNotDropped();
    return !this.#cancelled;
  }

  /**
   * What the task threw. Takes `self` in Rust and hands the payload to the
   * caller, so it consumes this error; a JoinError that reports a cancellation
   * carries no payload and panics here, as tokio's does.
   */
  into_panic(): unknown {
    this.assertNotDropped();
    if (this.#cancelled) {
      // Rust panics, and the unwind drops the error it was called on.
      this.markMoved();
      throw new Error('called into_panic() on a JoinError that reports a cancellation');
    }
    return this.#takePanic();
  }

  /**
   * The panic payload, or this error back again when the task was cancelled.
   * The Err arm hands the same value back rather than moving it, because in
   * Rust that arm returns `self` — so the caller still owns it and still drops
   * it.
   */
  try_into_panic(): Result<unknown, JoinError> {
    this.assertNotDropped();
    if (this.#cancelled) return Result.Err(this);
    return Result.Ok(this.#takePanic());
  }

  toString(): string {
    return this.#cancelled ? 'task was cancelled' : `task panicked: ${String(this.#panic)}`;
  }

  /**
   * The payload is a #private field, so the cascade cannot see it by walking
   * properties: a thrown value with drop glue would otherwise never be released
   * when the error is dropped.
   */
  protected override ownedFields(): unknown[] {
    return [...super.ownedFields(), this.#panic];
  }

  #takePanic(): unknown {
    const payload = this.#panic;
    this.#panic = undefined;
    this.markMoved();
    return payload;
  }
}

/**
 * A handle on a spawned task. Awaiting it consumes it and yields
 * `Result<T, JoinError>`; dropping it detaches the task, which keeps running
 * with its output discarded.
 */
export class JoinHandle<T> extends NamedFuture<Result<T, JoinError>> {
  #result: Result<T, JoinError> | null = null;

  /** @internal — only spawn() creates these. */
  constructor(task: Promise<T>, label?: string) {
    super(label === undefined ? 'JoinHandle' : `JoinHandle for ${label}`);
    // fire-and-forget: this is the whole point of spawn — the task runs on its
    // own and reports back through this handle.
    task.then(
      (value) => { this.#completed(value); },
      (thrown: unknown) => { this.#failed(thrown); },
    );
  }

  /**
   * Give up on the task. The handle resolves to a cancelled JoinError at once;
   * the task body cannot be stopped and runs to its end, and whatever it
   * produces is discarded.
   */
  abort(): void {
    this.assertNotDropped();
    if (this.isSettled()) return;
    this.#result = Result.Err(JoinError.cancelled());
    this.settle();
  }

  /**
   * Whether this handle has an answer. tokio reports on the task; this reports
   * on the handle, so it is true straight after abort() while the task body is
   * still running.
   */
  is_finished(): boolean {
    this.assertNotDropped();
    return this.isSettled();
  }

  #completed(value: T): void {
    if (this.isSettled() || this.isDropped) {
      // Aborted or detached: nobody is left to own what the task produced.
      discardValue(value);
      return;
    }
    this.#result = Result.Ok(value);
    this.settle();
  }

  #failed(thrown: unknown): void {
    // An ownership fatal is not a task failure and must never become one: a
    // JoinError is a Rust error value the emitted code is entitled to handle,
    // and the runtime has just found something Rust would not have compiled.
    // It goes back to the host on its own, and this handle never settles.
    if (reportAsyncFatal(thrown)) return;
    if (this.isSettled() || this.isDropped) {
      // tokio forwards a panic to the JoinHandle and drops it when there is no
      // handle left. There is no handle left, so this goes to the host's
      // diagnostic handler, which is silent unless setOnDiagnostic() wired it.
      diagnostic('ankurah: a spawned task failed and nothing is joined to it.', thrown);
      return;
    }
    this.#result = Result.Err(JoinError.panicked(thrown));
    this.settle();
  }

  protected override takeOutput(): Result<T, JoinError> {
    const result = this.#result as Result<T, JoinError>;
    this.#result = null;
    return result;
  }

  /**
   * Dropping a JoinHandle detaches the task — tokio keeps running it and
   * discards its output, and so does this. A result that already arrived and
   * was never taken is released here.
   */
  protected override onDrop(): void {
    const untaken = this.#result;
    this.#result = null;
    untaken?.drop();
  }
}

/** What spawn accepts: a running promise, or a function it should call. */
export type Spawnable<T> = PromiseLike<T> | (() => T | PromiseLike<T>);

function start<T>(task: Spawnable<T>): Promise<T> {
  // tokio does not poll a spawned future on the thread that spawned it, and
  // code that spawns while holding a lock depends on that. So a function is
  // called from a fresh turn, never on the caller's stack — and a synchronous
  // throw from it becomes a rejection this handle can report as a panic.
  if (typeof task === 'function') return Promise.resolve().then(() => task());
  return Promise.resolve(task);
}

/**
 * `tokio::spawn` — run the future to completion, independently of the caller.
 *
 * @param label — TS-only, like the one Mutex and RwLock take: what to call the
 * returned handle in a leak report, so the report names the task rather than
 * just the type.
 */
export function spawn<T>(task: Spawnable<T>, label?: string): JoinHandle<T> {
  return new JoinHandle<T>(start(task), label);
}

/**
 * `tokio::task::spawn_local`. Identical to spawn here: there is one thread, so
 * the distinction tokio draws — a task that need not be Send, pinned to the
 * current thread — has nothing to distinguish.
 */
export function spawn_local<T>(task: Spawnable<T>, label?: string): JoinHandle<T> {
  return spawn(task, label);
}

/**
 * `tokio::task::yield_now` — hand the turn back so other work can run. A
 * macrotask rather than a microtask, so timers and I/O get their turn too,
 * which is what yielding to tokio's scheduler buys.
 */
export function yield_now(): Promise<void> {
  return new Promise<void>((resolve) => { setTimeout(resolve, 0); });
}
