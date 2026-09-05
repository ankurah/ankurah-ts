// MIRRORS: ankurah/core/src/reactor/property_path.rs
import { Struct, Result, HashMap, HashSet, keyHash } from '@ankurah/base';
import { Json } from '../property/value/json';
import { AbstractEntity } from '../reactor';
import { Value } from '../value/index';
import { PathExpr } from '@ankurah/ankql';

export class PropertyPath extends Struct {
  root: string;
  subPath: string[];

  constructor(root: string, subPath: string[]) {
    super();
    this.root = root;
    this.subPath = subPath;
  }

  static fromPath(path: PathExpr): PropertyPath {
    const steps = path.steps;
    return new PropertyPath(steps[0].clone(), steps.slice(1).toVec());
  }

  root(): string {
    return this.root;
  }

  isSimple(): boolean {
    return this.subPath.length === 0;
  }

  extractValue<E extends AbstractEntity>(entity: E): Value | null {
    const _r0 = E.value(entity, this.root);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const rootValue = _r0.unwrap();
    if (this.subPath.length === 0) {
      return rootValue;
    } else {
      return rootValue.match({
        Json: (v) => {
          const json = v._0;
          let current = json;
          for (const key of this.subPath) {
            const _r1 = current.get(key);
            if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
            current = _r1.unwrap();
          }
          return new Value('Json', { _0: current.clone() });
        },
        Binary: (v) => {
          const bytes = v._0;
          const _r2 = serdeJson.fromSlice(bytes).ok();
          if (_r2 == null) return null;
          const json = _r2;
          let current = json;
          for (const key of this.subPath) {
            const _r3 = ((current as Record<string, unknown>)?.[key] ?? null);
            if (_r3 == null) return null;
            current = _r3;
          }
          return new Value('Json', { _0: structuredClone(current) });
        },
      });
    }
  }

  static from(val: string): PropertyPath {
    return new PropertyPath(val, []);
  }

  equals(other: PropertyPath): boolean {
    if (this.root !== other.root) return false;
    { if (this.subPath.length !== other.subPath.length) return false; for (let i = 0; i < this.subPath.length; i++) { if (this.subPath[i] !== other.subPath[i]) return false; } }
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [keyHash(this.root), keyHash(this.subPath)].join('|');
  }

  compareTo(other: PropertyPath): number {
    let c = this.root < other.root ? -1 : this.root > other.root ? 1 : 0;
    if (c !== 0) return c;
    c = ((xs, ys) => { const n = Math.min(xs.length, ys.length); for (let i = 0; i < n; i++) { const a = xs[i], b = ys[i]; const d = a < b ? -1 : a > b ? 1 : 0; if (d !== 0) return d; } return Math.sign(xs.length - ys.length); })(this.subPath, other.subPath);
    if (c !== 0) return c;
    return 0;
  }

  clone(): PropertyPath {
    return new PropertyPath(this.root, [...this.subPath]);
  }

  debug(): string {
    return `PropertyPath { root: ${JSON.stringify(this.root)}, subPath: ${`[${Array.from(this.subPath).map((e) => JSON.stringify(e)).join(', ')}]`} }`;
  }
}

