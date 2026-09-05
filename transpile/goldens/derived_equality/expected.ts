// MIRRORS: ankurah/derived_equality/src/input.rs
import { Struct, Enum, derivedEquals, derivedClone, HashMap, HashSet, keyHash } from '@ankurah/base';

export class Tag extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }

  equals(other: Tag): boolean {
    if (this.name !== other.name) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [keyHash(this.name)].map((p) => p.length + ':' + p).join('');
  }

  clone(): Tag {
    return new Tag(this.name);
  }
}

export class Buffers extends Struct {
  readonly parts: HashMap<string, Uint8Array>;

  constructor(parts: HashMap<string, Uint8Array>) {
    super();
    this.parts = parts;
  }

  equals(other: Buffers): boolean {
    { if (this.parts.size !== other.parts.size) return false; for (const [k, v] of this.parts) { if (!other.parts.has(k)) return false; const _w = other.parts.get(k)!; { if (v.length !== _w.length) return false; for (let i1 = 0; i1 < v.length; i1++) { if (v[i1] !== _w[i1]) return false; } } } }
    return true;
  }

  clone(): Buffers {
    return new Buffers(this.parts.clone());
  }
}

export class Groups extends Struct {
  readonly members: HashMap<string, Tag[]>;

  constructor(members: HashMap<string, Tag[]>) {
    super();
    this.members = members;
  }

  equals(other: Groups): boolean {
    { if (this.members.size !== other.members.size) return false; for (const [k, v] of this.members) { if (!other.members.has(k)) return false; const _w = other.members.get(k)!; { if (v.length !== _w.length) return false; for (let i1 = 0; i1 < v.length; i1++) { if (!v[i1].equals(_w[i1])) return false; } } } }
    return true;
  }

  clone(): Groups {
    return new Groups(this.members.clone());
  }
}

export class Marked extends Struct {
  readonly tags: HashSet<Tag>;

  constructor(tags: HashSet<Tag>) {
    super();
    this.tags = tags;
  }

  equals(other: Marked): boolean {
    { if (this.tags.size !== other.tags.size) return false; for (const e of this.tags) { if (!other.tags.has(e)) return false; } }
    return true;
  }

  clone(): Marked {
    return new Marked(this.tags.clone());
  }
}

export class Maybe extends Struct {
  readonly tag: Tag | null;
  readonly count: number | null;

  constructor(tag: Tag | null, count: number | null) {
    super();
    this.tag = tag;
    this.count = count;
  }

  equals(other: Maybe): boolean {
    if (this.tag === null && other.tag === null) { /* both null, ok */ }
    else if (this.tag === null || other.tag === null) return false;
    else if (!this.tag.equals(other.tag)) return false;
    if (this.count === null && other.count === null) { /* both null, ok */ }
    else if (this.count === null || other.count === null) return false;
    else if (this.count !== other.count) return false;
    return true;
  }

  clone(): Maybe {
    return new Maybe(this.tag?.clone() ?? null, this.count);
  }
}

export class Nested extends Struct {
  readonly rows: Uint8Array[];

  constructor(rows: Uint8Array[]) {
    super();
    this.rows = rows;
  }

  equals(other: Nested): boolean {
    { if (this.rows.length !== other.rows.length) return false; for (let i = 0; i < this.rows.length; i++) { { if (this.rows[i].length !== other.rows[i].length) return false; for (let i1 = 0; i1 < this.rows[i].length; i1++) { if (this.rows[i][i1] !== other.rows[i][i1]) return false; } } } }
    return true;
  }

  clone(): Nested {
    return new Nested(this.rows.map(e => new Uint8Array(e)));
  }
}

export class Sparse extends Struct {
  readonly slots: HashMap<string, Tag | null>;

  constructor(slots: HashMap<string, Tag | null>) {
    super();
    this.slots = slots;
  }

  equals(other: Sparse): boolean {
    { if (this.slots.size !== other.slots.size) return false; for (const [k, v] of this.slots) { if (!other.slots.has(k)) return false; const _w = other.slots.get(k)!; { if ((v == null) !== (_w == null)) return false; if (v != null) { if (!v.equals(_w)) return false; } } } }
    return true;
  }

  clone(): Sparse {
    return new Sparse(this.slots.clone());
  }
}

export class Paired extends Struct {
  readonly one: [number, Tag];
  readonly many: [string, Tag][];
  readonly maybe: [Tag[], boolean] | null;
  readonly single: [Tag];

  constructor(one: [number, Tag], many: [string, Tag][], maybe: [Tag[], boolean] | null, single: [Tag]) {
    super();
    this.one = one;
    this.many = many;
    this.maybe = maybe;
    this.single = single;
  }

  equals(other: Paired): boolean {
    { if (this.one[0] !== other.one[0]) return false; if (!this.one[1].equals(other.one[1])) return false; }
    { if (this.many.length !== other.many.length) return false; for (let i = 0; i < this.many.length; i++) { { if (this.many[i][0] !== other.many[i][0]) return false; if (!this.many[i][1].equals(other.many[i][1])) return false; } } }
    if (this.maybe === null && other.maybe === null) { /* both null, ok */ }
    else if (this.maybe === null || other.maybe === null) return false;
    else { { if (this.maybe[0].length !== other.maybe[0].length) return false; for (let i1 = 0; i1 < this.maybe[0].length; i1++) { if (!this.maybe[0][i1].equals(other.maybe[0][i1])) return false; } } if (this.maybe[1] !== other.maybe[1]) return false; }
    { if (!this.single[0].equals(other.single[0])) return false; }
    return true;
  }

  clone(): Paired {
    return new Paired([this.one[0], this.one[1].clone()] as [number, Tag], this.many.map(e => [e[0], e[1].clone()] as [string, Tag]), (this.maybe != null ? [this.maybe[0].map(e1 => e1.clone()), this.maybe[1]] as [Tag[], boolean] : null), [this.single[0].clone()] as [Tag]);
  }
}

export class Holder<T> extends Struct {
  readonly one: T;
  readonly many: T[];

  constructor(one: T, many: T[]) {
    super();
    this.one = one;
    this.many = many;
  }

  equals(other: Holder<T>): boolean {
    if (!derivedEquals(this.one, other.one)) return false;
    { if (this.many.length !== other.many.length) return false; for (let i = 0; i < this.many.length; i++) { if (!derivedEquals(this.many[i], other.many[i])) return false; } }
    return true;
  }

  clone(): Holder<T> {
    return new Holder(derivedClone(this.one), this.many.map(e => derivedClone(e)));
  }
}

export type SlotV<T> = {
  Empty: {};
  One: { _0: T };
  Many: { _0: T[] };
};

export class Slot<T> extends Enum<SlotV<T>> {

  clone(): Slot<T> {
    return this.match({
      Empty: () => new Slot<T>('Empty', {}),
      One: (v) => new Slot<T>('One', { _0: derivedClone(v._0) }),
      Many: (v) => new Slot<T>('Many', { _0: v._0.map(e => derivedClone(e)) }),
    });
  }

  equals(other: Slot<T>): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'One': {
        if (!derivedEquals((this.value as any)._0, (other.value as any)._0)) return false;
        break;
      }
      case 'Many': {
        { if ((this.value as any)._0.length !== (other.value as any)._0.length) return false; for (let i = 0; i < (this.value as any)._0.length; i++) { if (!derivedEquals((this.value as any)._0[i], (other.value as any)._0[i])) return false; } }
        break;
      }
    }
    return true;
  }
}

