//! Where the emitter decides what the emitted TypeScript releases.
//!
//! Body translation asks two different questions of every expression: what does
//! this say in TypeScript, and what does it do to the values in scope. The
//! first is `body.rs`; this is the second, and it is here rather than beside
//! the syntax so that "who releases this" has one place to be answered and one
//! place to be read.
//!
//! Everything here hangs off `BodyTranslator`, because the answers need what it
//! holds — the type context to say what a value is, the diagnostic sink to say
//! where the emitter could not tell, and the scope state below to remember what
//! the block still owes.

use crate::body::{indent, BodyTranslator};
use crate::name_map;
use crate::native_types;
use crate::ownership;

/// What the block now being translated owes, and what it has lifted out of the
/// statement it is on.
///
/// One value per body: `BodyTranslator` holds it and every method in this file
/// reads or writes it. It is separate from the translator's own state because
/// none of it is about what the TypeScript *says* — it is about what the
/// TypeScript has to release before it is allowed to leave a scope.
#[derive(Default)]
pub struct Lowering {
    /// The locals the statement now being translated binds and owes a release
    /// for. The block reads it to decide what its `finally` says.
    pub pending: std::cell::RefCell<Vec<ownership::Owned>>,
    /// Declarations lifted out of the statement now being translated, in order.
    /// A temporary and a `?` both produce one: a line that has to stand before
    /// the statement that needs it.
    pub prelude: std::cell::RefCell<Vec<ownership::Hoist>>,
    /// What the block now being translated decided about each of its locals,
    /// for the statement now being translated.
    pub stmt_dispositions:
        std::cell::RefCell<std::collections::HashMap<String, ownership::Disposition>>,
    /// The locals whose release is behind a drop flag, by the name Rust wrote,
    /// and the flag's identifier. Nested blocks read it: a move inside a branch
    /// sets the flag the enclosing block's `finally` tests.
    pub flags: std::cell::RefCell<std::collections::HashMap<String, String>>,
    /// How many temporaries and flags this body has taken, so two of them never
    /// share a name.
    pub hoisted: std::cell::Cell<usize>,
    /// The locals bound to an `OwnedClosure`. A closure that owns its captures
    /// is not a bare callable — it is invoked as `f.call(x)`, and this is how
    /// the call sites the emitter can see are written that way.
    pub owned_closure_locals: std::cell::RefCell<Vec<String>>,
}

impl<'a> BodyTranslator<'a> {
    /// Ask the engine something the emitter needs and the source did not write.
    ///
    /// A resolution files whatever it could not settle, and the expression that
    /// was written reports that gap for itself where it is emitted. An
    /// ownership decision asks about the same expression a second time, so the
    /// record is wound back and the same gap is not counted twice.
    pub(crate) fn quietly<T>(&self, ask: impl FnOnce() -> T) -> T {
        let Some(tc) = &self.types else { return ask() };
        let mark = tc.borrow().sink.mark();
        let answer = ask();
        tc.borrow().sink.rewind(mark);
        answer
    }

    /// Which by-value parameters the function still owns when it returns, and
    /// under what condition.
    ///
    /// A parameter is one of the body's owned values, exactly as a local is: it
    /// is released in the outermost `finally` where nothing took it, released
    /// behind a drop flag where a branch took it, and released nowhere where
    /// every path hands it on. Reading only "was it ever moved" suppressed the
    /// release on the paths that did not move it.
    pub(crate) fn claim_params(
        &self,
        block: &syn::Block,
        params: &[(String, crate::ty::Ty)],
    ) -> Vec<ownership::Owned> {
        let Some(tc) = &self.types else {
            return Vec::new();
        };
        let names: Vec<String> = params.iter().map(|(name, _)| name.clone()).collect();
        let scan = ownership::Scan::new(self);
        // Every site in the body stands after the parameter list, so the
        // parameters are the declaration each one is attributed to.
        let sites = scan.block(&block.stmts).into_iter().map(|site| (1, site));
        let dispositions = ownership::Dispositions::build(&[(0, names)], sites.collect());
        let mut owned = Vec::new();
        for (name, ty) in params {
            let drops = ownership::drops_of(&tc.borrow().probe(), ty);
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

    /// Which of this block's locals were handed to somebody else before it
    /// ended, and where. Everything the block owes turns on it.
    pub(crate) fn analyse_moves(&self, stmts: &[syn::Stmt]) -> ownership::Dispositions {
        let scan = ownership::Scan::new(self);
        let mut declarations: Vec<(usize, Vec<String>)> = Vec::new();
        // One walk over the whole block, not one per statement: whether a
        // straight-line move is on every path depends on what stands above it.
        let sites = scan.block_indexed(stmts);
        for (index, stmt) in stmts.iter().enumerate() {
            if let syn::Stmt::Local(local) = stmt {
                declarations.push((index, crate::body::pattern_names(&local.pat)));
            }
        }
        let dispositions = ownership::Dispositions::build(&declarations, sites);
        // A capture is not reported here: the closure that took it is what
        // decides whether anything releases it, and it says so where it is
        // written out.
        for site in &dispositions.unwritable {
            self.fallback(
                site.span,
                format!(
                    "`{}` is moved where no drop flag can be written, so it is left unreleased",
                    site.name
                ),
            );
        }
        dispositions
    }

    /// Publish, for the statement about to be translated, what its block
    /// decided about each name it binds.
    pub(crate) fn set_stmt_dispositions(
        &self,
        stmt: &syn::Stmt,
        dispositions: &ownership::Dispositions,
        ordinals: &std::cell::RefCell<std::collections::HashMap<String, usize>>,
    ) {
        let mut current = self.own.stmt_dispositions.borrow_mut();
        current.clear();
        let syn::Stmt::Local(local) = stmt else { return };
        for name in crate::body::pattern_names(&local.pat) {
            let mut seen = ordinals.borrow_mut();
            let ordinal = seen.entry(name.clone()).or_insert(0);
            *ordinal += 1;
            current.insert(name.clone(), dispositions.of(&name, *ordinal));
        }
    }

    /// `_moved_x = true;` for each flagged local this statement hands away
    /// itself. A move written inside a nested block belongs to that block,
    /// which writes the same line as one of its own statements.
    pub(crate) fn flag_sets(&self, stmt: &syn::Stmt) -> String {
        if self.own.flags.borrow().is_empty() {
            return String::new();
        }
        let scan = ownership::Scan::new(self);
        let mut out = String::new();
        let mut written: Vec<String> = Vec::new();
        for site in scan.shallow(stmt) {
            let Some(flag) = self.own.flags.borrow().get(&site.name).cloned() else {
                continue;
            };
            if written.contains(&flag) {
                continue;
            }
            written.push(flag.clone());
            out.push_str(&format!("{} = true;\n", flag));
        }
        out
    }

    /// The flag assignments an expression owes, for a position that is not a
    /// statement — a match arm's body, which the arm renders as a block.
    pub fn flag_sets_for(&self, expr: &syn::Expr) -> String {
        self.flag_sets(&syn::Stmt::Expr(expr.clone(), None))
    }

    /// Take a name nothing else will use.
    pub(crate) fn fresh_hoist(&self, prefix: &str) -> String {
        let n = self.own.hoisted.get();
        self.own.hoisted.set(n + 1);
        format!("{}{}", prefix, n)
    }

    /// Record what the block owes this `let`, and declare its drop flag where
    /// the local is handed away on some paths and not others.
    ///
    /// Only a plain name is claimed. A `let (a, b) = ..` or a `let Foo { x } =
    /// ..` takes a value apart, and releasing the parts is not the same as
    /// releasing the whole; that is the partial-move case, reported rather than
    /// guessed at.
    pub(crate) fn claim_local(
        &self,
        name: &str,
        emitted: &str,
        ty: Option<&crate::ty::Ty>,
        known: Option<ownership::Drops>,
        pat: &syn::Pat,
        disposition: ownership::Disposition,
    ) -> String {
        let Some(tc) = &self.types else { return String::new() };
        // A closure has no type the engine names, and the emitter has just
        // decided what this one is; anything else is read off the type.
        let drops = match (known, ty) {
            (Some(drops), _) => drops,
            (None, Some(ty)) => ownership::drops_of(&tc.borrow().probe(), ty),
            (None, None) => return String::new(),
        };
        if !drops.is_droppable() {
            if drops == ownership::Drops::Unknown {
                self.fallback(
                    syn::spanned::Spanned::span(pat),
                    format!(
                        "`{}` has a type the engine cannot say owns anything, so nothing \
                         releases it",
                        name
                    ),
                );
            }
            return String::new();
        }
        if !matches!(crate::body::strip_binding(pat), syn::Pat::Ident(_)) {
            self.fallback(
                syn::spanned::Spanned::span(pat),
                "this `let` takes a droppable value apart, and the parts are released \
                 separately in Rust; nothing releases them here",
            );
            return String::new();
        }
        let flag = match disposition {
            // Somebody else owns it by the time the block ends.
            ownership::Disposition::Moved | ownership::Disposition::Unsure => return String::new(),
            ownership::Disposition::Kept => None,
            ownership::Disposition::Flagged => {
                let flag = self.fresh_hoist("_moved");
                self.own.flags.borrow_mut().insert(name.to_string(), flag.clone());
                Some(flag)
            }
        };
        let declaration = match &flag {
            Some(flag) => format!("let {} = false;\n", flag),
            None => String::new(),
        };
        self.own.pending.borrow_mut().push(ownership::Owned {
            name: emitted.to_string(),
            source: Some(name.to_string()),
            drops,
            flag,
            statement_scoped: false,
        });
        declaration
    }

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

    /// Say so where a `match` has a shape the emitter has no lowering for.
    pub fn report_match_gap(&self, m: &syn::ExprMatch, what: &str) {
        self.fallback(
            syn::spanned::Spanned::span(m),
            format!("{}; nothing is emitted for it and no arm runs", what),
        );
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

    /// Can this scope hand away what lives at this place?
    ///
    /// Only the owner can. A `&self` method lends its receiver, a local bound
    /// to a `&T` lends what it points at, and Rust refuses a move out of
    /// either — so a read there is a borrow, whatever the position looks like.
    pub(crate) fn owns_place(&self, expr: &syn::Expr) -> bool {
        match ownership::places::root_of(expr) {
            syn::Expr::Path(path) if path.path.is_ident("self") => self.owns_self,
            root @ syn::Expr::Path(_) => !matches!(
                self.quietly(|| self.resolve_expr_type(root)),
                Ok(crate::ty::Ty::Ref { .. })
            ),
            // A value the expression built is nobody else's, so taking it apart
            // takes nothing from anybody.
            _ => true,
        }
    }

    /// An expression in a position that takes its value, rather than reading
    /// through it: an argument, a struct field, a tuple element, a `break`.
    ///
    /// The one thing that changes here is a field read. `take(pair.one)` hands
    /// `one` to the callee and leaves the rest of `pair` where it was, so the
    /// field has to come *out* of the struct — otherwise the callee releases it
    /// and `pair`'s own cascade releases it a second time.
    pub fn moved_value(&self, expr: &syn::Expr) -> String {
        self.partial_move(expr)
            .unwrap_or_else(|| self.expr_value(expr))
    }

    /// `s.field` in a value position, as `s.takeField('field')` — or nothing
    /// where the read is not a move.
    pub(crate) fn partial_move(&self, expr: &syn::Expr) -> Option<String> {
        let syn::Expr::Field(field) = expr else {
            return None;
        };
        if !ownership::places::is_field_of_place(expr) {
            return None;
        }
        if !self.owns_place(expr) {
            return None;
        }
        let tc = self.types.as_ref()?;
        let ty = self.quietly(|| self.resolve_expr_type(expr)).ok()?;
        let drops = ownership::drops_of(&tc.borrow().probe(), &ty);
        let (receiver, member) = self.field_parts(field);
        if drops == ownership::Drops::Cascade {
            // `takeField` is `AkObject`'s, and a field the runtime writes as a
            // plain array or `Map` is not one: the read hands the same object to
            // two owners and the emitter has no way to say so.
            self.fallback(
                syn::spanned::Spanned::span(expr),
                format!(
                    "`{}` moves a field the runtime writes as a plain value; both the struct \
                     and the new owner release it",
                    member
                ),
            );
            return None;
        }
        ownership::places::take_field(&receiver, &member, drops)
    }

    /// Translate something with a statement scope of its own.
    ///
    /// A closure body and a match arm become functions in TypeScript, and a
    /// declaration lifted out of one of them cannot stand outside it: the
    /// closure's parameter is not in scope there. So the lifted declarations
    /// come back with the text instead of escaping to the enclosing statement.
    pub fn with_own_hoists<R>(&self, f: impl FnOnce() -> R) -> (R, Vec<ownership::Hoist>) {
        let saved = std::mem::take(&mut *self.own.prelude.borrow_mut());
        let result = f();
        let lifted = std::mem::replace(&mut *self.own.prelude.borrow_mut(), saved);
        (result, lifted)
    }

    /// The body of an arrow function that produces `value`, with everything
    /// lifted out of it declared and released inside.
    pub fn arrow_body(value: &str, hoists: &[ownership::Hoist]) -> String {
        ownership::hoisted(&format!("return {};\n", value), hoists)
    }

    /// A condition, with everything it produced released before the body it
    /// guards runs.
    ///
    /// Rust drops a condition's temporaries at the end of the condition, so a
    /// lock taken to make the test is released before the branch is taken.
    /// Returns the text to test and the statements that have to stand before
    /// the `if` — nothing at all where the condition lifted nothing.
    pub fn settle_condition(
        &self,
        cond: String,
        lifted: &[ownership::Hoist],
    ) -> (String, String) {
        if lifted.is_empty() {
            return (cond, String::new());
        }
        let held = self.fresh_hoist("_c");
        let before = format!(
            "let {};\n{}",
            held,
            ownership::hoisted(&format!("{} = {};\n", held, cond), lifted)
        );
        (held, before)
    }

    /// Lift a receiver the statement produced and nothing binds.
    ///
    /// `self.inner.read().unwrap().len()` produces a guard: Rust drops it at
    /// the end of the statement, and the emitted TypeScript would otherwise
    /// hold the lock for the life of the program. A receiver that is a *place*
    /// — a name, a field, an index — produces nothing and is left alone, and so
    /// is one the callee takes by value, because the callee owns it from there.
    ///
    /// Only the values `@ankurah/base` hands back as owning objects are lifted.
    /// A `Vec` or a `HashMap` receiver is a plain JavaScript array or `Map` by
    /// the time it is written, and the native translations rewrite what the
    /// call produces, so a release written against the Rust type would not be
    /// releasing the Rust value.
    pub(crate) fn hoist_receiver(&self, call: &syn::ExprMethodCall, written: String) -> String {
        if <Self as ownership::moves::Consumes>::consumes_receiver(self, call) {
            return written;
        }
        self.hoist_produced(&call.receiver, written)
    }

    /// The same, for any expression the statement produced and nothing binds.
    pub(crate) fn hoist_produced(&self, expr: &syn::Expr, written: String) -> String {
        if crate::body::is_place(expr) {
            return written;
        }
        let Some(tc) = &self.types else { return written };
        let drops = {
            let tc = tc.borrow();
            let Ok(ty) = tc.resolve_expr(expr) else {
                return written;
            };
            ownership::drops_of(&tc.probe(), &ty)
        };
        if !matches!(drops, ownership::Drops::Guard | ownership::Drops::Own) {
            return written;
        }
        self.hoist_temporary(written, drops)
    }

    /// Give a value produced inside an expression a name and a release.
    ///
    /// Returns the name to write in place of the expression.
    pub(crate) fn hoist_temporary(&self, written: String, drops: ownership::Drops) -> String {
        let name = self.fresh_hoist("_t");
        self.own.prelude.borrow_mut().push(ownership::Hoist {
            declaration: format!("const {} = {};\n", name, written),
            owned: Some(ownership::Owned {
                name: name.clone(),
                source: None,
                drops,
                flag: None,
                statement_scoped: true,
            }),
        });
        name
    }

    /// Say so where a `select!` has a shape this lowering does not carry over.
    pub fn report_select_gap(&self, tokens: &proc_macro2::TokenStream, what: &str) {
        self.fallback(
            syn::spanned::Spanned::span(tokens),
            format!("`select!` is lowered to the runtime's arbiter, but {}", what),
        );
    }

    /// Say so where a macro the emitter does not expand is handed a value the
    /// block owns: the macro becomes a comment, and the value goes with it.
    pub fn report_unsupported_macro(&self, mac: &syn::Macro, name: &str) {
        let parse = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        let Ok(args) = syn::parse::Parser::parse2(parse, mac.tokens.clone()) else {
            return;
        };
        let owned: Vec<String> = args
            .iter()
            .filter_map(|arg| match arg {
                syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                    Some(path.path.segments[0].ident.to_string())
                }
                _ => None,
            })
            .map(|name| name_map::to_camel_case(&name))
            .filter(|name| self.owes_a_release(name))
            .collect();
        if owned.is_empty() {
            return;
        }
        self.fallback(
            syn::spanned::Spanned::span(mac),
            format!(
                "`{}!` is emitted as a comment and is handed {}, which nothing then releases",
                name,
                owned
                    .iter()
                    .map(|n| format!("`{}`", n))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }

    /// Does a value under this name owe the block a release?
    pub(crate) fn owes_a_release(&self, name: &str) -> bool {
        let Some(tc) = &self.types else { return false };
        let Some(ty) = tc.borrow().lookup(name) else {
            return false;
        };
        ownership::drops_of(&tc.borrow().probe(), &ty).is_droppable()
    }

    /// `place = value`, with what the place held released where Rust releases
    /// it: after the new value is evaluated and before it is stored.
    ///
    /// The bare assignment abandoned the old value — a `let mut` reassigned in
    /// a loop leaked one object per turn.
    pub(crate) fn assign(&self, assign: &syn::ExprAssign) -> String {
        let left = self.expr(&assign.left);
        let Some(release) = self.release_of(&assign.left, &left) else {
            return format!("{} = {}", left, self.moved_value(&assign.right));
        };
        // Where a branch already handed the old value away, whether there is
        // anything left to release is what the drop flag answers, and a flag
        // reset is not a thing this emitter writes.
        if self.own.flags.borrow().contains_key(&left) {
            self.fallback(
                syn::spanned::Spanned::span(assign),
                format!(
                    "`{}` is handed away on some path and assigned on another; the value it \
                     held at the assignment is not released",
                    left
                ),
            );
            return format!("{} = {}", left, self.moved_value(&assign.right));
        }
        let held = self.fresh_hoist("_a");
        let right = self.moved_value(&assign.right);
        self.own.prelude.borrow_mut().push(ownership::Hoist {
            declaration: format!("const {} = {};\n{}\n", held, right, release),
            owned: None,
        });
        format!("{} = {}", left, held)
    }

    /// `option.unwrap_or(default)` where the default owes a release.
    ///
    /// Rust evaluates the default eagerly and drops it on the path that did not
    /// take it, so the emitted code reads both, chooses, and releases the one it
    /// did not choose. Where the default owns nothing there is nothing to say
    /// and `??` stands on its own.
    pub(crate) fn nullable_default(
        &self,
        receiver: &str,
        default: &syn::Expr,
        default_ts: &str,
    ) -> Option<String> {
        let held = self.fresh_hoist("_d");
        let release = self.release_of(default, &held)?;
        let chosen = self.fresh_hoist("_u");
        self.own.prelude.borrow_mut().push(ownership::Hoist {
            declaration: format!(
                "const {held} = {default_ts};\nconst {chosen} = {receiver} ?? {held};\n\
                 if ({chosen} !== {held}) {release}\n",
                held = held,
                default_ts = default_ts,
                chosen = chosen,
                receiver = receiver,
                release = release,
            ),
            owned: None,
        });
        Some(chosen)
    }

    /// What releasing the value in this place costs, written against the text
    /// that names it.
    pub(crate) fn release_of(&self, place: &syn::Expr, text: &str) -> Option<String> {
        let tc = self.types.as_ref()?;
        let ty = self.quietly(|| self.resolve_expr_type(place)).ok()?;
        ownership::drops_of(&tc.borrow().probe(), &ty).release(text)
    }

    /// `drop(x)`, written as whatever releasing an `x` costs.    /// `drop(x)`, written as whatever releasing an `x` costs.
    ///
    /// The move analysis has already taken `x` off the block's list — it is an
    /// argument passed by value like any other — so this releases it once,
    /// where the source says.
    pub(crate) fn explicit_drop(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(path) = &*call.func else {
            return None;
        };
        let written = crate::body::BodyTranslator::path_static(&path.path);
        if !matches!(written.as_str(), "drop" | "mem.drop") || call.args.len() != 1 {
            return None;
        }
        let arg = &call.args[0];
        let text = self.moved_value(arg);
        let tc = self.types.as_ref()?;
        let ty = self.quietly(|| self.resolve_expr_type(arg)).ok()?;
        let drops = ownership::drops_of(&tc.borrow().probe(), &ty);
        match drops {
            ownership::Drops::Unknown => {
                self.fallback(
                    syn::spanned::Spanned::span(call),
                    "`drop` is called on a value the engine cannot say owns anything, so \
                     nothing releases it",
                );
                None
            }
            // Rust runs no glue for a `Copy` value; the argument is still
            // evaluated, and `void` says the result goes nowhere.
            ownership::Drops::Nothing => Some(format!("void {}", text)),
            _ => drops.release_expr(&text),
        }
    }

    /// Does the runtime write this call as something whose result is not what
    /// Rust returns?
    ///
    /// `map.insert(k, v)` becomes `map.set(k, v)`, and JavaScript's `Map.set`
    /// hands back the map where Rust hands back the value it displaced.
    /// Releasing the statement's value there would release the whole map.
    pub(crate) fn rewritten_by_runtime(&self, expr: &syn::Expr) -> bool {
        let syn::Expr::MethodCall(call) = expr else {
            return false;
        };
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        // Asking is not translating; the call reports its own gaps where it is
        // written out.
        let mark = tc.sink.mark();
        let found = tc.resolve_method_call_with(
            &call.receiver,
            &call.method.to_string(),
            call.turbofish.as_ref(),
        );
        tc.sink.rewind(mark);
        let Ok(found) = found else { return false };
        let args: Vec<String> = call.args.iter().map(|_| "_".to_string()).collect();
        let translated = native_types::translate_method(
            tc.registry,
            found.receiver_type(),
            "_",
            &call.method.to_string(),
            &args,
        );
        matches!(translated, native_types::MethodTranslation::Expr(_))
    }

    /// A statement's value, thrown away. Rust drops it at the end of the
    /// statement, so the emitted statement releases it there.
    pub(crate) fn discard(&self, expr: &syn::Expr, text: String) -> String {
        if !matches!(
            expr,
            syn::Expr::Call(_) | syn::Expr::MethodCall(_) | syn::Expr::Await(_)
        ) {
            return text;
        }
        let Some(tc) = &self.types else { return text };
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(expr)) else {
            return text;
        };
        // `n.notified().await` yields the future's `Output`, and awaiting takes
        // the future itself for good. Where the engine could not project
        // through `Future` it hands back the future's own type, and releasing
        // that would drop a value the await already moved.
        if let syn::Expr::Await(await_expr) = expr {
            if self.quietly(|| self.resolve_expr_type(&await_expr.base)).ok().as_ref() == Some(&ty) {
                self.fallback(
                    syn::spanned::Spanned::span(expr),
                    "the engine could not say what awaiting this produces, so the value the \
                     statement threw away is not released",
                );
                return text;
            }
        }
        let drops = ownership::drops_of(&tc.borrow().probe(), &ty);
        // A guard the statement produced is already lifted and released by the
        // hoist machinery, which also lists it in the enclosing `finally`.
        if drops == ownership::Drops::Guard {
            return text;
        }
        if self.rewritten_by_runtime(expr) {
            self.fallback(
                syn::spanned::Spanned::span(expr),
                "the runtime writes this call as something whose result is not the value Rust \
                 returns, so the value the statement threw away is not released",
            );
            return text;
        }
        let held = if text.starts_with("await ") {
            format!("({})", text)
        } else {
            text.clone()
        };
        drops.release_expr(&held).unwrap_or(text)
    }

    /// A closure, with what a `move` took by value released where Rust    /// A closure, with what a `move` took by value released where Rust
    /// releases it.
    ///
    /// A closure that captures nothing droppable stays a plain arrow function:
    /// there is nothing for the cascade to find, and wrapping it would only add
    /// a drop the emitter then has to place. One that does capture something
    /// becomes an `OwnedClosure`, except where it is called on the spot — there
    /// the captures are released inside the closure's own body, because Rust
    /// creates, calls and drops it in the one expression.
    pub(crate) fn closure(
        &self,
        closure: &syn::ExprClosure,
        placement: ownership::closures::Placement,
    ) -> String {
        use ownership::closures::Placement;
        let params: Vec<String> = closure.inputs.iter().map(Self::pat_static).collect();
        let params = params.join(", ");
        let captures = if closure.capture.is_some() {
            self.owned_captures(closure)
        } else {
            Vec::new()
        };
        // Push closure scope — param types may already be registered
        // by the calling method's closure param resolution
        self.push_block();
        // A block body carries its own braces; an expression body is the
        // closure's value, and a guard lifted out of it belongs inside the
        // arrow function, because its declaration names the closure's own
        // parameters and those do not exist outside.
        let (statements, arrow) = match &*closure.body {
            syn::Expr::Block(block) => {
                let body = self.translate_block(&block.block);
                (body.clone(), format!("({}) => {{\n{}}}", params, indent(&body)))
            }
            _ => {
                let (body, lifted) = self.with_own_hoists(|| self.expr_value(&closure.body));
                let inner = Self::arrow_body(&body, &lifted);
                let arrow = if !lifted.is_empty() {
                    format!("({}) => {{\n{}}}", params, indent(&inner))
                } else if body.starts_with("if ")
                    || body.starts_with("for ")
                    || body.starts_with("while ")
                    || body.starts_with('{')
                {
                    format!("({}) => {{\n  {}\n}}", params, body)
                } else {
                    format!("({}) => {}", params, body)
                };
                (inner, arrow)
            }
        };
        self.pop_scope();
        if captures.is_empty() {
            return arrow;
        }
        let owned: Vec<ownership::Owned> = captures
            .iter()
            .map(|(name, drops)| ownership::Owned {
                name: name.clone(),
                source: None,
                drops: *drops,
                flag: None,
                statement_scoped: false,
            })
            .collect();
        let names: Vec<String> = captures.iter().map(|(name, _)| name.clone()).collect();
        match placement {
            Placement::Immediate => ownership::closures::immediate(&params, &statements, &owned),
            Placement::Bound => ownership::closures::owned(&names, &arrow),
            Placement::Loose => {
                self.fallback(
                    syn::spanned::Spanned::span(closure),
                    ownership::closures::loose_report(&names),
                );
                ownership::closures::owned(&names, &arrow)
            }
        }
    }

    /// The captures a `move` closure took by value that owe a release, with
    /// what each of them costs.
    pub(crate) fn owned_captures(&self, closure: &syn::ExprClosure) -> Vec<(String, ownership::Drops)> {
        let Some(tc) = &self.types else {
            return Vec::new();
        };
        let scan = ownership::Scan::new(self);
        let mut out: Vec<(String, ownership::Drops)> = Vec::new();
        for site in scan.captures(closure) {
            // `this` is the enclosing method's receiver, not a local this scope
            // can hand over, so a closure that names it is left alone.
            if site.name == "this" || out.iter().any(|(name, _)| *name == site.name) {
                continue;
            }
            let Some(ty) = tc.borrow().lookup(&site.name) else {
                // A local the closure names and the engine could not type is
                // one nobody can say the closure owns. Listing the rest as
                // captures without it would say the closure owns exactly those,
                // which is a claim the emitter cannot make.
                if tc.borrow().is_bound(&site.name) {
                    self.fallback(
                        site.span,
                        format!(
                            "this closure takes `{}` by value and the engine could not type \
                             it, so it is left out of what the closure owns",
                            site.name
                        ),
                    );
                }
                continue;
            };
            let drops = ownership::drops_of(&tc.borrow().probe(), &ty);
            if drops.is_droppable() {
                // A shadow is emitted under a fresh identifier, and the capture
                // list has to name the value the body closes over rather than
                // the name Rust wrote for it.
                let emitted = self.emitted_name(&site.name).unwrap_or_else(|| site.name.clone());
                out.push((emitted, drops));
            }
        }
        out
    }

    /// `for x in seq { .. }`, with what the loop owns released where Rust    /// `for x in seq { .. }`, with what the loop owns released where Rust
    /// releases it.
    ///
    /// Rust's `IntoIterator` takes the sequence by value, hands out one element
    /// per turn and drops the rest when the loop stops — so the binding is
    /// released at the end of each turn, and a `break` or a `return` releases
    /// everything the loop never reached. A loop over `&seq` owns none of that,
    /// and the item type is what tells the two apart.
    pub(crate) fn for_loop(&self, for_loop: &syn::ExprForLoop) -> String {
        use ownership::iteration::Iterate;
        let pat = Self::pat_static(&for_loop.pat);
        let sequence = self.expr(&for_loop.expr);
        let item = self.iteration_item(&for_loop.expr);
        let sequence_ty = self.quietly(|| self.resolve_expr_type(&for_loop.expr)).ok();
        let form = match &self.types {
            Some(tc) => ownership::iteration::iterate(
                &tc.borrow().probe(),
                sequence_ty.as_ref(),
                item.as_ref(),
            ),
            None => Iterate::Borrowed,
        };
        let _bindings = self.enter_pattern(&for_loop.pat, item.as_ref());
        let owned = match form {
            Iterate::Borrowed => Vec::new(),
            _ => self.claim_bindings(&crate::body::pattern_names(&for_loop.pat), &for_loop.body.stmts),
        };
        let body = self.translate_block(&for_loop.body);
        drop(_bindings);
        let body = self.wrap_bindings(&owned, body);
        match form {
            Iterate::Borrowed => {
                format!("for (const {} of {}) {{\n{}}}", pat, sequence, indent(&body))
            }
            Iterate::OwnedArray => {
                let held = self.fresh_hoist("_seq");
                let at = self.fresh_hoist("_at");
                let loop_ts = ownership::iteration::owned_array_loop(&held, &at, &pat, &body);
                format!("const {} = {};\n{}", held, sequence, loop_ts)
            }
            Iterate::OwnedOpaque => {
                self.fallback(
                    syn::spanned::Spanned::span(&for_loop.expr),
                    "this loop takes the sequence by value, and the runtime does not write it \
                     as an array; the elements a `break` or a `return` leaves behind are not \
                     released",
                );
                format!("for (const {} of {}) {{\n{}}}", pat, sequence, indent(&body))
            }
        }
    }
}
