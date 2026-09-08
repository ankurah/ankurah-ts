// The cursor representation carried into a struct FIELD.
//
// A body that takes `I: Iterator<Item = V>` is handed a cursor, and the port
// has said so at a parameter for several slices. A FIELD declared with the same
// bounded parameter holds the same thing, and nothing said so: the class
// declared `walk: I`, which promises only what the bound promises, and the
// construction site handed over the array it had. `pull()` then read
// `this.walk.next()` — a method an array has not got.
//
// What tells the field apart from any other is the DECLARATION's bound on the
// parameter, not the field's own type, which is just `I`.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub struct Holder<I: Iterator<Item = Token>> {
    pub walk: I,
}

impl<I: Iterator<Item = Token>> Holder<I> {
    /// One element out of the walk the holder was built around.
    pub fn pull(&mut self) -> Option<Token> {
        self.walk.next()
    }
}

/// The construction site: the sequence is wrapped where it crosses into the
/// field, exactly as it is where it crosses into a parameter.
pub fn stored(tokens: Vec<Token>) -> Option<Token> {
    let mut holder: Holder<std::vec::IntoIter<Token>> = Holder { walk: tokens.into_iter() };
    holder.pull()
}

/// A second field beside the walk, so that the question is asked per field and
/// not per struct.
pub struct Pair<I: Iterator<Item = Token>> {
    pub walk: I,
    pub n: i64,
}

pub fn paired(tokens: Vec<Token>, n: i64) -> i64 {
    let pair: Pair<std::vec::IntoIter<Token>> = Pair { walk: tokens.into_iter(), n };
    pair.n
}
