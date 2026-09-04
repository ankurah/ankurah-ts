//! `pest` 2.8.4 — only the parse-tree walk `ankql` performs.
//!
//! `ankql/src/parser.rs` and `ankql/src/grammar.rs` are in `transpile.toml`'s
//! `[hardcode]` list, so no pest call site is transpiled today. The surface is
//! declared anyway: `Rule` and `Pair` leak into `ParseError::UnexpectedRule`
//! in `ankql/src/error.rs`, which is transpiled, and the hardcoded files stop
//! being hardcoded the moment someone writes the parser translation.

pub trait Parser<R: RuleType> {
    fn parse(rule: R, input: &str) -> Result<iterators::Pairs<'_, R>, error::Error<R>>;
}

pub trait RuleType: Copy + Debug + Eq + Hash + Ord {}

// `pest_derive` generates an ordinary `enum Rule`; without this blanket that
// enum satisfies nothing and every `Pair<Rule>` in ankql is unusable.
impl<T: Copy + Debug + Eq + Hash + Ord> RuleType for T {}

pub mod iterators {
    pub struct Pair<'i, R>;
    pub struct Pairs<'i, R>;

    impl<'i, R: RuleType> Pair<'i, R> {
        pub fn as_rule(&self) -> R { todo!() }
        pub fn as_str(&self) -> &'i str { todo!() }
        pub fn as_span(&self) -> Span<'i> { todo!() }
        pub fn into_inner(self) -> Pairs<'i, R> { todo!() }
        pub fn line_col(&self) -> (usize, usize) { todo!() }
        pub fn tokens(self) -> Tokens<'i, R> { todo!() }
    }

    impl<'i, R: Clone> Clone for Pair<'i, R> { fn clone(&self) -> Pair<'i, R> { todo!() } }
    impl<'i, R: RuleType> Debug for Pair<'i, R> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

    impl<'i, R: RuleType> Pairs<'i, R> {
        pub fn as_str(&self) -> &'i str { todo!() }
        pub fn peek(&self) -> Option<Pair<'i, R>> { todo!() }
        pub fn concat(&self) -> String { todo!() }
        pub fn flatten(self) -> FlatPairs<'i, R> { todo!() }
    }

    impl<'i, R: RuleType> Iterator for Pairs<'i, R> {
        type Item = Pair<'i, R>;
        fn next(&mut self) -> Option<Pair<'i, R>> { todo!() }
    }

    impl<'i, R: RuleType> DoubleEndedIterator for Pairs<'i, R> {
        fn next_back(&mut self) -> Option<Pair<'i, R>> { todo!() }
    }

    impl<'i, R: Clone> Clone for Pairs<'i, R> { fn clone(&self) -> Pairs<'i, R> { todo!() } }
    impl<'i, R: RuleType> Debug for Pairs<'i, R> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

    pub struct FlatPairs<'i, R>;
    pub struct Tokens<'i, R>;

    impl<'i, R: RuleType> Iterator for FlatPairs<'i, R> {
        type Item = Pair<'i, R>;
        fn next(&mut self) -> Option<Pair<'i, R>> { todo!() }
    }

    impl<'i, R: RuleType> Iterator for Tokens<'i, R> {
        type Item = Token<'i, R>;
        fn next(&mut self) -> Option<Token<'i, R>> { todo!() }
    }

    pub enum Token<'i, R> {
        Start { rule: R, pos: Position<'i> },
        End { rule: R, pos: Position<'i> },
    }
}

pub struct Span<'i>;

impl<'i> Span<'i> {
    pub fn start(&self) -> usize { todo!() }
    pub fn end(&self) -> usize { todo!() }
    pub fn as_str(&self) -> &'i str { todo!() }
    pub fn start_pos(&self) -> Position<'i> { todo!() }
}

pub struct Position<'i>;

impl<'i> Position<'i> {
    pub fn pos(&self) -> usize { todo!() }
    pub fn line_col(&self) -> (usize, usize) { todo!() }
}

pub mod error {
    /// `variant`, `location` and `line_col` are public *fields*, not methods.
    /// An earlier version of this file declared a `variant()` method, which
    /// made a call that does not compile resolve while real field access failed.
    pub struct Error<R> {
        pub variant: ErrorVariant<R>,
        pub location: InputLocation,
        pub line_col: LineColLocation,
    }

    impl<R: RuleType> Error<R> {
        pub fn line(&self) -> &str { todo!() }
        pub fn with_path(self, path: &str) -> Error<R> { todo!() }
        pub fn renamed_rules<F: FnMut(&R) -> String>(self, f: F) -> Error<R> { todo!() }
    }

    pub enum InputLocation {
        Pos(usize),
        Span((usize, usize)),
    }

    pub enum LineColLocation {
        Pos((usize, usize)),
        Span((usize, usize), (usize, usize)),
    }

    impl<R: RuleType> Debug for Error<R> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
    impl<R: RuleType> Display for Error<R> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
    impl<R: RuleType> std::error::Error for Error<R> {}

    pub enum ErrorVariant<R> {
        ParsingError { positives: Vec<R>, negatives: Vec<R> },
        CustomError { message: String },
    }
}
