//! `std::fmt`
//!
//! `write!(f, ..)` expands to `f.write_fmt(format_args!(..))`, and the engine
//! does not expand macros — the macro handler supplies the call's type from
//! `Write::write_fmt` declared here (spec 4.10). The `DebugStruct` chain is the
//! one the oracle walked in `signals/src/broadcast.rs`:
//! `Formatter::debug_struct` -> `DebugStruct::field` -> `DebugStruct::finish`.

pub type Result = std::result::Result<(), std::fmt::Error>;

pub struct Error;

impl Debug for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for Error { fn clone(&self) -> Error { todo!() } }
impl PartialEq for Error { fn eq(&self, other: &Error) -> bool { todo!() } }
impl std::error::Error for Error {}

pub trait Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result;
}

pub trait Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result;
}

pub trait Binary { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result; }
pub trait Octal { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result; }
pub trait LowerHex { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result; }
pub trait UpperHex { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result; }
pub trait Pointer { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result; }

pub trait Write {
    fn write_str(&mut self, s: &str) -> std::fmt::Result;
    fn write_char(&mut self, c: char) -> std::fmt::Result;
    fn write_fmt(&mut self, args: Arguments<'_>) -> std::fmt::Result;
}

pub struct Formatter<'a>;

impl<'a> Formatter<'a> {
    pub fn write_str(&mut self, data: &str) -> std::fmt::Result { todo!() }
    pub fn write_fmt(&mut self, args: Arguments<'_>) -> std::fmt::Result { todo!() }
    pub fn pad(&mut self, s: &str) -> std::fmt::Result { todo!() }
    pub fn pad_integral(&mut self, is_nonneg: bool, prefix: &str, buf: &str) -> std::fmt::Result { todo!() }

    pub fn alternate(&self) -> bool { todo!() }
    pub fn width(&self) -> Option<usize> { todo!() }
    pub fn precision(&self) -> Option<usize> { todo!() }
    pub fn fill(&self) -> char { todo!() }
    pub fn sign_plus(&self) -> bool { todo!() }
    pub fn sign_minus(&self) -> bool { todo!() }

    pub fn debug_struct<'b>(&'b mut self, name: &str) -> DebugStruct<'b, 'a> { todo!() }
    pub fn debug_tuple<'b>(&'b mut self, name: &str) -> DebugTuple<'b, 'a> { todo!() }
    pub fn debug_list<'b>(&'b mut self) -> DebugList<'b, 'a> { todo!() }
    pub fn debug_set<'b>(&'b mut self) -> DebugSet<'b, 'a> { todo!() }
    pub fn debug_map<'b>(&'b mut self) -> DebugMap<'b, 'a> { todo!() }
}

impl<'a> Write for Formatter<'a> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result { todo!() }
    fn write_char(&mut self, c: char) -> std::fmt::Result { todo!() }
    fn write_fmt(&mut self, args: Arguments<'_>) -> std::fmt::Result { todo!() }
}

impl<'a> Debug for Formatter<'a> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct DebugStruct<'a, 'b>;

impl<'a, 'b> DebugStruct<'a, 'b> {
    pub fn field(&mut self, name: &str, value: &dyn Debug) -> &mut DebugStruct<'a, 'b> { todo!() }
    pub fn finish(&mut self) -> std::fmt::Result { todo!() }
    pub fn finish_non_exhaustive(&mut self) -> std::fmt::Result { todo!() }
}

pub struct DebugTuple<'a, 'b>;

impl<'a, 'b> DebugTuple<'a, 'b> {
    pub fn field(&mut self, value: &dyn Debug) -> &mut DebugTuple<'a, 'b> { todo!() }
    pub fn finish(&mut self) -> std::fmt::Result { todo!() }
}

pub struct DebugList<'a, 'b>;

impl<'a, 'b> DebugList<'a, 'b> {
    pub fn entry(&mut self, entry: &dyn Debug) -> &mut DebugList<'a, 'b> { todo!() }
    pub fn entries<D: Debug, I: IntoIterator<Item = D>>(&mut self, entries: I) -> &mut DebugList<'a, 'b> { todo!() }
    pub fn finish(&mut self) -> std::fmt::Result { todo!() }
}

pub struct DebugSet<'a, 'b>;

impl<'a, 'b> DebugSet<'a, 'b> {
    pub fn entry(&mut self, entry: &dyn Debug) -> &mut DebugSet<'a, 'b> { todo!() }
    pub fn entries<D: Debug, I: IntoIterator<Item = D>>(&mut self, entries: I) -> &mut DebugSet<'a, 'b> { todo!() }
    pub fn finish(&mut self) -> std::fmt::Result { todo!() }
}

pub struct DebugMap<'a, 'b>;

impl<'a, 'b> DebugMap<'a, 'b> {
    pub fn entry(&mut self, key: &dyn Debug, value: &dyn Debug) -> &mut DebugMap<'a, 'b> { todo!() }
    pub fn key(&mut self, key: &dyn Debug) -> &mut DebugMap<'a, 'b> { todo!() }
    pub fn value(&mut self, value: &dyn Debug) -> &mut DebugMap<'a, 'b> { todo!() }
    pub fn finish(&mut self) -> std::fmt::Result { todo!() }
}

pub struct Arguments<'a>;

impl<'a> Arguments<'a> {
    pub fn as_str(&self) -> Option<&'static str> { todo!() }
}

impl<'a> Debug for Arguments<'a> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<'a> Display for Arguments<'a> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub fn format(args: Arguments<'_>) -> String { todo!() }

// A `&T` is `Debug`/`Display` when `T` is. `format!("{}", &x)` and every
// `.to_string()` on a reference goes through these.
impl<T: ?Sized + Debug> Debug for &T { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: ?Sized + Debug> Debug for &mut T { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: ?Sized + Display> Display for &T { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: ?Sized + Display> Display for &mut T { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

impl Debug for bool { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for bool { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for char { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for char { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for u8 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for u8 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl LowerHex for u8 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for u16 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for u16 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for u32 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for u32 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for u64 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for u64 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for u128 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for u128 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for usize { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for usize { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for i16 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for i16 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for i32 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for i32 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for i64 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for i64 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for isize { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for isize { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for f32 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for f32 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for f64 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for f64 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
