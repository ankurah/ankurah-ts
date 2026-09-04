use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub name: Option<String>,
    pub count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SlotError {
    Missing,
}

impl Slot {
    pub fn require_count(&self) -> Result<u32, SlotError> {
        match self.count {
            Some(n) => Ok(n),
            None => Err(SlotError::Missing),
        }
    }
}
