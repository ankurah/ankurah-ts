// MIRRORS: ankurah/a_cursors_two_axes/src/input.rs
import { Struct, Drop, dropOwned, checkedAdd, countOwned, SeqCursor } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function seen<I extends Iterable<Token>>(walk: SeqCursor<Token>): boolean {
  try {
    return walk.any((t) => {
      try {
        return t.n === 1n;
      } finally {
        t.drop();
      }
    });
  } finally {
    walk.drop();
  }
}

export function every<I extends Iterable<Token>>(walk: SeqCursor<Token>): boolean {
  try {
    return walk.all((t) => {
      try {
        return t.n > 0n;
      } finally {
        t.drop();
      }
    });
  } finally {
    walk.drop();
  }
}

export function third<I extends Iterable<Token>>(walk: SeqCursor<Token>): Token | null {
  try {
    return walk.nth(2);
  } finally {
    walk.drop();
  }
}

export function howManyLeft<I extends Iterable<Token>>(walk: SeqCursor<Token>): number {
  return walk.sizeHint()[0];
}

export function findThenNext<I extends Iterable<Token>>(walk: SeqCursor<Token>): [Token | null, Token | null] {
  try {
    let _moved0 = false;
    const found = walk.find((t) => t.n === 2n);
    try {
      const after = walk.next();
      _moved0 = true;
      return [found, after];
    } finally {
      if (!_moved0) dropOwned(found);
    }
  } finally {
    walk.drop();
  }
}

export function anyThenCount<I extends Iterable<Token>>(walk: SeqCursor<Token>): number {
  let _moved0 = false;
  try {
    const _ = walk.any((t) => {
      try {
        return t.n === 1n;
      } finally {
        t.drop();
      }
    });
    _moved0 = true;
    return countOwned(walk.takeRest());
  } finally {
    if (!_moved0) walk.drop();
  }
}

export function countRefs<I extends Iterable<Token>>(walk: SeqCursor<Token>): number {
  let n = 0;
  for (const _t of walk.takeRest()) {
    n = checkedAdd(n, 1, 'i32');
  }
  return n;
}

export function ignoreRefs<I extends Iterable<Token>>(_walk: SeqCursor<Token>): number {
  try {
    return 0;
  } finally {
    _walk.drop();
  }
}

export function oneRef<I extends Iterable<Token>>(walk: SeqCursor<Token>): boolean {
  try {
    return (walk.next() != null);
  } finally {
    walk.drop();
  }
}

export function borrowedShapes(tokens: Token[]): number {
  const read = countRefs(new SeqCursor([...tokens], 'borrow'));
  const unread = ignoreRefs(new SeqCursor([...tokens], 'borrow'));
  const partial = oneRef(new SeqCursor([...tokens], 'borrow'));
  return checkedAdd(checkedAdd(read, unread, 'usize'), (Number(partial)), 'usize');
}

export function ownedShapes(tokens: Token[]): boolean {
  return seen(new SeqCursor([...tokens]));
}

