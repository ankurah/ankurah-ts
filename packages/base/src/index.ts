// TS-ONLY: Rust ownership primitives for the ankurah port (see E11)

export { AkObject, dropOwned, type Slot } from './object.ts';
export { Struct } from './struct.ts';
export { Enum } from './enum.ts';
export { Result } from './result.ts';
export { Drop, DropGuard } from './std/drop.ts';
export {
  disposeSymbol,
  OwnershipFatal,
  setOnFatal,
  setCaptureStacks,
  isPoisoned,
  clearFatalLatch,
  type FatalHandler,
} from './drop_registry.ts';
export { Arc, Weak } from './std/arc.ts';
export { Mutex, MutexGuard } from './std/sync.ts';
export { RwLock, RwLockReadGuard, RwLockWriteGuard } from './std/rwlock.ts';
export { Guard, ReadGuard, WriteGuard } from './std/guard.ts';
export { AsyncMutex, AsyncMutexGuard } from './std/async_mutex.ts';
export { RefCell, Ref, RefMut } from './std/cell.ts';
export { Borrow, BorrowMut } from './std/borrow.ts';
export { ThreadLocal } from './std/thread_local.ts';
