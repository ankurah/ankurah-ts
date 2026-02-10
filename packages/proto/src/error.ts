// MIRRORS: ankurah/proto/src/error.rs

/**
 * Error type for decoding failures.
 * Mirrors Rust `DecodeError` enum as a TS Error subclass.
 */
export class DecodeError extends Error {
  readonly kind: DecodeErrorKind;

  constructor(kind: DecodeErrorKind, message?: string) {
    super(message ?? decodeErrorMessage(kind));
    this.name = 'DecodeError';
    this.kind = kind;
  }

  static notStringValue(): DecodeError {
    return new DecodeError('NotStringValue');
  }
  static invalidBase64(detail?: string): DecodeError {
    return new DecodeError('InvalidBase64', `Invalid Base64${detail ? ': ' + detail : ''}`);
  }
  static invalidLength(): DecodeError {
    return new DecodeError('InvalidLength');
  }
  static invalidUlid(): DecodeError {
    return new DecodeError('InvalidUlid');
  }
  static invalidFallback(): DecodeError {
    return new DecodeError('InvalidFallback');
  }
  static invalidFormat(): DecodeError {
    return new DecodeError('InvalidFormat');
  }
  static other(message: string): DecodeError {
    return new DecodeError('Other', `Other: ${message}`);
  }
}

export type DecodeErrorKind =
  | 'NotStringValue'
  | 'InvalidBase64'
  | 'InvalidLength'
  | 'InvalidUlid'
  | 'InvalidFallback'
  | 'InvalidFormat'
  | 'Other';

function decodeErrorMessage(kind: DecodeErrorKind): string {
  switch (kind) {
    case 'NotStringValue': return 'Not a string value';
    case 'InvalidBase64': return 'Invalid Base64';
    case 'InvalidLength': return 'Invalid Length';
    case 'InvalidUlid': return 'Invalid ULID';
    case 'InvalidFallback': return 'Invalid Fallback';
    case 'InvalidFormat': return 'Invalid Format';
    case 'Other': return 'Other';
  }
}

/**
 * Simplified error type for ID parsing.
 * Mirrors Rust `IdParseError` enum.
 */
export class IdParseError extends Error {
  readonly kind: IdParseErrorKind;

  constructor(kind: IdParseErrorKind, message?: string) {
    super(message ?? idParseErrorMessage(kind));
    this.name = 'IdParseError';
    this.kind = kind;
  }

  static invalidBase64(): IdParseError {
    return new IdParseError('InvalidBase64');
  }
  static invalidLength(): IdParseError {
    return new IdParseError('InvalidLength');
  }
  static invalidFormat(detail: string): IdParseError {
    return new IdParseError('InvalidFormat', `Invalid format: ${detail}`);
  }

  static fromDecodeError(e: DecodeError): IdParseError {
    switch (e.kind) {
      case 'InvalidBase64': return IdParseError.invalidBase64();
      case 'InvalidLength': return IdParseError.invalidLength();
      default: return IdParseError.invalidFormat(e.message);
    }
  }
}

export type IdParseErrorKind =
  | 'InvalidBase64'
  | 'InvalidLength'
  | 'InvalidFormat';

function idParseErrorMessage(kind: IdParseErrorKind): string {
  switch (kind) {
    case 'InvalidBase64': return 'Invalid base64 encoding';
    case 'InvalidLength': return 'Invalid ID length';
    case 'InvalidFormat': return 'Invalid format';
  }
}
