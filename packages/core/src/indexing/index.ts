// MIRRORS: ankurah/core/src/indexing/mod.rs

export {
  IndexDirection,
  NullsOrder,
  IndexSpecMatch,
  type IndexKeyPart,
  type KeySpec,
  indexKeyPartAsc,
  indexKeyPartDesc,
  indexKeyPartFromPath,
  indexKeyPartFromFlatPath,
  indexKeyPartFullPath,
  indexKeyPartAscPath,
  indexKeyPartDescPath,
  isDesc,
  keySpecNew,
  keySpecNameWith,
  keySpecMatches,
  keySpecEquals,
} from './key_spec.ts';

export {
  IndexError,
  encodeComponentTyped,
  encodeTupleValuesWithKeySpec,
} from './encoding.ts';
