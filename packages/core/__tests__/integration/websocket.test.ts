// MIRRORS: ankurah/tests/tests/websocket.rs
//
// WebSocket integration tests: client-server fetch, create propagation,
// subscription propagation, and bidirectional subscription via WebSocket transport.
//
// All tests require WebsocketClient + WebsocketServer integration which is not yet
// available. The TS packages (@ankurah/connector-websocket, @ankurah/connector-websocket-server)
// exist but are not wired up for integration testing.

import { describe, test } from 'bun:test';

describe('websocket', () => {
  // Mirrors: websocket.rs test_websocket_client_server_fetch
  // Cannot enable: requires WebsocketServer + WebsocketClient integration test harness.
  test.skip('test_websocket_client_server_fetch', async () => {
    // Needs WebsocketServer running on a random port + WebsocketClient connecting to it.
  });

  // Mirrors: websocket.rs test_websocket_client_create_propagation
  // Cannot enable: requires WebsocketServer + WebsocketClient integration test harness.
  test.skip('test_websocket_client_create_propagation', async () => {
    // Needs WebsocketServer + WebsocketClient for client-to-server propagation.
  });

  // Mirrors: websocket.rs test_websocket_subscription_propagation
  // Cannot enable: requires WebsocketServer + WebsocketClient + LiveQuery subscription over WS.
  test.skip('test_websocket_subscription_propagation', async () => {
    // Needs WebsocketServer + WebsocketClient + subscription relay over WebSocket.
  });

  // Mirrors: websocket.rs test_websocket_bidirectional_subscription
  // Cannot enable: requires WebsocketServer + WebsocketClient + bidirectional subscription.
  test.skip('test_websocket_bidirectional_subscription', async () => {
    // Needs WebsocketServer + WebsocketClient + bidirectional subscription propagation.
  });
});
