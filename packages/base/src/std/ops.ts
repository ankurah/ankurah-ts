// TS-ONLY: the operators Rust spells with a symbol that JavaScript spells
// differently, or does not have at all.

/**
 * Rust's `a & b` on `bool`.
 *
 * `&&` short-circuits and `&` does not: `false & touch()` calls `touch`, and
 * the emitted `false && touch()` did not. That difference is invisible until an
 * operand has an effect — a call that mutates, logs, or advances an iterator —
 * and then the ported program takes a different path from the Rust one with
 * nothing to show for it. JavaScript evaluates both arguments of a call, left
 * to right, exactly once, so a call is the eager form.
 */
export function boolAnd(left: boolean, right: boolean): boolean {
  return left && right;
}

/** Rust's `a | b` on `bool`. Eager for the same reason as `boolAnd`. */
export function boolOr(left: boolean, right: boolean): boolean {
  return left || right;
}

// ── Checked arithmetic (R7) ────────────────────────────────────────────────
//
// The port mirrors the `debug_assertions = true` build, and that build PANICS
// on arithmetic overflow: `u8::MAX + 1` is `attempt to add with overflow`, not
// `0`. JavaScript wraps nothing and saturates nothing — it goes on counting in
// doubles, silently losing precision above 2^53 — so an emitted `a + b` was a
// third answer, neither Rust's release wrap nor Rust's debug panic.
//
// So `+`, `-` and `*` on a fixed-width integer go through these. Division and
// remainder by zero panic here as they do in Rust. `wrapping_*`, `checked_*`,
// `saturating_*` and `overflowing_*` are the four families Rust offers for
// saying what should happen instead, and each maps to a helper below with
// Rust's own semantics.
//
// Floats are untouched: Rust's `f64` arithmetic is IEEE and so is JavaScript's.
//
// The cost is one call per operation. The emitter skips it where both operands
// are provably in range — a literal, a `length` — which is what keeps the
// common case readable. Documented in spec 7a.

/** The inclusive range of a Rust integer type, by the name the emitter uses. */
const RANGES: Record<string, readonly [bigint, bigint]> = {
  u8: [0n, 255n],
  u16: [0n, 65535n],
  u32: [0n, 4294967295n],
  u64: [0n, 18446744073709551615n],
  u128: [0n, (1n << 128n) - 1n],
  // R13: `usize` and `isize` are 32-bit, because the port's target is wasm32.
  // Ranged as 64-bit, `usize` arithmetic that Rust panics on came back a
  // rounded double instead. The 8 bytes they occupy on the bincode wire is a
  // separate fact and belongs to the codec.
  usize: [0n, 4294967295n],
  i8: [-128n, 127n],
  i16: [-32768n, 32767n],
  i32: [-2147483648n, 2147483647n],
  i64: [-9223372036854775808n, 9223372036854775807n],
  i128: [-(1n << 127n), (1n << 127n) - 1n],
  isize: [-2147483648n, 2147483647n],
};

/** What Rust's panic says, so a port failure reads like the Rust one. */
function overflow(op: string, width: string): never {
  throw new RangeError(`attempt to ${op} with overflow (${width})`);
}

function range(width: string): readonly [bigint, bigint] {
  const found = RANGES[width];
  if (found === undefined) {
    throw new RangeError(`\`${width}\` is not an integer type this runtime knows`);
  }
  return found;
}

/** Is `value` inside `width`'s range? */
export function inRange(value: bigint, width: string): boolean {
  const [low, high] = range(width);
  return value >= low && value <= high;
}

/** `a + b`, panicking on overflow as the debug build does. */
export function checkedAdd(a: number, b: number, width: string): number;
export function checkedAdd(a: bigint, b: bigint, width: string): bigint;
export function checkedAdd(a: number | bigint, b: number | bigint, width: string): number | bigint {
  return apply('add', a, b, width, (x, y) => x + y);
}

/** `a - b`, panicking on overflow. */
export function checkedSub(a: number, b: number, width: string): number;
export function checkedSub(a: bigint, b: bigint, width: string): bigint;
export function checkedSub(a: number | bigint, b: number | bigint, width: string): number | bigint {
  return apply('subtract', a, b, width, (x, y) => x - y);
}

/** `a * b`, panicking on overflow. */
export function checkedMul(a: number, b: number, width: string): number;
export function checkedMul(a: bigint, b: bigint, width: string): bigint;
export function checkedMul(a: number | bigint, b: number | bigint, width: string): number | bigint {
  return apply('multiply', a, b, width, (x, y) => x * y);
}

/** `a / b` on integers: truncating, and a panic on a zero divisor. */
export function checkedDiv(a: number, b: number, width: string): number;
export function checkedDiv(a: bigint, b: bigint, width: string): bigint;
export function checkedDiv(a: number | bigint, b: number | bigint, width: string): number | bigint {
  if (BigInt(b as never) === 0n) throw new RangeError('attempt to divide by zero');
  // `i32::MIN / -1` overflows, which is the one division that can.
  return apply('divide', a, b, width, (x, y) => {
    const q = x / y;
    return q;
  });
}

/** `a % b` on integers: Rust's remainder takes the dividend's sign. */
export function checkedRem(a: number, b: number, width: string): number;
export function checkedRem(a: bigint, b: bigint, width: string): bigint;
export function checkedRem(a: number | bigint, b: number | bigint, width: string): number | bigint {
  if (BigInt(b as never) === 0n) {
    throw new RangeError('attempt to calculate the remainder with a divisor of zero');
  }
  // `i32::MIN % -1` is `attempt to calculate the remainder with overflow` in
  // Rust, for the same reason `i32::MIN / -1` is: the operation is defined in
  // terms of a quotient the type cannot hold. The mathematical answer is 0, so
  // the range check never sees it — `checkedDiv` catches its own case because
  // the quotient IS the answer, and this one has to say so itself.
  const [low] = range(width);
  if (BigInt(b as never) === -1n && BigInt(a as never) === low) {
    overflow('calculate the remainder', width);
  }
  return apply('calculate the remainder', a, b, width, (x, y) => x % y);
}

function apply(
  op: string,
  a: number | bigint,
  b: number | bigint,
  width: string,
  combine: (x: bigint, y: bigint) => bigint,
): number | bigint {
  const exact = combine(BigInt(a as never), BigInt(b as never));
  if (!inRange(exact, width)) overflow(op, width);
  return typeof a === 'bigint' ? exact : exactly(exact, op, width);
}

/**
 * The exact integer as a `number`, or a panic where a `number` cannot hold it.
 *
 * A width the port spells `number` can still carry a value past
 * `Number.MAX_SAFE_INTEGER` — `u64` and `i64` are spelled `bigint` for exactly
 * that reason, but a caller can hand a `number` to any width. `Number(exact)`
 * there ROUNDS, and a rounded answer is not an answer: it is a wrong number the
 * program then computes with. Rust has no such case, so a panic is what says so.
 */
function exactly(exact: bigint, op: string, width: string): number {
  const asNumber = Number(exact);
  if (!Number.isSafeInteger(asNumber)) {
    throw new RangeError(
      `attempt to ${op} in ${width}: the answer ${exact} is past what a number holds exactly`,
    );
  }
  return asNumber;
}

/** `a.wrapping_add(b)` — the release build's answer, always. */
export function wrappingAdd(a: number, b: number, width: string): number;
export function wrappingAdd(a: bigint, b: bigint, width: string): bigint;
export function wrappingAdd(a: number | bigint, b: number | bigint, width: string): number | bigint {
  return wrap(BigInt(a as never) + BigInt(b as never), width, typeof a === 'bigint');
}

export function wrappingSub(a: number, b: number, width: string): number;
export function wrappingSub(a: bigint, b: bigint, width: string): bigint;
export function wrappingSub(a: number | bigint, b: number | bigint, width: string): number | bigint {
  return wrap(BigInt(a as never) - BigInt(b as never), width, typeof a === 'bigint');
}

export function wrappingMul(a: number, b: number, width: string): number;
export function wrappingMul(a: bigint, b: bigint, width: string): bigint;
export function wrappingMul(a: number | bigint, b: number | bigint, width: string): number | bigint {
  return wrap(BigInt(a as never) * BigInt(b as never), width, typeof a === 'bigint');
}

/** `a.checked_add(b)` — `None` where it would overflow. */
export function checkedAddOption(a: number, b: number, width: string): number | null;
export function checkedAddOption(a: bigint, b: bigint, width: string): bigint | null;
export function checkedAddOption(
  a: number | bigint,
  b: number | bigint,
  width: string,
): number | bigint | null {
  return option(BigInt(a as never) + BigInt(b as never), width, typeof a === 'bigint');
}

export function checkedSubOption(a: number, b: number, width: string): number | null;
export function checkedSubOption(a: bigint, b: bigint, width: string): bigint | null;
export function checkedSubOption(
  a: number | bigint,
  b: number | bigint,
  width: string,
): number | bigint | null {
  return option(BigInt(a as never) - BigInt(b as never), width, typeof a === 'bigint');
}

export function checkedMulOption(a: number, b: number, width: string): number | null;
export function checkedMulOption(a: bigint, b: bigint, width: string): bigint | null;
export function checkedMulOption(
  a: number | bigint,
  b: number | bigint,
  width: string,
): number | bigint | null {
  return option(BigInt(a as never) * BigInt(b as never), width, typeof a === 'bigint');
}

/**
 * `a.checked_div(b)` — `None` where Rust's `/` would panic.
 *
 * Two cases answer `None`, and they are the two `checkedDiv` panics on: a zero
 * divisor, and `MIN / -1`, whose quotient the type cannot hold. Everything else
 * is the truncating quotient. Declared in the std surface and never lowered,
 * `v.checked_div(d)` came out as `v.checkedDiv(d)` — a method no number has.
 */
export function checkedDivOption(a: number, b: number, width: string): number | null;
export function checkedDivOption(a: bigint, b: bigint, width: string): bigint | null;
export function checkedDivOption(
  a: number | bigint,
  b: number | bigint,
  width: string,
): number | bigint | null {
  const [x, y] = [BigInt(a as never), BigInt(b as never)];
  if (y === 0n) return null;
  return option(x / y, width, typeof a === 'bigint');
}

/**
 * `a.checked_rem(b)` — `None` where Rust's `%` would panic.
 *
 * The same two cases. `MIN % -1` is mathematically 0, which every range holds,
 * so this one says the case itself rather than leaving it to the range check —
 * exactly as `checkedRem` does for the panicking form.
 */
export function checkedRemOption(a: number, b: number, width: string): number | null;
export function checkedRemOption(a: bigint, b: bigint, width: string): bigint | null;
export function checkedRemOption(
  a: number | bigint,
  b: number | bigint,
  width: string,
): number | bigint | null {
  const [x, y] = [BigInt(a as never), BigInt(b as never)];
  if (y === 0n) return null;
  const [low] = range(width);
  if (y === -1n && x === low) return null;
  return option(x % y, width, typeof a === 'bigint');
}

/** `a.saturating_add(b)` — the nearer bound where it would overflow. */
export function saturatingAdd(a: number, b: number, width: string): number;
export function saturatingAdd(a: bigint, b: bigint, width: string): bigint;
export function saturatingAdd(
  a: number | bigint,
  b: number | bigint,
  width: string,
): number | bigint {
  return saturate(BigInt(a as never) + BigInt(b as never), width, typeof a === 'bigint');
}

export function saturatingSub(a: number, b: number, width: string): number;
export function saturatingSub(a: bigint, b: bigint, width: string): bigint;
export function saturatingSub(
  a: number | bigint,
  b: number | bigint,
  width: string,
): number | bigint {
  return saturate(BigInt(a as never) - BigInt(b as never), width, typeof a === 'bigint');
}

export function saturatingMul(a: number, b: number, width: string): number;
export function saturatingMul(a: bigint, b: bigint, width: string): bigint;
export function saturatingMul(
  a: number | bigint,
  b: number | bigint,
  width: string,
): number | bigint {
  return saturate(BigInt(a as never) * BigInt(b as never), width, typeof a === 'bigint');
}

/** `a.overflowing_add(b)` — the wrapped answer, and whether it wrapped. */
export function overflowingAdd(a: number, b: number, width: string): [number, boolean];
export function overflowingAdd(a: bigint, b: bigint, width: string): [bigint, boolean];
export function overflowingAdd(
  a: number | bigint,
  b: number | bigint,
  width: string,
): [number | bigint, boolean] {
  const exact = BigInt(a as never) + BigInt(b as never);
  return [wrap(exact, width, typeof a === 'bigint'), !inRange(exact, width)];
}

export function overflowingSub(a: number, b: number, width: string): [number, boolean];
export function overflowingSub(a: bigint, b: bigint, width: string): [bigint, boolean];
export function overflowingSub(
  a: number | bigint,
  b: number | bigint,
  width: string,
): [number | bigint, boolean] {
  const exact = BigInt(a as never) - BigInt(b as never);
  return [wrap(exact, width, typeof a === 'bigint'), !inRange(exact, width)];
}

export function overflowingMul(a: number, b: number, width: string): [number, boolean];
export function overflowingMul(a: bigint, b: bigint, width: string): [bigint, boolean];
export function overflowingMul(
  a: number | bigint,
  b: number | bigint,
  width: string,
): [number | bigint, boolean] {
  const exact = BigInt(a as never) * BigInt(b as never);
  return [wrap(exact, width, typeof a === 'bigint'), !inRange(exact, width)];
}

/** The value `exact` becomes when it is truncated to `width`'s bits. */
function wrap(exact: bigint, width: string, asBigint: boolean): number | bigint {
  const [low] = range(width);
  const bits = BigInt(bitsOf(width));
  const wrapped = low < 0n ? BigInt.asIntN(Number(bits), exact) : BigInt.asUintN(Number(bits), exact);
  return asBigint ? wrapped : Number(wrapped);
}

function saturate(exact: bigint, width: string, asBigint: boolean): number | bigint {
  const [low, high] = range(width);
  const clamped = exact < low ? low : exact > high ? high : exact;
  return asBigint ? clamped : Number(clamped);
}

function option(exact: bigint, width: string, asBigint: boolean): number | bigint | null {
  if (!inRange(exact, width)) return null;
  return asBigint ? exact : Number(exact);
}

function bitsOf(width: string): number {
  const digits = width.replace(/^[ui]/, '');
  // R13: `usize` and `isize` are 32-bit, because the port's target is wasm32.
  // This and RANGES are one statement about the same two types; disagreeing,
  // they wrapped a `usize` at 64 bits and ranged it at 32.
  if (digits === 'size') return 32;
  const bits = Number(digits);
  if (!Number.isInteger(bits)) {
    throw new RangeError(`\`${width}\` is not an integer type this runtime knows`);
  }
  return bits;
}

// ── The float methods JavaScript spells the same and answers differently ──
//
// `Math.round`, `Math.sign`, `Math.min` and `Math.max` each read like the Rust
// method of the same name and each disagrees with it about a value the corpus
// can hold. Written as the `Math.*` call they were, the port answered something
// else and nothing said so, which is why they are helpers rather than a
// spelling: a helper is one place to state the rule and one place to test it.

/**
 * Rust's `f64::round`: half away from ZERO.
 *
 * `Math.round` rounds half UP, so the two agree on every positive value and
 * disagree on every negative half: `(-2.5).round()` is `-3` in Rust and `-2` in
 * JavaScript. A sort key, a midpoint, a mean — anything that lands on a half
 * and can be negative — comes out one apart.
 */
export function floatRound(value: number): number {
  return value < 0 ? -Math.round(-value) : Math.round(value);
}

/**
 * Rust's `f64::signum`: `1.0` for anything positive, `-1.0` for anything
 * negative, and `NaN` for `NaN`.
 *
 * `Math.sign` answers `+0` for `+0.0` and `-0` for `-0.0`, where Rust answers
 * `1.0` and `-1.0` — Rust's signum has no zero, because a float's zero carries
 * a sign and the signum reports it. The two agree about `NaN`.
 */
export function floatSignum(value: number): number {
  if (Number.isNaN(value)) return NaN;
  // `Object.is` is what tells `-0` from `0`; `<` and `>` cannot.
  return value < 0 || Object.is(value, -0) ? -1 : 1;
}

/**
 * Rust's `f64::min`: the OTHER operand where one is `NaN`.
 *
 * `Math.min` answers `NaN` if either operand is, and Rust ignores a `NaN`
 * operand entirely — `f64::NAN.min(2.0)` is `2.0`. A running minimum over data
 * with one missing value is `NaN` for the rest of the fold under `Math.min`,
 * and the value Rust would have found under this.
 *
 * Rust also settles the two zeros: `min` may answer either `-0.0` or `0.0` for
 * `(-0.0).min(0.0)`, so `Math.min`'s `-0` is one of the answers it allows.
 */
export function floatMin(left: number, right: number): number {
  if (Number.isNaN(left)) return right;
  if (Number.isNaN(right)) return left;
  return Math.min(left, right);
}

/** Rust's `f64::max`: the OTHER operand where one is `NaN`. */
export function floatMax(left: number, right: number): number {
  if (Number.isNaN(left)) return right;
  if (Number.isNaN(right)) return left;
  return Math.max(left, right);
}
