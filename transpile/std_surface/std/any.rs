//! `std::any`
//!
//! `core/src/property/backend` keeps backends behind `Arc<dyn Any + Send +
//! Sync>` and recovers the concrete backend with `downcast`; `type_name::<T>()`
//! names collections in three places.

pub trait Any: 'static {
    fn type_id(&self) -> TypeId;
}

impl<T: 'static + ?Sized> Any for T {
    fn type_id(&self) -> TypeId { todo!() }
}

impl dyn Any {
    pub fn is<T: Any>(&self) -> bool { todo!() }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> { todo!() }
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> { todo!() }
}

impl dyn Any + Send {
    pub fn is<T: Any>(&self) -> bool { todo!() }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> { todo!() }
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> { todo!() }
}

impl dyn Any + Send + Sync {
    pub fn is<T: Any>(&self) -> bool { todo!() }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> { todo!() }
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> { todo!() }
}

impl Box<dyn Any> {
    pub fn downcast<T: Any>(self) -> Result<Box<T>, Box<dyn Any>> { todo!() }
}

impl Box<dyn Any + Send> {
    pub fn downcast<T: Any>(self) -> Result<Box<T>, Box<dyn Any + Send>> { todo!() }
}

pub struct TypeId;

impl TypeId {
    pub fn of<T: ?Sized + 'static>() -> TypeId { todo!() }
}

impl Clone for TypeId { fn clone(&self) -> TypeId { todo!() } }
impl Copy for TypeId {}
impl PartialEq for TypeId { fn eq(&self, other: &TypeId) -> bool { todo!() } }
impl Eq for TypeId {}
impl Hash for TypeId { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl Debug for TypeId { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub fn type_name<T: ?Sized>() -> &'static str { todo!() }
pub fn type_name_of_val<T: ?Sized>(val: &T) -> &'static str { todo!() }
