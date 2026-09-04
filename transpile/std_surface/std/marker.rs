//! `std::marker`
//!
//! `Send`, `Sync` and `Unpin` are declared `auto`, as std declares them.
//! Rust derives an auto trait structurally — a type has it when every field
//! has it — and no `impl` list can express that. The engine's rule, stated in
//! the README, is that an auto-trait bound is always satisfied: the corpus
//! compiles under rustc, so every auto-trait bound in it already holds.

pub unsafe auto trait Send {}

pub unsafe auto trait Sync {}

pub auto trait Unpin {}

/// `Sized` is deliberately *not* declared `auto`. It is a lang item, not an
/// auto trait, and writing `auto` here would be the same kind of fabrication
/// this surface exists to avoid. The engine treats a `Sized` bound as
/// satisfied for the same reason it treats an auto-trait bound as satisfied —
/// the corpus compiles — and the README says so alongside the auto traits.
pub trait Sized {}

pub trait Copy: Clone {}

pub struct PhantomData<T: ?Sized>;

impl<T: ?Sized> Clone for PhantomData<T> {
    fn clone(&self) -> PhantomData<T> { todo!() }
}

impl<T: ?Sized> Copy for PhantomData<T> {}

impl<T: ?Sized> Default for PhantomData<T> {
    fn default() -> PhantomData<T> { todo!() }
}

impl<T: ?Sized> PartialEq for PhantomData<T> {
    fn eq(&self, other: &PhantomData<T>) -> bool { todo!() }
}

impl<T: ?Sized> Eq for PhantomData<T> {}

impl<T: ?Sized> Debug for PhantomData<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() }
}

/// `Unsize` is how `[T; N]` reaches `[T]` and how `&T` reaches `&dyn Trait`:
/// a coercion, not a `Deref` step, which the oracle records as
/// `Pointer(Unsize)`. Unstable in real std, and declared here rather than in
/// `std/slice.rs` because `std::marker` is its module. The slice impl is in
/// `std/slice.rs`, next to the type it concerns.
pub trait Unsize<T: ?Sized> {}

impl<T: Debug> Unsize<dyn Debug> for T {}

/// `Tuple` is the bound `Fn`/`FnMut`/`FnOnce` place on their `Args` parameter.
/// Unstable in real std; declared because without it `FnOnce<u32>` resolves.
pub trait Tuple {}
