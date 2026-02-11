// MIRRORS: ankurah/core/src/lib.rs
//
// @ankurah/core — Core entity, model, transaction, and node logic.
//
// This package contains the central abstractions: Entity, Node, Context,
// Transaction, Model/View/Mutable, property backends (LWW, Yjs), Reactor,
// LiveQuery, and the defineModel() API.
//
// Rust crate: ankurah-core
// Key types: Node, Entity, Context, Transaction, Model, View, Mutable,
//            PropertyBackend, LwwBackend, YjsBackend, Reactor, LiveQuery
//
// ── Error types ──
export { AccessDenied, MutationError, RetrievalError, StateError, SubscriptionError } from './error.ts';

// ── Value types ──
export type { Value } from './value/index.ts';
export { CastError } from './value/cast.ts';

// ── Property system ──
export type { PropertyName, Property } from './property/index.ts';
export { PropertyError } from './property/traits.ts';
export type { PropertyBackend } from './property/backend/index.ts';
export { LWWBackend } from './property/backend/lww.ts';
export { YjsBackend } from './property/backend/yjs.ts';
export { LWW as LWWValue } from './property/value/lww.ts';
export { YrsString, stringFromYrsString, optionalStringFromYrsString } from './property/value/yrs_string.ts';

// ── Entity ──
export { Entity, WeakEntitySet } from './entity.ts';
export type { EntityKind } from './entity.ts';

// ── Model / defineModel ──
export type { ViewInstance, MutableInstance, ViewConstructor, ModelDefinition } from './model.ts';
export { defineModel, lww, yrsText, ephemeral } from './define-model.ts';
export type { FieldDefinition, FieldMetadata, DefinedModel, LWW, YjsText, BackendKind } from './define-model.ts';

// ── Model extras ──
export { MutableBorrow } from './model.ts';

// ── Transaction ──
export { Transaction } from './transaction.ts';

// ── Context ──
export { Context } from './context.ts';
export type { TContext } from './context.ts';

// ── Changes ──
export { EntityChange, itemChangeItem, itemChangeEvents, itemChangeKind } from './changes.ts';
export type { ItemChange, ChangeKind } from './changes.ts';

// ── Node ──
export { Node, NodeAndContext, matchArgs } from './node.ts';
export type { MatchArgs } from './node.ts';

// ── Storage ──
export type { StorageEngine, StorageCollection } from './storage.ts';

// ── Policy ──
export type { PolicyAgent } from './policy.ts';
export { OpenPolicy } from './policy.ts';

// ── Indexing ──
export { IndexDirection, NullsOrder, IndexSpecMatch } from './indexing/index.ts';
export type { IndexKeyPart, KeySpec } from './indexing/index.ts';
export { IndexError, encodeTupleValuesWithKeySpec } from './indexing/index.ts';
export {
  indexKeyPartAsc, indexKeyPartDesc, indexKeyPartFromPath, indexKeyPartFromFlatPath,
  keySpecNew, keySpecEquals,
} from './indexing/index.ts';

// ── ResultSet ──
export { EntityResultSet, ResultSetWrite, ResultSetRead } from './resultset.ts';

// ── Reactor ──
export { ComparisonIndex } from './reactor/comparison-index.ts';
export { PropertyPath } from './reactor/property-path.ts';
export { CandidateChanges } from './reactor/candidate-changes.ts';
export { ReactorSubscriptionId, WatcherSet } from './reactor/watcher_set.ts';
export type { WatcherOp, EntityWatcherId, WatcherChange, WatcherIdPair } from './reactor/watcher_set.ts';
export { entityWatcherIdKey, watcherChangeAdd, watcherChangeRemove } from './reactor/watcher_set.ts';
export type { GapFetcher } from './reactor/fetch_gap.ts';
export { QueryGapFetcher, buildContinuationPredicate, inferValueTypeForField } from './reactor/fetch_gap.ts';
export type { MembershipChange, ReactorUpdateItem } from './reactor/update.ts';
export { ReactorUpdate } from './reactor/update.ts';

// ── Selection/Filter ──
export type { Filterable } from './selection/filter.ts';
export { evaluatePredicate } from './selection/filter.ts';

// TODO: Port remaining types from ankurah/core/src/ (Reactor main, LiveQuery, Subscription)
