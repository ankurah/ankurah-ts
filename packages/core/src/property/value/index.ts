// MIRRORS: ankurah/core/src/property/value/mod.rs

// pub mod entity_ref — not yet ported
// pub mod json
export * from './json.ts';
// pub mod lww
export * from './lww.ts';
// pub mod yrs
// Divergence: yrs.rs → yrs_string.ts [E5]
export * from './yrs_string.ts';

// Rust: pub use entity_ref::Ref — not yet ported
// Rust: pub use json::Json — re-exported via ./json.ts above
// Rust: pub use lww::LWW — re-exported via ./lww.ts above
// Rust: pub use yrs::YrsString — re-exported via ./yrs_string.ts above
