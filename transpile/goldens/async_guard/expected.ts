// MIRRORS: ankurah/async_guard/src/input.rs
import { Struct, dropOwned, AsyncMutex, select, Receiver } from '@ankurah/base';

export class Gate extends Struct {
  readonly lock: AsyncMutex<number>;

  constructor(lock: AsyncMutex<number>) {
    super();
    this.lock = lock;
  }

  async bump(): Promise<number> {
    let guard = await this.lock.lock();
    try {
      guard.value += await step();
      return guard.value;
    } finally {
      guard.drop();
    }
  }
}

export async function step(): Promise<number> {
  return 1;
}

export async function race(left: Receiver<number>, right: Receiver<number>): Promise<number> {
  try {
    try {
      let winner = 0;
      await (async () => {
        const _v = [
          { tag: '_0', promise: left.recv() },
          { tag: '_1', promise: right.recv() },
        ];
        try {
          const _v1 = await select(_v);
          if (_v1.tag === '_0') {
            winner = 1;
          } else if (_v1.tag === '_1') {
            winner = 2;
          } else {
            throw new Error('select: the arbiter answered with a tag no arm wrote');
          }
        } finally {
          for (const _v2 of _v) dropOwned(_v2.promise);
        }
      })()
      return winner;
    } finally {
      right.drop();
    }
  } finally {
    left.drop();
  }
}

