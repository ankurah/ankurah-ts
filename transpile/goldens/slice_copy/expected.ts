// MIRRORS: ankurah/slice_copy/src/input.rs
import { Struct } from '@ankurah/base';

export class Event extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  clone(): Event {
    return new Event(this.n);
  }
}

export class Batch extends Struct {
  readonly events: Event[];

  constructor(events: Event[]) {
    super();
    this.events = events;
  }

  copyOfEvents(): Event[] {
    return this.events.map((e) => e.clone());
  }

  static copyOfCounts(counts: number[]): number[] {
    return [...counts];
  }

  static ownedEvents(events: Event[]): Event[] {
    return events.map((e) => e.clone());
  }
}

