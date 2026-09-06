//! The two byte-to-text answers Rust has, side by side.
//!
//! `serde_json::from_slice` refuses a byte run that is not UTF-8, and so does
//! every other reader that builds a `String` out of bytes: a Rust `String` is
//! UTF-8 by construction. `String::from_utf8_lossy` is the OTHER answer, and it
//! is a choice the source makes — `core/src/value/mod.rs:266` writes an
//! arbitrary byte value out as a query literal that way, where refusing would
//! take the whole query down.
//!
//! The port has to keep them apart. The host's default decoder substitutes
//! U+FFFD, which is `from_utf8_lossy`'s behaviour and NOT `from_slice`'s: read
//! through it, `[0x22, 0xff, 0x22]` becomes the JSON string `"\u{FFFD}"` and
//! flows on as though Rust had read it.

/// Rust answers `None` here for a byte run that is not UTF-8, and for one that
/// is UTF-8 but not JSON.
pub fn read_json(bytes: &[u8]) -> Option<serde_json::Value> {
    serde_json::from_slice(bytes).ok()
}

/// And this one answers a string with U+FFFD where the bytes were not UTF-8.
pub fn read_lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}
