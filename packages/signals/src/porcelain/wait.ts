// MIRRORS: ankurah/signals/src/porcelain/wait.rs
import { Arc } from '@ankurah/base';

export interface Wait<T> {
  waitValue(targetValue: T): Promise<void>;
  waitFor(predicate: F): Promise<Output>;
}

export interface WaitResult {
  result(): Output | null;
}

