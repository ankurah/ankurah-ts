// MIRRORS: ankurah/byte_targets/src/input.rs
import { checkedMul, wrappingSub } from '@ankurah/base';

export function descending(bytes: Uint8Array): Uint8Array {
  return Uint8Array.from([...bytes].map((b) => wrappingSub((255), b, 'u8')));
}

export function descendingLocal(bytes: Uint8Array): number {
  const out = Uint8Array.from([...bytes].map((b) => wrappingSub((255), b, 'u8')));
  return out.length;
}

export function firstComplement(bytes: Uint8Array): number {
  return Uint8Array.from([...bytes].map((b) => wrappingSub((255), b, 'u8')))[0];
}

export function oneByte(b: number): Uint8Array {
  return new Uint8Array([wrappingSub((255), b, 'u8')]);
}

export function doubled(ns: number[]): number[] {
  return [...ns].map((n) => checkedMul(n, 2, 'u32'));
}

export function copyOf(bytes: Uint8Array): Uint8Array {
  return Uint8Array.from([...[...bytes]]);
}

