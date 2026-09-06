//! What each declared name's moves add up to.
//!
//! One move on every path out is a MOVE and the block owes nothing; one on some
//! paths is FLAGGED and the block releases what the flag says it still holds;
//! none is KEPT. A move the emitter cannot write a flag for — inside a closure,
//! behind a `?` — is UNSURE, and the site is reported rather than guessed at.

use std::collections::HashMap;

use super::moves::{Disposition, Site, Where};

/// What each declared local's block should do with it.
///
/// Sites are attributed to the declaration that was in scope where they were
/// written, so `let staged = ..; use(staged); let staged = ..;` reads the first
/// binding as moved and the second as kept.
#[derive(Debug, Default)]
pub struct Dispositions {
    /// Keyed by the name Rust wrote and which declaration of it this is,
    /// counting from one.
    by_declaration: HashMap<(String, usize), Disposition>,
    /// The move sites that took a value into a closure, so the capture can be
    /// reported once at the site rather than once per use.
    pub captures: Vec<Site>,
    /// The sites the emitter could not write a flag for.
    pub unwritable: Vec<Site>,
}

impl Dispositions {
    pub fn of(&self, name: &str, ordinal: usize) -> Disposition {
        self.by_declaration
            .get(&(name.to_string(), ordinal))
            .copied()
            .unwrap_or(Disposition::Kept)
    }

    /// Attribute each site to the declaration it was written under.
    ///
    /// `declarations` is, in source order, the statement index of each `let`
    /// and the names it binds. A site in statement j belongs to the last
    /// declaration of that name before j.
    pub fn build(declarations: &[(usize, Vec<String>)], sites: Vec<(usize, Site)>) -> Dispositions {
        let mut result = Dispositions::default();
        for (stmt_index, site) in sites {
            let ordinal = declarations
                .iter()
                .filter(|(at, names)| *at < stmt_index && names.iter().any(|n| *n == site.name))
                .count();
            if ordinal == 0 {
                // Not one of this block's locals: a parameter, an outer local,
                // or a name from a pattern. The block that owns it decides.
                continue;
            }
            let key = (site.name.clone(), ordinal);
            let disposition = match site.at {
                Where::Straight | Where::Closure => Disposition::Moved,
                Where::Branch => Disposition::Flagged,
                Where::Unwritable => Disposition::Unsure,
            };
            if site.at == Where::Closure {
                result.captures.push(site.clone());
            }
            if site.at == Where::Unwritable {
                result.unwritable.push(site.clone());
            }
            // A local moved on a straight-line path is gone whatever else
            // happened to it; a flag would only ask a question already
            // answered. Otherwise the strongest claim wins.
            let existing = result.by_declaration.entry(key).or_insert(Disposition::Kept);
            *existing = stronger(*existing, disposition);
        }
        result
    }
}

/// Which of two claims about one local stands. "Gone" beats "sometimes gone"
/// beats "kept", because releasing a value somebody else owns is the failure
/// this analysis exists to prevent.
fn stronger(a: Disposition, b: Disposition) -> Disposition {
    let rank = |d: Disposition| match d {
        Disposition::Kept => 0,
        Disposition::Flagged => 1,
        Disposition::Unsure => 2,
        Disposition::Moved => 3,
    };
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

/// Every name a pattern binds, in the TypeScript spelling the sites use.
pub(super) fn collect_pattern_names(pat: &syn::Pat, out: &mut Vec<String>) {
    match pat {
        syn::Pat::Ident(ident) => {
            out.push(crate::name_map::to_camel_case(&ident.ident.to_string()));
            if let Some((_, sub)) = &ident.subpat {
                collect_pattern_names(sub, out);
            }
        }
        syn::Pat::Tuple(t) => t.elems.iter().for_each(|p| collect_pattern_names(p, out)),
        syn::Pat::TupleStruct(t) => t.elems.iter().for_each(|p| collect_pattern_names(p, out)),
        syn::Pat::Slice(s) => s.elems.iter().for_each(|p| collect_pattern_names(p, out)),
        syn::Pat::Struct(s) => s.fields.iter().for_each(|f| collect_pattern_names(&f.pat, out)),
        syn::Pat::Reference(r) => collect_pattern_names(&r.pat, out),
        syn::Pat::Type(t) => collect_pattern_names(&t.pat, out),
        syn::Pat::Paren(p) => collect_pattern_names(&p.pat, out),
        syn::Pat::Or(or) => or.cases.iter().for_each(|p| collect_pattern_names(p, out)),
        _ => {}
    }
}
