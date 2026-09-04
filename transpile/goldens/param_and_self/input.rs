//! A parameter taken by value is the callee's, so the callee releases it — and
//! where it hands it away on some paths and not others, Rust compiles a drop
//! flag. `self` taken by value is that same rule applied to the receiver:
//! `into_inner` hands one field to the caller and the rest of the receiver is
//! released here, while `&self` leaves the whole receiver to its caller.

pub struct Entity {
    pub name: String,
}

pub struct Holder {
    pub inner: Entity,
    pub spare: Entity,
}

pub fn consume(entity: Entity) -> usize { entity.name.len() }

pub fn borrow(entity: &Entity) -> usize { entity.name.len() }

/// Handed away only where the flag is set; kept and released otherwise.
pub fn forward(entity: Entity, hand_it_on: bool) -> usize {
    if hand_it_on {
        return consume(entity);
    }
    borrow(&entity)
}

impl Holder {
    /// `&self`: the caller still owns the `Holder` after this.
    pub fn width(&self) -> usize {
        borrow(&self.inner) + borrow(&self.spare)
    }

    /// `self` by value: `inner` goes to the caller, and the receiver — still
    /// holding `spare` — is released here.
    pub fn into_inner(self) -> Entity {
        self.inner
    }

    /// `self` by value with nothing handed out: both fields go with the
    /// receiver.
    pub fn width_owned(self) -> usize {
        borrow(&self.inner)
    }
}
