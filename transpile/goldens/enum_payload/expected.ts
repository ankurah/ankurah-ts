// MIRRORS: ankurah/enum_payload/src/input.rs
import { Enum, Result, JsonError } from '@ankurah/base';
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

  toJSON(): unknown {
    return this.match<unknown>({
      Idle: () => 'Idle',
      Text: (v) => ({ 'Text': v._0 }),
      Span: (v) => ({ 'Span': { 'start': v.start, 'end': v.end } }),
    });
  }

  static fromJson(value: unknown): Result<Notice, JsonError> {
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'Idle': return Result.Ok(new Notice('Idle', {}));
        }
      }
      const o = value as Record<string, unknown>;
      if ('Text' in o) {
        const p = o['Text'];
        return Result.Ok(new Notice('Text', { _0: ((v: unknown) => v as string)(p) }));
      }
      if ('Span' in o) {
        const p = o['Span'];
        return Result.Ok(new Notice('Span', { start: ((v: unknown) => v as number)((p as Record<string, unknown>)['start']), end: ((v: unknown) => v as number)((p as Record<string, unknown>)['end']) }));
      }
      return Result.Err(JsonError.custom('no variant of `Notice` matches this JSON'));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
  }
}

