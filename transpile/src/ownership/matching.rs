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
        // Q3: a match the `Option` rewriting made hands the payload over by
        // construction — that is what the rewriting is for — and the subject
        // expression still resolves to the `Option` around it.
        if self.matches_a_payload(&m.expr) {
            return ownership::scrutinee::Takes::Payload;
        }
        let patterns: Vec<&syn::Pat> = m.arms.iter().map(|arm| &arm.pat).collect();
        self.pattern_takes(&m.expr, &patterns)
    }

    /// The same question for an `if let` or a `while let`, whose one pattern
    /// takes the subject apart exactly as an arm does.
    ///
    /// Without this the scrutinee of `if let Some(payload) = value` was still
    /// the block's to release after the branch had taken the payload out of it,
    /// and the two releases landed on one value.
    pub fn let_takes(&self, let_expr: &syn::ExprLet) -> ownership::scrutinee::Takes {
        self.pattern_takes(&let_expr.expr, &[&let_expr.pat])
    }

    /// Do any of these patterns bind, by value, something the subject owns?
    fn pattern_takes(
        &self,
        subject_expr: &syn::Expr,
        patterns: &[&syn::Pat],
    ) -> ownership::scrutinee::Takes {
        let Some(tc) = &self.types else {
            return ownership::scrutinee::Takes::Nothing;
        };
        let tc = tc.borrow();
        // Asking is not translating: the questions this resolution defers are
        // reported once, where the match is written out.
        let mark = tc.sink.mark();
        let subject = tc.resolve_expr(subject_expr);
        tc.sink.rewind(mark);
        let Ok(subject) = subject else {
            return ownership::scrutinee::Takes::Nothing;
        };
        drop(tc);
        // `match &x { .. }` matches THROUGH a reference: Rust's match ergonomics
        // make every binding under it a borrow, and nothing can be moved out of
        // a `&` at all. The written `&` is the whole answer, and it is asked
        // here because the resolved type of `&x` does not always come back as a
        // reference — so an owned `b` matched as `match &b.low` was being
        // written as `intoMatch`, which hands the payload away and leaves the
        // enum moved inside a struct its owner still drops.
        if matches!(subject_expr, syn::Expr::Reference(_)) {
            return ownership::scrutinee::Takes::Nothing;
        }
        if !self.owns_place(subject_expr) {
            return ownership::scrutinee::Takes::Nothing;
        }
        let tc = self.types.as_ref().expect("just borrowed").borrow();
        ownership::scrutinee::takes(&tc.probe(), &subject, patterns, |path| {
            let mark = tc.sink.mark();
            let payload = tc.payload_of(path, Some(&subject));
            tc.sink.rewind(mark);
            // The NAMES travel with the types: a struct pattern pairs its
            // fields with the payload by name, and the two lists are not in the
            // same order (I2).
            payload.unwrap_or_default()
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
        self.claim_bindings_as(
            names,
            &|name| self.types.as_ref().and_then(|tc| tc.borrow().lookup(name)),
            ownership::Drops::Unknown,
            body,
        )
    }

    /// The same, told what each name's type is, and what to do with a name
    /// whose type nothing can say.
    ///
    /// For: `for ref item in owned_vec` binds a REFERENCE into the element the
    /// iterator handed out, and the loop still owns that element — Rust's
    /// `IntoIter` drops it at the end of the turn. The BINDING's own type is a
    /// `&T`, which owns nothing, so the loop asks under the element's type
    /// instead and the release lands on the name the loop wrote.
    ///
    /// `unresolved` is what a name whose type does not resolve owes. Most
    /// scopes owe `Drops::Unknown` — nothing, because nothing is known. A
    /// `Result` arm that BOUND the payload owes `Drops::Cascade`: the arm holds
    /// a value the side read out of the `Result` whatever its type turns out to
    /// be, `dropOwned` releases it by its runtime shape, and walking away from
    /// it left a `PropertyError` for the collector at four corpus sites.
    pub fn claim_bindings_as(
        &self,
        names: &[String],
        type_of: &dyn Fn(&str) -> Option<crate::ty::Ty>,
        unresolved: ownership::Drops,
        body: &[syn::Stmt],
    ) -> Vec<ownership::Owned> {
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
            let drops = match type_of(name) {
                Some(ty) => ownership::drops_of(&tc.borrow().probe(), &ty),
                None => unresolved,
            };
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
            // E15: a flag the arm never sets is dead; `ownership::wrap` has
            // already left it out of the release.
            if let Some(flag) = &value.flag {
                if ownership::sets_the_flag(&text, flag) {
                    declarations.push_str(&format!("let {} = false;\n", flag));
                }
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
