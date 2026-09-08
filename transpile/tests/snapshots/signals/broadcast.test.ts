// MIRRORS: ankurah/signals/src/broadcast.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Broadcast } from './broadcast';
import { Arc, Mutex, OwnedClosure, checkedAdd, tokio, wrappingAdd } from '@ankurah/base';
import { Mut } from './signal/mutable';

describe('broadcast unit tests', () => {
  test('test_multiple_subscribers', () => {
    const sender = Broadcast.new();
    try {
      const counter = Arc.new(new Mutex(0));
      try {
        const _sub1 = ((counter) => {
          const _t0 = sender.reference();
          try {
            return _t0.listen(new OwnedClosure([counter], (_) => {
              const _t1 = counter.value.lock();
              try {
                return _t1.value = checkedAdd(_t1.value, 1, 'i32');
              } finally {
                _t1.drop();
              }
            }));
          } finally {
            _t0.drop();
          }
        })(counter.clone());
        try {
          let _moved4 = false;
          const sub2 = ((counter) => {
            const _t2 = sender.reference();
            try {
              return _t2.listen(new OwnedClosure([counter], (_) => {
                const _t3 = counter.value.lock();
                try {
                  return _t3.value = checkedAdd(_t3.value, 10, 'i32');
                } finally {
                  _t3.drop();
                }
              }));
            } finally {
              _t2.drop();
            }
          })(counter.clone());
          try {
            sender.send([]);
            const _t5 = counter.value.lock();
            try {
              expect(_t5.value).toEqual(11);
            } finally {
              _t5.drop();
            }
            _moved4 = true;
            sub2.drop();
            sender.send([]);
            const _t6 = counter.value.lock();
            try {
              expect(_t6.value).toEqual(12);
            } finally {
              _t6.drop();
            }
          } finally {
            if (!_moved4) sub2.drop();
          }
        } finally {
          _sub1.drop();
        }
      } finally {
        counter.drop();
      }
    } finally {
      sender.drop();
    }
  });

  test('test_channel_sender_subscriber', () => {
    const sender = Broadcast.new();
    try {
      const [tx, rx] = tokio.sync.mpsc.unbounded_channel();
      const _t0 = sender.reference();
      try {
        const _sub = _t0.listen(tx);
        try {
          sender.send([]);
          const _t1 = rx.tryRecv();
          try {
            if (!(_t1.isOk())) throw new Error('assertion failed');
          } finally {
            _t1.drop();
          }
          sender.send([]);
          const _t2 = rx.tryRecv();
          try {
            if (!(_t2.isOk())) throw new Error('assertion failed');
          } finally {
            _t2.drop();
          }
          const _t3 = rx.tryRecv();
          try {
            if (!(_t3.isErr())) throw new Error('assertion failed');
          } finally {
            _t3.drop();
          }
        } finally {
          _sub.drop();
        }
      } finally {
        _t0.drop();
      }
    } finally {
      sender.drop();
    }
  });

  test('test_subscribe_trait', () => {
    const signal = Mut.new(42);
    try {
      const counter = Arc.new(0);
      try {
        const counterClone = counter.clone();
        const _subscription = signal.subscribe(new OwnedClosure([counterClone], (_) => {
          (() => { const _v = counterClone.value; counterClone.value = wrappingAdd(counterClone.value, 1, 'usize'); return _v; })();
        }));
        try {
          signal.set(100);
          expect(counter.value).toEqual(1);
        } finally {
          _subscription.drop();
        }
      } finally {
        counter.drop();
      }
    } finally {
      signal.drop();
    }
  });

  test('test_reentrant_subscription_during_send', () => {
    const sender = Broadcast.new();
    try {
      const counter = Arc.new(new Mutex(0));
      try {
        const senderClone = sender.clone();
        const counterClone = counter.clone();
        const _t0 = sender.reference();
        try {
          const _sub = _t0.listen(new OwnedClosure([counterClone, senderClone], (_) => {
            const _t1 = counterClone.value.lock();
            try {
              _t1.value = checkedAdd(_t1.value, 1, 'i32');
            } finally {
              _t1.drop();
            }
            const _t2 = senderClone.reference();
            try {
              const _tempSub = _t2.listen((_) => {
              });
              _tempSub.drop();
            } finally {
              _t2.drop();
            }
          }));
          try {
            sender.send([]);
            const _t3 = counter.value.lock();
            try {
              expect(_t3.value).toEqual(1);
            } finally {
              _t3.drop();
            }
            sender.send([]);
            const _t4 = counter.value.lock();
            try {
              expect(_t4.value).toEqual(2);
            } finally {
              _t4.drop();
            }
          } finally {
            _sub.drop();
          }
        } finally {
          _t0.drop();
        }
      } finally {
        counter.drop();
      }
    } finally {
      sender.drop();
    }
  });

});
