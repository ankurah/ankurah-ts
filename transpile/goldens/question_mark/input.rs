#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    Empty,
}

pub struct Header {
    pub name: String,
}

impl Header {
    pub fn parse(raw: &str) -> Result<Header, ParseError> {
        if raw.is_empty() {
            return Err(ParseError::Empty);
        }
        Ok(Header { name: raw.to_string() })
    }

    /// The `?` under test: a fallible call whose error type already matches.
    pub fn parse_twice(raw: &str) -> Result<(Header, Header), ParseError> {
        let first = Header::parse(raw)?;
        let second = Header::parse(raw)?;
        Ok((first, second))
    }
}
