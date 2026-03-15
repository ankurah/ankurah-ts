// MIRRORS: ankurah/core/src/reactor/fetch_gap.rs

import {
  type Predicate,
  type Selection,
  Selection as SelectionClass,
  type OrderByItem,
  type ComparisonOperator,
  PathExpr,
} from '@ankurah/ankql';
import type { CollectionId } from '@ankurah/proto';
import type { Entity } from '../entity.ts';
import { ValueType, valueType, valueToLiteral } from '../value/index.ts';

// ── GapFetcher ────────────────────────────────────────────────────────
// Rust: pub trait GapFetcher<E: AbstractEntity>: Send + Sync + 'static
// Divergence: No generic E — uses concrete Entity [E8].

export interface GapFetcher {
  fetchGap(
    collectionId: CollectionId,
    selection: Selection,
    lastEntity: Entity | null,
    gapSize: number,
  ): Promise<Entity[]>;
}

// ── NodeLike ──────────────────────────────────────────────────────────
// Rust: Uses NodeAndContext::fetch_entities() directly.
// Divergence: Placeholder interface to break circular import [E8].

export interface NodeLike {
  fetchEntities(
    collectionId: CollectionId,
    selection: Selection,
  ): Promise<Entity[]>;
}

// ── QueryGapFetcher ───────────────────────────────────────────────────
// Rust: pub struct QueryGapFetcher<SE, PA> { weak_node: Weak<NodeInner<SE, PA>>, cdata: PA::ContextData }
// Divergence: No SE/PA type params [E8].
// Divergence: WeakRef instead of Weak<NodeInner> [E8].
// Divergence: No cdata — node context handles this internally [E8].

export class QueryGapFetcher implements GapFetcher {
  // Rust: weak_node: Weak<NodeInner<SE, PA>>
  private nodeRef: WeakRef<NodeLike>;

  // Rust: pub fn new(node: &Node<SE, PA>, cdata: PA::ContextData) -> Self
  constructor(node: NodeLike) {
    this.nodeRef = new WeakRef(node);
  }

  // Rust: async fn fetch_gap(&self, collection_id, selection, last_entity, gap_size)
  async fetchGap(
    collectionId: CollectionId,
    selection: Selection,
    lastEntity: Entity | null,
    gapSize: number,
  ): Promise<Entity[]> {
    // Upgrade weak reference — mirrors Rust self.weak_node.upgrade()
    const node = this.nodeRef.deref();
    if (!node) {
      throw new Error('Node has been dropped, cannot fill gap');
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
      gapSelection = new SelectionClass(
        gapPredicate,
        selection.orderBy,
        gapSize,
      );
    } else {
      // No last entity — use original selection with gap_size limit
      gapSelection = new SelectionClass(
        selection.predicate,
        selection.orderBy,
        gapSize,
      );
    }

    return node.fetchEntities(collectionId, gapSelection);
  }
}

// ── buildContinuationPredicate ────────────────────────────────────────
// Rust: pub fn build_continuation_predicate<E: AbstractEntity>(...)
// For ORDER BY a ASC, b DESC with last entity having a=5, b=10:
// Returns: originalPredicate AND a >= 5 AND b <= 10 AND id != lastEntity.id
// Divergence: No generic E — uses concrete Entity [E8].
// Divergence: Throws instead of Result [E7].

/** Value types skipped for ORDER BY continuation (not orderable in AnkQL). */
const SKIP_VALUE_TYPES = new Set<string>(['Object', 'Binary', 'Json']);

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

    // Skip Object, Binary, Json — not commonly used in ORDER BY (mirrors Rust continue arm)
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

  // 3. Add entity ID exclusion to avoid fetching the last entity again
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

  return gapConditions.reduce((acc: Predicate, condition: Predicate): Predicate => ({
    type: 'And',
    left: acc,
    right: condition,
  }));
}

// ── inferValueTypeForField ────────────────────────────────────────────
// Rust: pub fn infer_value_type_for_field<E: AbstractEntity>(entities: &[E], field_name: &str) -> ValueType
// Divergence: No generic E — uses concrete Entity [E8].

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
