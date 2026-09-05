// MIRRORS: ankurah/core/src/task.rs
import { spawn } from '@ankurah/base';

export function spawn<F>(future: F): void {
  wasmBindgenFutures.spawnLocal(future);
}

