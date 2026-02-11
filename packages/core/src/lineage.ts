// MIRRORS: ankurah/core/src/lineage.rs

import { RetrievalError } from './error.ts';

// ---------------------------------------------------------------------------
// Re-exports (mirrors Rust `pub use`)
// ---------------------------------------------------------------------------

// Rust: pub use crate::retrieval::GetEvents;
// Rust: pub use crate::retrieval::Retrieve;
// These are already exported from retrieval.ts; not re-exported here to
// avoid circular dependency. Consumers import from retrieval.ts directly.

// ---------------------------------------------------------------------------
// TEvent / TClock — generic interfaces for lineage comparison
// ---------------------------------------------------------------------------

// Divergence: Rust lineage.rs uses the generic TEvent/TClock traits with
// associated types. The existing retrieval.ts defines concrete TEvent/TClock
// with EventId. For lineage we define generic versions so the BFS engine
// can operate over any Id type (needed for testing and for the real proto types).

/**
 * Generic clock interface for lineage comparison.
 * Rust: `pub trait TClock { type Id; fn members(&self) -> &[Self::Id]; }`
 */
export interface LClock<Id> {
  members(): readonly Id[];
}

/**
 * Generic event interface for lineage comparison.
 * Rust: `pub trait TEvent { type Id; type Parent: TClock; fn id(&self) -> Id; fn parent(&self) -> &Parent; }`
 */
export interface LEvent<Id, C extends LClock<Id>> {
  id(): Id;
  parent(): C;
}

/**
 * Generic event retrieval interface for lineage comparison.
 * Rust: `#[async_trait] pub trait GetEvents { type Id; type Event; ... }`
 */
export interface LGetEvents<Id, E> {
  retrieveEvent(eventIds: Id[]): Promise<[number, LAttested<E>[]]>;
}

/**
 * Lightweight attested wrapper for lineage.
 * Mirrors Attested<T> from proto but works with any payload type.
 */
export interface LAttested<T> {
  payload: T;
}

// ---------------------------------------------------------------------------
// EventAccumulator
// ---------------------------------------------------------------------------

/**
 * Accumulates events during lineage traversal for building event bridges.
 *
 * Rust: `pub struct EventAccumulator<Event>`
 */
export class EventAccumulator<Event> {
  private events: Event[] = [];
  private readonly maximum: number | null;

  constructor(maximum: number | null = null) {
    this.maximum = maximum;
  }

  add(event: Event): boolean {
    if (this.maximum !== null) {
      if (this.events.length >= this.maximum) {
        return false; // Reached maximum
      }
    }
    this.events.push(event);
    return true;
  }

  takeEvents(): Event[] {
    const result = this.events;
    this.events = [];
    return result;
  }

  isAtLimit(): boolean {
    if (this.maximum === null) return false;
    return this.events.length >= this.maximum;
  }
}

// ---------------------------------------------------------------------------
// Ordering — discriminated union
// ---------------------------------------------------------------------------

/**
 * Result of comparing two clocks in the event DAG.
 *
 * Rust: `pub enum Ordering<Id>`
 */
export type Ordering<Id> =
  | { type: 'Equal' }
  | { type: 'Descends' }
  | { type: 'NotDescends'; meet: Id[] }
  | { type: 'Incomparable' }
  | { type: 'PartiallyDescends'; meet: Id[] }
  | { type: 'BudgetExceeded'; subjectFrontier: Set<Id>; otherFrontier: Set<Id> };

// ---------------------------------------------------------------------------
// Origins — private helper (SmallVec<[Id; 8]> -> Array)
// ---------------------------------------------------------------------------

/**
 * Tracks which "other" heads originated a path to this node.
 *
 * Rust: `struct Origins<Id>(SmallVec<[Id; 8]>)`
 * Divergence: SmallVec -> plain Array [A6].
 */
class Origins<Id> {
  private items: Id[];

  constructor() {
    this.items = [];
  }

  add(id: Id): void {
    if (!this.items.includes(id)) {
      this.items.push(id);
    }
  }

  augment(other: Origins<Id>): void {
    for (const h of other.items) {
      if (!this.items.includes(h)) {
        this.items.push(h);
      }
    }
  }

  [Symbol.iterator](): Iterator<Id> {
    return this.items[Symbol.iterator]();
  }

  iter(): Id[] {
    return this.items;
  }

  clone(): Origins<Id> {
    const o = new Origins<Id>();
    o.items = [...this.items];
    return o;
  }
}

// ---------------------------------------------------------------------------
// State — private, per-node BFS bookkeeping
// ---------------------------------------------------------------------------

/**
 * Per-node bookkeeping for the BFS comparison engine.
 *
 * Rust: `struct State<Id>`
 */
class State<Id> {
  seenFromSubject: boolean = false;
  seenFromOther: boolean = false;
  commonChildCount: number = 0;
  origins: Origins<Id> = new Origins<Id>();

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
}

// ---------------------------------------------------------------------------
// Comparison — the core BFS engine
// ---------------------------------------------------------------------------

// Rust: pub(crate) struct Comparison<'a, G>
// Divergence: no lifetime parameter, no Deref on Origins [E8]

/**
 * Core bidirectional BFS engine for comparing two clocks in an event DAG.
 *
 * Rust: `pub(crate) struct Comparison<'a, G>`
 * Divergence: Rust uses lifetime 'a for getter reference; TS uses plain reference [E8].
 * Not exported from the package (pub(crate) -> internal) [A11].
 */
class Comparison<Id, E extends LEvent<Id, LClock<Id>>> {
  private readonly getter: LGetEvents<Id, E>;

  /** The original set of `other` event ids */
  private readonly originalOtherEvents: Set<Id>;

  /** The set of `other` heads still looking for a common ancestor */
  private outstandingHeads: Set<Id>;

  /** The remaining budget for fetching events */
  private remainingBudget: number;

  /* search frontiers */
  private subjectFrontier: Set<Id>;
  private otherFrontier: Set<Id>;

  /* per-node bookkeeping — use string keys for Map since Id may be objects */
  private states: Map<Id, State<Id>>;

  /* incremental meet construction */
  private meetCandidates: Set<Id>;

  /* enum-decision flags */
  private unseenOtherHeads: number;
  private headOverlap: boolean;
  private initialHeadsEqual: boolean;
  private anyCommon: boolean;

  /* event accumulator for building event bridges */
  private subjectEventAccumulator: EventAccumulator<LAttested<E>> | null;

  constructor(
    getter: LGetEvents<Id, E>,
    subject: LClock<Id>,
    other: LClock<Id>,
    budget: number,
    subjectEventAccumulator: EventAccumulator<LAttested<E>> | null = null,
  ) {
    this.getter = getter;

    const subjectMembers = subject.members();
    const otherMembers = other.members();

    this.subjectFrontier = new Set<Id>(subjectMembers);
    const otherSet = new Set<Id>(otherMembers);
    this.originalOtherEvents = new Set<Id>(otherMembers);

    // Early signal: if initial head sets are identical, we can short-circuit Equal
    this.initialHeadsEqual = setsEqual(this.subjectFrontier, otherSet);
    this.headOverlap = this.initialHeadsEqual;

    this.otherFrontier = new Set<Id>(otherMembers);
    this.remainingBudget = budget;

    this.unseenOtherHeads = otherSet.size;

    this.anyCommon = false;
    this.states = new Map<Id, State<Id>>();
    this.meetCandidates = new Set<Id>();
    this.outstandingHeads = new Set<Id>(otherMembers);
    this.subjectEventAccumulator = subjectEventAccumulator;
  }

  takeAccumulatedEvents(): LAttested<E>[] | null {
    if (this.subjectEventAccumulator === null) return null;
    return this.subjectEventAccumulator.takeEvents();
  }

  /**
   * Runs one step of the comparison.
   * Returns an Ordering if a conclusive determination can be made, or null if more steps needed.
   *
   * Rust: `pub async fn step(&mut self) -> Result<Option<Ordering<G::Id>>, RetrievalError>`
   */
  async step(): Promise<Ordering<Id> | null> {
    // Early short-circuit: if the initial head sets are identical, we are Equal.
    if (this.initialHeadsEqual) {
      return { type: 'Equal' };
    }

    // look up events in both frontiers (union)
    const ids: Id[] = setUnion(this.subjectFrontier, this.otherFrontier);
    const resultChecklist = new Set<Id>(ids);

    const [cost, events] = await this.getter.retrieveEvent(ids);
    this.remainingBudget = Math.max(0, this.remainingBudget - cost);

    for (const event of events) {
      if (resultChecklist.has(event.payload.id())) {
        resultChecklist.delete(event.payload.id());
        this.processEvent(event);
      }
    }

    if (resultChecklist.size > 0) {
      const missing = Array.from(resultChecklist).map(String).join(', ');
      throw new RetrievalError('StorageError', `Events not found: [${missing}]`);
    }

    const ordering = this.checkResult();
    if (ordering !== null) {
      return ordering;
    }

    // keep going
    return null;
  }

  private processEvent(event: LAttested<E>): void {
    const id = event.payload.id();
    const parents = event.payload.parent().members();
    const fromSubject = this.subjectFrontier.has(id);
    const fromOther = this.otherFrontier.has(id);

    if (fromSubject) this.subjectFrontier.delete(id);
    if (fromOther) this.otherFrontier.delete(id);

    // Process the current node and capture relevant state
    const nodeState = this.getOrCreateState(id);
    nodeState.markSeenFrom(fromSubject, fromOther);

    // Accumulate events from the subject side for event bridge building
    if (fromSubject && !this.originalOtherEvents.has(id) && !nodeState.isCommon()) {
      if (this.subjectEventAccumulator !== null) {
        this.subjectEventAccumulator.add(event);
      }
    }

    // Handle origins for "other" heads
    if (fromOther && this.originalOtherEvents.has(id)) {
      nodeState.origins.add(id);
    }

    // Capture state before potential modification
    const isCommon = nodeState.isCommon();
    const origins = nodeState.origins.clone();

    // Handle common node and parent updates
    if (isCommon && !this.meetCandidates.has(id)) {
      this.meetCandidates.add(id);
      this.anyCommon = true;

      // remove satisfied heads from the checklist
      for (const h of origins.iter()) {
        this.outstandingHeads.delete(h);
      }

      // Update common child count and propagate origins in one pass over parents
      for (const p of parents) {
        const parentState = this.getOrCreateState(p);
        if (fromOther) {
          parentState.origins.augment(origins);
        }
        parentState.commonChildCount += 1;
      }
    } else if (fromOther) {
      // Just propagate origins if not a common node
      for (const p of parents) {
        const parentState = this.getOrCreateState(p);
        parentState.origins.augment(origins);
      }
    }

    // Extend frontiers
    if (fromSubject) {
      for (const p of parents) {
        this.subjectFrontier.add(p);
      }

      if (this.originalOtherEvents.has(id)) {
        this.unseenOtherHeads = Math.max(0, this.unseenOtherHeads - 1);
        this.headOverlap = true;
      }
    }
    if (fromOther) {
      for (const p of parents) {
        this.otherFrontier.add(p);
      }
    }
  }

  private checkResult(): Ordering<Id> | null {
    // Budget exhausted - can't continue
    if (this.remainingBudget === 0) {
      return {
        type: 'BudgetExceeded',
        subjectFrontier: new Set(this.subjectFrontier),
        otherFrontier: new Set(this.otherFrontier),
      };
    }

    // Both frontiers exhausted - we have complete information
    if (this.subjectFrontier.size === 0 && this.otherFrontier.size === 0) {
      return this.determineFinalOrdering();
    }

    // Early determination: if we've found the meet and all other heads are accounted for,
    // we can determine NotDescends/PartiallyDescends without traversing to root
    if (this.anyCommon && this.outstandingHeads.size === 0 && this.unseenOtherHeads > 0) {
      return this.computeNotDescendsOrdering();
    }

    // Need more steps
    return null;
  }

  private determineFinalOrdering(): Ordering<Id> {
    // Subject has seen all of other's heads
    if (this.unseenOtherHeads === 0) {
      return this.initialHeadsEqual ? { type: 'Equal' } : { type: 'Descends' };
    }

    // Subject hasn't seen all of other's heads - check for common ancestors
    if (!this.anyCommon || this.outstandingHeads.size > 0) {
      return { type: 'Incomparable' };
    }

    return this.computeNotDescendsOrdering();
  }

  private computeNotDescendsOrdering(): Ordering<Id> {
    const meet: Id[] = [];
    for (const id of this.meetCandidates) {
      const state = this.states.get(id);
      const childCount = state ? state.commonChildCount : 0;
      if (childCount === 0) {
        meet.push(id);
      }
    }

    if (this.headOverlap) {
      return { type: 'PartiallyDescends', meet };
    } else {
      return { type: 'NotDescends', meet };
    }
  }

  private getOrCreateState(id: Id): State<Id> {
    let state = this.states.get(id);
    if (state === undefined) {
      state = new State<Id>();
      this.states.set(id, state);
    }
    return state;
  }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/**
 * Compares an unstored event against a stored clock by starting the comparison
 * from the event's parent clock and checking if the other clock is reachable.
 *
 * Rust: `pub async fn compare_unstored_event<G, E, C>(...)`
 */
export async function compareUnstoredEvent<Id, E extends LEvent<Id, LClock<Id>>>(
  getter: LGetEvents<Id, E>,
  subject: LEvent<Id, LClock<Id>>,
  other: LClock<Id>,
  budget: number,
): Promise<Ordering<Id>> {
  // Handle redundant delivery: if the other clock contains exactly this event,
  // return Equal immediately.
  const otherMembers = other.members();
  if (otherMembers.length === 1 && otherMembers[0] === subject.id()) {
    return { type: 'Equal' };
  }

  const subjectParent = subject.parent();

  // Compare the subject's parent clock with the other clock
  const result = await compare(getter, subjectParent, other, budget);
  if (result.type === 'Equal') {
    return { type: 'Descends' };
  }
  return result;
}

/**
 * Compares two Clocks, traversing the event history to classify their
 * causal relationship.
 *
 * Performs a simultaneous, breadth-first walk from the head sets of
 * `subject` and `other`, fetching parents in batches.
 *
 * Rust: `pub async fn compare<G, C>(...)`
 */
export async function compare<Id, E extends LEvent<Id, LClock<Id>>>(
  getter: LGetEvents<Id, E>,
  subject: LClock<Id>,
  other: LClock<Id>,
  budget: number,
): Promise<Ordering<Id>> {
  // bail out right away for the obvious cases
  if (subject.members().length === 0 || other.members().length === 0) {
    return { type: 'Incomparable' };
  }

  const subjectMembers = subject.members();
  const otherMembers = other.members();
  if (
    subjectMembers.length === otherMembers.length &&
    subjectMembers.every((m, i) => m === otherMembers[i])
  ) {
    return { type: 'Equal' };
  }

  const comparison = new Comparison<Id, E>(getter, subject, other, budget);

  // eslint-disable-next-line no-constant-condition
  while (true) {
    const ordering = await comparison.step();
    if (ordering !== null) {
      return ordering;
    }
  }
}

/**
 * Internal: run a comparison with an event accumulator attached.
 * Returns [ordering, accumulatedEvents].
 *
 * Rust: Comparison::new_with_accumulator + step loop + take_accumulated_events
 * Exposed for testing the accumulator behavior.
 */
export async function compareWithAccumulator<Id, E extends LEvent<Id, LClock<Id>>>(
  getter: LGetEvents<Id, E>,
  subject: LClock<Id>,
  other: LClock<Id>,
  budget: number,
  accumulator: EventAccumulator<LAttested<E>>,
): Promise<[Ordering<Id>, LAttested<E>[]]> {
  const comparison = new Comparison<Id, E>(getter, subject, other, budget, accumulator);

  // eslint-disable-next-line no-constant-condition
  while (true) {
    const ordering = await comparison.step();
    if (ordering !== null) {
      const events = comparison.takeAccumulatedEvents() ?? [];
      return [ordering, events];
    }
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function setsEqual<T>(a: Set<T>, b: Set<T>): boolean {
  if (a.size !== b.size) return false;
  for (const item of a) {
    if (!b.has(item)) return false;
  }
  return true;
}

function setUnion<T>(a: Set<T>, b: Set<T>): T[] {
  const result = new Set<T>(a);
  for (const item of b) {
    result.add(item);
  }
  return Array.from(result);
}
