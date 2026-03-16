// MIRRORS: ankurah/storage/sqlite/src/error.rs

/**
 * Error types for SQLite storage engine.
 *
 * Rust: `pub enum SqliteError { Rusqlite, Pool, Serialization, Json, DDL, SqlGeneration, TaskJoin }`
 * Divergence: TS uses a simple Error subclass with kind discriminator instead of thiserror [E8].
 * Divergence: No Rusqlite/Pool/TaskJoin variants — TS SQLite drivers surface different errors [E16].
 */

export type SqliteErrorKind =
  | 'SqliteDriver'
  | 'Pool'
  | 'Serialization'
  | 'Json'
  | 'DDL'
  | 'SqlGeneration';

export class SqliteError extends Error {
  readonly kind: SqliteErrorKind;

  constructor(kind: SqliteErrorKind, message: string) {
    super(message);
    this.name = 'SqliteError';
    this.kind = kind;
  }

  static driver(message: string): SqliteError {
    return new SqliteError('SqliteDriver', `SQLite error: ${message}`);
  }

  static pool(message: string): SqliteError {
    return new SqliteError('Pool', `Connection pool error: ${message}`);
  }

  static serialization(message: string): SqliteError {
    return new SqliteError('Serialization', `Serialization error: ${message}`);
  }

  static json(message: string): SqliteError {
    return new SqliteError('Json', `JSON error: ${message}`);
  }

  static ddl(message: string): SqliteError {
    return new SqliteError('DDL', `DDL error: ${message}`);
  }

  static sqlGeneration(message: string): SqliteError {
    return new SqliteError('SqlGeneration', `SQL generation error: ${message}`);
  }
}
