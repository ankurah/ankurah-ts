// MIRRORS: ankurah/ankql/src/ast.rs
import { Struct, Enum, Result, invokeRef, Invocable, JsonError, serde_json, jsonAll, dropOwned, OwnershipFatal, UnsupportedShape, iterLast } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { ParseError } from './error';
import { generateSelectionSql } from './selection/sql';

export class PathExpr extends Struct {
  readonly steps: string[];

  constructor(steps: string[]) {
    super();
    this.steps = steps;
  }

  static simple(name: string): PathExpr {
    return new PathExpr([name]);
  }

  isSimple(): boolean {
    return this.steps.length === 1;
  }

  first(): string {
    return this.steps[0];
  }

  property(): string {
    return (iterLast(this.steps) ?? (() => { throw new Error('PathExpr must have at least one step'); })());
  }

  toString(): string {
    return `${this.steps.join('.')}`;
  }

  equals(other: PathExpr): boolean {
    { if (this.steps.length !== other.steps.length) return false; for (let i = 0; i < this.steps.length; i++) { if (this.steps[i] !== other.steps[i]) return false; } }
    return true;
  }

  clone(): PathExpr {
    return new PathExpr([...this.steps]);
  }

  debug(): string {
    return `PathExpr { steps: ${`[${Array.from(this.steps).map((e) => JSON.stringify(e)).join(', ')}]`} }`;
  }

  encode(writer: BincodeWriter): void {
    writer.writeVec(this.steps, (w, item) => w.writeString(item));
  }

  static decode(reader: BincodeReader): PathExpr {
    const steps = reader.readVec((r) => r.readString());
    return new PathExpr(steps);
  }

  toJSON(): unknown {
    return { 'steps': this.steps };
  }

  static fromJson(value: unknown): Result<PathExpr, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `PathExpr`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('steps' in _o)) {
        return Result.Err(JsonError.custom('missing field `steps`'));
      }
      const _rsteps = ((v: unknown) => (Array.isArray(v) ? jsonAll(v.map((v) => (typeof v === 'string' ? Result.Ok(v as string) : Result.Err(JsonError.custom('expected a string'))))) : Result.Err(JsonError.custom('expected an array'))))(_o['steps']);
      if (_rsteps.isErr()) return Result.Err(_rsteps.unwrapErr());
      const steps = _rsteps.unwrap();
      return Result.Ok(new PathExpr(steps));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export class Selection extends Struct {
  readonly predicate: Predicate;
  readonly orderBy: OrderByItem[] | null;
  readonly limit: bigint | null;

  constructor(predicate: Predicate, orderBy: OrderByItem[] | null, limit: bigint | null) {
    super();
    this.predicate = predicate;
    this.orderBy = orderBy;
    this.limit = limit;
  }

  assumeNull(columns: string[]): Selection {
    const orderBy = (this.orderBy != null ? ((items) => {
      return [...[...items].filter((item) => {
        const colName = item.path.property();
        return !columns.includes(colName);
      })];
    })(this.orderBy!) : null);
    const orderBy_1 = (orderBy != null ? ((v) => (v.isEmpty() ? null : v))(orderBy!) : null);
    return new Selection(this.predicate.assumeNull(columns), orderBy_1, this.limit);
  }

  referencedColumns(): string[] {
    let columns = this.predicate.referencedColumns();
    {
      const _v = this.orderBy;
      if (_v != null) {
        const orderBy = _v;
        for (const item of orderBy) {
          const col = item.path.first();
          if (!columns.includes(col)) {
            columns.push(col);
          }
        }
      }
    }
    return columns;
  }

  toString(): string {
    let _result = '';
    _result += `${this.predicate}`;
    {
      const _v = this.orderBy;
      if (_v != null) {
        const orderBy = _v;
        _result += ' ORDER BY ';
        for (const [i, item] of [...orderBy].entries()) {
          if (i > 0) {
            _result += ', ';
          }
          _result += `${item}`;
        }
      }
    }
    {
      const _v1 = this.limit;
      if (_v1 != null) {
        const limit = _v1;
        _result += ` LIMIT ${limit}`;
      }
    }
    return _result;
  }

  static fromPredicate(predicate: Predicate): Selection {
    return new Selection(predicate, null, null);
  }

  equals(other: Selection): boolean {
    if (!this.predicate.equals(other.predicate)) return false;
    { if ((this.orderBy == null) !== (other.orderBy == null)) return false; if (this.orderBy != null) { { if (this.orderBy!.length !== other.orderBy!.length) return false; for (let i = 0; i < this.orderBy!.length; i++) { if (!this.orderBy![i].equals(other.orderBy![i])) return false; } } } }
    { if ((this.limit == null) !== (other.limit == null)) return false; if (this.limit != null) { if (this.limit! !== other.limit!) return false; } }
    return true;
  }

  clone(): Selection {
    return new Selection(this.predicate.clone(), (this.orderBy != null ? this.orderBy.map(e => e.clone()) : null), this.limit);
  }

  debug(): string {
    return `Selection { predicate: ${this.predicate.debug()}, orderBy: ${(($v) => $v === null ? 'None' : `Some(${`[${Array.from($v).map((e) => e.debug()).join(', ')}]`})`)(this.orderBy)}, limit: ${(($v) => $v === null ? 'None' : `Some(${String($v)})`)(this.limit)} }`;
  }

  encode(writer: BincodeWriter): void {
    this.predicate.encode(writer);
    writer.writeOption(this.orderBy, (w, v) => w.writeVec(v, (w, item) => item.encode(w)));
    writer.writeOption(this.limit, (w, v) => w.writeU64(v));
  }

  static decode(reader: BincodeReader): Selection {
    const predicate = Predicate.decode(reader);
    const orderBy = reader.readOption((r) => r.readVec((r) => OrderByItem.decode(r)));
    const limit = reader.readOption((r) => r.readU64());
    return new Selection(predicate, orderBy, limit);
  }
}

export class OrderByItem extends Struct {
  readonly path: PathExpr;
  readonly direction: OrderDirection;

  constructor(path: PathExpr, direction: OrderDirection) {
    super();
    this.path = path;
    this.direction = direction;
  }

  toString(): string {
    return `${this.path} ${this.direction.match({
      Asc: () => 'ASC',
      Desc: () => 'DESC',
    })}`;
  }

  equals(other: OrderByItem): boolean {
    if (!this.path.equals(other.path)) return false;
    if (!this.direction.equals(other.direction)) return false;
    return true;
  }

  clone(): OrderByItem {
    return new OrderByItem(this.path.clone(), this.direction.clone());
  }

  debug(): string {
    return `OrderByItem { path: ${this.path.debug()}, direction: ${this.direction.debug()} }`;
  }

  encode(writer: BincodeWriter): void {
    this.path.encode(writer);
    this.direction.encode(writer);
  }

  static decode(reader: BincodeReader): OrderByItem {
    const path = PathExpr.decode(reader);
    const direction = OrderDirection.decode(reader);
    return new OrderByItem(path, direction);
  }

  toJSON(): unknown {
    return { 'path': this.path.toJSON(), 'direction': this.direction.toJSON() };
  }

  static fromJson(value: unknown): Result<OrderByItem, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `OrderByItem`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('path' in _o)) {
        return Result.Err(JsonError.custom('missing field `path`'));
      }
      const _rpath = ((v: unknown) => PathExpr.fromJson(v))(_o['path']);
      if (_rpath.isErr()) return Result.Err(_rpath.unwrapErr());
      const path = _rpath.unwrap();
      $built.push(path);
      if (!('direction' in _o)) {
        return Result.Err(JsonError.custom('missing field `direction`'));
      }
      const _rdirection = ((v: unknown) => OrderDirection.fromJson(v))(_o['direction']);
      if (_rdirection.isErr()) return Result.Err(_rdirection.unwrapErr());
      const direction = _rdirection.unwrap();
      $built.push(direction);
      const $out = new OrderByItem(path, direction);
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

export type ExprV = {
  Literal: { _0: Literal };
  Path: { _0: PathExpr };
  Predicate: { _0: Predicate };
  InfixExpr: { left: Expr; operator: InfixOperator; right: Expr };
  ExprList: { _0: Expr[] };
  Placeholder: {};
};

export class Expr extends Enum<ExprV> {

  populateRecursive<I, V, E>(values: I): Result<Expr, ParseError> {
    return this.intoMatch({
      Placeholder: () => {
        const _v = values.next();
        if (_v != null) {
          const value = _v;
          const _r0 = value.tryInto().mapErr((e) => e);
          if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
          Result.Ok(_r0.unwrap())
        } else {
          Result.Err(new ParseError('InvalidPredicate', { _0: 'Not enough values provided for placeholders' }))
        }
      },
      Literal: (v) => {
        const lit = v._0;
        return Result.Ok(new Expr('Literal', { _0: lit }));
      },
      Path: (v) => {
        const path = v._0;
        return Result.Ok(new Expr('Path', { _0: path }));
      },
      Predicate: (v) => {
        const pred = v._0;
        const _r1 = pred.populateRecursive(values);
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        return Result.Ok(new Expr('Predicate', { _0: _r1.unwrap() }));
      },
      InfixExpr: (v) => {
        const left = v.left;
        const operator = v.operator;
        const right = v.right;
        const _r2 = left.populateRecursive(values);
        if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
        const _r3 = right.populateRecursive(values);
        if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
        return Result.Ok(new Expr('InfixExpr', { left: _r2.unwrap(), operator: operator, right: _r3.unwrap() }));
      },
      ExprList: (v) => {
        const exprs = v._0;
        let _moved4 = false;
        try {
          let populatedExprs = [];
          _moved4 = true;
          const _seq6 = exprs;
          let _at7 = 0;
          try {
            while (_at7 < _seq6.length) {
              const expr = _seq6[_at7++];
              const _r5 = expr.populateRecursive(values);
              if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
              populatedExprs.push(_r5.unwrap());
            }
          } finally {
            dropOwned(_seq6.slice(_at7));
          }
          return Result.Ok(new Expr('ExprList', { _0: populatedExprs }));
        } finally {
          if (!_moved4) dropOwned(exprs);
        }
      },
    });
  }

  static fromString(s: string): Expr {
    return new Expr('Literal', { _0: new Literal('String', { _0: s }) });
  }

  static fromI64(i: bigint): Expr {
    return new Expr('Literal', { _0: new Literal('I64', { _0: i }) });
  }

  static fromF64(f: number): Expr {
    return new Expr('Literal', { _0: new Literal('F64', { _0: f }) });
  }

  static fromBool(b: boolean): Expr {
    return new Expr('Literal', { _0: new Literal('Bool', { _0: b }) });
  }

  static fromLiteral(lit: Literal): Expr {
    return new Expr('Literal', { _0: lit });
  }

  static fromVecT<T>(vec: T[]): Expr {
    return new Expr('ExprList', { _0: [...vec].map((item) => item) });
  }

  static fromTN<T>(arr: T[]): Expr {
    return new Expr('ExprList', { _0: [...arr].map((item) => item) });
  }

  static fromT<T>(slice: T[]): Expr {
    return new Expr('ExprList', { _0: [...slice].map((item) => item.clone()) });
  }

  clone(): Expr {
    return this.match({
      Literal: (v) => new Expr('Literal', { _0: v._0.clone() }),
      Path: (v) => new Expr('Path', { _0: v._0.clone() }),
      Predicate: (v) => new Expr('Predicate', { _0: v._0.clone() }),
      InfixExpr: (v) => new Expr('InfixExpr', { left: v.left.clone(), operator: v.operator.clone(), right: v.right.clone() }),
      ExprList: (v) => new Expr('ExprList', { _0: v._0.map(e => e.clone()) }),
      Placeholder: () => new Expr('Placeholder', {}),
    });
  }

  equals(other: Expr): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'Literal': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'Path': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'Predicate': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'InfixExpr': {
        if (!(this.value as any).left.equals((other.value as any).left)) return false;
        if (!(this.value as any).operator.equals((other.value as any).operator)) return false;
        if (!(this.value as any).right.equals((other.value as any).right)) return false;
        break;
      }
      case 'ExprList': {
        { if ((this.value as any)._0.length !== (other.value as any)._0.length) return false; for (let i = 0; i < (this.value as any)._0.length; i++) { if (!(this.value as any)._0[i].equals((other.value as any)._0[i])) return false; } }
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      Literal: (v) => `Literal(${v._0.debug()})`,
      Path: (v) => `Path(${v._0.debug()})`,
      Predicate: (v) => `Predicate(${v._0.debug()})`,
      InfixExpr: (v) => `InfixExpr { left: ${v.left.debug()}, operator: ${v.operator.debug()}, right: ${v.right.debug()} }`,
      ExprList: (v) => `ExprList(${`[${Array.from(v._0).map((e) => e.debug()).join(', ')}]`})`,
      Placeholder: () => 'Placeholder',
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Literal: (v) => {
        writer.writeVariant(0);
        v._0.encode(writer);
      },
      Path: (v) => {
        writer.writeVariant(1);
        v._0.encode(writer);
      },
      Predicate: (v) => {
        writer.writeVariant(2);
        v._0.encode(writer);
      },
      InfixExpr: (v) => {
        writer.writeVariant(3);
        v.left.encode(writer);
        v.operator.encode(writer);
        v.right.encode(writer);
      },
      ExprList: (v) => {
        writer.writeVariant(4);
        writer.writeVec(v._0, (w, item) => item.encode(w));
      },
      Placeholder: (v) => {
        writer.writeVariant(5);
      },
    });
  }

  static decode(reader: BincodeReader): Expr {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const _0 = Literal.decode(reader);
        return new Expr('Literal', { _0 });
      }
      case 1: {
        const _0 = PathExpr.decode(reader);
        return new Expr('Path', { _0 });
      }
      case 2: {
        const _0 = Predicate.decode(reader);
        return new Expr('Predicate', { _0 });
      }
      case 3: {
        const left = Expr.decode(reader);
        const operator = InfixOperator.decode(reader);
        const right = Expr.decode(reader);
        return new Expr('InfixExpr', { left, operator, right });
      }
      case 4: {
        const _0 = reader.readVec((r) => Expr.decode(r));
        return new Expr('ExprList', { _0 });
      }
      case 5: {
        return new Expr('Placeholder', {});
      }
      default: throw new Error(`Unknown Expr variant: ${variant}`);
    }
  }
}

export type LiteralV = {
  I16: { _0: number };
  I32: { _0: number };
  I64: { _0: bigint };
  F64: { _0: number };
  Bool: { _0: boolean };
  String: { _0: string };
  EntityId: { _0: Ulid };
  Object: { _0: Uint8Array };
  Binary: { _0: Uint8Array };
  Json: { _0: unknown };
};

export class Literal extends Enum<LiteralV> {

  clone(): Literal {
    return this.match({
      I16: (v) => new Literal('I16', { _0: v._0 }),
      I32: (v) => new Literal('I32', { _0: v._0 }),
      I64: (v) => new Literal('I64', { _0: v._0 }),
      F64: (v) => new Literal('F64', { _0: v._0 }),
      Bool: (v) => new Literal('Bool', { _0: v._0 }),
      String: (v) => new Literal('String', { _0: v._0 }),
      EntityId: (v) => new Literal('EntityId', { _0: v._0.clone() }),
      Object: (v) => new Literal('Object', { _0: new Uint8Array(v._0) }),
      Binary: (v) => new Literal('Binary', { _0: new Uint8Array(v._0) }),
      Json: (v) => new Literal('Json', { _0: v._0.clone() }),
    });
  }

  equals(other: Literal): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'I16': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'I32': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'I64': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'F64': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'Bool': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'String': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'EntityId': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'Object': {
        { if ((this.value as any)._0.length !== (other.value as any)._0.length) return false; for (let i = 0; i < (this.value as any)._0.length; i++) { if ((this.value as any)._0[i] !== (other.value as any)._0[i]) return false; } }
        break;
      }
      case 'Binary': {
        { if ((this.value as any)._0.length !== (other.value as any)._0.length) return false; for (let i = 0; i < (this.value as any)._0.length; i++) { if ((this.value as any)._0[i] !== (other.value as any)._0[i]) return false; } }
        break;
      }
      case 'Json': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      I16: (v) => `I16(${String(v._0)})`,
      I32: (v) => `I32(${String(v._0)})`,
      I64: (v) => `I64(${String(v._0)})`,
      F64: (v) => `F64(${(($f) => Number.isFinite($f) ? (Number.isInteger($f) ? (Object.is($f, -0) ? '-0.0' : $f.toFixed(1)) : String($f)) : ($f !== $f ? 'NaN' : $f > 0 ? 'inf' : '-inf'))(v._0)})`,
      Bool: (v) => `Bool(${String(v._0)})`,
      String: (v) => `String(${JSON.stringify(v._0)})`,
      EntityId: (v) => `EntityId(${v._0})`,
      Object: (v) => `Object(${`[${Array.from(v._0).map((e) => String(e)).join(', ')}]`})`,
      Binary: (v) => `Binary(${`[${Array.from(v._0).map((e) => String(e)).join(', ')}]`})`,
      Json: (v) => `Json(${v._0})`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      I16: (v) => {
        writer.writeVariant(0);
        writer.writeI16(v._0);
      },
      I32: (v) => {
        writer.writeVariant(1);
        writer.writeI32(v._0);
      },
      I64: (v) => {
        writer.writeVariant(2);
        writer.writeI64(v._0);
      },
      F64: (v) => {
        writer.writeVariant(3);
        writer.writeF64(v._0);
      },
      Bool: (v) => {
        writer.writeVariant(4);
        writer.writeBool(v._0);
      },
      String: (v) => {
        writer.writeVariant(5);
        writer.writeString(v._0);
      },
      EntityId: (v) => {
        writer.writeVariant(6);
        v._0.encode(writer);
      },
      Object: (v) => {
        writer.writeVariant(7);
        writer.writeByteVec(v._0);
      },
      Binary: (v) => {
        writer.writeVariant(8);
        writer.writeByteVec(v._0);
      },
      Json: (v) => {
        writer.writeVariant(9);
        writer.writeByteVec(new TextEncoder().encode(JSON.stringify(v._0)));
      },
    });
  }

  static decode(reader: BincodeReader): Literal {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const _0 = reader.readI16();
        return new Literal('I16', { _0 });
      }
      case 1: {
        const _0 = reader.readI32();
        return new Literal('I32', { _0 });
      }
      case 2: {
        const _0 = reader.readI64();
        return new Literal('I64', { _0 });
      }
      case 3: {
        const _0 = reader.readF64();
        return new Literal('F64', { _0 });
      }
      case 4: {
        const _0 = reader.readBool();
        return new Literal('Bool', { _0 });
      }
      case 5: {
        const _0 = reader.readString();
        return new Literal('String', { _0 });
      }
      case 6: {
        const _0 = Ulid.decode(reader);
        return new Literal('EntityId', { _0 });
      }
      case 7: {
        const _0 = reader.readByteVec();
        return new Literal('Object', { _0 });
      }
      case 8: {
        const _0 = reader.readByteVec();
        return new Literal('Binary', { _0 });
      }
      case 9: {
        const _0 = serde_json.fromSlice(reader.readByteVec()).unwrap();
        return new Literal('Json', { _0 });
      }
      default: throw new Error(`Unknown Literal variant: ${variant}`);
    }
  }
}

export type OrderDirectionV = {
  Asc: {};
  Desc: {};
};

export class OrderDirection extends Enum<OrderDirectionV> {

  clone(): OrderDirection {
    return new OrderDirection(this.type, { ...this.value });
  }

  equals(other: OrderDirection): boolean {
    if (this.type !== other.type) return false;
    return true;
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

  static decode(reader: BincodeReader): OrderDirection {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        return new OrderDirection('Asc', {});
      }
      case 1: {
        return new OrderDirection('Desc', {});
      }
      default: throw new Error(`Unknown OrderDirection variant: ${variant}`);
    }
  }

  toJSON(): unknown {
    return this.match<unknown>({
      Asc: () => 'Asc',
      Desc: () => 'Desc',
    });
  }

  static fromJson(value: unknown): Result<OrderDirection, JsonError> {
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'Asc': return Result.Ok(new OrderDirection('Asc', {}));
          case 'Desc': return Result.Ok(new OrderDirection('Desc', {}));
        }
      }
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `OrderDirection`'));
      }
      const o = value as Record<string, unknown>;
      return Result.Err(JsonError.custom('no variant of `OrderDirection` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export type PredicateV = {
  Comparison: { left: Expr; operator: ComparisonOperator; right: Expr };
  IsNull: { _0: Expr };
  And: { _0: Predicate; _1: Predicate };
  Or: { _0: Predicate; _1: Predicate };
  Not: { _0: Predicate };
  True: {};
  False: {};
  Placeholder: {};
};

export class Predicate extends Enum<PredicateV> {

  walk<T>(accumulator: T, visitor: Invocable<[T, Predicate], T>): T {
    const accumulator_1 = invokeRef(visitor, accumulator, this);
    return this.match({
      And: (v) => {
        const left = v._0;
        const right = v._1;
        const accumulator_2 = left.walk(accumulator_1, visitor);
        return right.walk(accumulator_2, visitor);
      },
      Or: (v) => {
        const left = v._0;
        const right = v._1;
        const accumulator_2 = left.walk(accumulator_1, visitor);
        return right.walk(accumulator_2, visitor);
      },
      Not: (v) => {
        const inner = v._0;
        return inner.walk(accumulator_1, visitor);
      },
      Comparison: () => accumulator_1,
      IsNull: () => accumulator_1,
      True: () => accumulator_1,
      False: () => accumulator_1,
      Placeholder: () => accumulator_1,
    });
  }

  referencedColumns(): string[] {
    return this.walk([], (cols, pred) => {
      pred.match({
        Comparison: (v) => {
          const left = v.left;
          const right = v.right;
          for (const expr of [left, right]) {
            {
              const _v = expr;
              if (_v.is('Path')) {
                const { _0: path } = _v.value;
                const col = path.first().toString();
                if (!cols.includes(col)) {
                  cols.push(col);
                }
              }
            }
          }
        },
        IsNull: (v) => {
          const expr = v._0;
          {
            const _v1 = expr;
            if (_v1.is('Path')) {
              const { _0: path } = _v1.value;
              const col = path.first().toString();
              if (!cols.includes(col)) {
                cols.push(col);
              }
            }
          }
        },
      });
      return cols;
    });
  }

  assumeNull(columns: string[]): Predicate {
    return this.match({
      Comparison: (v) => {
        const left = v.left;
        const operator = v.operator;
        const right = v.right;
        const hasNullPath = (() => {
          const _v1 = [left, right];
          if (((_v1[0].is('Path'))) || ((_v1[1].is('Path')))) {
            const path = (((_v1[0].is('Path')))) ? _v1[0].value._0 : (((_v1[1].is('Path')))) ? _v1[1].value._0 : undefined;
            return columns.includes(path.property());
          } else {
            return false;
          }
        })();
        if (hasNullPath) {
          return operator.match({
            Equal: () => new Predicate('False', {}),
            NotEqual: () => new Predicate('False', {}),
            GreaterThan: () => new Predicate('False', {}),
            GreaterThanOrEqual: () => new Predicate('False', {}),
            LessThan: () => new Predicate('False', {}),
            LessThanOrEqual: () => new Predicate('False', {}),
            In: () => new Predicate('False', {}),
            Between: () => new Predicate('False', {}),
          });
        } else {
          return new Predicate('Comparison', { left: left.clone(), operator: operator.clone(), right: right.clone() });
        }
      },
      IsNull: (v) => {
        const expr = v._0;
        return expr.match({
          Path: (v) => {
            const path = v._0;
            const isNull = columns.includes(path.property());
            if (isNull) {
              return new Predicate('True', {});
            } else {
              return new Predicate('IsNull', { _0: expr.clone() });
            }
          },
          Literal: () => new Predicate('IsNull', { _0: expr.clone() }),
          Predicate: () => new Predicate('IsNull', { _0: expr.clone() }),
          InfixExpr: () => new Predicate('IsNull', { _0: expr.clone() }),
          ExprList: () => new Predicate('IsNull', { _0: expr.clone() }),
          Placeholder: () => new Predicate('IsNull', { _0: expr.clone() }),
        });
      },
      And: (v) => {
        const left = v._0;
        const right = v._1;
        let _moved0 = false;
        const left_1 = left.assumeNull(columns);
        try {
          let _moved1 = false;
          const right_1 = right.assumeNull(columns);
          try {
            const _v2 = [left_1, right_1];
            if (((_v2[0].is('False'))) || ((_v2[1].is('False')))) {
              return new Predicate('False', {});
            } else if ((_v2[0].is('True')) && (_v2[1].is('True'))) {
              return new Predicate('True', {});
            } else if (((_v2[0].is('True'))) || ((_v2[1].is('True')))) {
              const p = (((_v2[0].is('True')))) ? _v2[1] : (((_v2[1].is('True')))) ? _v2[0] : undefined;
              return p.clone();
            } else {
              _moved0 = true;
              _moved1 = true;
              return new Predicate('And', { _0: left_1, _1: right_1 });
            }
          } finally {
            if (!_moved1) right_1.drop();
          }
        } finally {
          if (!_moved0) left_1.drop();
        }
      },
      Or: (v) => {
        const left = v._0;
        const right = v._1;
        let _moved2 = false;
        const left_1 = left.assumeNull(columns);
        try {
          let _moved3 = false;
          const right_1 = right.assumeNull(columns);
          try {
            const _v3 = [left_1, right_1];
            if (((_v3[0].is('True'))) || ((_v3[1].is('True')))) {
              return new Predicate('True', {});
            } else if ((_v3[0].is('False')) && (_v3[1].is('False'))) {
              return new Predicate('False', {});
            } else if (((_v3[0].is('False'))) || ((_v3[1].is('False')))) {
              const p = (((_v3[0].is('False')))) ? _v3[1] : (((_v3[1].is('False')))) ? _v3[0] : undefined;
              return p.clone();
            } else {
              _moved2 = true;
              _moved3 = true;
              return new Predicate('Or', { _0: left_1, _1: right_1 });
            }
          } finally {
            if (!_moved3) right_1.drop();
          }
        } finally {
          if (!_moved2) left_1.drop();
        }
      },
      Not: (v) => {
        const pred = v._0;
        let _moved4 = false;
        const inner = pred.assumeNull(columns);
        try {
          return inner.match({
            True: () => new Predicate('False', {}),
            False: () => new Predicate('True', {}),
            Comparison: () => {
              _moved4 = true;
              return new Predicate('Not', { _0: inner });
            },
            IsNull: () => {
              _moved4 = true;
              return new Predicate('Not', { _0: inner });
            },
            And: () => {
              _moved4 = true;
              return new Predicate('Not', { _0: inner });
            },
            Or: () => {
              _moved4 = true;
              return new Predicate('Not', { _0: inner });
            },
            Not: () => {
              _moved4 = true;
              return new Predicate('Not', { _0: inner });
            },
            Placeholder: () => {
              _moved4 = true;
              return new Predicate('Not', { _0: inner });
            },
          });
        } finally {
          if (!_moved4) inner.drop();
        }
      },
      True: () => new Predicate('True', {}),
      False: () => new Predicate('False', {}),
      Placeholder: () => new Predicate('Placeholder', {}),
    });
  }

  populate<I, V, E>(values: I): Result<Predicate, ParseError> {
    let valuesIter = values.intoIter();
    const _r0 = this.populateRecursive(valuesIter);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const result = _r0.unwrap();
    try {
      if ((valuesIter.next() != null)) {
        return Result.Err(new ParseError('InvalidPredicate', { _0: 'Too many values provided for placeholders' }));
      }
      _moved1 = true;
      return Result.Ok(result);
    } finally {
      if (!_moved1) result.drop();
    }
  }

  populateRecursive<I, V, E>(values: I): Result<Predicate, ParseError> {
    return this.intoMatch({
      Comparison: (v) => {
        const left = v.left;
        const operator = v.operator;
        const right = v.right;
        const _r0 = left.populateRecursive(values);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const _r1 = right.populateRecursive(values);
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        return Result.Ok(new Predicate('Comparison', { left: _r0.unwrap(), operator: operator, right: _r1.unwrap() }));
      },
      And: (v) => {
        const left = v._0;
        const right = v._1;
        let _moved2 = false;
        let _moved3 = false;
        try {
          try {
            _moved2 = true;
            _moved3 = true;
            const _r4 = left.populateRecursive(values);
            if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
            const _r5 = right.populateRecursive(values);
            if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
            return Result.Ok(new Predicate('And', { _0: _r4.unwrap(), _1: _r5.unwrap() }));
          } finally {
            if (!_moved3) dropOwned(right);
          }
        } finally {
          if (!_moved2) dropOwned(left);
        }
      },
      Or: (v) => {
        const left = v._0;
        const right = v._1;
        let _moved6 = false;
        let _moved7 = false;
        try {
          try {
            _moved6 = true;
            _moved7 = true;
            const _r8 = left.populateRecursive(values);
            if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
            const _r9 = right.populateRecursive(values);
            if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
            return Result.Ok(new Predicate('Or', { _0: _r8.unwrap(), _1: _r9.unwrap() }));
          } finally {
            if (!_moved7) dropOwned(right);
          }
        } finally {
          if (!_moved6) dropOwned(left);
        }
      },
      Not: (v) => {
        const pred = v._0;
        const _r10 = pred.populateRecursive(values);
        if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
        return Result.Ok(new Predicate('Not', { _0: _r10.unwrap() }));
      },
      IsNull: (v) => {
        const expr = v._0;
        const _r11 = expr.populateRecursive(values);
        if (_r11.isErr()) return Result.Err(_r11.unwrapErr());
        return Result.Ok(new Predicate('IsNull', { _0: _r11.unwrap() }));
      },
      True: () => Result.Ok(new Predicate('True', {})),
      False: () => Result.Ok(new Predicate('False', {})),
      Placeholder: () => Result.Err(new ParseError('InvalidPredicate', { _0: 'Placeholder must be transformed before population' })),
    });
  }

  toString(): string {
    const _v = generateSelectionSql(this, null);
    if (_v.isOk()) {
      const sql = _v.unwrap();
      return `${sql}`;
    } else {
      const e = _v.unwrapErr();
      try {
        return `SQL Error: ${e}`;
      } finally {
        e.drop();
      }
    }
  }

  clone(): Predicate {
    return this.match({
      Comparison: (v) => new Predicate('Comparison', { left: v.left.clone(), operator: v.operator.clone(), right: v.right.clone() }),
      IsNull: (v) => new Predicate('IsNull', { _0: v._0.clone() }),
      And: (v) => new Predicate('And', { _0: v._0.clone(), _1: v._1.clone() }),
      Or: (v) => new Predicate('Or', { _0: v._0.clone(), _1: v._1.clone() }),
      Not: (v) => new Predicate('Not', { _0: v._0.clone() }),
      True: () => new Predicate('True', {}),
      False: () => new Predicate('False', {}),
      Placeholder: () => new Predicate('Placeholder', {}),
    });
  }

  equals(other: Predicate): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'Comparison': {
        if (!(this.value as any).left.equals((other.value as any).left)) return false;
        if (!(this.value as any).operator.equals((other.value as any).operator)) return false;
        if (!(this.value as any).right.equals((other.value as any).right)) return false;
        break;
      }
      case 'IsNull': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'And': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        if (!(this.value as any)._1.equals((other.value as any)._1)) return false;
        break;
      }
      case 'Or': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        if (!(this.value as any)._1.equals((other.value as any)._1)) return false;
        break;
      }
      case 'Not': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      Comparison: (v) => `Comparison { left: ${v.left.debug()}, operator: ${v.operator.debug()}, right: ${v.right.debug()} }`,
      IsNull: (v) => `IsNull(${v._0.debug()})`,
      And: (v) => `And(${v._0.debug()}, ${v._1.debug()})`,
      Or: (v) => `Or(${v._0.debug()}, ${v._1.debug()})`,
      Not: (v) => `Not(${v._0.debug()})`,
      True: () => 'True',
      False: () => 'False',
      Placeholder: () => 'Placeholder',
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Comparison: (v) => {
        writer.writeVariant(0);
        v.left.encode(writer);
        v.operator.encode(writer);
        v.right.encode(writer);
      },
      IsNull: (v) => {
        writer.writeVariant(1);
        v._0.encode(writer);
      },
      And: (v) => {
        writer.writeVariant(2);
        v._0.encode(writer);
        v._1.encode(writer);
      },
      Or: (v) => {
        writer.writeVariant(3);
        v._0.encode(writer);
        v._1.encode(writer);
      },
      Not: (v) => {
        writer.writeVariant(4);
        v._0.encode(writer);
      },
      True: (v) => {
        writer.writeVariant(5);
      },
      False: (v) => {
        writer.writeVariant(6);
      },
      Placeholder: (v) => {
        writer.writeVariant(7);
      },
    });
  }

  static decode(reader: BincodeReader): Predicate {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const left = Expr.decode(reader);
        const operator = ComparisonOperator.decode(reader);
        const right = Expr.decode(reader);
        return new Predicate('Comparison', { left, operator, right });
      }
      case 1: {
        const _0 = Expr.decode(reader);
        return new Predicate('IsNull', { _0 });
      }
      case 2: {
        const _0 = Predicate.decode(reader);
        const _1 = Predicate.decode(reader);
        return new Predicate('And', { _0, _1 });
      }
      case 3: {
        const _0 = Predicate.decode(reader);
        const _1 = Predicate.decode(reader);
        return new Predicate('Or', { _0, _1 });
      }
      case 4: {
        const _0 = Predicate.decode(reader);
        return new Predicate('Not', { _0 });
      }
      case 5: {
        return new Predicate('True', {});
      }
      case 6: {
        return new Predicate('False', {});
      }
      case 7: {
        return new Predicate('Placeholder', {});
      }
      default: throw new Error(`Unknown Predicate variant: ${variant}`);
    }
  }
}

export type ComparisonOperatorV = {
  Equal: {};
  NotEqual: {};
  GreaterThan: {};
  GreaterThanOrEqual: {};
  LessThan: {};
  LessThanOrEqual: {};
  In: {};
  Between: {};
};

export class ComparisonOperator extends Enum<ComparisonOperatorV> {

  clone(): ComparisonOperator {
    return new ComparisonOperator(this.type, { ...this.value });
  }

  equals(other: ComparisonOperator): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  debug(): string {
    return this.match({
      Equal: () => 'Equal',
      NotEqual: () => 'NotEqual',
      GreaterThan: () => 'GreaterThan',
      GreaterThanOrEqual: () => 'GreaterThanOrEqual',
      LessThan: () => 'LessThan',
      LessThanOrEqual: () => 'LessThanOrEqual',
      In: () => 'In',
      Between: () => 'Between',
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Equal: (v) => {
        writer.writeVariant(0);
      },
      NotEqual: (v) => {
        writer.writeVariant(1);
      },
      GreaterThan: (v) => {
        writer.writeVariant(2);
      },
      GreaterThanOrEqual: (v) => {
        writer.writeVariant(3);
      },
      LessThan: (v) => {
        writer.writeVariant(4);
      },
      LessThanOrEqual: (v) => {
        writer.writeVariant(5);
      },
      In: (v) => {
        writer.writeVariant(6);
      },
      Between: (v) => {
        writer.writeVariant(7);
      },
    });
  }

  static decode(reader: BincodeReader): ComparisonOperator {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        return new ComparisonOperator('Equal', {});
      }
      case 1: {
        return new ComparisonOperator('NotEqual', {});
      }
      case 2: {
        return new ComparisonOperator('GreaterThan', {});
      }
      case 3: {
        return new ComparisonOperator('GreaterThanOrEqual', {});
      }
      case 4: {
        return new ComparisonOperator('LessThan', {});
      }
      case 5: {
        return new ComparisonOperator('LessThanOrEqual', {});
      }
      case 6: {
        return new ComparisonOperator('In', {});
      }
      case 7: {
        return new ComparisonOperator('Between', {});
      }
      default: throw new Error(`Unknown ComparisonOperator variant: ${variant}`);
    }
  }

  toJSON(): unknown {
    return this.match<unknown>({
      Equal: () => 'Equal',
      NotEqual: () => 'NotEqual',
      GreaterThan: () => 'GreaterThan',
      GreaterThanOrEqual: () => 'GreaterThanOrEqual',
      LessThan: () => 'LessThan',
      LessThanOrEqual: () => 'LessThanOrEqual',
      In: () => 'In',
      Between: () => 'Between',
    });
  }

  static fromJson(value: unknown): Result<ComparisonOperator, JsonError> {
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'Equal': return Result.Ok(new ComparisonOperator('Equal', {}));
          case 'NotEqual': return Result.Ok(new ComparisonOperator('NotEqual', {}));
          case 'GreaterThan': return Result.Ok(new ComparisonOperator('GreaterThan', {}));
          case 'GreaterThanOrEqual': return Result.Ok(new ComparisonOperator('GreaterThanOrEqual', {}));
          case 'LessThan': return Result.Ok(new ComparisonOperator('LessThan', {}));
          case 'LessThanOrEqual': return Result.Ok(new ComparisonOperator('LessThanOrEqual', {}));
          case 'In': return Result.Ok(new ComparisonOperator('In', {}));
          case 'Between': return Result.Ok(new ComparisonOperator('Between', {}));
        }
      }
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `ComparisonOperator`'));
      }
      const o = value as Record<string, unknown>;
      return Result.Err(JsonError.custom('no variant of `ComparisonOperator` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export type InfixOperatorV = {
  Add: {};
  Subtract: {};
  Multiply: {};
  Divide: {};
};

export class InfixOperator extends Enum<InfixOperatorV> {

  clone(): InfixOperator {
    return new InfixOperator(this.type, { ...this.value });
  }

  equals(other: InfixOperator): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  debug(): string {
    return this.match({
      Add: () => 'Add',
      Subtract: () => 'Subtract',
      Multiply: () => 'Multiply',
      Divide: () => 'Divide',
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Add: (v) => {
        writer.writeVariant(0);
      },
      Subtract: (v) => {
        writer.writeVariant(1);
      },
      Multiply: (v) => {
        writer.writeVariant(2);
      },
      Divide: (v) => {
        writer.writeVariant(3);
      },
    });
  }

  static decode(reader: BincodeReader): InfixOperator {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        return new InfixOperator('Add', {});
      }
      case 1: {
        return new InfixOperator('Subtract', {});
      }
      case 2: {
        return new InfixOperator('Multiply', {});
      }
      case 3: {
        return new InfixOperator('Divide', {});
      }
      default: throw new Error(`Unknown InfixOperator variant: ${variant}`);
    }
  }

  toJSON(): unknown {
    return this.match<unknown>({
      Add: () => 'Add',
      Subtract: () => 'Subtract',
      Multiply: () => 'Multiply',
      Divide: () => 'Divide',
    });
  }

  static fromJson(value: unknown): Result<InfixOperator, JsonError> {
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'Add': return Result.Ok(new InfixOperator('Add', {}));
          case 'Subtract': return Result.Ok(new InfixOperator('Subtract', {}));
          case 'Multiply': return Result.Ok(new InfixOperator('Multiply', {}));
          case 'Divide': return Result.Ok(new InfixOperator('Divide', {}));
        }
      }
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `InfixOperator`'));
      }
      const o = value as Record<string, unknown>;
      return Result.Err(JsonError.custom('no variant of `InfixOperator` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

