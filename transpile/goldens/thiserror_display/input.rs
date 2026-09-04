//! `#[derive(thiserror::Error)]`, emitted rather than expanded.
//!
//! The `#[error("..")]` on each variant is what the type prints, and the port
//! has to print the same sentence: it goes into logs, into test assertions and
//! back to a user. A `#[from]` field is the conversion a `?` calls when it
//! carries an inner error out through this type, and it is named the way every
//! other `From` impl's method is named, so the `?` site and the class agree.

use thiserror::Error;

#[derive(Debug)]
pub struct Rule {
    pub name: String,
}

#[derive(Debug)]
pub struct Io {
    pub code: u32,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Empty expression")]
    Empty,
    #[error("Syntax error: {0}")]
    Syntax(String),
    #[error("Expected {expected}, got {got:?}")]
    Unexpected { expected: String, got: Rule },
    #[error("Invalid predicate: {0}")]
    Invalid(String),
    #[error("read failed")]
    Read(#[from] Io),
}

/// A `?` across the `#[from]` conversion calls the static the derive wrote.
pub fn parse(source: &Io) -> Result<u32, ParseError> {
    let n = read(source)?;
    Ok(n)
}

pub fn read(source: &Io) -> Result<u32, Io> {
    Ok(source.code)
}
