// MIRRORS: ankurah/core/src/property/backend/lww.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Operation } from '@ankurah/proto';
import { LWWBackend } from './lww.ts';
import type { Value } from '../../value/index.ts';

describe('LWWBackend', () => {
  test('new backend is empty', () => {
    const backend = new LWWBackend();
    expect(backend.properties()).toEqual([]);
    expect(backend.propertyValues().size).toBe(0);
  });

  test('set and get string value', () => {
    const backend = new LWWBackend();
    const value: Value = { type: 'String', value: 'hello' };
    backend.set('name', value);
    const result = backend.get('name');
    expect(result).not.toBeNull();
    expect(result!.type).toBe('String');
    expect((result as { type: 'String'; value: string }).value).toBe('hello');
  });

  test('set and get null value', () => {
    const backend = new LWWBackend();
    backend.set('name', null);
    const result = backend.get('name');
    expect(result).toBeNull();
  });

  test('get missing property returns null', () => {
    const backend = new LWWBackend();
    expect(backend.get('nonexistent')).toBeNull();
  });

  test('properties returns sorted keys', () => {
    const backend = new LWWBackend();
    backend.set('zebra', { type: 'String', value: 'z' });
    backend.set('apple', { type: 'String', value: 'a' });
    backend.set('mango', { type: 'String', value: 'm' });
    expect(backend.properties()).toEqual(['apple', 'mango', 'zebra']);
  });

  test('propertyValue delegates to get', () => {
    const backend = new LWWBackend();
    const value: Value = { type: 'I32', value: 42 };
    backend.set('count', value);
    const result = backend.propertyValue('count');
    expect(result).not.toBeNull();
    expect(result!.type).toBe('I32');
    expect((result as { type: 'I32'; value: number }).value).toBe(42);
  });

  test('propertyValues returns all values', () => {
    const backend = new LWWBackend();
    backend.set('a', { type: 'I32', value: 1 });
    backend.set('b', { type: 'String', value: 'two' });
    backend.set('c', null);
    const map = backend.propertyValues();
    expect(map.size).toBe(3);
    expect(map.get('c')).toBeNull();
  });

  test('fork creates deep copy with independent values', () => {
    const backend = new LWWBackend();
    backend.set('name', { type: 'String', value: 'original' });
    const forked = backend.fork() as LWWBackend;

    // Forked has the same value
    expect(forked.get('name')).not.toBeNull();
    expect((forked.get('name') as { type: 'String'; value: string }).value).toBe('original');

    // Modifying fork doesn't affect original
    forked.set('name', { type: 'String', value: 'modified' });
    expect((backend.get('name') as { type: 'String'; value: string }).value).toBe('original');
    expect((forked.get('name') as { type: 'String'; value: string }).value).toBe('modified');
  });

  test('toStateBuffer and fromStateBuffer round-trip', () => {
    const backend = new LWWBackend();
    backend.set('name', { type: 'String', value: 'Alice' });
    backend.set('age', { type: 'I32', value: 30 });
    backend.set('active', { type: 'Bool', value: true });
    backend.set('score', { type: 'F64', value: 3.14 });
    backend.set('empty', null);

    const buffer = backend.toStateBuffer();
    const restored = LWWBackend.fromStateBuffer(buffer);

    expect(restored.properties()).toEqual(backend.properties());
    expect((restored.get('name') as { type: 'String'; value: string }).value).toBe('Alice');
    expect((restored.get('age') as { type: 'I32'; value: number }).value).toBe(30);
    expect((restored.get('active') as { type: 'Bool'; value: boolean }).value).toBe(true);
    expect((restored.get('score') as { type: 'F64'; value: number }).value).toBeCloseTo(3.14);
    expect(restored.get('empty')).toBeNull();
  });

  test('toOperations returns null when nothing changed', () => {
    const backend = new LWWBackend();
    expect(backend.toOperations()).toBeNull();
  });

  test('toOperations returns operations for uncommitted values', () => {
    const backend = new LWWBackend();
    backend.set('name', { type: 'String', value: 'Bob' });
    const ops = backend.toOperations();
    expect(ops).not.toBeNull();
    expect(ops!.length).toBe(1);
    expect(ops![0]).toBeInstanceOf(Operation);
  });

  test('toOperations marks values as committed', () => {
    const backend = new LWWBackend();
    backend.set('name', { type: 'String', value: 'Bob' });
    backend.toOperations(); // first call
    const ops2 = backend.toOperations(); // second call
    expect(ops2).toBeNull(); // nothing new to commit
  });

  test('applyOperations merges values', () => {
    const sender = new LWWBackend();
    sender.set('name', { type: 'String', value: 'Charlie' });
    sender.set('count', { type: 'I32', value: 7 });
    const ops = sender.toOperations()!;

    const receiver = new LWWBackend();
    receiver.applyOperations(ops);

    expect((receiver.get('name') as { type: 'String'; value: string }).value).toBe('Charlie');
    expect((receiver.get('count') as { type: 'I32'; value: number }).value).toBe(7);
  });

  test('applyOperations notifies field listeners', () => {
    const receiver = new LWWBackend();
    let notified = false;
    const guard = receiver.listenField('name', () => {
      notified = true;
    });

    const sender = new LWWBackend();
    sender.set('name', { type: 'String', value: 'Dave' });
    const ops = sender.toOperations()!;

    receiver.applyOperations(ops);
    expect(notified).toBe(true);

    guard.drop();
  });

  test('listenField creates broadcast lazily', () => {
    const backend = new LWWBackend();
    const guard1 = backend.listenField('field1', () => {});
    const guard2 = backend.listenField('field1', () => {});
    // Both guards should share the same broadcast ID
    expect(guard1.broadcastId().equals(guard2.broadcastId())).toBe(true);
    guard1.drop();
    guard2.drop();
  });

  test('fieldBroadcastId returns consistent ID', () => {
    const backend = new LWWBackend();
    const id1 = backend.fieldBroadcastId('field1');
    const id2 = backend.fieldBroadcastId('field1');
    expect(id1.equals(id2)).toBe(true);
  });

  test('propertyBackendName returns lww', () => {
    expect(LWWBackend.propertyBackendName()).toBe('lww');
  });

  test('state buffer round-trip with all value types', () => {
    const { EntityId } = require('@ankurah/proto') as typeof import('@ankurah/proto');
    const backend = new LWWBackend();
    backend.set('i16', { type: 'I16', value: -123 });
    backend.set('i32', { type: 'I32', value: 100000 });
    backend.set('i64', { type: 'I64', value: 9007199254740991 });
    backend.set('f64', { type: 'F64', value: 2.71828 });
    backend.set('bool', { type: 'Bool', value: false });
    backend.set('string', { type: 'String', value: 'test string' });
    backend.set('entity_id', { type: 'EntityId', value: EntityId.new() });
    backend.set('object', { type: 'Object', value: new Uint8Array([1, 2, 3]) });
    backend.set('binary', { type: 'Binary', value: new Uint8Array([4, 5, 6]) });
    backend.set('json', { type: 'Json', value: { key: 'val', num: 42 } });

    const buffer = backend.toStateBuffer();
    const restored = LWWBackend.fromStateBuffer(buffer);

    expect(restored.properties().length).toBe(10);
    expect((restored.get('i16') as any).value).toBe(-123);
    expect((restored.get('i32') as any).value).toBe(100000);
    expect((restored.get('i64') as any).value).toBe(9007199254740991);
    expect((restored.get('f64') as any).value).toBeCloseTo(2.71828);
    expect((restored.get('bool') as any).value).toBe(false);
    expect((restored.get('string') as any).value).toBe('test string');
    expect((restored.get('entity_id') as any).value.equals((backend.get('entity_id') as any).value)).toBe(true);
    expect(Array.from((restored.get('object') as any).value)).toEqual([1, 2, 3]);
    expect(Array.from((restored.get('binary') as any).value)).toEqual([4, 5, 6]);
    expect((restored.get('json') as any).value).toEqual({ key: 'val', num: 42 });
  });

  test('operations round-trip with all value types', () => {
    const { EntityId } = require('@ankurah/proto') as typeof import('@ankurah/proto');
    const sender = new LWWBackend();
    sender.set('i16', { type: 'I16', value: 42 });
    sender.set('string', { type: 'String', value: 'hello' });
    sender.set('json', { type: 'Json', value: [1, 2, 3] });
    sender.set('entity_id', { type: 'EntityId', value: EntityId.new() });

    const ops = sender.toOperations()!;
    const receiver = new LWWBackend();
    receiver.applyOperations(ops);

    expect((receiver.get('i16') as any).value).toBe(42);
    expect((receiver.get('string') as any).value).toBe('hello');
    expect((receiver.get('json') as any).value).toEqual([1, 2, 3]);
    expect((receiver.get('entity_id') as any).value.equals((sender.get('entity_id') as any).value)).toBe(true);
  });

  test('applyOperations throws on unknown version', () => {
    const { BincodeWriter } = require('@ankurah/proto') as typeof import('@ankurah/proto');
    const writer = new BincodeWriter();
    writer.writeU8(99); // unknown version
    writer.writeByteVec(new Uint8Array(0));
    const badOp = new Operation(writer.finish());

    const backend = new LWWBackend();
    expect(() => backend.applyOperations([badOp])).toThrow();
  });

  test('fromStateBuffer with empty map', () => {
    const { BincodeWriter } = require('@ankurah/proto') as typeof import('@ankurah/proto');
    // An empty BTreeMap is just u64(0)
    const writer = new BincodeWriter();
    writer.writeU64(0n);
    const buffer = writer.finish();

    const backend = LWWBackend.fromStateBuffer(buffer);
    expect(backend.properties()).toEqual([]);
  });

  test('incremental operations only include uncommitted changes', () => {
    const backend = new LWWBackend();
    backend.set('a', { type: 'I32', value: 1 });
    backend.toOperations(); // commit 'a'

    backend.set('b', { type: 'I32', value: 2 }); // new uncommitted
    const ops = backend.toOperations()!;

    // Apply to receiver - should only have 'b'
    const receiver = new LWWBackend();
    receiver.applyOperations(ops);
    expect(receiver.get('a')).toBeNull();
    expect((receiver.get('b') as any).value).toBe(2);
  });
});
