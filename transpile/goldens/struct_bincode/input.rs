use serde::{Deserialize, Serialize};

/// A named-field struct that travels over the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub id: u64,
    pub label: String,
    pub payload: Vec<u8>,
}

impl Envelope {
    pub fn new(id: u64, label: String, payload: Vec<u8>) -> Self { Envelope { id, label, payload } }
}

/// A newtype over bytes, the shape ankurah uses for attestations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signature(Vec<u8>);
