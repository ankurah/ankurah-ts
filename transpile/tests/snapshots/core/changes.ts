// MIRRORS: ankurah/core/src/changes.rs
import { Struct, Enum, Result, dropOwned, derivedClone, iterFilterMap } from '@ankurah/base';
import { Attested, Event } from '@ankurah/proto';
import { Peek } from '@ankurah/signals';
import { Entity } from './entity';
import { MutationError } from './error';
import { ChangeNotification } from './reactor';
import { ResultSet } from './resultset';

export class EntityChange extends Struct implements ChangeNotification {
  entity: Entity;
  events: Attested<Event>[];

  constructor(entity: Entity, events: Attested<Event>[]) {
    super();
    this.entity = entity;
    this.events = events;
  }

  static new(entity: Entity, events: Attested<Event>[]): Result<EntityChange, MutationError> {
    let _moved0 = false;
    let _moved1 = false;
    try {
      try {
        for (const event of events) {
          const head = entity.head();
          try {
            if (!event.payload.entityId.equals(entity.deref().id)) {
              return Result.Err(new MutationError('InvalidEvent', {}));
            }
            let _c3;
            const _t2 = event.payload.id();
            try {
              _c3 = !head.contains(_t2);
            } finally {
              _t2.drop();
            }
            if (_c3) {
              return Result.Err(new MutationError('InvalidEvent', {}));
            }
          } finally {
            head.drop();
          }
        }
        _moved0 = true;
        _moved1 = true;
        return Result.Ok(new EntityChange(entity, events));
      } finally {
        if (!_moved1) dropOwned(events);
      }
    } finally {
      if (!_moved0) entity.drop();
    }
  }

  intoParts(): [Entity, Attested<Event>[]] {
    try {
      return [this.takeField('entity'), this.events];
    } finally {
      this.drop();
    }
  }

  entity(): Entity {
    return this.entity;
  }

  events(): Attested<Event>[] {
    return this.events;
  }

  toString(): string {
    return `EntityChange ${this.entity.collection()}/${this.entity.id()}`;
  }

  clone(): EntityChange {
    return new EntityChange(this.entity.clone(), this.events.map(e => e.clone()));
  }

  debug(): string {
    return `EntityChange { entity: ${this.entity.debug()}, events: ${this.events} }`;
  }
}

export class ChangeSet<R extends View & Clone> extends Struct {
  readonly resultset: ResultSet<R>;
  readonly changes: ItemChange<R>[];

  constructor(resultset: ResultSet<R>, changes: ItemChange<R>[]) {
    super();
    this.resultset = resultset;
    this.changes = changes;
  }

  initial(): R[] {
    return iterFilterMap([...this.changes], (change) => (() => {
      return change.match({
        Initial: (v) => {
          const item = v.item;
          return item.clone();
        },
        Add: () => null,
        Update: () => null,
        Remove: () => null,
      });
    })());
  }

  added(): R[] {
    return iterFilterMap([...this.changes], (change) => (() => {
      return change.match({
        Add: (v) => {
          const item = v.item;
          return item.clone();
        },
        Initial: () => null,
        Update: () => null,
        Remove: () => null,
      });
    })());
  }

  appeared(): R[] {
    return iterFilterMap([...this.changes], (change) => (() => {
      if ((change.is('Add')) || (change.is('Initial'))) {
        const { item } = change.value;
        return item.clone();
      } else {
        return null;
      }
    })());
  }

  adds(): R[] {
    return this.appeared();
  }

  removed(): R[] {
    return iterFilterMap([...this.changes], (change) => (() => {
      return change.match({
        Remove: (v) => {
          const item = v.item;
          return item.clone();
        },
        Initial: () => null,
        Add: () => null,
        Update: () => null,
      });
    })());
  }

  removes(): R[] {
    return this.removed();
  }

  updated(): R[] {
    return iterFilterMap([...this.changes], (change) => (() => {
      return change.match({
        Update: (v) => {
          const item = v.item;
          return item.clone();
        },
        Initial: () => null,
        Add: () => null,
        Remove: () => null,
      });
    })());
  }

  updates(): R[] {
    return this.updated();
  }

  toString(): string {
    let _result = '';
    const results = this.resultset.peek().length;
    _result += `ChangeSet(${results} results): ${[...this.changes].map((c) => c.toString()).join(', ')}`;
    return _result;
  }

  clone(): ChangeSet<R> {
    return new ChangeSet(this.resultset.clone(), this.changes.map(e => e.clone()));
  }

  debug(): string {
    return `ChangeSet { resultset: ${this.resultset.debug()}, changes: ${`[${Array.from(this.changes).map((e) => e.debug()).join(', ')}]`} }`;
  }
}

export type ItemChangeV<I> = {
  Initial: { item: I };
  Add: { item: I; events: Attested<Event>[] };
  Update: { item: I; events: Attested<Event>[] };
  Remove: { item: I; events: Attested<Event>[] };
};

export class ItemChange<I> extends Enum<ItemChangeV<I>> {

  entity(): I {
    {
      const { item } = this.value;
      return item;
    }
  }

  events(): Attested<Event>[] {
    if ((this.is('Add')) || (this.is('Update')) || (this.is('Remove'))) {
      const { events } = this.value;
      return events;
    } else {
      return [];
    }
  }

  kind(): ChangeKind {
    return ChangeKind.from(this);
  }

  toString(): string {
    return this.match({
      Initial: (v) => {
        const item = v.item;
        return `Initial ${I.collection()}/${item.id()}`;
      },
      Add: (v) => {
        const item = v.item;
        return `Add ${I.collection()}/${item.id()}`;
      },
      Update: (v) => {
        const item = v.item;
        return `Update ${I.collection()}/${item.id()}`;
      },
      Remove: (v) => {
        const item = v.item;
        return `Remove ${I.collection()}/${item.id()}`;
      },
    });
  }

  static from<I>(change: ItemChange<Entity>): ItemChange<I> {
    return change.intoMatch({
      Initial: (v) => {
        const item = v.item;
        return new ItemChange('Initial', { item: I.fromEntity(item) });
      },
      Add: (v) => {
        const item = v.item;
        const events = v.events;
        return new ItemChange('Add', { item: I.fromEntity(item), events: events });
      },
      Update: (v) => {
        const item = v.item;
        const events = v.events;
        return new ItemChange('Update', { item: I.fromEntity(item), events: events });
      },
      Remove: (v) => {
        const item = v.item;
        const events = v.events;
        return new ItemChange('Remove', { item: I.fromEntity(item), events: events });
      },
    });
  }

  clone(): ItemChange<I> {
    return this.match({
      Initial: (v) => new ItemChange<I>('Initial', { item: derivedClone(v.item) }),
      Add: (v) => new ItemChange<I>('Add', { item: derivedClone(v.item), events: v.events.map(e => e.clone()) }),
      Update: (v) => new ItemChange<I>('Update', { item: derivedClone(v.item), events: v.events.map(e => e.clone()) }),
      Remove: (v) => new ItemChange<I>('Remove', { item: derivedClone(v.item), events: v.events.map(e => e.clone()) }),
    });
  }

  debug(): string {
    return this.match({
      Initial: (v) => `Initial { item: ${v.item} }`,
      Add: (v) => `Add { item: ${v.item}, events: ${v.events} }`,
      Update: (v) => `Update { item: ${v.item}, events: ${v.events} }`,
      Remove: (v) => `Remove { item: ${v.item}, events: ${v.events} }`,
    });
  }
}

export type ChangeKindV = {
  Initial: {};
  Add: {};
  Remove: {};
  Update: {};
};

export class ChangeKind extends Enum<ChangeKindV> {

  static from<R>(change: ItemChange<R>): ChangeKind {
    return change.match({
      Initial: () => new ChangeKind('Initial', {}),
      Add: () => new ChangeKind('Add', {}),
      Remove: () => new ChangeKind('Remove', {}),
      Update: () => new ChangeKind('Update', {}),
    });
  }

  clone(): ChangeKind {
    return new ChangeKind(this.type, { ...this.value });
  }

  equals(other: ChangeKind): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  debug(): string {
    return this.match({
      Initial: () => 'Initial',
      Add: () => 'Add',
      Remove: () => 'Remove',
      Update: () => 'Update',
    });
  }
}

