//! `for x in owned_vec` hands each element to the body as the body's own, so
//! the body releases it at the end of every turn — the turn a `break` cuts
//! short included — and whatever the loop never reached is released with the
//! iterator. A loop over `&vec` hands out borrows and releases nothing.

pub struct Entity {
    pub name: String,
}

pub fn consume(entity: Entity) -> usize { entity.name.len() }

pub fn borrow(entity: &Entity) -> usize { entity.name.len() }

/// The body only borrows each element, so each turn releases the one it was
/// handed, and the `break` leaves the rest to the iterator.
pub fn drain(entities: Vec<Entity>, stop_at: usize) -> usize {
    let mut total = 0usize;
    for entity in entities {
        total += borrow(&entity);
        if total > stop_at {
            break;
        }
    }
    total
}

/// Every turn hands its element to a callee, so the body releases nothing.
pub fn consume_all(entities: Vec<Entity>) -> usize {
    let mut total = 0usize;
    for entity in entities {
        total += consume(entity);
    }
    total
}

/// The `break` stands *above* the move, so the element the turn was handed is
/// still the body's on that path: a drop flag decides which of the two paths
/// releases it.
pub fn take_until(entities: Vec<Entity>, stop_at: usize) -> usize {
    let mut total = 0usize;
    for entity in entities {
        if entity.name.len() > stop_at {
            break;
        }
        total += consume(entity);
    }
    total
}

/// Borrowed elements: the vector keeps every one of them.
pub fn measure(entities: &Vec<Entity>) -> usize {
    let mut total = 0usize;
    for entity in entities {
        total += borrow(entity);
    }
    total
}
