//! `std::ffi`
//!
//! Only `NulError`, and only because `rusqlite::Error::NulError(NulError)` names
//! it. The port has no C strings; this exists so that rusqlite's error enum has
//! a complete set of variant payload types and its `Debug`/`Display` signatures
//! survive into the table.

pub struct NulError;

impl NulError {
    pub fn nul_position(&self) -> usize { todo!() }
    pub fn into_vec(self) -> Vec<u8> { todo!() }
}

impl Debug for NulError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for NulError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for NulError { fn clone(&self) -> NulError { todo!() } }
impl PartialEq for NulError { fn eq(&self, other: &NulError) -> bool { todo!() } }
impl Eq for NulError {}
impl std::error::Error for NulError {}
