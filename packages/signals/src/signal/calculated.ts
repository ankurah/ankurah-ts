// MIRRORS: ankurah/signals/src/signal/calculated.rs
//
// STUB: Calculated<T> is deferred per architectural-decisions.md.
// Calculated signals require Observer/CurrentObserver (auto-dependency tracking)
// which is not implemented in Phase 1.
//
// Calculated<T> will be a derived signal that:
// - Takes a compute function that can access other signals
// - Automatically tracks which signals are accessed during computation
// - Recomputes when any upstream signal changes
// - Notifies downstream observers after recomputation
//
// Will be implemented in Phase 2 when Observer/CurrentObserver are ported.
