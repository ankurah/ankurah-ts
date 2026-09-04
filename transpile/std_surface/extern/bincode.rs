//! `bincode` 1.3.3
//!
//! The wire format is bincode's, matched byte for byte by the port
//! (CLAUDE.md's wire-protocol rule). What the engine needs is the free
//! functions' types: `serialize` yields `Vec<u8>` and `deserialize` is driven
//! entirely by the expected type at the call site (spec 4.6).

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<ErrorKind>;

pub fn serialize<T: ?Sized + Serialize>(value: &T) -> Result<Vec<u8>> { todo!() }
pub fn deserialize<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T> { todo!() }
pub fn serialize_into<W: std::io::Write, T: ?Sized + Serialize>(writer: W, value: &T) -> Result<()> { todo!() }
pub fn serialized_size<T: ?Sized + Serialize>(value: &T) -> Result<u64> { todo!() }

pub enum ErrorKind {
    Io(std::io::Error),
    InvalidUtf8Encoding(Utf8Error),
    InvalidBoolEncoding(u8),
    InvalidCharEncoding,
    InvalidTagEncoding(usize),
    DeserializeAnyNotSupported,
    SizeLimit,
    SequenceMustHaveLength,
    Custom(String),
}

impl Debug for ErrorKind { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for ErrorKind { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for ErrorKind {}
