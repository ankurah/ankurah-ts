// ONE emitted binding per module-level function, read by every reader.
//
// A module-level function is a BINDING, and JavaScript refuses a reserved word
// in that position: `pub fn with` is `export function with_`. That answer used
// to be derived at the declaration and derived again, differently, by the
// readers — so the map that says which module a name comes from held `with`
// while the declaration and every call wrote `with_`. Nothing matched, so no
// import was written at all, and the calls below named bindings this file never
// declares. In silence: there was no diagnostic, and the failure is a
// `ReferenceError` at the first call.

mod words;

use crate::words::{plain, unsupported, with};

pub fn caller() -> i64 {
    plain() + with() + unsupported()
}
