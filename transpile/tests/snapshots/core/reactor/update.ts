// MIRRORS: ankurah/core/src/reactor/update.rs
import { Struct, Enum, derivedEquals, derivedClone } from '@ankurah/base';
import { Attested, Event, QueryId } from '@ankurah/proto';

export class ReactorUpdate<E = Entity, Ev = Attested<Event>> extends Struct {
  readonly items: ReactorUpdateItem<E, Ev>[];

  constructor(items: ReactorUpdateItem<E, Ev>[]) {
    super();
    this.items = items;
  }

  equals(other: ReactorUpdate<E, Ev>): boolean {
    { if (this.items.length !== other.items.length) return false; for (let i = 0; i < this.items.length; i++) { if (!this.items[i].equals(other.items[i])) return false; } }
    return true;
  }

  clone(): ReactorUpdate<E, Ev> {
    return new ReactorUpdate(this.items.map(e => e.clone()));
  }

  debug(): string {
    return `ReactorUpdate { items: ${`[${Array.from(this.items).map((e) => e.debug()).join(', ')}]`} }`;
  }
}

export class ReactorUpdateItem<E = Entity, Ev extends Clone = Attested<Event>> extends Struct {
  readonly entity: E;
  readonly events: Ev[];
  readonly predicateRelevance: [QueryId, MembershipChange][];

  constructor(entity: E, events: Ev[], predicateRelevance: [QueryId, MembershipChange][]) {
    super();
    this.entity = entity;
    this.events = events;
    this.predicateRelevance = predicateRelevance;
  }

  hasMembershipChange(): boolean {
    return !(this.predicateRelevance.length === 0);
  }

  equals(other: ReactorUpdateItem<E, Ev>): boolean {
    if (!derivedEquals(this.entity, other.entity)) return false;
    { if (this.events.length !== other.events.length) return false; for (let i = 0; i < this.events.length; i++) { if (!derivedEquals(this.events[i], other.events[i])) return false; } }
    { if (this.predicateRelevance.length !== other.predicateRelevance.length) return false; for (let i = 0; i < this.predicateRelevance.length; i++) { if (!this.predicateRelevance[i].equals(other.predicateRelevance[i])) return false; } }
    return true;
  }

  clone(): ReactorUpdateItem<E, Ev> {
    return new ReactorUpdateItem(derivedClone(this.entity), this.events.map(e => derivedClone(e)), this.predicateRelevance.map(e => [e[0].clone(), e[1].clone()] as [QueryId, MembershipChange]));
  }

  debug(): string {
    return `ReactorUpdateItem { entity: ${this.entity}, events: ${this.events}, predicateRelevance: ${this.predicateRelevance} }`;
  }
}

export type MembershipChangeV = {
  Initial: {};
  Add: {};
  Remove: {};
};

export class MembershipChange extends Enum<MembershipChangeV> {

  clone(): MembershipChange {
    return new MembershipChange(this.type, { ...this.value });
  }

  equals(other: MembershipChange): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  debug(): string {
    return this.match({
      Initial: () => 'Initial',
      Add: () => 'Add',
      Remove: () => 'Remove',
    });
  }
}

