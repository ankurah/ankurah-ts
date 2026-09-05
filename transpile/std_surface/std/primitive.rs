//! The inherent and trait impls on the primitive types: `bool`, `char`, the
//! integers and the floats.
//!
//! This file is a drawer, not a module. `impl u64 { .. }`, `impl bool { .. }`
//! and their trait impls attach to the primitive type itself and belong to no
//! module, so the loader places them structurally and the file they sit in does
//! not matter. What does matter is that no *named* type is declared here: a
//! named type takes its module from its file's path, and `std::primitive` is
//! not where any of these live. `ParseIntError`, `ParseFloatError`,
//! `TryFromIntError` and the `NonZero` types are in `std/num.rs`;
//! `ToLowercase`, `ToUppercase`, `ParseCharError` and `CharTryFromError` are in
//! `std/char.rs`.

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
    pub fn to_lowercase(self) -> std::char::ToLowercase { todo!() }
    pub fn to_uppercase(self) -> std::char::ToUppercase { todo!() }
    pub fn len_utf8(self) -> usize { todo!() }
    pub fn from_u32(i: u32) -> Option<char> { todo!() }
    pub fn from_digit(num: u32, radix: u32) -> Option<char> { todo!() }
}

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

impl u8 {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 0;
    pub fn to_be_bytes(self) -> [u8; 1] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 1]) -> u8 { todo!() }
    pub fn min(self, other: u8) -> u8 { todo!() }
    pub fn max(self, other: u8) -> u8 { todo!() }
    pub fn is_ascii_digit(&self) -> bool { todo!() }
    pub fn is_ascii_alphanumeric(&self) -> bool { todo!() }
    pub fn to_ascii_lowercase(&self) -> u8 { todo!() }
    pub fn to_ascii_uppercase(&self) -> u8 { todo!() }
    // Rust declares the four explicit arithmetic families on EVERY integer
    // width, and the port's `@ankurah/base` has a free helper for each. Where a
    // width was missing one, the call resolved to nothing and every value it
    // produced was left untyped, so the next call fell back too.
    pub fn wrapping_add(self, rhs: u8) -> u8 { todo!() }
    pub fn wrapping_sub(self, rhs: u8) -> u8 { todo!() }
    pub fn wrapping_mul(self, rhs: u8) -> u8 { todo!() }
    pub fn saturating_add(self, rhs: u8) -> u8 { todo!() }
    pub fn saturating_sub(self, rhs: u8) -> u8 { todo!() }
    pub fn saturating_mul(self, rhs: u8) -> u8 { todo!() }
    pub fn checked_add(self, rhs: u8) -> Option<u8> { todo!() }
    pub fn checked_sub(self, rhs: u8) -> Option<u8> { todo!() }
    pub fn checked_mul(self, rhs: u8) -> Option<u8> { todo!() }
    pub fn checked_div(self, rhs: u8) -> Option<u8> { todo!() }
    pub fn checked_rem(self, rhs: u8) -> Option<u8> { todo!() }
    pub fn overflowing_add(self, rhs: u8) -> (u8, bool) { todo!() }
    pub fn overflowing_sub(self, rhs: u8) -> (u8, bool) { todo!() }
    pub fn overflowing_mul(self, rhs: u8) -> (u8, bool) { todo!() }
}

impl u16 {
    pub const MIN: u16 = 0;
    pub const MAX: u16 = 0;
    pub fn to_be_bytes(self) -> [u8; 2] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 2]) -> u16 { todo!() }
    pub fn to_le_bytes(self) -> [u8; 2] { todo!() }
    pub fn min(self, other: u16) -> u16 { todo!() }
    pub fn max(self, other: u16) -> u16 { todo!() }
    // Rust declares the four explicit arithmetic families on EVERY integer
    // width, and the port's `@ankurah/base` has a free helper for each. Where a
    // width was missing one, the call resolved to nothing and every value it
    // produced was left untyped, so the next call fell back too.
    pub fn wrapping_add(self, rhs: u16) -> u16 { todo!() }
    pub fn wrapping_sub(self, rhs: u16) -> u16 { todo!() }
    pub fn wrapping_mul(self, rhs: u16) -> u16 { todo!() }
    pub fn saturating_add(self, rhs: u16) -> u16 { todo!() }
    pub fn saturating_sub(self, rhs: u16) -> u16 { todo!() }
    pub fn saturating_mul(self, rhs: u16) -> u16 { todo!() }
    pub fn checked_add(self, rhs: u16) -> Option<u16> { todo!() }
    pub fn checked_sub(self, rhs: u16) -> Option<u16> { todo!() }
    pub fn checked_mul(self, rhs: u16) -> Option<u16> { todo!() }
    pub fn checked_div(self, rhs: u16) -> Option<u16> { todo!() }
    pub fn checked_rem(self, rhs: u16) -> Option<u16> { todo!() }
    pub fn overflowing_add(self, rhs: u16) -> (u16, bool) { todo!() }
    pub fn overflowing_sub(self, rhs: u16) -> (u16, bool) { todo!() }
    pub fn overflowing_mul(self, rhs: u16) -> (u16, bool) { todo!() }
}

impl u32 {
    pub const MIN: u32 = 0;
    pub const MAX: u32 = 0;
    pub fn to_be_bytes(self) -> [u8; 4] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 4]) -> u32 { todo!() }
    pub fn to_le_bytes(self) -> [u8; 4] { todo!() }
    pub fn from_le_bytes(bytes: [u8; 4]) -> u32 { todo!() }
    pub fn pow(self, exp: u32) -> u32 { todo!() }
    pub fn min(self, other: u32) -> u32 { todo!() }
    pub fn max(self, other: u32) -> u32 { todo!() }
    pub fn count_ones(self) -> u32 { todo!() }
    pub fn leading_zeros(self) -> u32 { todo!() }
    pub fn trailing_zeros(self) -> u32 { todo!() }
    // Rust declares the four explicit arithmetic families on EVERY integer
    // width, and the port's `@ankurah/base` has a free helper for each. Where a
    // width was missing one, the call resolved to nothing and every value it
    // produced was left untyped, so the next call fell back too.
    pub fn wrapping_add(self, rhs: u32) -> u32 { todo!() }
    pub fn wrapping_sub(self, rhs: u32) -> u32 { todo!() }
    pub fn wrapping_mul(self, rhs: u32) -> u32 { todo!() }
    pub fn saturating_add(self, rhs: u32) -> u32 { todo!() }
    pub fn saturating_sub(self, rhs: u32) -> u32 { todo!() }
    pub fn saturating_mul(self, rhs: u32) -> u32 { todo!() }
    pub fn checked_add(self, rhs: u32) -> Option<u32> { todo!() }
    pub fn checked_sub(self, rhs: u32) -> Option<u32> { todo!() }
    pub fn checked_mul(self, rhs: u32) -> Option<u32> { todo!() }
    pub fn checked_div(self, rhs: u32) -> Option<u32> { todo!() }
    pub fn checked_rem(self, rhs: u32) -> Option<u32> { todo!() }
    pub fn overflowing_add(self, rhs: u32) -> (u32, bool) { todo!() }
    pub fn overflowing_sub(self, rhs: u32) -> (u32, bool) { todo!() }
    pub fn overflowing_mul(self, rhs: u32) -> (u32, bool) { todo!() }
}

impl u64 {
    pub const MIN: u64 = 0;
    pub const MAX: u64 = 0;
    pub fn to_be_bytes(self) -> [u8; 8] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 8]) -> u64 { todo!() }
    pub fn to_le_bytes(self) -> [u8; 8] { todo!() }
    pub fn from_le_bytes(bytes: [u8; 8]) -> u64 { todo!() }
    pub fn pow(self, exp: u32) -> u64 { todo!() }
    pub fn min(self, other: u64) -> u64 { todo!() }
    pub fn max(self, other: u64) -> u64 { todo!() }
    pub fn count_ones(self) -> u32 { todo!() }
    pub fn leading_zeros(self) -> u32 { todo!() }
    pub fn trailing_zeros(self) -> u32 { todo!() }
    // Rust declares the four explicit arithmetic families on EVERY integer
    // width, and the port's `@ankurah/base` has a free helper for each. Where a
    // width was missing one, the call resolved to nothing and every value it
    // produced was left untyped, so the next call fell back too.
    pub fn wrapping_add(self, rhs: u64) -> u64 { todo!() }
    pub fn wrapping_sub(self, rhs: u64) -> u64 { todo!() }
    pub fn wrapping_mul(self, rhs: u64) -> u64 { todo!() }
    pub fn saturating_add(self, rhs: u64) -> u64 { todo!() }
    pub fn saturating_sub(self, rhs: u64) -> u64 { todo!() }
    pub fn saturating_mul(self, rhs: u64) -> u64 { todo!() }
    pub fn checked_add(self, rhs: u64) -> Option<u64> { todo!() }
    pub fn checked_sub(self, rhs: u64) -> Option<u64> { todo!() }
    pub fn checked_mul(self, rhs: u64) -> Option<u64> { todo!() }
    pub fn checked_div(self, rhs: u64) -> Option<u64> { todo!() }
    pub fn checked_rem(self, rhs: u64) -> Option<u64> { todo!() }
    pub fn overflowing_add(self, rhs: u64) -> (u64, bool) { todo!() }
    pub fn overflowing_sub(self, rhs: u64) -> (u64, bool) { todo!() }
    pub fn overflowing_mul(self, rhs: u64) -> (u64, bool) { todo!() }
}

impl u128 {
    pub const MIN: u128 = 0;
    pub const MAX: u128 = 0;
    pub fn to_be_bytes(self) -> [u8; 16] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 16]) -> u128 { todo!() }
    // Rust declares the four explicit arithmetic families on EVERY integer
    // width, and the port's `@ankurah/base` has a free helper for each. Where a
    // width was missing one, the call resolved to nothing and every value it
    // produced was left untyped, so the next call fell back too.
    pub fn wrapping_add(self, rhs: u128) -> u128 { todo!() }
    pub fn wrapping_sub(self, rhs: u128) -> u128 { todo!() }
    pub fn wrapping_mul(self, rhs: u128) -> u128 { todo!() }
    pub fn saturating_add(self, rhs: u128) -> u128 { todo!() }
    pub fn saturating_sub(self, rhs: u128) -> u128 { todo!() }
    pub fn saturating_mul(self, rhs: u128) -> u128 { todo!() }
    pub fn checked_add(self, rhs: u128) -> Option<u128> { todo!() }
    pub fn checked_sub(self, rhs: u128) -> Option<u128> { todo!() }
    pub fn checked_mul(self, rhs: u128) -> Option<u128> { todo!() }
    pub fn checked_div(self, rhs: u128) -> Option<u128> { todo!() }
    pub fn checked_rem(self, rhs: u128) -> Option<u128> { todo!() }
    pub fn overflowing_add(self, rhs: u128) -> (u128, bool) { todo!() }
    pub fn overflowing_sub(self, rhs: u128) -> (u128, bool) { todo!() }
    pub fn overflowing_mul(self, rhs: u128) -> (u128, bool) { todo!() }
}

// `usize` and `isize` are pointer-sized, and the transpiler evaluates cfg as
// ankurah's wasm32 configuration, so a pointer is 4 bytes here. `to_be_bytes`
// on a 64-bit host would be `[u8; 8]`; on the target this surface describes it
// is `[u8; 4]`, and the collation code's byte layout depends on which.
impl usize {
    pub const MIN: usize = 0;
    pub const MAX: usize = 0;
    pub const BITS: u32 = 32;
    pub fn to_be_bytes(self) -> [u8; 4] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 4]) -> usize { todo!() }
    pub fn to_le_bytes(self) -> [u8; 4] { todo!() }
    pub fn from_le_bytes(bytes: [u8; 4]) -> usize { todo!() }
    pub fn pow(self, exp: u32) -> usize { todo!() }
    pub fn min(self, other: usize) -> usize { todo!() }
    pub fn max(self, other: usize) -> usize { todo!() }
    // Rust declares the four explicit arithmetic families on EVERY integer
    // width, and the port's `@ankurah/base` has a free helper for each. Where a
    // width was missing one, the call resolved to nothing and every value it
    // produced was left untyped, so the next call fell back too.
    pub fn wrapping_add(self, rhs: usize) -> usize { todo!() }
    pub fn wrapping_sub(self, rhs: usize) -> usize { todo!() }
    pub fn wrapping_mul(self, rhs: usize) -> usize { todo!() }
    pub fn saturating_add(self, rhs: usize) -> usize { todo!() }
    pub fn saturating_sub(self, rhs: usize) -> usize { todo!() }
    pub fn saturating_mul(self, rhs: usize) -> usize { todo!() }
    pub fn checked_add(self, rhs: usize) -> Option<usize> { todo!() }
    pub fn checked_sub(self, rhs: usize) -> Option<usize> { todo!() }
    pub fn checked_mul(self, rhs: usize) -> Option<usize> { todo!() }
    pub fn checked_div(self, rhs: usize) -> Option<usize> { todo!() }
    pub fn checked_rem(self, rhs: usize) -> Option<usize> { todo!() }
    pub fn overflowing_add(self, rhs: usize) -> (usize, bool) { todo!() }
    pub fn overflowing_sub(self, rhs: usize) -> (usize, bool) { todo!() }
    pub fn overflowing_mul(self, rhs: usize) -> (usize, bool) { todo!() }
}

impl i16 {
    pub const MIN: i16 = 0;
    pub const MAX: i16 = 0;
    pub fn to_be_bytes(self) -> [u8; 2] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 2]) -> i16 { todo!() }
    pub fn abs(self) -> i16 { todo!() }
    pub fn signum(self) -> i16 { todo!() }
    pub fn min(self, other: i16) -> i16 { todo!() }
    pub fn max(self, other: i16) -> i16 { todo!() }
    // Rust declares the four explicit arithmetic families on EVERY integer
    // width, and the port's `@ankurah/base` has a free helper for each. Where a
    // width was missing one, the call resolved to nothing and every value it
    // produced was left untyped, so the next call fell back too.
    pub fn wrapping_add(self, rhs: i16) -> i16 { todo!() }
    pub fn wrapping_sub(self, rhs: i16) -> i16 { todo!() }
    pub fn wrapping_mul(self, rhs: i16) -> i16 { todo!() }
    pub fn saturating_add(self, rhs: i16) -> i16 { todo!() }
    pub fn saturating_sub(self, rhs: i16) -> i16 { todo!() }
    pub fn saturating_mul(self, rhs: i16) -> i16 { todo!() }
    pub fn checked_add(self, rhs: i16) -> Option<i16> { todo!() }
    pub fn checked_sub(self, rhs: i16) -> Option<i16> { todo!() }
    pub fn checked_mul(self, rhs: i16) -> Option<i16> { todo!() }
    pub fn checked_div(self, rhs: i16) -> Option<i16> { todo!() }
    pub fn checked_rem(self, rhs: i16) -> Option<i16> { todo!() }
    pub fn overflowing_add(self, rhs: i16) -> (i16, bool) { todo!() }
    pub fn overflowing_sub(self, rhs: i16) -> (i16, bool) { todo!() }
    pub fn overflowing_mul(self, rhs: i16) -> (i16, bool) { todo!() }
}

impl i32 {
    pub const MIN: i32 = 0;
    pub const MAX: i32 = 0;
    pub fn to_be_bytes(self) -> [u8; 4] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 4]) -> i32 { todo!() }
    pub fn to_le_bytes(self) -> [u8; 4] { todo!() }
    pub fn abs(self) -> i32 { todo!() }
    pub fn signum(self) -> i32 { todo!() }
    pub fn pow(self, exp: u32) -> i32 { todo!() }
    pub fn min(self, other: i32) -> i32 { todo!() }
    pub fn max(self, other: i32) -> i32 { todo!() }
    // Rust declares the four explicit arithmetic families on EVERY integer
    // width, and the port's `@ankurah/base` has a free helper for each. Where a
    // width was missing one, the call resolved to nothing and every value it
    // produced was left untyped, so the next call fell back too.
    pub fn wrapping_add(self, rhs: i32) -> i32 { todo!() }
    pub fn wrapping_sub(self, rhs: i32) -> i32 { todo!() }
    pub fn wrapping_mul(self, rhs: i32) -> i32 { todo!() }
    pub fn saturating_add(self, rhs: i32) -> i32 { todo!() }
    pub fn saturating_sub(self, rhs: i32) -> i32 { todo!() }
    pub fn saturating_mul(self, rhs: i32) -> i32 { todo!() }
    pub fn checked_add(self, rhs: i32) -> Option<i32> { todo!() }
    pub fn checked_sub(self, rhs: i32) -> Option<i32> { todo!() }
    pub fn checked_mul(self, rhs: i32) -> Option<i32> { todo!() }
    pub fn checked_div(self, rhs: i32) -> Option<i32> { todo!() }
    pub fn checked_rem(self, rhs: i32) -> Option<i32> { todo!() }
    pub fn overflowing_add(self, rhs: i32) -> (i32, bool) { todo!() }
    pub fn overflowing_sub(self, rhs: i32) -> (i32, bool) { todo!() }
    pub fn overflowing_mul(self, rhs: i32) -> (i32, bool) { todo!() }
}

impl i64 {
    pub const MIN: i64 = 0;
    pub const MAX: i64 = 0;
    pub fn to_be_bytes(self) -> [u8; 8] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 8]) -> i64 { todo!() }
    pub fn to_le_bytes(self) -> [u8; 8] { todo!() }
    pub fn from_le_bytes(bytes: [u8; 8]) -> i64 { todo!() }
    pub fn abs(self) -> i64 { todo!() }
    pub fn signum(self) -> i64 { todo!() }
    pub fn pow(self, exp: u32) -> i64 { todo!() }
    pub fn min(self, other: i64) -> i64 { todo!() }
    pub fn max(self, other: i64) -> i64 { todo!() }
    // Rust declares the four explicit arithmetic families on EVERY integer
    // width, and the port's `@ankurah/base` has a free helper for each. Where a
    // width was missing one, the call resolved to nothing and every value it
    // produced was left untyped, so the next call fell back too.
    pub fn wrapping_add(self, rhs: i64) -> i64 { todo!() }
    pub fn wrapping_sub(self, rhs: i64) -> i64 { todo!() }
    pub fn wrapping_mul(self, rhs: i64) -> i64 { todo!() }
    pub fn saturating_add(self, rhs: i64) -> i64 { todo!() }
    pub fn saturating_sub(self, rhs: i64) -> i64 { todo!() }
    pub fn saturating_mul(self, rhs: i64) -> i64 { todo!() }
    pub fn checked_add(self, rhs: i64) -> Option<i64> { todo!() }
    pub fn checked_sub(self, rhs: i64) -> Option<i64> { todo!() }
    pub fn checked_mul(self, rhs: i64) -> Option<i64> { todo!() }
    pub fn checked_div(self, rhs: i64) -> Option<i64> { todo!() }
    pub fn checked_rem(self, rhs: i64) -> Option<i64> { todo!() }
    pub fn overflowing_add(self, rhs: i64) -> (i64, bool) { todo!() }
    pub fn overflowing_sub(self, rhs: i64) -> (i64, bool) { todo!() }
    pub fn overflowing_mul(self, rhs: i64) -> (i64, bool) { todo!() }
}

impl isize {
    pub const MIN: isize = 0;
    pub const MAX: isize = 0;
    pub const BITS: u32 = 32;
    pub fn to_be_bytes(self) -> [u8; 4] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 4]) -> isize { todo!() }
    pub fn abs(self) -> isize { todo!() }
    pub fn min(self, other: isize) -> isize { todo!() }
    pub fn max(self, other: isize) -> isize { todo!() }
    // Rust declares the four explicit arithmetic families on EVERY integer
    // width, and the port's `@ankurah/base` has a free helper for each. Where a
    // width was missing one, the call resolved to nothing and every value it
    // produced was left untyped, so the next call fell back too.
    pub fn wrapping_add(self, rhs: isize) -> isize { todo!() }
    pub fn wrapping_sub(self, rhs: isize) -> isize { todo!() }
    pub fn wrapping_mul(self, rhs: isize) -> isize { todo!() }
    pub fn saturating_add(self, rhs: isize) -> isize { todo!() }
    pub fn saturating_sub(self, rhs: isize) -> isize { todo!() }
    pub fn saturating_mul(self, rhs: isize) -> isize { todo!() }
    pub fn checked_add(self, rhs: isize) -> Option<isize> { todo!() }
    pub fn checked_sub(self, rhs: isize) -> Option<isize> { todo!() }
    pub fn checked_mul(self, rhs: isize) -> Option<isize> { todo!() }
    pub fn checked_div(self, rhs: isize) -> Option<isize> { todo!() }
    pub fn checked_rem(self, rhs: isize) -> Option<isize> { todo!() }
    pub fn overflowing_add(self, rhs: isize) -> (isize, bool) { todo!() }
    pub fn overflowing_sub(self, rhs: isize) -> (isize, bool) { todo!() }
    pub fn overflowing_mul(self, rhs: isize) -> (isize, bool) { todo!() }
}

impl f32 {
    pub const NAN: f32 = 0.0;
    pub const INFINITY: f32 = 0.0;
    pub const NEG_INFINITY: f32 = 0.0;
    pub const EPSILON: f32 = 0.0;
    pub fn to_bits(self) -> u32 { todo!() }
    pub fn from_bits(v: u32) -> f32 { todo!() }
    pub fn to_be_bytes(self) -> [u8; 4] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 4]) -> f32 { todo!() }
    pub fn is_nan(self) -> bool { todo!() }
    pub fn is_infinite(self) -> bool { todo!() }
    pub fn is_finite(self) -> bool { todo!() }
    pub fn is_sign_negative(self) -> bool { todo!() }
    pub fn abs(self) -> f32 { todo!() }
    pub fn fract(self) -> f32 { todo!() }
    pub fn floor(self) -> f32 { todo!() }
    pub fn ceil(self) -> f32 { todo!() }
    pub fn round(self) -> f32 { todo!() }
    pub fn trunc(self) -> f32 { todo!() }
    pub fn min(self, other: f32) -> f32 { todo!() }
    pub fn max(self, other: f32) -> f32 { todo!() }
    pub fn total_cmp(&self, other: &f32) -> std::cmp::Ordering { todo!() }
}

impl f64 {
    pub const NAN: f64 = 0.0;
    pub const INFINITY: f64 = 0.0;
    pub const NEG_INFINITY: f64 = 0.0;
    pub const EPSILON: f64 = 0.0;
    pub const MAX: f64 = 0.0;
    pub const MIN: f64 = 0.0;
    pub fn to_bits(self) -> u64 { todo!() }
    pub fn from_bits(v: u64) -> f64 { todo!() }
    pub fn to_be_bytes(self) -> [u8; 8] { todo!() }
    pub fn from_be_bytes(bytes: [u8; 8]) -> f64 { todo!() }
    pub fn to_le_bytes(self) -> [u8; 8] { todo!() }
    pub fn is_nan(self) -> bool { todo!() }
    pub fn is_infinite(self) -> bool { todo!() }
    pub fn is_finite(self) -> bool { todo!() }
    pub fn is_sign_negative(self) -> bool { todo!() }
    pub fn is_sign_positive(self) -> bool { todo!() }
    pub fn abs(self) -> f64 { todo!() }
    pub fn fract(self) -> f64 { todo!() }
    pub fn floor(self) -> f64 { todo!() }
    pub fn ceil(self) -> f64 { todo!() }
    pub fn round(self) -> f64 { todo!() }
    pub fn trunc(self) -> f64 { todo!() }
    pub fn signum(self) -> f64 { todo!() }
    pub fn sqrt(self) -> f64 { todo!() }
    pub fn powi(self, n: i32) -> f64 { todo!() }
    pub fn powf(self, n: f64) -> f64 { todo!() }
    pub fn min(self, other: f64) -> f64 { todo!() }
    pub fn max(self, other: f64) -> f64 { todo!() }
    pub fn total_cmp(&self, other: &f64) -> std::cmp::Ordering { todo!() }
}

// ── Trait impls on the numeric primitives ────────────────────────────────────
//
// `.cloned()` on an iterator of `&u64` needs `u64: Clone`; `.copied()` needs
// `Copy`; a `BTreeMap<u64, _>` and every `sort()` need `Ord`. Without these the
// primitives look like bare types with no traits.

impl Clone for u8 { fn clone(&self) -> u8 { todo!() } }
impl Copy for u8 {}
impl PartialEq for u8 { fn eq(&self, other: &u8) -> bool { todo!() } }
impl Eq for u8 {}
impl PartialOrd for u8 { fn partial_cmp(&self, other: &u8) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for u8 { fn cmp(&self, other: &u8) -> std::cmp::Ordering { todo!() } }

impl Clone for u16 { fn clone(&self) -> u16 { todo!() } }
impl Copy for u16 {}
impl PartialEq for u16 { fn eq(&self, other: &u16) -> bool { todo!() } }
impl Eq for u16 {}
impl PartialOrd for u16 { fn partial_cmp(&self, other: &u16) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for u16 { fn cmp(&self, other: &u16) -> std::cmp::Ordering { todo!() } }

impl Clone for u32 { fn clone(&self) -> u32 { todo!() } }
impl Copy for u32 {}
impl PartialEq for u32 { fn eq(&self, other: &u32) -> bool { todo!() } }
impl Eq for u32 {}
impl PartialOrd for u32 { fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for u32 { fn cmp(&self, other: &u32) -> std::cmp::Ordering { todo!() } }

impl Clone for u64 { fn clone(&self) -> u64 { todo!() } }
impl Copy for u64 {}
impl PartialEq for u64 { fn eq(&self, other: &u64) -> bool { todo!() } }
impl Eq for u64 {}
impl PartialOrd for u64 { fn partial_cmp(&self, other: &u64) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for u64 { fn cmp(&self, other: &u64) -> std::cmp::Ordering { todo!() } }

impl Clone for u128 { fn clone(&self) -> u128 { todo!() } }
impl Copy for u128 {}
impl PartialEq for u128 { fn eq(&self, other: &u128) -> bool { todo!() } }
impl Eq for u128 {}
impl PartialOrd for u128 { fn partial_cmp(&self, other: &u128) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for u128 { fn cmp(&self, other: &u128) -> std::cmp::Ordering { todo!() } }

impl Clone for usize { fn clone(&self) -> usize { todo!() } }
impl Copy for usize {}
impl PartialEq for usize { fn eq(&self, other: &usize) -> bool { todo!() } }
impl Eq for usize {}
impl PartialOrd for usize { fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for usize { fn cmp(&self, other: &usize) -> std::cmp::Ordering { todo!() } }

impl Clone for i16 { fn clone(&self) -> i16 { todo!() } }
impl Copy for i16 {}
impl PartialEq for i16 { fn eq(&self, other: &i16) -> bool { todo!() } }
impl Eq for i16 {}
impl PartialOrd for i16 { fn partial_cmp(&self, other: &i16) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for i16 { fn cmp(&self, other: &i16) -> std::cmp::Ordering { todo!() } }

impl Clone for i32 { fn clone(&self) -> i32 { todo!() } }
impl Copy for i32 {}
impl PartialEq for i32 { fn eq(&self, other: &i32) -> bool { todo!() } }
impl Eq for i32 {}
impl PartialOrd for i32 { fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for i32 { fn cmp(&self, other: &i32) -> std::cmp::Ordering { todo!() } }

impl Clone for i64 { fn clone(&self) -> i64 { todo!() } }
impl Copy for i64 {}
impl PartialEq for i64 { fn eq(&self, other: &i64) -> bool { todo!() } }
impl Eq for i64 {}
impl PartialOrd for i64 { fn partial_cmp(&self, other: &i64) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for i64 { fn cmp(&self, other: &i64) -> std::cmp::Ordering { todo!() } }

impl Clone for isize { fn clone(&self) -> isize { todo!() } }
impl Copy for isize {}
impl PartialEq for isize { fn eq(&self, other: &isize) -> bool { todo!() } }
impl Eq for isize {}
impl PartialOrd for isize { fn partial_cmp(&self, other: &isize) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for isize { fn cmp(&self, other: &isize) -> std::cmp::Ordering { todo!() } }

// f32 and f64 are `PartialOrd` and not `Ord`; that difference is why the
// collation code sorts floats by `to_bits` rather than by `sort()`.
impl Clone for f32 { fn clone(&self) -> f32 { todo!() } }
impl Copy for f32 {}
impl PartialEq for f32 { fn eq(&self, other: &f32) -> bool { todo!() } }
impl PartialOrd for f32 { fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> { todo!() } }

impl Clone for f64 { fn clone(&self) -> f64 { todo!() } }
impl Copy for f64 {}
impl PartialEq for f64 { fn eq(&self, other: &f64) -> bool { todo!() } }
impl PartialOrd for f64 { fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> { todo!() } }
