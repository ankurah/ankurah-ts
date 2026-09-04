//! `std::hash`

pub trait Hash {
    fn hash<H: Hasher>(&self, state: &mut H);
    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H) where Self: Sized;
}

pub trait Hasher {
    fn finish(&self) -> u64;
    fn write(&mut self, bytes: &[u8]);
    fn write_u8(&mut self, i: u8);
    fn write_u32(&mut self, i: u32);
    fn write_u64(&mut self, i: u64);
    fn write_usize(&mut self, i: usize);
    fn write_i32(&mut self, i: i32);
    fn write_i64(&mut self, i: i64);
}

pub trait BuildHasher {
    type Hasher: Hasher;
    fn build_hasher(&self) -> Self::Hasher;
}

pub struct RandomState;

impl BuildHasher for RandomState {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> DefaultHasher { todo!() }
}

impl Clone for RandomState { fn clone(&self) -> RandomState { todo!() } }
impl Default for RandomState { fn default() -> RandomState { todo!() } }

pub struct DefaultHasher;

impl DefaultHasher {
    pub fn new() -> DefaultHasher { todo!() }
}

impl Hasher for DefaultHasher {
    fn finish(&self) -> u64 { todo!() }
    fn write(&mut self, bytes: &[u8]) { todo!() }
    fn write_u8(&mut self, i: u8) { todo!() }
    fn write_u32(&mut self, i: u32) { todo!() }
    fn write_u64(&mut self, i: u64) { todo!() }
    fn write_usize(&mut self, i: usize) { todo!() }
    fn write_i32(&mut self, i: i32) { todo!() }
    fn write_i64(&mut self, i: i64) { todo!() }
}

impl<T: Hash + ?Sized> Hash for &T { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for str { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for String { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T: Hash> Hash for [T] { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T: Hash> Hash for Vec<T> { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T: Hash> Hash for Option<T> { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for bool { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for char { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for u8 { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for u16 { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for u32 { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for u64 { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for usize { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for i16 { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for i32 { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for i64 { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Hash for isize { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
