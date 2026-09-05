// MIRRORS: ankurah/expected_types/src/input.rs
import { Struct, checkedAdd } from '@ankurah/base';

export class Header extends Struct {
  readonly version: number;
  readonly length: number;

  constructor(version: number, length: number) {
    super();
    this.version = version;
    this.length = length;
  }

  static first(): Header {
    return new Header(1, 512);
  }
}

export function preamble(): Uint8Array {
  return new Uint8Array([1, 2, 3, 4]);
}

export function tag(): Uint8Array {
  const bytes = new Uint8Array([7, 8, 9, 10]);
  return bytes;
}

export function nextLength(header: Header): number {
  const grown = checkedAdd(header.length, 1, 'u16');
  return grown;
}

export function lengths(headers: Header[]): number[] {
  const out = [...headers].map((header) => header.length);
  return out;
}

