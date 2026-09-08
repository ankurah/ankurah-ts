// MIRRORS: ankurah/a_cursor_through_a_field/src/input.rs
import { Struct, Drop, SeqCursor } from '@ankurah/base';

export class Token extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export class Holder<I extends Iterable<Token>> extends Struct {
  readonly walk: SeqCursor<Token>;

  constructor(walk: SeqCursor<Token>) {
    super();
    this.walk = walk;
  }

  pull(): Token | null {
    return this.walk.next();
  }
}

export class Pair<I extends Iterable<Token>> extends Struct {
  readonly walk: SeqCursor<Token>;
  readonly n: bigint;

  constructor(walk: SeqCursor<Token>, n: bigint) {
    super();
    this.walk = walk;
    this.n = n;
  }
}

export function stored(tokens: Token[]): Token | null {
  let holder = new Holder(new SeqCursor([...tokens]));
  try {
    return holder.pull();
  } finally {
    holder.drop();
  }
}

export function paired(tokens: Token[], n: bigint): bigint {
  const pair = new Pair(new SeqCursor([...tokens]), n);
  try {
    return pair.n;
  } finally {
    pair.drop();
  }
}

