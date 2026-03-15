// MIRRORS: ankurah/signals/tests/observer_context.rs
//
// All 9 Rust test functions ported.

import { describe, test, expect } from 'bun:test';
import { Mut, CurrentObserver, CallbackObserver } from '../src/index.ts';

describe('observer context tests (from tests/observer_context.rs)', () => {
  // Rust: async fn test_manual_subscription_works()
  test('test_manual_subscription_works', () => {
    const signal = new Mut(42);
    const readSignal = signal.read();

    const results: string[] = [];

    const subscriptionHandle = readSignal.subscribe((value: number) => {
      results.push(`notified: ${value}`);
    });

    signal.set(100);
    // Divergence: TS notifications are synchronous, no sleep needed [E8]

    expect(results).toEqual(['notified: 100']);

    subscriptionHandle.drop(); // Clean up
  });

  // Rust: async fn test_basic_observer_subscription()
  test('test_basic_observer_subscription', () => {
    const signal = new Mut(42);
    const readSignal = signal.read();

    const results: string[] = [];

    const observer = new CallbackObserver(() => {
      results.push('observer callback triggered');
    });

    // Step 1: Initially, no context is set
    expect(CurrentObserver.current()).toBeNull();

    // Step 2: Manually trigger observer with context to establish subscription
    observer.withContext(() => {
      // Context should be set during callback
      expect(CurrentObserver.current()).not.toBeNull();
      const value = readSignal.get(); // This should subscribe the observer to the signal
      results.push(`manual read: ${value}`);
    });

    // Context should be cleared after withContext
    expect(CurrentObserver.current()).toBeNull();
    expect(results).toEqual(['manual read: 42']);
    results.length = 0;

    // Step 3: Change the signal - should trigger the observer's callback
    signal.set(100);

    // Divergence: TS notifications are synchronous [E8]
    expect(results).toEqual(['observer callback triggered']);
  });

  // Rust: async fn test_multiple_signals_single_observer()
  test('test_multiple_signals_single_observer', () => {
    const name = new Mut<string>('Alice');
    const age = new Mut<number>(25);
    const nameRead = name.read();
    const ageRead = age.read();

    const results: string[] = [];

    // Rust: CallbackObserver::new(Arc::new(move || { ... }))
    // Divergence: Rust clones the Read signals for the closure; TS closures
    // capture by reference [E8]
    const observer = new CallbackObserver(() => {
      const nameVal = nameRead.get();
      const ageVal = ageRead.get();
      results.push(`${nameVal}: ${ageVal}`);
    });

    // Trigger observer to establish subscriptions
    observer.withContext(() => {
      const nameVal = nameRead.get();
      const ageVal = ageRead.get();
      results.push(`init: ${nameVal}: ${ageVal}`);
    });

    expect(results).toEqual(['init: Alice: 25']);
    results.length = 0;

    // Change name - should trigger observer
    name.set('Bob');
    expect(results).toEqual(['Bob: 25']);
    results.length = 0;

    // Change age - should also trigger observer
    age.set(30);
    expect(results).toEqual(['Bob: 30']);
  });

  // Rust: async fn test_nested_observer_contexts()
  test('test_nested_observer_contexts', () => {
    const outerSignal = new Mut<string>('outer');
    const innerSignal = new Mut<string>('inner');
    const outerRead = outerSignal.read();
    const innerRead = innerSignal.read();

    const trackingLog: string[] = [];

    const outerObserver = new CallbackObserver(() => {
      trackingLog.push('outer callback');
    });

    const innerObserver = new CallbackObserver(() => {
      trackingLog.push('inner callback');
    });

    // Test nested context setup and restoration
    CurrentObserver.set(outerObserver.clone());
    expect(CurrentObserver.current()).not.toBeNull();

    // Access outer signal - should subscribe to outer observer
    outerRead.get();

    // Nest inner observer context
    CurrentObserver.set(innerObserver.clone());

    // Access inner signal - should subscribe to inner observer (not outer)
    innerRead.get();

    // Restore outer context
    CurrentObserver.pop();

    // Context should be restored to outer observer
    expect(CurrentObserver.current()).not.toBeNull();

    // Clean up
    CurrentObserver.pop();
    expect(CurrentObserver.current()).toBeNull();

    // Test that signal changes trigger correct observers
    outerSignal.set('outer_changed');
    innerSignal.set('inner_changed');

    expect(trackingLog).toContain('outer callback');
    expect(trackingLog).toContain('inner callback');
  });

  // Rust: async fn test_deep_nested_context_restoration()
  test('test_deep_nested_context_restoration', () => {
    const signals = Array.from({ length: 5 }, (_, i) => new Mut(i));
    const reads = signals.map((s) => s.read());

    const observers = reads.map((read) =>
      new CallbackObserver(() => {
        read.get(); // Subscribe this observer to this signal
      }),
    );

    // Build nested context stack: 0 -> 1 -> 2 -> 3 -> 4
    for (let i = 0; i < 5; i++) {
      CurrentObserver.set(observers[i].clone());
      reads[i].get(); // Subscribe each observer to its signal

      // Verify current context is correct
      expect(CurrentObserver.current()).not.toBeNull();
    }

    // Now unwind the stack - each pop should restore previous context
    for (let i = 4; i >= 0; i--) {
      CurrentObserver.pop();

      if (i > 0) {
        // Should still have a context (the previous one)
        expect(CurrentObserver.current()).not.toBeNull();
      } else {
        // Final pop should leave no context
        expect(CurrentObserver.current()).toBeNull();
      }
    }
  });

  // Rust: async fn test_observer_cleanup()
  test('test_observer_cleanup', () => {
    const signal = new Mut<string>('test');
    const readSignal = signal.read();

    let notificationCount = 0;

    const observer = new CallbackObserver(() => {
      readSignal.get(); // Re-subscribe on each notification
      notificationCount++;
    });

    // Establish subscription
    observer.withContext(() => {
      readSignal.get();
    });

    // Change signal - should trigger notification
    signal.set('changed1');
    expect(notificationCount).toBe(1);

    // Clear observer subscriptions
    observer.clear();

    // Change signal again - should NOT trigger notification
    signal.set('changed2');
    expect(notificationCount).toBe(1); // Should still be 1
  });

  // Rust: async fn test_context_subscription_clearing()
  test('test_context_subscription_clearing', () => {
    const signal1 = new Mut(1);
    const signal2 = new Mut(2);
    const read1 = signal1.read();
    const read2 = signal2.read();

    let notificationCount = 0;

    const observer = new CallbackObserver(() => {
      notificationCount++;
    });

    // First context - subscribe to signal1 only
    observer.withContext(() => {
      read1.get();
    });

    // Change signal1 - should trigger
    signal1.set(10);
    expect(notificationCount).toBe(1);

    // Second context - should clear previous subscriptions and subscribe to signal2
    observer.withContext(() => {
      read2.get();
    });

    // Change signal1 - should NOT trigger (subscription was cleared)
    signal1.set(20);
    expect(notificationCount).toBe(1); // Still 1

    // Change signal2 - should trigger
    signal2.set(30);
    expect(notificationCount).toBe(2);
  });

  // Rust: async fn test_react_style_try_finally_pattern()
  test('test_react_style_try_finally_pattern', () => {
    const signal = new Mut<string>('react_test');
    const readSignal = signal.read();

    const results: string[] = [];

    const observer = new CallbackObserver(() => {
      const value = readSignal.get();
      results.push(`react: ${value}`);
    });

    // Simulate React useObserve pattern
    const simulateReactComponent = (): string => {
      CurrentObserver.set(observer.clone());

      let result: string;
      try {
        // This is where React component would render
        const value = readSignal.get();
        result = `rendered: ${value}`;
      } finally {
        // This must happen even if the component throws
        CurrentObserver.pop();
      }

      return result;
    };

    // Initial render
    const renderResult = simulateReactComponent();
    expect(renderResult).toBe('rendered: react_test');
    expect(CurrentObserver.current()).toBeNull(); // Context should be cleaned up

    // Change signal - should trigger observer
    signal.set('updated');
    expect(results).toEqual(['react: updated']);
  });

  // Rust: async fn test_context_remove_pointer_equality()
  test('test_context_remove_pointer_equality', () => {
    const observer1 = new CallbackObserver(() => {});
    const observer2 = new CallbackObserver(() => {});
    const observer3 = new CallbackObserver(() => {});

    // Initially no context
    expect(CurrentObserver.current()).toBeNull();

    // Build a stack: observer1 -> observer2 -> observer3
    CurrentObserver.set(observer1.clone());
    CurrentObserver.set(observer2.clone());
    CurrentObserver.set(observer3.clone());

    // Should have 3 observers in stack, current should be observer3
    expect(CurrentObserver.current()).not.toBeNull();

    // Test 1: Remove the top observer (observer3) - should work like pop()
    CurrentObserver.remove(observer3);
    expect(CurrentObserver.current()).not.toBeNull(); // Should still have observer2

    // Test 2: Remove middle observer (observer1) from stack [observer1, observer2]
    CurrentObserver.remove(observer1);
    expect(CurrentObserver.current()).not.toBeNull(); // Should still have observer2

    // Test 3: Remove the last observer (observer2)
    CurrentObserver.remove(observer2);
    expect(CurrentObserver.current()).toBeNull(); // Stack should be empty

    // Test 4: Try to remove an observer that's not in the stack (should not crash)
    const observer4 = new CallbackObserver(() => {});
    CurrentObserver.remove(observer4); // Should be safe no-op
    expect(CurrentObserver.current()).toBeNull(); // Still empty

    // Test 5: Verify that cloning doesn't break observer identification
    CurrentObserver.set(observer1.clone());
    const observer1Clone = observer1.clone();

    // Both the original and clone should refer to the same observer
    expect(CurrentObserver.current()).not.toBeNull();
    CurrentObserver.remove(observer1Clone); // Remove using clone
    expect(CurrentObserver.current()).toBeNull(); // Should work

    // Test 6: Test removing from middle of larger stack
    CurrentObserver.set(observer1.clone());
    CurrentObserver.set(observer2.clone());
    CurrentObserver.set(observer3.clone());
    CurrentObserver.set(observer4.clone());

    // Stack is now: [observer1, observer2, observer3, observer4]
    // Remove observer2 from the middle
    CurrentObserver.remove(observer2);

    // Stack should now be: [observer1, observer3, observer4]
    // Current should still be observer4
    expect(CurrentObserver.current()).not.toBeNull();

    // Clean up remaining observers
    CurrentObserver.remove(observer4);
    CurrentObserver.remove(observer3);
    CurrentObserver.remove(observer1);
    expect(CurrentObserver.current()).toBeNull();
  });
});
