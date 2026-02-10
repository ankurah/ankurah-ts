// MIRRORS: ankurah/proto/src/sys.rs

import { BincodeReader, BincodeWriter } from './codec';

/**
 * sys::Item enum.
 * Derived serde with `#[serde(other)]` on the Other variant.
 *
 * Variant indices:
 *   0 = SysRoot
 *   1 = Collection { name: String }
 *   2+ = Other (catch-all for unknown variants, per #[serde(other)])
 */
export type Item =
  | { type: 'SysRoot' }
  | { type: 'Collection'; name: string }
  | { type: 'Other' };

export function encodeItem(writer: BincodeWriter, item: Item): void {
  switch (item.type) {
    case 'SysRoot':
      writer.writeVariant(0);
      break;
    case 'Collection':
      writer.writeVariant(1);
      writer.writeString(item.name);
      break;
    case 'Other':
      // Other is never explicitly encoded (it only appears during decoding of unknown variants)
      throw new Error('Cannot encode sys::Item::Other — it is a decode-only catch-all');
  }
}

export function decodeItem(reader: BincodeReader): Item {
  const variant = reader.readVariant();
  switch (variant) {
    case 0:
      return { type: 'SysRoot' };
    case 1: {
      const name = reader.readString();
      return { type: 'Collection', name };
    }
    default:
      // #[serde(other)] — return Other for any unknown variant
      return { type: 'Other' };
  }
}
