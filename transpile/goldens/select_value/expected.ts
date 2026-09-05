// MIRRORS: ankurah/select_value/src/input.rs
import { dropOwned, checkedMul, select, Receiver } from '@ankurah/base';

export async function firstOf(left: Receiver<number>, right: Receiver<number>): Promise<number> {
  try {
    try {
      const winner = await (async () => {
        const _v = [
          { tag: '_0', promise: left.recv() },
          { tag: '_1', promise: right.recv() },
        ];
        try {
          const _v1 = await select(_v);
          if (_v1.tag === '_0') {
            return 1;
          } else if (_v1.tag === '_1') {
            return 2;
          } else {
            throw new Error('select: the arbiter answered with a tag no arm wrote');
          }
        } finally {
          for (const _v2 of _v) dropOwned(_v2.promise);
        }
      })();
      return winner * 10;
    } finally {
      right.drop();
    }
  } finally {
    left.drop();
  }
}

export async function doubled(left: Receiver<number>, right: Receiver<number>): Promise<number> {
  try {
    try {
      return twice(await (async () => {
        const _v = [
          { tag: '_0', promise: left.recv() },
          { tag: '_1', promise: right.recv() },
        ];
        try {
          const _v1 = await select(_v);
          if (_v1.tag === '_0') {
            return 3;
          } else if (_v1.tag === '_1') {
            return 4;
          } else {
            throw new Error('select: the arbiter answered with a tag no arm wrote');
          }
        } finally {
          for (const _v2 of _v) dropOwned(_v2.promise);
        }
      })());
    } finally {
      right.drop();
    }
  } finally {
    left.drop();
  }
}

export function twice(n: number): number {
  return checkedMul(n, 2, 'u32');
}

export async function lastWord(left: Receiver<number>, right: Receiver<number>): Promise<number> {
  try {
    try {
      return await (async () => {
        const _v = [
          { tag: '_0', promise: left.recv() },
          { tag: '_1', promise: right.recv() },
        ];
        try {
          const _v1 = await select(_v);
          if (_v1.tag === '_0') {
            return 5;
          } else if (_v1.tag === '_1') {
            return 6;
          } else {
            throw new Error('select: the arbiter answered with a tag no arm wrote');
          }
        } finally {
          for (const _v2 of _v) dropOwned(_v2.promise);
        }
      })();
    } finally {
      right.drop();
    }
  } finally {
    left.drop();
  }
}

export async function answer(left: Receiver<number>, right: Receiver<number>): Promise<number> {
  try {
    try {
      const _v = [
        { tag: '_0', promise: left.recv() },
        { tag: '_1', promise: right.recv() },
      ];
      try {
        const _v1 = await select(_v);
        if (_v1.tag === '_0') {
          return 7;
        } else if (_v1.tag === '_1') {
          return 8;
        }
      } finally {
        for (const _v2 of _v) dropOwned(_v2.promise);
      }
      return 0;
    } finally {
      right.drop();
    }
  } finally {
    left.drop();
  }
}

