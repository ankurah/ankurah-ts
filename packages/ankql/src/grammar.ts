// MIRRORS: ankurah/ankql/src/grammar.rs
// E6: ankql.pest is the reference; this file contains token definitions for the hand-written parser.
// Since there is no Pest equivalent in TypeScript, we define token types, keywords, and
// character classification helpers used by the recursive descent parser.

/** Keywords recognized by the parser (case-insensitive) */
export const KEYWORDS = [
  'and', 'or', 'not', 'in', 'between', 'is', 'null', 'true', 'false',
  'order', 'by', 'asc', 'desc', 'limit',
] as const;

/**
 * Reserved words that cannot be used as bare identifiers when followed by
 * whitespace, comma, '(' or EOF. Mirrors the pest `Reserved` rule.
 */
export const RESERVED_WORDS = [
  'left', 'having', 'not', 'inner', 'group', 'on', 'join',
  'from', 'exists', 'except', 'union', 'where', 'distinct', 'between', 'option',
  'values', 'limit', 'order',
] as const;

/** Token types emitted by the lexer */
export type TokenType =
  | 'Identifier'
  | 'Integer'
  | 'Unsigned'
  | 'Decimal'
  | 'Double'
  | 'String'
  | 'True'
  | 'False'
  | 'Null'
  | 'Eq'
  | 'NotEq'
  | 'Gt'
  | 'GtEq'
  | 'Lt'
  | 'LtEq'
  | 'Add'
  | 'Subtract'
  | 'Multiply'
  | 'Divide'
  | 'And'
  | 'Or'
  | 'Not'
  | 'In'
  | 'Between'
  | 'Is'
  | 'LParen'
  | 'RParen'
  | 'Comma'
  | 'Dot'
  | 'Question'
  | 'OrderBy'
  | 'Asc'
  | 'Desc'
  | 'Limit'
  | 'EOF';

export interface Token {
  type: TokenType;
  value: string;
  pos: number;
}

// ── Character classification helpers ─────────────────────────────────

/** Whitespace characters (matches pest WHITESPACE rule) */
export function isWhitespace(ch: string): boolean {
  return ch === ' ' || ch === '\t' || ch === '\n' || ch === '\r';
}

/** ASCII digit 0-9 */
export function isDigit(ch: string): boolean {
  return ch >= '0' && ch <= '9';
}

/** Matches pest IdentifierNonDigit: a-z A-Z cyrillic А-Я а-я - _ */
export function isIdentStart(ch: string): boolean {
  if ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z')) return true;
  if (ch === '_' || ch === '-') return true;
  // Cyrillic ranges
  const code = ch.charCodeAt(0);
  if ((code >= 0x0410 && code <= 0x042F) || (code >= 0x0430 && code <= 0x044F)) return true;
  return false;
}

/** Matches pest IDENT_CONT: ASCII_ALPHANUMERIC | "_" */
export function isIdentCont(ch: string): boolean {
  return isIdentStart(ch) || isDigit(ch);
}

/** Check if a string is a reserved word (case-insensitive) */
export function isReservedWord(word: string): boolean {
  return (RESERVED_WORDS as readonly string[]).includes(word.toLowerCase());
}
