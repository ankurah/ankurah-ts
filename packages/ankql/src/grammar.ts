// MIRRORS: ankurah/ankql/src/grammar.rs
//
// grammar.rs is nothing but `#[derive(Parser)] #[grammar = "ankql.pest"]` — a
// proc-macro that turns ankql.pest into a parser at compile time. TypeScript has no
// pest, so this file is what that macro would have produced: the same grammar,
// matched by hand, handing parser.ts the same `Pair` tree pest hands parser.rs.
// Every rule below carries its ankql.pest line above it so the two read side by side,
// and the rules appear in the order the grammar declares them.
//
// The grammar's behaviour, not SQL's, is what the port has to reproduce, and three
// PEG properties are where the two part company:
//
//   - Ordered choice commits. Once an alternative matches, a failure further along
//     never retries the alternatives after it. `('a')` matches
//     ExpressionInParentheses, so it is a grouped expression forever and never
//     re-reads as a one-element Row.
//   - Repetition is greedy and gives nothing back. `Expr` is a flat
//     `atom (op atom)*`, which is why AND and OR have no relative precedence at all —
//     parser.rs folds that flat list left to right.
//   - Atomicity governs whitespace and tokens. A `@` rule skips no whitespace and
//     emits no inner pairs (its span is one token, quotes and all); a `$` rule skips
//     no whitespace but does emit inner pairs; every other rule skips WHITESPACE
//     between its elements.

/** The pest `Rule` enum, restricted to the rules that reach parser.rs as pairs. */
export type Rule =
  | 'Expr'
  | 'Between'
  | 'And'
  | 'Or'
  | 'Add'
  | 'Subtract'
  | 'Multiply'
  | 'Divide'
  | 'Eq'
  | 'Gt'
  | 'GtEq'
  | 'Lt'
  | 'LtEq'
  | 'NotEq'
  | 'In'
  | 'UnaryNot'
  | 'IsNullPostfix'
  | 'True'
  | 'False'
  | 'Null'
  | 'Decimal'
  | 'Double'
  | 'Integer'
  | 'Unsigned'
  | 'SingleQuotedString'
  | 'QuestionParameter'
  | 'PathExpr'
  | 'ExpressionInParentheses'
  | 'Row'
  | 'Identifier'
  | 'OrderByClause'
  | 'OrderByItem'
  | 'OrderDirection'
  | 'LimitClause'
  | 'EOI';

/** A pest `Pair`: one rule match, its span, and the pairs nested inside it. */
export interface Pair {
  readonly rule: Rule;
  /** `Pair::as_str()` — the exact input span this rule matched. */
  readonly text: string;
  /** `Pair::into_inner()` — the child pairs, in the order they matched. */
  readonly inner: Pair[];
}

/** What `AnkqlParser::parse(Rule::Selection, input)` returns, as a Result. */
export type GrammarResult =
  | { readonly ok: true; readonly pairs: Pair[] }
  | { readonly ok: false; readonly message: string };

// Reserved = { ^"left" | ^"having" | ^"not" | … | ^"values" | LimitClause | OrderByClause }
// A word here only blocks an identifier when a "(", whitespace, a comma or the end of
// input follows it, so `option = 1` is a syntax error while `optional = 1` is a name.
// The order is the grammar's; ordered choice makes it observable, because a word that
// matches and then fails the boundary test does not let a later alternative try.
const RESERVED_WORDS = [
  'left', 'having', 'not', 'inner', 'group',
  'on', 'join', 'from', 'exists', 'except',
  'union', 'where', 'distinct', 'between', 'option',
  'values',
] as const;

/** ASCII_DIGIT */
function isAsciiDigit(ch: string | undefined): boolean {
  return ch !== undefined && ch >= '0' && ch <= '9';
}

/** IdentifierNonDigit = _{ 'a'..'z' | 'A'..'Z' | 'А'..'Я' | 'а'..'я' | "-" | "_" }
 *  ASCII letters, hyphen, underscore and the two Cyrillic ranges the grammar spells
 *  out — and nothing else, which is why `имя` is a name and `名前` is a syntax error. */
function isIdentifierNonDigit(ch: string | undefined): boolean {
  if (ch === undefined) return false;
  if ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z')) return true;
  if (ch === '-' || ch === '_') return true;
  const code = ch.charCodeAt(0);
  return (code >= 0x0410 && code <= 0x042f) || (code >= 0x0430 && code <= 0x044f);
}

/** IDENT_CONT = _{ ASCII_ALPHANUMERIC | "_" } — note it excludes "-". */
function isIdentCont(ch: string | undefined): boolean {
  if (ch === undefined) return false;
  return (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || (ch >= '0' && ch <= '9') || ch === '_';
}

class Grammar {
  private readonly src: string;
  private pos = 0;
  /** The furthest offset any rule reached — where the input stopped making sense. */
  private furthest = 0;

  constructor(src: string) {
    this.src = src;
  }

  // ── Terminals and bookkeeping ──────────────────────────────────────

  private mark(): void {
    if (this.pos > this.furthest) this.furthest = this.pos;
  }

  private pair(rule: Rule, start: number): Pair {
    return { rule, text: this.src.slice(start, this.pos), inner: [] };
  }

  /** A literal string. */
  private lit(s: string): boolean {
    if (!this.src.startsWith(s, this.pos)) {
      this.mark();
      return false;
    }
    this.pos += s.length;
    this.mark();
    return true;
  }

  /** `^"…"` — an ASCII case-insensitive literal. */
  private ci(s: string): boolean {
    const end = this.pos + s.length;
    if (end > this.src.length || this.src.slice(this.pos, end).toLowerCase() !== s) {
      this.mark();
      return false;
    }
    this.pos = end;
    this.mark();
    return true;
  }

  /** WHITESPACE = _{ " " | "\t" | "\n" | "\r\n" } — a lone "\r" is not whitespace. */
  private ws(): void {
    for (;;) {
      const ch = this.src[this.pos];
      if (ch === ' ' || ch === '\t' || ch === '\n') {
        this.pos++;
        continue;
      }
      if (ch === '\r' && this.src[this.pos + 1] === '\n') {
        this.pos += 2;
        continue;
      }
      return;
    }
  }

  /** `WS+`, the explicit whitespace the ORDER BY and LIMIT clauses spell out. */
  private ws1(): boolean {
    const start = this.pos;
    this.ws();
    return this.pos > start;
  }

  /** `ASCII_DIGIT*` */
  private digits(): void {
    while (isAsciiDigit(this.src[this.pos])) this.pos++;
    this.mark();
  }

  /** `ASCII_DIGIT+` */
  private digits1(): boolean {
    const start = this.pos;
    this.digits();
    return this.pos > start;
  }

  // ── Selection ──────────────────────────────────────────────────────

  // Selection = _{ SOI ~ Expr ~ OrderByClause? ~ LimitClause? ~ EOI }
  selection(): Pair[] | null {
    const out: Pair[] = [];
    this.ws();
    if (!this.expr(out)) return null;
    this.ws();
    this.orderByClause(out);
    this.ws();
    this.limitClause(out);
    this.ws();
    // EOI: the clause order is fixed and nothing may trail, so `a = 1;` and
    // `… LIMIT 5 ORDER BY x` both die here rather than in the AST builder.
    if (this.pos < this.src.length) {
      this.mark();
      return null;
    }
    out.push({ rule: 'EOI', text: '', inner: [] });
    return out;
  }

  diagnostic(): string {
    const rest = this.src.slice(this.furthest);
    if (rest.length === 0) return `unexpected end of input at position ${this.furthest}`;
    const shown = rest.length > 40 ? `${rest.slice(0, 40)}...` : rest;
    return `unexpected input at position ${this.furthest}: ${JSON.stringify(shown)}`;
  }

  // ── Expr ───────────────────────────────────────────────────────────

  // Expr = { ExprAtomValue ~ (ExprInfixOp ~ ExprAtomValue)* }
  // Flat by construction: the operators and their operands land side by side in
  // `inner`, with no nesting to carry precedence.
  private expr(out: Pair[]): boolean {
    const start = this.pos;
    const inner: Pair[] = [];
    if (!this.exprAtomValue(inner)) {
      this.pos = start;
      return false;
    }
    for (;;) {
      const mark = this.pos;
      const keep = inner.length;
      this.ws();
      if (!this.exprInfixOp(inner)) {
        this.pos = mark;
        inner.length = keep;
        break;
      }
      this.ws();
      if (!this.exprAtomValue(inner)) {
        this.pos = mark;
        inner.length = keep;
        break;
      }
    }
    out.push({ rule: 'Expr', text: this.src.slice(start, this.pos), inner });
    return true;
  }

  // ExprInfixOp = _{ Between | ArithInfixOp | CmpInfixOp | And | Or }
  private exprInfixOp(out: Pair[]): boolean {
    return this.between(out) || this.arithInfixOp(out) || this.cmpInfixOp(out) || this.keyword('And', 'and', out) || this.keyword('Or', 'or', out);
  }

  // Between = { NotFlag? ~ ^"between" }
  // NotFlag's own pair is never read by parser.rs, so no pair is emitted for it.
  private between(out: Pair[]): boolean {
    const start = this.pos;
    if (this.notFlag()) this.ws();
    if (!this.ci('between')) {
      this.pos = start;
      return false;
    }
    out.push(this.pair('Between', start));
    return true;
  }

  // ArithInfixOp = _{ Add | Subtract | Multiply | Divide }
  private arithInfixOp(out: Pair[]): boolean {
    return this.punct('Add', '+', out) || this.punct('Subtract', '-', out) || this.punct('Multiply', '*', out) || this.punct('Divide', '/', out);
  }

  // CmpInfixOp = _{ NotEq | GtEq | Gt | LtEq | Lt | Eq | Lt | In }
  // The grammar lists Lt twice; the second is unreachable and is not repeated here.
  private cmpInfixOp(out: Pair[]): boolean {
    return (
      this.notEq(out) ||
      this.punct('GtEq', '>=', out) ||
      this.punct('Gt', '>', out) ||
      this.punct('LtEq', '<=', out) ||
      this.punct('Lt', '<', out) ||
      this.punct('Eq', '=', out) ||
      this.inOp(out)
    );
  }

  // NotEq = { "<>" | "!=" } — both spellings produce the one rule, so they build
  // the identical AST.
  private notEq(out: Pair[]): boolean {
    const start = this.pos;
    if (!this.lit('<>') && !this.lit('!=')) return false;
    out.push(this.pair('NotEq', start));
    return true;
  }

  // Eq = { "=" }, Gt = { ">" }, GtEq = { ">=" }, Lt = { "<" }, LtEq = { "<=" },
  // Add = { "+" }, Subtract = { "-" }, Multiply = { "*" }, Divide = { "/" }
  private punct(rule: Rule, text: string, out: Pair[]): boolean {
    const start = this.pos;
    if (!this.lit(text)) return false;
    out.push(this.pair(rule, start));
    return true;
  }

  // In = { NotFlag? ~ ^"in" }
  // The optional NOT is consumed inside the In rule and never reaches parser.rs,
  // which is why `status NOT IN (…)` builds a tree identical to `status IN (…)`.
  private inOp(out: Pair[]): boolean {
    const start = this.pos;
    if (this.notFlag()) this.ws();
    if (!this.ci('in')) {
      this.pos = start;
      return false;
    }
    out.push(this.pair('In', start));
    return true;
  }

  // NotFlag = { ^"not" } — no !IDENT_CONT guard, so it also matches the "not" that
  // begins "nothing".
  private notFlag(): boolean {
    return this.ci('not');
  }

  // And = @{ ^"and" ~ !IDENT_CONT }, Or = @{ ^"or" ~ !IDENT_CONT },
  // True = @{ ^"true" ~ !IDENT_CONT }, False / Null likewise. The guard is what
  // keeps `and_field` an identifier.
  private keyword(rule: Rule, word: string, out: Pair[]): boolean {
    const start = this.pos;
    if (!this.ci(word) || isIdentCont(this.src[this.pos])) {
      this.pos = start;
      return false;
    }
    out.push(this.pair(rule, start));
    return true;
  }

  // ExprAtomValue = _{ UnaryNot* ~ AtomicExpr ~ IsNullPostfix? }
  private exprAtomValue(out: Pair[]): boolean {
    const start = this.pos;
    const keep = out.length;
    for (;;) {
      const mark = this.pos;
      if (!this.unaryNot(out)) {
        this.pos = mark;
        break;
      }
      this.ws();
    }
    if (!this.atomicExpr(out)) {
      this.pos = start;
      out.length = keep;
      return false;
    }
    const mark = this.pos;
    this.ws();
    if (!this.isNullPostfix(out)) this.pos = mark;
    return true;
  }

  // UnaryNot = @{ NotFlag }
  private unaryNot(out: Pair[]): boolean {
    const start = this.pos;
    if (!this.notFlag()) return false;
    out.push(this.pair('UnaryNot', start));
    return true;
  }

  // IsNullPostfix = { ^"is" ~ NotFlag? ~ ^"null" }
  // Not atomic, so the whitespace between the words is optional; parser.rs reads the
  // matched text to decide whether a NOT was in it.
  private isNullPostfix(out: Pair[]): boolean {
    const start = this.pos;
    if (!this.ci('is')) return false;
    this.ws();
    if (this.notFlag()) this.ws();
    if (!this.ci('null')) {
      this.pos = start;
      return false;
    }
    out.push(this.pair('IsNullPostfix', start));
    return true;
  }

  // AtomicExpr = _{ Literal | QuestionParameter | PathExpr | ExpressionInParentheses | Row }
  // ExpressionInParentheses before Row is what makes `('a')` a grouped expression:
  // the comma is what turns a parenthesized group into a list.
  private atomicExpr(out: Pair[]): boolean {
    return this.literal(out) || this.questionParameter(out) || this.pathExpr(out) || this.expressionInParentheses(out) || this.row(out);
  }

  // Literal = _{ True | False | Null | Double | Decimal | Unsigned | Integer | SingleQuotedString }
  private literal(out: Pair[]): boolean {
    return (
      this.keyword('True', 'true', out) ||
      this.keyword('False', 'false', out) ||
      this.keyword('Null', 'null', out) ||
      this.double(out) ||
      this.decimal(out) ||
      this.unsigned(out) ||
      this.integer(out) ||
      this.singleQuotedString(out)
    );
  }

  // Integer = @{ ("+" | "-")? ~ ASCII_DIGIT+ }
  private integerSpan(): boolean {
    const start = this.pos;
    if (this.src[this.pos] === '+' || this.src[this.pos] === '-') this.pos++;
    if (!this.digits1()) {
      this.pos = start;
      return false;
    }
    return true;
  }

  private integer(out: Pair[]): boolean {
    const start = this.pos;
    if (!this.integerSpan()) return false;
    out.push(this.pair('Integer', start));
    return true;
  }

  // Decimal = @{ Integer ~ ("." ~ ASCII_DIGIT*) }
  private decimal(out: Pair[]): boolean {
    const start = this.pos;
    if (!this.integerSpan() || !this.lit('.')) {
      this.pos = start;
      return false;
    }
    this.digits();
    out.push(this.pair('Decimal', start));
    return true;
  }

  // Double = @{ Integer ~ ("." ~ ASCII_DIGIT*)? ~ (^"e" ~ Integer) }
  private double(out: Pair[]): boolean {
    const start = this.pos;
    if (!this.integerSpan()) return false;
    const afterInteger = this.pos;
    if (this.lit('.')) this.digits();
    else this.pos = afterInteger;
    if (!this.ci('e') || !this.integerSpan()) {
      this.pos = start;
      return false;
    }
    out.push(this.pair('Double', start));
    return true;
  }

  // Unsigned = @{ ASCII_DIGIT+ }
  private unsigned(out: Pair[]): boolean {
    const start = this.pos;
    if (!this.digits1()) return false;
    out.push(this.pair('Unsigned', start));
    return true;
  }

  // SingleQuotedString = @{ OnlyQuotesSequence | AnythingButQuotesSequence }
  //   OnlyQuotesSequence        = @{ ("'" ~ "'")+ }
  //   AnythingButQuotesSequence = @{ "'" ~ (!("'") ~ ANY)* ~ "'" }
  // There is no escape mechanism: outside a run of quote pairs, the first quote after
  // the opening one ends the literal. `'it''s'` therefore yields the string `it` and
  // leaves `'s'` behind for EOI to choke on.
  private singleQuotedString(out: Pair[]): boolean {
    const start = this.pos;
    if (this.src[this.pos] === "'" && this.src[this.pos + 1] === "'") {
      while (this.src[this.pos] === "'" && this.src[this.pos + 1] === "'") this.pos += 2;
      this.mark();
      out.push(this.pair('SingleQuotedString', start));
      return true;
    }
    if (this.src[this.pos] !== "'") {
      this.mark();
      return false;
    }
    this.pos++;
    while (this.pos < this.src.length && this.src[this.pos] !== "'") this.pos++;
    this.mark();
    if (this.src[this.pos] !== "'") {
      this.pos = start;
      return false;
    }
    this.pos++;
    out.push(this.pair('SingleQuotedString', start));
    return true;
  }

  // QuestionParameter = @{ "?" }
  private questionParameter(out: Pair[]): boolean {
    return this.punct('QuestionParameter', '?', out);
  }

  // PathExpr = { Identifier ~ ("." ~ Identifier)* }
  private pathExpr(out: Pair[]): boolean {
    const start = this.pos;
    const inner: Pair[] = [];
    if (!this.identifier(inner)) {
      this.pos = start;
      return false;
    }
    for (;;) {
      const mark = this.pos;
      const keep = inner.length;
      this.ws();
      if (!this.lit('.')) {
        this.pos = mark;
        break;
      }
      this.ws();
      if (!this.identifier(inner)) {
        this.pos = mark;
        inner.length = keep;
        break;
      }
    }
    out.push({ rule: 'PathExpr', text: this.src.slice(start, this.pos), inner });
    return true;
  }

  // ExpressionInParentheses = { "(" ~ Expr ~ ")" }
  private expressionInParentheses(out: Pair[]): boolean {
    const start = this.pos;
    const inner: Pair[] = [];
    if (!this.lit('(')) return false;
    this.ws();
    if (!this.expr(inner)) {
      this.pos = start;
      return false;
    }
    this.ws();
    if (!this.lit(')')) {
      this.pos = start;
      return false;
    }
    out.push({ rule: 'ExpressionInParentheses', text: this.src.slice(start, this.pos), inner });
    return true;
  }

  // Row = { "(" ~ Expr ~ ("," ~ Expr)* ~ ")" }
  private row(out: Pair[]): boolean {
    const start = this.pos;
    const inner: Pair[] = [];
    if (!this.lit('(')) return false;
    this.ws();
    if (!this.expr(inner)) {
      this.pos = start;
      return false;
    }
    for (;;) {
      const mark = this.pos;
      const keep = inner.length;
      this.ws();
      if (!this.lit(',')) {
        this.pos = mark;
        break;
      }
      this.ws();
      if (!this.expr(inner)) {
        this.pos = mark;
        inner.length = keep;
        break;
      }
    }
    this.ws();
    if (!this.lit(')')) {
      this.pos = start;
      return false;
    }
    out.push({ rule: 'Row', text: this.src.slice(start, this.pos), inner });
    return true;
  }

  // ── Identifier ─────────────────────────────────────────────────────

  // Identifier = @{ DoubleQuotedIdentifier | IdentifierInner }
  //   DoubleQuotedIdentifier = @{ "\"" ~ IdentifierInner ~ "\"" }
  // Atomic, so the pair's span is the whole token: a double-quoted identifier keeps
  // its quotes, and they travel into the path step and out again through SQL
  // generation as `""quoted""`.
  private identifier(out: Pair[]): boolean {
    const start = this.pos;
    if (this.lit('"')) {
      if (this.identifierInner() && this.lit('"')) {
        out.push(this.pair('Identifier', start));
        return true;
      }
      this.pos = start;
    }
    if (!this.identifierInner()) {
      this.pos = start;
      return false;
    }
    out.push(this.pair('Identifier', start));
    return true;
  }

  // IdentifierInner = @{ !(Reserved ~ ("(" | WHITESPACE | "," | EOF))
  //                      ~ (IdentifierNonDigit ~ (IdentifierNonDigit | ASCII_DIGIT)*) }
  private identifierInner(): boolean {
    if (this.reservedAtBoundary()) return false;
    const start = this.pos;
    if (!isIdentifierNonDigit(this.src[this.pos])) {
      this.mark();
      return false;
    }
    this.pos++;
    while (isIdentifierNonDigit(this.src[this.pos]) || isAsciiDigit(this.src[this.pos])) this.pos++;
    this.mark();
    return this.pos > start;
  }

  /** The `Reserved ~ ("(" | WHITESPACE | "," | EOF)` the lookahead forbids. Consumes nothing. */
  private reservedAtBoundary(): boolean {
    const start = this.pos;
    const matched = this.reserved() && this.reservedBoundary();
    this.pos = start;
    return matched;
  }

  private reservedBoundary(): boolean {
    const ch = this.src[this.pos];
    if (ch === '(' || ch === ',' || ch === ' ' || ch === '\t' || ch === '\n') return true;
    if (ch === '\r' && this.src[this.pos + 1] === '\n') return true;
    // EOF = { EOI | ";" }
    return ch === undefined || ch === ';';
  }

  // Reserved = { ^"left" | … | ^"values" | LimitClause | OrderByClause }
  // Whole clauses are reserved, not just the words: `limit = 1` keeps `limit` as a
  // column name because LimitClause needs a number after it, and `order = 1` keeps
  // `order` because OrderByClause needs a `by`.
  private reserved(): boolean {
    for (const word of RESERVED_WORDS) {
      if (this.ci(word)) return true;
    }
    const discarded: Pair[] = [];
    return this.limitClause(discarded) || this.orderByClause(discarded);
  }

  // ── ORDER BY and LIMIT ─────────────────────────────────────────────

  // OrderByClause = ${ ^"order" ~ WS+ ~ ^"by" ~ WS+ ~ (OrderByItem ~ WS* ~ ("," ~ WS* ~ OrderByItem)*)? }
  // Compound-atomic: every space is spelled out, and there is no WS* before the
  // commas after the first item.
  private orderByClause(out: Pair[]): boolean {
    const start = this.pos;
    if (!this.ci('order') || !this.ws1() || !this.ci('by') || !this.ws1()) {
      this.pos = start;
      return false;
    }
    const inner: Pair[] = [];
    const afterBy = this.pos;
    if (this.orderByItem(inner)) {
      this.ws();
      for (;;) {
        const mark = this.pos;
        const keep = inner.length;
        if (!this.lit(',')) {
          this.pos = mark;
          break;
        }
        this.ws();
        if (!this.orderByItem(inner)) {
          this.pos = mark;
          inner.length = keep;
          break;
        }
      }
    } else {
      this.pos = afterBy;
    }
    out.push({ rule: 'OrderByClause', text: this.src.slice(start, this.pos), inner });
    return true;
  }

  // OrderByItem = ${ Identifier ~ (WS+ ~ OrderDirection)? }
  // An Identifier, not a PathExpr — so `ORDER BY licensing.territory` stops at the
  // dot and the whole Selection then fails at EOI.
  private orderByItem(out: Pair[]): boolean {
    const start = this.pos;
    const inner: Pair[] = [];
    if (!this.identifier(inner)) {
      this.pos = start;
      return false;
    }
    const afterIdentifier = this.pos;
    if (!this.ws1() || !this.orderDirection(inner)) this.pos = afterIdentifier;
    out.push({ rule: 'OrderByItem', text: this.src.slice(start, this.pos), inner });
    return true;
  }

  // OrderDirection = { ^"asc" | ^"desc" }
  private orderDirection(out: Pair[]): boolean {
    const start = this.pos;
    if (!this.ci('asc') && !this.ci('desc')) return false;
    out.push(this.pair('OrderDirection', start));
    return true;
  }

  // LimitClause = ${ ^"limit" ~ WS+ ~ Unsigned }
  // Unsigned, so `LIMIT -1` is a syntax error rather than a bad number.
  private limitClause(out: Pair[]): boolean {
    const start = this.pos;
    const inner: Pair[] = [];
    if (!this.ci('limit') || !this.ws1() || !this.unsigned(inner)) {
      this.pos = start;
      return false;
    }
    out.push({ rule: 'LimitClause', text: this.src.slice(start, this.pos), inner });
    return true;
  }
}

/**
 * Rust: `grammar::AnkqlParser::parse(grammar::Rule::Selection, input)`.
 *
 * On failure this reports where the input stopped matching. Pest's own diagnostic —
 * the input echoed with a caret and a list of expected rules — is pest's rendering,
 * changes with the pest version, and is deliberately not pinned by the fixtures.
 */
export function parseSelectionRule(input: string): GrammarResult {
  const grammar = new Grammar(input);
  const pairs = grammar.selection();
  if (pairs === null) return { ok: false, message: grammar.diagnostic() };
  return { ok: true, pairs };
}
