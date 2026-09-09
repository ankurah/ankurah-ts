// MIRRORS: ankurah/core/src/peer_subscription/client_relay.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Status, SubscriptionRelay } from './client_relay';
import { AnyhowError, Arc, Mutex, Result, Struct, dropOwned } from '@ankurah/base';
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
    let _moved1 = false;
    const _b0 = Arc.new(new Mutex([]));
    try {
      let _moved3 = false;
      const _b2 = Arc.new(new Mutex(null));
      try {
        let _moved5 = false;
        const _b4 = Arc.new(new Mutex(false));
        try {
          const _b6 = Arc.new(new Mutex(''));
          _moved1 = true;
          _moved3 = true;
          _moved5 = true;
          return new MockMessageSender(_b2, _b0, _b4, _b6, undefined /* PhantomData */);
        } finally {
          if (!_moved5) dropOwned(_b4);
        }
      } finally {
        if (!_moved3) dropOwned(_b2);
      }
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
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
          let _moved2 = false;
          const _b1 = collectionId.clone();
          try {
            const _b3 = selection.clone();
            _moved2 = true;
            _t0.value.push([peerId, queryId, _b1, _b3]);
          } finally {
            if (!_moved2) dropOwned(_b1);
          }
        } finally {
          _t0.drop();
        }
        const _t4 = this.nextError.value.lock();
        try {
          {
            const _v = _t4.value.take();
            if (_v != null) {
              const error = _v;
              return Result.Err(new RetrievalError('RequestError', { _0: error }));
            } else {
            return Result.Ok([]);
          }
          }
        } finally {
          _t4.drop();
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
    try {
      const mockSender = Arc.new(MockMessageSender.new());
      try {
        relay.setNode(mockSender.clone()).expect('Failed to set message sender');
        const queryId = proto.QueryId.new();
        const collectionId = createTestCollectionId();
        try {
          const predicate = createTestSelection();
          try {
            const peerId = EntityId.new();
            relay.notifyPeerConnected(peerId);
            let _moved1 = false;
            const _b0 = collectionId.clone();
            try {
              let _moved3 = false;
              const _b2 = predicate.clone();
              try {
                let _moved5 = false;
                const _b4 = collectionId.clone();
                try {
                  const _b6 = new MockLiveQuery();
                  _moved1 = true;
                  _moved3 = true;
                  _moved5 = true;
                  relay.subscribeQuery(queryId, _b0, _b2, _b4, 0, _b6);
                } finally {
                  if (!_moved5) dropOwned(_b4);
                }
              } finally {
                if (!_moved3) dropOwned(_b2);
              }
            } finally {
              if (!_moved1) dropOwned(_b0);
            }
            if (!(((_v) => {
              if (!(_v != null && (_v.is('Requested')))) return false;
              return true;
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            const sentRequests = mockSender.value.getSentRequests();
            try {
              expect(sentRequests.length).toEqual(1);
              expect(sentRequests[0][0]).toEqual(peerId);
              expect(sentRequests[0][1]).toEqual(queryId);
              expect(sentRequests[0][2]).toEqual(collectionId);
              if (!(((_v1) => {
                if (!(_v1 != null && (_v1.is('Established')))) return false;
                const { _0: establishedPeerId } = _v1.value;
                return establishedPeerId.equals(peerId);
              })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            } finally {
              dropOwned(sentRequests);
            }
          } finally {
            predicate.drop();
          }
        } finally {
          collectionId.drop();
        }
      } finally {
        mockSender.drop();
      }
    } finally {
      relay.drop();
    }
  });

  test('test_peer_disconnection_orphans_subscriptions', async () => {
    const relay = SubscriptionRelay.new();
    try {
      const mockSender = Arc.new(MockMessageSender.new());
      try {
        relay.setNode(mockSender.clone()).expect('Failed to set message sender');
        const queryId = proto.QueryId.new();
        const collectionId = createTestCollectionId();
        try {
          let _moved0 = false;
          const predicate = createTestSelection();
          try {
            const peerId = EntityId.new();
            relay.notifyPeerConnected(peerId);
            let _moved2 = false;
            const _b1 = collectionId.clone();
            try {
              let _moved4 = false;
              const _b3 = collectionId.clone();
              try {
                const _b5 = new MockLiveQuery();
                _moved2 = true;
                _moved4 = true;
                _moved0 = true;
                relay.subscribeQuery(queryId, _b1, predicate, _b3, 0, _b5);
              } finally {
                if (!_moved4) dropOwned(_b3);
              }
            } finally {
              if (!_moved2) dropOwned(_b1);
            }
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            if (!(((_v) => {
              if (!(_v != null && (_v.is('Established')))) return false;
              const { _0: establishedPeerId } = _v.value;
              return establishedPeerId.equals(peerId);
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            relay.notifyPeerDisconnected(peerId);
            if (!(((_v1) => {
              if (!(_v1 != null && (_v1.is('PendingRemote')))) return false;
              return true;
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
          } finally {
            if (!_moved0) predicate.drop();
          }
        } finally {
          collectionId.drop();
        }
      } finally {
        mockSender.drop();
      }
    } finally {
      relay.drop();
    }
  });

  test('test_peer_connection_triggers_setup', async () => {
    const relay = SubscriptionRelay.new();
    try {
      const mockSender = Arc.new(MockMessageSender.new());
      try {
        relay.setNode(mockSender.clone()).expect('Failed to set message sender');
        const queryId = proto.QueryId.new();
        const collectionId = createTestCollectionId();
        try {
          const predicate = createTestSelection();
          try {
            const peerId = EntityId.new();
            let _moved1 = false;
            const _b0 = collectionId.clone();
            try {
              let _moved3 = false;
              const _b2 = predicate.clone();
              try {
                let _moved5 = false;
                const _b4 = collectionId.clone();
                try {
                  const _b6 = new MockLiveQuery();
                  _moved1 = true;
                  _moved3 = true;
                  _moved5 = true;
                  relay.subscribeQuery(queryId, _b0, _b2, _b4, 0, _b6);
                } finally {
                  if (!_moved5) dropOwned(_b4);
                }
              } finally {
                if (!_moved3) dropOwned(_b2);
              }
            } finally {
              if (!_moved1) dropOwned(_b0);
            }
            if (!(((_v) => {
              if (!(_v != null && (_v.is('PendingRemote')))) return false;
              return true;
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            mockSender.value.clearSentRequests();
            relay.notifyPeerConnected(peerId);
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            const sentRequests = mockSender.value.getSentRequests();
            try {
              expect(sentRequests.length).toEqual(1);
              expect(sentRequests[0][0]).toEqual(peerId);
              expect(sentRequests[0][1]).toEqual(queryId);
              if (!(((_v1) => {
                if (!(_v1 != null && (_v1.is('Established')))) return false;
                const { _0: establishedPeerId } = _v1.value;
                return establishedPeerId.equals(peerId);
              })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            } finally {
              dropOwned(sentRequests);
            }
          } finally {
            predicate.drop();
          }
        } finally {
          collectionId.drop();
        }
      } finally {
        mockSender.drop();
      }
    } finally {
      relay.drop();
    }
  });

  test('test_failed_subscription_retry', async () => {
    const relay = SubscriptionRelay.new();
    try {
      const mockSender = Arc.new(MockMessageSender.new());
      try {
        relay.setNode(mockSender.clone()).expect('Failed to set message sender');
        const queryId = proto.QueryId.new();
        const collectionId = createTestCollectionId();
        try {
          const predicate = createTestSelection();
          try {
            const peerId = EntityId.new();
            relay.notifyPeerConnected(peerId);
            let _moved1 = false;
            const _b0 = collectionId.clone();
            try {
              let _moved3 = false;
              const _b2 = predicate.clone();
              try {
                let _moved5 = false;
                const _b4 = collectionId.clone();
                try {
                  const _b6 = new MockLiveQuery();
                  _moved1 = true;
                  _moved3 = true;
                  _moved5 = true;
                  relay.subscribeQuery(queryId, _b0, _b2, _b4, 0, _b6);
                } finally {
                  if (!_moved5) dropOwned(_b4);
                }
              } finally {
                if (!_moved3) dropOwned(_b2);
              }
            } finally {
              if (!_moved1) dropOwned(_b0);
            }
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            if (!(((_v) => {
              if (!(_v != null && (_v.is('Established')))) return false;
              const { _0: establishedPeerId } = _v.value;
              return establishedPeerId.equals(peerId);
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            relay.notifyPeerDisconnected(peerId);
            if (!(((_v1) => {
              if (!(_v1 != null && (_v1.is('PendingRemote')))) return false;
              return true;
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            mockSender.value.clearSentRequests();
            mockSender.value.setFailNext(new RequestError('ServerError', { _0: 'Invalid predicate' }));
            relay.notifyPeerConnected(peerId);
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            const sentRequests = mockSender.value.getSentRequests();
            try {
              expect(sentRequests.length).toEqual(1);
              if (!(((_v2) => {
                if (!(_v2 != null && (_v2.is('Failed')))) return false;
                return true;
              })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            } finally {
              dropOwned(sentRequests);
            }
          } finally {
            predicate.drop();
          }
        } finally {
          collectionId.drop();
        }
      } finally {
        mockSender.drop();
      }
    } finally {
      relay.drop();
    }
  });

  test('test_retryable_vs_non_retryable_failures', async () => {
    const relay = SubscriptionRelay.new();
    try {
      const mockSender = Arc.new(MockMessageSender.new());
      try {
        relay.setNode(mockSender.clone()).expect('Failed to set message sender');
        const retryableQueryId = proto.QueryId.new();
        const nonRetryableQueryId = proto.QueryId.new();
        const collectionId = createTestCollectionId();
        try {
          const predicate = createTestSelection();
          try {
            const peerId = EntityId.new();
            let _moved1 = false;
            const _b0 = collectionId.clone();
            try {
              let _moved3 = false;
              const _b2 = predicate.clone();
              try {
                let _moved5 = false;
                const _b4 = collectionId.clone();
                try {
                  const _b6 = new MockLiveQuery();
                  _moved1 = true;
                  _moved3 = true;
                  _moved5 = true;
                  relay.subscribeQuery(retryableQueryId, _b0, _b2, _b4, 0, _b6);
                } finally {
                  if (!_moved5) dropOwned(_b4);
                }
              } finally {
                if (!_moved3) dropOwned(_b2);
              }
            } finally {
              if (!_moved1) dropOwned(_b0);
            }
            let _moved8 = false;
            const _b7 = collectionId.clone();
            try {
              let _moved10 = false;
              const _b9 = predicate.clone();
              try {
                let _moved12 = false;
                const _b11 = collectionId.clone();
                try {
                  const _b13 = new MockLiveQuery();
                  _moved8 = true;
                  _moved10 = true;
                  _moved12 = true;
                  relay.subscribeQuery(nonRetryableQueryId, _b7, _b9, _b11, 0, _b13);
                } finally {
                  if (!_moved12) dropOwned(_b11);
                }
              } finally {
                if (!_moved10) dropOwned(_b9);
              }
            } finally {
              if (!_moved8) dropOwned(_b7);
            }
            (() => {
              let subscriptions = relay.inner.value.subscriptions.lock().unwrapOrElse((e) => e.intoInner());
              try {
                {
                  const _v = subscriptions.value.get(retryableQueryId);
                  if (_v != null) {
                    const info = _v;
                    const _a14 = new Status('PendingRemote', {});
                    info.status.drop();
                    info.status = _a14;
                  }
                }
                {
                  const _v1 = subscriptions.value.get(nonRetryableQueryId);
                  if (_v1 != null) {
                    const info = _v1;
                    const _a15 = new Status('Failed', {});
                    info.status.drop();
                    info.status = _a15;
                  }
                }
              } finally {
                subscriptions.drop();
              }
            })();
            relay.notifyPeerConnected(peerId);
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            const sentRequests = mockSender.value.getSentRequests();
            try {
              expect(sentRequests.length).toEqual(1);
              expect(sentRequests[0][1]).toEqual(retryableQueryId);
              if (!(((_v2) => {
                if (!(_v2 != null && (_v2.is('Established')))) return false;
                const { _0: establishedPeerId } = _v2.value;
                return establishedPeerId.equals(peerId);
              })(relay.getStatus(retryableQueryId)))) throw new Error('assertion failed');
              if (!(((_v3) => {
                if (!(_v3 != null && (_v3.is('Failed')))) return false;
                return true;
              })(relay.getStatus(nonRetryableQueryId)))) throw new Error('assertion failed');
            } finally {
              dropOwned(sentRequests);
            }
          } finally {
            predicate.drop();
          }
        } finally {
          collectionId.drop();
        }
      } finally {
        mockSender.drop();
      }
    } finally {
      relay.drop();
    }
  });

  test('test_subscription_removal', async () => {
    const relay = SubscriptionRelay.new();
    try {
      const mockSender = Arc.new(MockMessageSender.new());
      try {
        relay.setNode(mockSender.clone()).expect('Failed to set message sender');
        const queryId = proto.QueryId.new();
        const collectionId = createTestCollectionId();
        try {
          let _moved0 = false;
          const predicate = createTestSelection();
          try {
            const peerId = EntityId.new();
            relay.notifyPeerConnected(peerId);
            let _moved2 = false;
            const _b1 = collectionId.clone();
            try {
              let _moved4 = false;
              const _b3 = collectionId.clone();
              try {
                const _b5 = new MockLiveQuery();
                _moved2 = true;
                _moved4 = true;
                _moved0 = true;
                relay.subscribeQuery(queryId, _b1, predicate, _b3, 0, _b5);
              } finally {
                if (!_moved4) dropOwned(_b3);
              }
            } finally {
              if (!_moved2) dropOwned(_b1);
            }
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            if (!(((_v) => {
              if (!(_v != null && (_v.is('Established')))) return false;
              const { _0: establishedPeerId } = _v.value;
              return establishedPeerId.equals(peerId);
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            mockSender.value.clearSentRequests();
            relay.unsubscribePredicate(queryId);
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            const sentRequests = mockSender.value.getSentRequests();
            try {
              expect(sentRequests.length).toEqual(1);
              expect(sentRequests[0][0]).toEqual(peerId);
              expect(sentRequests[0][1]).toEqual(queryId);
              if (!(((_v1) => {
                if (!(_v1 == null)) return false;
                return true;
              })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            } finally {
              dropOwned(sentRequests);
            }
          } finally {
            if (!_moved0) predicate.drop();
          }
        } finally {
          collectionId.drop();
        }
      } finally {
        mockSender.drop();
      }
    } finally {
      relay.drop();
    }
  });

  test('test_edge_cases', async () => {
    const relay = SubscriptionRelay.new();
    try {
      const mockSender = Arc.new(MockMessageSender.new());
      try {
        const queryId = proto.QueryId.new();
        const collectionId = createTestCollectionId();
        try {
          const predicate = createTestSelection();
          try {
            const peerId = EntityId.new();
            let _moved1 = false;
            const _b0 = collectionId.clone();
            try {
              let _moved3 = false;
              const _b2 = predicate.clone();
              try {
                let _moved5 = false;
                const _b4 = collectionId.clone();
                try {
                  const _b6 = new MockLiveQuery();
                  _moved1 = true;
                  _moved3 = true;
                  _moved5 = true;
                  relay.subscribeQuery(queryId, _b0, _b2, _b4, 0, _b6);
                } finally {
                  if (!_moved5) dropOwned(_b4);
                }
              } finally {
                if (!_moved3) dropOwned(_b2);
              }
            } finally {
              if (!_moved1) dropOwned(_b0);
            }
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            if (!(((_v) => {
              if (!(_v != null && (_v.is('PendingRemote')))) return false;
              return true;
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            relay.setNode(mockSender.clone()).expect('Failed to set message sender');
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            if (!(((_v1) => {
              if (!(_v1 != null && (_v1.is('PendingRemote')))) return false;
              return true;
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            const _t7 = mockSender.value.getSentRequests();
            try {
              expect(_t7.length).toEqual(0);
            } finally {
              dropOwned(_t7);
            }
            relay.notifyPeerConnected(peerId);
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            if (!(((_v2) => {
              if (!(_v2 != null && (_v2.is('Established')))) return false;
              const { _0: establishedPeerId } = _v2.value;
              return establishedPeerId.equals(peerId);
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            const _t8 = mockSender.value.getSentRequests();
            try {
              expect(_t8.length).toEqual(1);
            } finally {
              dropOwned(_t8);
            }
          } finally {
            predicate.drop();
          }
        } finally {
          collectionId.drop();
        }
      } finally {
        mockSender.drop();
      }
    } finally {
      relay.drop();
    }
  });

  test('test_notify_unsubscribe_with_no_established_subscription', async () => {
    const relay = SubscriptionRelay.new();
    try {
      const mockSender = Arc.new(MockMessageSender.new());
      try {
        relay.setNode(mockSender.clone()).expect('Failed to set message sender');
        const queryId = proto.QueryId.new();
        const collectionId = createTestCollectionId();
        try {
          let _moved0 = false;
          const predicate = createTestSelection();
          try {
            let _moved2 = false;
            const _b1 = collectionId.clone();
            try {
              let _moved4 = false;
              const _b3 = collectionId.clone();
              try {
                const _b5 = new MockLiveQuery();
                _moved2 = true;
                _moved4 = true;
                _moved0 = true;
                relay.subscribeQuery(queryId, _b1, predicate, _b3, 0, _b5);
              } finally {
                if (!_moved4) dropOwned(_b3);
              }
            } finally {
              if (!_moved2) dropOwned(_b1);
            }
            if (!(((_v) => {
              if (!(_v != null && (_v.is('PendingRemote')))) return false;
              return true;
            })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            relay.unsubscribePredicate(queryId);
            await futuresTimer.Delay.new(time.Duration.fromMillis(10n));
            const sentRequests = mockSender.value.getSentRequests();
            try {
              expect(sentRequests.length).toEqual(0);
              if (!(((_v1) => {
                if (!(_v1 == null)) return false;
                return true;
              })(relay.getStatus(queryId)))) throw new Error('assertion failed');
            } finally {
              dropOwned(sentRequests);
            }
          } finally {
            if (!_moved0) predicate.drop();
          }
        } finally {
          collectionId.drop();
        }
      } finally {
        mockSender.drop();
      }
    } finally {
      relay.drop();
    }
  });

});
