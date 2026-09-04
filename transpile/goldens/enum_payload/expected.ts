// MIRRORS: ankurah/enum_payload/src/input.rs
import { Enum } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

export type NoticeV = {
  Idle: {};
  Text: { _0: string };
  Span: { start: number; end: number };
};

export class Notice extends Enum<NoticeV> {

  isIdle(): boolean {
    return this.match({
      Idle: () => true,
      Text: (v) => false,
      Span: () => false,
    });
  }

  clone(): Notice {
    return new Notice(this.type, { ...this.value });
  }

  debug(): string {
    return this.match({
      Idle: () => 'Idle',
      Text: (v) => `Text(${JSON.stringify(v._0)})`,
      Span: (v) => `Span { start: ${String(v.start)}, end: ${String(v.end)} }`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Idle: (v) => {
        writer.writeVariant(0);
      },
      Text: (v) => {
        writer.writeVariant(1);
        writer.writeString(v._0);
      },
      Span: (v) => {
        writer.writeVariant(2);
        writer.writeU32(v.start);
        writer.writeU32(v.end);
      },
    });
  }

  static decode(reader: BincodeReader): Notice {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        return new Notice('Idle', {});
      }
      case 1: {
        const _0 = reader.readString();
        return new Notice('Text', { _0 });
      }
      case 2: {
        const start = reader.readU32();
        const end = reader.readU32();
        return new Notice('Span', { start, end });
      }
      default: throw new Error(`Unknown Notice variant: ${variant}`);
    }
  }
}

