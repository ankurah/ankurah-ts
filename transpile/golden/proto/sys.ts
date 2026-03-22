// MIRRORS: ankurah/proto/src/sys.rs
import { Enum } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

export type ItemV = {
  SysRoot: {};
  Collection: { name: string };
  Other: {};
};

export class Item extends Enum<ItemV> {

  clone(): Item {
    return new Item(this.type, { ...this.value });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      SysRoot: (v) => {
        writer.writeVariant(0);
      },
      Collection: (v) => {
        writer.writeVariant(1);
        writer.writeString(v.name);
      },
      Other: () => {
        throw new Error('Cannot encode Item::Other — it is a decode-only catch-all');
      },
    });
  }

  static decode(reader: BincodeReader): Item {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        return new Item('SysRoot', {});
      }
      case 1: {
        const name = reader.readString();
        return new Item('Collection', { name });
      }
      default: return new Item('Other', {});
    }
  }
}

