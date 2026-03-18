// MIRRORS: ankurah/ankurah/src/lib.rs
//
// @ankurah/ankurah — Facade package for Ankurah.
//
// Observable, event-driven state management for native and web.
// Re-exports commonly used types from core, proto, signals, and ankql.
//
// Rust crate: ankurah

// Rust: pub use ankql;
// Divergence: TS re-exports specific types rather than the whole module [E8]
export {
  Selection,
  Predicate,
  Expr,
  Literal,
  PathExpr,
  ComparisonOperator,
  OrderByItem,
  OrderDirection,
  parseSelection,
} from '@ankurah/ankql';

// Rust: pub use ankurah_proto as proto;
// Re-export commonly used proto types
export {
  EntityId,
  EventId,
  CollectionId,
  QueryId,
  Clock,
  Presence,
  Message,
  NodeMessage,
} from '@ankurah/proto';
export type {
  EntityState,
  KnownEntity,
} from '@ankurah/proto';

// Rust: pub use ankurah_signals as signals;
// Re-export commonly used signal types
export {
  Mut,
  Read,
  Calculated,
  Memo,
  ListenerGuard,
  SubscriptionGuard,
  waitFor,
  waitValue,
} from '@ankurah/signals';
export type {
  Signal,
  Get,
  Subscribe,
  Wait,
} from '@ankurah/signals';

// Rust: pub use ankurah_core::{...}
// Re-export commonly used core types (matching Rust facade re-exports)

// Rust: pub use ankurah_core::context::Context;
export { Context } from '@ankurah/core';
export type { TContext } from '@ankurah/core';

// Rust: pub use ankurah_core::entity;
export { Entity } from '@ankurah/core';

// Rust: pub use ankurah_core::error;
export {
  AccessDenied,
  MutationError,
  RetrievalError,
  StateError,
  SubscriptionError,
  SendError,
} from '@ankurah/core';

// Rust: pub use ankurah_core::livequery::LiveQuery;
export { LiveQuery, EntityLiveQuery } from '@ankurah/core';

// Rust: pub use ankurah_core::model;
// Rust: pub use ankurah_core::model::{Model, View, Mutable};
export { defineModel, lww, yrsText, ephemeral } from '@ankurah/core';
export type { ViewInstance, MutableInstance, ModelDefinition, DefinedModel } from '@ankurah/core';

// Rust: pub use ankurah_core::node::{MatchArgs, Node};
export { Node, matchArgs } from '@ankurah/core';
export type { MatchArgs } from '@ankurah/core';

// Rust: pub use ankurah_core::policy::{self, PermissiveAgent};
export { OpenPolicy } from '@ankurah/core';
export type { PolicyAgent } from '@ankurah/core';
// Divergence: Rust exports PermissiveAgent; TS exports OpenPolicy (alias) [E8]

// Rust: pub use ankurah_core::property::{self, Property, Ref};
export type { Property } from '@ankurah/core';
// Divergence: property::Ref (entity reference) not yet exported from @ankurah/core index [E4]

// Rust: pub use ankurah_core::query_value::QueryValue;
export type { QueryValue } from '@ankurah/core';

// Rust: pub use ankurah_core::resultset::ResultSet;
export { EntityResultSet, ResultSetWrite, ResultSetRead } from '@ankurah/core';

// Rust: pub use ankurah_core::storage;
export type { StorageEngine, StorageCollection } from '@ankurah/core';

// Rust: pub use ankurah_core::transaction;
export { Transaction } from '@ankurah/core';

// Rust: pub use ankurah_core::value::{Value, ValueType};
export type { Value } from '@ankurah/core';
export { ValueType } from '@ankurah/core';

// Rust: pub use ankurah_core::changes;
export { EntityChange } from '@ankurah/core';

// Rust: pub use ankurah_core::connector;
export type { PeerSender, NodeComms } from '@ankurah/core';

// Divergence: Rust re-exports derive macros and derive_deps — no equivalent in TS [E8].
// Divergence: Rust re-exports create! and into! macros — TS uses defineModel() instead [E8].
// Divergence: Rust re-exports set_runtime_handle (Tokio) — not applicable in JS [E8].
