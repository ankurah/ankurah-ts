// MIRRORS: ankurah/derived_equality/src/input.rs
import { Struct, HashMap, HashSet, keyHash } from '@ankurah/base';

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

