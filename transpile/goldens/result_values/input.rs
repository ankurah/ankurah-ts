//! A function returning `Result` returns a `Result` value. `?` tests it, hands
//! the error back as a fresh `Err`, and consumes the wrapper either way — so
//! neither the `Ok` nor the `Err` is left for the leak registry to find.

#[derive(Debug, Clone, PartialEq)]
pub enum WireError {
    Truncated,
}

pub fn width(raw: &str) -> Result<usize, WireError> {
    if raw.is_empty() {
        return Err(WireError::Truncated);
    }
    Ok(raw.len())
}

/// `?` bound to a name.
pub fn bound(raw: &str) -> Result<usize, WireError> {
    let n = width(raw)?;
    Ok(n + 1)
}

/// `?` inside an expression: the test is lifted out of it, so the call happens
/// once and the error leaves the function.
pub fn inside_an_expression(raw: &str) -> Result<usize, WireError> {
    Ok(width(raw)? + 1)
}

/// `?` whose value nobody wants. Rust drops the `Ok` payload at the end of the
/// statement, and the wrapper with it.
pub fn discarded(raw: &str) -> Result<usize, WireError> {
    width(raw)?;
    Ok(0)
}

/// `unwrap_or` on a `Result` is the runtime's own method: `??` reads an object
/// as present and would always take the `Result` itself.
pub fn defaulted(raw: &str) -> usize {
    width(raw).unwrap_or(0)
}
