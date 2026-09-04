//! `bool` and `char`.
//!
//! std spreads these over `core::bool` and `core::char`; one file is enough for
//! what the corpus reaches.

impl bool {
    pub fn then<T, F: FnOnce() -> T>(self, f: F) -> Option<T> { todo!() }
    pub fn then_some<T>(self, t: T) -> Option<T> { todo!() }
}

impl char {
    pub const MAX: char = 'a';

    pub fn is_alphanumeric(self) -> bool { todo!() }
    pub fn is_alphabetic(self) -> bool { todo!() }
    pub fn is_numeric(self) -> bool { todo!() }
    pub fn is_whitespace(self) -> bool { todo!() }
    pub fn is_uppercase(self) -> bool { todo!() }
    pub fn is_lowercase(self) -> bool { todo!() }
    pub fn is_ascii(&self) -> bool { todo!() }
    pub fn is_ascii_digit(&self) -> bool { todo!() }
    pub fn is_ascii_alphanumeric(&self) -> bool { todo!() }
    pub fn is_ascii_hexdigit(&self) -> bool { todo!() }
    pub fn is_digit(self, radix: u32) -> bool { todo!() }
    pub fn to_digit(self, radix: u32) -> Option<u32> { todo!() }
    pub fn to_ascii_lowercase(&self) -> char { todo!() }
    pub fn to_ascii_uppercase(&self) -> char { todo!() }
    pub fn to_lowercase(self) -> ToLowercase { todo!() }
    pub fn to_uppercase(self) -> ToUppercase { todo!() }
    pub fn len_utf8(self) -> usize { todo!() }
    pub fn from_u32(i: u32) -> Option<char> { todo!() }
    pub fn from_digit(num: u32, radix: u32) -> Option<char> { todo!() }
}

pub struct ToLowercase;
pub struct ToUppercase;
pub struct CharTryFromError;
/// The error `"x".parse::<char>()` yields. Distinct from `CharTryFromError`,
/// which is what `u32::try_into::<char>()` yields.
pub struct ParseCharError;

impl Iterator for ToLowercase { type Item = char; fn next(&mut self) -> Option<char> { todo!() } }
impl Iterator for ToUppercase { type Item = char; fn next(&mut self) -> Option<char> { todo!() } }
impl Display for ToLowercase { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for ToUppercase { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for ParseCharError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for ParseCharError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for ParseCharError { fn clone(&self) -> ParseCharError { todo!() } }
impl std::error::Error for ParseCharError {}
impl Debug for CharTryFromError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for CharTryFromError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for CharTryFromError {}

impl Clone for bool { fn clone(&self) -> bool { todo!() } }
impl Copy for bool {}
impl PartialEq for bool { fn eq(&self, other: &bool) -> bool { todo!() } }
impl Eq for bool {}
impl PartialOrd for bool { fn partial_cmp(&self, other: &bool) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for bool { fn cmp(&self, other: &bool) -> std::cmp::Ordering { todo!() } }

impl Clone for char { fn clone(&self) -> char { todo!() } }
impl Copy for char {}
impl PartialEq for char { fn eq(&self, other: &char) -> bool { todo!() } }
impl Eq for char {}
impl PartialOrd for char { fn partial_cmp(&self, other: &char) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for char { fn cmp(&self, other: &char) -> std::cmp::Ordering { todo!() } }
