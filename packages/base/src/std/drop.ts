// TS-ONLY: For types with `impl Drop` in Rust — custom cleanup beyond auto-cascade
import { AkObject } from '../object.ts';
import { leakRegistry } from '../drop_registry.ts';

export { disposeSymbol, leakRegistry } from '../drop_registry.ts';

export abstract class Drop extends AkObject {
  abstract override drop(): void;
}

export class DropGuard {
  #dropped = false;

  constructor(host: object) {
    const label = host.constructor.name;
    const creationStack = new Error().stack ?? '';
    leakRegistry.register(host, { label, creationStack, severity: 'warning' }, host);
  }

  markDropped(host: object): void {
    if (this.#dropped) return;
    this.#dropped = true;
    leakRegistry.unregister(host);
  }

  assertNotDropped(): void {
    if (this.#dropped) throw new Error(`${this.constructor.name} has already been dropped`);
  }

  get isDropped(): boolean { return this.#dropped; }
}
