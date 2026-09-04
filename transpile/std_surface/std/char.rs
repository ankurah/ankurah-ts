//! `std::char`
//!
//! `to_lowercase` and `to_uppercase` on a `char` return iterators, not `char`,
//! because one character can uppercase to several. Those iterators and the two
//! char error types used to live in `std/primitive.rs`, which put them at
//! `std::primitive::ToLowercase` — a module std does not have. `std::char` is
//! their module, so this is their file.

pub struct ToLowercase;
pub struct ToUppercase;
pub struct CharTryFromError;
/// The error `"x".parse::<char>()` yields. Distinct from `CharTryFromError`,
/// which is what `u32::try_into::<char>()` yields.
pub struct ParseCharError;

impl Iterator for ToLowercase { type Item = char; fn next(&mut self) -> Option<char> { todo!() } }
impl Iterator for ToUppercase { type Item = char; fn next(&mut self) -> Option<char> { todo!() } }
impl std::fmt::Display for ToLowercase { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for ToUppercase { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for ParseCharError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for ParseCharError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for ParseCharError { fn clone(&self) -> ParseCharError { todo!() } }
impl std::error::Error for ParseCharError {}
impl Debug for CharTryFromError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for CharTryFromError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for CharTryFromError {}

/// `from_u32` and `from_digit` are free functions in `std::char` as well as
/// inherent associated functions on `char`; the inherent forms stay with the
/// other `impl char` methods in `std/primitive.rs`.
pub fn from_u32(i: u32) -> Option<char> { todo!() }
pub fn from_digit(num: u32, radix: u32) -> Option<char> { todo!() }
