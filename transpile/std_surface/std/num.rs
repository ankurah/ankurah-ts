//! `std::num`
//!
//! The named types only. The inherent `impl u64 { .. }` blocks that used to
//! share this file moved to `std/primitive.rs`: a file holding both a primitive
//! impl block and a named type forces the loader to pick one module mapping for
//! the whole file, and picking `std` put `ParseIntError` at
//! `std::ParseIntError`, where no qualified reference in the corpus finds it.
//! A primitive impl block attaches to its type structurally and so may live
//! anywhere; a named type must live in the file whose path is its module.

pub struct ParseIntError;
pub struct ParseFloatError;
pub struct TryFromIntError;

impl Debug for ParseIntError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for ParseIntError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for ParseIntError { fn clone(&self) -> ParseIntError { todo!() } }
impl PartialEq for ParseIntError { fn eq(&self, other: &ParseIntError) -> bool { todo!() } }
impl std::error::Error for ParseIntError {}

impl Debug for ParseFloatError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for ParseFloatError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for ParseFloatError { fn clone(&self) -> ParseFloatError { todo!() } }
impl std::error::Error for ParseFloatError {}

impl Debug for TryFromIntError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for TryFromIntError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for TryFromIntError { fn clone(&self) -> TryFromIntError { todo!() } }
impl std::error::Error for TryFromIntError {}

pub struct NonZeroUsize;
pub struct NonZeroU32;
pub struct NonZeroU64;

impl NonZeroUsize {
    pub fn new(n: usize) -> Option<NonZeroUsize> { todo!() }
    pub fn get(self) -> usize { todo!() }
}
