//! What each `From` / `TryFrom` impl's emitted static is CALLED, decided once.
//!
//! For: a type may convert from several sources, and every one of them wants a
//! name built from the same word. `RetrievalError` has `From<bincode::Error>`,
//! `From<crate::selection::filter::Error>` and `From<anyhow::Error>`; all three
//! spell `fromError` from the leaf alone. `Expr` has `From<String>`,
//! `From<i64>`, `From<f64>` and `From<bool>`, and all four spell `from`. A
//! class holds one member per name, so two impls that agree on a name mean one
//! of them is not emitted at all — and a CALL SITE that computes the name a
//! second way, from different information, reaches a member nothing declares.
//!
//! Both of those were live. The decision used to be a thread-local set of
//! `(self type LEAF, name)` strings, filled from the Rust leaf and read under
//! the TypeScript spelling, and three of the four call sites asked it with an
//! empty self type — so they never saw a contest and always wrote the plain
//! name while the declaration wrote the qualified one. Two unrelated `Wrap`
//! classes in different modules made each other contested. And `From<T>` and
//! `From<&T>` have one TypeScript signature, so they were merged: the owned
//! body ran for a borrowed value and dropped something its caller still owned.
//!
//! So the answer is computed HERE, once, over the whole impl table, keyed by
//! the resolved self `TypeId` and the source as the impl wrote it — and both
//! the class's declaration and every call site read the same map.

use std::collections::HashMap;

use crate::registry::{ImplId, TypeRegistry};
use crate::ty::{Ty, TypeId};

/// The name one conversion impl's static is emitted under, keyed by the two
/// things every asker has: which type is being built, and which source the impl
/// was written for, spelled as the impl wrote it.
pub type ConversionNames = HashMap<(TypeId, String), String>;

/// What tells two impls of one name apart.
///
/// The RUST source type, as the impl wrote it, and whether it is a reference.
/// Not the TypeScript spelling: R8 retracts "identical TypeScript signatures are
/// one method", which merged conversions with different BODIES.
/// `RetrievalError`'s three `From<..Error>` impls all spell `Error` in
/// TypeScript, and reading that as one identity emitted one of the three and
/// dropped the other two; `From<&str>` and `From<String>` are both
/// `from(v: string)` and are still two different conversions.
///
/// A reference is part of it because `From<Literal>` consumes its argument and
/// `From<&Literal>` does not, and TypeScript spells both the same.
#[derive(PartialEq, Eq, Hash, Clone)]
struct Identity {
    source: String,
    by_ref: bool,
}

struct Candidate {
    self_id: TypeId,
    written: String,
    base: &'static str,
    plain: bool,
    leaf: String,
    identity: Identity,
    /// The source as the engine resolved it, where it could. Read only to ask
    /// whether the source owns anything, which is what decides whether a
    /// reference is observable.
    resolved: Option<Ty>,
}

/// Every conversion static the impl table implies, named.
pub fn resolve_conversion_names(reg: &TypeRegistry) -> ConversionNames {
    let candidates = candidates(reg);
    // Group by the name each impl would take if nothing contested it.
    let mut groups: HashMap<(TypeId, String), Vec<&Candidate>> = HashMap::new();
    for candidate in &candidates {
        groups
            .entry((candidate.self_id, unqualified(candidate)))
            .or_default()
            .push(candidate);
    }
    let mut out: ConversionNames = HashMap::new();
    let mut settled: HashMap<(TypeId, String), Identity> = HashMap::new();
    let mut groups: Vec<((TypeId, String), Vec<&Candidate>)> = groups.into_iter().collect();
    // Deterministic order, so a name a contest cannot resolve is reported the
    // same way every run.
    groups.sort_by(|a, b| a.0 .1.cmp(&b.0 .1));
    for ((self_id, unqualified_name), members) in groups {
        let identities: std::collections::HashSet<&Identity> =
            members.iter().map(|c| &c.identity).collect();
        report_reference_merges(reg, self_id, &members);
        for candidate in &members {
            // One identity means one method, whatever the source paths say:
            // `From<&str>` and `From<String>` are the same `from(v: string)`,
            // and writing two of them would be a name collision rather than a
            // distinction.
            let name = if identities.len() == 1 {
                unqualified_name.clone()
            } else {
                qualified(candidate)
            };
            // The answer has to resolve the contest it was written for. Where
            // it does not, the site says so rather than letting emission keep
            // one impl and drop the other in silence.
            match settled.get(&(self_id, name.clone())) {
                Some(other) if *other != candidate.identity => {
                    crate::diag::pending::park_at(
                        0,
                        0,
                        format!(
                            "`{}` converts from `{}` and from another source, and both are \
                             emitted as `{}`; a class holds one member per name, so one of the \
                             two conversions is lost",
                            reg.describe(&Ty::Named {
                                id: self_id,
                                args: Vec::new()
                            }),
                            candidate.written,
                            name
                        ),
                    );
                }
                _ => {
                    settled.insert((self_id, name.clone()), candidate.identity.clone());
                }
            }
            out.insert((self_id, candidate.written.clone()), name);
        }
    }
    out
}

/// Two impls that differ only by a `&` on a source the port gives no class to
/// are ONE emitted method, and the site says so.
///
/// The `by_ref` bit marks a reference only where the source has an emitted
/// class, because that is where the difference is observable — `From<Literal>`
/// consumes what it is handed and `From<&Literal>` does not. A slice, an array
/// and a container have no class of their own, so `From<[T; N]>` and
/// `From<&[T; N]>` are one identity here, one emitted method, and one of the two
/// BODIES is lost: Rust's borrowed impl clones each element where the owned one
/// moves it. The engine cannot tell whether an open `T` owns anything, so it
/// cannot decide the pair either; what it can do is stop losing the body in
/// silence.
fn report_reference_merges(reg: &TypeRegistry, self_id: TypeId, members: &[&Candidate]) {
    let probe = crate::registry::Probe::new(reg, reg.crate_root());
    for (at, one) in members.iter().enumerate() {
        for other in members.iter().skip(at + 1) {
            if one.identity != other.identity {
                continue;
            }
            let (owned, borrowed) = match (one.written.starts_with('&'), other.written.starts_with('&')) {
                (false, true) => (one, other),
                (true, false) => (other, one),
                _ => continue,
            };
            if owned.written != borrowed.written.trim_start_matches('&') {
                continue;
            }
            // Only where the source could own something. A `String` is a
            // `string` here and the port has nothing to release, so
            // `From<String>` and `From<&String>` really are one conversion —
            // §4.7's rule, and reporting it would be noise. A value the runtime
            // owns, or one the engine cannot name, is the case where the two
            // bodies differ.
            let owns = match &owned.resolved {
                Some(ty) => !matches!(
                    crate::ownership::drops_of(&probe, ty.peel_refs()),
                    crate::ownership::Drops::Nothing
                ),
                None => false,
            };
            if !owns {
                continue;
            }
            crate::diag::pending::park_at(
                0,
                0,
                format!(
                    "`{}` converts from `{}` and from `{}`, and the port gives that source \
no class of its own — so the two are one emitted method and one of the two bodies is \
lost; Rust's borrowed impl clones what the owned one moves",
                    reg.describe(&Ty::Named { id: self_id, args: Vec::new() }),
                    owned.written,
                    borrowed.written
                ),
            );
        }
    }
}

/// Every `From` / `TryFrom` impl in the table, with what names it.
fn candidates(reg: &TypeRegistry) -> Vec<Candidate> {
    let mut out = Vec::new();
    for i in 0..reg.impls().len() {
        let def = reg.impl_def(ImplId(i as u32));
        // The declared std surface's impls describe conversions the runtime
        // already has; nothing is emitted for them and nothing names them.
        if reg.modules().get(def.module).is_system {
            continue;
        }
        let Some(implemented) = def.trait_ref.as_ref() else {
            continue;
        };
        let trait_leaf = leaf(&reg.name_of(implemented.id));
        let base = match trait_leaf.as_str() {
            "From" => "from",
            "TryFrom" => "tryFrom",
            _ => continue,
        };
        let Some(self_id) = def.self_ty.peel_refs().id() else {
            continue;
        };
        let Some(written) = def.trait_args_written.first().cloned() else {
            continue;
        };
        if written == "never" || written == "Infallible" {
            continue;
        }
        let source_leaf = leaf(written.trim_start_matches('&'));
        // `From<Literal>` and `From<&Literal>` are one TypeScript signature and
        // two different bodies: one consumes what it is given and one does not.
        // The distinction is only OBSERVABLE where the source is a value the
        // port owns — a `String` and a `&str` are both `string`, and there is
        // nothing to double-drop — so a reference marks the name only when the
        // source has a class of its own.
        let by_ref = matches!(implemented.args.first(), Some(Ty::Ref { .. }))
            && implemented
                .args
                .first()
                .is_some_and(|ty| super::dispatch::has_emitted_class(reg, ty));
        out.push(Candidate {
            self_id,
            base,
            plain: crate::emit::source_reads_as_plain(&written),
            identity: Identity {
                source: same_type(written.trim_start_matches('&')),
                by_ref,
            },
            resolved: implemented.args.first().cloned(),
            leaf: source_leaf,
            written,
        });
    }
    out
}

/// The name this impl takes when nothing contests it.
///
/// A source whose spelling carries type arguments — `Attested<Event>` — has no
/// name to give: `fromAttested<Event>` is not an identifier. It takes the plain
/// name, and the contest between two such sources is what the qualified form
/// below has to settle.
fn unqualified(candidate: &Candidate) -> String {
    if candidate.plain || !nameable(&candidate.leaf) {
        candidate.base.to_string()
    } else {
        format!("{}{}", candidate.base, capitalised(&candidate.leaf))
    }
}

/// Is this leaf a thing a method name can be built out of? A type name, with
/// no arguments, no tuple and no whitespace in it.
fn nameable(leaf: &str) -> bool {
    leaf.chars().next().is_some_and(|c| c.is_uppercase())
        && !leaf.contains('<')
        && !leaf.contains(',')
        && !leaf.contains(' ')
}

/// The name this impl takes when something does.
///
/// A source whose TypeScript spelling says nothing about which impl it is —
/// `String`, `i64`, `Vec<u8>` — is told apart by its Rust leaf; every other by
/// the module segment in front of it. A reference is marked, because the two
/// differ in what they do with what they are given and in nothing TypeScript
/// can see.
fn qualified(candidate: &Candidate) -> String {
    // Named from the IDENTITY, not from the candidate's own spelling: two impls
    // the identity says are one conversion have to reach one name, or
    // `From<&str>` and `From<String>` become `fromStr` and `fromString` — two
    // methods with one signature and one body.
    let source = &candidate.identity.source;
    let leaf = leaf(source);
    let tail = if candidate.plain || !nameable(&leaf) {
        // `From<i32>` and `From<f64>` are both `from(v: number)`, so the
        // TypeScript spelling cannot tell them apart and the RUST leaf is what
        // names them: `fromI32`, `fromF64`.
        crate::emit::name_fragment(&leaf).unwrap_or_else(|| capitalised(&leaf))
    } else {
        crate::emit::qualified_source(source)
    };
    if candidate.identity.by_ref {
        format!("{}Ref{}", candidate.base, tail)
    } else {
        format!("{}{}", candidate.base, tail)
    }
}

/// The source type, with a BORROWED spelling read as the owned one it is a
/// spelling of.
///
/// `From<&str>` and `From<String>` are two impls of one conversion: `str` is
/// `String`'s borrowed form, the port writes both as `string`, and there is
/// nothing a caller could tell apart. `From<i32>` and `From<f64>` are not —
/// both are `number` here and neither is the other's borrowed form, and reading
/// them as one emitted one impl and dropped the other. `by_ref` carries the
/// distinction that IS observable: a reference to a type the port gives a class
/// to, whose conversion does not consume what it is handed.
fn same_type(source: &str) -> String {
    match source {
        "str" => "String".to_string(),
        other => other.to_string(),
    }
}

fn capitalised(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The last segment of a path, where the `::` belongs to the PATH and not to a
/// generic argument inside it.
///
/// The leaf of `a::b::C` is `C`; the leaf of `Vec<a::Item>` is the whole
/// spelling, because `Vec` carries no qualifier of its own and the `a::` is
/// the ARGUMENT's. Split naively, `Vec<left::Item>` had the leaf `Item>` and
/// the qualifier `Vec<left`, and the conversion was named `fromVec<leftItem>`
/// — a method name with a type-parameter list in the middle of it.
fn leaf(path: &str) -> String {
    let outer = path.split_once('<').map(|(head, _)| head).unwrap_or(path);
    match outer.rsplit_once("::") {
        Some((_, last)) => format!("{}{}", last, &path[outer.len()..]),
        None => path.to_string(),
    }
}

thread_local! {
    /// The one decision, read by the class's declaration and by every call
    /// site. A thread-local because emission is a straight line through one
    /// crate and the registry is built before any of it is written; a value
    /// threaded through every emission signature would say nothing more.
    static NAMES: std::cell::RefCell<ConversionNames> =
        std::cell::RefCell::new(HashMap::new());
}

pub fn set_conversion_names(names: ConversionNames) {
    NAMES.with(|n| *n.borrow_mut() = names);
}

/// What this conversion's static is called: the type being built, and the
/// source as the impl wrote it.
pub fn conversion_name(self_id: TypeId, written_source: &str) -> Option<String> {
    NAMES.with(|n| {
        n.borrow()
            .get(&(self_id, written_source.to_string()))
            .cloned()
    })
}

/// The same, for a caller holding the impl rather than the pair.
pub fn conversion_name_of_impl(reg: &TypeRegistry, id: ImplId) -> Option<String> {
    let def = reg.impl_def(id);
    let self_id = def.self_ty.peel_refs().id()?;
    let written = def.trait_args_written.first()?;
    conversion_name(self_id, written)
}

#[cfg(test)]
mod source_tests {
    use crate::testing::Fixture;

    /// R8's identity is the RUST source, and it reaches all the way down. Two
    /// conversions whose sources differ only inside their arguments —
    /// `From<Vec<u32>>` and `From<Vec<i32>>` — both spell `number[]` in
    /// TypeScript, so they were one identity: one emitted static, one body, and
    /// the other lost with no diagnostic at all.
    #[test]
    fn two_generic_sources_are_two_conversions() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct T(pub String);\n\
             impl From<Vec<u32>> for T { fn from(v: Vec<u32>) -> Self { T(String::new()) } }\n\
             impl From<Vec<i32>> for T { fn from(v: Vec<i32>) -> Self { T(String::new()) } }\n\
             impl From<&[u32]> for T { fn from(v: &[u32]) -> Self { T(String::new()) } }",
        )]);
        let ts = f.emitted("lib.rs");
        assert!(ts.contains("static fromVecU32("), "{ts}");
        assert!(ts.contains("static fromVecI32("), "{ts}");
        assert!(ts.contains("static fromU32("), "{ts}");
    }

    /// Z5: and the module qualifier reaches all the way down too. Inside a
    /// generic argument it is the only thing telling two source types apart,
    /// and it was dropped: `From<Vec<a::Item>>` and `From<Vec<b::Item>>` were
    /// one identity, one static and one body, with the other lost silently.
    #[test]
    fn two_generic_sources_that_differ_only_by_module_are_two_conversions() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub mod a { pub struct Item; }
             pub mod b { pub struct Item; }
             pub struct T(pub String);
             impl From<Vec<a::Item>> for T { fn from(v: Vec<a::Item>) -> Self { T(String::new()) } }
             impl From<Vec<b::Item>> for T { fn from(v: Vec<b::Item>) -> Self { T(String::new()) } }",
        )]);
        let ts = f.emitted("lib.rs");
        assert!(ts.contains("static fromVecAItem("), "{ts}");
        assert!(ts.contains("static fromVecBItem("), "{ts}");
    }

    /// `From<&str>` and `From<String>` are still ONE conversion: `str` is
    /// `String`'s borrowed form, the port writes both as `string`, and there is
    /// nothing a caller could tell apart (fixpass3's §4.7).
    #[test]
    fn the_borrowed_form_of_a_string_is_still_one_conversion() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct T(pub String);\n\
             impl From<String> for T { fn from(v: String) -> Self { T(v) } }\n\
             impl From<&str> for T { fn from(v: &str) -> Self { T(v.to_string()) } }",
        )]);
        let ts = f.emitted("lib.rs");
        assert_eq!(ts.matches("static from").count(), 1, "{ts}");
    }

    /// A reference marks the name only where the source has a class of its own,
    /// so `From<[T; N]>` and `From<&[T; N]>` are one emitted method and one of
    /// the two BODIES is lost — Rust's borrowed impl clones what the owned one
    /// moves. The engine cannot decide the pair; it can stop losing the body in
    /// silence.
    #[test]
    fn a_reference_merge_the_engine_cannot_decide_is_reported() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Held { pub n: u32 }\n\
             pub struct T(pub String);\n\
             impl From<Vec<Held>> for T { fn from(v: Vec<Held>) -> Self { T(String::new()) } }\n\
             impl From<&Vec<Held>> for T { fn from(v: &Vec<Held>) -> Self { T(String::new()) } }",
        )]);
        let _ = f.emitted("lib.rs");
        assert!(
            f.messages().iter().any(|m| m.contains("one of the two bodies is lost")),
            "{:?}",
            f.messages()
        );
    }

    /// And a source the port has nothing to release is NOT reported: a `String`
    /// is a `string` here, so `From<String>` and `From<&String>` really are one
    /// conversion.
    #[test]
    fn a_reference_merge_over_a_value_that_owns_nothing_is_not_reported() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct T(pub String);\n\
             impl From<Vec<u32>> for T { fn from(v: Vec<u32>) -> Self { T(String::new()) } }\n\
             impl From<&Vec<u32>> for T { fn from(v: &Vec<u32>) -> Self { T(String::new()) } }",
        )]);
        let _ = f.emitted("lib.rs");
        assert!(
            !f.messages().iter().any(|m| m.contains("one of the two bodies is lost")),
            "{:?}",
            f.messages()
        );
    }
}
