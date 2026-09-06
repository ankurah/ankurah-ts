//! A unit struct written as a VALUE.
//!
//! Rust's `Mock` in expression position is a value of type `Mock`, and it is
//! how a zero-sized implementor is handed to something that calls its trait
//! methods. The port writes such a type as a class, so the name on its own is
//! the CONSTRUCTOR — which has none of the instance members the callee reaches
//! for. Nine live sites in `core/peer_subscription/client_relay.rs` pass
//! `MockLiveQuery` this way.

pub trait Greets {
    fn greeting(&self) -> String;
}

pub struct Mock;

impl Greets for Mock {
    fn greeting(&self) -> String { "mock".to_string() }
}

pub struct Loud;

impl Greets for Loud {
    fn greeting(&self) -> String { "LOUD".to_string() }
}

/// The value position: what the caller hands on.
pub fn a_mock() -> Mock { Mock }

/// And the same value reaching the trait method it was handed over for.
pub fn greet_with<G: Greets>(g: &G) -> String { g.greeting() }
