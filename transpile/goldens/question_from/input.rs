//! `?` across two error types calls `From` on the error, which is what Rust
//! does there and what handing the error on unchanged did not.
//!
//! Three shapes: the two error types differ and an impl converts them; the two
//! agree and nothing is written; and the value the `?` produced is what the
//! position said it would be.

pub struct Wire {
    pub code: u32,
}

pub struct Wrapped {
    pub code: u32,
    pub context: String,
}

impl From<Wire> for Wrapped {
    fn from(wire: Wire) -> Wrapped {
        Wrapped { code: wire.code, context: "wire".to_string() }
    }
}

pub fn read(raw: &str) -> Result<u32, Wire> {
    if raw.is_empty() {
        return Err(Wire { code: 7 });
    }
    Ok(raw.len() as u32)
}

/// The error types differ, so `?` converts: `Wrapped::from(wire)`.
pub fn wrapped(raw: &str) -> Result<u32, Wrapped> {
    let n = read(raw)?;
    Ok(n + 1)
}

/// The error types agree, so nothing is written around the error.
pub fn passed_on(raw: &str) -> Result<u32, Wire> {
    let n = read(raw)?;
    Ok(n + 1)
}

/// A `?` whose value the position names: the `Ok` payload leaves through the
/// function's own return type.
pub fn doubled(raw: &str) -> Result<u32, Wrapped> {
    Ok(wrapped(raw)? * 2)
}
