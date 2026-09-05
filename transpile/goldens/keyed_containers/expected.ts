// MIRRORS: ankurah/keyed_containers/src/input.rs
import { Struct, checkedAdd, HashMap, HashSet, keyHash } from '@ankurah/base';

export class Key extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }

  equals(other: Key): boolean {
    if (this.name !== other.name) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [keyHash(this.name)].map((p) => p.length + ':' + p).join('');
  }

  clone(): Key {
    return new Key(this.name);
  }
}

export class Bag extends Struct {
  readonly named: HashMap<Key, number>;
  readonly tags: HashSet<Key>;

  constructor(named: HashMap<Key, number>, tags: HashSet<Key>) {
    super();
    this.named = named;
    this.tags = tags;
  }

  clone(): Bag {
    return new Bag(this.named.clone(), this.tags.clone());
  }

  static default(): Bag {
    return new Bag(new HashMap(), new HashSet());
  }
}

export function built(): HashMap<Key, number> {
  return HashMap.from([[new Key('a'), 1]]);
}

export function tagged(): HashSet<Key> {
  return HashSet.from([new Key('a')]);
}

export function ordered(): HashMap<Key, number> {
  return HashMap.from([[new Key('a'), 1]]);
}

export function counted(words: Key[]): HashMap<Key, number> {
  const counts = new HashMap<Key, number>();
  for (const w of words) {
    const _m0 = counts.entry(w.clone()).orInsert(0);
    _m0.value = checkedAdd(_m0.value, 1, 'u32');
  }
  return counts;
}

