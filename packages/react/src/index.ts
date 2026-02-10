// TS-ONLY: React hooks for ankurah (absorbed from ankurah-react-hooks, replaces Rust signals/src/react.rs feature-gated module, see E15)
//
// @ankurah/react — React bindings for ankurah signals.
//
// Provides useObserve() hook and signalObserver() HOC using useSyncExternalStore.
// Factory pattern: createAnkurahReactHooks(bindings) takes TS ReactObserver implementation.
//
// Key exports: useObserve, signalObserver, createAnkurahReactHooks
//
// TODO: Implement React hooks using @ankurah/signals
