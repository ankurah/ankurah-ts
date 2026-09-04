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
  constructor(
    captures: readonly unknown[] | Record<string, unknown>,
    fn: (...args: A) => R,
    label?: string,
  ) {
    super(label ?? 'OwnedClosure');
    this.#captures = captures;
    this.#fn = fn;
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
