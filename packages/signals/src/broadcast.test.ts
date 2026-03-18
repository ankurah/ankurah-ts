// MIRRORS: ankurah/signals/src/broadcast.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Broadcast, BroadcastId } from './broadcast.ts';
import { Mut } from './signal/mutable.ts';

describe('Broadcast (unit tests from broadcast.rs)', () => {
  // Rust: fn test_multiple_subscribers()
  test('test_multiple_subscribers', () => {
    const sender = new Broadcast<void>();
    let counter = 0;

    // Subscribe two callbacks
    const _sub1 = sender.reference().listen({
      type: 'Payload',
      callback: () => { counter += 1; },
    });

    const sub2 = sender.reference().listen({
      type: 'Payload',
      callback: () => { counter += 10; },
    });

    // Send notification - both callbacks should be called
    sender.send(undefined as void);
    expect(counter).toBe(11); // 1 + 10

    // Drop one subscription
    sub2.drop();

    // Send again - only first callback should be called
    sender.send(undefined as void);
    expect(counter).toBe(12); // 11 + 1 (only sub1)
  });

  // Rust: fn test_channel_sender_subscriber()
  // SKIP: tokio feature-gated test using tokio::sync::mpsc::unbounded_channel.
  // TS has no channel equivalent; the Broadcast listener pattern covers this.

  // Rust: fn test_subscribe_trait()
  test('test_subscribe_trait', () => {
    const signal = new Mut(42);
    let callCount = 0;

    const _subscription = signal.subscribe(() => {
      callCount++;
    });

    signal.set(100);

    // Should have been called once
    expect(callCount).toBe(1);
  });

  // Rust: fn test_reentrant_subscription_during_send()
  test('test_reentrant_subscription_during_send', () => {
    const sender = new Broadcast<void>();
    let counter = 0;

    // Create a listener that will try to create new subscriptions during the callback
    // This tests that our approach handles re-entrancy without deadlocks
    const senderClone = sender.clone();
    const _sub = sender.reference().listen({
      type: 'NotifyOnly',
      callback: () => {
        counter += 1;

        // Try to add a new subscription during the callback - should work without deadlock
        const _tempSub = senderClone.reference().listen({
          type: 'NotifyOnly',
          callback: () => {
            // This callback doesn't matter for the test
          },
        });
        // temp_sub will be dropped here, which should also work without deadlock
        _tempSub.drop();
      },
    });

    // Send notification - this should work without deadlocks
    sender.send(undefined as void);

    // Verify the callback was called
    expect(counter).toBe(1);

    // Send again to verify the system is still working
    sender.send(undefined as void);
    expect(counter).toBe(2);
  });
});

// Additional TS-only tests for broadcast coverage
describe('Broadcast (additional TS tests)', () => {
  test('BroadcastId auto-incrementing IDs are unique', () => {
    const id1 = new BroadcastId();
    const id2 = new BroadcastId();
    expect(id1.equals(id2)).toBe(false);
    expect(id1.toNumber()).not.toBe(id2.toNumber());
  });

  test('BroadcastId equals returns true for same instance', () => {
    const id = new BroadcastId();
    expect(id.equals(id)).toBe(true);
  });

  test('BroadcastId toString returns string representation', () => {
    const id = new BroadcastId();
    expect(typeof id.toString()).toBe('string');
  });

  test('notify-only listeners', () => {
    const sender = new Broadcast<string>();
    let notified = false;

    const _sub = sender.reference().listen({
      type: 'NotifyOnly',
      callback: () => { notified = true; },
    });

    sender.send('hello');
    expect(notified).toBe(true);
  });

  test('payload listeners receive value', () => {
    const sender = new Broadcast<number>();
    const received: number[] = [];

    const _sub = sender.reference().listen({
      type: 'Payload',
      callback: (value) => { received.push(value); },
    });

    sender.send(42);
    sender.send(99);
    expect(received).toEqual([42, 99]);
  });

  test('broadcast ID is consistent', () => {
    const sender = new Broadcast<void>();
    const id1 = sender.id();
    const id2 = sender.id();
    expect(id1.equals(id2)).toBe(true);
  });

  test('reference broadcast ID matches sender ID', () => {
    const sender = new Broadcast<void>();
    const ref = sender.reference();
    expect(ref.broadcastId().equals(sender.id())).toBe(true);
  });

  test('listener guard broadcast ID matches sender ID', () => {
    const sender = new Broadcast<void>();
    const guard = sender.reference().listen({
      type: 'NotifyOnly',
      callback: () => {},
    });
    expect(guard.broadcastId().equals(sender.id())).toBe(true);
    guard.drop();
  });

  test('drop is idempotent', () => {
    const sender = new Broadcast<void>();
    let counter = 0;

    const sub = sender.reference().listen({
      type: 'NotifyOnly',
      callback: () => { counter += 1; },
    });

    sender.send(undefined as void);
    expect(counter).toBe(1);

    // Drop multiple times should not error
    sub.drop();
    sub.drop();
    sub.drop();

    sender.send(undefined as void);
    expect(counter).toBe(1); // Not called again
  });
});
