//! What a pattern nested inside a payload is looking at.
//!
//! For: the test a pattern writes depends on what the value IS —
//! `subject.isOk()` for the runtime's `Result`, `subject.is('V')` for one of
//! the port's enums, a plain destructuring for a struct. The outer pattern
//! knows its own subject's type, and until this file the nested one inherited
//! it: `if let Duo::A(Token { n }) = value` asked `Duo`'s question of a
//! `Token`, and wrote `v._0.is('Token')` — a method no struct has, on a value
//! nothing then released.
//!
//! So a pattern that descends into a payload member carries the member's own
//! declared type down with it, and the questions below are asked of that.
//! Match ergonomics travel with it: a member reached THROUGH a reference is
//! itself borrowed, which is what keeps `match &d { Duo::A(Ok(v)) => .. }`
//! reading `okRef()` rather than the `unwrap()` that takes the wrapper.

use crate::ty::Ty;

impl crate::body::BodyTranslator<'_> {
    /// The declared types of the members a variant or struct pattern names,
    /// paired with the member names `payload_parts` writes.
    ///
    /// `None` where the engine cannot say — an unresolved subject, a path it
    /// cannot place — and every caller reads that as "carry on as before",
    /// because narrowing on a type the engine failed to read would move working
    /// sites for a reason that has nothing to do with what they match.
    pub(crate) fn payload_member_types(&self, path: &syn::Path) -> Option<Vec<(String, Ty)>> {
        let tc = self.types.as_ref()?;
        let subject = self.subject_ty.borrow().clone()?;
        let tc = tc.borrow();
        // Asking is not translating: the resolution files what it deferred, and
        // the pattern's own gaps are reported where the pattern is written.
        let mark = tc.sink.mark();
        let found = tc.payload_of(path, Some(&subject));
        tc.sink.rewind(mark);
        let borrowed = matches!(subject, Ty::Ref { .. });
        Some(
            found?
                .into_iter()
                .map(|(name, ty)| match borrowed {
                    true => (name, Ty::Ref { mutable: false, inner: Box::new(ty) }),
                    false => (name, ty),
                })
                .collect(),
        )
    }

    /// Is the value this pattern is looking at the runtime's `Result`?
    ///
    /// A subject the engine could not type answers YES, which is where this
    /// question stood before it was asked at all: every `Ok`/`Err` pattern was
    /// opened with `isOk()`/`unwrap()` on the strength of its spelling.
    pub(crate) fn matches_a_result(&self) -> bool {
        match &*self.subject_ty.borrow() {
            Some(ty) => self.is_result(ty),
            None => true,
        }
    }

    /// Does this pattern's path name a plain STRUCT — a class with its fields
    /// on it, and none of `is`, `value` or `intoMatch`?
    ///
    /// Asked of the PATH, not of the value the pattern is looking at, and that
    /// is the load-bearing part: `NodeResponseBody::CommitComplete { .. }` is a
    /// struct-shaped pattern whose path names a VARIANT, and answering it from
    /// the subject's type made every such arm an unconditional match — two arms
    /// of `core/node.rs`'s relay loop stopped being written at all.
    ///
    /// A variant path resolves to no type; a struct's does.
    pub(crate) fn pattern_names_a_struct(&self, path: &syn::Path) -> bool {
        let Some(tc) = &self.types else { return false };
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let tc = tc.borrow();
        // Asking is not translating: what the lookup defers is reported where
        // the pattern is written.
        let mark = tc.sink.mark();
        let found = tc.registry.lookup(tc.module, crate::registry::Ns::Type, &segments);
        tc.sink.rewind(mark);
        let Ok(Some(crate::registry::Def::Type(id))) = found else { return false };
        matches!(
            tc.registry.def(id).map(|def| &def.kind),
            Some(crate::registry::TypeKind::Struct)
        )
    }
}

impl crate::body::BodyTranslator<'_> {
    /// `Token { n }` against a value that IS a `Token`: the test its nested
    /// patterns amount to, and the destructuring that binds its names.
    ///
    /// The fields stand on the object itself, so there is no `is` to ask and no
    /// `.value` to reach through — which is the whole difference between this
    /// and the variant form next door in `payload_parts`.
    pub(crate) fn struct_fields_of(&self, subject: &str, st: &syn::PatStruct) -> (String, String) {
        let declared = self.payload_member_types(&st.path);
        let mut tests: Vec<String> = Vec::new();
        let mut names: Vec<String> = Vec::new();
        let mut nested = String::new();
        for field in &st.fields {
            let member = match &field.member {
                syn::Member::Named(ident) => {
                    crate::name_map::to_camel_case(&ident.to_string())
                }
                syn::Member::Unnamed(idx) => format!("_{}", idx.index),
            };
            let pat = &*field.pat;
            // Asking nothing and binding nothing is written as nothing; a name
            // for the whole field is a destructuring; anything else is a test
            // made against the field's own place.
            if Self::binds_nothing(pat) && Self::is_irrefutable(pat) {
                continue;
            }
            if Self::is_irrefutable(pat) {
                let local = Self::pat_static(pat);
                names.push(match local == member {
                    true => member,
                    false => format!("{}: {}", member, local),
                });
                continue;
            }
            let place = format!("{}.{}", subject, member);
            let member_ty = member_type(declared.as_deref(), &member);
            let (test, bind) =
                self.matching(member_ty.as_ref(), || self.pattern_test(&place, pat));
            if test != "true" {
                tests.push(format!("({})", test));
            }
            nested.push_str(&bind);
        }
        let test = match tests.is_empty() {
            true => "true".to_string(),
            false => tests.join(" && "),
        };
        let mut bind = match names.is_empty() {
            true => String::new(),
            false => format!("const {{ {} }} = {};\n", names.join(", "), subject),
        };
        bind.push_str(&nested);
        (test, bind)
    }
}

/// The declared type of one member, found by the name the EMISSION writes.
///
/// The registry keeps a field under the name Rust declared it with and a tuple
/// member under the `_0` the emission writes, so both spellings are tried:
/// `expired_at` in the registry is `expiredAt` in the pattern's member list.
pub(crate) fn member_type(declared: Option<&[(String, Ty)]>, member: &str) -> Option<Ty> {
    declared?
        .iter()
        .find(|(name, _)| {
            name == member || crate::name_map::to_camel_case(name) == member
        })
        .map(|(_, ty)| ty.clone())
}

impl crate::body::BodyTranslator<'_> {
    /// Does this type DECLARE variants in Rust — whatever the runtime writes it
    /// as?
    ///
    /// `is_an_enum` asks the emission's question ("does the class carry `is`,
    /// `value` and `intoMatch`") and answers no for both a struct and base's
    /// `MapEntry`. Telling those two apart is what routing needs: a tuple takes
    /// its payload apart through the match writer perfectly well, and a Rust
    /// enum the runtime writes as a plain class has no form there at all.
    /// The same question, less the two the port does not write as classes at
    /// all: `Result` and `Option` are declared as Rust enums in the std surface
    /// and written as the runtime's wrapper and as `T | null`, so an `Ok` arm
    /// on either of them is the wrapper's, not a crate's own variant.
    pub(crate) fn declares_its_own_variants(&self, ty: &Ty) -> bool {
        self.declares_variants(ty) && !self.is_result(ty) && !self.is_nullable(ty)
    }

    pub(crate) fn declares_variants(&self, ty: &Ty) -> bool {
        let Some(tc) = &self.types else { return false };
        let Ty::Named { id, .. } = ty.peel_refs() else { return false };
        let tc = tc.borrow();
        matches!(
            tc.probe().reg.def(*id).map(|def| &def.kind),
            Some(crate::registry::TypeKind::Enum { .. })
        )
    }
}

impl crate::body::BodyTranslator<'_> {
    /// What a variant's payload contributes to the arm: the tests its members
    /// ask, the names the destructuring takes out of `subject.value`, and the
    /// bindings that live inside a member which asks a question of its own.
    ///
    /// A member that only binds is a name in the destructuring, which is what
    /// the port has always written. A member that tests — `Expr::Literal(
    /// Literal::String(s))`, `Op::Eq(0, b)` — needs its test written against the
    /// place the value sits in, or the arm runs for values it does not match;
    /// its own names then come out of that place rather than out of the
    /// destructuring, so they arrive here as statements instead.
    pub(crate) fn payload_parts<'p>(
        &self,
        subject: &str,
        path: &syn::Path,
        members: impl Iterator<Item = (String, &'p syn::Pat)>,
    ) -> (Vec<String>, Vec<String>, String) {
        // The member's own declared type travels with the pattern that reaches
        // into it, so the nested pattern asks its questions of what it is
        // actually looking at rather than of the value one level up.
        let declared = self.payload_member_types(path);
        let mut tests = Vec::new();
        let mut names = Vec::new();
        let mut nested = String::new();
        for (member, pat) in members {
            // `Comparison { left, operator: _, .. }` asks nothing of `operator`
            // and takes no name out of it, so the destructuring does not name
            // it either.
            //
            // Binding nothing is not the same as asking nothing:
            // `Wrap::Inner(Status::Requested(_, _))` takes no name and still
            // tests the variant. Skipped on the binding alone, the TEST went
            // with it and the arm ran for every `Wrap::Inner` — live in core's
            // `client_relay`.
            if Self::binds_nothing(pat) && Self::is_irrefutable(pat) {
                continue;
            }
            if Self::is_irrefutable(pat) {
                let local = Self::pat_static(pat);
                names.push(if local == member { member } else { format!("{}: {}", member, local) });
                continue;
            }
            let place = format!("{}.value.{}", subject, member);
            let member_ty = super::pattern_members::member_type(declared.as_deref(), &member);
            let (test, bind) =
                self.matching(member_ty.as_ref(), || self.pattern_test(&place, pat));
            if test != "true" {
                tests.push(test);
            }
            nested.push_str(&bind);
        }
        (tests, names, nested)
    }
}
