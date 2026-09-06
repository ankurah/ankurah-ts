// MIRRORS: ankurah/core/src/peer_subscription/client_relay.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Status, SubscriptionRelay } from './client_relay';
import { AnyhowError, Arc, Mutex, Result, Struct, valueEquals } from '@ankurah/base';
import { RequestError } from '../error';
import { Predicate, Selection } from '@ankurah/ankql';
import { CollectionId, EntityId, QueryId } from '@ankurah/proto';

class MockMessageSender<CD extends ContextData> extends Struct implements TNode<CD> {
  nextError: Arc<Mutex<RequestError | null>>;
  sentRequests: Arc<Mutex<[EntityId, QueryId, CollectionId, Selection][]>>;
  shouldFail: Arc<Mutex<boolean>>;
  failureMessage: Arc<Mutex<string>>;

  constructor(nextError: Arc<Mutex<RequestError | null>>, sentRequests: Arc<Mutex<[EntityId, QueryId, CollectionId, Selection][]>>, shouldFail: Arc<Mutex<boolean>>, failureMessage: Arc<Mutex<string>>) {
    super();
    this.nextError = nextError;
    this.sentRequests = sentRequests;
    this.shouldFail = shouldFail;
    this.failureMessage = failureMessage;
  }

  static new<CD>(): MockMessageSender<CD> {
    return new MockMessageSender(Arc.new(new Mutex(null)), Arc.new(new Mutex([])), Arc.new(new Mutex(false)), Arc.new(new Mutex('')), undefined /* PhantomData */);
  }

  setFailNext(error: RequestError): void {
    const _t0 = this.nextError.value.lock();
    try {
      _t0.value = error;
    } finally {
      _t0.drop();
    }
  }

  getSentRequests(): [EntityId, QueryId, CollectionId, Selection][] {
    const _t0 = this.sentRequests.value.lock();
    try {
      return _t0.value.map((e) => [e[0].clone(), e[1].clone(), e[2].clone(), e[3].clone()] as [EntityId, QueryId, CollectionId, Selection]);
    } finally {
      _t0.drop();
    }
  }

  clearSentRequests(): void {
    const _t0 = this.sentRequests.value.lock();
    try {
      _t0.value.length = 0;
    } finally {
      _t0.drop();
    }
  }

  async remoteSubscribe(peerId: EntityId, queryId: QueryId, collectionId: CollectionId, selection: Selection, _contextData: CD, _version: number): Promise<Result<void, RetrievalError>> {
    try {
      try {
        const _t0 = this.sentRequests.value.lock();
        try {
          _t0.value.push([peerId, queryId, collectionId.clone(), selection.clone()]);
        } finally {
          _t0.drop();
        }
        const _t1 = this.nextError.value.lock();
        try {
          {
            const _v = _t1.value.take();
            if (_v != null) {
              const error = _v;
              return Result.Err(new RetrievalError('RequestError', { _0: error }));
            } else {
            return Result.Ok([]);
          }
          }
        } finally {
          _t1.drop();
        }
      } finally {
        selection.drop();
      }
    } finally {
      collectionId.drop();
    }
  }

  async peerUnsubscribe(peerId: EntityId, queryId: QueryId): Promise<Result<void, AnyhowError>> {
    const _t0 = this.sentRequests.value.lock();
    try {
      _t0.value.push([peerId, queryId, CollectionId.from('unsubscribe'), new Selection(new Predicate('True', {}), null, null)]);
    } finally {
      _t0.drop();
    }
    const _t1 = this.nextError.value.lock();
    try {
      {
        const _v = _t1.value.take();
        if (_v != null) {
          const error = _v;
          try {
            return Result.Err(AnyhowError.from(error.toString()));
          } finally {
            error.drop();
          }
        } else {
        return Result.Ok([]);
      }
      }
    } finally {
      _t1.drop();
    }
  }

  debug(): string {
    return `MockMessageSender { nextError: ${this.nextError}, sentRequests: ${this.sentRequests}, shouldFail: ${this.shouldFail}, failureMessage: ${this.failureMessage}, _phantom: ${this._phantom} }`;
  }
}

class MockLiveQuery extends Struct implements RemoteQuerySubscriber {

  async subscriptionEstablished(_version: number): Promise<void> {

  }

  setLastError(_error: RetrievalError): void {
    _error.drop();
  }

  clone(): MockLiveQuery {
    return new MockLiveQuery();
  }
}

describe('client_relay unit tests', () => {
  function createTestSelection(): Selection {
    return new Selection(new Predicate('True', {}), null, null);
  }

  function createTestCollectionId(): CollectionId {
    return CollectionId.from('test_collection');
  }

  test('test_new_subscription_setup', async () => {
    const relay = SubscriptionRelay.new();
    const mockSender = Arc.new(MockMessageSender.new());
    relay.setNode(mockSender.clone());
    const queryId = proto.QueryId.new();
    const collectionId = createTestCollectionId();
    try {
      const predicate = createTestSelection();
      try {
        const peerId = EntityId.new();
        relay.notifyPeerConnected(peerId);
        relay.subscribeQuery(queryId, collectionId.clone(), predicate.clone(), collectionId.clone(), 0, MockLiveQuery);
        if (!(((_v) => {
          if (!(_v != null && (_v.is('Requested')))) return false;
          return true;
        })(relay.getStatus(queryId)))) throw new Error('assertion failed');
        await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
        const sentRequests = mockSender.getSentRequests();
        expect(sentRequests.length).toEqual(1);
        expect(sentRequests[0]._0).toEqual(peerId);
        expect(sentRequests[0]._1).toEqual(queryId);
        expect(sentRequests[0]._2).toEqual(collectionId);
        if (!(((_v1) => {
          if (!(_v1 != null && (_v1.is('Established')))) return false;
          const { _0: establishedPeerId } = _v1.value;
          return valueEquals(establishedPeerId, peerId);
        })(relay.getStatus(queryId)))) throw new Error('assertion failed');
      } finally {
        predicate.drop();
      }
    } finally {
      collectionId.drop();
    }
  });

  test('test_peer_disconnection_orphans_subscriptions', async () => {
    const relay = SubscriptionRelay.new();
    const mockSender = Arc.new(MockMessageSender.new());
    relay.setNode(mockSender.clone());
    const queryId = proto.QueryId.new();
    const collectionId = createTestCollectionId();
    try {
      const predicate = createTestSelection();
      const peerId = EntityId.new();
      relay.notifyPeerConnected(peerId);
      relay.subscribeQuery(queryId, collectionId.clone(), predicate, collectionId.clone(), 0, MockLiveQuery);
      await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
      if (!(((_v) => {
        if (!(_v != null && (_v.is('Established')))) return false;
        const { _0: establishedPeerId } = _v.value;
        return valueEquals(establishedPeerId, peerId);
      })(relay.getStatus(queryId)))) throw new Error('assertion failed');
      relay.notifyPeerDisconnected(peerId);
      if (!(((_v1) => {
        if (!(_v1 != null && (_v1.is('PendingRemote')))) return false;
        return true;
      })(relay.getStatus(queryId)))) throw new Error('assertion failed');
    } finally {
      collectionId.drop();
    }
  });

  test('test_peer_connection_triggers_setup', async () => {
    const relay = SubscriptionRelay.new();
    const mockSender = Arc.new(MockMessageSender.new());
    relay.setNode(mockSender.clone());
    const queryId = proto.QueryId.new();
    const collectionId = createTestCollectionId();
    try {
      const predicate = createTestSelection();
      try {
        const peerId = EntityId.new();
        relay.subscribeQuery(queryId, collectionId.clone(), predicate.clone(), collectionId.clone(), 0, MockLiveQuery);
        if (!(((_v) => {
          if (!(_v != null && (_v.is('PendingRemote')))) return false;
          return true;
        })(relay.getStatus(queryId)))) throw new Error('assertion failed');
        mockSender.clearSentRequests();
        relay.notifyPeerConnected(peerId);
        await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
        const sentRequests = mockSender.getSentRequests();
        expect(sentRequests.length).toEqual(1);
        expect(sentRequests[0]._0).toEqual(peerId);
        expect(sentRequests[0]._1).toEqual(queryId);
        if (!(((_v1) => {
          if (!(_v1 != null && (_v1.is('Established')))) return false;
          const { _0: establishedPeerId } = _v1.value;
          return valueEquals(establishedPeerId, peerId);
        })(relay.getStatus(queryId)))) throw new Error('assertion failed');
      } finally {
        predicate.drop();
      }
    } finally {
      collectionId.drop();
    }
  });

  test('test_failed_subscription_retry', async () => {
    const relay = SubscriptionRelay.new();
    const mockSender = Arc.new(MockMessageSender.new());
    relay.setNode(mockSender.clone());
    const queryId = proto.QueryId.new();
    const collectionId = createTestCollectionId();
    try {
      const predicate = createTestSelection();
      try {
        const peerId = EntityId.new();
        relay.notifyPeerConnected(peerId);
        relay.subscribeQuery(queryId, collectionId.clone(), predicate.clone(), collectionId.clone(), 0, MockLiveQuery);
        await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
        if (!(((_v) => {
          if (!(_v != null && (_v.is('Established')))) return false;
          const { _0: establishedPeerId } = _v.value;
          return valueEquals(establishedPeerId, peerId);
        })(relay.getStatus(queryId)))) throw new Error('assertion failed');
        relay.notifyPeerDisconnected(peerId);
        if (!(((_v1) => {
          if (!(_v1 != null && (_v1.is('PendingRemote')))) return false;
          return true;
        })(relay.getStatus(queryId)))) throw new Error('assertion failed');
        mockSender.clearSentRequests();
        mockSender.setFailNext(new RequestError('ServerError', { _0: 'Invalid predicate' }));
        relay.notifyPeerConnected(peerId);
        await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
        const sentRequests = mockSender.getSentRequests();
        expect(sentRequests.length).toEqual(1);
        if (!(((_v2) => {
          if (!(_v2 != null && (_v2.is('Failed')))) return false;
          return true;
        })(relay.getStatus(queryId)))) throw new Error('assertion failed');
      } finally {
        predicate.drop();
      }
    } finally {
      collectionId.drop();
    }
  });

  test('test_retryable_vs_non_retryable_failures', async () => {
    const relay = SubscriptionRelay.new();
    const mockSender = Arc.new(MockMessageSender.new());
    relay.setNode(mockSender.clone());
    const retryableQueryId = proto.QueryId.new();
    const nonRetryableQueryId = proto.QueryId.new();
    const collectionId = createTestCollectionId();
    try {
      const predicate = createTestSelection();
      try {
        const peerId = EntityId.new();
        relay.subscribeQuery(retryableQueryId, collectionId.clone(), predicate.clone(), collectionId.clone(), 0, MockLiveQuery);
        relay.subscribeQuery(nonRetryableQueryId, collectionId.clone(), predicate.clone(), collectionId.clone(), 0, MockLiveQuery);
        (() => {
          let subscriptions = relay.inner.subscriptions.lock().unwrapOrElse((e) => e.intoInner());
          {
            const _v = subscriptions.get(retryableQueryId);
            if (_v != null) {
              const info = _v;
              info.status = new Status('PendingRemote', {});
            }
          }
          {
            const _v1 = subscriptions.get(nonRetryableQueryId);
            if (_v1 != null) {
              const info = _v1;
              info.status = new Status('Failed', {});
            }
          }
        })();
        relay.notifyPeerConnected(peerId);
        await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
        const sentRequests = mockSender.getSentRequests();
        expect(sentRequests.length).toEqual(1);
        expect(sentRequests[0]._1).toEqual(retryableQueryId);
        if (!(((_v2) => {
          if (!(_v2 != null && (_v2.is('Established')))) return false;
          const { _0: establishedPeerId } = _v2.value;
          return valueEquals(establishedPeerId, peerId);
        })(relay.getStatus(retryableQueryId)))) throw new Error('assertion failed');
        if (!(((_v3) => {
          if (!(_v3 != null && (_v3.is('Failed')))) return false;
          return true;
        })(relay.getStatus(nonRetryableQueryId)))) throw new Error('assertion failed');
      } finally {
        predicate.drop();
      }
    } finally {
      collectionId.drop();
    }
  });

  test('test_subscription_removal', async () => {
    const relay = SubscriptionRelay.new();
    const mockSender = Arc.new(MockMessageSender.new());
    relay.setNode(mockSender.clone());
    const queryId = proto.QueryId.new();
    const collectionId = createTestCollectionId();
    try {
      const predicate = createTestSelection();
      const peerId = EntityId.new();
      relay.notifyPeerConnected(peerId);
      relay.subscribeQuery(queryId, collectionId.clone(), predicate, collectionId.clone(), 0, MockLiveQuery);
      await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
      if (!(((_v) => {
        if (!(_v != null && (_v.is('Established')))) return false;
        const { _0: establishedPeerId } = _v.value;
        return valueEquals(establishedPeerId, peerId);
      })(relay.getStatus(queryId)))) throw new Error('assertion failed');
      mockSender.clearSentRequests();
      relay.unsubscribePredicate(queryId);
      await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
      const sentRequests = mockSender.getSentRequests();
      expect(sentRequests.length).toEqual(1);
      expect(sentRequests[0]._0).toEqual(peerId);
      expect(sentRequests[0]._1).toEqual(queryId);
      if (!(((_v1) => {
        if (!(_v1 == null)) return false;
        return true;
      })(relay.getStatus(queryId)))) throw new Error('assertion failed');
    } finally {
      collectionId.drop();
    }
  });

  test('test_edge_cases', async () => {
    const relay = SubscriptionRelay.new();
    const mockSender = Arc.new(MockMessageSender.new());
    const queryId = proto.QueryId.new();
    const collectionId = createTestCollectionId();
    try {
      const predicate = createTestSelection();
      try {
        const peerId = EntityId.new();
        relay.subscribeQuery(queryId, collectionId.clone(), predicate.clone(), collectionId.clone(), 0, MockLiveQuery);
        await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
        if (!(((_v) => {
          if (!(_v != null && (_v.is('PendingRemote')))) return false;
          return true;
        })(relay.getStatus(queryId)))) throw new Error('assertion failed');
        relay.setNode(mockSender.clone());
        await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
        if (!(((_v1) => {
          if (!(_v1 != null && (_v1.is('PendingRemote')))) return false;
          return true;
        })(relay.getStatus(queryId)))) throw new Error('assertion failed');
        expect(mockSender.getSentRequests().length).toEqual(0);
        relay.notifyPeerConnected(peerId);
        await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
        if (!(((_v2) => {
          if (!(_v2 != null && (_v2.is('Established')))) return false;
          const { _0: establishedPeerId } = _v2.value;
          return valueEquals(establishedPeerId, peerId);
        })(relay.getStatus(queryId)))) throw new Error('assertion failed');
        expect(mockSender.getSentRequests().length).toEqual(1);
      } finally {
        predicate.drop();
      }
    } finally {
      collectionId.drop();
    }
  });

  test('test_notify_unsubscribe_with_no_established_subscription', async () => {
    const relay = SubscriptionRelay.new();
    const mockSender = Arc.new(MockMessageSender.new());
    relay.setNode(mockSender.clone());
    const queryId = proto.QueryId.new();
    const collectionId = createTestCollectionId();
    try {
      const predicate = createTestSelection();
      relay.subscribeQuery(queryId, collectionId.clone(), predicate, collectionId.clone(), 0, MockLiveQuery);
      if (!(((_v) => {
        if (!(_v != null && (_v.is('PendingRemote')))) return false;
        return true;
      })(relay.getStatus(queryId)))) throw new Error('assertion failed');
      relay.unsubscribePredicate(queryId);
      await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
      const sentRequests = mockSender.getSentRequests();
      expect(sentRequests.length).toEqual(0);
      if (!(((_v1) => {
        if (!(_v1 == null)) return false;
        return true;
      })(relay.getStatus(queryId)))) throw new Error('assertion failed');
    } finally {
      collectionId.drop();
    }
  });

});
