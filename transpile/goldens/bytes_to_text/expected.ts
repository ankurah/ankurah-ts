// MIRRORS: ankurah/bytes_to_text/src/input.rs
import { decodeUtf8Lossy, serde_json } from '@ankurah/base';

export function readJson(bytes: Uint8Array): unknown | null {
  return serde_json.fromSlice(bytes).ok();
}

export function readLossy(bytes: Uint8Array): string {
  return decodeUtf8Lossy(bytes);
}

