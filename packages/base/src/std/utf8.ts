// TS-ONLY: the two facts about text that Rust's `String` guarantees and a
// JavaScript string does not.
//
// Rust's `String` and `str` are UTF-8 and nothing else: every byte sequence in
// one is valid UTF-8, and no code point in one is a surrogate, because a
// surrogate cannot be encoded in UTF-8 at all. A JavaScript string is a
// sequence of UTF-16 code units and holds a lone surrogate happily; the host's
// `TextDecoder` replaces an invalid byte sequence with U+FFFD rather than
// refusing it, and `JSON.parse` and `JSON.stringify` pass a lone surrogate
// straight through.
//
// So a port that reads bytes into a string, or writes a string back out, has to
// make both checks itself. They live here rather than in `serde_json.ts`
// because they are facts about text: the bincode codec reads the same bytes.

/**
 * The bytes as UTF-8 text, or `null` where they are not UTF-8.
 *
 * `serde_json::from_slice` answers `Err` for an invalid leading byte, a missing
 * continuation byte, a truncated sequence and an overlong encoding. The host's
 * default decoder answers a string with U+FFFD in it for every one of them, so
 * a document Rust refuses used to parse here with a replacement character in it.
 */
export function decodeUtf8(bytes: Uint8Array): string | null {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return null;
  }
}

/**
 * The index of a surrogate code unit that is not half of a pair, or `null`.
 *
 * A lone surrogate is half a code point. Rust cannot hold one in a `String` and
 * cannot encode one as UTF-8, so a value carrying one could not have come from
 * Rust and cannot be written back out to it.
 */
export function unpairedSurrogateAt(text: string): number | null {
  for (let at = 0; at < text.length; at++) {
    const code = text.charCodeAt(at);
    if (code < 0xd800 || code > 0xdfff) continue;
    const low = text.charCodeAt(at + 1);
    if (code <= 0xdbff && low >= 0xdc00 && low <= 0xdfff) {
      at += 1;
      continue;
    }
    return at;
  }
  return null;
}

/**
 * Is there a `\uD800`-`\uDFFF` ESCAPE in this quoted JSON string that is not
 * half of a pair?
 *
 * `JSON.parse` accepts a lone `\uD800` and hands back a string no encoder can
 * write out again; serde_json answers `Err(unexpected end of hex escape)`. The
 * escapes are checked here and decoded by the host, so there is still only one
 * unescaper.
 */
export function unpairedEscapedSurrogate(quoted: string): boolean {
  for (let at = 0; at < quoted.length; at++) {
    if (quoted[at] !== '\\') continue;
    if (quoted[at + 1] !== 'u') {
      // Any other escape is two characters; skipping the second keeps a `\\\\`
      // from being read as the start of an escape.
      at += 1;
      continue;
    }
    const code = Number.parseInt(quoted.slice(at + 2, at + 6), 16);
    at += 5;
    if (!Number.isNaN(code) && code >= 0xd800 && code <= 0xdbff) {
      // A high surrogate: the next escape has to be its low half.
      const low = Number.parseInt(quoted.slice(at + 3, at + 7), 16);
      const paired =
        quoted[at + 1] === '\\' &&
        quoted[at + 2] === 'u' &&
        !Number.isNaN(low) &&
        low >= 0xdc00 &&
        low <= 0xdfff;
      if (!paired) return true;
      at += 6;
      continue;
    }
    // A low surrogate with no high half in front of it.
    if (!Number.isNaN(code) && code >= 0xdc00 && code <= 0xdfff) return true;
  }
  return false;
}
