// MIRRORS: ankurah/signals/src/observer/callback_observer.rs
//
// STUB: CallbackObserver is deferred per architectural-decisions.md.
// CallbackObserver wraps a callback that is called whenever observed signals
// notify the observer of a change. It uses mark-and-sweep for dependency tracking.
//
// Requires: Observer trait, CurrentObserver (context.ts)
// Will be implemented in Phase 2.
//
// CallbackObserver will:
// - Wrap a callback function
// - Track which signals are accessed during the callback via CurrentObserver
// - Automatically subscribe to those signals
// - Re-trigger the callback when any tracked signal changes
// - Use mark-and-sweep to add/remove subscriptions as dependencies change
