// MIRRORS: ankurah/core/src/task.rs

export function spawn<F>(future: F): void {
  wasmBindgenFutures.spawnLocal(future);
}

