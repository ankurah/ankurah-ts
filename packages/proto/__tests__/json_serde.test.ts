// TS-ONLY: the human-readable half of the provided types' serde.
//
// Rust's Serialize/Deserialize for EntityId and EventId branch on
// `serializer.is_human_readable()`: base64url without padding for JSON, raw bytes for
// bincode. The ULID-backed ids write their 26-char Crockford string in either format,
// and Clock is a newtype serde looks straight through. These tests pin the JSON strings
// the Rust tests assert (proto/src/id.rs and proto/src/data.rs, the
// `test_*_json_serialization` tests) and check that adding the JSON half left the
// bincode bytes alone.
//
// A parsed Result is read with isOk() and then unwrap(), never `using`: unwrap takes
// `self` in Rust, so it moves the Result rather than dropping it, and a moved value must
// not be dropped again.

import { describe, test, expect } from 'bun:test';
import {
  BincodeReader,
  BincodeWriter,
  Clock,
  EntityId,
  EventId,
  QueryId,
  RequestId,
  TransactionId,
  UpdateId,
} from '../src/index';
import { JsonError, Result } from '@ankurah/base';
import { DecodeError } from '../src/error';
import { ulidStringToBytes } from '../src/id.provided';

const ENTITY_BYTES = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
const ENTITY_JSON = '"AQIDBAUGBwgJCgsMDQ4PEA"';

const EVENT_BYTES = Array.from({ length: 32 }, (_, i) => i + 1);
const EVENT_JSON = '"AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA"';

// The ULID specification's own example value.
const ULID_STRING = '01ARZ3NDEKTSV4RRFFQ69G5FAV';

function encoded(value: { encode(w: BincodeWriter): void }): number[] {
  const writer = new BincodeWriter();
  value.encode(writer);
  return Array.from(writer.finish());
}

describe('json serialization — base64 ids', () => {
  // Rust: proto/src/id.rs, test_entity_id_json_serialization
  test('EntityId round-trips through the exact JSON Rust writes', () => {
    using id = EntityId.fromBytes(ENTITY_BYTES);
    const json = JSON.stringify(id);
    expect(json).toBe(ENTITY_JSON);

    const parsed = EntityId.fromJson(JSON.parse(json));
    expect(parsed.isOk()).toBe(true);
    using roundTrip = parsed.unwrap();
    expect(id.equals(roundTrip)).toBe(true);
  });

  // Rust: proto/src/data.rs, test_event_id_json_serialization
  test('EventId round-trips through the exact JSON Rust writes', () => {
    using id = EventId.fromBytes(EVENT_BYTES);
    const json = JSON.stringify(id);
    expect(json).toBe(EVENT_JSON);

    const parsed = EventId.fromJson(JSON.parse(json));
    expect(parsed.isOk()).toBe(true);
    using roundTrip = parsed.unwrap();
    expect(id.equals(roundTrip)).toBe(true);
  });

  test('an id nested in a struct serializes as the bare base64 string', () => {
    // A record standing in for a Rust struct with these fields. A generated struct is
    // not used here: its own newtype fields have no toJSON yet, so its JSON would differ
    // from Rust for reasons that have nothing to do with the ids.
    using entityId = EntityId.fromBytes(ENTITY_BYTES);
    using eventId = EventId.fromBytes(EVENT_BYTES);
    const record = { entity_id: entityId, head: eventId };

    expect(JSON.stringify(record)).toBe(
      `{"entity_id":${ENTITY_JSON},"head":${EVENT_JSON}}`,
    );
  });
});

describe('json serialization — ULID ids', () => {
  test('each ULID wrapper serializes as its 26-char Crockford string', () => {
    const bytes = ulidStringToBytes(ULID_STRING);
    using transaction = TransactionId.fromBytes(bytes);
    using request = RequestId.fromBytes(bytes);
    using query = QueryId.fromBytes(bytes);
    using update = UpdateId.fromBytes(bytes);

    for (const id of [transaction, request, query, update]) {
      expect(JSON.stringify(id)).toBe(`"${ULID_STRING}"`);
    }
    // toString is the short display form, and must not be what JSON gets.
    expect(transaction.toString()).not.toBe(ULID_STRING);
  });

  test('a ULID wrapper round-trips from its JSON string', () => {
    const parsed = TransactionId.fromJson(ULID_STRING);
    expect(parsed.isOk()).toBe(true);
    using id = parsed.unwrap();
    expect(JSON.stringify(id)).toBe(`"${ULID_STRING}"`);
  });
});

describe('json serialization — Clock', () => {
  test('Clock serializes as the array of its ids, not its display string', () => {
    using clock = Clock.new([
      EventId.fromBytes(EVENT_BYTES),
      EventId.fromBytes(Array.from({ length: 32 }, (_, i) => i + 33)),
    ]);

    // Not toBase64(), which is the bracketed comma-joined display string.
    const json = JSON.stringify(clock);
    expect(json).toBe(`[${EVENT_JSON},"ISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0-P0A"]`);

    const parsed = Clock.fromJson(JSON.parse(json));
    expect(parsed.isOk()).toBe(true);
    using roundTrip = parsed.unwrap();
    expect(clock.equals(roundTrip)).toBe(true);
  });
});

describe('json deserialization rejects what Rust rejects', () => {
  // A rejected parse fails with a JsonError, because a Deserialize impl fails with the
  // format's own error type: Rust writes `.map_err(serde::de::Error::custom)`, and
  // custom keeps the rendered text of the id's DecodeError and nothing else. So these
  // read the text rather than a kind. The error is a tracked value, and unwrapErr hands
  // it over, so each one is dropped where Rust drops it at the end of the scope.

  /** The message a rejected parse renders, with the error released. */
  function rejection(parsed: Result<unknown, JsonError>): string {
    expect(parsed.isErr()).toBe(true);
    const error = parsed.unwrapErr();
    const message = error.toString();
    error.drop();
    return message;
  }

  test('a non-string is NotStringValue', () => {
    for (const value of [42, null, true, {}, ['a']]) {
      expect(rejection(EntityId.fromJson(value))).toBe('Not a string value');
    }
  });

  test('base64 of the wrong byte count is InvalidLength', () => {
    // Valid base64url; decodes to 15 bytes, not 16.
    expect(rejection(EntityId.fromJson('AQIDBAUGBwgJCgsMDQ4P'))).toBe('Invalid Length');
  });

  test('a symbol outside the base64url alphabet is InvalidBase64', () => {
    // Standard-alphabet '+' and '/', and padding, are all outside URL_SAFE_NO_PAD.
    for (const value of ['AQIDBAUGBwgJCgsMDQ4P+A', 'AQIDBAUGBwgJCgsMDQ4P/A', 'AQIDBAUGBwgJCgsMDQ4PEA==']) {
      expect(rejection(EntityId.fromJson(value))).toStartWith('Invalid Base64');
    }
  });

  test('a string that is not 26 Crockford characters is InvalidUlid', () => {
    for (const value of ['01ARZ3NDEKTSV4RRFFQ69G5FA', '01ARZ3NDEKTSV4RRFFQ69G5FAU!']) {
      expect(rejection(TransactionId.fromJson(value))).toStartWith('Invalid ULID');
    }
  });

  test('a Clock element that fails carries the element error out', () => {
    expect(rejection(Clock.fromJson(['AQIDBAUGBwgJCgsMDQ4PEA']))).toBe('Invalid Length');
    expect(rejection(Clock.fromJson('AQIDBAUGBwgJCgsMDQ4PEA'))).toBe('Invalid Format');
  });

  test('the DecodeError kind is still reachable off the serde path', () => {
    // Rust's non-serde callers call `EntityId::from_base64` and match the DecodeError;
    // only the Deserialize impl converts, so the kind is not lost, just not in the Result.
    let caught: DecodeError | null = null;
    try {
      EntityId.fromBase64('AQIDBAUGBwgJCgsMDQ4P');
    } catch (e) {
      caught = e as DecodeError;
    }
    expect(caught).toBeInstanceOf(DecodeError);
    expect((caught as DecodeError).kind).toBe('InvalidLength');
  });
});

describe('bincode is unchanged by the JSON half', () => {
  // Rust: proto/src/id.rs, test_entity_id_bincode_serialization
  test('EntityId still encodes as its raw 16 bytes', () => {
    using id = EntityId.fromBytes(ENTITY_BYTES);
    const bytes = encoded(id);
    expect(bytes).toEqual(ENTITY_BYTES);

    using decoded = EntityId.decode(new BincodeReader(new Uint8Array(bytes)));
    expect(id.equals(decoded)).toBe(true);
  });

  // Rust: proto/src/data.rs, test_event_id_bincode_serialization
  test('EventId still encodes as its raw 32 bytes', () => {
    using id = EventId.fromBytes(EVENT_BYTES);
    const bytes = encoded(id);
    expect(bytes).toEqual(EVENT_BYTES);

    using decoded = EventId.decode(new BincodeReader(new Uint8Array(bytes)));
    expect(id.equals(decoded)).toBe(true);
  });

  test('a ULID wrapper still encodes as a length-prefixed 26-byte string', () => {
    using id = TransactionId.fromBytes(ulidStringToBytes(ULID_STRING));
    const bytes = encoded(id);
    expect(bytes.length).toBe(8 + 26);
    expect(bytes.slice(0, 8)).toEqual([26, 0, 0, 0, 0, 0, 0, 0]);
    expect(String.fromCharCode(...bytes.slice(8))).toBe(ULID_STRING);
  });

  test('Clock still encodes as a length prefix and its raw ids', () => {
    using clock = Clock.new([EventId.fromBytes(EVENT_BYTES)]);
    expect(encoded(clock)).toEqual([1, 0, 0, 0, 0, 0, 0, 0, ...EVENT_BYTES]);
  });
});
