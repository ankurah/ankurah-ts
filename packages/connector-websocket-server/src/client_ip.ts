// MIRRORS: ankurah/connectors/websocket-server/src/client_ip.rs

// Divergence: Rust uses axum::FromRequestParts with IpAddr.
// TS extracts client IP from Node.js IncomingMessage headers + socket [E8].

/// Determine the client IP from request headers or socket, matching Rust SmartClientIp.
///
/// Tries headers in this order (matching Rust):
/// 1. Forwarded (rightmost for-entry)
/// 2. CF-Connecting-IP
/// 3. X-Real-IP
/// 4. X-Forwarded-For (rightmost entry)
/// 5. Socket remote address
export function smartClientIp(
  headers: Record<string, string | string[] | undefined>,
  remoteAddress?: string,
): string | null {
  return (
    ipFromRightmostForwardedHeader(headers) ??
    ipFromCfConnectingIpHeader(headers) ??
    ipFromXRealIpHeader(headers) ??
    ipFromXForwardedForHeader(headers) ??
    remoteAddress ??
    null
  );
}

function lastHeaderValue(headers: Record<string, string | string[] | undefined>, name: string): string | null {
  const val = headers[name.toLowerCase()];
  if (val === undefined) return null;
  if (Array.isArray(val)) {
    return val.length > 0 ? val[val.length - 1] : null;
  }
  return val;
}

function ipFromRightmostForwardedHeader(headers: Record<string, string | string[] | undefined>): string | null {
  const headerValue = lastHeaderValue(headers, 'forwarded');
  if (headerValue === null) return null;

  // Parse RFC 7239 Forwarded header. Extract rightmost for= value.
  // Format: for=<ip>;by=...;host=..., for=<ip2>;...
  const parts = headerValue.split(',');
  for (let i = parts.length - 1; i >= 0; i--) {
    const part = parts[i].trim();
    const forMatch = part.match(/for\s*=\s*"?([^";,\s]+)"?/i);
    if (forMatch) {
      const forValue = forMatch[1];
      // Strip port from IPv4 (1.2.3.4:port) or bracketed IPv6 ([::1]:port)
      const ip = stripPort(forValue);
      if (ip) return ip;
    }
  }
  return null;
}

function ipFromXForwardedForHeader(headers: Record<string, string | string[] | undefined>): string | null {
  const headerValue = lastHeaderValue(headers, 'x-forwarded-for');
  if (headerValue === null) return null;

  // Take the rightmost entry (closest to the client in reverse proxy chain)
  const parts = headerValue.split(',');
  const last = parts[parts.length - 1]?.trim();
  return last || null;
}

function ipFromHeaderValue(headers: Record<string, string | string[] | undefined>, headerName: string): string | null {
  const val = lastHeaderValue(headers, headerName);
  return val?.trim() || null;
}

function ipFromXRealIpHeader(headers: Record<string, string | string[] | undefined>): string | null {
  return ipFromHeaderValue(headers, 'x-real-ip');
}

function ipFromCfConnectingIpHeader(headers: Record<string, string | string[] | undefined>): string | null {
  return ipFromHeaderValue(headers, 'cf-connecting-ip');
}

/// Strip port from IP address string.
/// "1.2.3.4:8080" → "1.2.3.4"
/// "[::1]:8080" → "::1"
/// "::1" → "::1"
function stripPort(addr: string): string | null {
  if (!addr) return null;
  // Bracketed IPv6: [::1]:port
  if (addr.startsWith('[')) {
    const end = addr.indexOf(']');
    if (end >= 0) return addr.substring(1, end);
    return null;
  }
  // If it has colons but more than one, it's bare IPv6
  const colonCount = (addr.match(/:/g) || []).length;
  if (colonCount > 1) return addr; // bare IPv6
  // IPv4:port
  if (colonCount === 1) {
    return addr.substring(0, addr.indexOf(':'));
  }
  return addr;
}
