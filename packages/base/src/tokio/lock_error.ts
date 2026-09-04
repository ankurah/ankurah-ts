// TS-ONLY: tokio::sync::TryLockError, shared by the async Mutex and RwLock.

import { Struct } from '../struct.ts';

/** A non-blocking lock attempt found the lock held. */
export class TryLockError extends Struct {
  toString(): string { return 'operation would block'; }
}
