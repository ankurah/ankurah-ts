// MIRRORS: ankurah/core/src/lineage.rs

import { describe, expect, test } from 'bun:test';
import {
  EventAccumulator,
  compare,
  compareUnstoredEvent,
  compareWithAccumulator,
  type Ordering,
  type LClock,
  type LEvent,
  type LGetEvents,
  type LAttested,
} from '../src/lineage.ts';

// ---------------------------------------------------------------------------
// Simple test types (mirrors Rust TestId/TestClock/TestEvent)
// ---------------------------------------------------------------------------
// Rust uses TestId = u32; we use string for simplicity in TS.

type TestId = string;

class TestClock implements LClock<TestId> {
  private readonly _members: TestId[];

  constructor(members: TestId[]) {
    this._members = members;
  }

  members(): readonly TestId[] {
    return this._members;
  }
}

class TestEvent implements LEvent<TestId, TestClock> {
  private readonly _id: TestId;
  private readonly parentClock: TestClock;

  constructor(id: TestId, parentClock: TestClock) {
    this._id = id;
    this.parentClock = parentClock;
  }

  id(): TestId {
    return this._id;
  }

  parent(): TestClock {
    return this.parentClock;
  }

  toString(): string {
    return `Event(${this._id})`;
  }
}

// ---------------------------------------------------------------------------
// MockEventStore
// ---------------------------------------------------------------------------

interface AttestedTestEvent extends LAttested<TestEvent> {
  payload: TestEvent;
}

class MockEventStore implements LGetEvents<TestId, TestEvent> {
  private events: Map<TestId, AttestedTestEvent> = new Map();

  add(id: TestId, parentIds: TestId[]): void {
    const event = new TestEvent(id, new TestClock(parentIds));
    const attested: AttestedTestEvent = { payload: event };
    this.events.set(id, attested);
  }

  async retrieveEvent(eventIds: TestId[]): Promise<[number, AttestedTestEvent[]]> {
    const result: AttestedTestEvent[] = [];
    for (const id of eventIds) {
      const event = this.events.get(id);
      if (event !== undefined) {
        result.push(event);
      }
    }
    return [1, result];
  }
}

// ---------------------------------------------------------------------------
// Helper: compare orderings (deep equality for discriminated unions)
// ---------------------------------------------------------------------------

function orderingEquals(a: Ordering<TestId>, b: Ordering<TestId>): boolean {
  if (a.type !== b.type) return false;
  switch (a.type) {
    case 'Equal':
    case 'Descends':
    case 'Incomparable':
      return true;
    case 'NotDescends':
    case 'PartiallyDescends': {
      const bMeet = (b as typeof a).meet;
      if (a.meet.length !== bMeet.length) return false;
      const aSorted = [...a.meet].sort();
      const bSorted = [...bMeet].sort();
      return aSorted.every((v, i) => v === bSorted[i]);
    }
    case 'BudgetExceeded': {
      const bBE = b as typeof a;
      const aSubject = [...a.subjectFrontier].sort();
      const bSubject = [...bBE.subjectFrontier].sort();
      const aOther = [...a.otherFrontier].sort();
      const bOther = [...bBE.otherFrontier].sort();
      return (
        aSubject.length === bSubject.length &&
        aSubject.every((v, i) => v === bSubject[i]) &&
        aOther.length === bOther.length &&
        aOther.every((v, i) => v === bOther[i])
      );
    }
  }
}

function expectOrdering(actual: Ordering<TestId>, expected: Ordering<TestId>): void {
  if (!orderingEquals(actual, expected)) {
    // Provide a clear error message
    expect(JSON.stringify(actual)).toBe(JSON.stringify(expected));
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('lineage', () => {
  // 1. test_linear_history
  test('test_linear_history', async () => {
    const store = new MockEventStore();

    // Create a linear chain: 1 <- 2 <- 3
    store.add('1', []);
    store.add('2', ['1']);
    store.add('3', ['2']);

    const ancestor = new TestClock(['1']);
    const descendant = new TestClock(['3']);

    // descendant descends from ancestor
    expectOrdering(await compare(store, descendant, ancestor, 100), { type: 'Descends' });

    // ancestor does not descend from descendant, but they both have a common ancestor: [1]
    expectOrdering(await compare(store, ancestor, descendant, 100), {
      type: 'NotDescends',
      meet: ['1'],
    });
  });

  // 2. test_concurrent_history
  test('test_concurrent_history', async () => {
    const store = new MockEventStore();

    //      1
    //   /  |  \
    //  2   3   4
    //   \ / \ /
    //    5   6
    //     \ /
    //      7
    store.add('1', []);
    store.add('2', ['1']);
    store.add('3', ['1']);
    store.add('4', ['1']);
    store.add('5', ['2', '3']);
    store.add('6', ['3', '4']);
    store.add('7', ['5', '6']);

    {
      const ancestor = new TestClock(['1']);
      const descendant = new TestClock(['5']);
      expectOrdering(await compare(store, descendant, ancestor, 100), { type: 'Descends' });
      expectOrdering(await compare(store, ancestor, descendant, 100), {
        type: 'NotDescends',
        meet: ['1'],
      });
    }
    {
      const ancestor = new TestClock(['2', '3']);
      const descendant = new TestClock(['5']);
      expectOrdering(await compare(store, descendant, ancestor, 100), { type: 'Descends' });
      expectOrdering(await compare(store, ancestor, descendant, 100), {
        type: 'NotDescends',
        meet: ['2', '3'],
      });
    }
    {
      const a = new TestClock(['2']);
      const b = new TestClock(['3']);
      expectOrdering(await compare(store, a, b, 100), {
        type: 'NotDescends',
        meet: ['1'],
      });
      expectOrdering(await compare(store, b, a, 100), {
        type: 'NotDescends',
        meet: ['1'],
      });
    }
    {
      const a = new TestClock(['6']);
      const b = new TestClock(['2', '3']);
      expectOrdering(await compare(store, a, b, 100), {
        type: 'PartiallyDescends',
        meet: ['3'],
      });
    }
  });

  // 3. test_incomparable
  test('test_incomparable', async () => {
    const store = new MockEventStore();

    //   1        6
    //   |  \     |
    //   2   4    7
    //   |   |    |
    //   3   5    8
    store.add('1', []);
    store.add('2', ['1']);
    store.add('3', ['2']);
    store.add('4', ['1']);
    store.add('5', ['4']);

    store.add('6', []);
    store.add('7', ['6']);
    store.add('8', ['7']);

    {
      const a = new TestClock(['3']);
      const b = new TestClock(['8']);
      expectOrdering(await compare(store, a, b, 100), { type: 'Incomparable' });
    }
    {
      const a = new TestClock(['2']);
      const b = new TestClock(['8']);
      expectOrdering(await compare(store, a, b, 100), { type: 'Incomparable' });
    }
    {
      const a = new TestClock(['3']);
      const b = new TestClock(['5', '8']);
      expectOrdering(await compare(store, a, b, 100), { type: 'Incomparable' });
    }
  });

  // 4. test_empty_clocks
  test('test_empty_clocks', async () => {
    const store = new MockEventStore();
    store.add('1', []);

    const empty = new TestClock([]);
    const nonEmpty = new TestClock(['1']);

    expectOrdering(await compare(store, empty, empty, 100), { type: 'Incomparable' });
    expectOrdering(await compare(store, nonEmpty, empty, 100), { type: 'Incomparable' });
    expectOrdering(await compare(store, empty, nonEmpty, 100), { type: 'Incomparable' });
  });

  // 5. test_budget_exceeded
  test('test_budget_exceeded', async () => {
    const store = new MockEventStore();

    //   1
    //   |  \
    //   2   5
    //   |   |  \
    //   3   6   8
    //   |   |
    //   4   7
    store.add('1', []);
    store.add('2', ['1']);
    store.add('3', ['2']);
    store.add('4', ['3']);
    store.add('5', ['1']);
    store.add('6', ['5']);
    store.add('7', ['6']);
    store.add('8', ['5']);

    {
      const ancestor = new TestClock(['1']);
      const descendant = new TestClock(['4']);

      expectOrdering(await compare(store, descendant, ancestor, 2), {
        type: 'BudgetExceeded',
        subjectFrontier: new Set(['2']),
        otherFrontier: new Set([]),
      });
    }
    {
      const ancestor = new TestClock(['1']);
      const descendant = new TestClock(['4', '5']);

      expectOrdering(await compare(store, descendant, ancestor, 10), { type: 'Descends' });

      expectOrdering(await compare(store, ancestor, descendant, 2), {
        type: 'BudgetExceeded',
        subjectFrontier: new Set([]),
        otherFrontier: new Set(['2']),
      });
    }
  });

  // 6. test_self_comparison
  test('test_self_comparison', async () => {
    const store = new MockEventStore();
    store.add('1', []);

    const clock = new TestClock(['1']);
    expectOrdering(await compare(store, clock, clock, 100), { type: 'Equal' });
  });

  // 7. multiple_roots
  test('multiple_roots', async () => {
    //   1   2   3   4   5   6  <- six independent roots
    //   |   |   |   |   |   |
    //   +-------+-----------+
    //           |
    //           7
    //           |
    //           8
    const store = new MockEventStore();

    for (let id = 1; id <= 6; id++) {
      store.add(String(id), []);
    }

    store.add('7', ['1', '2', '3', '4', '5', '6']);
    store.add('8', ['7']);

    const subject = new TestClock(['8']);
    const bigOther = new TestClock(['1', '2', '3', '4', '5', '6']);

    expectOrdering(await compare(store, subject, bigOther, 1000), { type: 'Descends' });

    expectOrdering(await compare(store, bigOther, subject, 1000), {
      type: 'NotDescends',
      meet: ['1', '2', '3', '4', '5', '6'],
    });
  });

  // 8. test_compare_event_unstored
  test('test_compare_event_unstored', async () => {
    const store = new MockEventStore();

    store.add('1', []);
    store.add('2', ['1']);
    store.add('3', ['2']);

    const unstoredEvent = new TestEvent('4', new TestClock(['3']));

    const clock1 = new TestClock(['1']);
    const clock2 = new TestClock(['2']);
    const clock3 = new TestClock(['3']);

    expectOrdering(await compareUnstoredEvent(store, unstoredEvent, clock1, 100), {
      type: 'Descends',
    });
    expectOrdering(await compareUnstoredEvent(store, unstoredEvent, clock2, 100), {
      type: 'Descends',
    });
    expectOrdering(await compareUnstoredEvent(store, unstoredEvent, clock3, 100), {
      type: 'Descends',
    });

    // Test with an unstored event that has multiple parents
    const unstoredMergeEvent = new TestEvent('5', new TestClock(['2', '3']));
    expectOrdering(await compareUnstoredEvent(store, unstoredMergeEvent, clock1, 100), {
      type: 'Descends',
    });

    // Test with an incomparable case
    store.add('10', []);
    const incomparableClock = new TestClock(['10']);
    expectOrdering(await compareUnstoredEvent(store, unstoredEvent, incomparableClock, 100), {
      type: 'Incomparable',
    });

    // Test root event case
    const rootEvent = new TestEvent('11', new TestClock([]));
    const emptyClock = new TestClock([]);
    expectOrdering(await compareUnstoredEvent(store, rootEvent, emptyClock, 100), {
      type: 'Incomparable',
    });
    expectOrdering(await compareUnstoredEvent(store, rootEvent, clock1, 100), {
      type: 'Incomparable',
    });

    // Test that a non-empty unstored event does not descend from an empty clock
    expectOrdering(await compareUnstoredEvent(store, unstoredEvent, emptyClock, 100), {
      type: 'Incomparable',
    });
  });

  // 9. test_compare_event_redundant_delivery
  test('test_compare_event_redundant_delivery', async () => {
    const store = new MockEventStore();

    store.add('1', []);
    store.add('2', ['1']);
    store.add('3', ['2']);

    const unstoredEvent = new TestEvent('4', new TestClock(['3']));

    // Test the normal case first
    const clock3 = new TestClock(['3']);
    expectOrdering(await compareUnstoredEvent(store, unstoredEvent, clock3, 100), {
      type: 'Descends',
    });

    // Now store event 4 to simulate it being applied
    store.add('4', ['3']);

    // Test redundant delivery: the event is already in the clock (exact match)
    const clockWithEvent = new TestClock(['4']);
    expectOrdering(await compareUnstoredEvent(store, unstoredEvent, clockWithEvent, 100), {
      type: 'Equal',
    });

    // Test case where the event is in the clock but with other events too
    const clockWithMultiple = new TestClock(['3', '4']);
    expectOrdering(await compareUnstoredEvent(store, unstoredEvent, clockWithMultiple, 100), {
      type: 'Incomparable',
    });
  });

  // 10. test_event_accumulator
  test('test_event_accumulator', async () => {
    const store = new MockEventStore();

    // Create a linear chain: 1 <- 2 <- 3 <- 4 <- 5
    store.add('1', []);
    store.add('2', ['1']);
    store.add('3', ['2']);
    store.add('4', ['3']);
    store.add('5', ['4']);

    const current = new TestClock(['5']);
    const known = new TestClock(['2']);

    const accumulator = new EventAccumulator<LAttested<TestEvent>>(null);
    const [ordering, events] = await compareWithAccumulator(store, current, known, 100, accumulator);

    expectOrdering(ordering, { type: 'Descends' });

    // Should have accumulated events 5, 4, 3 (traversing from current back to known)
    // Should NOT contain event 2 (that's the known head) or event 1 (common ancestor)
    const eventIds = events.map((e) => e.payload.id()).sort();
    expect(eventIds).toEqual(['3', '4', '5']);
  });

  // 11. test_event_accumulator_with_concurrent_history
  test('test_event_accumulator_with_concurrent_history', async () => {
    const store = new MockEventStore();

    //      1
    //   /  |  \
    //  2   3   4
    //   \ / \ /
    //    5   6
    //     \ /
    //      7
    store.add('1', []);
    store.add('2', ['1']);
    store.add('3', ['1']);
    store.add('4', ['1']);
    store.add('5', ['2', '3']);
    store.add('6', ['3', '4']);
    store.add('7', ['5', '6']);

    const current = new TestClock(['7']);
    const known = new TestClock(['1']);

    const accumulator = new EventAccumulator<LAttested<TestEvent>>(null);
    const [ordering, events] = await compareWithAccumulator(store, current, known, 100, accumulator);

    expectOrdering(ordering, { type: 'Descends' });

    const eventIds = events.map((e) => e.payload.id());

    // Should have accumulated all events from 7 back to (but not including) 1
    expect(eventIds.length).toBe(6); // 7, 5, 6, 2, 3, 4
    expect(eventIds).toContain('7');
    expect(eventIds).toContain('5');
    expect(eventIds).toContain('6');
    expect(eventIds).toContain('2');
    expect(eventIds).toContain('3');
    expect(eventIds).toContain('4');
    // Should NOT contain event 1 (that's the known head)
    expect(eventIds).not.toContain('1');
  });

  // 12. test_event_accumulator_equal_clocks
  test('test_event_accumulator_equal_clocks', async () => {
    const store = new MockEventStore();

    store.add('1', []);
    store.add('2', ['1']);
    store.add('3', ['2']);

    const current = new TestClock(['3']);
    const known = new TestClock(['3']);

    const accumulator = new EventAccumulator<LAttested<TestEvent>>(null);
    const [ordering, events] = await compareWithAccumulator(store, current, known, 100, accumulator);

    expectOrdering(ordering, { type: 'Equal' });

    // Equal clocks short-circuit immediately, no events traversed or accumulated
    expect(events.length).toBe(0);
  });

  // 13. test_event_accumulator_only_subject_side
  test('test_event_accumulator_only_subject_side', async () => {
    const store = new MockEventStore();

    //      1
    //   /     \
    //  2       3
    //  |       |
    //  4       5
    store.add('1', []);
    store.add('2', ['1']);
    store.add('3', ['1']);
    store.add('4', ['2']);
    store.add('5', ['3']);

    const subject = new TestClock(['4']);
    const other = new TestClock(['5']);

    const accumulator = new EventAccumulator<LAttested<TestEvent>>(null);
    const [ordering, events] = await compareWithAccumulator(store, subject, other, 100, accumulator);

    // Should be NotDescends with common ancestor [1]
    expect(ordering.type).toBe('NotDescends');

    const eventIds = events.map((e) => e.payload.id());

    // Should only accumulate events from the subject side (4, 2)
    // Should NOT accumulate events from the other side (5, 3)
    expect(eventIds).toContain('4');
    expect(eventIds).toContain('2');
    expect(eventIds).not.toContain('5');
    expect(eventIds).not.toContain('3');
  });
});
