//! What the translator knows while it writes: the scopes, the expectations and
//! the diagnostics.
//!
//! For: a body is translated one expression at a time, and each of them is
//! written from more than the expression itself — the names in scope and their
//! types, the type the POSITION wants of it (which is what settles a literal's
//! width and a closure's parameters), and the identifier a shadowed name was
//! freshened to. This is where all of that is asked and answered; nothing here
//! writes TypeScript.

use crate::native_types;

use super::{as_move_closure, position_of, span_position, BodyTranslator, PatternScope};

impl<'a> BodyTranslator<'a> {
    /// Translate one expression with the type its position wants of it.
    ///
    /// The expectation reaches that expression and nothing else: it is put back
    /// as it was afterwards, so a position that supplies one inside another
    /// does not leave it standing.
    pub(crate) fn expecting<T>(
        &self,
        expr: &syn::Expr,
        want: Option<&crate::ty::Ty>,
        translate: impl FnOnce() -> T,
    ) -> T {
        let held = self
            .expecting
            .replace(want.map(|ty| (position_of(expr), ty.clone())));
        let written = translate();
        *self.expecting.borrow_mut() = held;
        written
    }

    /// The type this expression's position wants of it, where the position
    /// said one.
    pub(crate) fn expectation_for(&self, expr: &syn::Expr) -> Option<crate::ty::Ty> {
        self.expectation_at(syn::spanned::Spanned::span(expr))
    }

    /// The same, asked by span, for a translation that has the span but no
    /// longer holds the expression it came from.
    pub(crate) fn expectation_at(&self, span: proc_macro2::Span) -> Option<crate::ty::Ty> {
        let slot = self.expecting.borrow();
        let (at, ty) = slot.as_ref()?;
        (*at == span_position(span)).then(|| ty.clone())
    }

    /// The registry this body is translated against, where there is one.
    ///
    /// The type context holds it by shared reference for the whole run, so it
    /// comes out of the borrow rather than staying inside it.
    pub(crate) fn registry(&self) -> Option<&'a crate::registry::TypeRegistry> {
        self.types.as_ref().map(|tc| tc.borrow().registry)
    }

    /// Does this written path name `Option`'s own `Some` or `None`, rather than
    /// a crate enum's variant of that name?
    ///
    /// The port writes an `Option<T>` as `T | null`, so `Some(x)` is a null
    /// test and `None` is `== null`. A crate enum with a variant of that name
    /// is a different value entirely: `enum State { None, Some(i32), Other }`
    /// under a guard came out `if (s != null) { const n = s; …` — arm one ran
    /// for `State::Other`, and `State::None` was dead. The routing decision got
    /// this by identity at step 8; the pattern tests still decided by NAME, so
    /// a guard, a loop jump or an `if let` walked straight past it.
    pub(crate) fn names_option_variant(&self, path: &syn::Path) -> bool {
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        self.names_option_variant_by(&segments)
    }

    /// The same for a bare identifier, which is how `None` is written.
    pub(crate) fn names_option_variant_by(&self, segments: &[String]) -> bool {
        let Some(tc) = &self.types else { return true };
        let tc = tc.borrow();
        // A qualified path names the enum outright.
        if segments.len() >= 2 {
            return match tc.registry.lookup_variant(tc.module, segments) {
                Some((id, _)) => tc.registry.name_of(id) == "Option",
                None => true,
            };
        }
        // A bare `Some`/`None` is the prelude's unless the value namespace of
        // this module answers with something else: `use State::*` brings a
        // crate enum's own variants into scope under their bare names.
        match tc
            .registry
            .lookup(tc.module, crate::registry::Ns::Value, segments)
        {
            Ok(Some(crate::registry::Def::Type(id))) => tc.registry.name_of(id) == "Option",
            _ => true,
        }
    }

    /// Resolve the type of an expression, or say why not.
    pub(crate) fn resolve_expr_type(&self, expr: &syn::Expr) -> Result<crate::ty::Ty, crate::diag::Diag> {
        let expected = self.expectation_for(expr);
        match &self.types {
            Some(tc) => tc.borrow().resolve_expr_expecting(expr, expected.as_ref()),
            None => Err(crate::diag::Diag {
                file: String::new(),
                line: 0,
                col: 0,
                message: "no type context on this translation path".to_string(),
            }),
        }
    }

    /// The one place a retained fallback is recorded.
    ///
    /// The engine could not type this site, so the translator does what it did
    /// before and this says what was given up. Every fallback that survives
    /// into this step goes through here, which is what makes the diagnostics
    /// count a coverage measure rather than a sample. The fail-loud step turns
    /// these into errors and deletes the fallbacks.
    pub(crate) fn fallback(&self, span: proc_macro2::Span, message: impl Into<String>) {
        match &self.types {
            Some(tc) => tc.borrow().sink.report(span, message),
            // A translation path with no type context has no sink either; the
            // fallback waits there until the caller that owns the file drains it.
            None => crate::diag::pending::park(span, message.into()),
        }
    }


    /// Report a shape with no lowering AND write the hole that stands where its
    /// output would have gone (R12).
    ///
    /// A gap used to be reported and emitted anyway, as the nearest thing the
    /// engine could write — a consuming arm whose guard was dropped, an arm that
    /// tests inside a payload, a struct literal that lost its `..rest`. Each of
    /// those RUNS and answers what Rust would not, and a wrong answer at run
    /// time is a bug nobody traces to a line printed during a build. So the
    /// site says what it could not translate and the emitted file carries a
    /// `unsupported('..')` that stops the program there instead. The returned
    /// text is an expression: `unsupported` answers `never`, so it stands
    /// wherever the expression it replaces stood.
    pub(crate) fn hole(&self, span: proc_macro2::Span, what: impl Into<String>) -> String {
        let what = what.into();
        self.fallback(span, what.clone());
        crate::body::hole_text(&what)
    }

    /// Take a resolved answer, or record the fallback taken instead of it.
    pub(crate) fn or_fallback<T>(&self, result: Result<T, crate::diag::Diag>, instead: &str) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(diag) => {
                let message = format!("{}; {}", diag.message, instead);
                // A refusal raised where there is no type context carries no
                // position; the sites on those paths report for themselves,
                // with the span they do have.
                if let Some(tc) = &self.types {
                    tc.borrow().sink.push(crate::diag::Diag { message, ..diag });
                }
                None
            }
        }
    }

    /// Is a `let` of this name here a second declaration in the same block?
    /// JavaScript refuses that, and the translator writes an assignment instead.
    pub(crate) fn redeclares_here(&self, name: &str) -> bool {
        self.types.as_ref().map(|tc| tc.borrow().redeclares(name)).unwrap_or(false)
    }

    /// Bind a variable in the current TypeContext scope.
    pub fn bind_var(&self, name: &str, ty: crate::ty::Ty) {
        if let Some(tc) = &self.types {
            tc.borrow_mut().bind(name, ty);
        }
    }

    /// Bind a name whose type is not known, so that it still shadows.
    pub(crate) fn bind_untyped(&self, name: &str) {
        if let Some(tc) = &self.types {
            tc.borrow_mut().bind_untyped(name);
        }
    }

    /// Push a block scope in the TypeContext.
    pub fn push_block(&self) {
        if let Some(tc) = &self.types {
            tc.borrow_mut().push_block();
        }
    }

    /// Pop scope in the TypeContext.
    pub fn pop_scope(&self) {
        if let Some(tc) = &self.types {
            tc.borrow_mut().pop();
        }
    }

    /// Push a closure scope with typed parameters.
    pub fn push_closure_scope(&self, params: Vec<(String, crate::ty::Ty)>) {
        if let Some(tc) = &self.types {
            tc.borrow_mut().push_closure(params);
        }
    }

    /// The type of an expression a PATTERN is matched against, with a written
    /// `&` kept on it.
    ///
    /// For: everywhere else the port erases references, because a TypeScript
    /// object reference is what a Rust reference becomes, so `resolve_expr_type`
    /// answers what a `&e` points at. Matching is where the difference decides
    /// who owns what the pattern binds: Rust's default binding mode (RFC 2005)
    /// says a pattern matched against a reference binds by reference, and the
    /// binding then owes no release. With the `&` erased, `for v in &map`
    /// released the borrowed key and value on every turn and the map released
    /// them again, and `if let Some(order_by) = &self.order_by` released a
    /// vector the field still holds — both double drops the strict registry
    /// aborts on.
    pub fn borrowed_scrutinee_type(&self, expr: &syn::Expr) -> Option<crate::ty::Ty> {
        match expr {
            syn::Expr::Reference(r) => Some(crate::ty::Ty::Ref {
                mutable: r.mutability.is_some(),
                inner: Box::new(self.borrowed_scrutinee_type(&r.expr)?),
            }),
            syn::Expr::Paren(p) => self.borrowed_scrutinee_type(&p.expr),
            syn::Expr::Group(g) => self.borrowed_scrutinee_type(&g.expr),
            _ => self.scrutinee_type(expr),
        }
    }

    /// The type of the expression a `for` loop iterates.
    ///
    /// `IntoIterator for Vec<T>` hands out a `T` the loop has to release, and
    /// `IntoIterator for &Vec<T>` a `&T` that stays the sequence's, so the
    /// written `&` decides the form of the whole loop.
    pub fn iterated_type(&self, iterated: &syn::Expr) -> Option<crate::ty::Ty> {
        self.borrowed_scrutinee_type(iterated)
    }

    /// What one turn of a `for` loop over this expression hands out.
    pub fn iteration_item(&self, iterated: &syn::Expr) -> Option<crate::ty::Ty> {
        let tc = self.types.as_ref()?;
        let ty = self.iterated_type(iterated)?;
        // A sequence the engine cannot name the element of leaves the loop
        // variable untyped; the uses of that variable are what report it, so
        // that one gap is counted once per site rather than twice.
        tc.borrow().iteration_item(&ty)
    }

    /// Note the function a call landed on, for the oracle comparison, without
    /// translating it. A call the engine resolved and the runtime then writes as
    /// nothing is still a resolution, and leaving it unrecorded made the engine
    /// look as though it had no answer.
    pub(crate) fn record_resolution(&self, call: &syn::ExprMethodCall, method: &str) {
        let Some(tc) = &self.types else { return };
        let tc = tc.borrow();
        let Ok(found) =
            tc.resolve_method_call_with(&call.receiver, method, call.turbofish.as_ref())
        else {
            return;
        };
        crate::trace::record(
            tc.registry,
            &tc.sink.file(),
            syn::spanned::Spanned::span(&call.receiver),
            method,
            &found,
        );
    }

    /// Does the port write this call as the value itself, where Rust wrote a
    /// `Result` around it?
    ///
    /// `lock()`, `read()` and `write()` hand back the guard, and
    /// `bincode::serialize`, `bincode::deserialize` and `serde_json::to_string`
    /// hand back what they produced. In every one of them the `Ok` the Rust
    /// source wrote has nothing to test and the `unwrap` after it nothing to do.
    pub fn writes_the_value_not_the_result(&self, expr: &syn::Expr) -> bool {
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        tc.is_lock_call(expr)
    }

    /// The type of the value a `match` or an `if let` takes apart. `None` means
    /// the engine could not read it, and the fallback has been recorded.
    pub fn scrutinee_type(&self, expr: &syn::Expr) -> Option<crate::ty::Ty> {
        let resolved = self.resolve_expr_type(expr);
        self.or_fallback(resolved, "the pattern's names are bound without types")
    }

    /// Write `body` with the pattern lowering told what the value being taken
    /// apart IS, for a site that writes the test after its binding scope has
    /// closed.
    ///
    /// `enter_pattern` sets the same flag, and most sites write the test inside
    /// the scope it opens; the `if let` lowering writes its branch first and its
    /// test afterwards, so it says so here instead.
    pub(crate) fn matching<R>(
        &self,
        scrutinee: Option<&crate::ty::Ty>,
        body: impl FnOnce() -> R,
    ) -> R {
        let held = self
            .borrowed_subject
            .replace(matches!(scrutinee, Some(crate::ty::Ty::Ref { .. })));
        let written = body();
        self.borrowed_subject.set(held);
        written
    }

    /// Is the value the pattern being written is matched against borrowed?
    ///
    /// `enter_pattern` sets it from the scrutinee's type and the scope it opens
    /// restores it, so it answers for the pattern currently being written.
    pub(crate) fn matches_a_reference(&self) -> bool {
        self.borrowed_subject.get()
    }

    /// Open a scope holding the names a pattern introduces, typed from the value
    /// being taken apart. The scope closes when the returned guard drops.
    ///
    /// A name the engine could not type is still bound, so that it shadows and
    /// so that a use of it says "bound but untyped" rather than "does not name a
    /// value". The gap is reported where the name is used, which is where the
    /// translator has to fall back; reporting it here as well would count one
    /// gap twice.
    pub fn enter_pattern<'t>(
        &'t self,
        pat: &syn::Pat,
        scrutinee: Option<&crate::ty::Ty>,
    ) -> PatternScope<'t, 'a> {
        self.push_block();
        let borrowed_before = self
            .borrowed_subject
            .replace(matches!(scrutinee, Some(crate::ty::Ty::Ref { .. })));
        self.bind_pattern_here(pat, scrutinee);
        PatternScope { translator: self, borrowed_before }
    }

    /// Write out what a native-type translation decided, reporting the calls it
    /// had to refuse.
    pub(crate) fn render_translation(
        &self,
        translated: native_types::MethodTranslation,
        receiver: &str,
        ts_method: &str,
        args: &[String],
        span: proc_macro2::Span,
    ) -> String {
        match translated {
            native_types::MethodTranslation::Expr(result) => result,
            native_types::MethodTranslation::Passthrough => {
                format!("{}.{}({})", receiver, ts_method, args.join(", "))
            }
            native_types::MethodTranslation::Refused { message, fallback } => {
                self.fallback(span, message);
                self.render_translation(*fallback, receiver, ts_method, args, span)
            }
        }
    }

    /// Emit this name under a fresh identifier from here on.
    pub(crate) fn freshen(&self, name: &str) -> String {
        match &self.types {
            Some(tc) => tc.borrow_mut().shadow(name),
            None => name.to_string(),
        }
    }

    /// This pattern written out, with each of the names that shadows something
    /// already in scope emitted under a fresh identifier.
    pub(crate) fn freshened_pattern(&self, pat: &syn::Pat, shadowing: &[String]) -> String {
        let fresh: Vec<(String, String)> = shadowing
            .iter()
            .map(|name| (name.clone(), self.freshen(name)))
            .collect();
        Self::pat_render(pat, &|name| {
            fresh
                .iter()
                .find(|(from, _)| from == name)
                .map(|(_, to)| to.clone())
                .unwrap_or_else(|| name.to_string())
        })
    }

    /// The same rename applied to a pattern's BINDING text, for a form whose
    /// names are written out as declarations rather than as one pattern.
    ///
    /// Only the DECLARING half of each line is rewritten — everything before
    /// the `=` — because the initialiser is written in the scope the binding is
    /// shadowing. The three shapes the pattern machinery produces:
    /// `const x = e;`, `const [a, b] = e;` and `const { k: v } = e;`, the last
    /// of which may be written shorthand.
    pub(crate) fn freshen_bindings(&self, bind: String, shadowing: &[String]) -> String {
        if shadowing.is_empty() {
            return bind;
        }
        let fresh: Vec<(String, String)> = shadowing
            .iter()
            .map(|name| (name.clone(), self.freshen(name)))
            .collect();
        let mut out = String::new();
        for line in bind.lines() {
            match line.split_once(" = ") {
                Some((head, tail)) => {
                    out.push_str(&rename_declared(head, &fresh));
                    out.push_str(" = ");
                    out.push_str(tail);
                }
                None => out.push_str(line),
            }
            out.push('\n');
        }
        out
    }

    /// Does this expression name a value the body holds in a runtime cell?
    pub(crate) fn names_a_cell(&self, expr: &syn::Expr) -> bool {
        let syn::Expr::Path(path) = expr else { return false };
        if path.path.segments.len() != 1 {
            return false;
        }
        let written = Self::path_static(&path.path);
        let written = self.emitted_name(&written).unwrap_or(written);
        self.boxed.borrow().iter().any(|name| *name == written)
    }

    /// Does this expression name a PARAMETER the body holds in a cell? Such a
    /// name is the reference itself, so handing it to another `&mut` parameter
    /// hands the cell over rather than a copy of what is inside it.
    pub(crate) fn names_a_cell_param(&self, expr: &syn::Expr) -> bool {
        let syn::Expr::Path(path) = expr else { return false };
        if path.path.segments.len() != 1 {
            return false;
        }
        let written = Self::path_static(&path.path);
        let written = self.emitted_name(&written).unwrap_or(written);
        self.cell_params.borrow().iter().any(|name| *name == written)
    }

    /// Which helper a call on a bound closure goes through, where the callee
    /// names a parameter or local whose type is a CALLABLE BOUND — `F: FnOnce(..)`,
    /// `impl Fn(..)`, `Box<dyn FnMut(..)>`.
    ///
    /// R10: whether a closure needed wrapping is a property of what it
    /// CAPTURED, and a callee cannot see that. `f(x)` written in such a callee
    /// raised `TypeError: f is not a function` the moment a caller handed it an
    /// `OwnedClosure` — three live sites did. Every call on one goes through
    /// one of base's two helpers, which tell the two shapes apart.
    ///
    /// `invoke` calls and then releases, which is right where the CALL is what
    /// consumes the closure — an `FnOnce` bound, and only when the parameter is
    /// written by value. `invokeRef` calls and leaves the closure whole, which
    /// is right for an `Fn` or `FnMut` bound (the call borrows, and the body may
    /// call again) and for anything written `&F` or `&mut F`, where the closure
    /// is somebody else's.
    ///
    /// A by-value `Fn`/`FnMut` parameter is still the body's to release — Rust
    /// drops it at the end — and that release is the parameter's, not the
    /// call's: `claim_params` writes it.
    pub(crate) fn bound_closure_helper(&self, callee: &syn::Expr) -> Option<&'static str> {
        let syn::Expr::Path(path) = callee else { return None };
        if path.path.segments.len() != 1 {
            return None;
        }
        let tc = self.types.as_ref()?;
        let ty = self.quietly(|| self.resolve_expr_type(callee)).ok()?;
        // A BOUND, not a concrete closure. A local the emitter wrote as a plain
        // arrow is called as one; what a caller may have wrapped is a value
        // whose type the callee knows only through a bound.
        if !matches!(
            ty.peel_refs(),
            crate::ty::Ty::Param(_) | crate::ty::Ty::ImplTrait { .. } | crate::ty::Ty::Dyn { .. }
        ) {
            return None;
        }
        let tc = tc.borrow();
        crate::infer::expected::fn_shape(tc.registry, &ty, &tc.param_bounds)?;
        // A parameter written `&F` or `&mut F` is the caller's whatever it is
        // bounded by, so no call on it may release it.
        let borrowed = matches!(ty, crate::ty::Ty::Ref { .. });
        let once = !borrowed
            && crate::infer::expected::consumed_by_the_call(
                tc.registry,
                &ty,
                &tc.param_bounds,
            );
        Some(if once { "invoke" } else { "invokeRef" })
    }

    /// Note that this body holds `name` in a runtime cell.
    pub(crate) fn hold_in_a_cell(&self, name: &str) {
        let mut boxed = self.boxed.borrow_mut();
        if !boxed.iter().any(|held| held == name) {
            boxed.push(name.to_string());
        }
    }

    /// Does the statement being written throw this call's value away?
    pub(crate) fn discards(&self, call: &syn::ExprMethodCall) -> bool {
        let at = syn::spanned::Spanned::span(&call.method).start();
        self.discarded_call.get() == Some((at.line, at.column))
    }

    /// Is this call the one a `*` is about to write through?
    ///
    /// `*map.entry(k).or_insert(0) += 1` stores into the map, and the deref arm
    /// wants the write-through `Slot` the finisher hands back. Every other use
    /// of the same finisher reads the VALUE the slot holds.
    pub(crate) fn is_written_through(&self, call: &syn::ExprMethodCall) -> bool {
        let at = syn::spanned::Spanned::span(&call.method).start();
        self.written_through.get() == Some((at.line, at.column))
    }

    /// Write `body` with `expr` marked as the place a `*` reaches through.
    pub(crate) fn through_place<R>(&self, expr: &syn::Expr, body: impl FnOnce() -> R) -> R {
        let marked = match expr {
            syn::Expr::MethodCall(call) => {
                let at = syn::spanned::Spanned::span(&call.method).start();
                self.written_through.replace(Some((at.line, at.column)))
            }
            _ => self.written_through.replace(None),
        };
        let out = body();
        self.written_through.set(marked);
        out
    }

    /// Note every local this block binds to a closure that hands a capture
    /// away, before the move scan asks which calls consume their callee.
    ///
    /// The scan runs over the whole block before any of it is translated, so
    /// the `let` that introduces such a closure has not been seen yet when the
    /// call to it is scanned. This reads the `let`s first.
    pub(crate) fn note_once_closures(&self, stmts: &[syn::Stmt]) {
        for stmt in stmts {
            let syn::Stmt::Local(local) = stmt else { continue };
            let Some(init) = &local.init else { continue };
            let Some(closure) = as_move_closure(&init.expr) else { continue };
            if self.owned_captures(closure).is_empty() || !self.hands_a_capture_away(closure) {
                continue;
            }
            let name = Self::pat_static(&local.pat);
            let mut once = self.own.once_closure_locals.borrow_mut();
            if !once.iter().any(|n| *n == name) {
                once.push(name);
            }
        }
    }

    /// Does this closure's body hand one of its captures away?
    ///
    /// Rust reads such a closure as an `FnOnce`: running it moves the capture
    /// into the body, so the closure has nothing left to drop afterwards.
    pub(crate) fn hands_a_capture_away(&self, closure: &syn::ExprClosure) -> bool {
        !crate::ownership::Scan::new(self).moved_captures(closure).is_empty()
    }

    /// Does this expression name something the body can reach?
    ///
    /// A format string's implicit capture is a name in the enclosing scope, and
    /// rustc refuses the macro where there is none — so a name the engine
    /// cannot find is a name the emitted template would read and not find
    /// either.
    pub(crate) fn names_something(&self, expr: &syn::Expr) -> bool {
        let Some(tc) = self.types.as_ref() else { return true };
        let tc = tc.borrow();
        let mark = tc.sink.mark();
        let resolved = tc.resolve_expr(expr);
        tc.sink.rewind(mark);
        resolved.is_ok()
    }

    /// The identifier a bound name is emitted under, which differs from the one
    /// the source wrote wherever a shadow was freshened.
    pub(crate) fn emitted_name(&self, name: &str) -> Option<String> {
        self.types.as_ref().and_then(|tc| tc.borrow().emitted_name(name))
    }

    /// The type a `let` introduces, asked before the name is bound so that the
    /// initialiser is read in the scope it shadows.
    pub(crate) fn resolve_local(&self, local: &syn::Local) -> Option<crate::ty::Ty> {
        let tc = self.types.as_ref()?;
        let resolved = tc.borrow().resolve_local_type(local);
        let pat = Self::pat_static(&local.pat);
        // Saying only that the type is missing left the ownership consequence
        // unsaid, and an untyped local is exactly one nothing releases.
        let instead = format!(
            "local `{}` is left untyped, so nothing releases whatever it holds",
            pat
        );
        self.or_fallback(resolved, &instead)
    }

    /// Bind a pattern's names in the scope that is already open. Used where the
    /// binding outlives the statement, as a `let` does.
    pub fn bind_pattern_here(&self, pat: &syn::Pat, scrutinee: Option<&crate::ty::Ty>) {
        let Some(tc) = &self.types else { return };
        tc.borrow_mut().bind_pattern(pat, scrutinee);
    }

}

/// Rename the names a declaration head introduces, leaving the KEYS of an
/// object pattern alone: `{ _0: path }` names the key `_0` and binds `path`,
/// and `{ path }` is the same thing written shorthand — which has to become
/// `{ path: path_1 }` rather than `{ path_1 }`, because the key is the payload's
/// and only the binding moves.
fn rename_declared(head: &str, fresh: &[(String, String)]) -> String {
    let mut out = String::new();
    let mut rest = head;
    let mut in_object = 0usize;
    while !rest.is_empty() {
        let take = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
            .unwrap_or(rest.len());
        if take == 0 {
            let ch = rest.chars().next().expect("not empty");
            match ch {
                '{' => in_object += 1,
                '}' => in_object = in_object.saturating_sub(1),
                _ => {}
            }
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        let word = &rest[..take];
        rest = &rest[take..];
        // A key is followed by `:`; the name after it is the binding.
        let is_key = in_object > 0 && rest.trim_start().starts_with(':');
        match fresh.iter().find(|(from, _)| from == word) {
            Some((from, to)) if !is_key && word != "const" && word != "let" => {
                // Shorthand `{ x }` becomes `{ x: x_1 }`: the key stays the
                // payload's and only the binding moves.
                let shorthand = in_object > 0
                    && !out.trim_end().ends_with(':');
                if shorthand {
                    out.push_str(&format!("{}: {}", from, to));
                } else {
                    out.push_str(to);
                }
            }
            _ => out.push_str(word),
        }
    }
    out
}
