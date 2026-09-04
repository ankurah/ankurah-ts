//! `std::collections` — the re-exports.
//!
//! A container is declared in the module that owns it and re-exported here,
//! exactly as std does it: `HashMap` is written `std::collections::HashMap` at
//! almost every use site and `std::collections::hash_map::Entry` at the ones
//! that name an entry. Both paths have to resolve, and to the same type.
//!
//! Only the containers are re-exported. Each module's `Iter`, `IntoIter`,
//! `Keys` and `Values` keep their own module, which is what makes
//! `hash_map::Iter` and `btree_map::Iter` two types rather than one.

pub use binary_heap::BinaryHeap;
pub use btree_map::BTreeMap;
pub use btree_set::BTreeSet;
pub use hash_map::HashMap;
pub use hash_set::HashSet;
pub use vec_deque::VecDeque;
