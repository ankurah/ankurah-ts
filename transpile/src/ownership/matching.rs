//! What a pattern's bindings own, for as long as the scope they belong to runs.
//!
//! A `match` arm, one turn of a `for` loop, an `if let` branch and a `while
//! let` turn are each handed values by a pattern, and Rust drops them at the
//! end of that scope and on an unwind through it, exactly as it drops a `let`.
//! Nothing outside the scope can release them: the names do not exist there.
//!
//! So the scope claims what its pattern bound before its text is written — the
//! flags first, because a branch inside the scope sets them — and closes over
//! that text with the `finally` that releases them. A `match` also has a
//! question no other scope has, which is whether the subject hands its payload
//! to the arms at all: `match_takes` answers it, and it decides between the
//! consuming form and the borrowing one.

use crate::body::BodyTranslator;
use crate::ownership;

impl<'a> BodyTranslator<'a> {
    /// Whether this `match` hands the subject's payload to an arm, which is
    /// what tells the consuming form from the borrowing one.
    pub fn match_takes(&self, m: &syn::ExprMatch) -> ownership::scrutinee::Takes {
        let Some(tc) = &self.types else {
            return ownership::scrutinee::Takes::Nothing;
        };
        let tc = tc.borrow();
        // Asking is not translating: the questions this resolution defers are
        // reported once, where the match is written out.
        let mark = tc.sink.mark();
        let subject = tc.resolve_expr(&m.expr);
        tc.sink.rewind(mark);
        let Ok(subject) = subject else {
            return ownership::scrutinee::Takes::Nothing;
        };
        drop(tc);
        if !self.owns_place(&m.expr) {
            return ownership::scrutinee::Takes::Nothing;
        }
        let tc = self.types.as_ref().expect("just borrowed").borrow();
        ownership::scrutinee::takes(&tc.probe(), &subject, &m.arms, |path| {
            let mark = tc.sink.mark();
            let payload = tc.payload_of(path, Some(&subject));
            tc.sink.rewind(mark);
            payload
                .unwrap_or_default()
                .into_iter()
                .map(|(_, ty)| ty)
                .collect()
        })
    }

    /// Say so where a `match` has a shape the emitter has no lowering for.
    pub fn report_match_gap(&self, m: &syn::ExprMatch, what: impl Into<String>) {
        self.fallback(syn::spanned::Spanned::span(m), what);
    }

    /// What the block owes each name a pattern bound, and under what condition.
    ///
    /// A match arm, one turn of a `for` loop and an `if let` branch are each
    /// handed values by a pattern and own them for the length of that scope:
    /// Rust drops each of them at the end of the arm and on an unwind through
    /// it, exactly as it drops a `let`. The flags are registered here, before
    /// the scope's text is written, because a branch inside it sets them.
    pub fn claim_bindings(&self, names: &[String], body: &[syn::Stmt]) -> Vec<ownership::Owned> {
        let Some(tc) = &self.types else {
            return Vec::new();
        };
        let scan = ownership::Scan::new(self);
        let sites: Vec<(usize, ownership::moves::Site)> = scan
            .block(body)
            .into_iter()
            .map(|site| (1, site))
            .collect();
        let dispositions =
            ownership::Dispositions::build(&[(0, names.to_vec())], sites);
        let mut owned = Vec::new();
        for name in names {
            let Some(ty) = tc.borrow().lookup(name) else {
                continue;
            };
            let drops = ownership::drops_of(&tc.borrow().probe(), &ty);
            if !drops.is_droppable() {
                continue;
            }
            let flag = match dispositions.of(name, 1) {
                ownership::Disposition::Moved | ownership::Disposition::Unsure => continue,
                ownership::Disposition::Kept => None,
                ownership::Disposition::Flagged => {
                    let flag = self.fresh_hoist("_moved");
                    self.own.flags.borrow_mut().insert(name.clone(), flag.clone());
                    Some(flag)
                }
            };
            owned.push(ownership::Owned {
                name: name.clone(),
                source: Some(name.clone()),
                drops,
                flag,
                statement_scoped: false,
            });
        }
        owned
    }

    /// Close the scope `claim_bindings` opened: the flag declarations stand
    /// first, the releases go in a `finally` in reverse binding order, and the
    /// flags stop being visible to anything written after.
    pub fn wrap_bindings(&self, owned: &[ownership::Owned], text: String) -> String {
        let mut declarations = String::new();
        for value in owned {
            if let Some(flag) = &value.flag {
                declarations.push_str(&format!("let {} = false;\n", flag));
            }
        }
        let mut out = text;
        for value in owned.iter().rev() {
            out = ownership::wrap(&out, value);
        }
        for value in owned {
            if let Some(source) = &value.source {
                self.own.flags.borrow_mut().remove(source);
            }
        }
        format!("{}{}", declarations, out)
    }
}
