// MIRRORS: ankurah/format_strings/src/input.rs
import { Struct } from '@ankurah/base';

export class Peer extends Struct {
  readonly id: number;
  readonly name: string;

  constructor(id: number, name: string) {
    super();
    this.id = id;
    this.name = name;
  }

  toString(): string {
    return `${this.name}#${this.id}`;
  }

  debug(): string {
    return `Peer { id: ${String(this.id)}, name: ${JSON.stringify(this.name)} }`;
  }
}

export function greeting(peer: Peer): string {
  return `hello ${peer.name}`;
}

export function positional(a: number, b: number): string {
  return `${a} then ${b}, and ${a} again`;
}

export function named(peer: Peer): string {
  return `${peer.name} is ${peer.id}`;
}

export function captured(name: string): string {
  return `captured ${name}`;
}

export function debugged(peer: Peer): string {
  return `peer ${peer.debug()}`;
}

export function braces(n: number): string {
  return `{${n}}`;
}

export function refuse(n: number): number {
  if (n === 0) {
    throw new Error(`refusing ${n}`);
  }
  return n;
}

