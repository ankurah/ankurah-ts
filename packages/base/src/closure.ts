// TS-ONLY: Base class for Rust `move` closures that capture droppable values.
//
// A `move` closure owns what it captures and drops it when the closure itself is
// dropped — a listener holding an Arc keeps that Arc alive for exactly as long
// as the listener lives. A JS closure captures the same values, but its captures
// are invisible: the cascade walks own properties, and there is no property to
// walk. So a callback stored in a struct would keep its captures alive with
// nothing left to release them, and the port would leak every one.
//
// OwnedClosure is where those captures become visible. The emitter lists them
// alongside the function, and from there they are ordinary owned fields.
//
// A closure that captures nothing droppable stays a plain function: there is
// nothing for the cascade to find, and wrapping it would only add a drop the
// emitter then has to place.

import { Drop } from './std/drop.ts';
import { dropOwned } from './object.ts';

/**
 * What a Rust `move` closure becomes when it captures values with drop glue.
 *
 * Called through `call(...)` rather than being invocable directly. A callable
 * object — a function with this prototype grafted on — would let emitted code
 * write `f(x)`, and that call would reach the body without passing a liveness
 * check, which is the one thing this type exists to impose. `call` is also what
 * Rust's `FnMut` invocation looks like at the call site, so the emitted form
 * stays close to the source.
 */
export class OwnedClosure<A extends unknown[] = unknown[], R = void> extends Drop {
  readonly #captures: readonly unknown[] | Record<string, unknown>;
  readonly #fn: (...args: A) => R;
  #capturesMoved = false;

  /**
   * @param captures — the values the Rust closure took by value, as an array or
   * a record. They are this closure's to release; nothing else may drop them.
   * @param fn — the body, which closes over the same values lexically.
   * @param label — TS-only: what to call this closure in a leak report.
   */
  /**
   * Does the body hand one of the captures away?
   *
   * Rust reads such a closure as an `FnOnce`: running it moves the capture into
   * the body, and the closure has nothing left to release. One that only READS
   * its captures still owns them after a call, and whoever consumed the closure
   * releases them by dropping it. `invoke` needs to tell the two apart, and
   * only the emitter — which read the body — can say which this is.
   *
   * `$`-namespaced because it is the emitter's word to the runtime and not part
   * of the Rust surface; no Rust field name can collide with a `$` name.
   */
  readonly $consumesCaptures: boolean;

  constructor(
    captures: readonly unknown[] | Record<string, unknown>,
    fn: (...args: A) => R,
    label?: string,
    consumesCaptures = false,
  ) {
    super(label ?? 'OwnedClosure');
    this.#captures = captures;
    this.#fn = fn;
    this.$consumesCaptures = consumesCaptures;
  }

  /**
   * How many arguments the body declares.
   *
   * For the open-bound dispatcher, which tells `Arc<dyn Fn(T)>` from
   * `Arc<dyn Fn()>` when two impls differ only in the arity of the callable
   * they are written for. Rust picks between them by type; the port has to ask
   * the value, and the function inside is `#private` so nothing outside could.
   *
   * `$`-namespaced because it is a convenience for the emitter and not part of
   * the mechanism — nothing in the runtime reads it, and no Rust field name can
   * collide with a `$` name.
   *
   * It reports what `Function.length` reports, which is the count of parameters
   * before the first default or rest parameter. A closure the emitter wrote
   * from a Rust signature has neither, so for emitted code the two counts are
   * the same number.
   *
   * Borrows — it reads the arity and leaves the closure whole — but it checks
   * liveness first, like every other read in the runtime. A dispatcher only
   * ever asks a value it is about to call, so a dropped closure reaching one
   * says the emitted scope released it too early, and that is worth a fatal
   * rather than a quietly successful branch test.
   */
  get $arity(): number {
    this.assertNotDropped();
    return this.#fn.length;
  }

  /** Invoke the closure. Calling a dropped one reads captures that are gone. */
  call(...args: A): R {
    this.assertNotDropped();
    return this.#fn(...args);
  }

  /**
   * `FnOnce::call_once(self, …)`: invoke the closure and consume it. The
   * captures become the body's, so this closure stops owning them and is left
   * moved — a second call, or a drop, is fatal, and the emitter emits no drop
   * after one.
   *
   * The body closes over the captures lexically, so it already reaches them;
   * what changes here is who releases them. That is now the body's job, exactly
   * as it is in a Rust `FnOnce` whose captures become locals in it.
   */
  callOnce(...args: A): R {
    this.assertNotDropped();
    const captures = this.#captures;
    // Moved before the body runs, so a body that calls back into this closure
    // finds it gone rather than re-entering a value Rust has already consumed.
    this.#capturesMoved = true;
    this.markMoved();
    try {
      return this.#fn(...args);
    } catch (thrown) {
      // Rust's unwind drops the locals the body was given, and after the move
      // nobody else can.
      dropOwned(captures);
      throw thrown;
    }
  }

  protected override onDrop(): void {}

  // The captures are #private, so the cascade cannot see them by walking
  // properties — unless callOnce has already handed them to the body, in which
  // case they are no longer this closure's to release. The function itself owns
  // nothing and is left alone.
  protected override ownedFields(): unknown[] {
    if (this.#capturesMoved) return super.ownedFields();
    return [...super.ownedFields(), this.#captures];
  }
}

/**
 * Call something the port may have wrapped: a plain function, or the
 * `OwnedClosure` the emitter writes when the Rust closure captured values with
 * drop glue.
 *
 * For: whether a closure needed wrapping is a property of what it CAPTURED, and
 * a callee cannot see that. `Result::map_err(f)` is one function in Rust and
 * two shapes here — `f(e)` and `f.callOnce(e)` — and an emitted callee that
 * wrote `f(e)` raised `TypeError: f is not a function` the moment a caller
 * handed it a wrapped one. Three live sites did: core's `node_applier`
 * (`Result.mapErr`), core's `entity` (`tryMutate`, its own file) and
 * storage-sqlite's `engine` (`withConnection`).
 *
 * R10: an argument the engine wraps is ALWAYS invoked through here, and nothing
 * else may call a bound closure parameter directly.
 *
 * `callOnce` rather than `call`, because a bound `FnOnce` parameter is consumed
 * by the call: the captures become the body's, and the closure is left moved so
 * a second call is fatal.
 */
export function invoke<A extends unknown[], R>(f: (...args: A) => R, ...args: A): R;
export function invoke<A extends unknown[], R>(f: OwnedClosure<A, R>, ...args: A): R;
export function invoke<A extends unknown[], R>(f: Invocable<A, R>, ...args: A): R;
export function invoke<A extends unknown[], R>(f: Invocable<A, R>, ...args: A): R {
  if (!(f instanceof OwnedClosure)) return f(...args);
  // The body takes the captures, so they become its and the closure is left
  // moved — exactly Rust's `FnOnce::call_once`.
  if (f.$consumesCaptures) return f.callOnce(...args);
  // The body only read them, so they are still the closure's; the CALL is what
  // consumed the closure, and dropping it here runs their glue where Rust runs
  // it — at the end of the call that took `f` by value.
  try {
    return f.call(...args);
  } finally {
    f.drop();
  }
}

/**
 * The same for an `Fn` or `FnMut` parameter, which Rust takes by REFERENCE:
 * the closure stays its owner's and may be called again, so nothing here
 * releases it.
 */
export function invokeRef<A extends unknown[], R>(f: (...args: A) => R, ...args: A): R;
export function invokeRef<A extends unknown[], R>(f: OwnedClosure<A, R>, ...args: A): R;
export function invokeRef<A extends unknown[], R>(f: Invocable<A, R>, ...args: A): R;
export function invokeRef<A extends unknown[], R>(f: Invocable<A, R>, ...args: A): R {
  return f instanceof OwnedClosure ? f.call(...args) : f(...args);
}

/** The type an emitted parameter takes when either shape may arrive. */
export type Invocable<A extends unknown[] = unknown[], R = void> =
  | OwnedClosure<A, R>
  | ((...args: A) => R);
