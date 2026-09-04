//! `rand` 0.8.5
//!
//! Not on the deliverable's list. One site: `Node::get_durable_peer_random`
//! picks a peer with `peers.choose(&mut rng)`, which is `SliceRandom` on the
//! slice, not a method on `rng`.

pub trait Rng: RngCore {
    fn gen<T>(&mut self) -> T where Standard: Distribution<T>;
    fn gen_range<T: SampleUniform, R: SampleRange<T>>(&mut self, range: R) -> T;
    fn gen_bool(&mut self, p: f64) -> bool;
    fn fill<T: Fill + ?Sized>(&mut self, dest: &mut T);
}

pub struct Standard;

pub trait Distribution<T> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> T;
}

pub trait SampleUniform: Sized {}
pub trait SampleRange<T> {}
pub trait Fill {}

impl Distribution<u32> for Standard { fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u32 { todo!() } }
impl Distribution<u64> for Standard { fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u64 { todo!() } }
impl Distribution<usize> for Standard { fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> usize { todo!() } }
impl Distribution<i32> for Standard { fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> i32 { todo!() } }
impl Distribution<i64> for Standard { fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> i64 { todo!() } }
impl Distribution<f64> for Standard { fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 { todo!() } }
impl Distribution<bool> for Standard { fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> bool { todo!() } }
impl Distribution<u8> for Standard { fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 { todo!() } }
impl Distribution<u128> for Standard { fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u128 { todo!() } }

impl SampleUniform for u32 {}
impl SampleUniform for u64 {}
impl SampleUniform for usize {}
impl SampleUniform for i32 {}
impl SampleUniform for i64 {}
impl SampleUniform for f64 {}
impl<T: SampleUniform> SampleRange<T> for std::ops::Range<T> {}
impl<T: SampleUniform> SampleRange<T> for RangeInclusive<T> {}
impl Fill for [u8] {}

pub trait RngCore {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    fn fill_bytes(&mut self, dest: &mut [u8]);
}

impl<R: RngCore + ?Sized> Rng for R {}

pub struct ThreadRng;

impl RngCore for ThreadRng {
    fn next_u32(&mut self) -> u32 { todo!() }
    fn next_u64(&mut self) -> u64 { todo!() }
    fn fill_bytes(&mut self, dest: &mut [u8]) { todo!() }
}

pub fn thread_rng() -> ThreadRng { todo!() }
pub fn random<T>() -> T where Standard: Distribution<T> { todo!() }

pub mod seq {
    pub trait SliceRandom {
        type Item;
        fn choose<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<&Self::Item>;
        fn choose_mut<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Option<&mut Self::Item>;
        fn choose_multiple<R: Rng + ?Sized>(&self, rng: &mut R, amount: usize) -> SliceChooseIter<'_, Self, Self::Item>;
        fn shuffle<R: Rng + ?Sized>(&mut self, rng: &mut R);
    }

    pub struct SliceChooseIter<'a, S: ?Sized, T>;

    impl<'a, S: ?Sized, T> Iterator for SliceChooseIter<'a, S, T> {
        type Item = &'a T;
        fn next(&mut self) -> Option<&'a T> { todo!() }
    }

    impl<T> SliceRandom for [T] {
        type Item = T;
        fn choose<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<&T> { todo!() }
        fn choose_mut<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Option<&mut T> { todo!() }
        fn choose_multiple<R: Rng + ?Sized>(&self, rng: &mut R, amount: usize) -> SliceChooseIter<'_, [T], T> { todo!() }
        fn shuffle<R: Rng + ?Sized>(&mut self, rng: &mut R) { todo!() }
    }
}

pub mod prelude {
    pub use super::seq::SliceRandom;
    pub use super::{random, thread_rng, Rng, RngCore, ThreadRng};
}
