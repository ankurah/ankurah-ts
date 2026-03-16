// MIRRORS: ankurah/core/src/peer_subscription/client_relay.rs #[cfg(test)]

import { describe, expect, test, afterEach } from 'bun:test';
import { EntityId, QueryId, CollectionId } from '@ankurah/proto';
import { Predicate, Selection } from '@ankurah/ankql';

import type { TNode, RemoteQuerySubscriber, Status } from '../src/peer_subscription/client_relay.ts';
import { SubscriptionRelay } from '../src/peer_subscription/client_relay.ts';
import { RequestError, RetrievalError } from '../src/error.ts';

// ── Helpers ──────────────────────────────────────────────────────────────

// Note: Some tests call setupRemoteSubscriptions() directly to test the core
// subscription setup logic in isolation, while others use notifyPeerConnected()
// to test the full event-driven flow. Both approaches are valuable:
// - Direct calls test the setup mechanism itself (error handling, state transitions)
// - Event-driven calls test the integration and user-facing API

// For testing, we'll use CollectionId as our ContextData
// Rust: impl ContextData for CollectionId {}

/// Delay helper — replaces Rust futures_timer::Delay
function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/// Mock message sender for testing
/// Rust: struct MockMessageSender<CD: ContextData>
class MockMessageSender implements TNode<CollectionId> {
  private nextError: RequestError | null = null;
  private sentRequests: Array<{
    peerId: EntityId;
    queryId: QueryId;
    collectionId: CollectionId;
    selection: Selection;
  }> = [];

  // Rust: fn set_fail_next(&self, error: RequestError)
  setFailNext(error: RequestError): void {
    this.nextError = error;
  }

  // Rust: fn get_sent_requests(&self)
  getSentRequests(): Array<{
    peerId: EntityId;
    queryId: QueryId;
    collectionId: CollectionId;
    selection: Selection;
  }> {
    return [...this.sentRequests];
  }

  // Rust: fn clear_sent_requests(&self)
  clearSentRequests(): void {
    this.sentRequests = [];
  }

  // impl TNode<CollectionId>
  async remoteSubscribe(
    peerId: EntityId,
    queryId: QueryId,
    collectionId: CollectionId,
    selection: Selection,
    _contextData: CollectionId,
    _version: number,
  ): Promise<void> {
    this.sentRequests.push({ peerId, queryId, collectionId, selection });

    // Check if there's an error to fail with
    if (this.nextError !== null) {
      const error = this.nextError;
      this.nextError = null;
      throw new RetrievalError('RequestError', `Request error: ${error.message}`, error);
    }
    // Mock successful subscription (fetch, subscribe, apply, store all succeeded)
  }

  async peerUnsubscribe(peerId: EntityId, queryId: QueryId): Promise<void> {
    this.sentRequests.push({
      peerId,
      queryId,
      collectionId: CollectionId.from('unsubscribe'),
      selection: new Selection(Predicate.True(), null, null),
    });

    // Check if there's an error to fail with
    if (this.nextError !== null) {
      const error = this.nextError;
      this.nextError = null;
      throw new Error(String(error));
    }
  }
}

/// Mock implementation of RemoteQuerySubscriber for tests
/// Rust: struct MockLiveQuery
const MockLiveQuery: RemoteQuerySubscriber = {
  async subscriptionEstablished(_version: number): Promise<void> {
    // Mock - no-op
  },
  setLastError(_error: RetrievalError): void {
    // For tests, we don't track errors
  },
};

function createTestSelection(): Selection {
  // Create a simple test predicate
  return new Selection(Predicate.True(), null, null);
}

function createTestCollectionId(): CollectionId {
  return CollectionId.from('test_collection');
}

// =========================================================================
// SubscriptionRelay tests
// MIRRORS: ankurah/core/src/peer_subscription/client_relay.rs #[cfg(test)]
// =========================================================================

describe('SubscriptionRelay', () => {
  let relay: SubscriptionRelay<CollectionId, RemoteQuerySubscriber>;

  afterEach(() => {
    // Clean up the retry interval
    if (relay) {
      relay.destroy();
    }
  });

  test('test_new_subscription_setup', async () => {
    relay = new SubscriptionRelay();
    const mockSender = new MockMessageSender();
    relay.setNode(mockSender);

    const queryId = QueryId.new();
    const collectionId = createTestCollectionId();
    const selection = createTestSelection();
    const peerId = EntityId.new();

    // Connect the peer first
    relay.notifyPeerConnected(peerId);

    // Notify of new subscription
    relay.subscribeQuery(queryId, collectionId, selection, collectionId, 0, MockLiveQuery);

    // Check initial state - subscription should immediately go to Requested state since peer is connected
    const status = relay.getStatus(queryId);
    expect(status).not.toBeNull();
    expect(status!.type).toBe('Requested');

    // Give async task time to complete (setup should happen automatically)
    await delay(10);

    // Verify request was sent
    const sentRequests = mockSender.getSentRequests();
    expect(sentRequests.length).toBe(1);
    expect(sentRequests[0].peerId.equals(peerId)).toBe(true);
    expect(sentRequests[0].queryId.equals(queryId)).toBe(true);
    expect(sentRequests[0].collectionId.equals(collectionId)).toBe(true);

    // Verify subscription is marked as established
    const finalStatus = relay.getStatus(queryId);
    expect(finalStatus).not.toBeNull();
    expect(finalStatus!.type).toBe('Established');
    if (finalStatus!.type === 'Established') {
      expect(finalStatus!.peerId.equals(peerId)).toBe(true);
    }
  });

  test('test_peer_disconnection_orphans_subscriptions', async () => {
    relay = new SubscriptionRelay();
    const mockSender = new MockMessageSender();
    relay.setNode(mockSender);

    const queryId = QueryId.new();
    const collectionId = createTestCollectionId();
    const selection = createTestSelection();
    const peerId = EntityId.new();

    // Connect the peer first
    relay.notifyPeerConnected(peerId);

    // Setup established subscription by going through the full flow
    relay.subscribeQuery(queryId, collectionId, selection, collectionId, 0, MockLiveQuery);

    // Give async task time to complete
    await delay(10);

    const status1 = relay.getStatus(queryId);
    expect(status1).not.toBeNull();
    expect(status1!.type).toBe('Established');
    if (status1!.type === 'Established') {
      expect(status1!.peerId.equals(peerId)).toBe(true);
    }

    // Simulate peer disconnection
    relay.notifyPeerDisconnected(peerId);

    // Verify subscription is marked as pending again
    const status2 = relay.getStatus(queryId);
    expect(status2).not.toBeNull();
    expect(status2!.type).toBe('PendingRemote');
  });

  test('test_peer_connection_triggers_setup', async () => {
    relay = new SubscriptionRelay();
    const mockSender = new MockMessageSender();
    relay.setNode(mockSender);

    const queryId = QueryId.new();
    const collectionId = createTestCollectionId();
    const selection = createTestSelection();
    const peerId = EntityId.new();

    // Add pending subscription (no peers connected yet)
    relay.subscribeQuery(queryId, collectionId, selection, collectionId, 0, MockLiveQuery);
    const status1 = relay.getStatus(queryId);
    expect(status1).not.toBeNull();
    expect(status1!.type).toBe('PendingRemote');

    // Clear any previous requests
    mockSender.clearSentRequests();

    // Simulate peer connection (should trigger automatic setup)
    relay.notifyPeerConnected(peerId);

    // Give async task time to complete
    await delay(10);

    // Verify request was sent
    const sentRequests = mockSender.getSentRequests();
    expect(sentRequests.length).toBe(1);
    expect(sentRequests[0].peerId.equals(peerId)).toBe(true);
    expect(sentRequests[0].queryId.equals(queryId)).toBe(true);

    // Verify subscription is established
    const status2 = relay.getStatus(queryId);
    expect(status2).not.toBeNull();
    expect(status2!.type).toBe('Established');
    if (status2!.type === 'Established') {
      expect(status2!.peerId.equals(peerId)).toBe(true);
    }
  });

  test('test_failed_subscription_retry', async () => {
    relay = new SubscriptionRelay();
    const mockSender = new MockMessageSender();
    relay.setNode(mockSender);

    const queryId = QueryId.new();
    const collectionId = createTestCollectionId();
    const selection = createTestSelection();
    const peerId = EntityId.new();

    // Connect peer and add subscription (should succeed initially)
    relay.notifyPeerConnected(peerId);
    relay.subscribeQuery(queryId, collectionId, selection, collectionId, 0, MockLiveQuery);

    // Give async task time to complete
    await delay(10);

    // Verify subscription is marked as established (since no error was set)
    const status1 = relay.getStatus(queryId);
    expect(status1).not.toBeNull();
    expect(status1!.type).toBe('Established');
    if (status1!.type === 'Established') {
      expect(status1!.peerId.equals(peerId)).toBe(true);
    }

    // Now test the retry behavior by disconnecting the peer (puts subscription back to PendingRemote)
    // then setting up the mock to fail, and reconnecting to trigger the retry
    relay.notifyPeerDisconnected(peerId);

    // Verify subscription is now in pending state
    const status2 = relay.getStatus(queryId);
    expect(status2).not.toBeNull();
    expect(status2!.type).toBe('PendingRemote');

    // Clear requests and set up mock to fail on the next call
    mockSender.clearSentRequests();
    mockSender.setFailNext(RequestError.serverError('Invalid predicate'));

    // Reconnect peer to trigger retry attempt
    relay.notifyPeerConnected(peerId);

    // Give async task time to complete
    await delay(10);

    // Verify retry was attempted (the error gets consumed)
    const sentRequests = mockSender.getSentRequests();
    expect(sentRequests.length).toBe(1);

    // Verify subscription remains in failed state (non-retryable error)
    const status3 = relay.getStatus(queryId);
    expect(status3).not.toBeNull();
    expect(status3!.type).toBe('Failed');
  });

  test('test_retryable_vs_non_retryable_failures', async () => {
    relay = new SubscriptionRelay();
    const mockSender = new MockMessageSender();
    relay.setNode(mockSender);

    const retryableQueryId = QueryId.new();
    const nonRetryableQueryId = QueryId.new();
    const collectionId = createTestCollectionId();
    const selection = createTestSelection();
    const peerId = EntityId.new();

    // Add subscriptions
    relay.subscribeQuery(retryableQueryId, collectionId, selection, collectionId, 0, MockLiveQuery);
    relay.subscribeQuery(nonRetryableQueryId, collectionId, selection, collectionId, 0, MockLiveQuery);

    // Divergence: Rust accesses relay.inner.subscriptions directly; TS has no inner field.
    // We use the public getStatus() to verify initial state, then rely on the state machine
    // behavior: PendingRemote subscriptions get picked up by setupRemoteSubscriptions,
    // Failed ones do not.
    // The subscriptions start as PendingRemote (no peers connected), so retryable is already correct.
    // We need to set the non-retryable one to Failed state.
    // Workaround: subscribe, connect peer to establish, disconnect, set fail, reconnect for non-retryable.
    // But Rust test directly mutates state. We'll do the equivalent by:
    // 1. Both are PendingRemote (correct for retryable)
    // 2. We need non-retryable to be Failed — connect a peer with fail set for the non-retryable query.
    // Actually the Rust test just directly sets status. Let's keep it simple and expose a test helper.

    // Simpler approach: both start as PendingRemote. Connect peer — both will attempt subscribe.
    // Set the mock to fail with non-retryable error for the first call (non-retryable query),
    // then succeed for the second (retryable query).
    // BUT: Map iteration order may not be deterministic.

    // Best approach matching Rust semantics: use internal state manipulation.
    // Since subscriptions Map is private, we'll test the observable behavior differently.
    // We'll establish both, then disconnect, then set only non-retryable to Failed via a failed reconnect.

    // Actually, let's just test the state machine behavior from the outside:
    // 1. Connect a peer, fail non_retryable with ServerError (non-retryable), succeed retryable
    // Since both are PendingRemote, both will be picked up. We can only set one error at a time.

    // Alternative: Use two separate rounds:
    // Round 1: Connect peer, both subscribe, both succeed -> both Established
    // Round 2: Disconnect, both go PendingRemote
    // Round 3: Set fail for non-retryable query with ServerError, reconnect
    //   - But we can't target which query gets the error

    // Simplest faithful approach: expose subscriptions for testing via a test accessor.
    // For now, just verify the core semantic: only PendingRemote gets retried, Failed does not.

    // Start fresh: add retryable as PendingRemote, manually force non-retryable to Failed.
    // We'll use a two-step approach:
    // Step 1: Subscribe non-retryable, connect peer with fail set -> it becomes Failed
    mockSender.setFailNext(RequestError.serverError('permanent failure'));
    relay.notifyPeerConnected(peerId);

    // Give async task time to complete
    await delay(10);

    // One of the two subscriptions got the error. Check which one.
    // Since Map ordering may vary, let's verify and adjust.
    const retryableStatus = relay.getStatus(retryableQueryId);
    const nonRetryableStatus = relay.getStatus(nonRetryableQueryId);

    // Both were attempted. One failed (got the error), one succeeded.
    // The test verifies that Failed subscriptions are NOT retried on reconnect.
    // We need to know which one failed to set up the second round correctly.

    if (nonRetryableStatus!.type === 'Failed') {
      // Good — non-retryable is Failed, retryable is Established
      // Disconnect to put retryable back to PendingRemote
      relay.notifyPeerDisconnected(peerId);

      // retryable should be PendingRemote, non-retryable should still be Failed
      expect(relay.getStatus(retryableQueryId)!.type).toBe('PendingRemote');
      expect(relay.getStatus(nonRetryableQueryId)!.type).toBe('Failed');

      // Clear requests
      mockSender.clearSentRequests();

      // Reconnect — only retryable (PendingRemote) should be attempted
      relay.notifyPeerConnected(peerId);
      await delay(10);

      const sentRequests = mockSender.getSentRequests();
      expect(sentRequests.length).toBe(1);
      expect(sentRequests[0].queryId.equals(retryableQueryId)).toBe(true);

      // Verify states
      expect(relay.getStatus(retryableQueryId)!.type).toBe('Established');
      expect(relay.getStatus(nonRetryableQueryId)!.type).toBe('Failed');
    } else {
      // retryable got the error, non-retryable is Established.
      // Swap roles: disconnect to put non-retryable back to PendingRemote.
      // The retryable is Failed. This still tests the same semantic.
      relay.notifyPeerDisconnected(peerId);

      expect(relay.getStatus(nonRetryableQueryId)!.type).toBe('PendingRemote');
      expect(relay.getStatus(retryableQueryId)!.type).toBe('Failed');

      mockSender.clearSentRequests();
      relay.notifyPeerConnected(peerId);
      await delay(10);

      const sentRequests = mockSender.getSentRequests();
      expect(sentRequests.length).toBe(1);
      expect(sentRequests[0].queryId.equals(nonRetryableQueryId)).toBe(true);

      // Non-retryable should now be established, retryable stays Failed
      expect(relay.getStatus(nonRetryableQueryId)!.type).toBe('Established');
      expect(relay.getStatus(retryableQueryId)!.type).toBe('Failed');
    }
  });

  test('test_subscription_removal', async () => {
    relay = new SubscriptionRelay();
    const mockSender = new MockMessageSender();
    relay.setNode(mockSender);

    const queryId = QueryId.new();
    const collectionId = createTestCollectionId();
    const selection = createTestSelection();
    const peerId = EntityId.new();

    // Connect peer and setup established subscription
    relay.notifyPeerConnected(peerId);
    relay.subscribeQuery(queryId, collectionId, selection, collectionId, 0, MockLiveQuery);

    // Give async task time to complete
    await delay(10);

    const status1 = relay.getStatus(queryId);
    expect(status1).not.toBeNull();
    expect(status1!.type).toBe('Established');
    if (status1!.type === 'Established') {
      expect(status1!.peerId.equals(peerId)).toBe(true);
    }

    // Clear previous requests to focus on unsubscribe
    mockSender.clearSentRequests();

    // Remove subscription
    relay.unsubscribePredicate(queryId);

    // Give async task time to complete
    await delay(10);

    // Verify unsubscribe message was sent
    const sentRequests = mockSender.getSentRequests();
    expect(sentRequests.length).toBe(1);
    expect(sentRequests[0].peerId.equals(peerId)).toBe(true);
    expect(sentRequests[0].queryId.equals(queryId)).toBe(true);

    // Verify subscription is gone
    expect(relay.getStatus(queryId)).toBeNull();
  });

  test('test_edge_cases', async () => {
    relay = new SubscriptionRelay();
    const mockSender = new MockMessageSender();

    const queryId = QueryId.new();
    const collectionId = createTestCollectionId();
    const selection = createTestSelection();
    const peerId = EntityId.new();

    // Test setup without message sender - should not crash
    relay.subscribeQuery(queryId, collectionId, selection, collectionId, 0, MockLiveQuery);
    await delay(10);

    // Should still be pending since no sender
    const status1 = relay.getStatus(queryId);
    expect(status1).not.toBeNull();
    expect(status1!.type).toBe('PendingRemote');

    // Now set sender and test with no connected peers
    relay.setNode(mockSender);
    await delay(10);

    // Should still be pending since no peers available
    const status2 = relay.getStatus(queryId);
    expect(status2).not.toBeNull();
    expect(status2!.type).toBe('PendingRemote');

    // Verify no requests were sent
    expect(mockSender.getSentRequests().length).toBe(0);

    // Now connect a peer (should trigger automatic setup)
    relay.notifyPeerConnected(peerId);
    await delay(10);

    // Should now be established
    const status3 = relay.getStatus(queryId);
    expect(status3).not.toBeNull();
    expect(status3!.type).toBe('Established');
    if (status3!.type === 'Established') {
      expect(status3!.peerId.equals(peerId)).toBe(true);
    }
    expect(mockSender.getSentRequests().length).toBe(1);
  });

  test('test_notify_unsubscribe_with_no_established_subscription', async () => {
    relay = new SubscriptionRelay();
    const mockSender = new MockMessageSender();
    relay.setNode(mockSender);

    const queryId = QueryId.new();
    const collectionId = createTestCollectionId();
    const selection = createTestSelection();

    // Add subscription but don't establish it
    relay.subscribeQuery(queryId, collectionId, selection, collectionId, 0, MockLiveQuery);
    const status1 = relay.getStatus(queryId);
    expect(status1).not.toBeNull();
    expect(status1!.type).toBe('PendingRemote');

    // Unsubscribe from pending subscription
    relay.unsubscribePredicate(queryId);

    // Give async task time to complete (though no request should be sent)
    await delay(10);

    // Verify no unsubscribe message was sent (since it wasn't established)
    const sentRequests = mockSender.getSentRequests();
    expect(sentRequests.length).toBe(0);

    // Verify subscription is gone
    expect(relay.getStatus(queryId)).toBeNull();
  });
});
