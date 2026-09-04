// TS-ONLY: Maps anyhow::Error to a chain of messages plus the values behind it.
//
// `anyhow::Error` is what 29 `?` sites in core convert into: a boxed error that
// anything implementing std::error::Error can become, that carries a chain of
// context strings added on the way up, and that can be asked afterwards what it
// originally was. All three of those are what this file provides.
//
// It is not a Result. `anyhow::Result<T>` is `Result<T, anyhow::Error>`, so a
// function returning one returns `Result<T, AnyhowError>` here — the error type
// changes, the Result machinery does not.

import { Struct } from './struct.ts';

/**
 * One step of the chain: the message at this level, and the error value it was
 * built from, if it was built from one rather than from a bare message.
 *
 * A plain record rather than a class, so the cascade walks it and releases the
 * error value it holds. The chain is the AnyhowError's, and nothing else's.
 */
interface Link {
  readonly message: string;
  readonly error: unknown;
}

/**
 * What to call an arbitrary thrown value in a message. An Error's `message` is
 * what Rust's Display would have printed; everything else falls back to its own
 * toString, then to JSON. Nothing here may throw: it runs while building an
 * error, and a second fault would bury the first.
 */
function renderError(value: unknown): string {
  if (value === null || value === undefined) return String(value);
  try {
    if (value instanceof Error) return value.message;
    const own = (value as { toString?: unknown }).toString;
    if (typeof own === 'function' && own !== Object.prototype.toString) return String(value);
    return JSON.stringify(value) ?? String(value);
  } catch {
    return '(unrenderable)';
  }
}

/**
 * `anyhow::Error`.
 *
 * It owns the error values in its chain, so dropping it releases them — which
 * is why it is a tracked value and not a bare string. `downcast_ref` and
 * `root_cause` hand back references into that chain: they borrow, exactly as
 * their Rust counterparts do, and the caller must not drop what they return.
 */
export class AnyhowError extends Struct {
  #links: readonly Link[];
  #linksMoved = false;

  private constructor(links: readonly Link[]) {
    super('anyhow::Error');
    this.#links = links;
  }

  /**
   * Rust's blanket `impl<E: std::error::Error> From<E> for anyhow::Error`,
   * which is what turns `?` on any error into this one. An AnyhowError is
   * already one and comes back untouched, the way `?` on an `anyhow::Error` is
   * the identity rather than a second box.
   */
  static from(error: unknown): AnyhowError {
    if (error instanceof AnyhowError) return error;
    return new AnyhowError([{ message: renderError(error), error }]);
  }

  /** `anyhow::Error::msg` — an error that is only a message. */
  static msg(text: string): AnyhowError {
    return new AnyhowError([{ message: text, error: undefined }]);
  }

  /**
   * `anyhow::Context::context` — a new error that says what was being done,
   * with this one as its cause.
   *
   * It takes `self` in Rust, so it consumes this error: the chain moves into
   * the error it returns, this one is left moved, and the emitter emits no drop
   * after it. Moving the chain rather than nesting the object is what keeps a
   * single owner for the error values in it.
   */
  context(message: string): AnyhowError {
    this.assertNotDropped();
    const chain = this.#links;
    this.#linksMoved = true;
    this.markMoved();
    return new AnyhowError([{ message, error: undefined }, ...chain]);
  }

  /**
   * anyhow's `Display`: the outermost message alone.
   *
   * Deliberately does not assert liveness, for the same reason `Enum.toString`
   * does not — rendering a value is what a panic message and a debugger do, and
   * both run precisely when something has already gone wrong.
   */
  toString(): string {
    if (this.isMoved) return 'anyhow::Error (moved)';
    if (this.isDropped) return 'anyhow::Error (dropped)';
    return this.#links[0]?.message ?? '';
  }

  /** anyhow's alternate Display, `{:#}`: every message in the chain, outermost first. */
  toStringAlternate(): string {
    if (this.isMoved || this.isDropped) return this.toString();
    return this.#links.map((link) => link.message).join(': ');
  }

  /**
   * `downcast_ref::<E>()` — the error this chain was built from, if one of its
   * links holds a value of that class. Borrows: the chain still owns what comes
   * back, and the caller must not drop it.
   */
  downcast_ref<T>(ctor: abstract new (...args: never[]) => T): T | null {
    this.assertNotDropped();
    for (const link of this.#links) {
      if (link.error instanceof ctor) return link.error as T;
    }
    return null;
  }

  /**
   * `root_cause()` — the innermost error value in the chain. Borrows.
   *
   * DELIBERATE DIFFERENCE: anyhow returns the `Error` itself when nothing
   * underlies it, because there it is always a `&dyn Error`. A chain that
   * bottoms out in `msg()` has no underlying value to return, so this returns
   * null — Rust's `Option<T>` shape, and the caller reads `toString()` for the
   * message instead.
   */
  root_cause(): unknown {
    this.assertNotDropped();
    for (let at = this.#links.length - 1; at >= 0; at--) {
      const link = this.#links[at] as Link;
      if (link.error !== undefined) return link.error;
    }
    return null;
  }

  // The chain is #private, so the cascade cannot see it by walking properties —
  // unless context() has already moved it into the error it returned, in which
  // case it is no longer this one's to release.
  protected override ownedFields(): unknown[] {
    if (this.#linksMoved) return super.ownedFields();
    return [...super.ownedFields(), this.#links];
  }
}

// `use anyhow::Error;` names it `Error`, and `import * as anyhow` makes that
// `anyhow.Error`. An export alias creates no local binding, so `Error` inside
// this module is still the global one.
export { AnyhowError as Error };
