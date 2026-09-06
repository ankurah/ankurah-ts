// MIRRORS: ankurah/hole_beside_a_question/src/input.rs
import { unsupported, checkedAdd, iterFindMap } from '@ankurah/base';

export function pick(xs: number[], ys: number[]): number | null {
  const _r0 = iterFindMap([...xs], (x) => {
    if (x === 99) {
      let it = [...ys.slice()];
      return unsupported('`next` advances an iterator\'s cursor, and the port writes an iterator as the whole sequence with no cursor to advance');
    } else if (x > 3) {
      return x;
    } else {
      return null;
    }
  });
  if (_r0 == null) return null;
  const v = _r0;
  return checkedAdd(v, 1, 'u32');
}

export function wholly(ys: number[]): number | null {
  let it = [...ys];
  const _r0 = unsupported('`next` advances an iterator\'s cursor, and the port writes an iterator as the whole sequence with no cursor to advance');
  const v = _r0;
  return checkedAdd(v, 1, 'u32');
}

