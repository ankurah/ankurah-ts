//! `std::error`

pub trait Error: Debug + Display {
    fn source(&self) -> Option<&(dyn Error + 'static)>;
    fn description(&self) -> &str;
    fn cause(&self) -> Option<&dyn Error>;
}

impl dyn Error + 'static {
    pub fn is<T: Error + 'static>(&self) -> bool { todo!() }
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> { todo!() }
    pub fn downcast_mut<T: Error + 'static>(&mut self) -> Option<&mut T> { todo!() }
}

impl dyn Error + Send + Sync + 'static {
    pub fn is<T: Error + 'static>(&self) -> bool { todo!() }
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> { todo!() }
}

impl<T: Error> Error for Box<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> { todo!() }
    fn description(&self) -> &str { todo!() }
    fn cause(&self) -> Option<&dyn Error> { todo!() }
}
