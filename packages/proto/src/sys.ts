// MIRRORS: ankurah/proto/src/sys.rs

import { Enum } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

// ─── Item ────────────────────────────────────────────────────────────────────

type ItemV = {
  /// The genesis clock for the system - this serves as the root of the clock tree for all entities in the system
  SysRoot: {};
  Collection: { name: string };
  // #[serde(other)]
  Other: {};
};

export class Item extends Enum<ItemV> {
  // impl Display for Item (Debug in Rust)
  toString(): string {
    return this.match({
      SysRoot: () => 'SysRoot',
      Collection: (v) => `Collection { name: ${v.name} }`,
      Other: () => 'Other',
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      SysRoot: () => {
        writer.writeVariant(0);
      },
      Collection: (v) => {
        writer.writeVariant(1);
        writer.writeString(v.name);
      },
      Other: () => {
        // Other is never explicitly encoded (it only appears during decoding of unknown variants)
        // Divergence: #[serde(other)] is decode-only in Rust; encoding Other throws
        throw new Error('Cannot encode sys::Item::Other — it is a decode-only catch-all');
      },
    });
  }

  static decode(reader: BincodeReader): Item {
    const variant = reader.readVariant();
    switch (variant) {
      case 0:
        return new Item('SysRoot', {});
      case 1: {
        const name = reader.readString();
        return new Item('Collection', { name });
      }
      default:
        // #[serde(other)] — return Other for any unknown variant
        return new Item('Other', {});
    }
  }
}
