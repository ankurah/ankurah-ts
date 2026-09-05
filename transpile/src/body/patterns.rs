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
            if Self::binds_nothing(pat) {
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
                match name.as_str() {
                    // The port writes an `Option<T>` as `T | null`, so the
                    // payload of a `Some` *is* the subject — which is why a
                    // pattern inside it tests against the same place, and
                    // `Some(true)` is a test and not only a binding.
                    "Some" => {
                        let Some(inner) = ts.elems.first() else {
                            return (format!("{} != null", subject), String::new());
                        };
                        // A pattern that only binds takes the whole payload in
                        // one declaration, so `Some((last, rest))` stays the
                        // array destructuring a reader of the port expects.
                        if Self::binds_nothing(inner) {
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
                        if name == "Ok" {
                            (
                                format!("{}.isOk()", subject),
                                format!("const {} = {}.unwrap();\n", var, subject),
                            )
                        } else {
                            (
                                format!("{}.isErr()", subject),
                                format!("const {} = {}.unwrapErr();\n", var, subject),
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
                let name = p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                match name.as_str() {
                    "None" => (format!("{} == null", subject), String::new()),
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
                    _ => {
                        self.fallback(
                            syn::spanned::Spanned::span(or),
                            "the alternatives of this pattern bind their names from \
                             different places, which the translator cannot write as one test, \
                             so the arm is written as one that never matches",
                        );
                        // NOT `true`: an arm whose test cannot be written and
                        // which is taken anyway runs a body naming the very
                        // bindings the alternatives disagreed about. ankql's
                        // `(Expr::Path(path), _) | (_, Expr::Path(path))` came
                        // out as an unconditional `return columns.includes(
                        // path.property())` with `path` bound nowhere, so the
                        // suite died on a ReferenceError instead of on the
                        // engine's own report.
                        ("false".to_string(), String::new())
                    }
                }
            }
            // A plain name binds whatever it was given, and always matches —
            // except `None`, which syn hands over as an identifier because it
            // is written without a path, and which Rust resolves to `Option`'s
            // empty case rather than to a binding (binding it is an error, not
            // a shadow). Reading it as a name made every `None` arm a
            // catch-all that ran for a value that was there.
            syn::Pat::Ident(ident) if ident.ident == "None" && ident.subpat.is_none() => {
                (format!("{} == null", subject), String::new())
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
            other => {
                self.fallback(
                    syn::spanned::Spanned::span(other),
                    "this pattern has no test the translator can write, so the loop runs unconditionally",
                );
                ("true".to_string(), String::new())
            }
        }
    }
}
