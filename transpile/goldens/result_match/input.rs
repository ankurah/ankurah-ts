//! `match` on a call that returns `Result` is a by-value match on the `Result`
//! the call built: both arms own their payload, both payloads have drop glue,
//! and whichever arm runs is what decides who releases the payload it was
//! handed. Nothing releases the `Result` itself, because the match moved it.

pub struct Entity {
    pub name: String,
}

pub struct Failure {
    pub reason: String,
}

pub fn consume_entity(entity: Entity) -> usize { entity.name.len() }

pub fn consume_failure(failure: Failure) -> usize { failure.reason.len() }

pub fn borrow_entity(entity: &Entity) -> usize { entity.name.len() }

pub fn borrow_failure(failure: &Failure) -> usize { failure.reason.len() }

pub fn fetch(raw: &str) -> Result<Entity, Failure> {
    if raw.is_empty() {
        return Err(Failure { reason: "empty".to_string() });
    }
    Ok(Entity { name: raw.to_string() })
}

/// Both arms hand their payload to a callee, so neither arm releases anything.
pub fn width(raw: &str) -> usize {
    match fetch(raw) {
        Ok(entity) => consume_entity(entity),
        Err(failure) => consume_failure(failure),
    }
}

/// Both arms keep their payload and only read it, so each arm releases the one
/// it was handed.
pub fn score(raw: &str) -> usize {
    match fetch(raw) {
        Ok(entity) => borrow_entity(&entity) + 1,
        Err(failure) => borrow_failure(&failure) + 100,
    }
}

/// The `Ok` arm hands its payload to the caller; the `Err` arm builds a fresh
/// one and still has to release the payload it was handed.
pub fn or_default(raw: &str) -> Entity {
    match fetch(raw) {
        Ok(entity) => entity,
        Err(failure) => Entity { name: "fallback".to_string() },
    }
}

// X9: a match written against a REFERENCE reads the payload and leaves the
// `Result` whole (RFC 2005: a pattern matched against a reference binds by
// reference). The port read it with `unwrap()`, which is Rust's `self` form and
// marks the `Result` moved — so the second read of the same value was
// `Result was used after being moved`.
pub fn width_of(result: &Result<Entity, Failure>) -> usize {
    match result {
        Ok(entity) => borrow_entity(entity),
        Err(failure) => borrow_failure(failure),
    }
}

/// The same through an `if let`, whose test is written after its branch.
pub fn entity_width(result: &Result<Entity, Failure>) -> usize {
    if let Ok(entity) = result { borrow_entity(entity) } else { 0 }
}

/// And nested under a borrowed `Option`, where the inner `Result` is borrowed
/// too.
pub fn maybe_width(result: &Option<Result<Entity, Failure>>) -> usize {
    match result {
        Some(Ok(entity)) => borrow_entity(entity),
        Some(Err(failure)) => borrow_failure(failure),
        None => 0,
    }
}
