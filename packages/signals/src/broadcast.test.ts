// MIRRORS: ankurah/signals/src/broadcast.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Broadcast } from './broadcast';
import { BincodeWriter, BincodeReader } from './codec';

describe('broadcast unit tests', () => {
  test('test_multiple_subscribers', () => {
    const sender = new Broadcast();
    const counter = Arc.new(new Mutex(0));
    const Sub1 = (() => {
      const counter = counter.clone();
      const _ret = sender.reference().listen((_) => counter.lock() += 1);
      counter.drop();
      return _ret;
    })();
    const sub2 = (() => {
      const counter = counter.clone();
      const _ret = sender.reference().listen((_) => counter.lock() += 10);
      counter.drop();
      return _ret;
    })();
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
    const sender = new Broadcast();
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
    const signal = new Mut(42);
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
    const sender = new Broadcast();
    const counter = Arc.new(new Mutex(0));
    const senderClone = sender.clone();
    const counterClone = counter.clone();
    const Sub = sender.reference().listen((_) => {
      counterClone.lock() += 1;
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
