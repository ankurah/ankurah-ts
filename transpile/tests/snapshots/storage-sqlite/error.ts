// MIRRORS: ankurah/storage/sqlite/src/error.rs
import { Enum } from '@ankurah/base';
import { MutationError, RetrievalError, StateError } from '@ankurah/core';

export type SqliteErrorV = {
  Rusqlite: { _0: Error };
  Pool: { _0: string };
  Serialization: { _0: Error };
  Json: { _0: Error };
  DDL: { _0: string };
  SqlGeneration: { _0: string };
  TaskJoin: { _0: string };
};

export class SqliteError extends Enum<SqliteErrorV> {

  debug(): string {
    return this.match({
      Rusqlite: (v) => `Rusqlite(${v._0})`,
      Pool: (v) => `Pool(${JSON.stringify(v._0)})`,
      Serialization: (v) => `Serialization(${v._0})`,
      Json: (v) => `Json(${v._0})`,
      DDL: (v) => `DDL(${JSON.stringify(v._0)})`,
      SqlGeneration: (v) => `SqlGeneration(${JSON.stringify(v._0)})`,
      TaskJoin: (v) => `TaskJoin(${JSON.stringify(v._0)})`,
    });
  }

  override toString(): string {
    return this.match({
      Rusqlite: (v) => `SQLite error: ${v._0}`,
      Pool: (v) => `Connection pool error: ${v._0}`,
      Serialization: (v) => `Serialization error: ${v._0}`,
      Json: (v) => `JSON error: ${v._0}`,
      DDL: (v) => `DDL error: ${v._0}`,
      SqlGeneration: (v) => `SQL generation error: ${v._0}`,
      TaskJoin: (v) => `Task join error: ${v._0}`,
    });
  }

  /** The error this one wraps: Rust's `Error::source`. */
  source(): unknown {
    switch (this.type) {
      case 'Rusqlite': return (this.value as any)._0;
      case 'Serialization': return (this.value as any)._0;
      case 'Json': return (this.value as any)._0;
      default: return null;
    }
  }

  static fromRusqliteError(inner: Error): SqliteError {
    return new SqliteError('Rusqlite', { _0: inner });
  }

  static fromBincodeError(inner: Error): SqliteError {
    return new SqliteError('Serialization', { _0: inner });
  }

  static fromSerdeJsonError(inner: Error): SqliteError {
    return new SqliteError('Json', { _0: inner });
  }
}

export function RetrievalError_fromSqliteError(err: SqliteError): RetrievalError {
  return new RetrievalError('StorageError', { _0: err });
}

export function MutationError_fromSqliteError(err: SqliteError): MutationError {
  return new MutationError('General', { _0: err });
}

export function StateError_fromSqliteError(err: SqliteError): StateError {
  return new StateError('DDLError', { _0: err });
}

