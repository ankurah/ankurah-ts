//! `.await` in every POSTFIX position.
//!
//! Rust's `.await` is postfix and binds tighter than whatever follows it;
//! JavaScript's `await` is a PREFIX operator that binds LOOSER than member
//! access, an index or a call. The fourth pass parenthesised the base of a
//! method call and of a field access; an INDEX, a SLICE and a direct CALL took
//! the promise instead — `get_vec().await[0]` answered `undefined`, and the
//! other two threw `TypeError`.

pub async fn get_vec() -> Vec<u32> { vec![1, 2, 3] }
/// A callee the port can TYPE, so what this golden is about is the parentheses
/// and not the `fn` pointer type the port has no spelling for.
pub async fn get_function() -> Box<dyn Fn(u32) -> u32> { Box::new(double) }
pub fn double(n: u32) -> u32 { n * 2 }

pub struct Holder { pub items: Vec<u32> }
pub async fn get_holder() -> Holder { Holder { items: vec![4, 5] } }

/// An INDEX whose base awaits.
pub async fn first() -> u32 {
    get_vec().await[0]
}

/// A SLICE whose base awaits, with a method on the slice.
pub async fn tail() -> Vec<u32> {
    get_vec().await[1..].to_vec()
}

/// A direct CALL whose callee awaits.
pub async fn through() -> u32 {
    get_function().await(8)
}

/// The two the fourth pass already covered, kept as neighbours.
pub async fn width() -> usize {
    get_vec().await.len()
}

pub async fn held() -> Vec<u32> {
    get_holder().await.items
}
