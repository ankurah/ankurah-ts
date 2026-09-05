//! A `?` and a `return` written inside a body the emitter lifts into an arrow.
//!
//! An arrow is a function, and `return` inside one returns from the arrow. So
//! the value of an `if`, of a block and of a `match` used as a value, and the
//! arms of a consuming `match` written as a statement, all have to hand the
//! function's exit back as a value the statement below performs — the same way
//! `break` is handed back. Before this, `Result.Err(..)` came out as the value
//! of the `if`, and `Result.Err` is a truthy object: `commit_remote_transaction`
//! took the success branch for an event that had failed to apply.

#[derive(Debug, Clone, PartialEq)]
pub enum ApplyError {
    Refused,
}

pub struct Entity {
    pub name: String,
}

impl Entity {
    pub fn apply(&self, ok: bool) -> Result<bool, ApplyError> {
        if ok {
            Ok(true)
        } else {
            Err(ApplyError::Refused)
        }
    }
}

pub enum Step {
    Skip,
    Apply(bool),
}

/// The `if` used as a value: the `?` in the else branch leaves `commit`.
pub fn commit(entity: &Entity, already: bool, ok: bool) -> Result<u32, ApplyError> {
    let applied = if already { true } else { entity.apply(ok)? };
    if applied {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// A block used as a value, with the `?` in the middle of it.
pub fn commit_block(entity: &Entity, ok: bool) -> Result<u32, ApplyError> {
    let applied = {
        let a = entity.apply(ok)?;
        a
    };
    if applied {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// A plain `return` written in a lifted body leaves the function too.
pub fn commit_early(entity: &Entity, stop: bool, ok: bool) -> Result<u32, ApplyError> {
    let applied = if stop {
        return Ok(7);
    } else {
        entity.apply(ok)?
    };
    if applied {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// A consuming `match` written as a statement, whose arm leaves with an error.
/// The arm is an arrow, so its `return` used to go nowhere.
pub fn run(entity: &Entity, step: Step) -> Result<u32, ApplyError> {
    let mut count = 0u32;
    match step {
        Step::Skip => {}
        Step::Apply(ok) => {
            let applied = entity.apply(ok)?;
            if applied {
                count += 1;
            }
        }
    }
    Ok(count)
}
