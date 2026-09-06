//! Derive hooks: what each `#[derive(..)]` writes, written by the emitter.
//!
//! For: a derive is a promise that the type has a behaviour, and the port has to
//! keep the same promise. The engine already registers the *impls* a derive
//! proves (`registry::build`), so a bound resolves; this is the other half —
//! the TypeScript those impls actually run.
//!
//! Nothing here expands a macro. Each hook reads the derive's own syntax — the
//! derive list, the `#[error("..")]` strings, the `#[from]` fields — and writes
//! what the equivalent hand port would write, with the engine supplying the
//! types.

pub mod debug_derive;
pub mod default_value;
pub mod cloning;
pub mod equality;
pub mod hashing;
pub mod ordering;
pub mod debug_fmt;
pub mod thiserror;

use std::collections::HashSet;

use crate::registry::TypeRegistry;
use crate::types::{EnumInfo, StructInfo};

/// Something a derive hook could not carry over, and where to say so.
///
/// Emission runs after body translation, so these do not reach the sink through
/// `BodyTranslator::fallback`; the emitter parks them the way that method parks
/// a fallback taken with no sink in reach, and the file's caller drains them.
pub type Gap = (proc_macro2::Span, String);

/// File every gap a derive hook found, at the declaration it came from.
pub fn report(gaps: Vec<Gap>) {
    for (span, message) in gaps {
        crate::diag::pending::park(span, message);
    }
}

/// Everything an enum's derives add to its class, and what the port could not
/// carry over.
///
/// `emitted` is what the class already has from its written impls. A derive is
/// the *absence* of a written impl, so anything already there wins: a
/// hand-written `impl Display` keeps its `toString`, exactly as Rust's coherence
/// would refuse the derive alongside it.
pub fn enum_members(
    reg: &TypeRegistry,
    self_id: Option<crate::ty::TypeId>,
    e: &EnumInfo,
    emitted: &mut HashSet<String>,
) -> (String, Vec<Gap>) {
    let mut out = String::new();
    let mut gaps = Vec::new();
    if e.derives.iter().any(|d| d == "Debug") && emitted.insert("debug".to_string()) {
        let (ts, mut said) = debug_derive::enum_debug(reg, e);
        out.push_str(&ts);
        gaps.append(&mut said);
    }
    if thiserror::is_thiserror(&e.derives) && emitted.insert("toString".to_string()) {
        let (ts, mut said) = thiserror::enum_error(reg, self_id, e);
        out.push_str(&ts);
        gaps.append(&mut said);
        // thiserror also reads a container-level `#[error(..)]`, which stands
        // for every variant at once. The reader takes the variant attributes
        // only, so one written on the type would leave every arm without a
        // message; saying so here is what keeps that from passing unnoticed.
        if e.variants.iter().all(|v| v.error_text.is_none()) && !e.variants.is_empty() {
            gaps.push((
                e.span,
                format!(
                    "no variant of `{}` carries an `#[error(\"..\")]`, so either the attribute is \
                     written on the type itself — which this hook does not read — or the derive \
                     is not thiserror's",
                    e.name
                ),
            ));
        }
    }
    (out, gaps)
}

/// The same for a struct. thiserror allows a struct error too; the corpus has
/// none, and one is reported rather than half-written.
pub fn struct_members(
    reg: &TypeRegistry,
    s: &StructInfo,
    emitted: &mut HashSet<String>,
) -> (String, Vec<Gap>) {
    let mut out = String::new();
    let mut gaps = Vec::new();
    if s.derives.iter().any(|d| d == "Debug") && emitted.insert("debug".to_string()) {
        let (ts, mut said) = debug_derive::struct_debug(reg, s);
        out.push_str(&ts);
        gaps.append(&mut said);
    }
    if thiserror::is_thiserror(&s.derives) {
        gaps.push((
            s.span,
            format!(
                "`{}` is a struct carrying `#[derive(Error)]`, and the hook writes the enum form \
                 only, so it has no `toString` of its own",
                s.name
            ),
        ));
    }
    (out, gaps)
}

