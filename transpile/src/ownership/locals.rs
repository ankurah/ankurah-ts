//! What a block owes for the values its `let`s and its parameters bind.
//!
//! A block that ends has to release everything it still owns and nothing it
//! does not, and the two are told apart per name and per path: a local handed
//! to somebody else on every path is gone, one handed away on some paths only
//! is released behind a drop flag, and one nobody took is released outright.
//!
//! So the block reads its own statements once, before writing any of them, and
//! records what it decided about each name it binds. `claim_local` and
//! `claim_params` turn that decision into the value's entry on the block's
//! list and, where a flag is called for, into the flag's declaration;
//! `flag_sets` writes the assignment that sets the flag at the site that
//! handed the value away.

use crate::body::BodyTranslator;
use crate::ownership;

impl<'a> BodyTranslator<'a> {
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
        // The scan spells the receiver `this` whatever the emitted parameter is
        // called, because that is what a method's body writes. An impl with no
        // class of its own is emitted as module-level functions whose first
        // parameter is named `self`, so the two spellings have to be lined up
        // or every move of the receiver is invisible and the body releases a
        // value it handed away.
        let scan_name = |name: &str| {
            if name == self.self_name { "this".to_string() } else { name.to_string() }
        };
        let names: Vec<String> = params.iter().map(|(name, _)| scan_name(name)).collect();
        let scan = ownership::Scan::new(self);
        // Every site in the body stands after the parameter list, so the
        // parameters are the declaration each one is attributed to.
        let sites = scan.block(&block.stmts).into_iter().map(|site| (1, site));
        let dispositions = ownership::Dispositions::build(&[(0, names)], sites.collect());
        let mut owned = Vec::new();
        for (name, ty) in params {
            self.own.by_value_params.borrow_mut().insert(name.clone());
            let drops = ownership::drops_of(&tc.borrow().probe(), ty);
            let drops = if drops.is_droppable() { drops } else { self.callable_by_value(ty) };
            if !drops.is_droppable() {
                continue;
            }
            let flag = match dispositions.of(&scan_name(name), 1) {
                ownership::Disposition::Moved | ownership::Disposition::Unsure => continue,
                ownership::Disposition::Kept => None,
                ownership::Disposition::Flagged => {
                    let flag = self.fresh_hoist("_moved");
                    self.own.flags.borrow_mut().insert(name.clone(), flag.clone());
                    Some(flag)
                }
            };
            self.own.claimed_params.borrow_mut().insert(name.clone());
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

    /// Does a value of this type, taken BY VALUE, owe a release?
    ///
    /// The same two questions `claim_params` asks of a function's parameter:
    /// what the type's own glue costs, and — where it costs nothing — whether
    /// the parameter is a callable the body owns.
    pub(crate) fn by_value_owes_a_release(&self, ty: &crate::ty::Ty) -> bool {
        let Some(tc) = &self.types else { return false };
        let drops = ownership::drops_of(&tc.borrow().probe(), ty);
        drops.is_droppable() || self.callable_by_value(ty).is_droppable()
    }

    /// A callable parameter the body OWNS, and therefore owes a release.
    ///
    /// `fn f<F: Fn(u32) -> u32>(f: F)` takes `f` by value and drops it at the
    /// end of the body; only the CALL borrows. The port wrote the call as
    /// `invokeRef`, which leaves the closure whole, and nothing released it — so
    /// every capture of every `OwnedClosure` handed to such a parameter leaked.
    /// Two live sites: core's `ResultSet::retain_dirty` and signals'
    /// `Value::set_with`.
    ///
    /// `Drops::Cascade`, which is `dropOwned(f)`: a plain function reaches none
    /// of that function's branches and is left alone, and an `OwnedClosure` has
    /// a `drop()` for it to find.
    ///
    /// Two parameters are NOT this. One written `&F` or `&mut F` is somebody
    /// else's, and one whose type is not a callable bound at all is an ordinary
    /// value, already answered by `drops_of`.
    ///
    /// A parameter whose bound is `FnOnce` alone IS this, even though `invoke`
    /// releases it where the call runs: the call is not on every path.
    /// `fn run(f: impl FnOnce() -> u32, take: bool) { if take { f() } }` leaked
    /// the closure and everything it captured whenever `take` was false. The
    /// parameter is claimed, and the move scan — which now counts an `invoke`
    /// call as the move it is — decides between releasing it, skipping it
    /// behind a flag, and not writing a release at all (R4).
    fn callable_by_value(&self, ty: &crate::ty::Ty) -> ownership::Drops {
        if matches!(ty, crate::ty::Ty::Ref { .. }) {
            return ownership::Drops::Unknown;
        }
        let Some(tc) = &self.types else { return ownership::Drops::Unknown };
        let tc = tc.borrow();
        if crate::infer::expected::fn_shape(tc.registry, ty, &tc.param_bounds).is_none() {
            return ownership::Drops::Unknown;
        }
        ownership::Drops::Cascade
    }

    /// Which of this block's locals were handed to somebody else before it
    /// ended, and where. Everything the block owes turns on it.
    pub(crate) fn analyse_moves(&self, stmts: &[syn::Stmt]) -> ownership::Dispositions {
        let scan = ownership::Scan::new(self);
        let mut declarations: Vec<(usize, Vec<String>)> = Vec::new();
        // The scan asks whether a `match` or an `if let` takes its subject
        // apart, which is a question about the subject's *type*. This runs
        // before the first statement is written, so the block's own `let`s are
        // not bound yet and a subject that is one of them answered nothing —
        // the move went unrecorded and the block released a value an arm had
        // already taken. Binding them here, in a scope of their own, is what
        // lets the scan ask.
        self.bind_locals_for_scan(stmts);
        // One walk over the whole block, not one per statement: whether a
        // straight-line move is on every path depends on what stands above it.
        let sites = scan.block_indexed(stmts);
        self.pop_scope();
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

    /// Bind this block's `let` names to the types their initializers resolve
    /// to, in a scope the caller pops.
    ///
    /// In source order, because a later initializer reads an earlier name. A
    /// `let` whose initializer the engine could not type binds nothing, which
    /// leaves the scan exactly where it stood before.
    fn bind_locals_for_scan(&self, stmts: &[syn::Stmt]) {
        self.push_block();
        for stmt in stmts {
            let syn::Stmt::Local(local) = stmt else { continue };
            let Some(init) = &local.init else { continue };
            let Ok(ty) = self.quietly(|| self.resolve_expr_type(&init.expr)) else {
                continue;
            };
            for name in crate::body::pattern_names(&local.pat) {
                self.bind_var(&name, ty.clone());
            }
        }
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
    ///
    /// The flag is set where the move is written and never cleared, and that is
    /// the whole of what this analysis says about a loop: a local handed away
    /// inside a loop body is gone from the turn that reached the move onwards,
    /// and the `finally` reads the flag once, when the block that declared the
    /// local ends. Telling one turn from another would matter only for a turn
    /// that reads a value an earlier turn handed away — which is a use after
    /// move, and rustc refuses to compile it, so no crate that builds can hand
    /// the emitter that shape.
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

    /// The same, split by WHERE the transfer happens.
    ///
    /// O6 puts a flag immediately above the transfer, which is BELOW everything
    /// the statement lifted out of itself — an argument that throws no longer
    /// leaves a flag saying the value was handed over. One hoist shape breaks
    /// that: a `?` lifts the very call that consumes. `let n = eat(c)?;`
    /// evaluates `eat(c)` in the hoist and leaves the statement on the error
    /// path, so a flag written below the prelude is never set at all and the
    /// block releases what `eat` took — a double drop. Those flags stand above
    /// the prelude, where every flag used to stand; every other one stands
    /// below it.
    pub(crate) fn flag_sets_split(&self, stmt: &syn::Stmt) -> (String, String) {
        let all = self.flag_sets(stmt);
        if all.is_empty() {
            return (String::new(), all);
        }
        let mut in_a_try: Vec<String> = Vec::new();
        for operand in crate::body::flags::try_operands(stmt) {
            let inner = syn::Stmt::Expr(operand, None);
            for site in ownership::Scan::new(self).shallow(&inner) {
                if let Some(flag) = self.own.flags.borrow().get(&site.name) {
                    in_a_try.push(flag.clone());
                }
            }
        }
        if in_a_try.is_empty() {
            return (String::new(), all);
        }
        let (mut above, mut below) = (String::new(), String::new());
        for line in all.lines() {
            let flag = line.split(' ').next().unwrap_or_default();
            let target = match in_a_try.iter().any(|f| f == flag) {
                true => &mut above,
                false => &mut below,
            };
            target.push_str(line);
            target.push('\n');
        }
        (above, below)
    }

    /// `_moved_x = true;` for a subject a *pattern* handed away rather than an
    /// expression: `other => ..` moves the whole subject into the arm's name,
    /// and the scan that reads expressions sees nothing to report at the site.
    pub(crate) fn flag_set_for_subject(&self, subject: &syn::Expr) -> String {
        let syn::Expr::Path(path) = subject else {
            return String::new();
        };
        let Some(name) = ownership::moves::local_name(path) else {
            return String::new();
        };
        match self.own.flags.borrow().get(&name) {
            Some(flag) => format!("{} = true;\n", flag),
            None => String::new(),
        }
    }

    /// The flag assignments an expression owes, for a position that is not a
    /// statement — a match arm's body, which the arm renders as a block.
    pub fn flag_sets_for(&self, expr: &syn::Expr) -> String {
        self.flag_sets(&syn::Stmt::Expr(expr.clone(), None))
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

    /// Does a value under this name owe the block a release?
    pub(crate) fn owes_a_release(&self, name: &str) -> bool {
        let Some(tc) = &self.types else { return false };
        let Some(ty) = tc.borrow().lookup(name) else {
            return false;
        };
        ownership::drops_of(&tc.borrow().probe(), &ty).is_droppable()
    }

    /// What a statement whose lowering REFUSED owes the locals it was going to
    /// hand away.
    ///
    /// K3: a statement that refuses hands nothing away. The move analysis has
    /// already decided what each local's disposition is, and it decided from
    /// the source: a local moved into a `collect` the engine has no
    /// construction for is `Moved`, so the block releases nothing and the hole
    /// throws with the value still sitting there.
    /// `core/property/backend/lww.ts`'s `from_state_buffer` is that case — the
    /// map its decoder had just built was moved into a refused `collect`, and
    /// nothing released it on any path. A `Flagged` local is left alone: the
    /// block's own `finally` releases it once the flag is not written (which
    /// is the other half of this rule, in `blocks.rs`).
    /// What a statement owes for the SOURCE values it named, on the path where
    /// it refused.
    ///
    /// The same question `released_by_a_refusal` answers, asked where the
    /// statement's own prefix may already have run: a `?` operand standing
    /// before the refusal is evaluated, and whatever it consumed is gone. So
    /// each release is guarded by the runtime's own answer, and it stands in a
    /// `finally` around the statement rather than above it.
    ///
    /// A by-value PARAMETER is here too, which `released_by_a_refusal` cannot
    /// reach: it is not a local of any block, so it has no ordinal and no
    /// disposition — the function's own claim decided it, and where that claim
    /// said "handed on" and the statement that would have handed it on refused,
    /// nothing releases it at all.
    pub(crate) fn released_after_a_refusal(
        &self,
        stmt: &syn::Stmt,
        dispositions: &ownership::Dispositions,
        ordinals: &std::cell::RefCell<std::collections::HashMap<String, usize>>,
    ) -> String {
        let scan = ownership::Scan::new(self);
        let mut out = String::new();
        let mut written: Vec<String> = Vec::new();
        for site in scan.shallow(stmt) {
            if written.contains(&site.name) || self.own.flags.borrow().contains_key(&site.name) {
                continue;
            }
            let ordinal = ordinals.borrow().get(&site.name).copied().unwrap_or(0);
            let owed = if ordinal == 0 {
                // Not a local of any block here: a parameter the function's own
                // claim let go of, or a name from further out that this body
                // does not own at all. Only the first is owed, and the claim is
                // what tells them apart.
                self.own.by_value_params.borrow().contains(&site.name)
                    && !self.own.claimed_params.borrow().contains(&site.name)
            } else {
                dispositions.of(&site.name, ordinal) == ownership::Disposition::Moved
            };
            if !owed {
                continue;
            }
            let Some(tc) = &self.types else { continue };
            let Some(ty) = tc.borrow().lookup(&site.name) else { continue };
            if !ownership::drops_of(&tc.borrow().probe(), &ty).is_droppable() {
                continue;
            }
            let name = self.emitted_name(&site.name).unwrap_or_else(|| site.name.clone());
            written.push(site.name.clone());
            out.push_str(&ownership::guarded_release(&name));
        }
        out
    }

    pub(crate) fn released_by_a_refusal(
        &self,
        stmt: &syn::Stmt,
        dispositions: &ownership::Dispositions,
        ordinals: &std::cell::RefCell<std::collections::HashMap<String, usize>>,
    ) -> String {
        let scan = ownership::Scan::new(self);
        let mut out = String::new();
        let mut written: Vec<String> = Vec::new();
        for site in scan.shallow(stmt) {
            if written.contains(&site.name) || self.own.flags.borrow().contains_key(&site.name) {
                continue;
            }
            let ordinal = ordinals.borrow().get(&site.name).copied().unwrap_or(0);
            if ordinal == 0 || dispositions.of(&site.name, ordinal) != ownership::Disposition::Moved
            {
                continue;
            }
            let Some(tc) = &self.types else { continue };
            let Some(ty) = tc.borrow().lookup(&site.name) else { continue };
            let drops = ownership::drops_of(&tc.borrow().probe(), &ty);
            let name = self.emitted_name(&site.name).unwrap_or_else(|| site.name.clone());
            let Some(release) = drops.release(&name) else { continue };
            written.push(site.name.clone());
            out.push_str(&format!("{}\n", release));
        }
        out
    }
}
