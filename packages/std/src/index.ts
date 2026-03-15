// TS-ONLY: Rust std-equivalent provided types for ownership model (see E11)
//
// Organized to mirror Rust's std module structure:
//   std::ops::Drop    -> std/drop.ts   (Drop, DropGuard)
//   std::cell          -> std/cell.ts   (RefCell, Ref, RefMut)
//   std::sync          -> std/sync.ts   (Mutex, MutexGuard)
//
// See port/ownership.md and port/ownership/provided-types.md for API spec.

export { Drop, DropGuard, disposeSymbol, leakRegistry } from './std/drop.ts';
export { RefCell, Ref, RefMut } from './std/cell.ts';
export { Mutex, MutexGuard } from './std/sync.ts';
