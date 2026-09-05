//! What a pattern asks of a value, and what it takes out of it.
//!
//! For: every `match` arm, every `let`, every `if let` and every closure
//! parameter is a pattern, and the port has to write two things for each — the
//! TEST that decides whether the arm runs, and the DECLARATIONS that give its
//! names values. Rust says both in one piece of syntax; TypeScript has neither
//! in one place, so both are written out here.
//!
//! Two halves. The static renderers (`pat_static`, `pat_render`) write a
//! pattern as a destructuring, which is what a `let` and a closure parameter
//! need; `pattern_test` and `payload_parts` write the test and the bindings
//! apart, which is what an arm needs. They agree about one thing above all:
//! Rust's `_` takes NO name, and TypeScript's `_` is a variable called `_`.

use crate::name_map;

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


    /// Does this pattern match whatever it is given?
    ///
    /// A name and a `_` take the value and always match; every other pattern
    /// asks a question of it. Callers use the answer to decide whether a
    /// position can be written as a binding alone, or needs a test written
    /// beside it so that the question still gets asked.
    pub(crate) fn is_irrefutable(pat: &syn::Pat) -> bool {
        match pat {
            // `None` is not a binding. syn hands it over as an identifier
            // because it is written without a path, and Rust resolves it to
            // `Option`'s empty case — binding it is an error, not a shadow.
            // `pattern_test` was given this exception and this was not, so a
            // `None` NESTED in any pattern — `Some(None)`, `E::Opt(None)` —
            // was read as a name that matches everything, and the arm ran for
            // a value that was there.
            syn::Pat::Ident(ident) if ident.ident == "None" && ident.subpat.is_none() => false,
            // `x @ Some(_)` binds *and* asks.
            syn::Pat::Ident(ident) => ident
                .subpat
                .as_ref()
                .map(|(_, inner)| Self::is_irrefutable(inner))
                .unwrap_or(true),
            syn::Pat::Wild(_) => true,
            syn::Pat::Reference(r) => Self::is_irrefutable(&r.pat),
            syn::Pat::Paren(p) => Self::is_irrefutable(&p.pat),
            syn::Pat::Type(t) => Self::is_irrefutable(&t.pat),
            syn::Pat::Tuple(t) => t.elems.iter().all(Self::is_irrefutable),
            _ => false,
        }
    }

    /// Does this pattern take no name out of the value at all?
    ///
    /// Rust's `_` is not a name: it says "there is a value here and I want
    /// nothing from it", and two of them in one pattern are two nothings.
    /// TypeScript has no such spelling — writing `_` there declares a variable
    /// called `_`, so `(Some(_), None)` emitted two `const _` in one block and
    /// `Comparison { left, operator: _, right: _ }` two `_` keys, and a
    /// JavaScript engine refuses the whole module. Every caller that would
    /// write a name asks this first and writes nothing instead.
    pub(crate) fn binds_nothing(pat: &syn::Pat) -> bool {
        match pat {
            syn::Pat::Wild(_) => true,
            syn::Pat::Reference(r) => Self::binds_nothing(&r.pat),
            syn::Pat::Paren(p) => Self::binds_nothing(&p.pat),
            syn::Pat::Type(t) => Self::binds_nothing(&t.pat),
            syn::Pat::Tuple(t) => t.elems.iter().all(Self::binds_nothing),
            syn::Pat::Slice(sl) => sl.elems.iter().all(Self::binds_nothing),
            syn::Pat::Struct(st) => st.fields.iter().all(|f| Self::binds_nothing(&f.pat)),
            syn::Pat::TupleStruct(ts) => ts.elems.iter().all(Self::binds_nothing),
            _ => false,
        }
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
            syn::Pat::Tuple(tuple) => {
                let mut tests = Vec::new();
                let mut binds = String::new();
                for (i, element) in tuple.elems.iter().enumerate() {
                    let (test, bind) = self.pattern_test(&format!("{}[{}]", subject, i), element);
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
                    _ => match per_alternative(&tests, &binds) {
                        Some(bind) => (tests.join(" || "), bind),
                        None => unreadable_alternatives(self, or),
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
            // A pattern with no test the translator can write is NOT a
            // catch-all. `true` here was the opposite convention from the
            // or-pattern's `false` a hundred lines above, and it ran an arm
            // whose bindings the translator had just said it could not write.
            other => {
                self.fallback(
                    syn::spanned::Spanned::span(other),
                    "this pattern has no test the translator can write, so the arm is written \
                     as one that never matches",
                );
                ("false".to_string(), String::new())
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
    // Every alternative binds the same names, in the same order: Rust requires
    // it, and a set that differs here means this did not read them properly.
    for other in &parsed {
        if other.len() != first.len() {
            return None;
        }
        if other.iter().zip(first).any(|((a, _), (b, _))| a != b) {
            return None;
        }
    }
    let mut out = String::new();
    for (index, (name, _)) in first.iter().enumerate() {
        let mut written = "undefined".to_string();
        for (alternative, test) in parsed.iter().zip(tests).rev() {
            written = format!("({}) ? {} : {}", test, alternative[index].1, written);
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

impl<'a> BodyTranslator<'a> {
    /// Does this path name a `const` or a `static`, rather than a binding?
    ///
    /// Rust resolves a pattern's identifier in the VALUE namespace first: a
    /// name that lands on a const is a comparison against its value, and only a
    /// name that lands on nothing binds. The registry's value namespace holds
    /// consts, statics and free functions; a function has a signature and a
    /// const does not, which is what tells them apart.
    ///
    /// The answer carries the const's declared type, because that is what
    /// decides how the comparison is written.
    pub(crate) fn names_a_const(&self, segments: &[String]) -> Option<Option<crate::ty::Ty>> {
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let mark = tc.sink.mark();
        let found = tc
            .registry
            .lookup(tc.module, crate::registry::Ns::Value, segments);
        tc.sink.rewind(mark);
        match found {
            Ok(Some(crate::registry::Def::Value(id))) => {
                let value = tc.registry.value(id)?;
                // A free function is in the value namespace too, and naming one
                // in a pattern is not a comparison.
                if value.sig.is_some() {
                    return None;
                }
                Some(value.ty.clone())
            }
            _ => None,
        }
    }

    /// Is every use of this name a fresh value, so the emitted name is a
    /// function this use calls? See `ValueDef::fresh_at_each_use`.
    pub(crate) fn names_a_fresh_const(&self, segments: &[String]) -> bool {
        let Some(tc) = self.types.as_ref() else { return false };
        let tc = tc.borrow();
        let mark = tc.sink.mark();
        let found = tc
            .registry
            .lookup(tc.module, crate::registry::Ns::Value, segments);
        tc.sink.rewind(mark);
        match found {
            Ok(Some(crate::registry::Def::Value(id))) => tc
                .registry
                .value(id)
                .is_some_and(|value| value.fresh_at_each_use),
            _ => false,
        }
    }

    /// The test a const pattern writes: the subject against the const's value.
    fn const_pattern_test(
        &self,
        subject: &str,
        segments: &[String],
        pat: &syn::Pat,
    ) -> (String, String) {
        let name = crate::name_map::escape_reserved(segments.last().expect("a path has a segment"));
        let ty = self.names_a_const(segments).flatten();
        let compares_by_identity = matches!(
            ty.as_ref().map(|t| t.peel_refs()),
            Some(crate::ty::Ty::Prim(_)) | Some(crate::ty::Ty::Str) | None
        );
        if compares_by_identity {
            return (format!("{} === {}", subject, name), String::new());
        }
        // A const of a type the port writes as an object compares by value in
        // Rust, and `===` here is reference identity. R12: the arm says so and
        // stops rather than answering what Rust would not.
        let hole = self.hole(
            syn::spanned::Spanned::span(pat),
            format!(
                "`{}` is a const of a type the port compares by identity, and Rust compares a \
                 const pattern by value",
                segments.join("::")
            ),
        );
        (hole, String::new())
    }
}


/// An or-pattern whose alternatives bind their names in a form the translator
/// cannot read back, as the R12 hole it is.
///
/// PREMISE CHANGED 2026-09-05 (fixpass4 item 6): what stood here was `false` —
/// an arm written as one that never matches. That is a wrong answer twice over.
/// The branch is SKIPPED, so the program carries on as though the pattern had
/// not matched (core's `watcherset.ts` never registered an index watcher), and
/// the skipped branch still carried its own releases, naming bindings nothing
/// declared: `if (false) { .. } finally { literal.drop() }` is a
/// `ReferenceError` waiting for the day the test stops being `false`.
///
/// R12: the test is the hole, so reaching the branch reports what the port
/// cannot do; and the names the branch's body reads are declared from a hole
/// too, so the emitted text is still one a JavaScript engine loads and
/// TypeScript types (`unsupported` answers `never`).
fn unreadable_alternatives(t: &BodyTranslator, or: &syn::PatOr) -> (String, String) {
    let what = "the alternatives of this pattern bind their names in a form the translator \
                cannot read back — each alternative has to bind the same names, one `const` \
                apiece — so this branch is a hole";
    t.fallback(syn::spanned::Spanned::span(or), what);
    let hole = crate::body::hole_text(what);
    let mut declared: Vec<String> = Vec::new();
    let mut bind = String::new();
    for case in &or.cases {
        for name in crate::body::pattern_names(case) {
            if declared.contains(&name) {
                continue;
            }
            bind.push_str(&format!("const {} = {};\n", name, hole));
            declared.push(name);
        }
    }
    (hole, bind)
}

#[cfg(test)]
mod const_pattern_tests {
    use crate::testing::Fixture;

    /// A const pattern binds NOTHING: Rust compares the subject against the
    /// const's value. Read as a binding, the arm owned a value nothing
    /// declared — `match p { ORIGIN => true, _ => false }` released `oRIGIN`,
    /// an identifier the emitted file never introduces, and the arm the hole
    /// had replaced still carried it.
    #[test]
    fn a_const_pattern_binds_nothing_and_releases_nothing() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Point { pub x: i32 }\n             impl Drop for Point { fn drop(&mut self) {} }\n             pub const ORIGIN: Point = Point { x: 0 };\n             pub fn at_origin(p: Point) -> bool { match p { ORIGIN => true, _ => false } }",
        )]);
        let ts = f.translated_method("lib.rs", "at_origin");
        assert!(ts.contains("if (unsupported("), "the test is the hole:\n{ts}");
        assert!(!ts.contains("oRIGIN"), "and it declares no binding:\n{ts}");
        // The subject is still the body's, released where nothing took it.
        assert!(ts.contains("p.drop()"), "{ts}");
    }

    /// The same for a const the port compares by value, which is not a hole:
    /// no binding there either, and the arms below it are reachable.
    #[test]
    fn a_primitive_const_pattern_is_a_comparison() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub const LIMIT: i32 = 5;\n             pub fn at_limit(n: i32) -> bool { match n { LIMIT => true, _ => false } }",
        )]);
        let ts = f.translated_method("lib.rs", "at_limit");
        assert!(ts.contains("n === LIMIT"), "{ts}");
        assert!(!ts.contains("const lIMIT"), "{ts}");
    }
}

#[cfg(test)]
mod or_pattern_tests {
    use crate::testing::Fixture;

    /// PREMISE CHANGED 2026-09-05 (fixpass4 item 6): an or-pattern the
    /// translator cannot read back used to be written as a branch that never
    /// matches. That is a wrong answer twice: the program carried on as though
    /// the pattern had not matched — core's `watcherset.ts` never registered an
    /// index watcher — and the skipped branch still carried its own releases,
    /// naming bindings nothing declared.
    #[test]
    fn an_or_pattern_the_translator_cannot_read_back_is_a_hole() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Lit { pub n: u32 }\n\
             pub struct Path { pub s: String }\n\
             pub enum Side { Literal(Lit), Property(Path) }\n\
             pub fn pair(left: Side, right: Side) -> u32 {\n\
               if let (Side::Property(p), Side::Literal(l)) | (Side::Literal(l), Side::Property(p)) = (left, right) {\n\
                 l.n\n\
               } else {\n\
                 0\n\
               }\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "pair");
        assert!(ts.contains("if (unsupported("), "the test is the hole:\n{}", ts);
        // and every name the branch reads is declared, from a hole, so what is
        // emitted is still text a JavaScript engine loads.
        assert!(ts.contains("const l = unsupported("), "{}", ts);
        assert!(ts.contains("const p = unsupported("), "{}", ts);
        assert!(!ts.contains("if (false)"), "{}", ts);
        assert!(
            f.messages().iter().any(|m| m.contains("cannot read back")),
            "and it says why: {:?}",
            f.messages()
        );
    }
}
