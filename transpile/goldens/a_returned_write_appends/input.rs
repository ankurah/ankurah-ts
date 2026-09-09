// Every `write!` inside a `Display` APPENDS to what the formatter has composed,
// including one a `return` carries out of the body.
//
// Read as an ordinary `return`, the string that write produced became the whole
// answer and everything written before it was discarded: `Size(200)` printed as
// `big)` where Rust prints `Size(big)`.

use std::fmt;

pub struct Size {
    pub n: u32,
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Size(")?;
        if self.n > 100 {
            return write!(f, "big)");
        }
        write!(f, "{})", self.n)
    }
}

pub fn large() -> String {
    Size { n: 200 }.to_string()
}

pub fn small() -> String {
    Size { n: 7 }.to_string()
}
