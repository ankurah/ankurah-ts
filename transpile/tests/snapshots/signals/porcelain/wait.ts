// MIRRORS: ankurah/signals/src/porcelain/wait.rs

export interface Wait<T> {
  waitValue(targetValue: T): Promise<void>;
  waitFor(predicate: F): Promise<Output>;
}

export interface WaitResult {
  result(): Output | null;
}

export function Bool_result(self: boolean): Output | null {
  if (self) {
    return [];
  } else {
    return null;
  }
}

export function Option_result<T>(self: T | null): T | null {
  return self;
}

