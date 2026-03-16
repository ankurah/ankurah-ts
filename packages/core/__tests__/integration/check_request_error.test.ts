// MIRRORS: ankurah/tests/tests/check_request_error.rs
//
// Tests that check_request errors are properly propagated back to the client.
// This test ensures that when the server's PolicyAgent rejects a request via check_request,
// the client receives an error response rather than hanging indefinitely.
//
// ALL tests in this file require LocalProcessConnection (inter-node communication).

import { describe, test } from 'bun:test';

// ── Tests ──

describe('check_request_error', () => {
  // Mirrors: check_request_error_returns_to_client
  // Requires LocalProcessConnection which is not yet ported.
  // Also requires a custom RejectingAgent PolicyAgent that rejects all incoming requests
  // at the check_request stage.
  test.skip('check_request error returns to client (requires LocalProcessConnection)', () => {
    // Rust: let server = Node::new_durable(Arc::new(SledStorageEngine::new_test().unwrap()), RejectingAgent);
    // Rust: server.system.create().await?;
    // Rust: let client = Node::new(Arc::new(SledStorageEngine::new_test().unwrap()), PermissiveAgent::new());
    // Rust: let _conn = LocalProcessConnection::new(&server, &client).await?;
    // Rust: client.system.wait_system_ready().await;
    //
    // The test creates a custom RejectingAgent that implements PolicyAgent but always
    // returns Err(ValidationError::ValidationFailed(...)) from check_request.
    // It then connects a permissive client to the rejecting server and verifies that
    // trx.commit() returns an error (not hangs) when the server rejects the request.
    //
    // Dependencies:
    // - @ankurah/connector-local (LocalProcessConnection)
    // - Custom PolicyAgent with check_request rejection
  });
});
