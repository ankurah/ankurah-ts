// MIRRORS: ankurah/core/tests/backend_fixtures.rs
//
// The proto fixtures pin the envelope: at the proto layer a `StateBuffers` value and
// an `Operation.diff` are opaque byte vectors and nothing checks what is inside them.
// These fixtures pin the inside — the encodings the LWW property backend produces,
// and the proto `EntityState` and `Event` assembled from them.
//
// Each fixture gets three kinds of check:
//   - decode: the TypeScript backend must recover the sidecar's `decoded` /
//     `decoded_data` property map from the fixture's bytes
//   - replay: driving the TypeScript backend through the sidecar's `scenario` must
//     produce those bytes back, byte for byte
//   - envelope: an LWW operation diff is `LWWDiff { version: u8, data: Vec<u8> }`,
//     so `version` is one byte and the nested buffer's length prefix starts at
//     offset 1. A port that widens `version` to u32 shifts everything after it.
//
// The scenarios below are the Rust call sequences from each sidecar, transcribed to
// the TypeScript API. The sidecar's own `scenario` array is asserted against the
// number of steps so a scenario that changes on the Rust side is noticed here.

import { describe, test, expect } from 'bun:test';

import { LWWBackend } from '../src/property/backend/lww';
import { Entity } from '../src/entity';
import type { Value } from '../src/value/index';
import {
  BincodeReader,
  BincodeWriter,
  CollectionId,
  EntityId,
  EntityState,
  Event,
  Operation,
  OperationSet,
} from '@ankurah/proto';

import { readFixtureBytes, readSidecar, toHex } from '../../proto/__tests__/support/fixtures';
import { propertyMapToSerde, toSerde } from '../../proto/__tests__/support/serde';

const CORE_DIR = 'core/test_fixtures';

/** The Rust helper `entity_id(seed)`: sixteen bytes, each `seed + i`. */
function entityId(seed: number): EntityId {
  const bytes = new Uint8Array(16);
  for (let i = 0; i < 16; i++) bytes[i] = (seed + i) & 0xff;
  return EntityId.fromBytes(bytes);
}

const NON_ASCII_KEY = '名前';
const NON_ASCII_VALUE = 'café 日本語 🚀 مرحبا';

function str(value: string): Value { return { type: 'String', value }; }
function i32(value: number): Value { return { type: 'I32', value }; }

/** The single diff a `to_operations()` call must produce, or a loud failure. */
function singleDiff(backend: LWWBackend): Uint8Array {
  const ops = backend.toOperations();
  if (ops === null) throw new Error('to_operations() returned null where the fixture has an operation');
  if (ops.length !== 1) throw new Error(`to_operations() returned ${ops.length} operations; LWW batches into one`);
  return ops[0].diff;
}

/** Read an LWW operation diff back into the property map it carries. */
function decodeDiff(diff: Uint8Array): Record<string, unknown> {
  const backend = new LWWBackend();
  backend.applyOperations([new Operation(diff)]);
  return propertyMapToSerde(backend.propertyValues());
}

// ── LWW fixtures ────────────────────────────────────────────────────────────

interface LwwScenario {
  /** How many steps the sidecar's `scenario` array has, so a Rust-side change shows up. */
  steps: number;
  produce(): Uint8Array;
}

const LWW_SCENARIOS: Record<string, LwwScenario> = {
  'empty_state.bin': {
    steps: 1,
    produce: () => new LWWBackend().toStateBuffer(),
  },

  'state_after_first_set.bin': {
    steps: 3,
    produce: () => {
      const b = new LWWBackend();
      b.set('name', str('Alice'));
      b.set('count', i32(1));
      return b.toStateBuffer();
    },
  },

  'op_first_set.bin': {
    steps: 4,
    produce: () => {
      const b = new LWWBackend();
      b.set('name', str('Alice'));
      b.set('count', i32(1));
      return singleDiff(b);
    },
  },

  'state_after_second_set.bin': {
    steps: 3,
    produce: () => {
      const b = new LWWBackend();
      b.set('name', str('Alice'));
      b.set('count', i32(1));
      b.toOperations();
      b.set('count', i32(2));
      b.set('cleared', null);
      return b.toStateBuffer();
    },
  },

  'op_second_set.bin': {
    steps: 4,
    produce: () => {
      const b = new LWWBackend();
      b.set('name', str('Alice'));
      b.set('count', i32(1));
      b.toOperations();
      b.set('count', i32(2));
      b.set('cleared', null);
      return singleDiff(b);
    },
  },

  'op_idempotent_set.bin': {
    steps: 4,
    produce: () => {
      const b = new LWWBackend();
      b.set('k', i32(1));
      b.toOperations();
      b.set('k', i32(1)); // the same value again
      return singleDiff(b);
    },
  },

  'all_value_types.bin': {
    steps: 1,
    produce: () => {
      const b = new LWWBackend();
      // Insertion order deliberately differs from sorted order.
      b.set('v_i16_min', { type: 'I16', value: -32768 });
      b.set('v_i16_max', { type: 'I16', value: 32767 });
      b.set('v_i32_min', i32(-2147483648));
      b.set('v_i32_max', i32(2147483647));
      b.set('v_i64_min', { type: 'I64', value: -9223372036854775808 });
      b.set('v_i64_max', { type: 'I64', value: 9223372036854775807 });
      b.set('v_i64_beyond_js_safe', { type: 'I64', value: 9007199254740993 });
      b.set('v_f64_zero', { type: 'F64', value: 0.0 });
      b.set('v_f64_neg_zero', { type: 'F64', value: -0.0 });
      b.set('v_f64_fraction', { type: 'F64', value: 0.1 + 0.2 });
      b.set('v_f64_min', { type: 'F64', value: -Number.MAX_VALUE });
      b.set('v_f64_max', { type: 'F64', value: Number.MAX_VALUE });
      b.set('v_bool_true', { type: 'Bool', value: true });
      b.set('v_bool_false', { type: 'Bool', value: false });
      b.set('v_string', str('hello'));
      b.set('v_string_empty', str(''));
      b.set('v_entity_id', { type: 'EntityId', value: entityId(0x00) });
      b.set('v_object', { type: 'Object', value: new Uint8Array([0xde, 0xad, 0xbe, 0xef]) });
      b.set('v_object_empty', { type: 'Object', value: new Uint8Array([]) });
      b.set('v_binary', { type: 'Binary', value: new Uint8Array([0x00, 0xff]) });
      b.set('v_json', { type: 'Json', value: { a: 1, b: [true, null] } });
      b.set('v_none', null);
      return b.toStateBuffer();
    },
  },

  'non_ascii.bin': {
    steps: 4,
    produce: () => {
      const b = new LWWBackend();
      b.set(NON_ASCII_KEY, str(NON_ASCII_VALUE));
      // A precomposed key paired with a decomposed value: a port that normalizes
      // either one produces different bytes.
      b.set('caf\u00e9', str('cafe\u0301'));
      b.set('\u{1F680}', str('\u{1F30D}'));
      b.set('a\u0000b', str('x\u0000y'));
      return b.toStateBuffer();
    },
  },
};

for (const [binName, scenario] of Object.entries(LWW_SCENARIOS)) {
  const rel = `lww/${binName}`;
  const bytes = readFixtureBytes(CORE_DIR, rel);
  const sidecar = readSidecar(CORE_DIR, rel.replace(/\.bin$/, '.json')) as any;
  const isOperation = 'envelope' in sidecar;

  describe(rel, () => {
    test('file length matches the sidecar', () => {
      expect(bytes.length).toBe(sidecar.total_len);
      expect(sidecar.scenario.length).toBe(scenario.steps);
    });

    test('the TypeScript backend decodes the sidecar values', () => {
      if (isOperation) {
        expect(decodeDiff(bytes)).toEqual(sidecar.decoded_data);
      } else {
        expect(propertyMapToSerde(LWWBackend.fromStateBuffer(bytes).propertyValues())).toEqual(sidecar.decoded);
      }
    });

    test('replaying the scenario produces the fixture bytes', () => {
      expect(toHex(scenario.produce())).toBe(toHex(bytes));
    });

    if (isOperation) {
      test('the produced diff has the LWWDiff envelope', () => {
        const produced = scenario.produce();
        const env = sidecar.envelope;
        // version is a bare u8 at offset 0 — one byte, not four.
        expect(produced[env.version_offset]).toBe(env.version);
        // the nested buffer's u64 length prefix starts immediately after it.
        const declared = new DataView(produced.buffer, produced.byteOffset, produced.byteLength)
          .getBigUint64(env.data_length_prefix_offset, true);
        expect(Number(declared)).toBe(env.data_len);
        expect(produced.length).toBe(env.data_offset + env.data_len);
      });
    }
  });
}

// The sidecar for `op_second_set` says it carries only the two changed keys. The
// consequence that matters to a replicating peer: replaying that operation on top of
// the earlier state must land exactly on the later state.
test('state_after_first_set + op_second_set == state_after_second_set', () => {
  const state1 = readFixtureBytes(CORE_DIR, 'lww/state_after_first_set.bin');
  const op2 = readFixtureBytes(CORE_DIR, 'lww/op_second_set.bin');
  const state2 = readFixtureBytes(CORE_DIR, 'lww/state_after_second_set.bin');

  const replayed = LWWBackend.fromStateBuffer(state1);
  replayed.applyOperations([new Operation(op2)]);
  expect(toHex(replayed.toStateBuffer())).toBe(toHex(state2));
});

// `to_operations()` marks every entry committed, so a second call with no
// intervening set reports no change at all — null, never an operation carrying an
// empty map. A port that emits the empty operation floods its peers.
test('a second to_operations() with no intervening set is null', () => {
  const b = new LWWBackend();
  expect(b.toOperations()).toBeNull();
  b.set('k', i32(1));
  expect(b.toOperations()).not.toBeNull();
  expect(b.toOperations()).toBeNull();
});

// ── EntityState assembly ────────────────────────────────────────────────────

describe('entity_state.bin', () => {
  const bytes = readFixtureBytes(CORE_DIR, 'entity_state.bin');
  const sidecar = readSidecar(CORE_DIR, 'entity_state.json') as any;

  test('file length matches the sidecar', () => {
    expect(bytes.length).toBe(sidecar.total_len);
  });

  test('proto EntityState decodes to the sidecar value', () => {
    const reader = new BincodeReader(bytes);
    const state = EntityState.decode(reader);
    expect(reader.remaining).toBe(0);
    expect(toSerde(state)).toEqual(sidecar.decoded);
  });

  test('the nested lww buffer decodes to the sidecar property map', () => {
    const state = EntityState.decode(new BincodeReader(bytes));
    const decoded: Record<string, unknown> = {};
    for (const [name, buffer] of state.state.stateBuffers.entries()) {
      if (name !== 'lww') throw new Error(`unexpected backend ${name} in the fixture`);
      decoded[name] = propertyMapToSerde(LWWBackend.fromStateBuffer(buffer as Uint8Array).propertyValues());
    }
    expect(decoded).toEqual(sidecar.state_buffers_decoded);
  });

  test('replaying the scenario produces the fixture bytes', () => {
    const entity = Entity.create(entityId(0x42), CollectionId.from('album'));
    const lww = entity.getBackend(LWWBackend);
    lww.set('name', str('Ice Nine'));
    lww.set('year', i32(1993));
    lww.set(NON_ASCII_KEY, str(NON_ASCII_VALUE));

    const entityState = entity.toEntityState();
    expect(entityState.state.head.length).toBe(0);
    expect([...entityState.state.stateBuffers.entries()].map(([k]) => k)).toEqual(['lww']);

    const writer = new BincodeWriter();
    entityState.encode(writer);
    expect(toHex(writer.finish())).toBe(toHex(bytes));
  });
});

// ── Event assembly ──────────────────────────────────────────────────────────

describe('event.bin', () => {
  const bytes = readFixtureBytes(CORE_DIR, 'event.bin');
  const sidecar = readSidecar(CORE_DIR, 'event.json') as any;

  test('file length matches the sidecar', () => {
    expect(bytes.length).toBe(sidecar.total_len);
  });

  test('proto Event decodes to the sidecar value', () => {
    const reader = new BincodeReader(bytes);
    const event = Event.decode(reader);
    expect(reader.remaining).toBe(0);
    expect(toSerde(event)).toEqual(sidecar.decoded);
  });

  test('the nested lww diffs decode to the sidecar property maps', () => {
    const event = Event.decode(new BincodeReader(bytes));
    const decoded: Record<string, unknown> = {};
    for (const [name, ops] of event.operations.entries()) {
      decoded[name] = (ops as Operation[]).map((op) => decodeDiff(op.diff));
    }
    expect(decoded).toEqual(sidecar.operations_decoded);
  });

  // EventId::from_parts is SHA-256 over the bincode of entity_id, operations and
  // parent in that order, so matching it proves all three encodings at once.
  test('Event::id() matches the sidecar EventId', () => {
    const event = Event.decode(new BincodeReader(bytes));
    expect(event.id().toBase64()).toBe(sidecar.event_id_base64);
  });

  test('replaying the scenario produces the fixture bytes', () => {
    const entity = Entity.create(entityId(0x42), CollectionId.from('album'));
    const lww = entity.getBackend(LWWBackend);
    lww.set('name', str('Ice Nine'));
    lww.set('year', i32(1993));

    const ops = lww.toOperations();
    if (ops === null) throw new Error('two pending sets must yield operations');

    const event = new Event(
      entity.collection(),
      entity.id(),
      new OperationSet(new Map([[LWWBackend.propertyBackendName(), ops]])),
      entity.head(),
    );
    expect(event.isEntityCreate()).toBe(true);

    const writer = new BincodeWriter();
    event.encode(writer);
    expect(toHex(writer.finish())).toBe(toHex(bytes));
    expect(event.id().toBase64()).toBe(sidecar.event_id_base64);
  });
});
