//! Rust settles from the position what the expression does not carry: the width
//! of a literal written without a suffix, the element type of a sequence
//! literal, and which type reads itself out of a `parse` or a `collect`.

pub struct Header {
    pub version: u8,
    pub length: u16,
}

impl Header {
    /// Both literals take the width their field declares, not the `i32` a bare
    /// literal defaults to.
    pub fn first() -> Header {
        Header { version: 1, length: 512 }
    }
}

/// `Vec<u8>` is a `Uint8Array` in the port, so the literal written into one is
/// emitted as bytes rather than as a JavaScript array.
pub fn preamble() -> Vec<u8> {
    vec![1, 2, 3, 4]
}

/// The same for an array literal at a binding site with an annotation.
pub fn tag() -> [u8; 4] {
    let bytes: [u8; 4] = [7, 8, 9, 10];
    bytes
}

/// The annotation says what the sum is, so the literal added to it is that
/// width too.
pub fn next_length(header: &Header) -> u16 {
    let grown: u16 = header.length + 1;
    grown
}

/// A hole the position closes: `Vec<_>` says the container and the iterator
/// says the element.
pub fn lengths(headers: &[Header]) -> Vec<u16> {
    let out: Vec<_> = headers.iter().map(|header| header.length).collect();
    out
}
