//! `std::sync::atomic`
//!
//! This module's `Ordering` is not `std::cmp::Ordering`. Both are declared, in
//! their own modules, and the registry keeps same-leaf-name types in different
//! modules apart (spec section 1, the non-flat registry ruling).

pub enum Ordering {
    Relaxed,
    Release,
    Acquire,
    AcqRel,
    SeqCst,
}

impl Clone for Ordering { fn clone(&self) -> Ordering { todo!() } }
impl Copy for Ordering {}
impl Debug for Ordering { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct AtomicBool;

impl AtomicBool {
    pub fn new(v: bool) -> AtomicBool { todo!() }
    pub fn load(&self, order: Ordering) -> bool { todo!() }
    pub fn store(&self, val: bool, order: Ordering) { todo!() }
    pub fn swap(&self, val: bool, order: Ordering) -> bool { todo!() }
    pub fn compare_exchange(&self, current: bool, new: bool, success: Ordering, failure: Ordering) -> Result<bool, bool> { todo!() }
    pub fn compare_exchange_weak(&self, current: bool, new: bool, success: Ordering, failure: Ordering) -> Result<bool, bool> { todo!() }
    pub fn fetch_and(&self, val: bool, order: Ordering) -> bool { todo!() }
    pub fn fetch_or(&self, val: bool, order: Ordering) -> bool { todo!() }
    pub fn fetch_xor(&self, val: bool, order: Ordering) -> bool { todo!() }
    pub fn into_inner(self) -> bool { todo!() }
    pub fn get_mut(&mut self) -> &mut bool { todo!() }
}

impl Default for AtomicBool { fn default() -> AtomicBool { todo!() } }
impl Debug for AtomicBool { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl From<bool> for AtomicBool { fn from(v: bool) -> AtomicBool { todo!() } }

pub struct AtomicUsize;

impl AtomicUsize {
    pub fn new(v: usize) -> AtomicUsize { todo!() }
    pub fn load(&self, order: Ordering) -> usize { todo!() }
    pub fn store(&self, val: usize, order: Ordering) { todo!() }
    pub fn swap(&self, val: usize, order: Ordering) -> usize { todo!() }
    pub fn fetch_add(&self, val: usize, order: Ordering) -> usize { todo!() }
    pub fn fetch_sub(&self, val: usize, order: Ordering) -> usize { todo!() }
    pub fn fetch_max(&self, val: usize, order: Ordering) -> usize { todo!() }
    pub fn fetch_min(&self, val: usize, order: Ordering) -> usize { todo!() }
    pub fn compare_exchange(&self, current: usize, new: usize, success: Ordering, failure: Ordering) -> Result<usize, usize> { todo!() }
    pub fn compare_exchange_weak(&self, current: usize, new: usize, success: Ordering, failure: Ordering) -> Result<usize, usize> { todo!() }
    pub fn into_inner(self) -> usize { todo!() }
    pub fn get_mut(&mut self) -> &mut usize { todo!() }
}

impl Default for AtomicUsize { fn default() -> AtomicUsize { todo!() } }
impl Debug for AtomicUsize { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct AtomicU32;

impl AtomicU32 {
    pub fn new(v: u32) -> AtomicU32 { todo!() }
    pub fn load(&self, order: Ordering) -> u32 { todo!() }
    pub fn store(&self, val: u32, order: Ordering) { todo!() }
    pub fn swap(&self, val: u32, order: Ordering) -> u32 { todo!() }
    pub fn fetch_add(&self, val: u32, order: Ordering) -> u32 { todo!() }
    pub fn fetch_sub(&self, val: u32, order: Ordering) -> u32 { todo!() }
    pub fn compare_exchange(&self, current: u32, new: u32, success: Ordering, failure: Ordering) -> Result<u32, u32> { todo!() }
    pub fn into_inner(self) -> u32 { todo!() }
}

impl Default for AtomicU32 { fn default() -> AtomicU32 { todo!() } }
impl Debug for AtomicU32 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct AtomicU64;

impl AtomicU64 {
    pub fn new(v: u64) -> AtomicU64 { todo!() }
    pub fn load(&self, order: Ordering) -> u64 { todo!() }
    pub fn store(&self, val: u64, order: Ordering) { todo!() }
    pub fn swap(&self, val: u64, order: Ordering) -> u64 { todo!() }
    pub fn fetch_add(&self, val: u64, order: Ordering) -> u64 { todo!() }
    pub fn fetch_sub(&self, val: u64, order: Ordering) -> u64 { todo!() }
    pub fn compare_exchange(&self, current: u64, new: u64, success: Ordering, failure: Ordering) -> Result<u64, u64> { todo!() }
    pub fn into_inner(self) -> u64 { todo!() }
}

impl Default for AtomicU64 { fn default() -> AtomicU64 { todo!() } }
impl Debug for AtomicU64 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct AtomicI64;

impl AtomicI64 {
    pub fn new(v: i64) -> AtomicI64 { todo!() }
    pub fn load(&self, order: Ordering) -> i64 { todo!() }
    pub fn store(&self, val: i64, order: Ordering) { todo!() }
    pub fn fetch_add(&self, val: i64, order: Ordering) -> i64 { todo!() }
    pub fn fetch_sub(&self, val: i64, order: Ordering) -> i64 { todo!() }
    pub fn into_inner(self) -> i64 { todo!() }
}

impl Default for AtomicI64 { fn default() -> AtomicI64 { todo!() } }
impl Debug for AtomicI64 { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
