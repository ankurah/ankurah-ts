// MIRRORS: ankurah/option_result_fields/src/input.rs
import { Struct, Enum, Result, JsonError } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';

export class Slot extends Struct {
  readonly name: string | null;
  readonly count: number | null;

  constructor(name: string | null, count: number | null) {
    super();
    this.name = name;
    this.count = count;
  }

  requireCount(): Result<number, SlotError> {
    if (this.count != null) {
      const n = this.count;
      return Result.Ok(n);
    } else {
      return Result.Err(new SlotError('Missing', {}));
    }
  }

  clone(): Slot {
    return new Slot(this.name, this.count);
  }

  debug(): string {
    return `Slot { name: ${(($v) => $v === null ? 'None' : `Some(${JSON.stringify($v)})`)(this.name)}, count: ${(($v) => $v === null ? 'None' : `Some(${String($v)})`)(this.count)} }`;
  }

  encode(writer: BincodeWriter): void {
    writer.writeOption(this.name, (w, v) => w.writeString(v));
    writer.writeOption(this.count, (w, v) => w.writeU32(v));
  }

  static decode(reader: BincodeReader): Slot {
    const name = reader.readOption((r) => r.readString());
    const count = reader.readOption((r) => r.readU32());
    return new Slot(name, count);
  }

  toJSON(): unknown {
    return {
      'name': this.name,
      'count': this.count,
    };
  }

  static fromJson(value: unknown): Result<Slot, JsonError> {
    try {
      const o = value as Record<string, unknown>;
      const name = ((v: unknown) => v as string | null)(o['name']);
      const count = ((v: unknown) => v as number | null)(o['count']);
      return Result.Ok(new Slot(name, count));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export type SlotErrorV = {
  Missing: {};
};

export class SlotError extends Enum<SlotErrorV> {

  clone(): SlotError {
    return new SlotError(this.type, { ...this.value });
  }

  equals(other: SlotError): boolean {
    return true;
  }

  debug(): string {
    return this.match({
      Missing: () => 'Missing',
    });
  }
}

