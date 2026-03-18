// MIRRORS: ankurah/core/src/resultset.rs #[cfg(test)] mod tests
import { describe, test, expect } from 'bun:test';
import { EntityId, CollectionId } from '@ankurah/proto';
import { Entity } from './entity.ts';
import { EntityResultSet } from './resultset.ts';
import type { Value } from './value/index.ts';
import { ValueType } from './value/index.ts';
import { IndexDirection, NullsOrder, type KeySpec } from './indexing/key_spec.ts';

// ── Test helpers ──

/** Create a test entity with a specific ID byte and optional properties. */
function testEntity(idByte: number, properties?: Record<string, unknown>): Entity {
  const idBytes = new Uint8Array(16);
  idBytes[15] = idByte;
  const entity = Entity.create(EntityId.fromBytes(idBytes), CollectionId.fixedName('test'));

  if (properties) {
    for (const [key, value] of Object.entries(properties)) {
      entity.initializeProperty(key, value, 'lww');
    }
  }

  return entity;
}

// ── Tests ──

describe('resultset', () => {
  // Rust: fn test_entity_id_ordering()
  test('entity id ordering', () => {
    const resultset = EntityResultSet.empty();
    const write = resultset.write();

    // Create entities with different IDs (bytes sort chronologically)
    const entity1 = testEntity(1);
    const entity2 = testEntity(2);
    const entity3 = testEntity(3);

    // Add in reverse order
    write.add(entity3);
    write.add(entity1);
    write.add(entity2);

    write.drop();

    // Should be sorted by entity ID
    const readGuard = resultset.read();
    const entities = readGuard.iterEntities();
    expect(entities.length).toBe(3);
    expect(entities[0][0].toBase64()).toBe(entity1.id().toBase64());
    expect(entities[1][0].toBase64()).toBe(entity2.id().toBase64());
    expect(entities[2][0].toBase64()).toBe(entity3.id().toBase64());
  });

  // Rust: fn test_limit_functionality()
  test('limit functionality', () => {
    const resultset = EntityResultSet.empty();

    // Add some entities
    const write = resultset.write();
    for (let i = 0; i < 5; i++) {
      const entity = testEntity(i, { value: i });
      write.add(entity);
    }
    write.drop();

    expect(resultset.len()).toBe(5);

    // Apply limit
    resultset.setLimit(3);
    expect(resultset.len()).toBe(3);

    // Remove limit
    resultset.setLimit(null);
    expect(resultset.len()).toBe(3); // Should stay truncated
  });

  // Rust: fn test_dirty_tracking()
  test('dirty tracking', () => {
    const resultset = EntityResultSet.empty();

    const entity1 = testEntity(1, { active: true });
    const entity2 = testEntity(2, { active: false });

    const write = resultset.write();
    write.add(entity1);
    write.add(entity2);

    // Mark all dirty
    write.markAllDirty();

    // Retain only active entities
    const removed = write.retainDirty((entity) => {
      const val = entity.getPropertyValue('active');
      return val !== null && val.type === 'Bool' && val.value === true;
    });

    write.drop();

    expect(removed.length).toBe(1);
    expect(removed[0].toBase64()).toBe(entity2.id().toBase64());
    expect(resultset.len()).toBe(1);
    expect(resultset.read().iterEntities()[0][0].toBase64()).toBe(entity1.id().toBase64());
  });

  // Rust: fn test_write_guard_atomic_operations()
  test('write guard atomic operations', () => {
    const resultset = EntityResultSet.empty();

    // Multiple operations in one write guard should be atomic
    {
      const write = resultset.write();
      const entity1 = testEntity(1);
      const entity2 = testEntity(2);

      write.add(entity1);
      write.add(entity2);

      // Operations are visible within the same write guard
      expect(write.iterEntities().length).toBe(2);

      // Notification sent when drop() is called
      write.drop();
    }

    // Operations should be visible after guard is dropped
    expect(resultset.len()).toBe(2);
  });

  // Rust: fn test_order_by_with_tie_breaking()
  test('order by with tie breaking', () => {
    const resultset = EntityResultSet.empty();

    // Create entities with same name but different IDs
    const entity1 = testEntity(1, { name: 'Alice' });
    const entity2 = testEntity(2, { name: 'Alice' });
    const entity3 = testEntity(3, { name: 'Bob' });

    // Set up ordering by name
    const keySpec: KeySpec = {
      keyparts: [{
        column: 'name',
        subPath: null,
        direction: IndexDirection.Asc,
        nulls: NullsOrder.Last,
        collation: null,
        valueType: ValueType.String,
      }],
    };
    resultset.orderBy(keySpec);

    const write = resultset.write();
    write.add(entity2);
    write.add(entity3);
    write.add(entity1);
    write.drop();

    // Should be sorted by name, then by entity ID for tie-breaking
    const readGuard = resultset.read();
    const entities = readGuard.iterEntities();
    expect(entities.length).toBe(3);
    // Both Alice entities should come first (sorted by ID), then Bob
    expect(entities[0][0].toBase64()).toBe(entity1.id().toBase64()); // Alice (earlier ID)
    expect(entities[1][0].toBase64()).toBe(entity2.id().toBase64()); // Alice (later ID)
    expect(entities[2][0].toBase64()).toBe(entity3.id().toBase64()); // Bob
  });

  // Rust: fn test_ivec_small_keys(), test_ivec_large_keys(), test_ivec_boundary()
  // Divergence: IVec is not ported — TS uses plain Uint8Array. No Small/Large variant distinction [E8].
  // These tests are not applicable.
});
