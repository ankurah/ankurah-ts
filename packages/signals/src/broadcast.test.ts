// MIRRORS: ankurah/signals/src/broadcast.rs

import { describe, test, expect } from 'bun:test';
import { Broadcast, BroadcastId } from './broadcast.ts';

describe('BroadcastId', () => {
  test('auto-incrementing IDs are unique', () => {
    const id1 = new BroadcastId();
    const id2 = new BroadcastId();
    expect(id1.equals(id2)).toBe(false);
    expect(id1.value).not.toBe(id2.value);
  });

  test('equals returns true for same instance', () => {
    const id = new BroadcastId();
    expect(id.equals(id)).toBe(true);
  });

  test('toString returns string representation', () => {
    const id = new BroadcastId();
    expect(typeof id.toString()).toBe('string');
  });
});

describe('Broadcast', () => {
  test('multiple subscribers', () => {
    const sender = new Broadcast<void>();
    let counter = 0;

    // Subscribe two callbacks
    const sub1 = sender.reference().listen({
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

    // Dispose one subscription
    sub2.dispose();

    // Send again - only first callback should be called
    sender.send(undefined as void);
    expect(counter).toBe(12); // 11 + 1 (only sub1)

    sub1.dispose();
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
    guard.dispose();
  });

  test('reentrant subscription during send', () => {
    const sender = new Broadcast<void>();
    let counter = 0;

    // Create a listener that will try to create new subscriptions during the callback
    // This tests that our approach handles re-entrancy without issues
    const _sub = sender.reference().listen({
      type: 'NotifyOnly',
      callback: () => {
        counter += 1;
        // Try to add a new subscription during the callback
        const tempSub = sender.reference().listen({
          type: 'NotifyOnly',
          callback: () => {
            // This callback doesn't matter for the test
          },
        });
        // temp_sub will be disposed here
        tempSub.dispose();
      },
    });

    // Send notification - should work without issues
    sender.send(undefined as void);
    expect(counter).toBe(1);

    // Send again to verify the system is still working
    sender.send(undefined as void);
    expect(counter).toBe(2);
  });

  test('dispose is idempotent', () => {
    const sender = new Broadcast<void>();
    let counter = 0;

    const sub = sender.reference().listen({
      type: 'NotifyOnly',
      callback: () => { counter += 1; },
    });

    sender.send(undefined as void);
    expect(counter).toBe(1);

    // Dispose multiple times should not error
    sub.dispose();
    sub.dispose();
    sub.dispose();

    sender.send(undefined as void);
    expect(counter).toBe(1); // Not called again
  });
});
