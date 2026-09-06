// TS-ONLY: `{:?}` for a value whose TYPE the emitter could not see.
//
// For: a hand-written generic type — `Attested<T>` is the port's only one — has
// to print its payload the way `#[derive(Debug)]` prints it, and `T` is
// whatever the instantiation put there. The emitter decides every other Debug
// rendering from the resolved `Ty`, which is what makes a Rust `String` print
// quoted and a Rust enum print its variant name even though both are a
// JavaScript string. Here there is no `Ty` to read, so the decision is the
// value's own surface at run time.
//
// `Attested<String>` printed `payload: secret` where Rust prints
// `payload: "secret"`, because the fallback was `String(payload)` (F7).
//
// **The one shape this cannot get right**, and the reason the emitter reports
// it separately: an erased JavaScript string is a Rust `String` and a Rust
// `char` alike, and their Debug syntax differs — `"a"` against `'a'`. A
// provided generic type instantiated with `char` is therefore reported at the
// type position rather than rendered as a guess.

/**
 * Rust's `{:?}` for a value the emitter could not type.
 *
 * A string is a Rust `String`; a number, a bigint and a boolean print as
 * themselves; `null` is `None`, because that is how the port spells it; an
 * array prints element-wise in brackets and a typed array as the list of bytes
 * it is; an object declaring `debug()` prints through it. Anything else is
 * REFUSED by name rather than printed through `toString`, which for a class is
 * `[object Object]` — the very answer this exists to replace.
 */
export function debugValue(v: unknown): string {
  switch (typeof v) {
    case 'string':
      return debugString(v);
    case 'number':
      return debugNumber(v);
    case 'bigint':
      return String(v);
    case 'boolean':
      return String(v);
    case 'undefined':
      // The port spells Rust's `None` as `null`; `undefined` reaching a Debug
      // is a hole in the port rather than a value Rust could have had.
      return refuse(v);
    default:
      break;
  }
  if (v === null) return 'None';
  if (ArrayBuffer.isView(v)) {
    // `ArrayBuffer.isView` narrows to `ArrayBufferView`, which carries no
    // element type. Every typed array the port builds holds numbers, and a
    // `DataView` — the one view that does not — never reaches a Debug.
    // Divergence: the narrowing has no element type to read.
    return `[${Array.from(v as unknown as ArrayLike<number>).join(', ')}]`;
  }
  if (Array.isArray(v)) return `[${v.map(debugValue).join(', ')}]`;
  const own = (v as { debug?: unknown }).debug;
  if (typeof own === 'function') return String((own as () => unknown).call(v));
  return refuse(v);
}

function refuse(v: unknown): never {
  const name =
    v === null || v === undefined
      ? String(v)
      : ((v as object).constructor?.name ?? typeof v);
  throw new TypeError(
    `debugValue: a ${name} stands where a hand-written generic type prints its payload,\n` +
      `and it declares no debug(). Rust's #[derive(Debug)] on a generic carries a Debug\n` +
      `bound on that parameter, so this is a value the port put there and Rust would not\n` +
      `have. Printing it through toString would answer "[object Object]".`,
  );
}

/**
 * A JavaScript string as Rust's `Debug for str` writes one: double quotes, with
 * `\\`, `"`, and the control characters escaped the way `char::escape_debug`
 * escapes them.
 *
 * `JSON.stringify` is close and not the same: it writes `\\u0000` where Rust
 * writes `\\0`, and `\\b` and `\\f` where Rust writes `\\u{8}` and `\\u{c}`.
 */
export function debugString(s: string): string {
  let out = '"';
  for (const ch of s) out += escapeInto(ch, '"');
  return `${out}"`;
}

/**
 * A one-character string as Rust's `Debug for char` writes one: single quotes,
 * with `'` escaped and `"` left alone — the mirror of `debugString`.
 *
 * The port writes a `char` as a one-character string, so the quotes ARE the
 * rendering, and a `'`, a `\\` or a newline inside them was printed raw:
 * `'''`, `'\\'` and a literal line break where Rust writes `'\\''`, `'\\\\'`
 * and `'\\n'`.
 */
export function debugChar(c: string): string {
  let out = "'";
  for (const ch of c) out += escapeInto(ch, "'");
  return `${out}'`;
}

/** One character, escaped for the quote it stands inside. */
function escapeInto(ch: string, quote: '"' | "'"): string {
  if (ch === '\\') return '\\\\';
  if (ch === quote) return `\\${quote}`;
  if (ch === '\n') return '\\n';
  if (ch === '\r') return '\\r';
  if (ch === '\t') return '\\t';
  if (ch === '\0') return '\\0';
  const code = ch.codePointAt(0) as number;
  // Rust's `escape_debug` leaves printable characters alone and writes every
  // other one as `\u{..}` in lower-case hex with no padding.
  if (code < 0x20 || code === 0x7f) return `\\u{${code.toString(16)}}`;
  return ch;
}

/**
 * A number as Rust's Debug prints one.
 *
 * The emitter writes this rendering out for a resolved `f32`/`f64`; here the
 * type is erased, so an integral value prints as an integer — which is what
 * every integer width wants — and a fractional one keeps its digits. A value
 * that is a Rust float and happens to be whole therefore prints without the
 * `.0` Rust would write, which is the same erasure the `char` gap is, and is
 * recorded beside it.
 */
function debugNumber(n: number): string {
  if (Number.isNaN(n)) return 'NaN';
  if (n === Infinity) return 'inf';
  if (n === -Infinity) return '-inf';
  return Object.is(n, -0) ? '-0' : String(n);
}
