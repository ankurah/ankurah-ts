// MIRRORS: ankurah/core/src/schema.rs
import { Result } from '@ankurah/base';
import { PathExpr } from '@ankurah/ankql';

export interface CollectionSchema {
  fieldType(path: PathExpr): Result<ValueType, PropertyError>;
}

