use serde::{Deserialize, Serialize};

/// One variant of each shape: unit, tuple payload, and named fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Notice {
    Idle,
    Text(String),
    Span { start: u32, end: u32 },
}

impl Notice {
    pub fn is_idle(&self) -> bool {
        match self {
            Notice::Idle => true,
            Notice::Text(_) => false,
            Notice::Span { .. } => false,
        }
    }
}
