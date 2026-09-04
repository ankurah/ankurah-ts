//! `std::str` — the primitive `str` and its iterators.
//!
//! `Pattern` is unstable in real std but is declared in full, GATs and all,
//! because `split`, `find`, `contains`, `trim_matches` and friends all take one
//! and their return types depend on it. On the pinned nightly `Pattern` has no
//! lifetime parameter — it was GAT-ified in the 1.79/1.80 cycle — so the bound
//! is `P: Pattern` and the searcher is `P::Searcher<'a>`. Reverse operations
//! carry `for<'a> P::Searcher<'a>: ReverseSearcher<'a>`, which is what keeps
//! `s.rsplit(|c| ..)` from resolving against a forward-only pattern.

impl str {
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn as_bytes(&self) -> &[u8] { todo!() }
    pub fn as_ptr(&self) -> *const u8 { todo!() }
    pub fn is_char_boundary(&self, index: usize) -> bool { todo!() }

    pub fn get<I: SliceIndex<str>>(&self, i: I) -> Option<&<I as SliceIndex<str>>::Output> { todo!() }

    pub fn chars(&self) -> Chars<'_> { todo!() }
    pub fn char_indices(&self) -> CharIndices<'_> { todo!() }
    pub fn bytes(&self) -> Bytes<'_> { todo!() }
    pub fn lines(&self) -> Lines<'_> { todo!() }
    pub fn split_whitespace(&self) -> SplitWhitespace<'_> { todo!() }

    pub fn split<P: Pattern>(&self, pat: P) -> Split<'_, P> { todo!() }
    pub fn splitn<P: Pattern>(&self, n: usize, pat: P) -> SplitN<'_, P> { todo!() }
    pub fn rsplit<P: Pattern>(&self, pat: P) -> RSplit<'_, P> where for<'a> <P as Pattern>::Searcher<'a>: ReverseSearcher<'a> { todo!() }
    pub fn rsplitn<P: Pattern>(&self, n: usize, pat: P) -> RSplitN<'_, P> where for<'a> <P as Pattern>::Searcher<'a>: ReverseSearcher<'a> { todo!() }
    pub fn split_terminator<P: Pattern>(&self, pat: P) -> SplitTerminator<'_, P> { todo!() }
    pub fn split_once<P: Pattern>(&self, delimiter: P) -> Option<(&str, &str)> { todo!() }
    pub fn rsplit_once<P: Pattern>(&self, delimiter: P) -> Option<(&str, &str)> where for<'a> <P as Pattern>::Searcher<'a>: ReverseSearcher<'a> { todo!() }
    pub fn matches<P: Pattern>(&self, pat: P) -> Matches<'_, P> { todo!() }

    pub fn contains<P: Pattern>(&self, pat: P) -> bool { todo!() }
    pub fn starts_with<P: Pattern>(&self, pat: P) -> bool { todo!() }
    pub fn ends_with<P: Pattern>(&self, pat: P) -> bool where for<'a> <P as Pattern>::Searcher<'a>: ReverseSearcher<'a> { todo!() }
    pub fn find<P: Pattern>(&self, pat: P) -> Option<usize> { todo!() }
    pub fn rfind<P: Pattern>(&self, pat: P) -> Option<usize> where for<'a> <P as Pattern>::Searcher<'a>: ReverseSearcher<'a> { todo!() }
    pub fn strip_prefix<P: Pattern>(&self, prefix: P) -> Option<&str> { todo!() }
    pub fn strip_suffix<P: Pattern>(&self, suffix: P) -> Option<&str> where for<'a> <P as Pattern>::Searcher<'a>: ReverseSearcher<'a> { todo!() }

    pub fn trim(&self) -> &str { todo!() }
    pub fn trim_start(&self) -> &str { todo!() }
    pub fn trim_end(&self) -> &str { todo!() }
    pub fn trim_matches<P: Pattern>(&self, pat: P) -> &str where for<'a> <P as Pattern>::Searcher<'a>: DoubleEndedSearcher<'a> { todo!() }
    pub fn trim_start_matches<P: Pattern>(&self, pat: P) -> &str { todo!() }
    pub fn trim_end_matches<P: Pattern>(&self, pat: P) -> &str where for<'a> <P as Pattern>::Searcher<'a>: ReverseSearcher<'a> { todo!() }

    pub fn replace<P: Pattern>(&self, from: P, to: &str) -> String { todo!() }
    pub fn replacen<P: Pattern>(&self, pat: P, to: &str, count: usize) -> String { todo!() }
    pub fn to_lowercase(&self) -> String { todo!() }
    pub fn to_uppercase(&self) -> String { todo!() }
    pub fn to_ascii_lowercase(&self) -> String { todo!() }
    pub fn to_ascii_uppercase(&self) -> String { todo!() }
    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool { todo!() }
    pub fn repeat(&self, n: usize) -> String { todo!() }

    pub fn parse<F: FromStr>(&self) -> Result<F, <F as FromStr>::Err> { todo!() }
}

pub trait FromStr: Sized {
    type Err;
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

impl FromStr for bool { type Err = ParseBoolError; fn from_str(s: &str) -> Result<bool, ParseBoolError> { todo!() } }
impl FromStr for char { type Err = std::char::ParseCharError; fn from_str(s: &str) -> Result<char, std::char::ParseCharError> { todo!() } }
impl FromStr for u8 { type Err = std::num::ParseIntError; fn from_str(s: &str) -> Result<u8, std::num::ParseIntError> { todo!() } }
impl FromStr for u16 { type Err = std::num::ParseIntError; fn from_str(s: &str) -> Result<u16, std::num::ParseIntError> { todo!() } }
impl FromStr for u32 { type Err = std::num::ParseIntError; fn from_str(s: &str) -> Result<u32, std::num::ParseIntError> { todo!() } }
impl FromStr for u64 { type Err = std::num::ParseIntError; fn from_str(s: &str) -> Result<u64, std::num::ParseIntError> { todo!() } }
impl FromStr for usize { type Err = std::num::ParseIntError; fn from_str(s: &str) -> Result<usize, std::num::ParseIntError> { todo!() } }
impl FromStr for i16 { type Err = std::num::ParseIntError; fn from_str(s: &str) -> Result<i16, std::num::ParseIntError> { todo!() } }
impl FromStr for i32 { type Err = std::num::ParseIntError; fn from_str(s: &str) -> Result<i32, std::num::ParseIntError> { todo!() } }
impl FromStr for i64 { type Err = std::num::ParseIntError; fn from_str(s: &str) -> Result<i64, std::num::ParseIntError> { todo!() } }
impl FromStr for f32 { type Err = std::num::ParseFloatError; fn from_str(s: &str) -> Result<f32, std::num::ParseFloatError> { todo!() } }
impl FromStr for f64 { type Err = std::num::ParseFloatError; fn from_str(s: &str) -> Result<f64, std::num::ParseFloatError> { todo!() } }
impl FromStr for String { type Err = Infallible; fn from_str(s: &str) -> Result<String, Infallible> { todo!() } }

pub trait Pattern: Sized {
    type Searcher<'a>: Searcher<'a>;

    fn into_searcher(self, haystack: &str) -> Self::Searcher<'_>;
    fn is_contained_in(self, haystack: &str) -> bool;
    fn is_prefix_of(self, haystack: &str) -> bool;
    fn is_suffix_of<'a>(self, haystack: &'a str) -> bool where Self::Searcher<'a>: ReverseSearcher<'a>;
    fn strip_prefix_of(self, haystack: &str) -> Option<&str>;
    fn strip_suffix_of<'a>(self, haystack: &'a str) -> Option<&'a str> where Self::Searcher<'a>: ReverseSearcher<'a>;
}

pub unsafe trait Searcher<'a> {
    fn haystack(&self) -> &'a str;
    fn next(&mut self) -> SearchStep;
    fn next_match(&mut self) -> Option<(usize, usize)>;
    fn next_reject(&mut self) -> Option<(usize, usize)>;
}

pub unsafe trait ReverseSearcher<'a>: Searcher<'a> {
    fn next_back(&mut self) -> SearchStep;
    fn next_match_back(&mut self) -> Option<(usize, usize)>;
    fn next_reject_back(&mut self) -> Option<(usize, usize)>;
}

pub trait DoubleEndedSearcher<'a>: ReverseSearcher<'a> {}

pub enum SearchStep {
    Match(usize, usize),
    Reject(usize, usize),
    Done,
}

pub struct CharSearcher<'a>;
pub struct StrSearcher<'a, 'b>;
pub struct CharSliceSearcher<'a, 'b>;
pub struct CharPredicateSearcher<'a, F>;

unsafe impl<'a> Searcher<'a> for CharSearcher<'a> {
    fn haystack(&self) -> &'a str { todo!() }
    fn next(&mut self) -> SearchStep { todo!() }
    fn next_match(&mut self) -> Option<(usize, usize)> { todo!() }
    fn next_reject(&mut self) -> Option<(usize, usize)> { todo!() }
}
unsafe impl<'a> ReverseSearcher<'a> for CharSearcher<'a> {
    fn next_back(&mut self) -> SearchStep { todo!() }
    fn next_match_back(&mut self) -> Option<(usize, usize)> { todo!() }
    fn next_reject_back(&mut self) -> Option<(usize, usize)> { todo!() }
}
impl<'a> DoubleEndedSearcher<'a> for CharSearcher<'a> {}

unsafe impl<'a, 'b> Searcher<'a> for StrSearcher<'a, 'b> {
    fn haystack(&self) -> &'a str { todo!() }
    fn next(&mut self) -> SearchStep { todo!() }
    fn next_match(&mut self) -> Option<(usize, usize)> { todo!() }
    fn next_reject(&mut self) -> Option<(usize, usize)> { todo!() }
}
unsafe impl<'a, 'b> ReverseSearcher<'a> for StrSearcher<'a, 'b> {
    fn next_back(&mut self) -> SearchStep { todo!() }
    fn next_match_back(&mut self) -> Option<(usize, usize)> { todo!() }
    fn next_reject_back(&mut self) -> Option<(usize, usize)> { todo!() }
}

unsafe impl<'a, 'b> Searcher<'a> for CharSliceSearcher<'a, 'b> {
    fn haystack(&self) -> &'a str { todo!() }
    fn next(&mut self) -> SearchStep { todo!() }
    fn next_match(&mut self) -> Option<(usize, usize)> { todo!() }
    fn next_reject(&mut self) -> Option<(usize, usize)> { todo!() }
}
unsafe impl<'a, 'b> ReverseSearcher<'a> for CharSliceSearcher<'a, 'b> {
    fn next_back(&mut self) -> SearchStep { todo!() }
    fn next_match_back(&mut self) -> Option<(usize, usize)> { todo!() }
    fn next_reject_back(&mut self) -> Option<(usize, usize)> { todo!() }
}
impl<'a, 'b> DoubleEndedSearcher<'a> for CharSliceSearcher<'a, 'b> {}

unsafe impl<'a, F: FnMut(char) -> bool> Searcher<'a> for CharPredicateSearcher<'a, F> {
    fn haystack(&self) -> &'a str { todo!() }
    fn next(&mut self) -> SearchStep { todo!() }
    fn next_match(&mut self) -> Option<(usize, usize)> { todo!() }
    fn next_reject(&mut self) -> Option<(usize, usize)> { todo!() }
}
unsafe impl<'a, F: FnMut(char) -> bool> ReverseSearcher<'a> for CharPredicateSearcher<'a, F> {
    fn next_back(&mut self) -> SearchStep { todo!() }
    fn next_match_back(&mut self) -> Option<(usize, usize)> { todo!() }
    fn next_reject_back(&mut self) -> Option<(usize, usize)> { todo!() }
}
impl<'a, F: FnMut(char) -> bool> DoubleEndedSearcher<'a> for CharPredicateSearcher<'a, F> {}

impl Pattern for char {
    type Searcher<'a> = CharSearcher<'a>;
    fn into_searcher(self, haystack: &str) -> CharSearcher<'_> { todo!() }
    fn is_contained_in(self, haystack: &str) -> bool { todo!() }
    fn is_prefix_of(self, haystack: &str) -> bool { todo!() }
    fn is_suffix_of<'a>(self, haystack: &'a str) -> bool where CharSearcher<'a>: ReverseSearcher<'a> { todo!() }
    fn strip_prefix_of(self, haystack: &str) -> Option<&str> { todo!() }
    fn strip_suffix_of<'a>(self, haystack: &'a str) -> Option<&'a str> where CharSearcher<'a>: ReverseSearcher<'a> { todo!() }
}

impl<'b> Pattern for &'b str {
    type Searcher<'a> = StrSearcher<'a, 'b>;
    fn into_searcher(self, haystack: &str) -> StrSearcher<'_, 'b> { todo!() }
    fn is_contained_in(self, haystack: &str) -> bool { todo!() }
    fn is_prefix_of(self, haystack: &str) -> bool { todo!() }
    fn is_suffix_of<'a>(self, haystack: &'a str) -> bool where StrSearcher<'a, 'b>: ReverseSearcher<'a> { todo!() }
    fn strip_prefix_of(self, haystack: &str) -> Option<&str> { todo!() }
    fn strip_suffix_of<'a>(self, haystack: &'a str) -> Option<&'a str> where StrSearcher<'a, 'b>: ReverseSearcher<'a> { todo!() }
}

impl<'b> Pattern for &'b String {
    type Searcher<'a> = StrSearcher<'a, 'b>;
    fn into_searcher(self, haystack: &str) -> StrSearcher<'_, 'b> { todo!() }
    fn is_contained_in(self, haystack: &str) -> bool { todo!() }
    fn is_prefix_of(self, haystack: &str) -> bool { todo!() }
    fn is_suffix_of<'a>(self, haystack: &'a str) -> bool where StrSearcher<'a, 'b>: ReverseSearcher<'a> { todo!() }
    fn strip_prefix_of(self, haystack: &str) -> Option<&str> { todo!() }
    fn strip_suffix_of<'a>(self, haystack: &'a str) -> Option<&'a str> where StrSearcher<'a, 'b>: ReverseSearcher<'a> { todo!() }
}

impl<'b> Pattern for &'b [char] {
    type Searcher<'a> = CharSliceSearcher<'a, 'b>;
    fn into_searcher(self, haystack: &str) -> CharSliceSearcher<'_, 'b> { todo!() }
    fn is_contained_in(self, haystack: &str) -> bool { todo!() }
    fn is_prefix_of(self, haystack: &str) -> bool { todo!() }
    fn is_suffix_of<'a>(self, haystack: &'a str) -> bool where CharSliceSearcher<'a, 'b>: ReverseSearcher<'a> { todo!() }
    fn strip_prefix_of(self, haystack: &str) -> Option<&str> { todo!() }
    fn strip_suffix_of<'a>(self, haystack: &'a str) -> Option<&'a str> where CharSliceSearcher<'a, 'b>: ReverseSearcher<'a> { todo!() }
}

impl<F: FnMut(char) -> bool> Pattern for F {
    type Searcher<'a> = CharPredicateSearcher<'a, F>;
    fn into_searcher(self, haystack: &str) -> CharPredicateSearcher<'_, F> { todo!() }
    fn is_contained_in(self, haystack: &str) -> bool { todo!() }
    fn is_prefix_of(self, haystack: &str) -> bool { todo!() }
    fn is_suffix_of<'a>(self, haystack: &'a str) -> bool where CharPredicateSearcher<'a, F>: ReverseSearcher<'a> { todo!() }
    fn strip_prefix_of(self, haystack: &str) -> Option<&str> { todo!() }
    fn strip_suffix_of<'a>(self, haystack: &'a str) -> Option<&'a str> where CharPredicateSearcher<'a, F>: ReverseSearcher<'a> { todo!() }
}

// `impl<const N: usize> Pattern for [char; N]` and its `CharArraySearcher` are
// omitted. std has them, but the corpus never writes a `[char; N]` pattern —
// it uses `&str`, `char` and closures — and the impl was the only place a const
// generic flowed from an impl's parameter list into an associated type, which
// the loader could not carry. `&[char]` patterns are still declared, through
// `CharSliceSearcher`. Restore both if a future ankurah version writes
// `s.split(['a', 'b'])`.

pub struct Chars<'a>;
pub struct CharIndices<'a>;
pub struct Bytes<'a>;
pub struct Lines<'a>;
pub struct SplitWhitespace<'a>;
pub struct Split<'a, P>;
pub struct SplitN<'a, P>;
pub struct RSplit<'a, P>;
pub struct RSplitN<'a, P>;
pub struct SplitTerminator<'a, P>;
pub struct Matches<'a, P>;
pub struct Utf8Error;
/// `"maybe".parse::<bool>()` yields this. Referenced by `impl FromStr for bool`
/// above and previously never declared, which dropped that impl's signature.
pub struct ParseBoolError;

impl<'a> Iterator for Chars<'a> { type Item = char; fn next(&mut self) -> Option<char> { todo!() } }
impl<'a> DoubleEndedIterator for Chars<'a> { fn next_back(&mut self) -> Option<char> { todo!() } }
impl<'a> Chars<'a> { pub fn as_str(&self) -> &'a str { todo!() } }
impl<'a> Clone for Chars<'a> { fn clone(&self) -> Chars<'a> { todo!() } }

impl<'a> Iterator for CharIndices<'a> { type Item = (usize, char); fn next(&mut self) -> Option<(usize, char)> { todo!() } }
impl<'a> Iterator for Bytes<'a> { type Item = u8; fn next(&mut self) -> Option<u8> { todo!() } }
impl<'a> ExactSizeIterator for Bytes<'a> { fn len(&self) -> usize { todo!() } }
impl<'a> Iterator for Lines<'a> { type Item = &'a str; fn next(&mut self) -> Option<&'a str> { todo!() } }
impl<'a> Iterator for SplitWhitespace<'a> { type Item = &'a str; fn next(&mut self) -> Option<&'a str> { todo!() } }
impl<'a, P: Pattern> Iterator for Split<'a, P> { type Item = &'a str; fn next(&mut self) -> Option<&'a str> { todo!() } }
impl<'a, P: Pattern> Iterator for SplitN<'a, P> { type Item = &'a str; fn next(&mut self) -> Option<&'a str> { todo!() } }
impl<'a, P: Pattern> Iterator for RSplit<'a, P> where for<'b> <P as Pattern>::Searcher<'b>: ReverseSearcher<'b> { type Item = &'a str; fn next(&mut self) -> Option<&'a str> { todo!() } }
impl<'a, P: Pattern> Iterator for RSplitN<'a, P> where for<'b> <P as Pattern>::Searcher<'b>: ReverseSearcher<'b> { type Item = &'a str; fn next(&mut self) -> Option<&'a str> { todo!() } }
impl<'a, P: Pattern> Iterator for SplitTerminator<'a, P> { type Item = &'a str; fn next(&mut self) -> Option<&'a str> { todo!() } }
impl<'a, P: Pattern> Iterator for Matches<'a, P> { type Item = &'a str; fn next(&mut self) -> Option<&'a str> { todo!() } }

impl Debug for str { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for str { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl PartialEq for str { fn eq(&self, other: &str) -> bool { todo!() } }
impl Eq for str {}
impl PartialOrd for str { fn partial_cmp(&self, other: &str) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for str { fn cmp(&self, other: &str) -> std::cmp::Ordering { todo!() } }

pub fn from_utf8(v: &[u8]) -> Result<&str, Utf8Error> { todo!() }

impl Debug for ParseBoolError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for ParseBoolError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for ParseBoolError { fn clone(&self) -> ParseBoolError { todo!() } }
impl PartialEq for ParseBoolError { fn eq(&self, other: &ParseBoolError) -> bool { todo!() } }
impl Eq for ParseBoolError {}
impl std::error::Error for ParseBoolError {}

impl Debug for Utf8Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for Utf8Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for Utf8Error {}
