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
export { ValueType, valueType, valueFromLiteral, valuePartialCmp, valueEquals, valueGt, valueGe, valueLt, valueLe } from './value/index.ts';
export type { CastError } from './value/cast.ts';

// ── Property system ──
export type { PropertyName, Property } from './property/index.ts';
export { PropertyError } from './property/traits.ts';
export type { PropertyBackend } from './property/backend/index.ts';
export { backendFromString } from './property/backend/index.ts';
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
export type { ItemChange, ChangeKind, ChangeSet } from './changes.ts';

// ── Node ──
export { Node, NodeAndContext, matchArgs } from './node.ts';
export type { MatchArgs } from './node.ts';

// ── Schema ──
export type { CollectionSchema } from './schema.ts';

// ── Query Value ──
export type { QueryValue } from './query_value.ts';
export {
  queryValueString, queryValueInt, queryValueFloat, queryValueBool, queryValueEntityId,
  queryValueToExpr,
} from './query_value.ts';

// ── Connector ──
export { SendError } from './connector.ts';
export type { PeerSender, NodeComms } from './connector.ts';

// ── Storage ──
export type { StorageEngine, StorageCollection } from './storage.ts';

// ── Retrieval ──
export type { TEvent, TClock, GetEvents, Retrieve } from './retrieval.ts';
export { clockMembers, eventAsTEvent, LocalRetriever } from './retrieval.ts';

// ── CollectionSet ──
export { CollectionSet } from './collectionset.ts';

// ── Policy ──
export type { PolicyAgent } from './policy.ts';
export { OpenPolicy } from './policy.ts';

// ── Indexing ──
export { IndexDirection, NullsOrder, IndexSpecMatch } from './indexing/index.ts';
export type { IndexKeyPart, KeySpec } from './indexing/index.ts';
export { IndexError, encodeTupleValuesWithKeySpec } from './indexing/index.ts';
export {
  indexKeyPartAsc, indexKeyPartDesc, indexKeyPartFromPath, indexKeyPartFromFlatPath,
  indexKeyPartAscPath, indexKeyPartDescPath, indexKeyPartFullPath,
  keySpecNew, keySpecEquals, keySpecNameWith,
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
export type { ReactorUpdate } from './reactor/update.ts';

// ── Lineage ──
export { EventAccumulator, compare, compareUnstoredEvent, compareWithAccumulator } from './lineage.ts';
export type { Ordering, LClock, LEvent, LGetEvents, LAttested } from './lineage.ts';

// ── Selection/Filter ──
export type { Filterable } from './selection/filter.ts';
export { evaluatePredicate } from './selection/filter.ts';

// ── System ──
export { SystemManager, SYSTEM_COLLECTION_ID, PROTECTED_COLLECTIONS, sysItemToValue, sysItemFromValue } from './system.ts';

// ── Reactor main ──
export { Reactor } from './reactor/index.ts';
export type { PreNotifyHook, ReactorNodeLike } from './reactor/index.ts';
export { Subscription, VecAccumulator, NoopAccumulator, buildKeySpecFromSelection } from './reactor/subscription_state.ts';
export type { ChangeNotification, UpdateItemAccumulator, QueryState } from './reactor/subscription_state.ts';
export { ReactorSubscription } from './reactor/subscription.ts';
export { hasMembershipChange } from './reactor/update.ts';
export type { ReactorUpdate as ReactorUpdateType } from './reactor/update.ts';

// ── LiveQuery ──
export { EntityLiveQuery, WeakEntityLiveQuery, LiveQuery } from './livequery.ts';
export type { RemoteQuerySubscriber } from './livequery.ts';

// ── NodeApplier ──
export { NodeApplier } from './node_applier.ts';

// ── base (TS-ONLY: Rust ownership primitives, see E11)
export { AkObject, Struct, Enum, Drop, DropGuard, Arc, Weak, Mutex, MutexGuard, RefCell, Ref, RefMut, Borrow, BorrowMut } from '@ankurah/base';

// TODO: Port remaining types from ankurah/core/src/ (peer_subscription)
