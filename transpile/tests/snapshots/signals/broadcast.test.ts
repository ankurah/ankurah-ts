// MIRRORS: ankurah/signals/src/broadcast.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Broadcast } from './broadcast';
import { Arc, Mutex, OwnedClosure, tokio } from '@ankurah/base';
import { Mut } from './signal/mutable';

describe('broadcast unit tests', () => {
  test('test_multiple_subscribers', () => {
    const sender = Broadcast.new();
    try {
      const counter = Arc.new(new Mutex(0));
      const _sub1 = ((counter) => {
        const _t0 = sender.reference();
        try {
          return _t0.listen((_) => counter.lock().value += 1);
        } finally {
          _t0.drop();
        }
      })(counter.clone());
      try {
        const sub2 = ((counter) => {
          const _t1 = sender.reference();
          try {
            return _t1.listen((_) => counter.lock().value += 10);
          } finally {
            _t1.drop();
          }
        })(counter.clone());
        sender.send([]);
        expect(counter.lock()).toEqual(11);
        sub2.drop();
        sender.send([]);
        expect(counter.lock()).toEqual(12);
      } finally {
        _sub1.drop();
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
    const counter = Arc.new(0);
    const counterClone = counter.clone();
    const _subscription = signal.subscribe((_) => {
      counterClone.fetchAdd(1, undefined /* Ordering */.SeqCst);
    });
    signal.set(100);
    expect(counter.load(undefined /* Ordering */.SeqCst)).toEqual(1);
  });

  test('test_reentrant_subscription_during_send', () => {
    const sender = Broadcast.new();
    try {
      const counter = Arc.new(new Mutex(0));
      const senderClone = sender.clone();
      const counterClone = counter.clone();
      const _t0 = sender.reference();
      try {
        const _sub = _t0.listen(new OwnedClosure([senderClone], (_) => {
          counterClone.lock().value += 1;
          const _t1 = senderClone.reference();
          try {
            const _tempSub = _t1.listen((_) => {
            });
            try {
            } finally {
              _tempSub.drop();
            }
          } finally {
            _t1.drop();
          }
        }));
        try {
          sender.send([]);
          expect(counter.lock()).toEqual(1);
          sender.send([]);
          expect(counter.lock()).toEqual(2);
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

});
