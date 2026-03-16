// MIRRORS: ankurah/storage/indexeddb-wasm/src/error.rs

// Divergence: Rust uses wasm_bindgen JsValue→String conversion. [E16]
// In TS, errors are native JS Error objects, so extractMessage simply
// reads err.message or stringifies the value.

export function extractMessage(err: unknown): string {
  // If it's a standard Error, grab name + message
  if (err instanceof Error) {
    return `${err.name}: ${err.message}`;
  }

  // If it's already a string, return directly
  if (typeof err === 'string') {
    return err;
  }

  // Fallback: stringify the value
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}
