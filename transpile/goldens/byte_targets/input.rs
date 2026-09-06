//! `collect` into a `Vec<u8>` builds BYTES, not an array of numbers. E1.
//!
//! For: the port writes `Vec<u8>` as a `Uint8Array`, and everything downstream
//! reads it as one — `encode`, a `TextDecoder`, a comparison by byte. `collect`
//! is `FromIterator`, so the TARGET decides what is built; the arm that answers
//! it treated a byte target as "the sequence is already the answer" and handed
//! back the `number[]` the adaptors had built. Three sites in
//! `core/indexing/encoding.ts` returned one behind a
//! `Result<Vec<u8>, IndexError>`, where a descending index component is read
//! back as bytes.
//!
//! The neighbouring `vec![..]` arm has always written `new Uint8Array([..])`,
//! so one function here builds its answer both ways and the two must agree.

/// The `encode_value_component` shape: complement every byte for a descending
/// index component. The parent engine (b05f82c) answered `number[]` here.
pub fn descending(bytes: Vec<u8>) -> Vec<u8> {
    bytes.into_iter().map(|b| 0xFFu8.wrapping_sub(b)).collect()
}

/// The same target named by a `let` annotation rather than by the return type.
pub fn descending_local(bytes: Vec<u8>) -> usize {
    let out: Vec<u8> = bytes.into_iter().map(|b| 0xFFu8.wrapping_sub(b)).collect();
    out.len()
}

/// And by a turbofish, which is the only thing that names the target where the
/// value is consumed on the spot.
pub fn first_complement(bytes: Vec<u8>) -> u8 {
    bytes.into_iter().map(|b| 0xFFu8.wrapping_sub(b)).collect::<Vec<u8>>()[0]
}

/// The other spelling of the same answer, which the port has always written as
/// a `Uint8Array`: the two must agree.
pub fn one_byte(b: u8) -> Vec<u8> {
    vec![0xFFu8.wrapping_sub(b)]
}

/// A `Vec` of anything else is the sequence itself, and is not copied into a
/// byte buffer.
pub fn doubled(ns: Vec<u32>) -> Vec<u32> {
    ns.into_iter().map(|n| n * 2).collect()
}

/// Bytes collected out of a BORROWED slice, which is the other reader shape:
/// `iter().copied()` rather than `into_iter()`.
pub fn copy_of(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied().collect()
}
