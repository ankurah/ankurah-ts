//! `std::mem`
//!
//! `MaybeUninit` is here for `signals/src/porcelain`'s deferred initialization,
//! which calls `assume_init_ref`, `assume_init_read` and `assume_init_drop` —
//! all `unsafe fn`, and written as such.

pub fn swap<T>(x: &mut T, y: &mut T) { todo!() }
pub fn replace<T>(dest: &mut T, src: T) -> T { todo!() }
pub fn take<T: Default>(dest: &mut T) -> T { todo!() }
pub fn drop<T>(_x: T) { todo!() }
pub fn forget<T>(t: T) { todo!() }
pub fn size_of<T>() -> usize { todo!() }
pub fn size_of_val<T: ?Sized>(val: &T) -> usize { todo!() }
pub fn align_of<T>() -> usize { todo!() }
pub unsafe fn transmute<Src, Dst>(src: Src) -> Dst { todo!() }
pub fn discriminant<T>(v: &T) -> Discriminant<T> { todo!() }

pub struct Discriminant<T>;

impl<T> PartialEq for Discriminant<T> { fn eq(&self, other: &Discriminant<T>) -> bool { todo!() } }
impl<T> Eq for Discriminant<T> {}
impl<T> Hash for Discriminant<T> { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T> Clone for Discriminant<T> { fn clone(&self) -> Discriminant<T> { todo!() } }
impl<T> Debug for Discriminant<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct MaybeUninit<T>;

impl<T> MaybeUninit<T> {
    pub fn new(val: T) -> MaybeUninit<T> { todo!() }
    pub fn uninit() -> MaybeUninit<T> { todo!() }
    pub fn zeroed() -> MaybeUninit<T> { todo!() }
    pub fn write(&mut self, val: T) -> &mut T { todo!() }
    pub fn as_ptr(&self) -> *const T { todo!() }
    pub fn as_mut_ptr(&mut self) -> *mut T { todo!() }
    pub unsafe fn assume_init(self) -> T { todo!() }
    pub unsafe fn assume_init_ref(&self) -> &T { todo!() }
    pub unsafe fn assume_init_mut(&mut self) -> &mut T { todo!() }
    pub unsafe fn assume_init_read(&self) -> T { todo!() }
    pub unsafe fn assume_init_drop(&mut self) { todo!() }
}
