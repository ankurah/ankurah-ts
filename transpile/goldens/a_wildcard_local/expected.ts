// MIRRORS: ankurah/a_wildcard_local/src/input.rs
import { Struct, Drop, checkedAdd } from '@ankurah/base';

export class Ticket extends Drop {
  readonly n: bigint;

  constructor(n: bigint) {
    super();
    this.n = n;
  }

  protected override onDrop(): void {

  }
}

export class Desk extends Struct {
  issued: bigint;

  constructor(issued: bigint) {
    super();
    this.issued = issued;
  }

  issue(n: bigint): Ticket {
    this.issued = checkedAdd(this.issued, 1n, 'u64');
    return new Ticket(n);
  }

  async serve(n: bigint): Promise<Ticket> {
    return this.issue(n);
  }
}

export async function serveAndForget(desk: Desk, n: bigint): Promise<void> {
  (await desk.serve(n)).drop();
}

export function issueAndForget(desk: Desk, n: bigint): void {
  desk.issue(n).drop();
}

export function readWithoutMoving(desk: Desk): bigint {
  const held = desk.issue(7n);
  try {
    held;
    return held.n;
  } finally {
    held.drop();
  }
}

