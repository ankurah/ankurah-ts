// TS-ONLY: Base class for all ported Rust types (see E11)

import { disposeSymbol, leakRegistry } from './drop_registry.ts';

export class AkObject {
  #dropped = false;

  constructor() {
    const label = this.constructor.name;
    const creationStack = new Error().stack ?? '';
    leakRegistry.register(this, { label, creationStack, severity: 'fatal' }, this);
  }

  /** Custom cleanup hook. Override in Drop subclass. */
  drop(): void {}

  /** Drop glue — idempotent. Calls drop(), then cascades to all owned fields. */
  [disposeSymbol](): void {
    if (this.#dropped) return;
    this.#dropped = true;
    leakRegistry.unregister(this);
    this.drop();
    for (const key of Object.getOwnPropertyNames(this)) {
      const val = (this as any)[key];
      if (val == null) continue;
      if (typeof val[disposeSymbol] === 'function') {
        val[disposeSymbol]();
      } else if (Array.isArray(val)) {
        for (const item of val) {
          if (item != null && typeof item[disposeSymbol] === 'function') {
            item[disposeSymbol]();
          }
        }
      }
    }
  }

  protected assertNotDropped(): void {
    if (this.#dropped) throw new Error(`${this.constructor.name} has already been dropped`);
  }

  get isDropped(): boolean {
    return this.#dropped;
  }
}
