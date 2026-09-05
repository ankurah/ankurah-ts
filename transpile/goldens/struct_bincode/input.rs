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

/// `usize` is eight bytes on the bincode wire and a `number` in the port, and
/// the conversion belongs at that boundary — INSIDE a sequence's element writer
/// and reader as much as at a field. `writeVec(this.e, (w, item) =>
/// w.writeU64(item))` handed `setBigUint64` a number, which throws, and
/// `readVec((r) => r.readU64())` put a `bigint[]` into a `number[]` field.
#[derive(Serialize, Deserialize)]
pub struct Sizes {
    pub one: usize,
    pub many: Vec<usize>,
    pub nested: Vec<Vec<usize>>,
    pub signed: Vec<isize>,
    pub narrow: Vec<u32>,
}
