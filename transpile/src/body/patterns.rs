//! What a pattern asks of a value, and what it takes out of it.
//!
//! For: every `match` arm, every `let`, every `if let` and every closure
//! parameter is a pattern, and the port has to write two things for each — the
//! TEST that decides whether the arm runs, and the DECLARATIONS that give its
//! names values. Rust says both in one piece of syntax; TypeScript has neither
//! in one place, so both are written out here.
//!
//! The shape questions a caller asks of a pattern before writing either —
//! whether it can fail, and whether it binds anything — live next door in
//! `pat_shape.rs`.
//!
//! Two halves. The static renderers (`pat_static`, `pat_render`) write a
//! pattern as a destructuring, which is what a `let` and a closure parameter
//! need; `pattern_test` and `payload_parts` write the test and the bindings
//! apart, which is what an arm needs. They agree about one thing above all:
//! Rust's `_` takes NO name, and TypeScript's `_` is a variable called `_`.

use crate::name_map;
use super::unreadable_alternatives::unreadable_alternatives;

use super::{translate_lit, BodyTranslator};

impl BodyTranslator<'_> {
    // ── Pattern translation (static — no self_type needed) ──────────

    pub fn pat_static(pat: &syn::Pat) -> String {
        Self::pat_render(pat, &|name| name.to_string())
    }

    /// The same, with every name the pattern binds put through `rename`.
    ///
    /// A Rust shadow introduces a NEW variable, and JavaScript will not declare
    /// one name twice in a scope — so a shadowing binding is emitted under a
    /// fresh identifier. That was written for a `let` that binds ONE name;
    /// `let Some([queryId, ..]) = ..` binds several, and each of them shadows
    /// on its own. `core/src/reactor/subscription_state.ts` declared `queryId`
    /// beside the parameter of the same name, and the module would not load.
    pub fn pat_render(pat: &syn::Pat, rename: &dyn Fn(&str) -> String) -> String {
        let recur = |p: &syn::Pat| Self::pat_render(p, rename);
        match pat {
            syn::Pat::Ident(ident) => rename(&name_map::escape_reserved(
                &name_map::to_camel_case(&ident.ident.to_string()),
            )),
            // `(a, _, c)` destructures as `[a, , c]`: a hole is how JavaScript
            // says "skip this one", where `_` would declare a variable of that
            // name — and a second `_` beside it is a duplicate declaration.
            syn::Pat::Tuple(tuple) => {
                let parts: Vec<String> = tuple.elems.iter().map(|p| Self::pat_slot_with(p, rename)).collect();
                format!("[{}]", parts.join(", "))
            }
            syn::Pat::TupleStruct(ts) => {
                let parts: Vec<String> = ts.elems.iter().map(&recur).collect();
                parts.join(", ")
            }
            syn::Pat::Struct(s) => {
                let fields: Vec<String> = s.fields.iter().filter(|f| !Self::binds_nothing(&f.pat)).map(|f| {
                    let member = match &f.member {
                        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    let pat = recur(&f.pat);
                    if member == pat { member } else { format!("{}: {}", member, pat) }
                }).collect();
                format!("{{ {} }}", fields.join(", "))
            }
            syn::Pat::Wild(_) => rename("_"),
            syn::Pat::Lit(_) => "/* pat literal */".to_string(),
            syn::Pat::Path(path) => Self::path_static(&path.path),
            syn::Pat::Reference(r) => recur(&r.pat),
            syn::Pat::Type(t) => recur(&t.pat),
            syn::Pat::Or(or_pat) => {
                let parts: Vec<String> = or_pat.cases.iter().map(&recur).collect();
                parts.join(" | ")
            }
            syn::Pat::Slice(slice) => {
                let parts: Vec<String> = slice.elems.iter().map(|p| Self::pat_slot_with(p, rename)).collect();
                format!("[{}]", parts.join(", "))
            }
            syn::Pat::Rest(_) => "...".to_string(),
            _ => "/* unknown pat */".to_string(),
        }
    }

    /// One position of an array destructuring: the name the pattern binds, or
    /// nothing at all where it binds nothing. `[a, , c]` skips the middle
    /// element; `[a, _, c]` would declare a variable called `_`, and a second
    /// `_` beside it is what a JavaScript engine refuses.
    fn pat_slot_with(pat: &syn::Pat, rename: &dyn Fn(&str) -> String) -> String {
        if Self::binds_nothing(pat) { String::new() } else { Self::pat_render(pat, rename) }
    }
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
        members: impl Iterator<Item = (String, &'p syn::Pat)>,
    ) -> (Vec<String>, Vec<String>, String) {
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
            let (test, bind) = self.pattern_test(&place, pat);
            if test != "true" {
                tests.push(test);
            }
            nested.push_str(&bind);
        }
        (tests, names, nested)
    }

    /// How TypeScript asks whether a value matches a pattern, and what it writes
    /// to take the pattern's names out of it.
    ///
    /// `Some`/`None` test the nullable the port maps `Option` to, `Ok`/`Err` ask
    /// the `Result`, a variant asks the `Enum`, and a plain name always matches.
    pub(crate) fn pattern_test(&self, subject: &str, pat: &syn::Pat) -> (String, String) {
        match pat {
            syn::Pat::TupleStruct(ts) => {
                let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                // `Some` and `None` are decided by IDENTITY, not by the name: a
                // crate enum with a variant of that name is a different value,
                // and reading `State::Some(n)` as a null test made arm one run
                // for `State::Other`.
                let option = self.names_option_variant(&ts.path);
                match name.as_str() {
                    // The port writes an `Option<T>` as `T | null`, so the
                    // payload of a `Some` *is* the subject — which is why a
                    // pattern inside it tests against the same place, and
                    // `Some(true)` is a test and not only a binding.
                    "Some" if option => {
                        let Some(inner) = ts.elems.first() else {
                            return (format!("{} != null", subject), String::new());
                        };
                        // A pattern that only binds takes the whole payload in
                        // one declaration, so `Some((last, rest))` stays the
                        // array destructuring a reader of the port expects.
                        // `Some(Status::Requested(_, _))` is not one of those:
                        // it takes no name and still tests the variant, and the
                        // test used to go with the binding.
                        if Self::binds_nothing(inner) && Self::is_irrefutable(inner) {
                            return (format!("{} != null", subject), String::new());
                        }
                        if Self::is_irrefutable(inner) {
                            return (
                                format!("{} != null", subject),
                                format!("const {} = {};\n", Self::pat_static(inner), subject),
                            );
                        }
                        let (inner_test, bind) = self.pattern_test(subject, inner);
                        (format!("{} != null && ({})", subject, inner_test), bind)
                    }
                    // A `Result`'s payload is behind `unwrap`, which the runtime
                    // counts as a read, so it is written once — into the
                    // binding. A pattern that would have to be *tested* there
                    // cannot be, and is reported rather than dropped.
                    "Ok" | "Err" => {
                        let inner = ts.elems.first();
                        if let Some(pat) = inner.filter(|p| !Self::is_irrefutable(p)) {
                            self.fallback(
                                syn::spanned::Spanned::span(pat),
                                format!(
                                    "`{}` carries a pattern that has to be tested, and the port \
                                     reads a `Result`'s payload once, into the binding, so what \
                                     it tests is not tested",
                                    name
                                ),
                            );
                        }
                        // A pattern that only binds names the payload; one that
                        // tests has no name of its own, and writing the pattern
                        // where a name belongs emitted `const /* pat literal */
                        // = _v.unwrap();` and `const PropertyError.Missing =
                        // ..`, neither of which is a declaration.
                        let var = match inner {
                            // `Err(_)` still takes the payload out — that read
                            // is what releases it — but `_` is not a name it
                            // can be taken into.
                            Some(pat) if Self::binds_nothing(pat) => self.fresh_temp(),
                            Some(pat) if Self::is_irrefutable(pat) => Self::pat_static(pat),
                            Some(_) => self.fresh_temp(),
                            None => "v".to_string(),
                        };
                        // A borrowed `Result` is still its owner's: `match
                        // &r { Ok(v) => .. }` binds `v: &T` and leaves the
                        // wrapper whole. `unwrap()` is the `self` form and
                        // marks it moved, so a second read of the same value
                        // was `Result was used after being moved`.
                        let borrowed = self.matches_a_reference();
                        if name == "Ok" {
                            let read = if borrowed { "okRef" } else { "unwrap" };
                            (
                                format!("{}.isOk()", subject),
                                format!("const {} = {}.{}();\n", var, subject, read),
                            )
                        } else {
                            let read = if borrowed { "errRef" } else { "unwrapErr" };
                            (
                                format!("{}.isErr()", subject),
                                format!("const {} = {}.{}();\n", var, subject, read),
                            )
                        }
                    }
                    _ => {
                        let (payload_tests, names, nested) =
                            self.payload_parts(subject, ts.elems.iter().enumerate().map(|(i, p)| {
                                (format!("_{}", i), p)
                            }));
                        let mut test = format!("{}.is('{}')", subject, name);
                        for extra in payload_tests {
                            test.push_str(&format!(" && ({})", extra));
                        }
                        let mut bind = if names.is_empty() {
                            String::new()
                        } else {
                            format!("const {{ {} }} = {}.value;\n", names.join(", "), subject)
                        };
                        bind.push_str(&nested);
                        (test, bind)
                    }
                }
            }
            syn::Pat::Path(p) => {
                // An ordering is a number here, so an arm naming one of its
                // three values compares against that number rather than asking
                // an enum which variant it is.
                if let Some(number) = self.ordering_variant(&p.path) {
                    return (format!("{} === {}", subject, number), String::new());
                }
                let segments: Vec<String> =
                    p.path.segments.iter().map(|s| s.ident.to_string()).collect();
                if self.names_a_const(&segments).is_some() {
                    return self.const_pattern_test(subject, &segments, pat);
                }
                let name = p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                match name.as_str() {
                    "None" if self.names_option_variant(&p.path) => {
                        (format!("{} == null", subject), String::new())
                    }
                    _ => (format!("{}.is('{}')", subject, name), String::new()),
                }
            }
            syn::Pat::Struct(st) => {
                let name = st.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                let (payload_tests, names, nested) = self.payload_parts(
                    subject,
                    st.fields.iter().map(|f| {
                        let member = match &f.member {
                            syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                            syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                        };
                        (member, &*f.pat)
                    }),
                );
                let mut test = format!("{}.is('{}')", subject, name);
                for extra in payload_tests {
                    test.push_str(&format!(" && ({})", extra));
                }
                let mut bind = if names.is_empty() {
                    String::new()
                } else {
                    format!("const {{ {} }} = {}.value;\n", names.join(", "), subject)
                };
                bind.push_str(&nested);
                (test, bind)
            }
            // `(a, b)` tests each element against its own pattern and binds
            // through all of them; the port writes a Rust tuple as an array.
            // K16: each element carries its own borrowedness, or a `(&a, &b)`
            // of Results had both sides `unwrap()`ed.
            syn::Pat::Tuple(tuple) => {
                let mut tests = Vec::new();
                let mut binds = String::new();
                for (i, element) in tuple.elems.iter().enumerate() {
                    let (test, bind) = self.matching_element(i, || {
                        self.pattern_test(&format!("{}[{}]", subject, i), element)
                    });
                    if test != "true" {
                        tests.push(format!("({})", test));
                    }
                    binds.push_str(&bind);
                }
                let test = if tests.is_empty() { "true".to_string() } else { tests.join(" && ") };
                (test, binds)
            }
            // `A(x) | B(x)`. Rust makes every alternative bind the same names,
            // so where they also read them out of the same place — which two
            // variants of one enum do — the test is the disjunction and the
            // binding is what they agree on. Where they do not, which name came
            // from which alternative is a question this cannot answer, and it
            // says so rather than binding one of them.
            syn::Pat::Or(or) => {
                let mut tests = Vec::new();
                let mut binds: Vec<String> = Vec::new();
                for case in &or.cases {
                    let (test, bind) = self.pattern_test(subject, case);
                    tests.push(format!("({})", test));
                    binds.push(bind);
                }
                match binds.first() {
                    Some(first) if binds.iter().all(|b| b == first) => {
                        (tests.join(" || "), first.clone())
                    }
                    // The alternatives bind the same names from DIFFERENT
                    // places, which is what Rust's or-pattern is for:
                    // `(Expr::Path(p), _) | (_, Expr::Path(p))` binds `p` out
                    // of the left or out of the right, whichever matched. One
                    // test cannot say which, so the BINDING asks: each name is
                    // read from the alternative whose test passed.
                    // R12's wording is that the hole throws where the BRANCH
                    // would have run. Written as the test, it threw for every
                    // value the match was given — including the ones whose
                    // pattern does not match, which Rust answers with an empty
                    // `else`: core's `recursePredicateWatchers` refused every
                    // `Comparison` predicate rather than the one shape it
                    // cannot read. So the test is the honest disjunction and
                    // the refusal is the branch's first statement, which is
                    // what declaring each name from a hole already is.
                    _ => match per_alternative(&tests, &binds) {
                        Some(bind) => (tests.join(" || "), bind),
                        None => (tests.join(" || "), unreadable_alternatives(self, or)),
                    },
                }
            }
            // A plain name binds whatever it was given, and always matches —
            // except `None`, which syn hands over as an identifier because it
            // is written without a path, and which Rust resolves to `Option`'s
            // empty case rather than to a binding (binding it is an error, not
            // a shadow). Reading it as a name made every `None` arm a
            // catch-all that ran for a value that was there.
            syn::Pat::Ident(ident)
                if ident.ident == "None"
                    && ident.subpat.is_none()
                    && self.names_option_variant_by(&[ident.ident.to_string()]) =>
            {
                (format!("{} == null", subject), String::new())
            }
            // `n @ 1` and `whole @ Some(_)` bind AND ask: the subpattern is
            // the test, and the name is bound to the same subject. Ignoring the
            // subpattern made `Some(n @ 1) => n` an arm that matches every
            // `Some`, so `o == 7` took it.
            syn::Pat::Ident(ident) if ident.subpat.is_some() => {
                let (_, inner) = ident.subpat.as_ref().expect("just tested");
                let (test, mut bind) = self.pattern_test(subject, inner);
                let var = name_map::escape_reserved(&name_map::to_camel_case(
                    &ident.ident.to_string(),
                ));
                bind.insert_str(0, &format!("const {} = {};\n", var, subject));
                (test, bind)
            }
            // A name that resolves to a `const` is a PATH pattern, not a
            // binding: Rust compares the subject against the const's value.
            // Read as a binding, `BASE => ..` bound `bASE` and matched
            // everything, and the arms below it were reported as unreachable —
            // the only diagnostic named the wrong arm.
            syn::Pat::Ident(ident)
                if ident.subpat.is_none()
                    && ident.by_ref.is_none()
                    && self.names_a_const(&[ident.ident.to_string()]).is_some() =>
            {
                self.const_pattern_test(subject, &[ident.ident.to_string()], pat)
            }
            syn::Pat::Ident(_) => {
                let var = Self::pat_static(pat);
                ("true".to_string(), format!("const {} = {};\n", var, subject))
            }
            // `0 => ..` compares. Writing the pattern where the test belongs
            // emitted `if (/* pat literal */)`, which does not parse.
            syn::Pat::Lit(lit) => match translate_lit(&lit.lit) {
                Some(written) => (format!("{} === {}", subject, written), String::new()),
                None => {
                    self.fallback(
                        syn::spanned::Spanned::span(lit),
                        "this literal pattern has a form the port has no spelling for, so the \
                         arm is written as one that never matches",
                    );
                    ("false".to_string(), String::new())
                }
            },
            // `1..=9 => ..`. Rust's exclusive form stops one short of the end.
            syn::Pat::Range(range) => {
                let mut tests = Vec::new();
                if let Some(from) = &range.start {
                    tests.push(format!("{} >= {}", subject, self.expr(from)));
                }
                if let Some(to) = &range.end {
                    let op = match range.limits {
                        syn::RangeLimits::Closed(_) => "<=",
                        syn::RangeLimits::HalfOpen(_) => "<",
                    };
                    tests.push(format!("{} {} {}", subject, op, self.expr(to)));
                }
                let test = if tests.is_empty() { "true".to_string() } else { tests.join(" && ") };
                (test, String::new())
            }
            syn::Pat::Wild(_) => ("true".to_string(), String::new()),
            syn::Pat::Reference(r) => self.pattern_test(subject, &r.pat),
            syn::Pat::Paren(p) => self.pattern_test(subject, &p.pat),
            // `[a, b]` against a sequence: the length, and then each position
            // by its own rule. A `..` is the case this has no lowering for —
            // the positions after it are counted from the END, and the names
            // before and after it index differently — so that one falls to the
            // refusal below.
            syn::Pat::Slice(slice)
                if !slice.elems.iter().any(|p| matches!(p, syn::Pat::Rest(_))) =>
            {
                let mut tests = vec![format!("{}.length === {}", subject, slice.elems.len())];
                let mut binds = String::new();
                for (at, elem) in slice.elems.iter().enumerate() {
                    let place = format!("{}[{}]", subject, at);
                    let (test, bind) = self.pattern_test(&place, elem);
                    if test.trim() != "true" {
                        tests.push(test);
                    }
                    binds.push_str(&bind);
                }
                (tests.join(" && "), binds)
            }
            // A pattern with no test the translator can write is NOT a
            // catch-all, and it is not an arm that never matches either: the
            // arm's own bindings are written whatever the test says, so
            // `if (false)` named `a` and `b` where nothing declares them. D2:
            // the test is `true` and the refusal is the branch's first
            // statement, so every value that reaches the arm gets it, loudly.
            other => {
                let hole = self.hole(
                    syn::spanned::Spanned::span(other),
                    "this pattern has no test the translator can write",
                );
                ("true".to_string(), format!("{};\n", hole))
            }
        }
    }
}

/// One binding per name, read from whichever alternative of an or-pattern
/// matched.
///
/// `(Expr::Path(p), _) | (_, Expr::Path(p))` binds `p` out of the left element
/// or out of the right, and the arm's body names `p` once. The test is the
/// disjunction; the binding is a chain of conditionals over the same tests, in
/// the same order, so the name is read from the alternative Rust would have
/// taken. `None` where an alternative binds something this cannot read back —
/// a destructuring, a differing set of names — and the site says so.
fn per_alternative(tests: &[String], binds: &[String]) -> Option<String> {
    let parsed: Vec<Vec<(String, String)>> = binds.iter().map(|b| simple_bindings(b)).collect::<Option<Vec<_>>>()?;
    let first = parsed.first()?;
    if first.is_empty() {
        return None;
    }
    // Rust requires every alternative to bind the same SET of names; it says
    // nothing about the ORDER, and the whole point of an or-pattern is that the
    // same name comes out of a different place in each alternative.
    // `(Expr::Path(path), Expr::Literal(literal)) | (Expr::Literal(literal),
    // Expr::Path(path))` — core's `watcherset.rs:171` — binds `path` and
    // `literal` in opposite positions, and comparing by position refused it. So
    // each name is looked up BY NAME in every alternative, and an alternative
    // that does not carry one of them is a set that differs, which means this
    // did not read them properly.
    for other in &parsed {
        if other.len() != first.len() {
            return None;
        }
        if first.iter().any(|(name, _)| !other.iter().any(|(other_name, _)| other_name == name)) {
            return None;
        }
    }
    let place_in = |alternative: &Vec<(String, String)>, name: &str| -> Option<String> {
        alternative.iter().find(|(bound, _)| bound == name).map(|(_, place)| place.clone())
    };
    let mut out = String::new();
    for (name, _) in first {
        let mut written = "undefined".to_string();
        for (alternative, test) in parsed.iter().zip(tests).rev() {
            written = format!("({}) ? {} : {}", test, place_in(alternative, name)?, written);
        }
        out.push_str(&format!("const {} = {};\n", name, written));
    }
    Some(out)
}

/// What a pattern's binding text names, as `(name, the expression it reads)`
/// pairs — or `None` for a form this cannot read back.
///
/// The two forms the pattern machinery writes: `const x = e;`, and the payload
/// destructuring `const { _0: x, name: y } = e;`. The second is turned back
/// into member reads, which is what a conditional over two alternatives needs:
/// there is no destructuring that reads from one place or another.
fn simple_bindings(text: &str) -> Option<Vec<(String, String)>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let rest = line.strip_prefix("const ")?;
        let (bound, value) = rest.split_once(" = ")?;
        let value = value.trim_end_matches(';').trim();
        match bound.strip_prefix('{').and_then(|b| b.strip_suffix('}')) {
            Some(fields) => {
                for field in fields.split(',') {
                    let field = field.trim();
                    if field.is_empty() {
                        continue;
                    }
                    let (key, name) = match field.split_once(':') {
                        Some((key, name)) => (key.trim(), name.trim()),
                        None => (field, field),
                    };
                    if !is_identifier(key) || !is_identifier(name) {
                        return None;
                    }
                    out.push((name.to_string(), format!("{}.{}", value, key)));
                }
            }
            None => {
                if !is_identifier(bound) {
                    return None;
                }
                out.push((bound.to_string(), value.to_string()));
            }
        }
    }
    Some(out)
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}
