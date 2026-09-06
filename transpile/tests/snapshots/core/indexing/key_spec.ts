// MIRRORS: ankurah/core/src/indexing/key_spec.rs
import { Struct, Enum, Result, JsonError, jsonAll, dropOwned, OwnershipFatal, UnsupportedShape, HashMap, HashSet, keyHash } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { ValueType } from '../value/index';
import { PathExpr } from '@ankurah/ankql';

export class KeySpec extends Struct {
  readonly keyparts: IndexKeyPart[];

  constructor(keyparts: IndexKeyPart[]) {
    super();
    this.keyparts = keyparts;
  }

  static new(keyparts: IndexKeyPart[]): KeySpec {
    return new KeySpec(keyparts);
  }

  nameWith(prefix: string, delim: string): string {
    const fields = [...this.keyparts].map((k) => {
      const dir = k.direction.match({
        Asc: () => 'asc',
        Desc: () => 'desc',
      });
      const colName = k.fullPath();
      if ((k.collation != null) || (k.nulls != null)) {
        let extras = [];
        {
          const _v = k.collation;
          if (_v != null) {
            const c = _v;
            extras.push(`collate=${c}`);
          }
        }
        {
          const _v1 = k.nulls;
          if (_v1 != null) {
            const n = _v1;
            extras.push(`nulls=${n.debug()}`.toLowerCase());
          }
        }
        return `${colName} ${dir}(${extras.join(',')})`;
      } else {
        return `${colName} ${dir}`;
      }
    });
    if (prefix.length === 0) {
      return fields.join(delim);
    } else {
      return `${prefix}${delim}${fields.join(delim)}`;
    }
  }

  matches(other: KeySpec): IndexSpecMatch | null {
    if (this.keyparts.length > other.keyparts.length) {
      return null;
    }
    let directMatch = true;
    let inverseMatch = true;
    for (const [selfKeypart, otherKeypart] of [...this.keyparts].zip([...other.keyparts])) {
      if (selfKeypart.column !== otherKeypart.column || selfKeypart.subPath !== otherKeypart.subPath) {
        return null;
      }
      if (selfKeypart.direction !== otherKeypart.direction) {
        directMatch = false;
      }
      if (selfKeypart.direction === otherKeypart.direction) {
        inverseMatch = false;
      }
    }
    if (directMatch) {
      return new IndexSpecMatch('Match', {});
    } else if (inverseMatch) {
      return new IndexSpecMatch('Inverse', {});
    } else {
      return null;
    }
  }

  equals(other: KeySpec): boolean {
    { if (this.keyparts.length !== other.keyparts.length) return false; for (let i = 0; i < this.keyparts.length; i++) { if (!this.keyparts[i].equals(other.keyparts[i])) return false; } }
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [keyHash(this.keyparts)].map((p) => p.length + ':' + p).join('');
  }

  clone(): KeySpec {
    return new KeySpec(this.keyparts.map(e => e.clone()));
  }

  debug(): string {
    return `KeySpec { keyparts: ${`[${Array.from(this.keyparts).map((e) => e.debug()).join(', ')}]`} }`;
  }

  encode(writer: BincodeWriter): void {
    writer.writeVec(this.keyparts, (w, item) => item.encode(w));
  }

  static decode(reader: BincodeReader): KeySpec {
    const keyparts = reader.readVec((r) => IndexKeyPart.decode(r));
    return new KeySpec(keyparts);
  }

  toJSON(): unknown {
    return { 'keyparts': this.keyparts.map((x) => x.toJSON()) };
  }

  static fromJson(value: unknown): Result<KeySpec, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `KeySpec`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('keyparts' in _o)) {
        return Result.Err(JsonError.custom('missing field `keyparts`'));
      }
      const _rkeyparts = ((v: unknown) => (Array.isArray(v) ? jsonAll(v.map((v) => IndexKeyPart.fromJson(v))) : Result.Err(JsonError.custom('expected an array'))))(_o['keyparts']);
      if (_rkeyparts.isErr()) return Result.Err(_rkeyparts.unwrapErr());
      const keyparts = _rkeyparts.unwrap();
      $built.push(keyparts);
      const $out = new KeySpec(keyparts);
      $kept = true;
      return Result.Ok($out);
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    } finally {
      if (!$kept) dropOwned($built);
    }
  }
}

export class IndexKeyPart extends Struct {
  readonly column: string;
  readonly subPath: string[] | null;
  readonly direction: IndexDirection;
  readonly valueType: ValueType;
  readonly nulls: NullsOrder | null;
  readonly collation: string | null;

  constructor(column: string, subPath: string[] | null, direction: IndexDirection, valueType: ValueType, nulls: NullsOrder | null, collation: string | null) {
    super();
    this.column = column;
    this.subPath = subPath;
    this.direction = direction;
    this.valueType = valueType;
    this.nulls = nulls;
    this.collation = collation;
  }

  static asc<S extends Into>(col: S, valueType: ValueType): IndexKeyPart {
    return new IndexKeyPart(col, null, new IndexDirection('Asc', {}), valueType, null, null);
  }

  static desc<S extends Into>(col: S, valueType: ValueType): IndexKeyPart {
    return new IndexKeyPart(col, null, new IndexDirection('Desc', {}), valueType, null, null);
  }

  static fromPath(path: PathExpr, direction: IndexDirection, valueType: ValueType): IndexKeyPart {
    const [column, subPath] = (() => {
      if (path.steps.length === 1) {
        return [path.steps[0], null];
      } else {
        const column = path.steps[0];
        const subPath = path.steps.slice(1).slice();
        return [column, subPath];
      }
    })();
    return new IndexKeyPart(column, subPath, direction, valueType, null, null);
  }

  fullPath(): string {
    if (this.subPath == null) {
      return this.column;
    } else {
      const sub = this.subPath;
      {
        let parts = [this.column];
        parts.push(...sub.slice());
        return parts.join('.');
      }
    }
  }

  static fromFlatPath(path: string, direction: IndexDirection, valueType: ValueType): IndexKeyPart {
    const parts = path.split('.');
    const [column, subPath] = (() => {
      if (parts.length === 1) {
        return [parts[0], null];
      } else {
        const column = parts[0];
        const subPath = [...parts.slice(1)].map((s) => s);
        return [column, subPath];
      }
    })();
    return new IndexKeyPart(column, subPath, direction, valueType, null, null);
  }

  static ascPath(path: string, valueType: ValueType): IndexKeyPart {
    return IndexKeyPart.fromFlatPath(path, new IndexDirection('Asc', {}), valueType);
  }

  static descPath(path: string, valueType: ValueType): IndexKeyPart {
    return IndexKeyPart.fromFlatPath(path, new IndexDirection('Desc', {}), valueType);
  }

  equals(other: IndexKeyPart): boolean {
    if (this.column !== other.column) return false;
    { if ((this.subPath == null) !== (other.subPath == null)) return false; if (this.subPath != null) { { if (this.subPath!.length !== other.subPath!.length) return false; for (let i = 0; i < this.subPath!.length; i++) { if (this.subPath![i] !== other.subPath![i]) return false; } } } }
    if (!this.direction.equals(other.direction)) return false;
    if (!this.valueType.equals(other.valueType)) return false;
    { if ((this.nulls == null) !== (other.nulls == null)) return false; if (this.nulls != null) { if (!this.nulls!.equals(other.nulls!)) return false; } }
    { if ((this.collation == null) !== (other.collation == null)) return false; if (this.collation != null) { if (this.collation! !== other.collation!) return false; } }
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [keyHash(this.column), keyHash(this.subPath), this.direction.hash(), this.valueType.hash(), keyHash(this.nulls), keyHash(this.collation)].map((p) => p.length + ':' + p).join('');
  }

  clone(): IndexKeyPart {
    return new IndexKeyPart(this.column, (this.subPath != null ? [...this.subPath] : null), this.direction.clone(), this.valueType.clone(), this.nulls?.clone() ?? null, this.collation);
  }

  debug(): string {
    return `IndexKeyPart { column: ${JSON.stringify(this.column)}, subPath: ${(($v) => $v === null ? 'None' : `Some(${`[${Array.from($v).map((e) => JSON.stringify(e)).join(', ')}]`})`)(this.subPath)}, direction: ${this.direction.debug()}, valueType: ${this.valueType.debug()}, nulls: ${(($v) => $v === null ? 'None' : `Some(${$v.debug()})`)(this.nulls)}, collation: ${(($v) => $v === null ? 'None' : `Some(${JSON.stringify($v)})`)(this.collation)} }`;
  }

  encode(writer: BincodeWriter): void {
    writer.writeString(this.column);
    writer.writeOption(this.subPath, (w, v) => w.writeVec(v, (w, item) => w.writeString(item)));
    this.direction.encode(writer);
    this.valueType.encode(writer);
    writer.writeOption(this.nulls, (w, v) => v.encode(w));
    writer.writeOption(this.collation, (w, v) => w.writeString(v));
  }

  static decode(reader: BincodeReader): IndexKeyPart {
    const column = reader.readString();
    const subPath = reader.readOption((r) => r.readVec((r) => r.readString()));
    const direction = IndexDirection.decode(reader);
    const valueType = ValueType.decode(reader);
    const nulls = reader.readOption((r) => NullsOrder.decode(r));
    const collation = reader.readOption((r) => r.readString());
    return new IndexKeyPart(column, subPath, direction, valueType, nulls, collation);
  }

  toJSON(): unknown {
    return { 'column': this.column, 'sub_path': this.subPath, 'direction': this.direction.toJSON(), 'value_type': this.valueType.toJSON(), 'nulls': (this.nulls == null ? null : this.nulls.toJSON()), 'collation': this.collation };
  }

  static fromJson(value: unknown): Result<IndexKeyPart, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `IndexKeyPart`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('column' in _o)) {
        return Result.Err(JsonError.custom('missing field `column`'));
      }
      const _rcolumn = ((v: unknown) => (typeof v === 'string' ? Result.Ok(v as string) : Result.Err(JsonError.custom('expected a string'))))(_o['column']);
      if (_rcolumn.isErr()) return Result.Err(_rcolumn.unwrapErr());
      const column = _rcolumn.unwrap();
      const _rsubPath = ((v: unknown) => (v == null ? Result.Ok(null) : ((v: unknown) => (Array.isArray(v) ? jsonAll(v.map((v) => (typeof v === 'string' ? Result.Ok(v as string) : Result.Err(JsonError.custom('expected a string'))))) : Result.Err(JsonError.custom('expected an array'))))(v)))(_o['sub_path']);
      if (_rsubPath.isErr()) return Result.Err(_rsubPath.unwrapErr());
      const subPath = _rsubPath.unwrap();
      if (!('direction' in _o)) {
        return Result.Err(JsonError.custom('missing field `direction`'));
      }
      const _rdirection = ((v: unknown) => IndexDirection.fromJson(v))(_o['direction']);
      if (_rdirection.isErr()) return Result.Err(_rdirection.unwrapErr());
      const direction = _rdirection.unwrap();
      $built.push(direction);
      if (!('value_type' in _o)) {
        return Result.Err(JsonError.custom('missing field `value_type`'));
      }
      const _rvalueType = ((v: unknown) => ValueType.fromJson(v))(_o['value_type']);
      if (_rvalueType.isErr()) return Result.Err(_rvalueType.unwrapErr());
      const valueType = _rvalueType.unwrap();
      $built.push(valueType);
      const _rnulls = ((v: unknown) => (v == null ? Result.Ok(null) : ((v: unknown) => NullsOrder.fromJson(v))(v)))(_o['nulls']);
      if (_rnulls.isErr()) return Result.Err(_rnulls.unwrapErr());
      const nulls = _rnulls.unwrap();
      $built.push(nulls);
      const _rcollation = ((v: unknown) => (v == null ? Result.Ok(null) : ((v: unknown) => (typeof v === 'string' ? Result.Ok(v as string) : Result.Err(JsonError.custom('expected a string'))))(v)))(_o['collation']);
      if (_rcollation.isErr()) return Result.Err(_rcollation.unwrapErr());
      const collation = _rcollation.unwrap();
      const $out = new IndexKeyPart(column, subPath, direction, valueType, nulls, collation);
      $kept = true;
      return Result.Ok($out);
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    } finally {
      if (!$kept) dropOwned($built);
    }
  }
}

export type IndexDirectionV = {
  Asc: {};
  Desc: {};
};

export class IndexDirection extends Enum<IndexDirectionV> {

  isDesc(): boolean {
    return this.is('Desc');
  }

  clone(): IndexDirection {
    return new IndexDirection(this.type, { ...this.value });
  }

  equals(other: IndexDirection): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return String(this.type);
  }

  debug(): string {
    return this.match({
      Asc: () => 'Asc',
      Desc: () => 'Desc',
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Asc: (v) => {
        writer.writeVariant(0);
      },
      Desc: (v) => {
        writer.writeVariant(1);
      },
    });
  }

  static decode(reader: BincodeReader): IndexDirection {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        return new IndexDirection('Asc', {});
      }
      case 1: {
        return new IndexDirection('Desc', {});
      }
      default: throw new Error(`Unknown IndexDirection variant: ${variant}`);
    }
  }

  toJSON(): unknown {
    return this.match<unknown>({
      Asc: () => 'Asc',
      Desc: () => 'Desc',
    });
  }

  static fromJson(value: unknown): Result<IndexDirection, JsonError> {
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'Asc': return Result.Ok(new IndexDirection('Asc', {}));
          case 'Desc': return Result.Ok(new IndexDirection('Desc', {}));
        }
      }
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `IndexDirection`'));
      }
      const o = value as Record<string, unknown>;
      return Result.Err(JsonError.custom('no variant of `IndexDirection` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export type NullsOrderV = {
  First: {};
  Last: {};
};

export class NullsOrder extends Enum<NullsOrderV> {

  clone(): NullsOrder {
    return new NullsOrder(this.type, { ...this.value });
  }

  equals(other: NullsOrder): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return String(this.type);
  }

  debug(): string {
    return this.match({
      First: () => 'First',
      Last: () => 'Last',
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      First: (v) => {
        writer.writeVariant(0);
      },
      Last: (v) => {
        writer.writeVariant(1);
      },
    });
  }

  static decode(reader: BincodeReader): NullsOrder {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        return new NullsOrder('First', {});
      }
      case 1: {
        return new NullsOrder('Last', {});
      }
      default: throw new Error(`Unknown NullsOrder variant: ${variant}`);
    }
  }

  toJSON(): unknown {
    return this.match<unknown>({
      First: () => 'First',
      Last: () => 'Last',
    });
  }

  static fromJson(value: unknown): Result<NullsOrder, JsonError> {
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'First': return Result.Ok(new NullsOrder('First', {}));
          case 'Last': return Result.Ok(new NullsOrder('Last', {}));
        }
      }
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `NullsOrder`'));
      }
      const o = value as Record<string, unknown>;
      return Result.Err(JsonError.custom('no variant of `NullsOrder` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export type IndexSpecMatchV = {
  Match: {};
  Inverse: {};
};

export class IndexSpecMatch extends Enum<IndexSpecMatchV> {

  clone(): IndexSpecMatch {
    return new IndexSpecMatch(this.type, { ...this.value });
  }

  equals(other: IndexSpecMatch): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return String(this.type);
  }

  debug(): string {
    return this.match({
      Match: () => 'Match',
      Inverse: () => 'Inverse',
    });
  }
}

