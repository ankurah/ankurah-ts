// MIRRORS: ankurah/core/src/lineage.rs
import { Struct, Enum, Result, dropOwned, checkedAdd, HashMap, HashSet } from '@ankurah/base';
import { RetrievalError } from './error';
import { TClock, TClock_dispatch_members, TEvent_dispatch_id, TEvent_dispatch_parent } from './retrieval';
import { Attested, Event } from '@ankurah/proto';

export class EventAccumulator<Event extends Clone> extends Struct {
  events: Event[];
  maximum: number | null;

  constructor(events: Event[], maximum: number | null) {
    super();
    this.events = events;
    this.maximum = maximum;
  }

  static new<Event>(maximum: number | null): EventAccumulator<Event> {
    return new EventAccumulator([], maximum);
  }

  add(event: Event): boolean {
    {
      const _v = this.maximum;
      if (_v != null) {
        const max = _v;
        if (this.events.length >= max) {
          return false;
        }
      }
    }
    this.events.push(event.clone());
    return true;
  }

  takeEvents(): Event[] {
    try {
      return this.events;
    } finally {
      this.drop();
    }
  }

  isAtLimit(): boolean {
    return this.maximum != null ? ((max) => this.events.length >= max)(this.maximum!) : false;
  }

  clone(): EventAccumulator<Event> {
    return new EventAccumulator(this.events.map(e => e.clone()), this.maximum);
  }

  debug(): string {
    return `EventAccumulator { events: ${this.events}, maximum: ${(($v) => $v === null ? 'None' : `Some(${String($v)})`)(this.maximum)} }`;
  }
}

class Origins<Id extends Clone & PartialEq> extends Struct {
  _0: SmallVec<Id[]>;

  constructor(_0: SmallVec<Id[]>) {
    super();
    this._0 = _0;
  }

  static new<Id>(): Origins<Id> {
    return new Origins(SmallVec.new());
  }

  add(id: Id): void {
    if (!this._0.contains(id)) {
      this._0.push(id);
    }
  }

  augment(other: Origins<Id>): void {
    for (const h of other._0.iter()) {
      if (!this._0.contains(h)) {
        this._0.push(h.clone());
      }
    }
  }

  deref(): Id[] {
    return this._0;
  }

  clone(): Origins<Id> {
    return new Origins(this._0.clone());
  }

  static default<Id>(): Origins<Id> {
    return new Origins(undefined);
  }

  debug(): string {
    return `Origins(${this._0})`;
  }
}

class State<Id extends Clone & PartialEq> extends Struct {
  seenFromSubject: boolean;
  seenFromOther: boolean;
  commonChildCount: number;
  origins: Origins<Id>;

  constructor(seenFromSubject: boolean, seenFromOther: boolean, commonChildCount: number, origins: Origins<Id>) {
    super();
    this.seenFromSubject = seenFromSubject;
    this.seenFromOther = seenFromOther;
    this.commonChildCount = commonChildCount;
    this.origins = origins;
  }

  isCommon(): boolean {
    return this.seenFromSubject && this.seenFromOther;
  }

  markSeenFrom(fromSubject: boolean, fromOther: boolean): void {
    if (fromSubject) {
      this.seenFromSubject = true;
    }
    if (fromOther) {
      this.seenFromOther = true;
    }
  }

  static default<Id>(): State<Id> {
    return new State(false, false, 0, Origins.new());
  }

  clone(): State<Id> {
    return new State(this.seenFromSubject, this.seenFromOther, this.commonChildCount, this.origins.clone());
  }

  debug(): string {
    return `State { seenFromSubject: ${String(this.seenFromSubject)}, seenFromOther: ${String(this.seenFromOther)}, commonChildCount: ${String(this.commonChildCount)}, origins: ${this.origins.debug()} }`;
  }
}

class Comparison<G extends GetEvents> extends Struct {
  getter: G;
  originalOtherEvents: HashSet<Id>;
  outstandingHeads: HashSet<Id>;
  remainingBudget: number;
  subjectFrontier: HashSet<Id>;
  otherFrontier: HashSet<Id>;
  states: HashMap<Id, State<Id>>;
  meetCandidates: HashSet<Id>;
  unseenOtherHeads: number;
  headOverlap: boolean;
  initialHeadsEqual: boolean;
  anyCommon: boolean;
  subjectEventAccumulator: EventAccumulator<Attested<Event>> | null;

  constructor(getter: G, originalOtherEvents: HashSet<Id>, outstandingHeads: HashSet<Id>, remainingBudget: number, subjectFrontier: HashSet<Id>, otherFrontier: HashSet<Id>, states: HashMap<Id, State<Id>>, meetCandidates: HashSet<Id>, unseenOtherHeads: number, headOverlap: boolean, initialHeadsEqual: boolean, anyCommon: boolean, subjectEventAccumulator: EventAccumulator<Attested<Event>> | null) {
    super();
    this.getter = getter;
    this.originalOtherEvents = originalOtherEvents;
    this.outstandingHeads = outstandingHeads;
    this.remainingBudget = remainingBudget;
    this.subjectFrontier = subjectFrontier;
    this.otherFrontier = otherFrontier;
    this.states = states;
    this.meetCandidates = meetCandidates;
    this.unseenOtherHeads = unseenOtherHeads;
    this.headOverlap = headOverlap;
    this.initialHeadsEqual = initialHeadsEqual;
    this.anyCommon = anyCommon;
    this.subjectEventAccumulator = subjectEventAccumulator;
  }

  // A `&T` field is a borrow: dropping this releases the borrow and nothing
  // else, so the cascade must not walk it.
  protected override ownedFields(): unknown[] {
    return [this.originalOtherEvents, this.outstandingHeads, this.remainingBudget, this.subjectFrontier, this.otherFrontier, this.states, this.meetCandidates, this.unseenOtherHeads, this.headOverlap, this.initialHeadsEqual, this.anyCommon, this.subjectEventAccumulator];
  }

  static new<G, C extends TClock>(getter: G, subject: C, other: C, budget: number): Comparison<G> {
    return Comparison.newWithAccumulator(getter, subject, other, budget, null);
  }

  static newWithAccumulator<G, C extends TClock>(getter: G, subject: C, other: C, budget: number, subjectEventAccumulator: EventAccumulator<Attested<Event>> | null): Comparison<G> {
    const subjectFrontier = [...[...TClock_dispatch_members(subject)]];
    const other_1 = [...[...TClock_dispatch_members(other)]];
    const originalOtherEvents = other_1.clone();
    const initialHeadsEqual = subjectFrontier === other_1;
    const headOverlap = initialHeadsEqual;
    return new Comparison(getter, originalOtherEvents, other_1, budget, subjectFrontier, other_1.clone(), new HashMap(), new HashSet(), other_1.size, headOverlap, initialHeadsEqual, false, subjectEventAccumulator);
  }

  takeAccumulatedEvents(): Attested<Event>[] | null {
    try {
      return this.subjectEventAccumulator != null ? ((acc) => acc.takeEvents())(this.subjectEventAccumulator!) : null;
    } finally {
      this.drop();
    }
  }

  async step(): Promise<Result<Ordering<Id> | null, RetrievalError>> {
    if (this.initialHeadsEqual) {
      return Result.Ok(new Ordering('Equal', {}));
    }
    const ids = [...this.subjectFrontier.union(this.otherFrontier)];
    const resultChecklist = [...[...ids]];
    const _r0 = await this.getter.retrieveEvent(ids);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const [cost, events] = _r0.unwrap();
    this.remainingBudget = this.remainingBudget.saturatingSub(cost);
    const _seq1 = events;
    let _at2 = 0;
    try {
      while (_at2 < _seq1.length) {
        const event = _seq1[_at2++];
        try {
          if (resultChecklist.remove(event.payload.id())) {
            this.processEvent(event);
          }
        } finally {
          event.drop();
        }
      }
    } finally {
      dropOwned(_seq1.slice(_at2));
    }
    if (!(resultChecklist.size === 0)) {
      return Result.Err(new RetrievalError('StorageError', { _0: `Events not found: ${resultChecklist}` }));
    }
    {
      const _v = this.checkResult();
      if (_v != null) {
        const ordering = _v;
        return Result.Ok(ordering);
      }
    }
    return Result.Ok(null);
  }

  processEvent(event: Attested<Event>): void {
    const id = event.payload.id();
    const parents = event.payload.parent().members();
    const fromSubject = this.subjectFrontier.remove(id);
    const fromOther = this.otherFrontier.remove(id);
    const [isCommon, origins] = (() => {
      const nodeState = this.states.entry(id.clone()).orDefault();
      nodeState.markSeenFrom(fromSubject, fromOther);
      if (fromSubject && !this.originalOtherEvents.has(id) && !nodeState.isCommon()) {
        {
          const _v = this.subjectEventAccumulator;
          if (_v != null) {
            const accumulator = _v;
            accumulator.add(event);
          }
        }
      }
      if (fromOther && this.originalOtherEvents.has(id)) {
        nodeState.origins.add(id.clone());
      }
      return [nodeState.isCommon(), nodeState.origins.clone()];
    })();
    if (isCommon && this.meetCandidates.insert(id.clone())) {
      this.anyCommon = true;
      for (const h of [...origins]) {
        this.outstandingHeads.delete(h);
      }
      for (const p of parents) {
        const parentState = this.states.entry(p.clone()).orDefault();
        if (fromOther) {
          parentState.origins.augment(origins);
        }
        parentState.commonChildCount = checkedAdd(parentState.commonChildCount, 1, 'usize');
      }
    } else if (fromOther) {
      for (const p of parents) {
        const parentState = this.states.entry(p.clone()).orDefault();
        parentState.origins.augment(origins);
      }
    }
    if (fromSubject) {
      this.subjectFrontier.extend([...[...parents]]);
      if (this.originalOtherEvents.has(id)) {
        this.unseenOtherHeads = this.unseenOtherHeads.saturatingSub(1);
        this.headOverlap = true;
      }
    }
    if (fromOther) {
      this.otherFrontier.extend([...[...parents]]);
    }
  }

  checkResult(): Ordering<Id> | null {
    if (this.remainingBudget === 0) {
      return new Ordering('BudgetExceeded', { subjectFrontier: this.subjectFrontier.clone(), otherFrontier: this.otherFrontier.clone() });
    }
    if (this.subjectFrontier.size === 0 && this.otherFrontier.size === 0) {
      return this.determineFinalOrdering();
    }
    if (this.anyCommon && this.outstandingHeads.size === 0 && this.unseenOtherHeads > 0) {
      return this.computeNotDescendsOrdering();
    }
    return null;
  }

  determineFinalOrdering(): Ordering<Id> {
    if (this.unseenOtherHeads === 0) {
      return this.initialHeadsEqual ? new Ordering('Equal', {}) : new Ordering('Descends', {});
    }
    if (!this.anyCommon || !(this.outstandingHeads.size === 0)) {
      return new Ordering('Incomparable', {});
    }
    return this.computeNotDescendsOrdering();
  }

  computeNotDescendsOrdering(): Ordering<Id> {
    const meet = [...[...this.meetCandidates].filter((id) => this.states.get(id) != null ? ((state) => state.commonChildCount)(this.states.get(id)!) : 0 === 0)];
    if (this.headOverlap) {
      return new Ordering('PartiallyDescends', { meet: meet });
    } else {
      return new Ordering('NotDescends', { meet: meet });
    }
  }
}

export type OrderingV = {
  Equal: {};
  Descends: {};
  NotDescends: { meet: Id[] };
  Incomparable: {};
  PartiallyDescends: { meet: Id[] };
  BudgetExceeded: { subjectFrontier: HashSet<Id>; otherFrontier: HashSet<Id> };
};

export class Ordering<Id> extends Enum<OrderingV> {

  equals(other: Ordering<Id>): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'NotDescends': {
        { if ((this.value as any).meet.length !== (other.value as any).meet.length) return false; for (let i = 0; i < (this.value as any).meet.length; i++) { if (!(this.value as any).meet[i].equals((other.value as any).meet[i])) return false; } }
        break;
      }
      case 'PartiallyDescends': {
        { if ((this.value as any).meet.length !== (other.value as any).meet.length) return false; for (let i = 0; i < (this.value as any).meet.length; i++) { if (!(this.value as any).meet[i].equals((other.value as any).meet[i])) return false; } }
        break;
      }
      case 'BudgetExceeded': {
        if (!(this.value as any).subjectFrontier.equals((other.value as any).subjectFrontier)) return false;
        if (!(this.value as any).otherFrontier.equals((other.value as any).otherFrontier)) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      Equal: () => 'Equal',
      Descends: () => 'Descends',
      NotDescends: (v) => `NotDescends { meet: ${v.meet} }`,
      Incomparable: () => 'Incomparable',
      PartiallyDescends: (v) => `PartiallyDescends { meet: ${v.meet} }`,
      BudgetExceeded: (v) => `BudgetExceeded { subjectFrontier: ${v.subjectFrontier}, otherFrontier: ${v.otherFrontier} }`,
    });
  }
}

export async function compareUnstoredEvent<G, E, C>(getter: G, subject: E, other: C, budget: number): Promise<Result<Ordering<Id>, RetrievalError>> {
  if (TClock_dispatch_members(other).length === 1 && TClock_dispatch_members(other)[0] === TEvent_dispatch_id(subject)) {
    return Result.Ok(new Ordering('Equal', {}));
  }
  const subjectParent = TEvent_dispatch_parent(subject);
  const _r0 = await compare(getter, subjectParent, other, budget);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  let _moved1 = false;
  const result = _r0.unwrap();
  try {
    return Result.Ok((() => {
      return result.match({
        Equal: () => new Ordering('Descends', {}),
        Descends: () => {
          _moved1 = true;
          const other = result;
          return other;
        },
        NotDescends: () => {
          _moved1 = true;
          const other = result;
          return other;
        },
        Incomparable: () => {
          _moved1 = true;
          const other = result;
          return other;
        },
        PartiallyDescends: () => {
          _moved1 = true;
          const other = result;
          return other;
        },
        BudgetExceeded: () => {
          _moved1 = true;
          const other = result;
          return other;
        },
      });
    })());
  } finally {
    if (!_moved1) result.drop();
  }
}

export async function compare<G, C>(getter: G, subject: C, other: C, budget: number): Promise<Result<Ordering<Id>, RetrievalError>> {
  if (TClock_dispatch_members(subject).length === 0 || TClock_dispatch_members(other).length === 0) {
    return Result.Ok(new Ordering('Incomparable', {}));
  }
  if (TClock_dispatch_members(subject) === TClock_dispatch_members(other)) {
    return Result.Ok(new Ordering('Equal', {}));
  }
  let comparison = Comparison.new(getter, subject, other, budget);
  while (true) {
    const _r0 = await comparison.step();
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    {
      const _v = _r0.unwrap();
      if (_v != null) {
        const ordering = _v;
        return Result.Ok(ordering);
      }
    }
  }
}

