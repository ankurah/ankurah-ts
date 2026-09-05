// MIRRORS: ankurah/proto/src/sys.rs
import { Enum, Result, JsonError, OwnershipFatal } from '@ankurah/base';
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

  toJSON(): unknown {
    return this.match<unknown>({
      SysRoot: () => 'SysRoot',
      Collection: (v) => ({ 'Collection': { 'name': v.name } }),
      Other: () => 'Other',
    });
  }

  static fromJson(value: unknown): Result<Item, JsonError> {
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'SysRoot': return Result.Ok(new Item('SysRoot', {}));
          case 'Other': return Result.Ok(new Item('Other', {}));
        }
        return Result.Ok(new Item('Other', {}));
      }
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `Item`'));
      }
      const o = value as Record<string, unknown>;
      if ('Collection' in o) {
        if (o['Collection'] === null || typeof o['Collection'] !== 'object' || Array.isArray(o['Collection'])) {
          return Result.Err(JsonError.custom('expected an object for `Item`'));
        }
        const _o = o['Collection'] as Record<string, unknown>;
        if (!('name' in _o)) {
          return Result.Err(JsonError.custom('missing field `name`'));
        }
        const _rname = ((v: unknown) => (typeof v === 'string' ? Result.Ok(v as string) : Result.Err(JsonError.custom('expected a string'))))(_o['name']);
        if (_rname.isErr()) return Result.Err(_rname.unwrapErr());
        const name = _rname.unwrap();
        
        return Result.Ok(new Item('Collection', { name: name }));
      }
      return Result.Ok(new Item('Other', {}));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

