//! Operators, once the operands are named.
//!
//! JavaScript's operators are not Rust's: `==` on two objects compares
//! identity, integer division leaves a fraction behind, `!` on a number is a
//! boolean, and a `bigint` beside a `number` throws rather than adding. Each of
//! those is a wrong answer the emitted code used to give silently.

#[derive(Clone, PartialEq)]
pub struct Tag {
    pub id: u32,
}

/// `==` between two values of a type that derives `PartialEq` is the `equals`
/// the derive emitted, not a reference comparison.
pub fn same(a: &Tag, b: &Tag) -> bool {
    a == b
}

pub fn different(a: &Tag, b: &Tag) -> bool {
    a != b
}

/// Integer division truncates towards zero in Rust and does not in JavaScript.
pub fn halves(n: u32) -> u32 {
    n / 2
}

/// `!` on an integer flips its bits; JavaScript spells that `~`.
pub fn flipped(bits: u32) -> u32 {
    !bits
}

/// `!` on a boolean is the negation both languages spell the same way.
pub fn negated(yes: bool) -> bool {
    !yes
}

/// 64-bit arithmetic is `bigint` arithmetic, and a literal written against one
/// has to be a `bigint` too.
pub fn shifted(bits: u64) -> u64 {
    bits ^ (1 << 63)
}

/// Comparison and arithmetic on ordinary numbers stay what they were.
pub fn bigger(a: u32, b: u32) -> bool {
    a > b
}

/// An overloaded operator is a method call, and Rust's operator traits take
/// both operands by value: `a + b` releases both, so the block that held them
/// must not release them again — and what the call answers is what its impl's
/// `Output` says, so the local it is bound to is released like any other.
pub struct Weight {
    pub label: String,
    pub grams: u64,
}

impl std::ops::Add for Weight {
    type Output = Weight;
    fn add(self, rhs: Weight) -> Weight {
        Weight { label: self.label, grams: self.grams + rhs.grams }
    }
}

pub fn combined(a: Weight, b: Weight) -> Weight {
    a + b
}

/// The result is a value of its own: the block owns it and releases it.
pub fn heavier(a: Weight, b: Weight) -> bool {
    let total = a + b;
    total.grams > 100
}

/// An impl written for REFERENCES is an impl of its own: Rust looks operator
/// impls up on the exact operand types, never through a reference and never
/// through `Deref`. Looking this one up with the references peeled off missed
/// it, and what stood was the JavaScript `+` between two objects.
pub struct Left {
    pub grams: u64,
}

pub struct Right {
    pub grams: u64,
}

impl std::ops::Add<&Right> for &Left {
    type Output = u64;
    fn add(self, rhs: &Right) -> u64 {
        self.grams + rhs.grams
    }
}

pub fn borrowed_sum(a: &Left, b: &Right) -> u64 {
    a + b
}

/// A heterogeneous `Rhs` whose type is only known from a LATER local. The move
/// scan runs before that local has a type, and guessing `Rhs = Self` found no
/// impl at all — so nothing marked the left operand moved, `add` consumed it,
/// and the block released it again.
pub struct Parcel {
    pub grams: u64,
}

impl std::ops::Add<Right> for Parcel {
    type Output = u64;
    fn add(self, rhs: Right) -> u64 {
        self.grams + rhs.grams
    }
}

pub fn later_local(parcel: Parcel) -> u64 {
    let right = Right { grams: 2 };
    parcel + right
}

/// A generic impl says what it answers in terms of its own parameters, and the
/// match that selected it says which ones this site has. Refusing every generic
/// impl left the result local untyped, so nothing released what it held.
pub struct Boxed<T> {
    pub value: T,
}

impl<T> std::ops::Add for Boxed<T> {
    type Output = Boxed<T>;
    fn add(self, rhs: Boxed<T>) -> Boxed<T> {
        rhs
    }
}

pub fn generic_sum(a: Boxed<u64>, b: Boxed<u64>) -> u64 {
    let result = a + b;
    result.value
}

/// Rust's `&` and `|` on booleans evaluate both operands; `&&` and `||` do not.
/// The two agree in value and differ in what runs, so an operand that records
/// something recorded it in Rust and did not here.
pub fn eager_and(flag: bool, seen: &mut Vec<u32>) -> bool {
    flag & note(seen)
}

pub fn eager_or(flag: bool, seen: &mut Vec<u32>) -> bool {
    flag | note(seen)
}

fn note(seen: &mut Vec<u32>) -> bool {
    seen.push(1);
    true
}

/// Rust's float-to-integer `as` SATURATES at the target's bounds and answers 0
/// for a NaN — including into `u64` and `i64`, where the port truncated and
/// then kept the low bits, and where `BigInt(NaN)` threw outright.
pub fn to_u64(f: f64) -> u64 {
    f as u64
}

pub fn to_i64(f: f64) -> i64 {
    f as i64
}

/// Every `f32` destination rounds to single precision, whatever the source is.
pub fn to_f32(v: u64) -> f32 {
    v as f32
}

/// A compound bit operation is the operation and then the assignment, and its
/// result needs the same wrapping the expression form gets.
pub fn shift_assign_32(mut value: u32) -> u32 {
    value <<= 31;
    value
}

pub fn shift_assign_8(mut value: u8) -> u8 {
    value <<= 7;
    value
}

/// A `<<` on a bigint grows without bound where Rust keeps the low bits.
pub fn shift_64(value: u64) -> u64 {
    value << 1
}

/// A bigint shift beside a literal the engine typed one way and the literal
/// emitter wrote the other: inside a tuple this threw `Cannot mix BigInt and
/// other types`.
pub fn shifts(a: u32, b: u8, c: u64) -> (u32, u8, u64) {
    (a << 31, b << 4, c << 40)
}

/// The unary operators and indexing resolve through their impls exactly as the
/// binary ones do. `-object` is `NaN` and `object[0]` is `undefined`, and the
/// port used to write both without a word.
pub struct Charge {
    pub amount: i32,
}

impl std::ops::Neg for Charge {
    type Output = Charge;
    fn neg(self) -> Charge {
        Charge { amount: -self.amount }
    }
}

impl std::ops::Not for Charge {
    type Output = Charge;
    fn not(self) -> Charge {
        Charge { amount: !self.amount }
    }
}

impl std::ops::Index<usize> for Charge {
    type Output = i32;
    fn index(&self, _at: usize) -> &i32 {
        &self.amount
    }
}

pub fn charge_negated(c: Charge) -> Charge {
    -c
}

pub fn complemented(c: Charge) -> Charge {
    !c
}

pub fn indexed(c: &Charge) -> i32 {
    c[0]
}

/// The four explicit families Rust offers for saying what should happen instead
/// of the debug build's panic. A JavaScript number has none of these as
/// methods, so `x.wrapping_add(1)` was a `TypeError` at the call; each is a free
/// helper in `@ankurah/base` that takes the width, because `u8` and `usize` are
/// both `number` here and the answers differ.
pub fn wraps(a: u8, b: u8) -> u8 {
    a.wrapping_add(b)
}

pub fn saturates(a: u8, b: u8) -> u8 {
    a.saturating_add(b)
}

pub fn checks(a: u8, b: u8) -> Option<u8> {
    a.checked_mul(b)
}

pub fn overflows(a: u8, b: u8) -> (u8, bool) {
    a.overflowing_add(b)
}

/// A unary literal operand keeps the operator's primitive, so the division is
/// Rust's truncating one and `i32::MIN / -1` panics as Rust's does.
pub fn divided_by_negative_one(x: i32) -> i32 {
    x / -1
}

/// R7 reaches through the cell a `&mut` to a JavaScript value becomes.
pub fn bump(n: &mut u32) {
    *n += 1;
}

// The four explicit arithmetic families are free helpers taking the WIDTH, and
// which width a receiver has is the only thing that says which. A `u64` and an
// `i64` are `bigint` here, and a `bigint` has no `saturatingAdd` of its own:
// written as a method, storage-indexeddb's `next_upper_bound` raised on every
// I64-keyed index range. `isize` is a `number` here and had the same defect.
pub fn saturate_u64(v: u64) -> u64 {
    v.saturating_add(1)
}

pub fn wrap_i64(v: i64) -> i64 {
    v.wrapping_sub(1)
}

pub fn saturate_isize(v: isize) -> isize {
    v.saturating_add(1)
}

pub fn wrap_u128(v: u128) -> u128 {
    v.wrapping_add(1)
}

/// `Math.min` converts its arguments to numbers, and converting a `bigint`
/// throws. A 64-bit width takes the comparison written out.
pub fn smaller(a: u64, b: u64) -> u64 {
    a.min(b)
}

pub fn magnitude(v: i64) -> i64 {
    v.abs()
}
