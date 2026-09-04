//! `std::string`
//!
//! `ToString` and its blanket impl live here, as in real std. 330 `.to_string()`
//! calls in the corpus resolve through that one blanket.

pub struct String;

impl String {
    pub fn new() -> String { todo!() }
    pub fn with_capacity(capacity: usize) -> String { todo!() }
    pub fn from_utf8(vec: Vec<u8>) -> Result<String, FromUtf8Error> { todo!() }
    pub fn from_utf8_lossy(v: &[u8]) -> Cow<'_, str> { todo!() }

    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn capacity(&self) -> usize { todo!() }
    pub fn reserve(&mut self, additional: usize) { todo!() }

    pub fn as_str(&self) -> &str { todo!() }
    pub fn as_mut_str(&mut self) -> &mut str { todo!() }
    pub fn as_bytes(&self) -> &[u8] { todo!() }
    pub fn into_bytes(self) -> Vec<u8> { todo!() }
    pub fn into_boxed_str(self) -> Box<str> { todo!() }

    pub fn push(&mut self, ch: char) { todo!() }
    pub fn push_str(&mut self, string: &str) { todo!() }
    pub fn pop(&mut self) -> Option<char> { todo!() }
    pub fn insert(&mut self, idx: usize, ch: char) { todo!() }
    pub fn insert_str(&mut self, idx: usize, string: &str) { todo!() }
    pub fn remove(&mut self, idx: usize) -> char { todo!() }
    pub fn truncate(&mut self, new_len: usize) { todo!() }
    pub fn clear(&mut self) { todo!() }
    pub fn split_off(&mut self, at: usize) -> String { todo!() }
    pub fn retain<F: FnMut(char) -> bool>(&mut self, f: F) { todo!() }
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_> { todo!() }
}

impl Deref for String {
    type Target = str;
    fn deref(&self) -> &str { todo!() }
}

impl DerefMut for String {
    fn deref_mut(&mut self) -> &mut str { todo!() }
}

impl Clone for String { fn clone(&self) -> String { todo!() } }
impl Default for String { fn default() -> String { todo!() } }
impl Debug for String { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for String { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl PartialEq for String { fn eq(&self, other: &String) -> bool { todo!() } }
impl Eq for String {}
impl PartialEq<str> for String { fn eq(&self, other: &str) -> bool { todo!() } }
impl PartialEq<&str> for String { fn eq(&self, other: &&str) -> bool { todo!() } }
impl PartialEq<String> for str { fn eq(&self, other: &String) -> bool { todo!() } }
impl PartialEq<String> for &str { fn eq(&self, other: &String) -> bool { todo!() } }
impl PartialOrd for String { fn partial_cmp(&self, other: &String) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for String { fn cmp(&self, other: &String) -> std::cmp::Ordering { todo!() } }

impl FromIterator<char> for String { fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> String { todo!() } }
impl<'a> FromIterator<&'a str> for String { fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> String { todo!() } }
impl<'a> FromIterator<&'a char> for String { fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> String { todo!() } }
impl FromIterator<String> for String { fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> String { todo!() } }
impl Extend<char> for String { fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) { todo!() } }
impl<'a> Extend<&'a str> for String { fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) { todo!() } }

pub trait ToString {
    fn to_string(&self) -> String;
}

impl<T: std::fmt::Display + ?Sized> ToString for T {
    fn to_string(&self) -> String { todo!() }
}

pub struct FromUtf8Error;

impl FromUtf8Error {
    pub fn into_bytes(self) -> Vec<u8> { todo!() }
    pub fn utf8_error(&self) -> Utf8Error { todo!() }
}

impl Debug for FromUtf8Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for FromUtf8Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for FromUtf8Error {}

pub struct Drain<'a>;
impl<'a> Iterator for Drain<'a> { type Item = char; fn next(&mut self) -> Option<char> { todo!() } }

// `write!(s, ..)` into a String goes through `fmt::Write`, not `io::Write`.
impl std::fmt::Write for String {
    fn write_str(&mut self, s: &str) -> std::fmt::Result { todo!() }
    fn write_char(&mut self, c: char) -> std::fmt::Result { todo!() }
}
