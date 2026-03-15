// MIRRORS: ankurah/core/src/property/backend/yrs.rs (tests module)
import { describe, test, expect } from 'bun:test';
import { Operation } from '@ankurah/proto';
import { YjsBackend } from '../src/property/backend/yjs.ts';
import { YrsString, stringFromYrsString, optionalStringFromYrsString } from '../src/property/value/yrs_string.ts';
import { PropertyError } from '../src/property/traits.ts';

describe('YjsBackend', () => {

  test('constructor creates empty doc', () => {
    const backend = new YjsBackend();
    expect(backend.properties()).toEqual([]);
    expect(backend.getString('test')).toBeNull();
    expect(backend.propertyValue('test')).toBeNull();
  });

  test('propertyBackendName returns "yjs"', () => {
    expect(YjsBackend.propertyBackendName()).toBe('yjs');
  });

  test('insert and getString', () => {
    const backend = new YjsBackend();
    backend.insert('content', 0, 'Hello');
    expect(backend.getString('content')).toBe('Hello');
  });

  test('insert at index', () => {
    const backend = new YjsBackend();
    backend.insert('content', 0, 'Hello');
    backend.insert('content', 5, ', World!');
    expect(backend.getString('content')).toBe('Hello, World!');
  });

  test('delete characters', () => {
    const backend = new YjsBackend();
    backend.insert('content', 0, 'Hello, World!');
    backend.delete('content', 5, 8); // delete ", World!"
    expect(backend.getString('content')).toBe('Hello');
  });

  test('multiple fields', () => {
    const backend = new YjsBackend();
    backend.insert('title', 0, 'My Title');
    backend.insert('body', 0, 'My Body');
    expect(backend.getString('title')).toBe('My Title');
    expect(backend.getString('body')).toBe('My Body');
    expect(backend.properties().sort()).toEqual(['body', 'title']);
  });

  test('propertyValue returns Value.String', () => {
    const backend = new YjsBackend();
    backend.insert('field', 0, 'test');
    const value = backend.propertyValue('field');
    expect(value).not.toBeNull();
    expect(value!.type).toBe('String');
    expect(value!.value).toBe('test');
  });

  test('propertyValues returns all fields', () => {
    const backend = new YjsBackend();
    backend.insert('a', 0, 'alpha');
    backend.insert('b', 0, 'beta');
    const values = backend.propertyValues();
    expect(values.size).toBe(2);
    expect(values.get('a')).toEqual({ type: 'String', value: 'alpha' });
    expect(values.get('b')).toEqual({ type: 'String', value: 'beta' });
  });

  test('toStateBuffer and fromStateBuffer round-trip', () => {
    const backend1 = new YjsBackend();
    backend1.insert('content', 0, 'Hello, World!');
    const stateBuffer = backend1.toStateBuffer();
    expect(stateBuffer).toBeInstanceOf(Uint8Array);
    expect(stateBuffer.length).toBeGreaterThan(0);

    const backend2 = YjsBackend.fromStateBuffer(stateBuffer);
    expect(backend2.getString('content')).toBe('Hello, World!');
  });

  test('fork creates independent copy', () => {
    const backend1 = new YjsBackend();
    backend1.insert('content', 0, 'Hello');
    const backend2 = backend1.fork();

    // Both should have the same content
    expect(backend2.getString('content')).toBe('Hello');

    // Mutating one should not affect the other
    backend1.insert('content', 5, ', World!');
    expect(backend1.getString('content')).toBe('Hello, World!');
    expect(backend2.getString('content')).toBe('Hello');
  });

  test('toOperations returns null when no changes', () => {
    const backend = new YjsBackend();
    const ops = backend.toOperations();
    expect(ops).toBeNull();
  });

  test('toOperations returns diff after changes', () => {
    const backend = new YjsBackend();
    backend.insert('content', 0, 'Hello');
    const ops = backend.toOperations();
    expect(ops).not.toBeNull();
    expect(ops!.length).toBe(1);
    expect(ops![0]).toBeInstanceOf(Operation);
    expect(ops![0].diff.length).toBeGreaterThan(0);
  });

  test('toOperations returns null on second call with no new changes', () => {
    const backend = new YjsBackend();
    backend.insert('content', 0, 'Hello');
    backend.toOperations(); // consume the diff
    const ops = backend.toOperations(); // should be null now
    expect(ops).toBeNull();
  });

  test('toOperations returns incremental diff', () => {
    const backend = new YjsBackend();
    backend.insert('content', 0, 'Hello');
    const ops1 = backend.toOperations()!;

    backend.insert('content', 5, ', World!');
    const ops2 = backend.toOperations()!;

    // ops2 should be a smaller diff than ops1 (only the incremental change)
    expect(ops2).not.toBeNull();
    expect(ops2.length).toBe(1);

    // Apply ops1 + ops2 to a fresh backend
    const fresh = new YjsBackend();
    fresh.applyOperations(ops1);
    fresh.applyOperations(ops2);
    expect(fresh.getString('content')).toBe('Hello, World!');
  });

  test('applyOperations syncs state between backends', () => {
    const backend1 = new YjsBackend();
    backend1.insert('content', 0, 'Hello, World!');
    const ops = backend1.toOperations()!;

    const backend2 = new YjsBackend();
    backend2.applyOperations(ops);
    expect(backend2.getString('content')).toBe('Hello, World!');
  });

  test('listenField notifies on applyOperations', () => {
    const backend1 = new YjsBackend();
    const backend2 = new YjsBackend();

    let notified = false;
    backend2.listenField('content', () => { notified = true; });

    // Make a change on backend1
    backend1.insert('content', 0, 'Hello');
    const ops = backend1.toOperations()!;

    // Apply to backend2
    backend2.applyOperations(ops);
    expect(notified).toBe(true);
    expect(backend2.getString('content')).toBe('Hello');
  });

  test('listenField does not notify for unchanged fields', () => {
    const backend1 = new YjsBackend();
    const backend2 = new YjsBackend();

    let titleNotified = false;
    let bodyNotified = false;
    backend2.listenField('title', () => { titleNotified = true; });
    backend2.listenField('body', () => { bodyNotified = true; });

    // Only change 'title' on backend1
    backend1.insert('title', 0, 'My Title');
    const ops = backend1.toOperations()!;

    // Apply to backend2
    backend2.applyOperations(ops);
    expect(titleNotified).toBe(true);
    expect(bodyNotified).toBe(false);
  });

  test('listenField guard can unsubscribe', () => {
    const backend1 = new YjsBackend();
    const backend2 = new YjsBackend();

    let notifyCount = 0;
    const guard = backend2.listenField('content', () => { notifyCount++; });

    // First change
    backend1.insert('content', 0, 'Hello');
    backend2.applyOperations(backend1.toOperations()!);
    expect(notifyCount).toBe(1);

    // Unsubscribe
    guard.drop();

    // Second change should not notify
    backend1.insert('content', 5, ', World!');
    backend2.applyOperations(backend1.toOperations()!);
    expect(notifyCount).toBe(1); // still 1
  });

  test('fieldBroadcastId returns consistent id', () => {
    const backend = new YjsBackend();
    const id1 = backend.fieldBroadcastId('content');
    const id2 = backend.fieldBroadcastId('content');
    expect(id1.equals(id2)).toBe(true);

    const id3 = backend.fieldBroadcastId('other');
    expect(id1.equals(id3)).toBe(false);
  });

  test('backendFromString creates YjsBackend', () => {
    // Import the factory function
    const { backendFromString } = require('../src/property/backend/index.ts');

    const backend = backendFromString('yjs') as YjsBackend;
    expect(backend).toBeInstanceOf(YjsBackend);
  });

  test('backendFromString creates YjsBackend from state buffer', () => {
    const { backendFromString } = require('../src/property/backend/index.ts');

    const original = new YjsBackend();
    original.insert('content', 0, 'test data');
    const buffer = original.toStateBuffer();

    const restored = backendFromString('yjs', buffer) as YjsBackend;
    expect(restored.getString('content')).toBe('test data');
  });
});

describe('YrsString', () => {

  test('basic value access', () => {
    const backend = new YjsBackend();
    const entity = { isWritable: () => true } as any;
    const yrsString = new YrsString('content', backend, entity);

    expect(yrsString.value()).toBeNull();
    yrsString.insert(0, 'Hello');
    expect(yrsString.value()).toBe('Hello');
  });

  test('insert and delete', () => {
    const backend = new YjsBackend();
    const entity = { isWritable: () => true } as any;
    const yrsString = new YrsString('content', backend, entity);

    yrsString.insert(0, 'Hello, World!');
    expect(yrsString.value()).toBe('Hello, World!');

    yrsString.delete(5, 8);
    expect(yrsString.value()).toBe('Hello');
  });

  test('overwrite', () => {
    const backend = new YjsBackend();
    const entity = { isWritable: () => true } as any;
    const yrsString = new YrsString('content', backend, entity);

    yrsString.insert(0, 'Hello, World!');
    yrsString.overwrite(7, 5, 'TypeScript');
    expect(yrsString.value()).toBe('Hello, TypeScript!');
  });

  test('replace', () => {
    const backend = new YjsBackend();
    const entity = { isWritable: () => true } as any;
    const yrsString = new YrsString('content', backend, entity);

    yrsString.insert(0, 'Hello');
    yrsString.replace('Goodbye');
    expect(yrsString.value()).toBe('Goodbye');
  });

  test('replace on empty', () => {
    const backend = new YjsBackend();
    const entity = { isWritable: () => true } as any;
    const yrsString = new YrsString('content', backend, entity);

    yrsString.replace('Hello');
    expect(yrsString.value()).toBe('Hello');
  });

  test('throws when entity is not writable', () => {
    const backend = new YjsBackend();
    const entity = { isWritable: () => false } as any;
    const yrsString = new YrsString('content', backend, entity);

    expect(() => yrsString.insert(0, 'test')).toThrow();
    expect(() => yrsString.delete(0, 1)).toThrow();
    expect(() => yrsString.overwrite(0, 1, 'x')).toThrow();
    expect(() => yrsString.replace('x')).toThrow();
  });

  test('works without entity writability check', () => {
    // When entity doesn't have isWritable (e.g., during initialization)
    const backend = new YjsBackend();
    const entity = {} as any;
    const yrsString = new YrsString('content', backend, entity);

    yrsString.insert(0, 'test');
    expect(yrsString.value()).toBe('test');
  });

  test('Signal interface: listen and broadcastId', () => {
    const backend = new YjsBackend();
    const entity = { isWritable: () => true } as any;
    const yrsString = new YrsString('content', backend, entity);

    // broadcastId should return a valid BroadcastId
    const id = yrsString.broadcastId();
    expect(id).toBeDefined();

    // listen should return a guard
    let notified = false;
    const guard = yrsString.listen(() => { notified = true; });
    expect(guard).toBeDefined();

    // Notification happens via applyOperations on the backend
    const other = new YjsBackend();
    other.insert('content', 0, 'trigger');
    const ops = other.toOperations()!;
    backend.applyOperations(ops);

    expect(notified).toBe(true);

    guard.drop();
  });

  test('stringFromYrsString extracts string', () => {
    const backend = new YjsBackend();
    const entity = { isWritable: () => true } as any;
    const yrsString = new YrsString('content', backend, entity);

    yrsString.insert(0, 'Hello');
    expect(stringFromYrsString(yrsString)).toBe('Hello');
  });

  test('stringFromYrsString throws on missing', () => {
    const backend = new YjsBackend();
    const entity = {} as any;
    const yrsString = new YrsString('content', backend, entity);

    expect(() => stringFromYrsString(yrsString)).toThrow(PropertyError);
  });

  test('optionalStringFromYrsString returns null on missing', () => {
    const backend = new YjsBackend();
    const entity = {} as any;
    const yrsString = new YrsString('content', backend, entity);

    expect(optionalStringFromYrsString(yrsString)).toBeNull();
  });

  test('optionalStringFromYrsString returns string when present', () => {
    const backend = new YjsBackend();
    const entity = { isWritable: () => true } as any;
    const yrsString = new YrsString('content', backend, entity);

    yrsString.insert(0, 'Hello');
    expect(optionalStringFromYrsString(yrsString)).toBe('Hello');
  });
});
