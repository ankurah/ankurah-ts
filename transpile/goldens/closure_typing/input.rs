//! A closure is written without types because the position it stands in
//! supplies them: the `Fn` bound on the parameter it is passed to, the
//! annotation at the binding site, or the closure's own `|x: T|`. Until those
//! are read, every call inside the body is dispatched by name.

pub struct Reading {
    pub level: u32,
}

impl Reading {
    pub fn doubled(&self) -> u32 {
        self.level * 2
    }
}

/// The callee's bound is the only thing that says what `reading` holds.
pub fn each_doubled(readings: &[Reading]) -> Vec<u32> {
    readings.iter().map(|reading| reading.doubled()).collect()
}

/// The annotation the closure writes for itself wins, because it is what the
/// source said.
pub fn scaled(readings: &[Reading]) -> Vec<u32> {
    readings.iter().map(|reading: &Reading| reading.level).collect()
}

/// A `Box<dyn Fn(..)>` at a binding site types the parameter, and the body's
/// tail is what the result is.
pub fn threshold(limit: u32) -> Box<dyn Fn(u32) -> bool> {
    Box::new(move |level| level > limit)
}

/// A closure taking nothing, so the callable has no inputs to read.
pub fn counted(readings: &[Reading]) -> usize {
    readings.iter().filter(|reading| reading.level > 0).count()
}
