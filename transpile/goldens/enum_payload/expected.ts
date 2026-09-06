// MIRRORS: ankurah/enum_payload/src/input.rs
import { Enum, Result, JsonError, OwnershipFatal, UnsupportedShape, debugString } from '@ankurah/base';
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
      Text: (v) => `Text(${debugString(v._0)})`,
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
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `Notice`'));
      }
      const o = value as Record<string, unknown>;
      if ('Text' in o) {
        const _r_0 = ((v: unknown) => (typeof v === 'string' ? Result.Ok(v as string) : Result.Err(JsonError.custom('expected a string'))))(o['Text']);
        if (_r_0.isErr()) return Result.Err(_r_0.unwrapErr());
        const _0 = _r_0.unwrap();
        
        return Result.Ok(new Notice('Text', { _0: _0 }));
      }
      if ('Span' in o) {
        if (o['Span'] === null || typeof o['Span'] !== 'object' || Array.isArray(o['Span'])) {
          return Result.Err(JsonError.custom('expected an object for `Notice`'));
        }
        const _o = o['Span'] as Record<string, unknown>;
        if (!('start' in _o)) {
          return Result.Err(JsonError.custom('missing field `start`'));
        }
        const _rstart = ((v: unknown) => (typeof v === 'number' && Number.isInteger(v) && v >= 0 && v <= 4294967295 ? Result.Ok(v as number) : Result.Err(JsonError.custom('expected a u32'))))(_o['start']);
        if (_rstart.isErr()) return Result.Err(_rstart.unwrapErr());
        const start = _rstart.unwrap();
        if (!('end' in _o)) {
          return Result.Err(JsonError.custom('missing field `end`'));
        }
        const _rend = ((v: unknown) => (typeof v === 'number' && Number.isInteger(v) && v >= 0 && v <= 4294967295 ? Result.Ok(v as number) : Result.Err(JsonError.custom('expected a u32'))))(_o['end']);
        if (_rend.isErr()) return Result.Err(_rend.unwrapErr());
        const end = _rend.unwrap();
        
        return Result.Ok(new Notice('Span', { start: start, end: end }));
      }
      return Result.Err(JsonError.custom('no variant of `Notice` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

