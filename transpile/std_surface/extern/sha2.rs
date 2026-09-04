//! `sha2` 0.10.9
//!
//! Not on the deliverable's list. `proto/src/data.rs` derives an `EventId` by
//! hashing the event payload, so the digest is part of the wire identity and
//! the port has to compute the same bytes.

pub struct Sha256;

pub trait Digest {
    fn new() -> Self;
    fn update(&mut self, data: impl AsRef<[u8]>);
    fn finalize(self) -> Output;
    fn digest(data: impl AsRef<[u8]>) -> Output;
}

impl Digest for Sha256 {
    fn new() -> Sha256 { todo!() }
    fn update(&mut self, data: impl AsRef<[u8]>) { todo!() }
    fn finalize(self) -> Output { todo!() }
    fn digest(data: impl AsRef<[u8]>) -> Output { todo!() }
}

/// Real `sha2` returns `GenericArray<u8, U32>`, a `typenum`-sized array. That
/// machinery buys the corpus nothing — every use immediately slices or copies
/// the bytes — so it is declared as the fixed-size array it always is.
pub struct Output;

impl Output {
    pub fn as_slice(&self) -> &[u8] { todo!() }
    pub fn to_vec(&self) -> Vec<u8> { todo!() }
}

impl Deref for Output {
    type Target = [u8];
    fn deref(&self) -> &[u8] { todo!() }
}

impl AsRef<[u8]> for Output { fn as_ref(&self) -> &[u8] { todo!() } }
