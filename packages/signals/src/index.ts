// MIRRORS: ankurah/signals/src/lib.rs
//
// @ankurah/signals -- Reactive signals and subscriptions.
//
// This package provides Broadcast, Signal trait, Subscribe, and ListenerGuard.
// Used by core for reactive updates and by @ankurah/react for UI bindings.
//
// Rust crate: ankurah-signals
// Key types: Broadcast, Signal, Subscribe, ListenerGuard, BroadcastId

// Public modules
export {
  BroadcastId,
  Broadcast,
  BroadcastRef,
  ListenerGuard as BroadcastListenerGuard,
  type BroadcastListener,
  type TListenerGuard,
} from './broadcast.ts';

// context.ts is a stub for Phase 1 (no observer tracking)
// export * from './context.ts';

// observer/ is a stub for Phase 1 (no observer tracking)
// export * from './observer/index.ts';

// Signal types and traits
export {
  Mut,
  Read,
  ListenerGuard,
  type Signal,
  type Get,
  type Peek,
  type With,
  type GetReadCell,
  type Listener,
} from './signal/index.ts';

// Value cells (internal storage, but exported for use by other packages)
export { ValueCell, ReadValueCell } from './value.ts';

// Porcelain (subscribe)
export { type Subscribe, SubscriptionGuard } from './porcelain/index.ts';
