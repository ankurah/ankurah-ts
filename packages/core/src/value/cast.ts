// MIRRORS: ankurah/core/src/value/cast.rs

import { EntityId } from '@ankurah/proto';
import type { Value } from './index';
import { ValueType, valueType } from './index';

// ── CastError ────────────────────────────────────────────────────────

export type CastError =
  | { type: 'IncompatibleTypes'; from: ValueType; to: ValueType }
  | { type: 'InvalidFormat'; value: string; targetType: ValueType }
  | { type: 'NumericOverflow'; value: string; targetType: ValueType };

export class CastErrorException extends Error {
  readonly castError: CastError;

  constructor(castError: CastError) {
    super(castErrorToString(castError));
    this.name = 'CastErrorException';
    this.castError = castError;
  }
}

function castErrorToString(err: CastError): string {
  switch (err.type) {
    case 'IncompatibleTypes':
      return `Cannot cast from ${err.from} to ${err.to}`;
    case 'InvalidFormat':
      return `Invalid format '${err.value}' for type ${err.targetType}`;
    case 'NumericOverflow':
      return `Numeric overflow: '${err.value}' cannot fit in ${err.targetType}`;
  }
}

// ── Numeric range constants ──────────────────────────────────────────
// Divergence: Rust has i16/i32/i64 distinct integer types; TS uses number for i16/i32 and
// bigint or number for i64. Since the Value union uses number for I16/I32/I64 and F64,
// we define JS-side range checks. [E8]

const I16_MIN = -32768;
const I16_MAX = 32767;
const I32_MIN = -2147483648;
const I32_MAX = 2147483647;
// For I64 we use Number.MIN_SAFE_INTEGER / MAX_SAFE_INTEGER since TS number cannot
// represent the full i64 range. The Rust code uses i64 min/max.
const I64_MIN = Number.MIN_SAFE_INTEGER;
const I64_MAX = Number.MAX_SAFE_INTEGER;

// ── castTo ───────────────────────────────────────────────────────────

/** Cast a Value to the specified target ValueType. Throws CastErrorException on failure. */
export function castTo(value: Value, targetType: ValueType): Value {
  const sourceType = valueType(value);

  // If already the target type, return as-is
  if (sourceType === targetType) {
    return value;
  }

  // String to EntityId conversion
  if (value.type === 'String' && targetType === ValueType.EntityId) {
    try {
      const entityId = EntityId.fromBase64(value.value);
      return { type: 'EntityId', value: entityId };
    } catch {
      throw new CastErrorException({ type: 'InvalidFormat', value: value.value, targetType: ValueType.EntityId });
    }
  }

  // EntityId to String conversion
  if (value.type === 'EntityId' && targetType === ValueType.String) {
    return { type: 'String', value: value.value.toBase64() };
  }

  // ── Numeric conversions ────────────────────────────────────────────

  // I16 -> wider types
  if (value.type === 'I16' && targetType === ValueType.I32) return { type: 'I32', value: value.value };
  if (value.type === 'I16' && targetType === ValueType.I64) return { type: 'I64', value: value.value };
  if (value.type === 'I16' && targetType === ValueType.F64) return { type: 'F64', value: value.value };

  // I32 -> I16 (with overflow check)
  if (value.type === 'I32' && targetType === ValueType.I16) {
    if (value.value >= I16_MIN && value.value <= I16_MAX) {
      return { type: 'I16', value: value.value };
    }
    throw new CastErrorException({ type: 'NumericOverflow', value: String(value.value), targetType: ValueType.I16 });
  }
  if (value.type === 'I32' && targetType === ValueType.I64) return { type: 'I64', value: value.value };
  if (value.type === 'I32' && targetType === ValueType.F64) return { type: 'F64', value: value.value };

  // I64 -> I16 (with overflow check)
  if (value.type === 'I64' && targetType === ValueType.I16) {
    if (value.value >= I16_MIN && value.value <= I16_MAX) {
      return { type: 'I16', value: value.value };
    }
    throw new CastErrorException({ type: 'NumericOverflow', value: String(value.value), targetType: ValueType.I16 });
  }
  // I64 -> I32 (with overflow check)
  if (value.type === 'I64' && targetType === ValueType.I32) {
    if (value.value >= I32_MIN && value.value <= I32_MAX) {
      return { type: 'I32', value: value.value };
    }
    throw new CastErrorException({ type: 'NumericOverflow', value: String(value.value), targetType: ValueType.I32 });
  }
  if (value.type === 'I64' && targetType === ValueType.F64) return { type: 'F64', value: value.value };

  // F64 -> integer types (with finite + range check)
  if (value.type === 'F64' && targetType === ValueType.I16) {
    if (Number.isFinite(value.value) && value.value >= I16_MIN && value.value <= I16_MAX) {
      return { type: 'I16', value: Math.trunc(value.value) };
    }
    throw new CastErrorException({ type: 'NumericOverflow', value: String(value.value), targetType: ValueType.I16 });
  }
  if (value.type === 'F64' && targetType === ValueType.I32) {
    if (Number.isFinite(value.value) && value.value >= I32_MIN && value.value <= I32_MAX) {
      return { type: 'I32', value: Math.trunc(value.value) };
    }
    throw new CastErrorException({ type: 'NumericOverflow', value: String(value.value), targetType: ValueType.I32 });
  }
  if (value.type === 'F64' && targetType === ValueType.I64) {
    if (Number.isFinite(value.value) && value.value >= I64_MIN && value.value <= I64_MAX) {
      return { type: 'I64', value: Math.trunc(value.value) };
    }
    throw new CastErrorException({ type: 'NumericOverflow', value: String(value.value), targetType: ValueType.I64 });
  }

  // ── String to numeric conversions ──────────────────────────────────

  if (value.type === 'String' && targetType === ValueType.I16) {
    const n = parseInt(value.value, 10);
    if (!isNaN(n) && String(n) === value.value.trim() && n >= I16_MIN && n <= I16_MAX) {
      return { type: 'I16', value: n };
    }
    throw new CastErrorException({ type: 'InvalidFormat', value: value.value, targetType: ValueType.I16 });
  }
  if (value.type === 'String' && targetType === ValueType.I32) {
    const n = parseInt(value.value, 10);
    if (!isNaN(n) && String(n) === value.value.trim() && n >= I32_MIN && n <= I32_MAX) {
      return { type: 'I32', value: n };
    }
    throw new CastErrorException({ type: 'InvalidFormat', value: value.value, targetType: ValueType.I32 });
  }
  if (value.type === 'String' && targetType === ValueType.I64) {
    const n = parseInt(value.value, 10);
    if (!isNaN(n) && String(n) === value.value.trim() && n >= I64_MIN && n <= I64_MAX) {
      return { type: 'I64', value: n };
    }
    throw new CastErrorException({ type: 'InvalidFormat', value: value.value, targetType: ValueType.I64 });
  }
  if (value.type === 'String' && targetType === ValueType.F64) {
    const n = parseFloat(value.value);
    if (!isNaN(n)) {
      return { type: 'F64', value: n };
    }
    throw new CastErrorException({ type: 'InvalidFormat', value: value.value, targetType: ValueType.F64 });
  }
  if (value.type === 'String' && targetType === ValueType.Bool) {
    switch (value.value.toLowerCase()) {
      case 'true': case '1': case 'yes': case 'on':
        return { type: 'Bool', value: true };
      case 'false': case '0': case 'no': case 'off':
        return { type: 'Bool', value: false };
      default:
        throw new CastErrorException({ type: 'InvalidFormat', value: value.value, targetType: ValueType.Bool });
    }
  }

  // ── Numeric to string conversions ──────────────────────────────────

  if (value.type === 'I16' && targetType === ValueType.String) return { type: 'String', value: String(value.value) };
  if (value.type === 'I32' && targetType === ValueType.String) return { type: 'String', value: String(value.value) };
  if (value.type === 'I64' && targetType === ValueType.String) return { type: 'String', value: String(value.value) };
  if (value.type === 'F64' && targetType === ValueType.String) return { type: 'String', value: String(value.value) };
  if (value.type === 'Bool' && targetType === ValueType.String) return { type: 'String', value: String(value.value) };

  // ── Bool to numeric conversions ────────────────────────────────────

  if (value.type === 'Bool' && targetType === ValueType.I16) return { type: 'I16', value: value.value ? 1 : 0 };
  if (value.type === 'Bool' && targetType === ValueType.I32) return { type: 'I32', value: value.value ? 1 : 0 };
  if (value.type === 'Bool' && targetType === ValueType.I64) return { type: 'I64', value: value.value ? 1 : 0 };
  if (value.type === 'Bool' && targetType === ValueType.F64) return { type: 'F64', value: value.value ? 1.0 : 0.0 };

  // ── Numeric to bool conversions ────────────────────────────────────

  if (value.type === 'I16' && targetType === ValueType.Bool) return { type: 'Bool', value: value.value !== 0 };
  if (value.type === 'I32' && targetType === ValueType.Bool) return { type: 'Bool', value: value.value !== 0 };
  if (value.type === 'I64' && targetType === ValueType.Bool) return { type: 'Bool', value: value.value !== 0 };
  if (value.type === 'F64' && targetType === ValueType.Bool) return { type: 'Bool', value: value.value !== 0.0 };

  // ── Cast TO Json ───────────────────────────────────────────────────

  if (value.type === 'String' && targetType === ValueType.Json) return { type: 'Json', value: value.value };
  if (value.type === 'I64' && targetType === ValueType.Json) return { type: 'Json', value: value.value };
  if (value.type === 'I32' && targetType === ValueType.Json) return { type: 'Json', value: value.value };
  if (value.type === 'I16' && targetType === ValueType.Json) return { type: 'Json', value: value.value };
  if (value.type === 'F64' && targetType === ValueType.Json) return { type: 'Json', value: value.value };
  if (value.type === 'Bool' && targetType === ValueType.Json) return { type: 'Json', value: value.value };

  // ── Cast FROM Json ─────────────────────────────────────────────────

  if (value.type === 'Json' && targetType === ValueType.String) {
    if (typeof value.value === 'string') return { type: 'String', value: value.value };
    throw new CastErrorException({ type: 'IncompatibleTypes', from: sourceType, to: targetType });
  }
  if (value.type === 'Json' && targetType === ValueType.I64) {
    if (typeof value.value === 'number' && Number.isInteger(value.value)) return { type: 'I64', value: value.value };
    throw new CastErrorException({ type: 'IncompatibleTypes', from: sourceType, to: targetType });
  }
  if (value.type === 'Json' && targetType === ValueType.I32) {
    if (typeof value.value === 'number' && Number.isInteger(value.value)) {
      const i = value.value;
      if (i >= I32_MIN && i <= I32_MAX) return { type: 'I32', value: i };
      throw new CastErrorException({ type: 'NumericOverflow', value: String(i), targetType: ValueType.I32 });
    }
    throw new CastErrorException({ type: 'IncompatibleTypes', from: sourceType, to: targetType });
  }
  if (value.type === 'Json' && targetType === ValueType.I16) {
    if (typeof value.value === 'number' && Number.isInteger(value.value)) {
      const i = value.value;
      if (i >= I16_MIN && i <= I16_MAX) return { type: 'I16', value: i };
      throw new CastErrorException({ type: 'NumericOverflow', value: String(i), targetType: ValueType.I16 });
    }
    throw new CastErrorException({ type: 'IncompatibleTypes', from: sourceType, to: targetType });
  }
  if (value.type === 'Json' && targetType === ValueType.F64) {
    if (typeof value.value === 'number') return { type: 'F64', value: value.value };
    throw new CastErrorException({ type: 'IncompatibleTypes', from: sourceType, to: targetType });
  }
  if (value.type === 'Json' && targetType === ValueType.Bool) {
    if (typeof value.value === 'boolean') return { type: 'Bool', value: value.value };
    throw new CastErrorException({ type: 'IncompatibleTypes', from: sourceType, to: targetType });
  }

  // All other combinations are incompatible
  throw new CastErrorException({ type: 'IncompatibleTypes', from: sourceType, to: targetType });
}

/** Try to cast a Value to the specified target type, returning null if the cast fails. */
export function tryCastTo(value: Value, targetType: ValueType): Value | null {
  try {
    return castTo(value, targetType);
  } catch {
    return null;
  }
}
