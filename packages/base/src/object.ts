// TS-ONLY: Base class for all ported Rust types (see E11)

import { disposeSymbol, leakRegistry } from './drop_registry.ts';

export class AkObject {
  #dropped = false;

  constructor() {
    const label = this.constructor.name;
    const creationStack = new Error().stack ?? '';
    leakRegistry.register(this, { label, creationStack, severity: 'fatal' }, this);
  }

  get isDropped(): boolean { return this.#dropped; }

  /** Dispose this object. Override in Drop subclasses for custom cleanup, call super.drop().
   *  The transpiler generates .drop() calls for scope cleanup. */
  drop(): void {
    if (this.#dropped) return;
    this.#dropped = true;
    leakRegistry.unregister(this);
    // Cascade disposal to all owned fields
    for (const key of Object.getOwnPropertyNames(this)) {
      const val = (this as any)[key];
      if (val == null) continue;
      if (typeof val.drop === 'function') {
        val.drop();
      } else if (Array.isArray(val)) {
        for (const item of val) {
          if (item != null && typeof item.drop === 'function') {
            item.drop();
          }
        }
      }
    }
  }

  /** Symbol.dispose — delegates to drop() for using/with compatibility */
  [disposeSymbol](): void {
    this.drop();
  }

  protected assertNotDropped(): void {
    if (this.#dropped) throw new Error(`${this.constructor.name} has already been dropped`);
  }

  get isDropped(): boolean {
    return this.#dropped;
  }
}
