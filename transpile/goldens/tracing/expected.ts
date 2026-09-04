// MIRRORS: ankurah/tracing/src/input.rs
import { Struct, tracing } from '@ankurah/base';

export class Peer extends Struct {
  readonly id: number;

  constructor(id: number) {
    super();
    this.id = id;
  }
}

export function connect(peer: Peer): void {
  tracing.info(`connecting to ${peer.id}`);
  tracing.debug(`peer ${peer.id} state ready`);
}

export function lost(peer: Peer, reason: string): void {
  tracing.warn(`lost ${peer.id}: ${reason}`);
  tracing.error(`giving up on ${peer.id}`);
  tracing.trace('done');
}

