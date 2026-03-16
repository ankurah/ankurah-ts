// MIRRORS: ankurah/connectors/websocket-server/src/state.rs

import type { SendError } from '@ankurah/core';
import { Message, BincodeWriter } from '@ankurah/proto';
import { WebSocketClientSender, type WsSendFn } from './sender.ts';

// Divergence: Rust uses axum::extract::ws::WebSocket + SplitSink.
// TS uses a WsSendFn callback for both initial and established states [E8].

export type Connection =
  | { type: 'Initial'; send: WsSendFn | null }
  | { type: 'Established'; sender: WebSocketClientSender };

export function connectionInitial(send: WsSendFn): Connection {
  return { type: 'Initial', send };
}

export function connectionEstablished(sender: WebSocketClientSender): Connection {
  return { type: 'Established', sender };
}

/// Send a proto::Message through the connection.
/// Mirrors Rust `Connection::send()`.
export function connectionSend(conn: Connection, message: Message): void {
  switch (conn.type) {
    case 'Initial': {
      if (conn.send !== null) {
        const writer = new BincodeWriter();
        message.encode(writer);
        const data = writer.finish();
        conn.send(data);
      } else {
        throw new Error('Connection send function is null');
      }
      break;
    }
    case 'Established': {
      conn.sender.sendProtoMessage(message);
      break;
    }
  }
}
