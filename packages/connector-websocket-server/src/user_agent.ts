// MIRRORS: ankurah/connectors/websocket-server/src/user_agent.rs

// Divergence: Rust uses axum::FromRequestParts + TypedHeader<headers::UserAgent>.
// TS extracts user-agent from IncomingMessage headers directly [E8].

/// Extract optional user-agent string from HTTP headers.
/// Mirrors Rust `OptionalUserAgent(pub Option<String>)`.
export function extractUserAgent(headers: Record<string, string | string[] | undefined>): string | null {
  const ua = headers['user-agent'];
  if (ua === undefined) return null;
  if (Array.isArray(ua)) return ua[0] ?? null;
  return ua;
}
