// TS-ONLY: cross-language bincode fixture validation, driven by the sidecars.
//
// Every `.bin` under `proto/test_fixtures` is a sequence of bincode values with no
// framing, and its `.json` sidecar names each value, records the byte `offset` and
// `len` it occupies, and records what it must decode to. This test walks that
// manifest: nothing here names a fixture or a value, so a fixture added on the Rust
// side is picked up the next time this runs.
//
// Each item gets four checks, in this order:
//   1. the TypeScript decoder for the sidecar's type decodes the slice at `offset`
//   2. the reader has advanced exactly `len` — the check byte-equality cannot make,
//      since a decoder can recover every value and still consume the wrong count
//   3. the decoded value equals the sidecar's expected value, in serde's shape —
//      the check that catches a decoder which swaps two same-typed fields
//   4. re-encoding produces the fixture's bytes back
//
// Items carrying `debug` instead of `json` are the three non-finite `f64` cases,
// which serde_json turns into `null`; those get checks 1, 2 and 4 only.

import { afterAll, describe, test, expect } from 'bun:test';
import { existsSync } from 'fs';

import {
  BincodeReader,
  BincodeWriter,
  EntityId,
  EventId,
  TransactionId,
  RequestId,
  QueryId,
  UpdateId,
  CollectionId,
  Clock,
  AuthData,
  Attestation,
  AttestationSet,
  Attested,
  Principal,
  Operation,
  OperationSet,
  StateBuffers,
  State,
  EntityState,
  Event,
  EventFragment,
  StateFragment,
  NodeRequest,
  NodeRequestBody,
  NodeResponse,
  NodeResponseBody,
  NodeUpdate,
  NodeUpdateAck,
  NodeUpdateBody,
  NodeUpdateAckBody,
  UpdateContent,
  MembershipChange,
  SubscriptionUpdateItem,
  Presence,
  CausalRelation,
  CausalAssertion,
  CausalAssertionFragment,
  DeltaContent,
  EntityDelta,
  KnownEntity,
  Message,
  NodeMessage,
} from '../src/index';
import { Item } from '../src/sys';

import {
  Expr,
  InfixOperator,
  Literal,
  OrderByItem,
  OrderDirection,
  PathExpr,
  Predicate,
  Selection,
} from '@ankurah/ankql';

import { fixturePath, listFixtureDir, readFixtureBytes, readSidecar, toHex } from './support/fixtures';
import { toSerde } from './support/serde';

/**
 * Release a decoded value.
 *
 * Every decoded value here is owned by the test that decoded it, and the
 * ownership runtime tracks each one until something calls `drop()`. A few
 * sidecar types decode to a plain number or string, which owns nothing and has
 * no `drop`, so the call is guarded rather than written per type.
 */
function dropIfOwned(value: unknown): void {
  const owned = value as { drop?: () => void };
  if (owned && typeof owned.drop === 'function') owned.drop();
}

const FIXTURE_DIR = 'proto/test_fixtures';

// ── Type registry ───────────────────────────────────────────────────────────
//
// The sidecar's `type` names the Rust type *and*, for the gap fixtures, the edge
// case it probes — `Presence.durable`, `Literal::I16`, `EntityId ([u8; 16], no
// length prefix)`. The value on the wire is always the type at the head of that
// string, so the table maps the whole string to the port's codec for it. Nothing
// here implements a decoder: every entry calls the port's own `decode`/`encode`,
// and a type the port has not implemented fails loudly rather than being skipped.

interface Codec {
  decode(reader: BincodeReader): unknown;
  encode(writer: BincodeWriter, value: unknown): void;
}

function codecFor(name: string, T: any): Codec {
  return {
    decode(reader) {
      if (typeof T?.decode !== 'function') throw new Error(`${name}: the port has no static decode()`);
      return T.decode(reader);
    },
    encode(writer, value) {
      const v = value as { encode?: (w: BincodeWriter) => void };
      if (typeof v?.encode !== 'function') throw new Error(`${name}: the decoded value has no encode()`);
      v.encode(writer);
    },
  };
}

/** Attested<T> takes the payload's codec as a callback rather than knowing it. */
function attestedCodec(name: string, T: any): Codec {
  return {
    decode(reader) {
      if (typeof T?.decode !== 'function') throw new Error(`${name}: the port has no static decode()`);
      return Attested.decode(reader, (r: BincodeReader) => T.decode(r));
    },
    encode(writer, value) {
      (value as Attested<any>).encode(writer, (w: BincodeWriter, p: any) => p.encode(w));
    },
  };
}

const CODECS: Record<string, Codec> = {
  // Ids
  'EntityId': codecFor('EntityId', EntityId),
  'EntityId ([u8; 16], no length prefix)': codecFor('EntityId', EntityId),
  'EventId': codecFor('EventId', EventId),
  'EventId ([u8; 32], no length prefix)': codecFor('EventId', EventId),
  'EventId (= Event::id())': codecFor('EventId', EventId),
  'TransactionId': codecFor('TransactionId', TransactionId),
  'RequestId': codecFor('RequestId', RequestId),
  'QueryId': codecFor('QueryId', QueryId),
  'UpdateId': codecFor('UpdateId', UpdateId),
  'CollectionId': codecFor('CollectionId', CollectionId),

  // Clock and auth
  'Clock': codecFor('Clock', Clock),
  'AuthData': codecFor('AuthData', AuthData),
  'Attestation': codecFor('Attestation', Attestation),
  'AttestationSet': codecFor('AttestationSet', AttestationSet),
  'Principal': codecFor('Principal', Principal),
  'Attested<EntityState>': attestedCodec('EntityState', EntityState),
  'Attested<Event>': attestedCodec('Event', Event),

  // Data
  'Operation': codecFor('Operation', Operation),
  'Operation.diff (Vec<u8>, u64 length prefix)': codecFor('Operation', Operation),
  'OperationSet': codecFor('OperationSet', OperationSet),
  'StateBuffers': codecFor('StateBuffers', StateBuffers),
  'State': codecFor('State', State),
  'StateFragment': codecFor('StateFragment', StateFragment),
  'Event': codecFor('Event', Event),
  'EventFragment': codecFor('EventFragment', EventFragment),
  'EntityState': codecFor('EntityState', EntityState),

  // Request / response
  'NodeRequest': codecFor('NodeRequest', NodeRequest),
  'NodeRequestBody': codecFor('NodeRequestBody', NodeRequestBody),
  'NodeRequestBody::SubscribeQuery.version': codecFor('NodeRequestBody', NodeRequestBody),
  'NodeResponse': codecFor('NodeResponse', NodeResponse),
  'NodeResponseBody': codecFor('NodeResponseBody', NodeResponseBody),
  'NodeResponseBody::Error': codecFor('NodeResponseBody', NodeResponseBody),

  // Causality and deltas
  'CausalRelation': codecFor('CausalRelation', CausalRelation),
  'CausalAssertion': codecFor('CausalAssertion', CausalAssertion),
  'CausalAssertionFragment': codecFor('CausalAssertionFragment', CausalAssertionFragment),
  'DeltaContent': codecFor('DeltaContent', DeltaContent),
  'EntityDelta': codecFor('EntityDelta', EntityDelta),
  'KnownEntity': codecFor('KnownEntity', KnownEntity),

  // Updates
  'NodeUpdate': codecFor('NodeUpdate', NodeUpdate),
  'NodeUpdateBody': codecFor('NodeUpdateBody', NodeUpdateBody),
  'NodeUpdateAck': codecFor('NodeUpdateAck', NodeUpdateAck),
  'NodeUpdateAckBody': codecFor('NodeUpdateAckBody', NodeUpdateAckBody),
  'NodeUpdateAckBody::Error': codecFor('NodeUpdateAckBody', NodeUpdateAckBody),
  'SubscriptionUpdateItem': codecFor('SubscriptionUpdateItem', SubscriptionUpdateItem),
  'UpdateContent': codecFor('UpdateContent', UpdateContent),
  'MembershipChange': codecFor('MembershipChange', MembershipChange),

  // Messages, presence, sys
  'Message': codecFor('Message', Message),
  'NodeMessage': codecFor('NodeMessage', NodeMessage),
  'Presence': codecFor('Presence', Presence),
  'Presence.durable': codecFor('Presence', Presence),
  'sys::Item': codecFor('sys::Item', Item),

  // ankql AST, carried on the proto wire inside Fetch/SubscribeQuery selections
  'Literal': codecFor('Literal', Literal),
  'Literal (json_as_bytes)': codecFor('Literal', Literal),
  'PathExpr': codecFor('PathExpr', PathExpr),
  'Expr': codecFor('Expr', Expr),
  'InfixOperator': codecFor('InfixOperator', InfixOperator),
  'Predicate': codecFor('Predicate', Predicate),
  'Predicate::Comparison': codecFor('Predicate', Predicate),
  'OrderDirection': codecFor('OrderDirection', OrderDirection),
  'OrderByItem': codecFor('OrderByItem', OrderByItem),
  'Selection': codecFor('Selection', Selection),
  'ankql::ast::Selection': codecFor('Selection', Selection),
  'Selection.limit: Option<u64>': codecFor('Selection', Selection),
  'Literal::I16': codecFor('Selection', Selection),
  'Literal::I32': codecFor('Selection', Selection),
  'Literal::I64': codecFor('Selection', Selection),
};

// ── Manifest ────────────────────────────────────────────────────────────────

interface SidecarItem {
  label: string;
  type: string;
  offset: number;
  len: number;
  json?: unknown;
  debug?: string;
}

interface Sidecar {
  fixture: string;
  total_len: number;
  items: SidecarItem[];
}

const fixtures = listFixtureDir(FIXTURE_DIR)
  .filter((f) => f.endsWith('.bin'))
  .filter((f) => existsSync(fixturePath(FIXTURE_DIR, f.replace(/\.bin$/, '.json'))));

if (fixtures.length === 0) {
  throw new Error(`No .bin/.json fixture pairs under ${fixturePath(FIXTURE_DIR)}`);
}

for (const binName of fixtures) {
  const jsonName = binName.replace(/\.bin$/, '.json');
  const bytes = readFixtureBytes(FIXTURE_DIR, binName);
  const sidecar = readSidecar(FIXTURE_DIR, jsonName) as Sidecar;

  describe(binName, () => {
    test('file length matches the sidecar', () => {
      expect(bytes.length).toBe(sidecar.total_len);
    });

    for (const item of sidecar.items) {
      const slice = bytes.subarray(item.offset, item.offset + item.len);

      test(`${item.label} (${item.type})`, () => {
        const codec = CODECS[item.type];
        if (!codec) throw new Error(`No codec registered for sidecar type ${JSON.stringify(item.type)}`);

        // 1. decode from the item's offset
        const tail = bytes.subarray(item.offset);
        const reader = new BincodeReader(tail);
        const value = codec.decode(reader);

        // 2. the reader consumed exactly the bytes the sidecar says it should
        const consumed = tail.length - reader.remaining;
        expect(consumed).toBe(item.len);

        // 3. the decoded value is what the sidecar says it must be
        if ('json' in item) {
          expect(toSerde(value)).toEqual(item.json as any);
        }

        // 4. re-encoding reproduces the fixture bytes
        const writer = new BincodeWriter();
        codec.encode(writer, value);
        expect(toHex(writer.finish())).toBe(toHex(slice));

        // The decoded value is this test's, so this test releases it. Without
        // this the suite leaks one tracked value per item, which the registry
        // only says out loud once a collection happens — and a suite short
        // enough to finish before the first collection reports nothing.
        dropIfOwned(value);
      });
    }
  });
}

// ── Derived-value fixture ───────────────────────────────────────────────────
//
// `event_id_derivation.bin` holds four EventIds followed by the four Events they
// came from. `EventId::from_parts` is SHA-256 over the bincode of `entity_id`,
// `operations` and `parent` in that order, so recomputing each id from its event
// proves all three encodings at once — a check no single-value comparison makes.

describe('event_id_derivation.bin — Event::id() derivation', () => {
  const bytes = readFixtureBytes(FIXTURE_DIR, 'event_id_derivation.bin');
  const sidecar = readSidecar(FIXTURE_DIR, 'event_id_derivation.json') as Sidecar;

  const ids = sidecar.items.filter((i) => i.type === 'EventId (= Event::id())');
  const events = sidecar.items.filter((i) => i.type === 'Event');

  for (const idItem of ids) {
    const eventItem = events.find((e) => e.label === `${idItem.label}__input_event`);

    test(`${idItem.label}: Event::id() recomputes the fixture's EventId`, () => {
      if (!eventItem) throw new Error(`No input event for ${idItem.label}`);
      const event = Event.decode(new BincodeReader(bytes.subarray(eventItem.offset))) as Event;
      const id = event.id();
      expect(toSerde(id)).toEqual(idItem.json as any);
      dropIfOwned(id);
      dropIfOwned(event);
    });
  }

  test('collection is not part of the hash', () => {
    const standard = ids.find((i) => i.label === 'standard_event');
    const other = ids.find((i) => i.label === 'collection_is_not_hashed');
    if (!standard || !other) throw new Error('expected standard_event and collection_is_not_hashed items');
    expect(other.json).toEqual(standard.json as any);
  });
});

// ── The suite's own ownership check ──────────────────────────────────────────
//
// A leak is reported from a FinalizationRegistry, so nothing is said until a
// collection happens — and a suite that finishes in forty milliseconds can leak
// every value it decodes and still print nothing. Forcing a collection at the
// end is what makes this suite's silence worth something; it also means a leak
// introduced here is reported here rather than in whichever suite happens to
// run long enough afterwards to trigger the collector.
afterAll(() => {
  Bun.gc(true);
});
