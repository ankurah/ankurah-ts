//! `std::path`
//!
//! Declared because `rusqlite::Connection::open<P: AsRef<Path>>` and
//! `rusqlite::Error::InvalidPath(PathBuf)` name these, and an undeclared name in
//! a signature drops that signature from the table. The port has no file
//! system: nothing here is reachable from transpiled code, and `Path` exists so
//! that the sqlite driver's `open` has a bound to resolve.

pub struct Path;

impl Path {
    pub fn new<S: AsRef<str> + ?Sized>(s: &S) -> &Path { todo!() }
    pub fn to_str(&self) -> Option<&str> { todo!() }
    pub fn to_string_lossy(&self) -> Cow<'_, str> { todo!() }
    pub fn to_path_buf(&self) -> PathBuf { todo!() }
    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf { todo!() }
    pub fn parent(&self) -> Option<&Path> { todo!() }
    pub fn file_name(&self) -> Option<&str> { todo!() }
    pub fn extension(&self) -> Option<&str> { todo!() }
    pub fn display(&self) -> Display<'_> { todo!() }
    pub fn exists(&self) -> bool { todo!() }
    pub fn is_absolute(&self) -> bool { todo!() }
}

impl AsRef<Path> for Path { fn as_ref(&self) -> &Path { todo!() } }
impl AsRef<Path> for str { fn as_ref(&self) -> &Path { todo!() } }
impl AsRef<Path> for String { fn as_ref(&self) -> &Path { todo!() } }
impl ToOwned for Path { type Owned = PathBuf; fn to_owned(&self) -> PathBuf { todo!() } fn clone_into(&self, target: &mut PathBuf) { todo!() } }
impl Debug for Path { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl PartialEq for Path { fn eq(&self, other: &Path) -> bool { todo!() } }
impl Eq for Path {}

pub struct PathBuf;

impl PathBuf {
    pub fn new() -> PathBuf { todo!() }
    pub fn from<S: Into<String>>(s: S) -> PathBuf { todo!() }
    pub fn push<P: AsRef<Path>>(&mut self, path: P) { todo!() }
    pub fn pop(&mut self) -> bool { todo!() }
    pub fn as_path(&self) -> &Path { todo!() }
    pub fn into_os_string(self) -> String { todo!() }
}

impl Deref for PathBuf { type Target = Path; fn deref(&self) -> &Path { todo!() } }
impl AsRef<Path> for PathBuf { fn as_ref(&self) -> &Path { todo!() } }
impl Borrow<Path> for PathBuf { fn borrow(&self) -> &Path { todo!() } }
impl Clone for PathBuf { fn clone(&self) -> PathBuf { todo!() } }
impl Debug for PathBuf { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Default for PathBuf { fn default() -> PathBuf { todo!() } }
impl PartialEq for PathBuf { fn eq(&self, other: &PathBuf) -> bool { todo!() } }
impl Eq for PathBuf {}
impl From<String> for PathBuf { fn from(s: String) -> PathBuf { todo!() } }
impl<'a> From<&'a str> for PathBuf { fn from(s: &'a str) -> PathBuf { todo!() } }

/// `path.display()` returns this, not a `String`; it is `Display` and nothing
/// else, which is why `format!("{}", p.display())` is the idiom.
pub struct Display<'a>;

impl<'a> std::fmt::Display for Display<'a> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<'a> Debug for Display<'a> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
