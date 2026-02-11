# Fetch-Gap / GapFetcher Port Spec

**Source**: `/Users/daniel/ak/ankurah/core/src/reactor/fetch_gap.rs`
**Target**: `/Users/daniel/ak/ankurah-ts/packages/core/src/reactor/fetch-gap.ts`
**Mirrors header**: `// MIRRORS: ankurah/core/src/reactor/fetch_gap.rs`

---

## 1. Overview

The fetch-gap module provides automatic gap-filling for LIMIT-constrained reactive queries. When an entity is removed from a sorted, limited result set (e.g., `ORDER BY year ASC LIMIT 5`), the result set drops below its limit. The GapFetcher mechanism detects this gap and fetches replacement entities from storage (local or remote) that would be the "next" entities in sort order.

### Key components

| Rust item | TS equivalent | Purpose |
|---|---|---|
| `trait GapFetcher<E>` | `interface GapFetcher` | Async fetch of replacement entities |
| `struct QueryGapFetcher<SE, PA>` | `class QueryGapFetcher` | Concrete impl using a node reference |
| `fn build_continuation_predicate()` | `function buildContinuationPredicate()` | Constructs AnkQL predicate for "after last entity" |
| `fn infer_value_type_for_field()` | `function inferValueTypeForField()` | Type inference from entity data |

---

## 2. Import Dependencies

```typescript
// From @ankurah/ankql
import type {
  Predicate,
  Selection,
  OrderByItem,
  OrderDirection,
  ComparisonOperator,
  Expr,
  Literal,
  PathExpr,
} from '@ankurah/ankql';

// From @ankurah/proto
import type { CollectionId, EntityId } from '@ankurah/proto';

// From @ankurah/core (local)
import type { Entity } from '../entity.ts';
import type { Value } from '../value/index.ts';
import { ValueType, valueType, valueToLiteral } from '../value/index.ts';
```

---

## 3. Types and Interfaces

### 3.1 GapFetcher Interface

**Rust source** (lines 16-34):
```rust
#[async_trait]
pub trait GapFetcher<E: AbstractEntity>: Send + Sync + 'static {
    async fn fetch_gap(
        &self,
        collection_id: &proto::CollectionId,
        selection: &ankql::ast::Selection,
        last_entity: Option<&E>,
        gap_size: usize,
    ) -> Result<Vec<E>, RetrievalError>;
}
```

**TypeScript port**:
```typescript
/**
 * Interface for fetching entities to fill gaps when LIMIT causes entities to be evicted.
 *
 * MIRRORS: Rust trait GapFetcher<E: AbstractEntity>
 * Divergence: No generic type parameter E -- TS uses concrete Entity type [E8].
 * Divergence: No Send + Sync + 'static bounds -- JS is single-threaded [E8].
 * Divergence: Returns Promise instead of async_trait future [E8].
 */
export interface GapFetcher {
  /**
   * Fetch entities to fill a gap in a limited result set.
   *
   * @param collectionId - The collection to fetch from
   * @param selection - The original selection (predicate, orderBy, limit)
   * @param lastEntity - The last entity in the current result set (used to build continuation predicate), or null
   * @param gapSize - Number of entities needed to fill the gap
   * @returns Array of entities that match the selection and come after lastEntity in sort order
   * @throws RetrievalError on failure
   */
  fetchGap(
    collectionId: CollectionId,
    selection: Selection,
    lastEntity: Entity | null,
    gapSize: number,
  ): Promise<Entity[]>;
}
```

**Key divergences from Rust**:
- Rust uses `Option<&E>` for `last_entity`; TS uses `Entity | null`.
- Rust generic `E: AbstractEntity` is erased; TS uses concrete `Entity`.
- Rust `Result<Vec<E>, RetrievalError>` becomes `Promise<Entity[]>` (throws on error).
- Rust `async_trait` becomes a normal method returning `Promise`.

### 3.2 QueryGapFetcher Class

**Rust source** (lines 37-101):
```rust
#[derive(Clone)]
pub struct QueryGapFetcher<SE, PA> where SE: StorageEngine, PA: PolicyAgent {
    weak_node: Weak<NodeInner<SE, PA>>,
    cdata: PA::ContextData,
}
```

**TypeScript port**:
```typescript
/**
 * Concrete implementation of GapFetcher that uses a Node reference to fetch entities.
 *
 * MIRRORS: Rust struct QueryGapFetcher<SE, PA>
 * Divergence: No StorageEngine/PolicyAgent type params -- TS node is untyped [E8].
 * Divergence: No Weak<NodeInner> -- uses plain reference (JS GC handles lifecycle) [E8].
 * Divergence: No cdata (context data) -- TS node context handles this internally [E8].
 */
export class QueryGapFetcher implements GapFetcher {
  /**
   * Reference to the node for fetching entities.
   * Rust uses Weak<NodeInner<SE, PA>> to avoid preventing node cleanup.
   * Divergence: JS uses a plain reference. If the node is garbage collected,
   * the WeakRef will return undefined and fetchGap will throw [E8].
   */
  private nodeRef: WeakRef<Node>;

  constructor(node: Node) {
    this.nodeRef = new WeakRef(node);
  }

  async fetchGap(
    collectionId: CollectionId,
    selection: Selection,
    lastEntity: Entity | null,
    gapSize: number,
  ): Promise<Entity[]> {
    // Upgrade weak reference
    const node = this.nodeRef.deref();
    if (!node) {
      throw new RetrievalError('Node has been dropped, cannot fill gap');
    }

    // Build gap selection with continuation predicate
    let gapSelection: Selection;
    if (lastEntity !== null) {
      let gapPredicate: Predicate;
      if (selection.orderBy !== null) {
        gapPredicate = buildContinuationPredicate(
          selection.predicate,
          selection.orderBy,
          lastEntity,
        );
      } else {
        gapPredicate = selection.predicate;
      }
      gapSelection = new Selection(gapPredicate, selection.orderBy, gapSize);
    } else {
      // No last entity -- use original selection with gap_size limit
      gapSelection = new Selection(
        selection.predicate,
        selection.orderBy,
        gapSize,
      );
    }

    return node.fetchEntities(collectionId, gapSelection);
  }
}
```

**Concurrency note**: The Rust implementation uses `Weak<NodeInner<SE, PA>>` to avoid preventing the node from being dropped. In JS, we use `WeakRef<Node>` for the same purpose. The `FinalizationRegistry` pattern is not needed here because we simply check `deref()` at call time.

**Node dependency**: The `QueryGapFetcher` depends on a `Node` (or similar) type that has a `fetchEntities(collectionId, selection)` method. This mirrors Rust's `NodeAndContext::fetch_entities()`. The exact TS `Node` type will need to be defined or imported from the node module when it is ported.

---

## 4. Functions

### 4.1 `buildContinuationPredicate()`

**Rust source** (lines 103-165):

```rust
pub fn build_continuation_predicate<E: AbstractEntity>(
    original_predicate: &ankql::ast::Predicate,
    order_by: &[ankql::ast::OrderByItem],
    last_entity: &E,
) -> Result<ankql::ast::Predicate, String>
```

**TypeScript signature**:
```typescript
/**
 * Build a supplemental predicate to fetch entities after the last entity in sort order.
 *
 * For ORDER BY a ASC, b DESC with last entity having a=5, b=10:
 * Returns: original_predicate AND a >= 5 AND b <= 10 AND id != last_entity.id
 *
 * MIRRORS: Rust fn build_continuation_predicate()
 * Divergence: No generic E -- uses concrete Entity [E8].
 * Divergence: Returns Predicate directly; throws string on error instead of Result<_, String> [E7].
 *
 * @param originalPredicate - The original WHERE clause predicate
 * @param orderBy - The ORDER BY items from the selection
 * @param lastEntity - The last entity in the current result set
 * @returns A new predicate combining original + continuation conditions + ID exclusion
 * @throws string on error (e.g., if entity values cannot be extracted)
 */
export function buildContinuationPredicate(
  originalPredicate: Predicate,
  orderBy: OrderByItem[],
  lastEntity: Entity,
): Predicate
```

**Detailed algorithm**:

1. **Start with a conditions array**: `gapConditions: Predicate[] = []`

2. **Add original predicate**: Push `originalPredicate` as the first condition.

3. **For each ORDER BY item**:
   - Extract the field name via `orderItem.path.property()` (last step of the PathExpr).
   - Get the field value from the last entity: `lastEntity.getPropertyValue(fieldName)`.
   - If the value is `null`, skip this order item (field not present on entity).
   - Convert the `Value` to an AnkQL `Literal` using `valueToLiteral(fieldValue)`.
   - **Skip non-orderable types**: If the value type is `'Object'`, `'Binary'`, or `'Json'`, skip (these are not commonly used in ORDER BY, and `valueToLiteral` for Object/Binary produces lossy String literals anyway -- but we skip them to match Rust behavior).
   - Determine the comparison operator based on sort direction:
     - `'Asc'` --> `'GreaterThanOrEqual'`
     - `'Desc'` --> `'LessThanOrEqual'`
   - Construct a `Predicate` node:
     ```typescript
     const condition: Predicate = {
       type: 'Comparison',
       left: { type: 'Path', value: orderItem.path } as Expr,
       operator: operator,
       right: { type: 'Literal', value: literal } as Expr,
     };
     ```
   - Push `condition` to `gapConditions`.

4. **Add entity ID exclusion**:
   ```typescript
   const idExclusion: Predicate = {
     type: 'Comparison',
     left: { type: 'Path', value: PathExpr.simple('id') } as Expr,
     operator: 'NotEqual' as ComparisonOperator,
     right: {
       type: 'Literal',
       value: { type: 'EntityId', value: lastEntity.id().toBytes() } as Literal,
     } as Expr,
   };
   gapConditions.push(idExclusion);
   ```

5. **Combine all conditions with AND**:
   ```typescript
   const result = gapConditions.reduce((acc, condition) => ({
     type: 'And' as const,
     left: acc,
     right: condition,
   }));
   ```
   If `gapConditions` is empty (should not happen since we always have at least the original predicate), return `{ type: 'True' }`.

6. **Return** the combined predicate.

**Value-to-Literal conversion mapping** (matches Rust lines 125-135):

| Value type | Literal type | Notes |
|---|---|---|
| `{ type: 'String', value: s }` | `{ type: 'String', value: s }` | Direct mapping |
| `{ type: 'I16', value: n }` | `{ type: 'I16', value: n }` | Direct mapping |
| `{ type: 'I32', value: n }` | `{ type: 'I32', value: n }` | Direct mapping |
| `{ type: 'I64', value: n }` | `{ type: 'I64', value: BigInt(n) }` | Number to BigInt conversion |
| `{ type: 'F64', value: f }` | `{ type: 'F64', value: f }` | Direct mapping |
| `{ type: 'Bool', value: b }` | `{ type: 'Bool', value: b }` | Direct mapping |
| `{ type: 'EntityId', value: id }` | `{ type: 'EntityId', value: id.toBytes() }` | EntityId to bytes |
| `{ type: 'Object', ... }` | **SKIP** | Not used in ORDER BY |
| `{ type: 'Binary', ... }` | **SKIP** | Not used in ORDER BY |
| `{ type: 'Json', ... }` | **SKIP** | Not used in ORDER BY |

Note: The existing `valueToLiteral()` function from `/Users/daniel/ak/ankurah-ts/packages/core/src/value/index.ts` (line 291) already handles this conversion. Use it directly, but add the skip-guard for Object/Binary/Json before calling it.

### 4.2 `inferValueTypeForField()`

**Rust source** (lines 167-177):

```rust
pub fn infer_value_type_for_field<E: AbstractEntity>(entities: &[E], field_name: &str) -> ValueType {
    for entity in entities {
        if let Some(value) = entity.value(field_name) {
            return ValueType::of(&value);
        }
    }
    ValueType::String // Default fallback
}
```

**TypeScript signature**:
```typescript
/**
 * Infer ValueType from the first non-null value in a collection of entities.
 * Scans entities in order and returns the ValueType of the first entity that
 * has a non-null value for the given field.
 *
 * Falls back to ValueType.String if no entity has the field.
 *
 * MIRRORS: Rust fn infer_value_type_for_field()
 * Divergence: No generic E -- uses concrete Entity [E8].
 *
 * @param entities - Array of entities to scan
 * @param fieldName - The field name to look up
 * @returns The inferred ValueType, or ValueType.String as default
 */
export function inferValueTypeForField(
  entities: Entity[],
  fieldName: string,
): ValueType {
  for (const entity of entities) {
    const value = entity.getPropertyValue(fieldName);
    if (value !== null) {
      return valueType(value);
    }
  }
  // TODO: Get type from system catalog instead of defaulting to String
  return ValueType.String;
}
```

**Implementation**: Straightforward 1:1 port. Uses `Entity.getPropertyValue()` (which mirrors Rust's `AbstractEntity::value()`) and the existing `valueType()` function from `/Users/daniel/ak/ankurah-ts/packages/core/src/value/index.ts` (line 44).

---

## 5. Integration Points

### 5.1 Where GapFetcher is used in the Rust codebase

The `GapFetcher` is stored inside `QueryState` and used from `subscription_state.rs`:

**`QueryState` fields** (subscription_state.rs lines 58-68):
```rust
pub struct QueryState<E: AbstractEntity + Filterable> {
    pub(crate) collection_id: proto::CollectionId,
    pub(crate) selection: Option<ankql::ast::Selection>,
    pub(crate) gap_fetcher: Arc<dyn GapFetcher<E>>,
    pub(crate) paused: bool,
    pub(crate) resultset: EntityResultSet<E>,
    pub(crate) version: u32,
}
```

**TS equivalent** (when subscription_state is ported):
```typescript
interface QueryState {
  collectionId: CollectionId;
  selection: Selection | null;     // null until first update_query
  gapFetcher: GapFetcher;          // No Arc needed -- JS reference counting via GC
  paused: boolean;
  resultset: EntityResultSet;      // To be ported
  version: number;                 // u32 maps to number
}
```

### 5.2 Gap detection logic

The `extract_gap_data` method (subscription_state.rs lines 608-637) determines when gap filling is needed:

```typescript
/**
 * Determines if a query needs gap filling and extracts the data needed to perform it.
 *
 * Gap filling is triggered when:
 * 1. The resultset's gap_dirty flag is set (set when entities are removed and count drops below limit)
 * 2. A limit is configured
 * 3. Current entity count is less than the limit
 */
function extractGapData(queryId: QueryId, queryState: QueryState): GapFillData | null {
  const resultset = queryState.resultset;

  if (!resultset.isGapDirty()) {
    return null;
  }

  const limit = resultset.getLimit();
  if (limit === null) {
    return null;
  }

  const currentLen = resultset.len();
  if (currentLen >= limit) {
    return null;
  }

  const gapSize = limit - currentLen;
  const lastEntity = resultset.lastEntity(); // Entity | null

  const selection = queryState.selection;
  if (selection === null) {
    throw new Error('extractGapData called before updateQuery');
  }

  return {
    queryId,
    gapFetcher: queryState.gapFetcher,
    collectionId: queryState.collectionId,
    selection,
    resultset,
    lastEntity,
    gapSize,
  };
}
```

**`GapFillData` type**:
```typescript
interface GapFillData {
  queryId: QueryId;
  gapFetcher: GapFetcher;
  collectionId: CollectionId;
  selection: Selection;
  resultset: EntityResultSet;
  lastEntity: Entity | null;
  gapSize: number;
}
```

### 5.3 The `gap_dirty` flag on EntityResultSet

The Rust `EntityResultSet` has internal state tracking:
- `gap_dirty: boolean` -- Set to `true` when entities are removed from a result set that was at its limit (i.e., `was_at_limit && order.len() < limit`). This is the trigger for gap filling.
- `isGapDirty()` -- reads the flag
- `clearGapDirty()` -- resets the flag after gap filling completes
- `lastEntity()` -- returns the last entity in sort order (used as the continuation point)
- `getLimit()` -- returns the configured limit

These methods must exist on the TS `EntityResultSet` when it is ported.

---

## 6. Concurrency Patterns Needing JS Simplification

### 6.1 `Weak<NodeInner>` --> `WeakRef<Node>`

**Rust**: `Weak<NodeInner<SE, PA>>` prevents the GapFetcher from preventing Node cleanup. `upgrade()` returns `None` if the node has been dropped.

**TS**: `WeakRef<Node>` provides the same semantics. `deref()` returns `undefined` if the object has been garbage collected.

### 6.2 `Arc<dyn GapFetcher<E>>` --> plain `GapFetcher` reference

**Rust**: `Arc<dyn GapFetcher<E>>` provides thread-safe shared ownership of the gap fetcher across subscriptions.

**TS**: JS is single-threaded. A plain `GapFetcher` reference suffices. No `Arc` equivalent is needed.

### 6.3 `Mutex<State>` --> direct property access

**Rust**: `EntityResultSet` wraps state in `Mutex<State>` for thread-safe access. `ResultSetWrite`/`ResultSetRead` are RAII guards.

**TS**: JS is single-threaded. Direct property access replaces `Mutex`. If `EntityResultSet` is ported with guard-like patterns for batching notifications (e.g., only broadcast after a batch of mutations), those can use try/finally blocks rather than RAII:

```typescript
const write = resultset.beginWrite();
try {
  write.add(entity);
  // ... more mutations ...
} finally {
  write.commit(); // sends notification if changed
}
```

### 6.4 `tokio::spawn` for background gap filling --> `Promise` / microtask

**Rust** (subscription_state.rs lines 583-586): Gap fills are run concurrently via `future::join_all(gap_fill_futures)` and spawned via `crate::task::spawn()`.

**TS**: Use `Promise.all()` for concurrent gap fills. No background task spawning is needed since JS async/await naturally handles this:

```typescript
// Multiple gap fills concurrently
const gapResults = await Promise.all(
  gapsToFill.map(({ queryId, gapFetcher, collectionId, selection, resultset, lastEntity, gapSize }) =>
    processGapFill(queryId, gapFetcher, collectionId, selection, resultset, lastEntity, gapSize)
  )
);
```

### 6.5 `AtomicBool` for `loaded` flag --> plain boolean

**Rust**: `EntityResultSet` uses `AtomicBool` for the `loaded` flag.

**TS**: Plain `boolean` property. No atomics needed in single-threaded JS.

### 6.6 Locking discipline (from plan spec)

The Rust implementation carefully manages lock ordering:
1. Take subscription state lock briefly to snapshot query data
2. Drop lock before async work (gap fetching)
3. Re-take lock briefly to commit results

In JS, since there's no multi-threading, the equivalent is:
1. Read query state synchronously
2. Perform async fetch (during which other microtasks can run)
3. Commit results synchronously after await

The important JS consideration: during the `await` in step 2, other event handlers may modify the result set. This is analogous to the Rust concern about lock ordering. The JS solution is the same: re-validate state after the await before committing.

---

## 7. Complete File Template

```typescript
// MIRRORS: ankurah/core/src/reactor/fetch_gap.rs

import {
  type Predicate,
  type Selection,
  Selection as SelectionClass,
  type OrderByItem,
  type ComparisonOperator,
  type Expr,
  type Literal,
  PathExpr,
} from '@ankurah/ankql';
import type { CollectionId } from '@ankurah/proto';
import type { Entity } from '../entity.ts';
import type { Value } from '../value/index.ts';
import { ValueType, valueType, valueToLiteral } from '../value/index.ts';

// ── GapFetcher Interface ────────────────────────────────────────────

/**
 * Interface for fetching entities to fill gaps when LIMIT causes entities
 * to be evicted from a result set.
 *
 * MIRRORS: Rust trait GapFetcher<E: AbstractEntity>
 * Divergence: No generic E -- uses concrete Entity [E8].
 * Divergence: No Send + Sync bounds -- JS single-threaded [E8].
 */
export interface GapFetcher {
  fetchGap(
    collectionId: CollectionId,
    selection: Selection,
    lastEntity: Entity | null,
    gapSize: number,
  ): Promise<Entity[]>;
}

// ── QueryGapFetcher ─────────────────────────────────────────────────

// NOTE: QueryGapFetcher depends on a Node type that is not yet ported.
// The class below uses a placeholder `NodeLike` interface.
// Replace with the actual Node type when available.

/**
 * Minimal interface for the Node dependency used by QueryGapFetcher.
 * Will be replaced by the actual Node type when the node module is ported.
 */
interface NodeLike {
  fetchEntities(
    collectionId: CollectionId,
    selection: Selection,
  ): Promise<Entity[]>;
}

/**
 * Concrete implementation of GapFetcher using a Node reference.
 *
 * MIRRORS: Rust struct QueryGapFetcher<SE, PA>
 * Divergence: No StorageEngine/PolicyAgent type params [E8].
 * Divergence: WeakRef instead of Weak<NodeInner> [E8].
 * Divergence: No cdata -- node context handles this internally [E8].
 */
export class QueryGapFetcher implements GapFetcher {
  private nodeRef: WeakRef<NodeLike>;

  constructor(node: NodeLike) {
    this.nodeRef = new WeakRef(node);
  }

  async fetchGap(
    collectionId: CollectionId,
    selection: Selection,
    lastEntity: Entity | null,
    gapSize: number,
  ): Promise<Entity[]> {
    const node = this.nodeRef.deref();
    if (!node) {
      throw new Error('Node has been dropped, cannot fill gap');
    }

    let gapSelection: Selection;
    if (lastEntity !== null) {
      let gapPredicate: Predicate;
      if (selection.orderBy !== null) {
        gapPredicate = buildContinuationPredicate(
          selection.predicate,
          selection.orderBy,
          lastEntity,
        );
      } else {
        gapPredicate = selection.predicate;
      }
      gapSelection = new SelectionClass(
        gapPredicate,
        selection.orderBy,
        gapSize,
      );
    } else {
      gapSelection = new SelectionClass(
        selection.predicate,
        selection.orderBy,
        gapSize,
      );
    }

    return node.fetchEntities(collectionId, gapSelection);
  }
}

// ── buildContinuationPredicate ──────────────────────────────────────

/** Value types that are skipped for ORDER BY continuation (not orderable in AnkQL). */
const SKIP_VALUE_TYPES = new Set<string>(['Object', 'Binary', 'Json']);

/**
 * Build a supplemental predicate to fetch entities after the last entity
 * in sort order.
 *
 * For ORDER BY a ASC, b DESC with last entity having a=5, b=10:
 * Returns: originalPredicate AND a >= 5 AND b <= 10 AND id != lastEntity.id
 *
 * MIRRORS: Rust fn build_continuation_predicate()
 * Divergence: No generic E -- uses Entity [E8].
 * Divergence: Throws instead of Result [E7].
 */
export function buildContinuationPredicate(
  originalPredicate: Predicate,
  orderBy: OrderByItem[],
  lastEntity: Entity,
): Predicate {
  const gapConditions: Predicate[] = [];

  // 1. Add original predicate
  gapConditions.push(originalPredicate);

  // 2. Add ORDER BY continuation conditions
  for (const orderItem of orderBy) {
    const fieldName = orderItem.path.property();

    // Get the field value from the last entity
    const fieldValue = lastEntity.getPropertyValue(fieldName);
    if (fieldValue === null) {
      continue;
    }

    // Skip non-orderable types (Object, Binary, Json)
    if (SKIP_VALUE_TYPES.has(fieldValue.type)) {
      continue;
    }

    const literal = valueToLiteral(fieldValue);

    const operator: ComparisonOperator =
      orderItem.direction === 'Asc'
        ? 'GreaterThanOrEqual'
        : 'LessThanOrEqual';

    const condition: Predicate = {
      type: 'Comparison',
      left: { type: 'Path', value: orderItem.path },
      operator,
      right: { type: 'Literal', value: literal },
    };

    gapConditions.push(condition);
  }

  // 3. Add entity ID exclusion
  const idExclusion: Predicate = {
    type: 'Comparison',
    left: { type: 'Path', value: PathExpr.simple('id') },
    operator: 'NotEqual',
    right: {
      type: 'Literal',
      value: { type: 'EntityId', value: lastEntity.id().toBytes() },
    },
  };
  gapConditions.push(idExclusion);

  // 4. Combine all conditions with AND
  if (gapConditions.length === 0) {
    return { type: 'True' };
  }

  return gapConditions.reduce<Predicate>((acc, condition) => ({
    type: 'And',
    left: acc,
    right: condition,
  }));
}

// ── inferValueTypeForField ──────────────────────────────────────────

/**
 * Infer ValueType from the first non-null value in a collection of entities.
 *
 * MIRRORS: Rust fn infer_value_type_for_field()
 * Divergence: No generic E -- uses Entity [E8].
 */
export function inferValueTypeForField(
  entities: Entity[],
  fieldName: string,
): ValueType {
  for (const entity of entities) {
    const value = entity.getPropertyValue(fieldName);
    if (value !== null) {
      return valueType(value);
    }
  }
  // TODO: Get type from system catalog instead of defaulting to String
  return ValueType.String;
}
```

---

## 8. Testing Strategy

The Rust tests (fetch_gap.rs lines 179-258) should be ported as-is. They use a `TestEntity` mock that implements `AbstractEntity`. In TS, create a mock entity or use the real `Entity` class.

### 8.1 Test: single column ASC continuation predicate

```typescript
// Rust: test_build_gap_predicate_single_column_asc
// Entity with name="John", ORDER BY name ASC
// Expected: TRUE AND name >= 'John' AND id != <entity_id>
```

### 8.2 Test: multi-column continuation predicate

```typescript
// Rust: test_build_gap_predicate_multi_column
// Entity with name="John", age=30, ORDER BY name ASC, age DESC
// Expected: TRUE AND name >= 'John' AND age <= 30 AND id != <entity_id>
```

### 8.3 Test: infer value type for field

```typescript
// Rust: test_infer_value_type_for_field
// Entity 1: name="Alice", Entity 2: age=25
// inferValueTypeForField(entities, "name") === ValueType.String
// inferValueTypeForField(entities, "age") === ValueType.I32
// inferValueTypeForField(entities, "nonexistent") === ValueType.String
```

---

## 9. EntityResultSet Methods Required for Gap Filling

The following methods on `EntityResultSet` are used by the gap-filling logic in `subscription_state.rs` and must be available on the TS `EntityResultSet` when it is ported:

| Method | Signature | Purpose |
|---|---|---|
| `isGapDirty()` | `() => boolean` | Check if gap filling is needed |
| `clearGapDirty()` | `() => void` | Reset the gap_dirty flag after filling |
| `getLimit()` | `() => number \| null` | Get the configured limit |
| `len()` | `() => number` | Current entity count |
| `lastEntity()` | `() => Entity \| null` | Last entity in sort order (continuation point) |
| `write().add(entity)` | `(entity: Entity) => boolean` | Add entity, returns true if new |
| `containsKey(id)` | `(id: EntityId) => boolean` | Check if entity exists |

The `gap_dirty` flag is set automatically by `EntityResultSet` when:
- An entity is **removed** from a result set that was at its **limit** (see `resultset.rs` line 163: `guard.limit.is_some_and(|limit| guard.order.len() == limit)`)
- Entities are removed via `retain_dirty()` and the count drops below limit (see `resultset.rs` line 245)

---

## 10. Cross-Reference: Rust Source Line Numbers

| Item | File | Lines |
|---|---|---|
| `GapFetcher` trait | `fetch_gap.rs` | 16-34 |
| `QueryGapFetcher` struct | `fetch_gap.rs` | 37-45 |
| `QueryGapFetcher::new()` | `fetch_gap.rs` | 52 |
| `QueryGapFetcher::fetch_gap()` | `fetch_gap.rs` | 61-101 |
| `build_continuation_predicate()` | `fetch_gap.rs` | 107-165 |
| `infer_value_type_for_field()` | `fetch_gap.rs` | 168-177 |
| `QueryState` (uses GapFetcher) | `subscription_state.rs` | 58-68 |
| `extract_gap_data()` | `subscription_state.rs` | 608-637 |
| `process_gap_fill_entities()` | `subscription_state.rs` | 532-568 |
| `fill_gaps_and_notify()` | `subscription_state.rs` | 572-606 |
| `evaluate_changes()` (triggers gap) | `subscription_state.rs` | 360-469 |
| `build_key_spec_from_selection()` | `reactor.rs` | 196-219 |
| `EntityResultSet` (gap_dirty) | `resultset.rs` | 70-77, 447-468 |
| Integration test (single node) | `limit_gap_filling.rs` | 9-36 |
| Integration test (inter-node) | `limit_gap_filling.rs` | 79-104 |
| Design spec (completed) | `query_add_update_gapfiller_plan.md` | 1-141 |
