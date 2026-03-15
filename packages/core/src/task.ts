// MIRRORS: ankurah/core/src/task.rs

// Divergence: Rust has two spawn implementations gated by #[cfg]:
//   - Native: Tokio runtime handle with OnceLock fallback
//   - WASM: wasm_bindgen_futures::spawn_local
// TS is single-threaded; spawn simply schedules a microtask [E8].

// Divergence: set_runtime_handle / RUNTIME_HANDLE omitted — no Tokio runtime in JS [E8]

/**
 * Spawn a task (fire-and-forget async execution).
 *
 * Rust: `pub fn spawn<F>(future: F) where F: Future + Send + 'static`
 * Divergence: JS is single-threaded; this schedules the promise on the microtask queue.
 * Unhandled rejections are logged to console.error [E8].
 */
export function spawn(future: Promise<void>): void {
  future.catch((err) => {
    console.error('task::spawn: unhandled rejection:', err);
  });
}
