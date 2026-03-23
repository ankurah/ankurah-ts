// MIRRORS: ankurah/signals/src/broadcast.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Broadcast } from './broadcast';
import { Arc, Mutex } from '@ankurah/base';
import { Mut } from './signal/mutable';

describe('broadcast unit tests', () => {
  test('test_multiple_subscribers', () => {
    const sender = Broadcast.new();
    const counter = Arc.new(new Mutex(0));
    const Sub1 = ((counter) => {
      const _ret = sender.reference().listen((_) => counter.lock().value += 1);
      counter.drop();
      return _ret;
    })(counter.clone());
    const sub2 = ((counter) => {
      const _ret = sender.reference().listen((_) => counter.lock().value += 10);
      counter.drop();
      return _ret;
    })(counter.clone());
    sender.send([]);
    expect(counter.lock()).toEqual(11);
    drop(sub2);
    sender.send([]);
    expect(counter.lock()).toEqual(12);
    sub2.drop();
    Sub1.drop();
    counter.drop();
    sender.drop();
  });

  test('test_channel_sender_subscriber', () => {
    const sender = Broadcast.new();
    const [tx, rx] = tokio.mpsc.unboundedChannel();
    const Sub = sender.reference().listen(tx);
    sender.send([]);
    if (!(rx . try_recv () . is_ok ())) throw new Error('assertion failed');
    sender.send([]);
    if (!(rx . try_recv () . is_ok ())) throw new Error('assertion failed');
    if (!(rx . try_recv () . is_err ())) throw new Error('assertion failed');
    Sub.drop();
    rx.drop();
    tx.drop();
    sender.drop();
  });

  test('test_subscribe_trait', () => {
    const signal = Mut.new(42);
    const counter = Arc.new(0);
    const counterClone = counter.clone();
    const Subscription = signal.subscribe((_) => {
      counterClone.fetchAdd(1, undefined /* Ordering */.SeqCst);
    });
    signal.set(100);
    expect(counter.load(undefined /* Ordering */.SeqCst)).toEqual(1);
    Subscription.drop();
    counterClone.drop();
    counter.drop();
    signal.drop();
  });

  test('test_reentrant_subscription_during_send', () => {
    const sender = Broadcast.new();
    const counter = Arc.new(new Mutex(0));
    const senderClone = sender.clone();
    const counterClone = counter.clone();
    const Sub = sender.reference().listen((_) => {
      counterClone.lock().value += 1;
      const TempSub = senderClone.reference().listen((_) => {
      });
      TempSub.drop();
    });
    sender.send([]);
    expect(counter.lock()).toEqual(1);
    sender.send([]);
    expect(counter.lock()).toEqual(2);
    Sub.drop();
    counterClone.drop();
    senderClone.drop();
    counter.drop();
    sender.drop();
  });

});
