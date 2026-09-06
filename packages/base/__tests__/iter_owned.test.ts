// TS-ONLY: the consuming iterator terminals, and who releases what. F1.
//
// Rust's `into_iter().find(p)` owns the sequence it walks: the element it
// selects becomes the caller's and every other one is dropped, including the
// ones the walk never reached. `max_by_key` drops both losers and hands back
// the winner. `position` moves each element into the closure, which is then the
// only thing that can drop it.
//
// Every test below is written against a droppable element that counts its own
// drops, because the number of drops IS the property: one for every element
// that did not reach the caller, and none for the one that did.

import { describe, expect, test } from 'bun:test';
import { Drop, OwnedClosure } from '../src/index.ts';
import {
  iterFindMapOwned,
  iterFindOwned,
  iterLastOwned,
  iterMaxByKeyOwned,
  iterMaxByOwned,
  iterMinByKeyOwned,
  iterMinByOwned,
  iterPositionOwned,
  iterReduceOwned,
  iterRpositionOwned,
} from '../src/std/iter_owned.ts';

/** An element that says when it was dropped, and refuses a second drop. */
class Token extends Drop {
  static dropped: number[] = [];
  constructor(readonly n: number) {
    super();
  }
  protected override onDrop(): void {
    Token.dropped.push(this.n);
  }
}

function tokens(...ns: number[]): Token[] {
  Token.dropped = [];
  return ns.map((n) => new Token(n));
}

/** What is still alive, by the numbers the elements were built with. */
function dropped(): number[] {
  return [...Token.dropped].sort((a, b) => a - b);
}

describe('a consuming terminal drops every element it does not hand back', () => {
  test('find: the losers before it, and everything after it', () => {
    const xs = tokens(1, 2, 3, 4);
    const found = iterFindOwned(xs, (t) => t.n === 2);
    expect(found?.n).toBe(2);
    expect(dropped()).toEqual([1, 3, 4]);
    // The one the caller was given is still alive, and is the caller's to drop.
    found!.drop();
    expect(dropped()).toEqual([1, 2, 3, 4]);
  });

  test('find with no match drops all of them and answers null', () => {
    const xs = tokens(1, 2, 3);
    expect(iterFindOwned(xs, () => false)).toBe(null);
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('position: the closure owns what it was handed, so nothing here drops it', () => {
    const xs = tokens(1, 2, 3, 4);
    const at = iterPositionOwned(xs, (t) => {
      // Exactly what Rust permits: the closure was given the element by value.
      if (t.n !== 3) t.drop();
      return t.n === 3;
    });
    expect(at).toBe(2);
    // 1 and 2 were dropped by the closure; 4 was never reached and is the
    // iterator's. 3 is the closure's, and it kept it.
    expect(dropped()).toEqual([1, 2, 4]);
  });

  test('rposition walks from the end and leaves the front to be dropped', () => {
    const xs = tokens(1, 2, 3, 4);
    const at = iterRpositionOwned(xs, (t) => {
      if (t.n !== 3) t.drop();
      return t.n === 3;
    });
    expect(at).toBe(2);
    // 4 was dropped by the closure; 1 and 2 were never reached.
    expect(dropped()).toEqual([1, 2, 4]);
  });

  test('find_map answers the first Some and drops what it never reached', () => {
    const xs = tokens(1, 2, 3);
    const got = iterFindMapOwned(xs, (t) => {
      t.drop();
      return t.n === 2 ? `n${t.n}` : null;
    });
    expect(got).toBe('n2');
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('last drops every element the walk passed', () => {
    const xs = tokens(1, 2, 3);
    const got = iterLastOwned(xs);
    expect(got?.n).toBe(3);
    expect(dropped()).toEqual([1, 2]);
    expect(iterLastOwned([])).toBe(null);
  });

  test('reduce hands every element to the closure and drops none of them', () => {
    const xs = tokens(1, 2, 3);
    const got = iterReduceOwned(xs, (a, b) => {
      b.drop();
      return a;
    });
    expect(got?.n).toBe(1);
    expect(dropped()).toEqual([2, 3]);
  });

  test('max_by and min_by drop every loser', () => {
    const xs = tokens(2, 3, 1);
    const most = iterMaxByOwned(xs, (a, b) => a.n - b.n);
    expect(most?.n).toBe(3);
    expect(dropped()).toEqual([1, 2]);

    const ys = tokens(2, 3, 1);
    const least = iterMinByOwned(ys, (a, b) => a.n - b.n);
    expect(least?.n).toBe(1);
    expect(dropped()).toEqual([2, 3]);
  });

  test('max_by_key and min_by_key drop every loser', () => {
    const xs = tokens(2, 3, 1);
    const most = iterMaxByKeyOwned(xs, (t) => t.n);
    expect(most?.n).toBe(3);
    expect(dropped()).toEqual([1, 2]);

    const ys = tokens(2, 3, 1);
    const least = iterMinByKeyOwned(ys, (t) => t.n);
    expect(least?.n).toBe(1);
    expect(dropped()).toEqual([2, 3]);
  });

  test('an empty sequence answers null and drops nothing', () => {
    Token.dropped = [];
    expect(iterMaxByOwned([], (a: Token, b: Token) => a.n - b.n)).toBe(null);
    expect(iterMaxByKeyOwned([], (t: Token) => t.n)).toBe(null);
    expect(iterReduceOwned([], (a: Token) => a)).toBe(null);
    expect(dropped()).toEqual([]);
  });
});

describe('a callback that throws leaves nothing behind', () => {
  test('find: the element the predicate was reading, and everything after it', () => {
    const xs = tokens(1, 2, 3);
    expect(() =>
      iterFindOwned(xs, (t) => {
        if (t.n === 2) throw new Error('no');
        return false;
      }),
    ).toThrow('no');
    // 1 was passed and dropped; 2 was only borrowed by the predicate, so it is
    // still the walk's; 3 was never reached.
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('max_by: the accumulator as well as the rest', () => {
    const xs = tokens(1, 2, 3);
    expect(() =>
      iterMaxByOwned(xs, () => {
        throw new Error('no');
      }),
    ).toThrow('no');
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('max_by_key: the winner so far, its key, and the rest', () => {
    const xs = tokens(1, 2, 3);
    expect(() =>
      iterMaxByKeyOwned(xs, (t) => {
        if (t.n === 2) throw new Error('no');
        return t.n;
      }),
    ).toThrow('no');
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('position: the element is the closure’s, and the rest is the walk’s', () => {
    const xs = tokens(1, 2, 3);
    expect(() =>
      iterPositionOwned(xs, (t) => {
        if (t.n === 2) throw new Error('no');
        t.drop();
        return false;
      }),
    ).toThrow('no');
    // 1 was dropped by the closure, 3 was never reached. 2 belongs to the
    // closure that threw, exactly as Rust's unwind leaves it.
    expect(dropped()).toEqual([1, 3]);
  });
});

describe('a wrapped closure reaches the terminal through invokeRef', () => {
  // F9/R10: a `move` closure over something with drop glue is written as an
  // `OwnedClosure`, which is not callable as a function. Called as one it
  // raised `TypeError: p is not a function` on the first element.
  test('an OwnedClosure predicate is called and then released', () => {
    const captured = new Token(9);
    Token.dropped = [];
    const xs = [new Token(1), new Token(2)];
    const p = new OwnedClosure([captured], (t: Token) => t.n === captured.n - 7);
    const found = iterFindOwned(xs, p);
    expect(found?.n).toBe(2);
    // The closure was released when the terminal ended, which released its
    // captures — Rust drops the `F` the terminal took by value.
    expect(p.isDropped).toBe(true);
    expect(Token.dropped).toContain(9);
    found!.drop();
  });

  test('and so is a plain arrow, which owns nothing', () => {
    const xs = tokens(1, 2);
    expect(iterFindOwned(xs, (t) => t.n === 1)?.n).toBe(1);
  });
});
