//! R1: a `?` keeps its test unless its OWN operand's value is a hole.
//!
//! The refusal here is inside a callback the operand passes, on a branch no
//! caller in the driver reaches. `try_operand` used to read a GLOBAL hole
//! counter before and after lowering the operand, so any refusal anywhere in
//! the subtree read as "the operand IS a hole" and the null test was dropped —
//! the body then computed on `null` where Rust answers `None`, silently. Live
//! at `storage-common/planner.ts`'s `build_ineq_first_plan`.

/// The `?` operand is `find_map(..)`, a real `Option`. The hole sits on the
/// `== 99` branch of the callback.
pub fn pick(xs: Vec<u32>, ys: Vec<u32>) -> Option<u32> {
    let v: u32 = xs
        .iter()
        .find_map(|x| {
            if *x == 99 {
                let mut it = ys.clone().into_iter();
                it.next()
            } else if *x > 3 {
                Some(*x)
            } else {
                None
            }
        })?;
    Some(v + 1)
}

/// The operand IS the hole, so there is nothing to test and the `?` stands for
/// the name the hole left behind. This is the case the counter was written for,
/// and it still answers the same way.
pub fn wholly(ys: Vec<u32>) -> Option<u32> {
    let mut it = ys.into_iter();
    let v = it.next()?;
    Some(v + 1)
}
