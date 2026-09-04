//! `std::io`
//!
//! Only what the corpus reaches: `Error::other` wraps a domain error as an
//! `io::Error` at the storage and node boundaries (17 sites, 11 of them in the
//! sqlite engine), and `io::Write` is the trait `proto`'s encoders write into.
//! Nothing here is a file-system operation; the port has no file system.

pub struct Error;

impl Error {
    pub fn new<E: Into<Box<dyn std::error::Error + Send + Sync>>>(kind: ErrorKind, error: E) -> Error { todo!() }
    pub fn other<E: Into<Box<dyn std::error::Error + Send + Sync>>>(error: E) -> Error { todo!() }
    pub fn kind(&self) -> ErrorKind { todo!() }
    pub fn into_inner(self) -> Option<Box<dyn std::error::Error + Send + Sync>> { todo!() }
    pub fn get_ref(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> { todo!() }
}

impl Debug for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for Error {}

pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    InvalidInput,
    InvalidData,
    UnexpectedEof,
    Other,
}

impl Clone for ErrorKind { fn clone(&self) -> ErrorKind { todo!() } }
impl Copy for ErrorKind {}
impl Debug for ErrorKind { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl PartialEq for ErrorKind { fn eq(&self, other: &ErrorKind) -> bool { todo!() } }
impl Eq for ErrorKind {}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;
    fn write_all(&mut self, buf: &[u8]) -> Result<()>;
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> Result<()>;
}

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize>;
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()>;
}

impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> { todo!() }
    fn flush(&mut self) -> Result<()> { todo!() }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> { todo!() }
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> Result<()> { todo!() }
}

impl<W: Write + ?Sized> Write for &mut W {
    fn write(&mut self, buf: &[u8]) -> Result<usize> { todo!() }
    fn flush(&mut self) -> Result<()> { todo!() }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> { todo!() }
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> Result<()> { todo!() }
}
