// MIRRORS: ankurah/core/src/lineage.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Comparison, EventAccumulator, Ordering, compare, compareUnstoredEvent } from './lineage';
import { HashMap, Result, Struct } from '@ankurah/base';

class TestClock extends Struct implements TClock {
  members: TestId[];

  constructor(members: TestId[]) {
    super();
    this.members = members;
  }

  members(): Uint8Array {
    return this.members;
  }

  clone(): TestClock {
    return new TestClock(this.members.map(e => e.clone()));
  }
}

class TestEvent extends Struct implements TEvent {
  id: TestId;
  parentClock: TestClock;

  constructor(id: TestId, parentClock: TestClock) {
    super();
    this.id = id;
    this.parentClock = parentClock;
  }

  id(): TestId {
    return this.id;
  }

  parent(): TestClock {
    return this.parentClock;
  }

  toString(): string {
    return `Event(${this.id})`;
  }

  clone(): TestEvent {
    return new TestEvent(this.id.clone(), this.parentClock.clone());
  }
}

class MockEventStore extends Struct implements GetEvents {
  events: HashMap<TestId, Attested<TestEvent>>;

  constructor(events: HashMap<TestId, Attested<TestEvent>>) {
    super();
    this.events = events;
  }

  static new(): MockEventStore {
    return new MockEventStore(new HashMap());
  }

  add(id: TestId, parentIds: TestId[]): void {
    const event = new TestEvent(id, new TestClock(parentIds.slice()));
    const attested = new Attested(event, AttestationSet.default());
    this.events.set(id, attested);
  }

  async retrieveEvent(eventIds: number[]): Promise<Result<[number, Attested<Event>[]], RetrievalError>> {
    let result = [];
    for (const id of eventIds) {
      {
        const _v = this.events.get(id);
        if (_v != null) {
          const event = _v;
          result.push(event.clone());
        }
      }
    }
    return Result.Ok([1, result]);
  }

  stageEvents(_events: Attested<TestEvent>[]): void {

  }

  markEventUsed(_eventId: number): void {

  }
}

type TestId = number;

describe('lineage unit tests', () => {
  test('test_linear_history', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      store.add(2, [1]);
      store.add(3, [2]);
      const ancestor = new TestClock([1]);
      try {
        const descendant = new TestClock([3]);
        try {
          const _t0 = await compare(store, descendant, ancestor, 100).unwrap();
          try {
            expect(_t0).toEqual(new Ordering('Descends', {}));
          } finally {
            _t0.drop();
          }
          const _t1 = await compare(store, ancestor, descendant, 100).unwrap();
          try {
            const _t2 = new Ordering('NotDescends', { meet: [1] });
            try {
              expect(_t1).toEqual(_t2);
            } finally {
              _t2.drop();
            }
          } finally {
            _t1.drop();
          }
        } finally {
          descendant.drop();
        }
      } finally {
        ancestor.drop();
      }
    } finally {
      store.drop();
    }
  });

  test('test_concurrent_history', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      store.add(2, [1]);
      store.add(3, [1]);
      store.add(4, [1]);
      store.add(5, [2, 3]);
      store.add(6, [3, 4]);
      store.add(7, [5, 6]);
      await (async () => {
        const ancestor = new TestClock([1]);
        try {
          const descendant = new TestClock([5]);
          try {
            const _t0 = await compare(store, descendant, ancestor, 100).unwrap();
            try {
              expect(_t0).toEqual(new Ordering('Descends', {}));
            } finally {
              _t0.drop();
            }
            const _t1 = await compare(store, ancestor, descendant, 100).unwrap();
            try {
              const _t2 = new Ordering('NotDescends', { meet: [1] });
              try {
                expect(_t1).toEqual(_t2);
              } finally {
                _t2.drop();
              }
            } finally {
              _t1.drop();
            }
          } finally {
            descendant.drop();
          }
        } finally {
          ancestor.drop();
        }
      })()
      await (async () => {
        const ancestor = new TestClock([2, 3]);
        try {
          const descendant = new TestClock([5]);
          try {
            const _t3 = await compare(store, descendant, ancestor, 100).unwrap();
            try {
              expect(_t3).toEqual(new Ordering('Descends', {}));
            } finally {
              _t3.drop();
            }
            const _t4 = await compare(store, ancestor, descendant, 100).unwrap();
            try {
              const _t5 = new Ordering('NotDescends', { meet: [2, 3] });
              try {
                expect(_t4).toEqual(_t5);
              } finally {
                _t5.drop();
              }
            } finally {
              _t4.drop();
            }
          } finally {
            descendant.drop();
          }
        } finally {
          ancestor.drop();
        }
      })()
      await (async () => {
        const a = new TestClock([2]);
        try {
          const b = new TestClock([3]);
          try {
            const _t6 = await compare(store, a, b, 100).unwrap();
            try {
              const _t7 = new Ordering('NotDescends', { meet: [1] });
              try {
                expect(_t6).toEqual(_t7);
              } finally {
                _t7.drop();
              }
            } finally {
              _t6.drop();
            }
            const _t8 = await compare(store, b, a, 100).unwrap();
            try {
              const _t9 = new Ordering('NotDescends', { meet: [1] });
              try {
                expect(_t8).toEqual(_t9);
              } finally {
                _t9.drop();
              }
            } finally {
              _t8.drop();
            }
          } finally {
            b.drop();
          }
        } finally {
          a.drop();
        }
      })()
      {
        const a = new TestClock([6]);
        try {
          const b = new TestClock([2, 3]);
          try {
            const _t10 = await compare(store, a, b, 100).unwrap();
            try {
              const _t11 = new Ordering('PartiallyDescends', { meet: [3] });
              try {
                expect(_t10).toEqual(_t11);
              } finally {
                _t11.drop();
              }
            } finally {
              _t10.drop();
            }
          } finally {
            b.drop();
          }
        } finally {
          a.drop();
        }
      }
    } finally {
      store.drop();
    }
  });

  test('test_incomparable', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      store.add(2, [1]);
      store.add(3, [2]);
      store.add(4, [1]);
      store.add(5, [4]);
      store.add(6, []);
      store.add(7, [6]);
      store.add(8, [7]);
      await (async () => {
        const a = new TestClock([3]);
        try {
          const b = new TestClock([8]);
          try {
            const _t0 = await compare(store, a, b, 100).unwrap();
            try {
              expect(_t0).toEqual(new Ordering('Incomparable', {}));
            } finally {
              _t0.drop();
            }
          } finally {
            b.drop();
          }
        } finally {
          a.drop();
        }
      })()
      await (async () => {
        const a = new TestClock([2]);
        try {
          const b = new TestClock([8]);
          try {
            const _t1 = await compare(store, a, b, 100).unwrap();
            try {
              expect(_t1).toEqual(new Ordering('Incomparable', {}));
            } finally {
              _t1.drop();
            }
          } finally {
            b.drop();
          }
        } finally {
          a.drop();
        }
      })()
      {
        const a = new TestClock([3]);
        try {
          const b = new TestClock([5, 8]);
          try {
            const _t2 = await compare(store, a, b, 100).unwrap();
            try {
              expect(_t2).toEqual(new Ordering('Incomparable', {}));
            } finally {
              _t2.drop();
            }
          } finally {
            b.drop();
          }
        } finally {
          a.drop();
        }
      }
    } finally {
      store.drop();
    }
  });

  test('test_empty_clocks', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      const empty = new TestClock([]);
      try {
        const nonEmpty = new TestClock([1]);
        try {
          const _t0 = await compare(store, empty, empty, 100).unwrap();
          try {
            expect(_t0).toEqual(new Ordering('Incomparable', {}));
          } finally {
            _t0.drop();
          }
          const _t1 = await compare(store, nonEmpty, empty, 100).unwrap();
          try {
            expect(_t1).toEqual(new Ordering('Incomparable', {}));
          } finally {
            _t1.drop();
          }
          const _t2 = await compare(store, empty, nonEmpty, 100).unwrap();
          try {
            expect(_t2).toEqual(new Ordering('Incomparable', {}));
          } finally {
            _t2.drop();
          }
        } finally {
          nonEmpty.drop();
        }
      } finally {
        empty.drop();
      }
    } finally {
      store.drop();
    }
  });

  test('test_budget_exceeded', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      store.add(2, [1]);
      store.add(3, [2]);
      store.add(4, [3]);
      store.add(5, [1]);
      store.add(6, [5]);
      store.add(7, [6]);
      store.add(8, [5]);
      await (async () => {
        const ancestor = new TestClock([1]);
        try {
          const descendant = new TestClock([4]);
          try {
            const _t0 = await compare(store, descendant, ancestor, 2).unwrap();
            try {
              const _t1 = new Ordering('BudgetExceeded', { subjectFrontier: [2], otherFrontier: [] });
              try {
                expect(_t0).toEqual(_t1);
              } finally {
                _t1.drop();
              }
            } finally {
              _t0.drop();
            }
          } finally {
            descendant.drop();
          }
        } finally {
          ancestor.drop();
        }
      })()
      {
        const ancestor = new TestClock([1]);
        try {
          const descendant = new TestClock([4, 5]);
          try {
            const _t2 = await compare(store, descendant, ancestor, 10).unwrap();
            try {
              expect(_t2).toEqual(new Ordering('Descends', {}));
            } finally {
              _t2.drop();
            }
            const _t3 = await compare(store, ancestor, descendant, 2).unwrap();
            try {
              const _t4 = new Ordering('BudgetExceeded', { subjectFrontier: [], otherFrontier: [2] });
              try {
                expect(_t3).toEqual(_t4);
              } finally {
                _t4.drop();
              }
            } finally {
              _t3.drop();
            }
          } finally {
            descendant.drop();
          }
        } finally {
          ancestor.drop();
        }
      }
    } finally {
      store.drop();
    }
  });

  test('test_self_comparison', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      const clock = new TestClock([1]);
      try {
        const _t0 = await compare(store, clock, clock, 100).unwrap();
        try {
          expect(_t0).toEqual(new Ordering('Equal', {}));
        } finally {
          _t0.drop();
        }
      } finally {
        clock.drop();
      }
    } finally {
      store.drop();
    }
  });

  test('multiple_roots', async () => {
    let store = MockEventStore.new();
    try {
      for (const id of undefined /* range 1..6 */) {
        store.add(id, []);
      }
      store.add(7, [1, 2, 3, 4, 5, 6]);
      store.add(8, [7]);
      const subject = new TestClock([8]);
      try {
        const bigOther = new TestClock([1, 2, 3, 4, 5, 6]);
        try {
          const _t0 = await compare(store, subject, bigOther, 1000).unwrap();
          try {
            expect(_t0).toEqual(new Ordering('Descends', {}));
          } finally {
            _t0.drop();
          }
          const _t1 = await compare(store, bigOther, subject, 1000).unwrap();
          try {
            const _t2 = new Ordering('NotDescends', { meet: [1, 2, 3, 4, 5, 6] });
            try {
              expect(_t1).toEqual(_t2);
            } finally {
              _t2.drop();
            }
          } finally {
            _t1.drop();
          }
        } finally {
          bigOther.drop();
        }
      } finally {
        subject.drop();
      }
    } finally {
      store.drop();
    }
  });

  test('test_compare_event_unstored', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      store.add(2, [1]);
      store.add(3, [2]);
      const unstoredEvent = new TestEvent(4, new TestClock([3]));
      try {
        const clock1 = new TestClock([1]);
        try {
          const clock2 = new TestClock([2]);
          try {
            const clock3 = new TestClock([3]);
            try {
              const _t0 = await compareUnstoredEvent(store, unstoredEvent, clock1, 100).unwrap();
              try {
                expect(_t0).toEqual(new Ordering('Descends', {}));
              } finally {
                _t0.drop();
              }
              const _t1 = await compareUnstoredEvent(store, unstoredEvent, clock2, 100).unwrap();
              try {
                expect(_t1).toEqual(new Ordering('Descends', {}));
              } finally {
                _t1.drop();
              }
              const _t2 = await compareUnstoredEvent(store, unstoredEvent, clock3, 100).unwrap();
              try {
                expect(_t2).toEqual(new Ordering('Descends', {}));
              } finally {
                _t2.drop();
              }
              const unstoredMergeEvent = new TestEvent(5, new TestClock([2, 3]));
              try {
                const _t3 = await compareUnstoredEvent(store, unstoredMergeEvent, clock1, 100).unwrap();
                try {
                  expect(_t3).toEqual(new Ordering('Descends', {}));
                } finally {
                  _t3.drop();
                }
                store.add(10, []);
                const incomparableClock = new TestClock([10]);
                try {
                  const _t4 = await compareUnstoredEvent(store, unstoredEvent, incomparableClock, 100).unwrap();
                  try {
                    expect(_t4).toEqual(new Ordering('Incomparable', {}));
                  } finally {
                    _t4.drop();
                  }
                  const rootEvent = new TestEvent(11, new TestClock([]));
                  try {
                    const emptyClock = new TestClock([]);
                    try {
                      const _t5 = await compareUnstoredEvent(store, rootEvent, emptyClock, 100).unwrap();
                      try {
                        expect(_t5).toEqual(new Ordering('Incomparable', {}));
                      } finally {
                        _t5.drop();
                      }
                      const _t6 = await compareUnstoredEvent(store, rootEvent, clock1, 100).unwrap();
                      try {
                        expect(_t6).toEqual(new Ordering('Incomparable', {}));
                      } finally {
                        _t6.drop();
                      }
                      const emptyClock_1 = new TestClock([]);
                      try {
                        const _t7 = await compareUnstoredEvent(store, unstoredEvent, emptyClock_1, 100).unwrap();
                        try {
                          expect(_t7).toEqual(new Ordering('Incomparable', {}));
                        } finally {
                          _t7.drop();
                        }
                      } finally {
                        emptyClock_1.drop();
                      }
                    } finally {
                      emptyClock.drop();
                    }
                  } finally {
                    rootEvent.drop();
                  }
                } finally {
                  incomparableClock.drop();
                }
              } finally {
                unstoredMergeEvent.drop();
              }
            } finally {
              clock3.drop();
            }
          } finally {
            clock2.drop();
          }
        } finally {
          clock1.drop();
        }
      } finally {
        unstoredEvent.drop();
      }
    } finally {
      store.drop();
    }
  });

  test('test_compare_event_redundant_delivery', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      store.add(2, [1]);
      store.add(3, [2]);
      const unstoredEvent = new TestEvent(4, new TestClock([3]));
      try {
        const clock3 = new TestClock([3]);
        try {
          const _t0 = await compareUnstoredEvent(store, unstoredEvent, clock3, 100).unwrap();
          try {
            expect(_t0).toEqual(new Ordering('Descends', {}));
          } finally {
            _t0.drop();
          }
          store.add(4, [3]);
          const clockWithEvent = new TestClock([4]);
          try {
            const _t1 = await compareUnstoredEvent(store, unstoredEvent, clockWithEvent, 100).unwrap();
            try {
              expect(_t1).toEqual(new Ordering('Equal', {}));
            } finally {
              _t1.drop();
            }
            const clockWithMultiple = new TestClock([3, 4]);
            try {
              const _t2 = await compareUnstoredEvent(store, unstoredEvent, clockWithMultiple, 100).unwrap();
              try {
                expect(_t2).toEqual(new Ordering('Incomparable', {}));
              } finally {
                _t2.drop();
              }
            } finally {
              clockWithMultiple.drop();
            }
          } finally {
            clockWithEvent.drop();
          }
        } finally {
          clock3.drop();
        }
      } finally {
        unstoredEvent.drop();
      }
    } finally {
      store.drop();
    }
  });

  test('test_event_accumulator', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      store.add(2, [1]);
      store.add(3, [2]);
      store.add(4, [3]);
      store.add(5, [4]);
      const current = new TestClock([5]);
      try {
        const known = new TestClock([2]);
        try {
          const accumulator = EventAccumulator.new(null);
          let comparison = Comparison.newWithAccumulator(store, current, known, 100, accumulator);
          while (true) {
            {
              const _v = await comparison.step();
              if (_v != null) {
                const ordering = _v;
                expect(ordering).toEqual(new Ordering('Descends', {}));
                break;
              }
            }
          }
          const events = comparison.takeAccumulatedEvents();
          expect([...events].map((e) => e.payload.id()).sorted()).toEqual([3, 4, 5]);
        } finally {
          known.drop();
        }
      } finally {
        current.drop();
      }
    } finally {
      store.drop();
    }
  });

  test('test_event_accumulator_with_concurrent_history', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      store.add(2, [1]);
      store.add(3, [1]);
      store.add(4, [1]);
      store.add(5, [2, 3]);
      store.add(6, [3, 4]);
      store.add(7, [5, 6]);
      const current = new TestClock([7]);
      try {
        const known = new TestClock([1]);
        try {
          const accumulator = EventAccumulator.new(null);
          let comparison = Comparison.newWithAccumulator(store, current, known, 100, accumulator);
          while (true) {
            {
              const _v = await comparison.step();
              if (_v != null) {
                const ordering = _v;
                expect(ordering).toEqual(new Ordering('Descends', {}));
                break;
              }
            }
          }
          const events = comparison.takeAccumulatedEvents();
          const eventIds = [...events].map((e) => e.payload.id());
          expect(eventIds.length).toEqual(6);
          if (!(eventIds.includes(7))) throw new Error('assertion failed');
          if (!(eventIds.includes(5))) throw new Error('assertion failed');
          if (!(eventIds.includes(6))) throw new Error('assertion failed');
          if (!(eventIds.includes(2))) throw new Error('assertion failed');
          if (!(eventIds.includes(3))) throw new Error('assertion failed');
          if (!(eventIds.includes(4))) throw new Error('assertion failed');
          if (!(!eventIds.includes(1))) throw new Error('assertion failed');
        } finally {
          known.drop();
        }
      } finally {
        current.drop();
      }
    } finally {
      store.drop();
    }
  });

  test('test_event_accumulator_equal_clocks', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      store.add(2, [1]);
      store.add(3, [2]);
      const current = new TestClock([3]);
      try {
        const known = new TestClock([3]);
        try {
          const accumulator = EventAccumulator.new(null);
          let comparison = Comparison.newWithAccumulator(store, current, known, 100, accumulator);
          while (true) {
            {
              const _v = await comparison.step();
              if (_v != null) {
                const ordering = _v;
                expect(ordering).toEqual(new Ordering('Equal', {}));
                break;
              }
            }
          }
          const events = comparison.takeAccumulatedEvents();
          expect(events.length).toEqual(0);
        } finally {
          known.drop();
        }
      } finally {
        current.drop();
      }
    } finally {
      store.drop();
    }
  });

  test('test_event_accumulator_only_subject_side', async () => {
    let store = MockEventStore.new();
    try {
      store.add(1, []);
      store.add(2, [1]);
      store.add(3, [1]);
      store.add(4, [2]);
      store.add(5, [3]);
      const subject = new TestClock([4]);
      try {
        const other = new TestClock([5]);
        try {
          const accumulator = EventAccumulator.new(null);
          let comparison = Comparison.newWithAccumulator(store, subject, other, 100, accumulator);
          while (true) {
            {
              const _v = await comparison.step();
              if (_v != null) {
                const ordering = _v;
                if (!(ordering.is('NotDescends'))) throw new Error('assertion failed');
                break;
              }
            }
          }
          const events = comparison.takeAccumulatedEvents();
          const eventIds = [...events].map((e) => e.payload.id());
          if (!(eventIds.includes(4))) throw new Error('assertion failed');
          if (!(eventIds.includes(2))) throw new Error('assertion failed');
          if (!(!eventIds.includes(5))) throw new Error('assertion failed');
          if (!(!eventIds.includes(3))) throw new Error('assertion failed');
        } finally {
          other.drop();
        }
      } finally {
        subject.drop();
      }
    } finally {
      store.drop();
    }
  });

});
