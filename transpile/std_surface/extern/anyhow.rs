//! `anyhow` 1.0.100
//!
//! The blanket `From<E>` at the bottom is what makes `f()?` compile inside a
//! function returning `anyhow::Result<_>` when `f` returns a domain error. The
//! corpus does this at dozens of sites; spec 4.6 names this impl specifically.
//! `anyhow!` is a macro and out of scope — the macro handler gives its call
//! sites the type `anyhow::Error`.

pub struct Error;

impl Error {
    pub fn new<E: std::error::Error + Send + Sync + 'static>(error: E) -> Error { todo!() }
    pub fn msg<M: Display + Debug + Send + Sync + 'static>(message: M) -> Error { todo!() }
    pub fn context<C: Display + Send + Sync + 'static>(self, context: C) -> Error { todo!() }
    pub fn chain(&self) -> Chain<'_> { todo!() }
    pub fn root_cause(&self) -> &(dyn std::error::Error + 'static) { todo!() }
    pub fn is<E: Display + Debug + Send + Sync + 'static>(&self) -> bool { todo!() }
    pub fn downcast<E: Display + Debug + Send + Sync + 'static>(self) -> Result<E, Error> { todo!() }
    pub fn downcast_ref<E: Display + Debug + Send + Sync + 'static>(&self) -> Option<&E> { todo!() }
}

impl Debug for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

impl Deref for Error {
    type Target = dyn std::error::Error + Send + Sync + 'static;
    fn deref(&self) -> &(dyn std::error::Error + Send + Sync + 'static) { todo!() }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Chain<'a>;

impl<'a> Iterator for Chain<'a> {
    type Item = &'a (dyn std::error::Error + 'static);
    fn next(&mut self) -> Option<&'a (dyn std::error::Error + 'static)> { todo!() }
}

pub trait Context<T, E> {
    fn context<C: Display + Send + Sync + 'static>(self, context: C) -> Result<T, Error>;
    fn with_context<C: Display + Send + Sync + 'static, F: FnOnce() -> C>(self, f: F) -> Result<T, Error>;
}

/// anyhow bounds `Context for Result<T, E>` on its own sealed `ext::StdError`,
/// whose impls cover both ordinary `std::error::Error` types *and*
/// `anyhow::Error` itself. Requiring `E: std::error::Error` instead would
/// reject `Result<T, anyhow::Error>.context(..)`, which is valid and common.
pub trait StdError {}

impl<E: std::error::Error + Send + Sync + 'static> StdError for E {}
impl StdError for Error {}

impl<T, E: StdError + Send + Sync + 'static> Context<T, E> for std::result::Result<T, E> {
    fn context<C: Display + Send + Sync + 'static>(self, context: C) -> Result<T, Error> { todo!() }
    fn with_context<C: Display + Send + Sync + 'static, F: FnOnce() -> C>(self, f: F) -> Result<T, Error> { todo!() }
}

impl<T> Context<T, Infallible> for Option<T> {
    fn context<C: Display + Send + Sync + 'static>(self, context: C) -> Result<T, Error> { todo!() }
    fn with_context<C: Display + Send + Sync + 'static, F: FnOnce() -> C>(self, f: F) -> Result<T, Error> { todo!() }
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for Error {
    fn from(error: E) -> Error { todo!() }
}
