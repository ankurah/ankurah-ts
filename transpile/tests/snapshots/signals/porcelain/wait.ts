// MIRRORS: ankurah/signals/src/porcelain/wait.rs
import { Arc, OwnedClosure, tokio } from '@ankurah/base';

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

export async function waitValue<T extends Clone, S extends Signal>(self: S, targetValue: T): Promise<void> {
  let _c1;
  const _t0 = self.getReadcell();
  try {
    _c1 = _t0.with((v) => v === targetValue);
  } finally {
    _t0.drop();
  }
  if (_c1) {
    return;
  }
  const [tx, rx] = tokio.sync.mpsc.unbounded_channel();
  const _subscription = self.listen(Arc.new(new OwnedClosure([tx], (_) => {
    const __1 = tx.send([]);
  })));
  try {
    while (true) {
      const _v = await rx.recv();
      if (_v != null) {
        let _c3;
        const _t2 = self.getReadcell();
        try {
          _c3 = _t2.with((v) => v === targetValue);
        } finally {
          _t2.drop();
        }
        if (_c3) {
          break;
        }
      } else {
        {
          break;
        }
      }
    }
  } finally {
    _subscription.drop();
  }
}

export async function waitFor<T extends Clone, S extends Signal, F, R>(self: S, predicate: F): Promise<Output> {
  const _t0 = self.getReadcell();
  try {
    {
      const _v = _t0.with((value) => predicate(value).result());
      if (_v != null) {
        const result = _v;
        return result;
      }
    }
  } finally {
    _t0.drop();
  }
  const [tx, rx] = tokio.sync.mpsc.unbounded_channel();
  const _subscription = self.listen(Arc.new(new OwnedClosure([tx], (_) => {
    const __1 = tx.send([]);
  })));
  try {
    while (true) {
      const _t1 = self.getReadcell();
      try {
        const _v1 = await rx.recv();
        if (_v1 != null) {
          {
            const _v2 = _t1.with((value) => predicate(value).result());
            if (_v2 != null) {
              const result = _v2;
              return result;
            }
          }
        } else {
          {
            break;
          }
        }
      } finally {
        _t1.drop();
      }
    }
    throw new Error('Subscription channel closed unexpectedly - this should not be possible');
  } finally {
    _subscription.drop();
  }
}

