//! `std::ops`
//!
//! `?` is not modelled here. Rust's `Try`/`FromResidual` machinery is unstable
//! and the engine handles `?` directly (spec 4.6), so a stub for it would be a
//! declaration nothing reads.

pub trait Deref {
    type Target: ?Sized;
    fn deref(&self) -> &Self::Target;
}

pub trait DerefMut: Deref {
    fn deref_mut(&mut self) -> &mut Self::Target;
}

// `&T` and `&mut T` really do implement `Deref` in std. The engine has a
// builtin rule for stripping a reference (spec 4.3), so it will not consult
// these; they are written because they are true, and because a reader checking
// the surface against std should not find a hole where std has an impl.
impl<T: ?Sized> Deref for &T {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<T: ?Sized> Deref for &mut T {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<T: ?Sized> DerefMut for &mut T {
    fn deref_mut(&mut self) -> &mut T { todo!() }
}

pub trait Drop {
    fn drop(&mut self);
}

pub trait Index<Idx: ?Sized> {
    type Output: ?Sized;
    fn index(&self, index: Idx) -> &Self::Output;
}

pub trait IndexMut<Idx: ?Sized>: std::ops::Index<Idx> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output;
}

pub trait Add<Rhs = Self> { type Output; fn add(self, rhs: Rhs) -> Self::Output; }
pub trait Sub<Rhs = Self> { type Output; fn sub(self, rhs: Rhs) -> Self::Output; }
pub trait Mul<Rhs = Self> { type Output; fn mul(self, rhs: Rhs) -> Self::Output; }
pub trait Div<Rhs = Self> { type Output; fn div(self, rhs: Rhs) -> Self::Output; }
pub trait Rem<Rhs = Self> { type Output; fn rem(self, rhs: Rhs) -> Self::Output; }
pub trait Neg { type Output; fn neg(self) -> Self::Output; }
pub trait Not { type Output; fn not(self) -> Self::Output; }
pub trait BitAnd<Rhs = Self> { type Output; fn bitand(self, rhs: Rhs) -> Self::Output; }
pub trait BitOr<Rhs = Self> { type Output; fn bitor(self, rhs: Rhs) -> Self::Output; }
pub trait BitXor<Rhs = Self> { type Output; fn bitxor(self, rhs: Rhs) -> Self::Output; }
pub trait Shl<Rhs = Self> { type Output; fn shl(self, rhs: Rhs) -> Self::Output; }
pub trait Shr<Rhs = Self> { type Output; fn shr(self, rhs: Rhs) -> Self::Output; }

pub trait AddAssign<Rhs = Self> { fn add_assign(&mut self, rhs: Rhs); }
pub trait SubAssign<Rhs = Self> { fn sub_assign(&mut self, rhs: Rhs); }
pub trait MulAssign<Rhs = Self> { fn mul_assign(&mut self, rhs: Rhs); }
pub trait DivAssign<Rhs = Self> { fn div_assign(&mut self, rhs: Rhs); }
pub trait RemAssign<Rhs = Self> { fn rem_assign(&mut self, rhs: Rhs); }
pub trait BitAndAssign<Rhs = Self> { fn bitand_assign(&mut self, rhs: Rhs); }
pub trait BitOrAssign<Rhs = Self> { fn bitor_assign(&mut self, rhs: Rhs); }
pub trait BitXorAssign<Rhs = Self> { fn bitxor_assign(&mut self, rhs: Rhs); }

// `Args: Tuple` is what stops `FnOnce<u32>` resolving, and the call methods
// really are `extern "rust-call"`. Both are unstable to write in user code and
// both are part of the declaration.
pub trait FnOnce<Args: Tuple> {
    type Output;
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output;
}

pub trait FnMut<Args: Tuple>: FnOnce<Args> {
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output;
}

pub trait Fn<Args: Tuple>: FnMut<Args> {
    extern "rust-call" fn call(&self, args: Args) -> Self::Output;
}

// ── Range types ──────────────────────────────────────────────────────────────

pub struct Range<Idx> { pub start: Idx, pub end: Idx }
pub struct RangeFrom<Idx> { pub start: Idx }
pub struct RangeTo<Idx> { pub end: Idx }
pub struct RangeFull;
pub struct RangeInclusive<Idx>;
pub struct RangeToInclusive<Idx> { pub end: Idx }

impl<Idx: PartialOrd<Idx>> Range<Idx> {
    pub fn contains<U: ?Sized + PartialOrd<Idx>>(&self, item: &U) -> bool where Idx: PartialOrd<U> { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
}

impl<Idx> RangeInclusive<Idx> {
    pub fn new(start: Idx, end: Idx) -> RangeInclusive<Idx> { todo!() }
    pub fn start(&self) -> &Idx { todo!() }
    pub fn end(&self) -> &Idx { todo!() }
    pub fn into_inner(self) -> (Idx, Idx) { todo!() }
}

impl<Idx: PartialOrd<Idx>> RangeInclusive<Idx> {
    pub fn contains<U: ?Sized + PartialOrd<Idx>>(&self, item: &U) -> bool where Idx: PartialOrd<U> { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
}

impl<Idx: Clone> Clone for Range<Idx> { fn clone(&self) -> Range<Idx> { todo!() } }
impl<Idx: Debug> Debug for Range<Idx> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub enum Bound<T> {
    Included(T),
    Excluded(T),
    Unbounded,
}

impl<T: Clone> Clone for Bound<T> { fn clone(&self) -> Bound<T> { todo!() } }
impl<T: Debug> Debug for Bound<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub trait RangeBounds<T: ?Sized> {
    fn start_bound(&self) -> Bound<&T>;
    fn end_bound(&self) -> Bound<&T>;
    fn contains<U: ?Sized>(&self, item: &U) -> bool where T: PartialOrd<U>, U: ?Sized + PartialOrd<T>;
}

impl<T> RangeBounds<T> for Range<T> {
    fn start_bound(&self) -> Bound<&T> { todo!() }
    fn end_bound(&self) -> Bound<&T> { todo!() }
    fn contains<U: ?Sized>(&self, item: &U) -> bool where T: PartialOrd<U>, U: ?Sized + PartialOrd<T> { todo!() }
}
impl<T> RangeBounds<T> for RangeFrom<T> {
    fn start_bound(&self) -> Bound<&T> { todo!() }
    fn end_bound(&self) -> Bound<&T> { todo!() }
    fn contains<U: ?Sized>(&self, item: &U) -> bool where T: PartialOrd<U>, U: ?Sized + PartialOrd<T> { todo!() }
}
impl<T> RangeBounds<T> for RangeTo<T> {
    fn start_bound(&self) -> Bound<&T> { todo!() }
    fn end_bound(&self) -> Bound<&T> { todo!() }
    fn contains<U: ?Sized>(&self, item: &U) -> bool where T: PartialOrd<U>, U: ?Sized + PartialOrd<T> { todo!() }
}
impl<T: ?Sized> RangeBounds<T> for RangeFull {
    fn start_bound(&self) -> Bound<&T> { todo!() }
    fn end_bound(&self) -> Bound<&T> { todo!() }
    fn contains<U: ?Sized>(&self, item: &U) -> bool where T: PartialOrd<U>, U: ?Sized + PartialOrd<T> { todo!() }
}
impl<T> RangeBounds<T> for RangeInclusive<T> {
    fn start_bound(&self) -> Bound<&T> { todo!() }
    fn end_bound(&self) -> Bound<&T> { todo!() }
    fn contains<U: ?Sized>(&self, item: &U) -> bool where T: PartialOrd<U>, U: ?Sized + PartialOrd<T> { todo!() }
}
impl<T> RangeBounds<T> for RangeToInclusive<T> {
    fn start_bound(&self) -> Bound<&T> { todo!() }
    fn end_bound(&self) -> Bound<&T> { todo!() }
    fn contains<U: ?Sized>(&self, item: &U) -> bool where T: PartialOrd<U>, U: ?Sized + PartialOrd<T> { todo!() }
}
impl<T> RangeBounds<T> for (Bound<T>, Bound<T>) {
    fn start_bound(&self) -> Bound<&T> { todo!() }
    fn end_bound(&self) -> Bound<&T> { todo!() }
    fn contains<U: ?Sized>(&self, item: &U) -> bool where T: PartialOrd<U>, U: ?Sized + PartialOrd<T> { todo!() }
}

// ── Operator impls on primitives ─────────────────────────────────────────────
//
// One `impl Add for u8` ... per width would be 8 traits x 12 primitives of
// near-identical text. Only the widths the corpus actually adds, subtracts,
// compares or negates are written out; the rest is a declared gap the engine
// reports rather than guesses (README, "Omissions").

impl Add for usize { type Output = usize; fn add(self, rhs: usize) -> usize { todo!() } }
impl Sub for usize { type Output = usize; fn sub(self, rhs: usize) -> usize { todo!() } }
impl Mul for usize { type Output = usize; fn mul(self, rhs: usize) -> usize { todo!() } }
impl Div for usize { type Output = usize; fn div(self, rhs: usize) -> usize { todo!() } }
impl Rem for usize { type Output = usize; fn rem(self, rhs: usize) -> usize { todo!() } }
impl AddAssign for usize { fn add_assign(&mut self, rhs: usize) { todo!() } }
impl SubAssign for usize { fn sub_assign(&mut self, rhs: usize) { todo!() } }

impl Add for u8 { type Output = u8; fn add(self, rhs: u8) -> u8 { todo!() } }
impl Sub for u8 { type Output = u8; fn sub(self, rhs: u8) -> u8 { todo!() } }
impl Mul for u8 { type Output = u8; fn mul(self, rhs: u8) -> u8 { todo!() } }
impl Div for u8 { type Output = u8; fn div(self, rhs: u8) -> u8 { todo!() } }
impl Rem for u8 { type Output = u8; fn rem(self, rhs: u8) -> u8 { todo!() } }
impl BitAnd for u8 { type Output = u8; fn bitand(self, rhs: u8) -> u8 { todo!() } }
impl BitOr for u8 { type Output = u8; fn bitor(self, rhs: u8) -> u8 { todo!() } }
impl Shl<u32> for u8 { type Output = u8; fn shl(self, rhs: u32) -> u8 { todo!() } }
impl Shr<u32> for u8 { type Output = u8; fn shr(self, rhs: u32) -> u8 { todo!() } }

impl Add for u16 { type Output = u16; fn add(self, rhs: u16) -> u16 { todo!() } }
impl Sub for u16 { type Output = u16; fn sub(self, rhs: u16) -> u16 { todo!() } }

impl Add for u32 { type Output = u32; fn add(self, rhs: u32) -> u32 { todo!() } }
impl Sub for u32 { type Output = u32; fn sub(self, rhs: u32) -> u32 { todo!() } }
impl Mul for u32 { type Output = u32; fn mul(self, rhs: u32) -> u32 { todo!() } }
impl Div for u32 { type Output = u32; fn div(self, rhs: u32) -> u32 { todo!() } }
impl AddAssign for u32 { fn add_assign(&mut self, rhs: u32) { todo!() } }

impl Add for u64 { type Output = u64; fn add(self, rhs: u64) -> u64 { todo!() } }
impl Sub for u64 { type Output = u64; fn sub(self, rhs: u64) -> u64 { todo!() } }
impl Mul for u64 { type Output = u64; fn mul(self, rhs: u64) -> u64 { todo!() } }
impl Div for u64 { type Output = u64; fn div(self, rhs: u64) -> u64 { todo!() } }
impl Rem for u64 { type Output = u64; fn rem(self, rhs: u64) -> u64 { todo!() } }
impl BitAnd for u64 { type Output = u64; fn bitand(self, rhs: u64) -> u64 { todo!() } }
impl BitOr for u64 { type Output = u64; fn bitor(self, rhs: u64) -> u64 { todo!() } }
impl BitXor for u64 { type Output = u64; fn bitxor(self, rhs: u64) -> u64 { todo!() } }
impl Shl<u32> for u64 { type Output = u64; fn shl(self, rhs: u32) -> u64 { todo!() } }
impl Shr<u32> for u64 { type Output = u64; fn shr(self, rhs: u32) -> u64 { todo!() } }
impl AddAssign for u64 { fn add_assign(&mut self, rhs: u64) { todo!() } }

impl Add for i16 { type Output = i16; fn add(self, rhs: i16) -> i16 { todo!() } }
impl Sub for i16 { type Output = i16; fn sub(self, rhs: i16) -> i16 { todo!() } }
impl Neg for i16 { type Output = i16; fn neg(self) -> i16 { todo!() } }

impl Add for i32 { type Output = i32; fn add(self, rhs: i32) -> i32 { todo!() } }
impl Sub for i32 { type Output = i32; fn sub(self, rhs: i32) -> i32 { todo!() } }
impl Mul for i32 { type Output = i32; fn mul(self, rhs: i32) -> i32 { todo!() } }
impl Div for i32 { type Output = i32; fn div(self, rhs: i32) -> i32 { todo!() } }
impl Rem for i32 { type Output = i32; fn rem(self, rhs: i32) -> i32 { todo!() } }
impl Neg for i32 { type Output = i32; fn neg(self) -> i32 { todo!() } }
impl AddAssign for i32 { fn add_assign(&mut self, rhs: i32) { todo!() } }

impl Add for i64 { type Output = i64; fn add(self, rhs: i64) -> i64 { todo!() } }
impl Sub for i64 { type Output = i64; fn sub(self, rhs: i64) -> i64 { todo!() } }
impl Mul for i64 { type Output = i64; fn mul(self, rhs: i64) -> i64 { todo!() } }
impl Div for i64 { type Output = i64; fn div(self, rhs: i64) -> i64 { todo!() } }
impl Rem for i64 { type Output = i64; fn rem(self, rhs: i64) -> i64 { todo!() } }
impl Neg for i64 { type Output = i64; fn neg(self) -> i64 { todo!() } }
impl AddAssign for i64 { fn add_assign(&mut self, rhs: i64) { todo!() } }

impl Add for f64 { type Output = f64; fn add(self, rhs: f64) -> f64 { todo!() } }
impl Sub for f64 { type Output = f64; fn sub(self, rhs: f64) -> f64 { todo!() } }
impl Mul for f64 { type Output = f64; fn mul(self, rhs: f64) -> f64 { todo!() } }
impl Div for f64 { type Output = f64; fn div(self, rhs: f64) -> f64 { todo!() } }
impl Rem for f64 { type Output = f64; fn rem(self, rhs: f64) -> f64 { todo!() } }
impl Neg for f64 { type Output = f64; fn neg(self) -> f64 { todo!() } }
impl AddAssign for f64 { fn add_assign(&mut self, rhs: f64) { todo!() } }

impl Add for f32 { type Output = f32; fn add(self, rhs: f32) -> f32 { todo!() } }
impl Sub for f32 { type Output = f32; fn sub(self, rhs: f32) -> f32 { todo!() } }
impl Neg for f32 { type Output = f32; fn neg(self) -> f32 { todo!() } }

impl Not for bool { type Output = bool; fn not(self) -> bool { todo!() } }
impl BitAnd for bool { type Output = bool; fn bitand(self, rhs: bool) -> bool { todo!() } }
impl BitOr for bool { type Output = bool; fn bitor(self, rhs: bool) -> bool { todo!() } }

// `String + &str` is the only `Add` on a non-primitive the corpus reaches.
impl Add<&str> for String { type Output = String; fn add(self, rhs: &str) -> String { todo!() } }
impl AddAssign<&str> for String { fn add_assign(&mut self, rhs: &str) { todo!() } }

// ── Index impls ──────────────────────────────────────────────────────────────

// std states these once, generically over `SliceIndex`, and the `Output` comes
// from the index type's projection: `v[0]` is `T` because `usize: SliceIndex<[T],
// Output = T>`, `v[a..b]` is `[T]` because `Range<usize>` projects to `[T]`.
// Writing them out per range type would have to be kept in step with
// `slice.rs`'s `SliceIndex` list by hand.
impl<T, I: SliceIndex<[T]>> std::ops::Index<I> for [T] {
    type Output = <I as SliceIndex<[T]>>::Output;
    fn index(&self, index: I) -> &<I as SliceIndex<[T]>>::Output { todo!() }
}
impl<T, I: SliceIndex<[T]>> std::ops::IndexMut<I> for [T] {
    fn index_mut(&mut self, index: I) -> &mut <I as SliceIndex<[T]>>::Output { todo!() }
}

impl<T, I: SliceIndex<[T]>> std::ops::Index<I> for Vec<T> {
    type Output = <I as SliceIndex<[T]>>::Output;
    fn index(&self, index: I) -> &<I as SliceIndex<[T]>>::Output { todo!() }
}
impl<T, I: SliceIndex<[T]>> std::ops::IndexMut<I> for Vec<T> {
    fn index_mut(&mut self, index: I) -> &mut <I as SliceIndex<[T]>>::Output { todo!() }
}

impl<I: SliceIndex<str>> std::ops::Index<I> for str {
    type Output = <I as SliceIndex<str>>::Output;
    fn index(&self, index: I) -> &<I as SliceIndex<str>>::Output { todo!() }
}
impl<I: SliceIndex<str>> std::ops::IndexMut<I> for str {
    fn index_mut(&mut self, index: I) -> &mut <I as SliceIndex<str>>::Output { todo!() }
}

impl<I: SliceIndex<str>> std::ops::Index<I> for String {
    type Output = <I as SliceIndex<str>>::Output;
    fn index(&self, index: I) -> &<I as SliceIndex<str>>::Output { todo!() }
}
impl<I: SliceIndex<str>> std::ops::IndexMut<I> for String {
    fn index_mut(&mut self, index: I) -> &mut <I as SliceIndex<str>>::Output { todo!() }
}

impl<T, const N: usize, I: SliceIndex<[T]>> std::ops::Index<I> for [T; N] {
    type Output = <I as SliceIndex<[T]>>::Output;
    fn index(&self, index: I) -> &<I as SliceIndex<[T]>>::Output { todo!() }
}

impl<K: Eq + Hash, Q: ?Sized + Eq + Hash, V, S: BuildHasher> Index<&Q> for HashMap<K, V, S> where K: Borrow<Q> {
    type Output = V;
    fn index(&self, key: &Q) -> &V { todo!() }
}

impl<K: Ord, Q: ?Sized + Ord, V> Index<&Q> for BTreeMap<K, V> where K: Borrow<Q> {
    type Output = V;
    fn index(&self, key: &Q) -> &V { todo!() }
}

impl<T> Index<usize> for VecDeque<T> { type Output = T; fn index(&self, index: usize) -> &T { todo!() } }
impl<T> IndexMut<usize> for VecDeque<T> { fn index_mut(&mut self, index: usize) -> &mut T { todo!() } }
