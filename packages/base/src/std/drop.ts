// TS-ONLY: For types with `impl Drop` in Rust — custom cleanup beyond auto-cascade
import { AkObject } from '../object.ts';
import {
  assertNotPoisoned,
  creationStack,
  fatalUseAfterDrop,
  leakRegistry,
} from '../drop_registry.ts';

export abstract class Drop extends AkObject {
  /**
   * Rust's `impl Drop for T`. It runs before this value's fields are dropped, so
   * a body here still sees every field alive — which is the whole point of the
   * hook, and why cleanup that reads a field belongs in it.
   *
   * Subclasses implement this and never override drop(): drop() is AkObject's
   * template, and overriding it is how the order gets broken.
   */
  protected abstract override onDrop(): void;
}

/**
 * Leak tracking and liveness for a class that cannot extend AkObject — the
 * containers, which are generic wrappers rather than ported Rust types.
 */
export class DropGuard {
  #dropped = false;
  readonly #label: string;

  /** @param label — what to call the host in diagnostics; defaults to its class
   *  name. Containers pass the name of the field they stand for, so a leak
   *  report points at the site rather than at the type. */
  constructor(host: object, label?: string) {
    this.#label = label ?? host.constructor.name;
    leakRegistry.register(host, { label: this.#label, creationStack: creationStack() }, host);
  }

  markDropped(host: object): void {
    if (this.#dropped) return;
    this.#dropped = true;
    leakRegistry.unregister(host);
  }

  // Names the host rather than the guard. A DropGuard exists only to diagnose
  // the object holding it, and `this.constructor.name` here would report the
  // useless "DropGuard has already been dropped".
  assertNotDropped(): void {
    assertNotPoisoned();
    if (this.#dropped) fatalUseAfterDrop(this.#label);
  }

  get isDropped(): boolean { return this.#dropped; }
}
