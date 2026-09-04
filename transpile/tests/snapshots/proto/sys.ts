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

  debug(): string {
    return this.match({
      SysRoot: () => 'SysRoot',
      Collection: (v) => `Collection { name: ${JSON.stringify(v.name)} }`,
      Other: () => 'Other',
    });
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
      Other: (v) => {
        writer.writeVariant(2);
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
      case 2: {
        return new Item('Other', {});
      }
      default: return new Item('Other', {});
    }
  }
}

