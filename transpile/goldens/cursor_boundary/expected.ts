// MIRRORS: ankurah/cursor_boundary/src/input.rs
import { Struct, Drop, iterLastOwned, skipOwned, SeqCursor } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export function first<I extends Iterable<Token>>(walk: SeqCursor<Token>): Token | null {
  try {
    return walk.next();
  } finally {
    walk.drop();
  }
}

export function ignored<I extends Iterable<Token>>(_walk: SeqCursor<Token>): bigint {
  try {
    return 7n;
  } finally {
    _walk.drop();
  }
}

export function lastOf<I extends Iterable<Token>>(walk: SeqCursor<Token>): Token | null {
  return iterLastOwned(walk.takeRest());
}

export function restAfter<I extends Iterable<Token>>(walk: SeqCursor<Token>, n: number): Token[] {
  return skipOwned(walk.takeRest(), n);
}

export function head(tokens: Token[]): Token | null {
  return first(new SeqCursor([...tokens]));
}

export function dropped(tokens: Token[]): bigint {
  return ignored(new SeqCursor([...tokens]));
}

export function tail(tokens: Token[]): Token | null {
  return lastOf(new SeqCursor([...tokens]));
}

export function allBut(tokens: Token[], n: number): Token[] {
  return restAfter(new SeqCursor([...tokens]), n);
}

export function headOf<J extends Iterable<Token>>(walk: SeqCursor<Token>): Token | null {
  return first(walk);
}

