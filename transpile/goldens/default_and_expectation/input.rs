//! What an arm is expected to produce, and what `unwrap_or_default` answers.
//! P1, N10, L2.
//!
//! For: an arm's value IS the match's value, so what the position wants of the
//! match it wants of every arm. The keyed `.match({..})` form set no
//! expectation on its arms at all, so a `vec![b as u8]` inside one built a
//! `number[]` where the same expression written as the function's tail builds a
//! `Uint8Array` — and every other arm of the same match answers one. Live at
//! `core/collation.ts` and `core/value/collatable.ts`.
//!
//! `unwrap_or_default` answers the value or the payload type's `Default`, and
//! the port carries no `Default` at all. Written from the method's NAME it came
//! out `x.unwrapOrDefault()`, which nothing declares — eleven emitted sites,
//! and five of them on a `string | null`, which has no members whatever. Where
//! the port can name the default from the resolved type it writes it; where it
//! cannot, the site is a hole rather than `undefined`.

pub enum Lit {
    Bool(bool),
    Text(String),
}

/// P1: the arm's `vec!` is expected to be the function's `Vec<u8>`.
pub fn to_bytes(l: &Lit) -> Vec<u8> {
    match l {
        Lit::Bool(b) => vec![*b as u8],
        Lit::Text(_) => vec![0u8, 1u8],
    }
}

/// N10: on a nullable, `unwrap_or_default` is `??` and the default of the
/// payload.
pub fn text_or_empty(s: Option<String>) -> String {
    s.unwrap_or_default()
}

pub fn count_or_zero(n: Option<u32>) -> u32 {
    n.unwrap_or_default()
}

/// On a `Result` it is `unwrapOr`, which releases the wrapper as Rust does.
pub fn bytes_or_empty(r: Result<Vec<u8>, String>) -> Vec<u8> {
    r.unwrap_or_default()
}

/// L2: the port's `serde_json` reads and writes plain JavaScript values, so a
/// `Value` variant construction is the identity on its payload.
pub fn json_of(flag: bool) -> serde_json::Value {
    serde_json::Value::Bool(flag)
}

pub fn json_null() -> serde_json::Value {
    serde_json::Value::Null
}

/// And `to_vec` is the JSON text as bytes.
pub fn json_bytes(v: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap_or_default()
}
