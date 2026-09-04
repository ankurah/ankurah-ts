//! Phase 2: Body translation — syn::Expr/Stmt → TS expression/statement text
//!
//! Translates Rust function bodies to TypeScript. Each syn expression variant
//! maps to a TS expression string. The output is deterministic and structural,
//! prioritizing 1:1 correspondence with the Rust source over elegance.

use syn;
use crate::name_map;
use crate::macros;
use crate::match_expr;
use crate::control_flow;
use crate::ownership;
use crate::native_types;

/// Check if an expression is a write!/writeln! macro call
fn is_write_macro(expr: &syn::Expr) -> bool {
    if let syn::Expr::Macro(mac) = expr {
        let name = mac.mac.path.segments.last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        matches!(name.as_str(), "write" | "writeln")
    } else {
        false
    }
}

/// Extract the Macro from an expression (for write! detection)
fn extract_macro(expr: &syn::Expr) -> Option<&syn::Macro> {
    if let syn::Expr::Macro(mac) = expr {
        Some(&mac.mac)
    } else {
        None
    }
}

/// Check if a match expression has arms that are write! macro calls (Display pattern)
fn is_match_with_write_arms(expr: &syn::Expr) -> bool {
    if let syn::Expr::Match(m) = expr {
        m.arms.iter().any(|arm| {
            matches!(&*arm.body, syn::Expr::Try(t) if is_write_macro(&t.expr))
        })
    } else {
        false
    }
}

/// Extract a single expression from a block (for ternary conversion)
fn single_block_expr(block: &syn::Block) -> Option<&syn::Expr> {
    if block.stmts.len() == 1 {
        if let syn::Stmt::Expr(expr, _) = &block.stmts[0] {
            return Some(expr);
        }
    }
    None
}

// ── Public entry points ─────────────────────────────────────────────────

/// Translate a single expression (used by match_expr, control_flow, macros modules)
pub fn translate_expr(expr: &syn::Expr) -> String {
    BodyTranslator::new("Self").expr(expr)
}

/// Translate a pattern (used by match_expr, control_flow modules)
pub fn translate_pat(pat: &syn::Pat) -> String {
    BodyTranslator::pat_static(pat)
}

/// Indent each line by 2 spaces
pub fn indent(s: &str) -> String {
    s.lines()
        .map(|line| if line.is_empty() { String::new() } else { format!("  {}", line) })
        .collect::<Vec<_>>()
        .join("\n")
        + if s.ends_with('\n') { "\n" } else { "" }
}

/// Where an expression sits in the file, as the two ends of its span. Two
/// expressions are the same one when they start and end in the same place,
/// which is what an expectation is matched against.
type Position = ((usize, usize), (usize, usize));

fn position_of(expr: &syn::Expr) -> Position {
    span_position(syn::spanned::Spanned::span(expr))
}

fn span_position(span: proc_macro2::Span) -> Position {
    let (start, end) = (span.start(), span.end());
    ((start.line, start.column), (end.line, end.column))
}

// ── Translator struct ───────────────────────────────────────────────────

pub struct BodyTranslator<'a> {
    pub self_type: &'a str,
    /// Type context for comprehensive expression type resolution.
    /// Wraps ScopeStack + TypeRegistry. None for legacy codepaths
    /// (free function shims, match_expr, control_flow).
    pub types: Option<std::cell::RefCell<crate::infer::TypeContext<'a>>>,
    /// Inline module names for the current file — path qualifiers
    /// stripped during path resolution (symbols imported from separate .ts file).
    pub inline_module_names: Vec<String>,
    /// Names the enclosing block-as-expression already supplies as arrow
    /// function parameters. The `let` that would have introduced each of them
    /// has nothing left to do, and re-declaring one would shadow the value that
    /// was threaded in.
    threaded: std::cell::RefCell<Vec<String>>,
    /// How many temporaries this body has taken, so that one `if let` nested in
    /// another does not read both scrutinees into the same name.
    temporaries: std::cell::Cell<usize>,
    /// What the enclosing function returns, resolved. `?` reads it to say
    /// whether the error it propagates needs a conversion, and the `Result`
    /// lowering reads it to know it is inside a function that returns one.
    pub fn_return: Option<crate::ty::Ty>,
    /// Whether this method took `self` by value. A `&self` method lends the
    /// receiver, so nothing inside it can move a field out of `self` or match
    /// it by value — Rust would refuse both, and emitting them left the caller
    /// holding a value the body had marked moved.
    pub owns_self: bool,
    /// What this body still owes: the values in scope, the declarations lifted
    /// out of the statement being written, and the drop flags. Every decision
    /// that reads or writes it lives in `ownership::lowering`.
    pub own: crate::ownership::Lowering,
    /// What the position an expression stands in says its type has to be, and
    /// which expression that position holds.
    ///
    /// Rust settles a great deal from context that the expression itself does
    /// not carry — the width of a literal, the target of an `.into()`, a
    /// closure's parameter types — so the position hands the expression a type
    /// to be. The span is part of the slot because a sub-expression is often
    /// translated before its parent, and it must not take an answer meant for
    /// the parent.
    expecting: std::cell::RefCell<Option<(Position, crate::ty::Ty)>>,
    /// The identifier Rust's `self` is emitted as in this body.
    ///
    /// A method on an emitted class writes `this`. A method whose impl is
    /// written for a type that has no class of its own — a blanket impl, or an
    /// impl on `Arc<Inner<T>>` — is emitted as a module-level function that
    /// takes its receiver as an ordinary first parameter, and there `self` is
    /// that parameter's name.
    pub self_name: &'a str,
}

impl<'a> BodyTranslator<'a> {
    pub fn new(self_type: &'a str) -> Self {
        Self {
            self_type,
            types: None,
            inline_module_names: vec![],
            threaded: std::cell::RefCell::new(Vec::new()),
            temporaries: std::cell::Cell::new(0),
            fn_return: None,
            owns_self: false,
            own: Default::default(),
            expecting: std::cell::RefCell::new(None),
            self_name: "this",
        }
    }

    pub fn with_context(self_type: &'a str, tc: crate::infer::TypeContext<'a>) -> Self {
        Self {
            self_type,
            types: Some(std::cell::RefCell::new(tc)),
            inline_module_names: vec![],
            threaded: std::cell::RefCell::new(Vec::new()),
            temporaries: std::cell::Cell::new(0),
            fn_return: None,
            owns_self: false,
            own: Default::default(),
            expecting: std::cell::RefCell::new(None),
            self_name: "this",
        }
    }

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

    /// What one turn of a `for` loop over this expression hands out.
    pub fn iteration_item(&self, iterated: &syn::Expr) -> Option<crate::ty::Ty> {
        let tc = self.types.as_ref()?;
        let ty = self.scrutinee_type(iterated)?;
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
        self.bind_pattern_here(pat, scrutinee);
        PatternScope { translator: self }
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

    // ── Block translation with ownership tracking ───────────────────

    /// A function's own body.
    ///
    /// Rust drops a by-value parameter when the function returns, so the body
    /// owns its parameters the way it owns its locals — released in the same
    /// `finally`, after everything the body itself declared, and not at all
    /// where the body hands one on.
    pub fn translate_fn_block(
        &self,
        block: &syn::Block,
        params: &[(String, crate::ty::Ty)],
    ) -> String {
        self.push_block();
        // The parameters are claimed before the body is written, because a
        // parameter handed away inside a branch needs its drop flag registered
        // by the time the branch that sets it is translated.
        let owned = self.claim_params(block, params);
        let body = self.translate_block_stmts(block);
        self.pop_scope();
        for owned in &owned {
            if let Some(source) = &owned.source {
                self.own.flags.borrow_mut().remove(source);
            }
        }
        let mut out = body;
        for param in owned.iter().rev() {
            out = ownership::wrap(&out, param);
        }
        let mut declarations = String::new();
        for param in &owned {
            if let Some(flag) = &param.flag {
                declarations.push_str(&format!("let {} = false;\n", flag));
            }
        }
        format!("{}{}", declarations, out)
    }


    pub fn translate_block(&self, block: &syn::Block) -> String {
        // A Rust block is a scope: a `let` inside it shadows what is outside and
        // stops shadowing at the closing brace, and TypeScript's `const` in a
        // nested block does the same.
        self.push_block();
        let out = self.translate_block_stmts(block);
        self.pop_scope();
        out
    }

    /// A block's statements, with the releases the block owes written into it.
    ///
    /// Every value the block still owns when it ends is released in a
    /// `finally`, so that a `return`, a `?`, a `break` and a thrown fatal all
    /// leave through it — which is what Rust's drop glue does and what a run of
    /// drops at the end of the block did not. The `try` opens immediately after
    /// each declaration rather than at the top of the block: a `const` declared
    /// inside a `try` is not in scope in its `finally`, and opening one `try`
    /// per declaration also gets reverse declaration order for free, since the
    /// innermost `finally` runs first.
    pub(crate) fn translate_block_stmts(&self, block: &syn::Block) -> String {
        let stmts = &block.stmts;
        let dispositions = self.analyse_moves(stmts);
        let ordinals = std::cell::RefCell::new(std::collections::HashMap::new());
        self.emit_from(stmts, 0, &dispositions, &ordinals)
    }

    /// Statements `i..` of a block, with everything after an owning
    /// declaration nested inside that declaration's `try`.
    pub(crate) fn emit_from(
        &self,
        stmts: &[syn::Stmt],
        i: usize,
        dispositions: &ownership::Dispositions,
        ordinals: &std::cell::RefCell<std::collections::HashMap<String, usize>>,
    ) -> String {
        let Some(stmt) = stmts.get(i) else {
            return String::new();
        };
        // A brace-delimited macro at the end of a block is the block's value in
        // Rust, and `syn` parses it as `Stmt::Macro` rather than `Stmt::Expr` —
        // so the tail path used to walk past `select! { .. }` written last and
        // throw away the value the arm produced. A macro with a semicolon after
        // it is a statement like any other.
        let tail_macro = match stmt {
            syn::Stmt::Macro(mac) => mac.semi_token.is_none(),
            _ => false,
        };
        let is_tail =
            i + 1 == stmts.len() && (matches!(stmt, syn::Stmt::Expr(_, None)) || tail_macro);

        // What this statement's own `let` should do with each name it binds,
        // read before it is translated because `local()` acts on it.
        self.set_stmt_dispositions(stmt, dispositions, ordinals);

        // A move written directly by this statement sets its flag first: after
        // it would be dead code behind a `return`, and the flag only ever
        // decides what the `finally` releases.
        let mut out = self.flag_sets(stmt);

        let previous_prelude = std::mem::take(&mut *self.own.prelude.borrow_mut());
        let previous_pending = std::mem::take(&mut *self.own.pending.borrow_mut());
        let text = if is_tail {
            let held;
            let expr = match stmt {
                syn::Stmt::Expr(expr, None) => expr,
                syn::Stmt::Macro(mac) => {
                    held = syn::Expr::Macro(syn::ExprMacro {
                        attrs: mac.attrs.clone(),
                        mac: mac.mac.clone(),
                    });
                    &held
                }
                _ => unreachable!("the tail was just matched"),
            };
            // A block's tail leaves through the function's return type, so that
            // is what the tail expression is expected to produce (spec 4.6).
            let want = self.fn_return.clone();
            self.expecting(expr, want.as_ref(), || {
                format!(
                    "{}\n",
                    control_flow::translate_expr_in_return_position(expr, self)
                )
            })
        } else {
            self.stmt(stmt)
        };
        let prelude = std::mem::replace(&mut *self.own.prelude.borrow_mut(), previous_prelude);
        let owned = std::mem::replace(&mut *self.own.pending.borrow_mut(), previous_pending);

        let rest = self.emit_from(stmts, i + 1, dispositions, ordinals);
        // A drop flag is only readable while the local it stands for is in
        // scope. Taking it off again keeps a later block that reuses the name
        // from setting a flag nothing tests.
        for local in &owned {
            if let Some(source) = &local.source {
                self.own.flags.borrow_mut().remove(source);
            }
        }

        // A `let` puts a name in scope for everything after it, so a temporary
        // its initialiser lifted has to stay live that long too: both open a
        // `try` over the rest of the block, and the releases then happen in
        // reverse declaration order. Any other statement is over when it is
        // over, and Rust drops its temporaries there — which is what keeps a
        // lock taken in an argument from being held for the rest of the block.
        let declares = matches!(stmt, syn::Stmt::Local(_));
        let mut inner = text;
        if declares {
            // Releasing a guard at the end of its statement is what keeps the
            // lock from outliving the line that took it. Where the statement
            // *is* the rest of the block, the `finally` below says the same
            // thing, so only one of them is written.
            if !rest.trim().is_empty() {
                for hoist in prelude.iter().rev() {
                    if let Some(temp) = &hoist.owned {
                        inner.push_str(&temp.statement_release());
                    }
                }
            }
            let mut tail = rest;
            for local in owned.iter().rev() {
                tail = ownership::wrap(&tail, local);
            }
            inner.push_str(&tail);
            out.push_str(&ownership::hoisted(&inner, &prelude));
            return out;
        }
        out.push_str(&ownership::hoisted(&inner, &prelude));
        out.push_str(&rest);
        out
    }






    // ── Statement translation ───────────────────────────────────────


    pub(crate) fn stmt(&self, stmt: &syn::Stmt) -> String {
        match stmt {
            syn::Stmt::Local(local) => self.local(local),
            syn::Stmt::Expr(expr, semi) => {
                // Detect standalone `expr?;` — emit Result check
                if semi.is_some() {
                    if let syn::Expr::Try(try_expr) = expr {
                        // Special case: write!(f, ...)?; in Display impls — emit string append
                        if is_write_macro(&try_expr.expr) {
                            let fmt_str = macros::translate_macro(extract_macro(&try_expr.expr).unwrap(), self);
                            return format!("_result += {};\n", fmt_str);
                        }
                        // A `?` whose value nobody binds. Rust drops the `Ok`
                        // payload at the end of the statement, and the wrapper
                        // with it; `wrapper.drop()` cascades into both, which
                        // is why the wrapper is not simply abandoned here.
                        let lowered = self.lower_try(try_expr);
                        return match &lowered.wrapper {
                            Some(wrapper) => {
                                format!("{}{}.drop();\n", lowered.declaration, wrapper)
                            }
                            // An `Option` has no wrapper here — the port writes
                            // it as a nullable — but Rust still drops the `Some`
                            // payload at the end of the statement.
                            None => {
                                let release = self
                                    .release_of(&syn::Expr::Try(try_expr.clone()), &lowered.value)
                                    .unwrap_or_else(|| format!("{};", lowered.value));
                                format!("{}{}\n", lowered.declaration, release)
                            }
                        };
                    }
                }
                let ts = self.expr(expr);
                // If a match expression contains write! arms (Display pattern),
                // append the result to _result
                let ts = if is_match_with_write_arms(expr) {
                    format!("_result += {}", ts)
                } else {
                    ts
                };
                if semi.is_some() {
                    format!("{};\n", self.discard(expr, ts))
                } else {
                    format!("{}\n", ts)
                }
            }
            syn::Stmt::Item(_) => String::new(),
            syn::Stmt::Macro(macro_stmt) => {
                let ts = macros::translate_macro(&macro_stmt.mac, self);
                if macro_stmt.semi_token.is_some() {
                    format!("{};\n", ts)
                } else {
                    format!("{}\n", ts)
                }
            }
        }
    }

    pub(crate) fn local(&self, local: &syn::Local) -> String {
        let pat = Self::pat_static(&local.pat);

        let Some(init) = &local.init else {
            return format!("let {};\n", pat);
        };

        // Read before the initialiser is translated. An initialiser that is a
        // block of its own — `let sub = { let c = c.clone(); f(c) }` — runs the
        // whole block machinery again and leaves its own statement's answers
        // behind, so asking afterwards asked about the wrong statement and
        // dropped a local the outer block had already handed away.
        let disposition = self
            .own
            .stmt_dispositions
            .borrow()
            .get(&pat)
            .copied()
            .unwrap_or(ownership::Disposition::Kept);

        // Rust allows `let x = x.method()` to shadow; JavaScript refuses a
        // second declaration of the same name in the same block. This has to
        // be asked before the binding is made.
        let already_in_scope = self.redeclares_here(&pat);

        // The initialiser is translated before the binding exists, because
        // it is written in the scope the `let` is shadowing:
        // `let stack = stack.borrow_mut()` borrows the *outer* `stack`, and
        // binding first would resolve that receiver to the guard the line is
        // about to introduce and reach through it.
        // `let listener = move |..| ..` binds a closure that owns what it
        // captured, so the block releases it as it releases any owned local —
        // and asking the engine for a closure's type would only report a gap
        // this line has already closed.
        let bound_closure = as_move_closure(&init.expr)
            .filter(|closure| !self.owned_captures(closure).is_empty());
        // What the `let` wrote for itself is what its initialiser has to
        // produce (spec 4.6): `let f: Box<dyn Fn(u32)> = |x| ..` types `x`, and
        // `let n: u8 = 1` writes a byte rather than the `i32` a bare literal
        // defaults to.
        let annotation = self
            .types
            .as_ref()
            .and_then(|tc| tc.borrow().local_annotation(local));
        let ty = match bound_closure {
            Some(_) => None,
            None => self.expecting(&init.expr, annotation.as_ref(), || self.resolve_local(local)),
        };
        let expr = match bound_closure {
            Some(closure) => self.closure(
                closure,
                ownership::closures::Placement::Bound,
                annotation.as_ref(),
            ),
            None => self.expecting(&init.expr, annotation.as_ref(), || {
                self.moved_value(&init.expr)
            }),
        };

        // Only now does the name mean the new value. A `let` may take one
        // apart — `let (a, b) = ...`, `let Foo { x } = ...` — so every name
        // the pattern writes is bound, each typed from its own position.
        if self.types.is_some() {
            self.bind_pattern_here(&local.pat, ty.as_ref());
        }

        if let Some((_tok, _diverge)) = &init.diverge {
            return format!("/* let-else */ const {} = {};\n", pat, expr);
        }

        // A name the enclosing block-as-expression already threaded in as a
        // parameter is already this value; declaring it again would shadow
        // what was threaded.
        if self.threaded.borrow().iter().any(|n| *n == pat) {
            let rust_name = if let syn::Pat::Ident(ident) = &local.pat {
                ident.ident.to_string()
            } else {
                pat.clone()
            };
            if references_var(&init.expr, &rust_name) {
                return String::new();
            }
        }
        let keyword = if is_mut_binding(&local.pat) { "let" } else { "const" };
        // A Rust shadow introduces a *new* variable. Assigning to the old
        // one instead changed a value other code — a closure that captured
        // it, a caller that owns it — can still see. JavaScript will not
        // declare the same name twice here, so the shadow is emitted under a
        // fresh identifier and every later use of the name follows it.
        let emitted = if already_in_scope {
            self.freshen(&pat)
        } else {
            pat.clone()
        };
        if bound_closure.is_some() {
            self.own.owned_closure_locals.borrow_mut().push(emitted.clone());
        }
        let drops = bound_closure.map(|_| ownership::Drops::Own);
        let flag = self.claim_local(&pat, &emitted, ty.as_ref(), drops, &local.pat, disposition);
        format!("{}{} {} = {};\n", flag, keyword, emitted, expr)
    }


    // ── Pattern translation (static — no self_type needed) ──────────

    pub fn pat_static(pat: &syn::Pat) -> String {
        match pat {
            syn::Pat::Ident(ident) => {
                name_map::escape_reserved(&name_map::to_camel_case(&ident.ident.to_string()))
            }
            syn::Pat::Tuple(tuple) => {
                let parts: Vec<String> = tuple.elems.iter().map(Self::pat_static).collect();
                format!("[{}]", parts.join(", "))
            }
            syn::Pat::TupleStruct(ts) => {
                let parts: Vec<String> = ts.elems.iter().map(Self::pat_static).collect();
                parts.join(", ")
            }
            syn::Pat::Struct(s) => {
                let fields: Vec<String> = s.fields.iter().map(|f| {
                    let member = match &f.member {
                        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    let pat = Self::pat_static(&f.pat);
                    if member == pat { member } else { format!("{}: {}", member, pat) }
                }).collect();
                format!("{{ {} }}", fields.join(", "))
            }
            syn::Pat::Wild(_) => "_".to_string(),
            syn::Pat::Lit(_) => "/* pat literal */".to_string(),
            syn::Pat::Path(path) => Self::path_static(&path.path),
            syn::Pat::Reference(r) => Self::pat_static(&r.pat),
            syn::Pat::Type(t) => Self::pat_static(&t.pat),
            syn::Pat::Or(or_pat) => {
                let parts: Vec<String> = or_pat.cases.iter().map(Self::pat_static).collect();
                parts.join(" | ")
            }
            syn::Pat::Slice(slice) => {
                let parts: Vec<String> = slice.elems.iter().map(Self::pat_static).collect();
                format!("[{}]", parts.join(", "))
            }
            syn::Pat::Rest(_) => "...".to_string(),
            _ => "/* unknown pat */".to_string(),
        }
    }

    // ── Matches: what the arms do to the subject ────────────────────





    // ── Field reads and the places a value moves out of ─────────────

    /// A field read split into what stands to the left of the field name and
    /// the field name itself, with every wrapper the field sits behind written
    /// out — one accessor per hop the engine took to find it.
    pub(crate) fn field_parts(&self, field: &syn::ExprField) -> (String, String) {
        let base = self.expr(&field.base);
        let member = match &field.member {
            syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
            syn::Member::Unnamed(idx) => format!("_{}", idx.index),
        };
        // A body with no type context never went through `path_expr`, so a bare
        // `self` written there still has to become the receiver this body emits
        // it as. One that did is already correct and must not be rewritten:
        // taking a module-level function's `self` parameter back to `this` read
        // a binding that does not exist there.
        let base = if base == "self" { self.self_name.to_string() } else { base };
        // Reading a field off something the expression itself produced —
        // `m.lock().unwrap().n` — leaves that value with nobody to release it.
        // Rust drops it at the end of the statement, and it is the guard case
        // that shows: the mutex stayed locked for the life of the program.
        let base = self.hoist_produced(&field.base, base);
        let Some(tc) = &self.types else {
            return (base, member);
        };
        let found = tc.borrow().resolve_field_access(&field.base, &member);
        let instead = format!("`.{}` is emitted without a wrapper accessor", member);
        let Some(found) = self.or_fallback(found, &instead) else {
            return (base, member);
        };
        let mut receiver = base;
        for accessor in found.accessors() {
            receiver.push('.');
            receiver.push_str(&accessor);
        }
        (receiver, member)
    }




    /// A call written as the module-level function its impl was emitted as.
    ///
    /// The receiver goes first, as it does in Rust's own reading of a method.
    /// Where the impl is a blanket one the engine picked it without knowing
    /// what the receiver will be at run time — the bound is what the receiver
    /// has to satisfy, and several impls of the trait may satisfy it — so the
    /// site says which impl was written and which ones the emitted call cannot
    /// reach.
    fn render_free_call(
        &self,
        free: &crate::emit_impls::FreeCall,
        receiver: &str,
        args: &[String],
        call: &syn::ExprMethodCall,
    ) -> String {
        if free.is_blanket {
            self.fallback(
                syn::spanned::Spanned::span(call),
                format!(
                    "`{}` here dispatches through a bound the engine cannot close, so the \
                     call is written as `{}`, the blanket impl's function; a receiver that \
                     one of the trait's other impls is written for reaches the wrong one",
                    call.method, free.name
                ),
            );
        }
        let mut written = vec![receiver.to_string()];
        written.extend(args.iter().cloned());
        format!("{}({})", free.name, written.join(", "))
    }

    /// What the receiver of a wrapper-opening call is expected to produce.
    ///
    /// `unwrap`, `expect` and their kind hand back what a `Result` or an
    /// `Option` carries, so a position that wants a `T` of the call wants a
    /// `Result<T, E>` or an `Option<T>` of the receiver — which is how
    /// `serde_json::from_str(&s).unwrap()` learns, from the value it is
    /// compared against, which type reads itself out of the parsed text. Every
    /// other method leaves the receiver's type to the receiver.
    fn receiver_expectation(
        &self,
        call: &syn::ExprMethodCall,
        expected: Option<&crate::ty::Ty>,
    ) -> Option<crate::ty::Ty> {
        const OPENS_A_WRAPPER: [&str; 4] = ["unwrap", "expect", "unwrap_or_default", "ok"];
        let want = expected?;
        if !OPENS_A_WRAPPER.contains(&call.method.to_string().as_str()) {
            return None;
        }
        let tc = self.types.as_ref()?;
        let receiver = self.quietly(|| self.resolve_expr_type(&call.receiver)).ok()?;
        // The wrapper is the receiver's own, so the expectation is written
        // inside whichever one it turns out to be rather than guessed at.
        let crate::ty::Ty::Named { id, args } = receiver.peel_refs() else {
            return None;
        };
        let tc = tc.borrow();
        if !tc.is_result(&receiver) && !tc.is_option(&receiver) {
            return None;
        }
        Some(crate::ty::Ty::Named {
            id: *id,
            args: std::iter::once(want.clone())
                .chain(args.iter().skip(1).cloned())
                .collect(),
        })
    }

    /// A list literal, written as the runtime type the position wants.
    ///
    /// `Vec<u8>` and `[u8; N]` are a `Uint8Array` in the port, and a plain
    /// JavaScript array compares unequal to one — which is what
    /// `assert_eq!(bytes, [1, 2, 3])` was failing on.
    pub(crate) fn sequence_literal(
        &self,
        items: Vec<String>,
        expected: Option<&crate::ty::Ty>,
    ) -> String {
        let list = format!("[{}]", items.join(", "));
        let bytes = match (expected, &self.types) {
            (Some(want), Some(tc)) => {
                crate::infer::expected::expects_bytes(tc.borrow().registry, want)
            }
            _ => false,
        };
        if bytes {
            format!("new Uint8Array({})", list)
        } else {
            list
        }
    }

    /// What each field of a struct literal is declared to hold.
    pub(crate) fn struct_field_types(
        &self,
        lit: &syn::ExprStruct,
    ) -> Vec<(String, crate::ty::Ty)> {
        match &self.types {
            Some(tc) => self.quietly(|| tc.borrow().struct_literal_field_types(lit)),
            None => Vec::new(),
        }
    }

    /// Which type reads itself out of a `serde_json::from_str` or a
    /// `bincode::deserialize`.
    ///
    /// Rust takes it from the turbofish where one is written and from the
    /// position otherwise — `let id: EntityId = bincode::deserialize(&bytes)?`
    /// names `EntityId` as surely as `deserialize::<EntityId>` does — so both
    /// are read here, the turbofish first because it is what the source said.
    fn read_into_type(
        &self,
        callee: Option<&syn::Path>,
        span: proc_macro2::Span,
    ) -> Option<String> {
        if let Some(written) = turbofish_type(callee) {
            return Some(written);
        }
        let want = self.expectation_at(span)?;
        let tc = self.types.as_ref()?;
        // The call yields the value itself; a `Result` or an `Option` around it
        // belongs to the `?` or the `unwrap` that follows, not to the type that
        // reads itself out of the bytes.
        let payload = tc.borrow().unwrapped_payload(&want);
        Some(name_map::map_ty(tc.borrow().registry, &payload))
    }

    /// What the callee of a method call declares each of its arguments to be.
    ///
    /// Asking is not translating: the resolution files whatever it could not
    /// settle, and the call is resolved again when it is written, so the record
    /// is wound back and the same gap is not counted twice.
    pub(crate) fn argument_types(&self, call: &syn::ExprMethodCall) -> Vec<Option<crate::ty::Ty>> {
        let Some(tc) = &self.types else {
            return Vec::new();
        };
        let method = call.method.to_string();
        let found = self.quietly(|| {
            tc.borrow()
                .resolve_method_call_with(&call.receiver, &method, call.turbofish.as_ref())
        });
        let Ok(found) = found else {
            return Vec::new();
        };
        // The bound is written in the callee's own terms — `FnMut(Self::Item)`
        // — so the projection is settled against the receiver before the
        // closure is asked to take its parameter from it. A projection left
        // standing is not a type anything inside the closure can be resolved
        // against.
        let tc = tc.borrow();
        let probe = tc.probe();
        tc.registry
            .method_param_types(&found)
            .into_iter()
            .map(|ty| Some(probe.normalize(&ty)))
            .collect()
    }

    // ── Expression translation ──────────────────────────────────────

    pub fn expr(&self, expr: &syn::Expr) -> String {
        let expected = self.expectation_for(expr);
        match expr {
            syn::Expr::Lit(lit) => translate_lit(&lit.lit),
            syn::Expr::Path(path) => self.path_expr(&path.path),

            syn::Expr::Field(field) => {
                let (receiver, member) = self.field_parts(field);
                format!("{}.{}", receiver, member)
            }

            syn::Expr::MethodCall(call) => {
                // `unwrap` and its kind take a wrapper apart, so what the
                // position wants of the whole is what it wants of the payload:
                // `assert_eq!(id, from_str(&s).unwrap())` says the parse
                // produces an `EntityId`, and nothing else does.
                let through = self.receiver_expectation(call, expected.as_ref());
                let receiver = self.expecting(&call.receiver, through.as_ref(), || {
                    self.expr(&call.receiver)
                });
                let receiver = self.hoist_receiver(call, receiver);
                let rust_method = call.method.to_string();
                let ts_method = name_map::map_fn_name(&rust_method);

                // The callee's signature is what says what each argument has to
                // be: a closure takes its parameter types from there, an
                // `.into()` its target, and a literal its width.
                let want = self.argument_types(call);
                let args: Vec<String> = call
                    .args
                    .iter()
                    .enumerate()
                    .map(|(index, a)| {
                        self.expecting(a, want.get(index).and_then(|t| t.as_ref()), || {
                            self.moved_value(a)
                        })
                    })
                    .collect();

                // ── unwrap/expect: single decision point ──
                // In TS, only Result has a real .unwrap(). All other types
                // (guards, Option/nullable, etc.) treat it as identity.
                if matches!(rust_method.as_str(), "unwrap" | "expect") {
                    // A `Result` the port really builds is unwrapped; a lock
                    // call's `LockResult` was never built, because the port's
                    // `lock()` hands back the guard. Everything else — a guard,
                    // a nullable — has no `unwrap` of its own and the call
                    // writes nothing.
                    let receiver_ty = self.resolve_expr_type(&call.receiver);
                    let instead = format!("`{}` is treated as the identity", rust_method);
                    let is_result = self
                        .or_fallback(receiver_ty, &instead)
                        .and_then(|ty| self.types.as_ref().map(|tc| tc.borrow().is_result(&ty)))
                        .unwrap_or(false)
                        && !self.writes_the_value_not_the_result(&call.receiver);
                    if !is_result {
                        // The call still resolved — `unwrap` on a `LockResult`
                        // is `Result::unwrap` and hands back the guard — so it
                        // is recorded before the runtime's own answer is
                        // written, which is nothing at all.
                        self.record_resolution(call, &rust_method);
                        return receiver.to_string();
                    }
                }
                // `??` reads a *value* for null, and a `Result` is an object:
                // `r ?? d` always takes `r`, whatever it holds. Only the
                // nullable the port maps `Option` to can be written that way;
                // a `Result` calls the runtime's own method, which consumes the
                // receiver as Rust's does.
                if matches!(rust_method.as_str(), "unwrap_or" | "unwrap_or_else") && args.len() == 1
                {
                    let nullable = self
                        .resolve_expr_type(&call.receiver)
                        .ok()
                        .is_some_and(|ty| self.is_nullable(&ty));
                    if nullable {
                        if rust_method == "unwrap_or" {
                            // Rust evaluates the default before it knows whether
                            // it is wanted, and drops it where it is not. `??`
                            // alone left it to the leak registry.
                            if let Some(chosen) =
                                self.nullable_default(&receiver, &call.args[0], &args[0])
                            {
                                return chosen;
                            }
                        }
                        return match rust_method.as_str() {
                            "unwrap_or" => format!("{} ?? {}", receiver, args[0]),
                            _ => format!("{} ?? ({})()", receiver, args[0]),
                        };
                    }
                }

                // ── the resolved call ──
                // The engine says which function this is and what has to be
                // written between the receiver and it: one accessor per wrapper
                // the chain went through. The translation is then chosen by the
                // type the callee is actually written for, not by the method's
                // name.
                if let Some(tc) = &self.types {
                    let found = tc.borrow().resolve_method_call_with(
                        &call.receiver,
                        &rust_method,
                        call.turbofish.as_ref(),
                    );
                    let instead = format!("`{}` is dispatched by name", rust_method);
                    if let Some(found) = self.or_fallback(found, &instead) {
                        let mut recv = receiver.clone();
                        for accessor in found.accessors() {
                            recv = format!("{}.{}", recv, accessor);
                        }
                        let tc_ref = tc.borrow();
                        crate::trace::record(
                            tc_ref.registry,
                            &tc_ref.sink.file(),
                            syn::spanned::Spanned::span(&call.receiver),
                            &rust_method,
                            &found,
                        );
                        // An impl written for a type with no emitted class is a
                        // module-level function, and the receiver is its first
                        // argument.
                        if let Some(free) = crate::emit_impls::free_call(tc_ref.registry, &found) {
                            drop(tc_ref);
                            return self.render_free_call(&free, &recv, &args, call);
                        }
                        let translated = native_types::translate_method(
                            tc_ref.registry,
                            found.receiver_type(),
                            &recv,
                            &rust_method,
                            &args,
                        );
                        drop(tc_ref);
                        return self.render_translation(
                            translated,
                            &recv,
                            &ts_method,
                            &args,
                            syn::spanned::Spanned::span(call),
                        );
                    }
                }

                self.translate_unresolved_call(&receiver, &rust_method, &ts_method, &args, Some(&call.receiver))
            }

            syn::Expr::Call(call) => {
                // `drop(x)` runs x's glue where the source says, and what that
                // costs is the glue engine's answer, not always `.drop()`: a
                // `Vec<Owned>` is a JavaScript array and has no method of its
                // own.
                if let Some(released) = self.explicit_drop(call) {
                    return released;
                }
                // `(move || …)()` is created, called and dropped in the one
                // expression, so what it captured is released inside it.
                if let Some(closure) = as_closure(&call.func) {
                    let arrow =
                        self.closure(closure, ownership::closures::Placement::Immediate, None);
                    let args: Vec<String> =
                        call.args.iter().map(|a| self.moved_value(a)).collect();
                    return format!("({})({})", arrow, args.join(", "));
                }
                let func = self.expr(&call.func);
                // What the callee declares each argument to be, with the
                // position the call stands in used to close whatever the
                // signature left open. This is what types the closure in
                // `Box::new(move |level| ..)`, whose parameter the signature
                // alone says nothing about.
                let want = match &self.types {
                    Some(tc) => {
                        self.quietly(|| tc.borrow().call_argument_types(call, expected.as_ref()))
                    }
                    None => Vec::new(),
                };
                let args: Vec<String> = call
                    .args
                    .iter()
                    .enumerate()
                    .map(|(index, a)| {
                        self.expecting(a, want.get(index).and_then(|t| t.as_ref()), || {
                            self.moved_value(a)
                        })
                    })
                    .collect();
                // An `OwnedClosure` is deliberately not a bare callable, so a
                // call that reached the body without a liveness check would be
                // exactly the bug it exists to catch.
                if self.own.owned_closure_locals.borrow().iter().any(|n| *n == func) {
                    return format!("{}.call({})", func, args.join(", "));
                }
                let path = match &*call.func {
                    syn::Expr::Path(path) => Some(&path.path),
                    _ => None,
                };
                self.translate_call(&func, &args, syn::spanned::Spanned::span(call), path)
            }

            // `*place += value` reads and writes through the wrapper, and the
            // `*place` half is written by the deref arm below like any other
            // deref — which is what names the temporary it produced and
            // releases it. Writing it here instead released nothing.
            syn::Expr::Binary(bin) => {
                // `*place += value` stores through the wrapper, so the target is
                // written as a place; everything else about it is an ordinary
                // binary expression, and the deref arm below hoists whatever
                // the target produced either way.
                if is_assign_op(&bin.op) {
                    if let syn::Expr::Unary(unary) = &*bin.left {
                        if matches!(unary.op, syn::UnOp::Deref(_)) {
                            let place = self.deref_place(unary);
                            let op = translate_binop(&bin.op);
                            return format!("{} {} {}", place, op, self.expr(&bin.right));
                        }
                    }
                }
                let left = self.expr(&bin.left);
                // `&&` and `||` evaluate their right operand only if the left
                // one allows it, so anything that operand took to evaluate
                // itself belongs inside the branch the short circuit guards.
                if matches!(bin.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
                    let (right, lifted) = self.with_own_hoists(|| self.expr(&bin.right));
                    let right = self.short_circuit_operand(&bin.right, right, lifted);
                    return format!("{} {} {}", left, translate_binop(&bin.op), right);
                }
                format!("{} {} {}", left, translate_binop(&bin.op), self.expr(&bin.right))
            }

            syn::Expr::Unary(unary) => {
                let e = self.expr(&unary.expr);
                match &unary.op {
                    syn::UnOp::Not(_) => {
                        if e.contains("===") || e.contains("!==") || e.contains(">=") || e.contains("<=") {
                            format!("!({})", e)
                        } else {
                            format!("!{}", e)
                        }
                    }
                    syn::UnOp::Neg(_) => format!("-{}", e),
                    // `*guard` reads what the container holds, which the port
                    // writes as a field on the guard. Emitting the guard itself
                    // handed the guard where the value was wanted, and a guard
                    // the statement produced was never released — the lock
                    // stayed held for the life of the program.
                    syn::UnOp::Deref(_) => {
                        let written = self.hoist_produced(&unary.expr, e);
                        match &self.types {
                            Some(tc) => {
                                let accessor = tc.borrow().deref_accessor_of(&unary.expr);
                                match accessor {
                                    Ok(accessor) => format!("{}.{}", written, accessor),
                                    // Not every `*` reaches through a wrapper:
                                    // `*x` on a `&T` is the `T`, and emission
                                    // erases the reference.
                                    Err(_) => written,
                                }
                            }
                            None => written,
                        }
                    }
                    _ => format!("/* unknown unary op */ {}", e),
                }
            }

            syn::Expr::If(if_expr) => control_flow::translate_if(if_expr, self),

            syn::Expr::Block(block) => {
                if block.block.stmts.len() == 1 {
                    if let syn::Stmt::Expr(expr, None) = &block.block.stmts[0] {
                        return self.expr(expr);
                    }
                }
                // Multi-statement block as expression → IIFE
                // Detect shadowed variables: if a local in the block has the same name
                // as a variable used in its init, thread it as an IIFE parameter
                let mut shadow_params: Vec<(String, String, Option<crate::ty::Ty>)> = Vec::new();
                for stmt in &block.block.stmts {
                    if let syn::Stmt::Local(local) = stmt {
                        let pat_name = Self::pat_static(&local.pat);
                        if let Some(init) = &local.init {
                            // Check if the init expression references pat_name as a
                            // standalone variable (not as a field name in a.b.c)
                            if references_var(&init.expr, &pat_name) {
                                // This is a shadow pattern — pass as IIFE param.
                                // The parameter holds the value of the initialiser,
                                // resolved out here, before the block's scope exists.
                                let resolved = self.resolve_expr_type(&init.expr);
                                let instead =
                                    format!("IIFE parameter `{}` is left untyped", pat_name);
                                let ty = self.or_fallback(resolved, &instead);
                                let init_ts = self.expr(&init.expr);
                                shadow_params.push((pat_name, init_ts, ty));
                            }
                        }
                    }
                }
                if !shadow_params.is_empty() {
                    // Thread shadowed variables as IIFE parameters.
                    // Push shadow names into scope so local() skips their declarations
                    // (they're already bound as IIFE params).
                    self.push_block();
                    for (name, _, ty) in &shadow_params {
                        match ty {
                            Some(ty) => self.bind_var(name, ty.clone()),
                            None => self.bind_untyped(name),
                        }
                        self.threaded.borrow_mut().push(name.clone());
                    }
                    let body = self.translate_block_stmts(&block.block);
                    for _ in &shadow_params {
                        self.threaded.borrow_mut().pop();
                    }
                    self.pop_scope();
                    let params: Vec<&str> =
                        shadow_params.iter().map(|(n, _, _)| n.as_str()).collect();
                    let args: Vec<&str> = shadow_params.iter().map(|(_, v, _)| v.as_str()).collect();
                    format!("(({}) => {{\n{}}})({})", params.join(", "), indent(&body), args.join(", "))
                } else {
                    let body = self.translate_block(&block.block);
                    format!("(() => {{\n{}}})()", indent(&body))
                }
            }

            syn::Expr::Return(ret) => {
                if let Some(expr) = &ret.expr {
                    // What is returned leaves through the function's return
                    // type, which is what says what it has to be.
                    let want = self.fn_return.clone();
                    let value =
                        self.expecting(expr, want.as_ref(), || self.moved_value(expr));
                    format!("return {}", value)
                } else {
                    "return".to_string()
                }
            }

            syn::Expr::Match(me) => match_expr::translate_match(me, self),

            syn::Expr::Closure(closure) => self.closure(
                closure,
                ownership::closures::Placement::Loose,
                expected.as_ref(),
            ),

            syn::Expr::ForLoop(for_loop) => self.for_loop(for_loop),

            syn::Expr::While(while_loop) => {
                // `while let PAT = e` is a loop that re-evaluates `e` each turn,
                // tests it against the pattern and stops when it does not match.
                // Emitting the condition as an expression produced a comment
                // where the test should be and left the binding undeclared.
                let label = ownership::iteration::label_of(&while_loop.label);
                if let syn::Expr::Let(let_expr) = &*while_loop.cond {
                    return self.while_let(let_expr, &while_loop.body, &label);
                }
                // Rust evaluates the condition afresh each turn and drops
                // what it produced before the body runs. Where the condition
                // lifted anything, the test moves inside the loop: leaving it
                // in the `while` header evaluated it once and held whatever it
                // took for the life of the loop.
                let (cond, lifted) = self.with_own_hoists(|| self.expr(&while_loop.cond));
                let body = self.translate_block(&while_loop.body);
                if lifted.is_empty() {
                    return format!("{}while ({}) {{\n{}}}", label, cond, indent(&body));
                }
                let held = self.fresh_hoist("_c");
                let test = ownership::hoisted(&format!("{} = {};\n", held, cond), &lifted);
                format!(
                    "{}for (;;) {{\n  let {};\n{}  if (!{}) break;\n{}}}",
                    label,
                    held,
                    indent(&test),
                    held,
                    indent(&body)
                )
            }

            syn::Expr::Loop(loop_expr) => {
                let body = self.translate_block(&loop_expr.body);
                format!(
                    "{}while (true) {{\n{}}}",
                    ownership::iteration::label_of(&loop_expr.label),
                    indent(&body)
                )
            }

            // `break 'outer` names the loop it leaves, and so does
            // `continue 'outer`. A bare `break` leaves the innermost loop, which
            // is a different program wherever the source named an outer one.
            syn::Expr::Break(brk) => {
                let target = ownership::iteration::target_of(&brk.label);
                if let Some(expr) = &brk.expr {
                    format!("break{} /* {} */", target, self.expr(expr))
                } else {
                    format!("break{}", target)
                }
            }

            syn::Expr::Continue(cont) => {
                format!("continue{}", ownership::iteration::target_of(&cont.label))
            }

            // A `*place = value` goes through the same door as every other
            // assignment. Writing it here instead named no temporary and
            // released nothing: `*cell.lock().unwrap() = v` left the mutex
            // locked for the life of the program, and the value the place
            // already held was abandoned where Rust drops it.
            syn::Expr::Assign(assign) => self.assign(assign),

            syn::Expr::Index(idx) => {
                // `v[a..b]` is a slice, not an index. Emitting the range as an
                // index expression produced `v[/* range a..b */]`, which does
                // not parse.
                if let syn::Expr::Range(range) = &*idx.index {
                    let from = range
                        .start
                        .as_ref()
                        .map(|e| self.expr(e))
                        .unwrap_or_else(|| "0".to_string());
                    let to = range.end.as_ref().map(|e| self.expr(e));
                    let end = match (&range.limits, to) {
                        // `..=b` includes the last element.
                        (syn::RangeLimits::Closed(_), Some(to)) => format!(", {} + 1", to),
                        (_, Some(to)) => format!(", {}", to),
                        (_, None) => String::new(),
                    };
                    return format!("{}.slice({}{})", self.expr(&idx.expr), from, end);
                }
                // `[Owned::new()][0]` reads out of a sequence the expression
                // itself built, and that sequence is a temporary Rust drops at
                // the end of the statement.
                let base = self.expr(&idx.expr);
                let base = self.hoist_produced(&idx.expr, base);
                format!("{}[{}]", base, self.expr(&idx.index))
            }

            // `look(&Owned { .. })` builds a value the callee only reads, and
            // Rust drops it at the end of the statement. Emitting the
            // expression in place left nothing holding it and nothing releasing
            // it.
            syn::Expr::Reference(reference) => {
                let written = self.expr(&reference.expr);
                self.hoist_produced(&reference.expr, written)
            }

            syn::Expr::Paren(paren) => format!("({})", self.expr(&paren.expr)),

            syn::Expr::Tuple(tuple) => {
                let parts: Vec<String> = tuple.elems.iter().map(|e| self.moved_value(e)).collect();
                format!("[{}]", parts.join(", "))
            }

            syn::Expr::Array(arr) => {
                let items: Vec<String> = arr.elems.iter().map(|e| self.moved_value(e)).collect();
                self.sequence_literal(items, expected.as_ref())
            }

            syn::Expr::Struct(s) => {
                let mut name = Self::path_static(&s.path);
                if name == "Self" { name = self.self_type.to_string(); }
                // A field's declared type is what its initialiser has to be, so
                // `Header { len: 1 }` writes the width the field declares.
                let want = self.struct_field_types(s);
                let values: Vec<String> = s
                    .fields
                    .iter()
                    .map(|f| {
                        let member = crate::infer::member_name(&f.member);
                        let field = want.iter().find(|(name, _)| *name == member).map(|(_, t)| t);
                        self.expecting(&f.expr, field, || self.moved_value(&f.expr))
                    })
                    .collect();
                format!("new {}({})", name, values.join(", "))
            }

            syn::Expr::Try(try_expr) => {
                // Special case: write!(f, ...)? in expression position — just the format string
                if is_write_macro(&try_expr.expr) {
                    let fmt_str = macros::translate_macro(extract_macro(&try_expr.expr).unwrap(), self);
                    return fmt_str;
                }
                let lowered = self.lower_try(try_expr);
                self.own.prelude.borrow_mut().push(ownership::Hoist {
                    declaration: lowered.declaration,
                    owned: None,
                });
                lowered.value
            }
            syn::Expr::Await(await_expr) => format!("await {}", self.expr(&await_expr.base)),

            syn::Expr::Range(range) => {
                let from = range.start.as_ref().map(|e| self.expr(e)).unwrap_or_default();
                let to = range.end.as_ref().map(|e| self.expr(e)).unwrap_or_default();
                format!("/* range {}..{} */", from, to)
            }

            syn::Expr::Cast(cast) => {
                format!("{} as {}", self.expr(&cast.expr), name_map::map_type(&cast.ty))
            }

            // A macro's own span is what its translation asks the expectation
            // by, so the type the position wants is re-keyed onto it: `vec![1,
            // 2]` behind a `Vec<u8>` writes bytes, not a JavaScript array.
            syn::Expr::Macro(mac) => {
                let held = self.expecting.replace(
                    expected
                        .as_ref()
                        .map(|ty| (span_position(syn::spanned::Spanned::span(&mac.mac)), ty.clone())),
                );
                let written = macros::translate_macro(&mac.mac, self);
                *self.expecting.borrow_mut() = held;
                written
            }

            syn::Expr::Unsafe(unsafe_block) => {
                let body = self.translate_block(&unsafe_block.block).trim().to_string();
                format!("/* unsafe — consider provided impl */ {}", body)
            }

            syn::Expr::Async(async_block) => {
                let body = self.translate_block(&async_block.block);
                format!("(async () => {{\n{}}})()", indent(&body))
            }

            syn::Expr::Let(let_expr) => {
                let pat = Self::pat_static(&let_expr.pat);
                let expr = self.expr(&let_expr.expr);
                format!("/* let {} = {} */", pat, expr)
            }

            syn::Expr::Repeat(repeat) => {
                format!("Array({}).fill({})", self.expr(&repeat.len), self.expr(&repeat.expr))
            }

            _ => "/* TODO: unhandled expr */".to_string(),
        }
    }

    /// An expression whose *value* the surrounding code needs.
    ///
    /// An `if` is a value in Rust and a statement in TypeScript, and the two
    /// need different code: as a statement it is an `if`, as a value it is a
    /// ternary where both branches are expressions and an immediately-called
    /// arrow function where they are not. Emitting the statement form in value
    /// position wrote a block where an expression had to stand, which does not
    /// parse.
    pub fn expr_value(&self, expr: &syn::Expr) -> String {
        let syn::Expr::If(if_expr) = expr else {
            return self.expr(expr);
        };
        if let Some(ternary) = self.try_ternary(if_expr) {
            return ternary;
        }
        let body = control_flow::translate_expr_in_return_position(expr, self);
        format!("(() => {{\n{}}})()", indent(&format!("{}\n", body)))
    }

    /// `e?` — take the value out, or leave with the error.
    ///
    /// The test and the early exit are statements, so they are lifted into the
    /// prelude and the expression becomes the name they left behind. That is
    /// what makes `f(g()?)` work: `g()` is asked once, its error leaves the
    /// function, and `f` is called with the value.
    ///
    /// The `Ok` wrapper is consumed by the `unwrap` that follows, and the `Err`
    /// wrapper by the `unwrapErr` that rebuilds it, so neither is left for the
    /// leak registry to find.
    pub(crate) fn lower_try(&self, try_expr: &syn::ExprTry) -> Lowered {
        let inner = self.expr(&try_expr.expr);
        let span = syn::spanned::Spanned::span(try_expr);
        let ty = self.resolve_expr_type(&try_expr.expr).ok();
        let temp = self.fresh_hoist("_r");

        // `?` on an `Option<T>` leaves with `None`, which this port writes as
        // null. The engine names the type; a receiver it could not name is a
        // `Result`, which is what all but a handful of `?` in the corpus are.
        let is_option = ty.as_ref().is_some_and(|ty| self.is_nullable(ty));
        if is_option {
            return Lowered {
                declaration: format!(
                    "const {} = {};\nif ({} == null) return null;\n",
                    temp, inner, temp
                ),
                value: temp,
                wrapper: None,
            };
        }

        if ty.is_none() {
            self.fallback(
                span,
                "`?` is lowered as a `Result` without the engine having named what it tests",
            );
        } else {
            self.report_try_conversion(ty.as_ref(), span);
        }
        Lowered {
            declaration: format!(
                "const {} = {};\nif ({}.isErr()) return Result.Err({}.unwrapErr());\n",
                temp, inner, temp, temp
            ),
            value: format!("{}.unwrap()", temp),
            wrapper: Some(temp),
        }
    }

    /// Is this the `Option<T>` the port writes as `T | null`?
    pub(crate) fn is_nullable(&self, ty: &crate::ty::Ty) -> bool {
        let Some(tc) = &self.types else { return false };
        let Some(id) = ty.peel_refs().id() else {
            return false;
        };
        matches!(
            tc.borrow().registry.shapes().form(id),
            Some(crate::name_map::system_shapes::Form::Nullable)
        )
    }

    /// Say so when `?` crosses two different error types.
    ///
    /// Rust calls `From` there, and the engine does not yet say which `From`.
    /// The emitted code hands the error on unchanged, which is right wherever
    /// the two types agree and wrong wherever they do not — so every site where
    /// they differ is reported rather than silently mistranslated.
    pub(crate) fn report_try_conversion(&self, ty: Option<&crate::ty::Ty>, span: proc_macro2::Span) {
        let (Some(ty), Some(returns)) = (ty, self.fn_return.as_ref()) else {
            return;
        };
        let error_of = |ty: &crate::ty::Ty| match ty.peel_refs() {
            crate::ty::Ty::Named { args, .. } if args.len() == 2 => Some(args[1].clone()),
            _ => None,
        };
        let (Some(from), Some(to)) = (error_of(ty), error_of(returns)) else {
            return;
        };
        if from == to {
            return;
        }
        let Some(tc) = &self.types else { return };
        let tc = tc.borrow();
        self.fallback(
            span,
            format!(
                "`?` converts {} to {} through `From`, which the engine has not resolved; \
                 the error is handed on unconverted",
                name_map::map_ty(tc.registry, &from),
                name_map::map_ty(tc.registry, &to),
            ),
        );
    }







    /// A name for a value the emitted code has to hold on to.
    ///
    /// A pattern match tests its subject and then takes it apart, and the
    /// subject has to be the *same* value both times: `if let Some(x) =
    /// c.step().await?` that writes the call twice calls it twice.
    pub fn fresh_temp(&self) -> String {
        let n = self.temporaries.get();
        self.temporaries.set(n + 1);
        if n == 0 {
            "_v".to_string()
        } else {
            format!("_v{}", n)
        }
    }


    /// An expression in a position that runs it rather than reading its value.
    ///
    /// A block runs as statements here rather than as an immediately-called
    /// arrow function, so a `break` written inside it still leaves the loop it
    /// was written in.
    pub fn statements(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Block(block) => self.translate_block(&block.block),
            _ => self.expr(expr),
        }
    }













    /// `while let PAT = e { body }` as a loop that tests each turn.
    ///
    /// The scrutinee is read once per turn into a temporary, tested against the
    /// pattern, and its payload bound inside the body — which is what Rust does
    /// and what the previous emission, a comment in the condition, did not.
    pub(crate) fn while_let(
        &self,
        let_expr: &syn::ExprLet,
        body: &syn::Block,
        label: &str,
    ) -> String {
        // The scrutinee is read afresh every turn, in value position: it is the
        // turn's own value, and an `if` written there is a run of statements
        // that `const _v = …` cannot hold. Whatever it lifted belongs inside
        // the loop with it, because it is taken again on the next turn.
        let (scrutinee, lifted) = self.with_own_hoists(|| self.expr_value(&let_expr.expr));
        let ty = self.scrutinee_type(&let_expr.expr);
        let _bindings = self.enter_pattern(&let_expr.pat, ty.as_ref());
        // The pattern binds afresh each turn, and Rust drops what it bound at
        // the end of that turn — so the release goes inside the loop, not after
        // it.
        let owned = self.claim_bindings(&bound_names(&let_expr.pat), &body.stmts);
        let translated = self.translate_block(body);
        drop(_bindings);

        let subject = self.fresh_temp();
        let (test, bind) = self.pattern_test(&subject, &let_expr.pat);
        let turn = self.wrap_bindings(&owned, translated);
        let leaving = self.abandoned_scrutinee(&let_expr.expr, &let_expr.pat, &subject);
        let read = ownership::hoisted(&format!("const {} = {};\n", subject, scrutinee), &lifted);
        format!(
            "{}for (;;) {{\n{}  if (!({})) {{\n{}    break;\n  }}\n{}{}}}",
            label,
            indent(&read),
            test,
            indent(&indent(&leaving)),
            indent(&bind),
            indent(&turn)
        )
    }

    /// What the turn owes for a scrutinee whose pattern did not match.
    ///
    /// Rust drops the value the turn read when no pattern took it apart, so the
    /// path that leaves the loop releases it. The path that *did* match is a
    /// different question: where the pattern took an owned payload out, that
    /// payload belongs to the binding from there and the enum it came out of
    /// has to be marked moved — which only `intoMatch` does, and an arrow
    /// function is not something a `break` can leave. That one is reported.
    fn abandoned_scrutinee(&self, expr: &syn::Expr, pat: &syn::Pat, subject: &str) -> String {
        // A nullable scrutinee is its own payload: `Option<T>` is `T | null`
        // here, so `Some(v)` binds the very value the turn read and the turn
        // that did not match read a `null`, which owns nothing. There is no
        // wrapper left over on either path.
        if self
            .quietly(|| self.resolve_expr_type(expr))
            .is_ok_and(|ty| self.is_nullable(&ty))
        {
            return String::new();
        }
        let Some(release) = self.release_of(expr, subject) else {
            return String::new();
        };
        if self.pattern_takes_a_payload(expr, pat) {
            self.fallback(
                syn::spanned::Spanned::span(expr),
                "this `while let` takes an owned payload out of the value it read, and the \
                 value it came out of is not marked moved, so nothing releases the rest of it",
            );
        }
        format!("{}\n", release)
    }

    /// Does this pattern take a payload out of the scrutinee that the arm then
    /// owns, rather than binding the whole value?
    fn pattern_takes_a_payload(&self, expr: &syn::Expr, pat: &syn::Pat) -> bool {
        let Some(tc) = &self.types else { return false };
        let Ok(subject) = self.quietly(|| self.resolve_expr_type(expr)) else {
            return false;
        };
        let tc = tc.borrow();
        let arm = syn::Arm {
            attrs: Vec::new(),
            pat: pat.clone(),
            guard: None,
            fat_arrow_token: syn::token::FatArrow::default(),
            body: Box::new(syn::Expr::Verbatim(proc_macro2::TokenStream::new())),
            comma: None,
        };
        let takes = ownership::scrutinee::takes(&tc.probe(), &subject, &[arm], |path| {
            let mark = tc.sink.mark();
            let payload = tc.payload_of(path, Some(&subject));
            tc.sink.rewind(mark);
            payload.unwrap_or_default().into_iter().map(|(_, ty)| ty).collect()
        });
        takes == ownership::scrutinee::Takes::Payload
    }

    /// `*place`, written as the place an assignment stores into.
    ///
    /// A `*` in a value position may reach through nothing at all — `*x` on a
    /// `&T` is the `T`, and emission erases the reference — so a deref the
    /// engine could not resolve is written as the value itself. An assignment
    /// target cannot be: `*guard = v` and `*guard += 1` store *through* the
    /// wrapper whatever the engine could say about it, and dropping the
    /// accessor there emitted `counter.lock() += 1`, which names no place at
    /// all. So the target keeps `.value` as its default, and says that it
    /// assumed it.
    pub(crate) fn deref_place(&self, unary: &syn::ExprUnary) -> String {
        let inner = self.expr(&unary.expr);
        let inner = self.hoist_produced(&unary.expr, inner);
        let Some(tc) = &self.types else {
            self.fallback(syn::spanned::Spanned::span(&*unary.expr), ASSUMED_ACCESSOR);
            return format!("{}.value", inner);
        };
        let accessor = tc.borrow().deref_accessor_of(&unary.expr);
        match self.or_fallback(accessor, ASSUMED_ACCESSOR) {
            Some(accessor) => format!("{}.{}", inner, accessor),
            None => format!("{}.value", inner),
        }
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
                let var = ts.elems.first().map(Self::pat_static).unwrap_or_else(|| "v".to_string());
                match name.as_str() {
                    "Some" => (format!("{} != null", subject), format!("const {} = {};\n", var, subject)),
                    "Ok" => (format!("{}.isOk()", subject), format!("const {} = {}.unwrap();\n", var, subject)),
                    "Err" => (format!("{}.isErr()", subject), format!("const {} = {}.unwrapErr();\n", var, subject)),
                    _ => {
                        let names: Vec<String> = ts.elems.iter().enumerate()
                            .map(|(i, p)| {
                                let local = Self::pat_static(p);
                                if local == format!("_{}", i) { local } else { format!("_{}: {}", i, local) }
                            })
                            .collect();
                        (
                            format!("{}.is('{}')", subject, name),
                            format!("const {{ {} }} = {}.value;\n", names.join(", "), subject),
                        )
                    }
                }
            }
            syn::Pat::Path(p) => {
                let name = p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                match name.as_str() {
                    "None" => (format!("{} == null", subject), String::new()),
                    _ => (format!("{}.is('{}')", subject, name), String::new()),
                }
            }
            syn::Pat::Struct(st) => {
                let name = st.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                let fields: Vec<String> = st.fields.iter().map(|f| {
                    let member = match &f.member {
                        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    let local = Self::pat_static(&f.pat);
                    if member == local { member } else { format!("{}: {}", member, local) }
                }).collect();
                (
                    format!("{}.is('{}')", subject, name),
                    format!("const {{ {} }} = {}.value;\n", fields.join(", "), subject),
                )
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
                             different places, which the translator cannot write as one test",
                        );
                        ("true".to_string(), String::new())
                    }
                }
            }
            // A plain name binds whatever it was given, and always matches.
            syn::Pat::Ident(_) => {
                let var = Self::pat_static(pat);
                ("true".to_string(), format!("const {} = {};\n", var, subject))
            }
            // `0 => ..` compares. Writing the pattern where the test belongs
            // emitted `if (/* pat literal */)`, which does not parse.
            syn::Pat::Lit(lit) => (
                format!("{} === {}", subject, translate_lit(&lit.lit)),
                String::new(),
            ),
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

    /// Try to translate an if/else as a ternary expression.
    /// Returns Some(ternary) if both branches are single expressions.
    pub(crate) fn try_ternary(&self, if_expr: &syn::ExprIf) -> Option<String> {
        // Must not be if-let
        if matches!(&*if_expr.cond, syn::Expr::Let(_)) { return None; }
        // Must have an else branch
        let (_, else_expr) = if_expr.else_branch.as_ref()?;
        // Then branch must be a single expression
        let then_val = single_block_expr(&if_expr.then_branch)?;
        // Else branch must be a single expression (not another if)
        let else_val = match else_expr.as_ref() {
            syn::Expr::Block(block) => single_block_expr(&block.block)?,
            _ => return None,
        };
        // A branch that hands a flagged local away needs a statement to set the
        // flag in, and a ternary branch is not one. The `if` form is.
        if !self.flag_sets_for(then_val).is_empty() || !self.flag_sets_for(else_val).is_empty() {
            return None;
        }
        // The branches are written out to find out whether they fit; a branch
        // that lifts a declaration out of itself does not, because the
        // declaration would run whichever branch was taken. The attempt takes
        // its diagnostics back with it, so the abandoned form is not counted.
        let mark = self.mark();
        let cond = self.expr(&if_expr.cond);
        let (then_ts, then_lifted) = self.with_own_hoists(|| self.expr_value(then_val));
        let (else_ts, else_lifted) = self.with_own_hoists(|| self.expr_value(else_val));
        if !then_lifted.is_empty() || !else_lifted.is_empty() {
            self.rewind(mark);
            return None;
        }
        Some(format!("{} ? {} : {}", cond, then_ts, else_ts))
    }

    /// Where the diagnostics record stands, for a form the translator may
    /// abandon.
    pub(crate) fn mark(&self) -> usize {
        self.types.as_ref().map(|tc| tc.borrow().sink.mark()).unwrap_or(0)
    }

    pub(crate) fn rewind(&self, mark: usize) {
        if let Some(tc) = &self.types {
            tc.borrow().sink.rewind(mark);
        }
    }

    // ── Method call translation ─────────────────────────────────────
    //
    // Dispatches to native_types modules based on resolved receiver type.
    // System types (Arc, RwLock, Result, etc.) pass through — their TS
    // implementations handle the method names directly.

    /// A call the engine could not resolve to a function.
    ///
    /// This is the transitional path spec section 4.11 keeps: the diagnostic has
    /// already been filed, and the translator does what it did before the impl
    /// table existed — reach through one wrapper if the receiver has one and the
    /// name is not its own, dispatch on whatever type it does know, and fall
    /// back to the name when it knows nothing. The std-surface step is what
    /// empties this path out; the fail-loud step deletes it.
    pub(crate) fn translate_unresolved_call(&self, receiver: &str, rust_method: &str, ts_method: &str, args: &[String], receiver_expr: Option<&syn::Expr>) -> String {
        if let (Some(receiver_expr), Some(tc)) = (receiver_expr, &self.types) {
            let tc_ref = tc.borrow();
            if let Ok(receiver_ty) = tc_ref.resolve_expr(receiver_expr) {
                let probe = tc_ref.probe();
                let step = probe.deref_once(&receiver_ty);
                let reach_through = !probe.declares_method(&receiver_ty, rust_method);
                let (target, receiver) = match (&step, reach_through) {
                    (Some(step), true) => {
                        let written = match &step.accessor {
                            Some(accessor) => format!("{}.{}", receiver, accessor.written()),
                            None => receiver.to_string(),
                        };
                        (step.to.clone(), written)
                    }
                    _ => (receiver_ty.clone(), receiver.to_string()),
                };
                let translated =
                    native_types::translate_method(tc_ref.registry, &target, &receiver, rust_method, args);
                drop(tc_ref);
                return self.render_translation(
                    translated,
                    &receiver,
                    ts_method,
                    args,
                    syn::spanned::Spanned::span(receiver_expr),
                );
            }
        }

        // No type at all — the methods that translate the same way whatever the
        // receiver is.
        self.render_translation(
            native_types::translate_untyped(receiver, rust_method, args),
            receiver,
            ts_method,
            args,
            proc_macro2::Span::call_site(),
        )
    }

    // ── Function call translation ───────────────────────────────────
    //
    // Language-level constructs (Self, Ok/Err/Some/None, enum variants,
    // constructor heuristic) stay here. Type-specific translations
    // (Vec::new, HashMap::new, etc.) are in native_types/ modules.

    pub(crate) fn translate_call(
        &self,
        func: &str,
        args: &[String],
        span: proc_macro2::Span,
        callee: Option<&syn::Path>,
    ) -> String {
        // 0. Resolve inline module qualifiers (e.g., stack.track → track)
        // Resolve inline module qualifiers (e.g., stack.track → track).
        // Import generation is handled by codegen.rs scanning the translated bodies.
        for mod_name in &self.inline_module_names {
            let prefix = format!("{}.", mod_name);
            if let Some(stripped) = func.strip_prefix(&prefix) {
                return self.translate_call(stripped, args, span, callee);
            }
        }

        // 1. Language-level constructs
        match func {
            "Self" => return format!("new {}({})", self.self_type, args.join(", ")),
            "Ok" => return format!("Result.Ok({})", args.join(", ")),
            "Err" => return format!("Result.Err({})", args.join(", ")),
            "Some" if args.len() == 1 => return args[0].clone(),
            "Some" => return args.join(", "),
            "None" => return "null".to_string(),
            // `drop(x)` takes x by value and runs its glue there and then. The
            // move analysis has already taken x off the block's list — it is an
            // argument passed by value like any other — so this releases it
            // once, where the source says.
            "drop" | "mem.drop" | "mem::drop" if args.len() == 1 => {
                return format!("{}.drop()", args[0]);
            }
            // `forget` is the one thing this model cannot express: it hands a
            // value to nobody and cancels its drop. Emitting the release would
            // run glue Rust suppressed, and emitting nothing leaks.
            "mem.forget" | "mem::forget" | "forget" if args.len() == 1 => {
                self.fallback(
                    span,
                    "`mem::forget` suppresses drop glue, which the emitted ownership model \
                     has no way to say; the value is left to the leak registry",
                );
                return format!("/* mem::forget */ void {}", args[0]);
            }
            _ => {}
        }

        // 2. Native type static calls (Vec::new, HashMap::new, Arc::clone, etc.)
        //
        // The table is keyed by the written name, so it is only consulted where
        // that name does not belong to a type this crate declared. A crate's own
        // `Vec` is its own class, and `Vec::new()` on it is `Vec.new()`, not a
        // JavaScript array literal.
        if !self.names_crate_type(callee) {
            if let Some(result) = native_types::translate_static_call(func, args) {
                return result;
            }
        }

        // 3. Serde/bincode crate calls
        match func {
            "serde_json.to_string" | "serde_json::to_string" | "serdeJson.toString"
                if args.len() == 1 => return format!("JSON.stringify({})", args[0]),
            // `from_str::<T>(s)` parses the text and then asks `T` to read
            // itself out of it, which is what `Deserialize` does. Parsing alone
            // hands back a plain object where a `T` was wanted.
            "serde_json.from_str" | "serde_json::from_str" | "serdeJson.fromStr"
                if args.len() == 1 =>
            {
                return match self.read_into_type(callee, span) {
                    Some(ty) => format!("{}.fromJson(JSON.parse({}))", ty, args[0]),
                    None => {
                        self.fallback(
                            span,
                            "`serde_json::from_str` is written without saying which type reads \
                             itself out of the parsed value, so the parse stands alone",
                        );
                        format!("JSON.parse({})", args[0])
                    }
                };
            }
            "bincode.serialize" | "bincode::serialize" if args.len() == 1 =>
                return format!("(() => {{ const _w = new BincodeWriter(); {}.encode(_w); return _w.finish(); }})()", args[0]),
            "bincode.deserialize" | "bincode::deserialize" if args.len() == 1 => {
                return match self.read_into_type(callee, span) {
                    Some(ty) => format!(
                        "(() => {{ const _r = new BincodeReader({}); return {}.decode(_r); }})()",
                        args[0], ty
                    ),
                    None => {
                        self.fallback(
                            span,
                            "`bincode::deserialize` is written without saying which type reads \
                             itself out of the bytes, so the reader stands alone",
                        );
                        format!(
                            "(() => {{ const _r = new BincodeReader({}); return _r; }})()",
                            args[0]
                        )
                    }
                };
            }
            _ => {}
        }

        // 4. Box::new is transparent
        if matches!(func, "Box.new" | "Box::new") && args.len() == 1 {
            return args[0].clone();
        }

        // 5. Arc static methods → instance methods
        match func {
            "Arc.asPtr" | "Arc::asPtr" | "Arc.as_ptr" | "Arc::as_ptr"
                if args.len() == 1 => return format!("{}.asPtr()", args[0]),
            "Arc.downgrade" | "Arc::downgrade"
                if args.len() == 1 => return format!("{}.downgrade()", args[0]),
            _ => {}
        }

        // 6. Type::new() constructor pattern
        // System/base types (Arc, Mutex, RwLock, RefCell, etc.) use `new Type(args)` because
        // their TS constructors match the Rust ::new() signature directly.
        // Crate-defined types use `Type.new(args)` because the transpiler emits a
        // `static new()` method with custom initialization logic.
        if func.ends_with(".new") || func.ends_with("::new") {
            let type_name = func.trim_end_matches(".new").trim_end_matches("::new");
            let type_name = if type_name == "Self" { self.self_type } else { type_name };
            // System types with public constructors matching ::new() signature
            let use_constructor = matches!(type_name,
                "Mutex" | "RwLock" | "RefCell" | "HashMap" | "BTreeMap"
                | "HashSet" | "BTreeSet" | "Vec" | "RwLockReadGuard" | "RwLockWriteGuard"
                | "MutexGuard" | "Ref" | "RefMut" | "Box" | "ThreadLocal"
            );
            if use_constructor {
                return format!("new {}({})", type_name, args.join(", "));
            }
            // Everything else (crate-defined types + Arc/Weak): use static new()
            return format!("{}.new({})", type_name, args.join(", "));
        }

        // 7. Self::method() → TypeName.method()
        if func.starts_with("Self.") || func.starts_with("Self::") {
            let method = func.split("::").last()
                .or_else(|| func.split('.').last())
                .unwrap_or(func);
            return format!("{}.{}({})", self.self_type, method, args.join(", "));
        }

        // 8. Enum variant constructor: Type.Variant(args) → new Type('Variant', {...})
        if let Some(dot) = func.rfind('.') {
            let type_name = &func[..dot];
            let variant = &func[dot+1..];

            // The registry answers wherever it has a declaration, which is now
            // everywhere a body is translated — match arms included. A type from
            // another crate has no declaration here, because each crate is
            // transpiled on its own; the engine says "not a variant" and the
            // call is written as the associated function the other crate's
            // TypeScript exposes. Only a translation path with no type context
            // at all is left to guess from the shape of the name.
            let is_enum_variant = match &self.types {
                Some(tc) => tc.borrow().is_variant(type_name, variant),
                None => {
                    let guess = type_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                        && variant.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                        && !matches!(type_name, "Math" | "JSON" | "Object" | "Array" | "console" | "Promise");
                    if guess {
                        self.fallback(
                            span,
                            format!("`{}` is guessed to be an enum variant from its capitalisation", func),
                        );
                    }
                    guess
                }
            };

            if is_enum_variant {
                if args.is_empty() {
                    return format!("new {}('{}', {{}})", type_name, variant);
                } else if args.len() == 1 {
                    return format!("new {}('{}', {{ _0: {} }})", type_name, variant, args[0]);
                } else {
                    let fields: Vec<String> = args.iter().enumerate()
                        .map(|(i, a)| format!("_{}: {}", i, a))
                        .collect();
                    return format!("new {}('{}', {{ {} }})", type_name, variant, fields.join(", "));
                }
            }
        }

        // 9. PascalCase function → constructor heuristic
        if func.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && !func.contains('.')
            && !matches!(func, "Ok" | "Some" | "Err" | "None" | "Self")
        {
            self.fallback(
                span,
                format!("`{}` is guessed to be a constructor from its capitalisation", func),
            );
            return format!("new {}({})", func, args.join(", "));
        }

        // 10. Default: plain function call
        format!("{}({})", func, args.join(", "))
    }

    /// Does the type part of this callee path name a type this crate declared?
    ///
    /// `Vec::new` in ankurah is std's `Vec` unless ankurah declares one of its
    /// own; the name tables downstream cannot tell the difference, so the
    /// registry is asked before they are consulted.
    pub(crate) fn names_crate_type(&self, callee: Option<&syn::Path>) -> bool {
        let (Some(path), Some(tc)) = (callee, &self.types) else {
            return false;
        };
        if path.segments.len() < 2 {
            return false;
        }
        let owner: Vec<String> = path
            .segments
            .iter()
            .take(path.segments.len() - 1)
            .map(|s| s.ident.to_string())
            .collect();
        let tc = tc.borrow();
        matches!(
            tc.registry.lookup_type(tc.module, &owner),
            Ok(Some(crate::registry::Def::Type(id)))
                if !id.is_foreign() && !tc.registry.is_system(id)
        )
    }

    // ── Path translation ────────────────────────────────────────────

    /// A path in expression position. The standard-library qualifiers are
    /// dropped so that `std::sync::Arc::new` becomes `Arc.new`, which is a
    /// guess about what the remaining segments mean; it is recorded as one.
    pub(crate) fn path_expr(&self, path: &syn::Path) -> String {
        // A path of one segment is a name — a local, a parameter, a free
        // function — and never a module qualifier. Filtering it as one deleted
        // every local called `ops`, `iter` or `fmt`: `ops.iter()` came out as
        // `[...]`, a spread of nothing.
        let dropped: Vec<String> = if path.segments.len() == 1 {
            Vec::new()
        } else {
            path.segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .filter(|name| STD_QUALIFIERS.contains(&name.as_str()))
                .collect()
        };
        // A `tokio` path keeps its segments, so nothing was given up.
        let dropped = if path.segments.first().is_some_and(|s| s.ident == "tokio") {
            Vec::new()
        } else {
            dropped
        };
        if !dropped.is_empty() {
            self.fallback(
                syn::spanned::Spanned::span(path),
                format!("path qualifiers {} are dropped by name", dropped.join(", ")),
            );
        }
        // A single name may be a local the translator had to emit under a
        // different identifier, because a Rust shadow cannot be declared twice
        // in one JavaScript scope.
        if path.segments.len() == 1 {
            // A body emitted as a module-level function has no `this`: its
            // receiver arrived as an ordinary first parameter, under the name
            // `self_name`.
            if path.segments[0].ident == "self" {
                return self.self_name.to_string();
            }
            let written = Self::path_static(path);
            // A path of one lowercase segment names a local, a parameter or a
            // free function — a binding, and JavaScript will not accept every
            // Rust name in that position. `Type::new()` is not one: `new` there
            // is a property, which may be a keyword, so the escape is confined
            // to the single-segment case and to the names this function merely
            // camel-cased. `self` becomes `this` and `None` becomes `null`,
            // which are the keywords themselves and not names.
            let ident = path.segments[0].ident.to_string();
            let written = if written == name_map::to_camel_case(&ident) {
                name_map::escape_reserved(&written)
            } else {
                written
            };
            if let Some(emitted) = self.emitted_name(&written) {
                return emitted;
            }
            return written;
        }
        Self::path_static(path)
    }

    pub(crate) fn path_static(path: &syn::Path) -> String {
        let single = path.segments.len() == 1;
        // A path through `tokio` keeps every segment and every name as written:
        // `@ankurah/base` mirrors the crate's module tree and spells the
        // functions the way tokio spells them, so `tokio::sync::mpsc::
        // unbounded_channel` is `tokio.sync.mpsc.unbounded_channel`.
        let through_tokio = path.segments.first().is_some_and(|s| s.ident == "tokio");
        if through_tokio {
            let names: Vec<String> =
                path.segments.iter().map(|seg| seg.ident.to_string()).collect();
            return names.join(".");
        }
        let segments: Vec<String> = path.segments.iter().map(|seg| {
            let name = seg.ident.to_string();
            match name.as_str() {
                "self" => "this".to_string(),
                "Self" => "Self".to_string(),
                "None" => "null".to_string(),
                "true" | "false" => name,
                "Ok" | "Some" | "Err" => name,
                "std" | "core" | "alloc" | "crate" | "super" | "marker" => name,
                "PhantomData" => return "undefined /* PhantomData */".to_string(),
                // Ordering::SeqCst etc. — no JS equivalent, stripped by method call handlers
                "Ordering" => return "undefined /* Ordering */".to_string(),
                _ => {
                    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        name
                    } else {
                        name_map::to_camel_case(&name)
                    }
                }
            }
        }).collect();

        // Strip std/core/alloc module prefixes, keep type+method. A lone
        // segment is a name, not a qualifier: a local called `ops` is `ops`.
        let segments: Vec<String> = segments.into_iter()
            .filter(|s| single || !STD_QUALIFIERS.contains(&s.as_str()))
            .collect();
        let joined = segments.join(".");
        match joined.as_str() {
            s if s.starts_with("crate.") => {
                segments.last().cloned().unwrap_or(joined)
            }
            _ => joined,
        }
    }
}

/// Does this method call take its receiver by value?
///
/// The move analysis asks; the impl table answers. `Result::unwrap`,
/// `Option::take` and the `into_*` family all take `self`, and a receiver they
/// took is not the block's to release any more.
impl ownership::moves::Consumes for BodyTranslator<'_> {
    fn consumes_receiver(&self, call: &syn::ExprMethodCall) -> bool {
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        // Asking is not translating. The resolution files the questions it
        // deferred, and this asks the same call several times — once per
        // statement scan, once per flag scan — so the record is wound back to
        // where it stood. The translation of the call reports them once.
        let mark = tc.sink.mark();
        let found = tc.resolve_method_call_with(
            &call.receiver,
            &call.method.to_string(),
            call.turbofish.as_ref(),
        );
        tc.sink.rewind(mark);
        let Ok(found) = found else {
            // `.await` on a named future and `Result::unwrap` on a receiver the
            // engine could not type are the two that matter, and both are worth
            // reading off the name rather than losing: taking a receiver that
            // was not taken leaks, and leaving one that was taken double-drops.
            return matches!(
                call.method.to_string().as_str(),
                "unwrap" | "expect" | "unwrap_err" | "expect_err" | "unwrap_or" | "unwrap_or_else"
                    | "unwrap_or_default" | "into" | "into_inner" | "into_iter" | "take"
                    | "ok" | "err" | "map_err" | "and_then" | "or_else"
            ) || call.method.to_string().starts_with("into_");
        };
        matches!(
            tc.registry.method_self_kind(&found),
            Some(crate::types::SelfKind::Value)
        )
    }

    fn consumes_scrutinee(&self, m: &syn::ExprMatch) -> bool {
        self.match_takes(m) == ownership::scrutinee::Takes::Payload
    }
}

/// The names a `let` pattern introduces, in the TypeScript spelling.
/// The closure a callee expression is, where it is one written in place.
fn as_closure(expr: &syn::Expr) -> Option<&syn::ExprClosure> {
    match expr {
        syn::Expr::Closure(closure) => Some(closure),
        syn::Expr::Paren(p) => as_closure(&p.expr),
        syn::Expr::Group(g) => as_closure(&g.expr),
        _ => None,
    }
}

/// The same, restricted to a `move` closure, which is the only kind that takes
/// its captures by value.
fn as_move_closure(expr: &syn::Expr) -> Option<&syn::ExprClosure> {
    as_closure(expr).filter(|closure| closure.capture.is_some())
}

/// The names a pattern binds, in the TypeScript spelling they are emitted
/// under.
pub fn pattern_names(pat: &syn::Pat) -> Vec<String> {
    bound_names(pat)
}

fn bound_names(pat: &syn::Pat) -> Vec<String> {
    let mut out = Vec::new();
    collect_bound(pat, &mut out);
    out
}

fn collect_bound(pat: &syn::Pat, out: &mut Vec<String>) {
    match pat {
        syn::Pat::Ident(ident) => {
            out.push(name_map::escape_reserved(&name_map::to_camel_case(
                &ident.ident.to_string(),
            )));
            if let Some((_, sub)) = &ident.subpat {
                collect_bound(sub, out);
            }
        }
        syn::Pat::Tuple(t) => t.elems.iter().for_each(|p| collect_bound(p, out)),
        syn::Pat::TupleStruct(t) => t.elems.iter().for_each(|p| collect_bound(p, out)),
        syn::Pat::Slice(s) => s.elems.iter().for_each(|p| collect_bound(p, out)),
        syn::Pat::Struct(s) => s.fields.iter().for_each(|f| collect_bound(&f.pat, out)),
        syn::Pat::Reference(r) => collect_bound(&r.pat, out),
        syn::Pat::Type(t) => collect_bound(&t.pat, out),
        syn::Pat::Paren(p) => collect_bound(&p.pat, out),
        syn::Pat::Or(or) => or.cases.iter().for_each(|p| collect_bound(p, out)),
        _ => {}
    }
}

/// What `e?` became: the statements that test it, the expression that stands in
/// its place, and the `Result` wrapper — which a statement-position `?` has to
/// release, because nothing downstream consumes it.
pub(crate) struct Lowered {
    pub(crate) declaration: String,
    pub(crate) value: String,
    pub(crate) wrapper: Option<String>,
}

/// The type a call's turbofish names: `from_str::<EntityId>(s)` says which
/// type is being read out of the parsed value.
fn turbofish_type(callee: Option<&syn::Path>) -> Option<String> {
    let segment = callee?.segments.last()?;
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        syn::GenericArgument::Type(ty) => Some(name_map::map_type(ty)),
        _ => None,
    }
}

/// Is this the `+=` family, which reads a place and writes back to it?
fn is_assign_op(op: &syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

/// Does this expression name storage that already exists, rather than produce
/// a value? Rust drops what a statement produced and nothing else, so only the
/// second kind needs a release written for it.
pub fn is_place(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Path(_) | syn::Expr::Field(_) | syn::Expr::Index(_) => true,
        syn::Expr::Unary(u) => matches!(u.op, syn::UnOp::Deref(_)) && is_place(&u.expr),
        syn::Expr::Reference(r) => is_place(&r.expr),
        syn::Expr::Paren(p) => is_place(&p.expr),
        syn::Expr::Group(g) => is_place(&g.expr),
        // `x?` hands back what was inside a Result the lowering already
        // consumed; `x.await` takes the future by value. Neither leaves a value
        // behind for the statement to release.
        syn::Expr::Try(t) => is_place(&t.expr),
        _ => false,
    }
}

/// Look through the wrappers a binding can be written behind — `let mut x`,
/// `let x: T`, `let (x)` — to whatever it really binds.
pub fn strip_binding(pat: &syn::Pat) -> &syn::Pat {
    match pat {
        syn::Pat::Type(t) => strip_binding(&t.pat),
        syn::Pat::Paren(p) => strip_binding(&p.pat),
        other => other,
    }
}

/// A scope holding one pattern's bindings; it closes when this drops.
pub struct PatternScope<'t, 'a> {
    translator: &'t BodyTranslator<'a>,
}

impl Drop for PatternScope<'_, '_> {
    fn drop(&mut self) {
        self.translator.pop_scope();
    }
}

// ── Standalone helpers ──────────────────────────────────────────────────

/// What `*x = y` falls back to when the engine cannot say what `x` wraps.
const ASSUMED_ACCESSOR: &str = "the wrapper accessor is assumed to be `value`";

/// Path segments dropped when a written path becomes a TypeScript expression.
/// Resolving the path properly is the value-namespace work in the engine; this
/// list is what stands in for it, and dropping any of them is recorded.
const STD_QUALIFIERS: [&str; 11] = [
    "std", "core", "alloc", "sync", "collections", "convert", "fmt", "ops", "iter", "atomic",
    "marker",
];

fn is_mut_binding(pat: &syn::Pat) -> bool {
    if let syn::Pat::Ident(ident) = pat {
        ident.mutability.is_some()
    } else {
        false
    }
}

fn translate_lit(lit: &syn::Lit) -> String {
    match lit {
        syn::Lit::Str(s) => format!("'{}'", s.value().replace('\'', "\\'")),
        syn::Lit::Int(i) => i.base10_digits().to_string(),
        syn::Lit::Float(f) => f.base10_digits().to_string(),
        syn::Lit::Bool(b) => if b.value { "true" } else { "false" }.to_string(),
        syn::Lit::Char(c) => format!("'{}'", c.value()),
        syn::Lit::Byte(b) => format!("{}", b.value()),
        _ => "/* unknown literal */".to_string(),
    }
}

/// Check if an expression references a variable name as a standalone path
/// (not as a field name in `a.field`). Used for shadow detection.
fn references_var(expr: &syn::Expr, name: &str) -> bool {
    match expr {
        syn::Expr::Path(path) => {
            // Standalone variable reference: just the name
            path.path.segments.len() == 1
                && path.path.segments[0].ident == name
        }
        syn::Expr::MethodCall(call) => {
            // Check receiver and args, but NOT the method name
            references_var(&call.receiver, name)
                || call.args.iter().any(|a| references_var(a, name))
        }
        syn::Expr::Call(call) => {
            references_var(&call.func, name)
                || call.args.iter().any(|a| references_var(a, name))
        }
        syn::Expr::Field(field) => {
            // Check the base, but NOT the field name
            references_var(&field.base, name)
        }
        syn::Expr::Binary(bin) => {
            references_var(&bin.left, name) || references_var(&bin.right, name)
        }
        syn::Expr::Unary(unary) => references_var(&unary.expr, name),
        syn::Expr::Reference(r) => references_var(&r.expr, name),
        syn::Expr::Paren(p) => references_var(&p.expr, name),
        syn::Expr::Block(b) => {
            b.block.stmts.iter().any(|s| match s {
                syn::Stmt::Expr(e, _) => references_var(e, name),
                _ => false,
            })
        }
        syn::Expr::Closure(c) => references_var(&c.body, name),
        _ => false,
    }
}


fn translate_binop(op: &syn::BinOp) -> &'static str {
    match op {
        syn::BinOp::Add(_) => "+",
        syn::BinOp::Sub(_) => "-",
        syn::BinOp::Mul(_) => "*",
        syn::BinOp::Div(_) => "/",
        syn::BinOp::Rem(_) => "%",
        syn::BinOp::And(_) => "&&",
        syn::BinOp::Or(_) => "||",
        syn::BinOp::BitXor(_) => "^",
        syn::BinOp::BitAnd(_) => "&",
        syn::BinOp::BitOr(_) => "|",
        syn::BinOp::Shl(_) => "<<",
        syn::BinOp::Shr(_) => ">>",
        syn::BinOp::Eq(_) => "===",
        syn::BinOp::Lt(_) => "<",
        syn::BinOp::Le(_) => "<=",
        syn::BinOp::Ne(_) => "!==",
        syn::BinOp::Ge(_) => ">=",
        syn::BinOp::Gt(_) => ">",
        syn::BinOp::AddAssign(_) => "+=",
        syn::BinOp::SubAssign(_) => "-=",
        syn::BinOp::MulAssign(_) => "*=",
        syn::BinOp::DivAssign(_) => "/=",
        syn::BinOp::RemAssign(_) => "%=",
        syn::BinOp::BitXorAssign(_) => "^=",
        syn::BinOp::BitAndAssign(_) => "&=",
        syn::BinOp::BitOrAssign(_) => "|=",
        syn::BinOp::ShlAssign(_) => "<<=",
        syn::BinOp::ShrAssign(_) => ">>=",
        _ => "/* unknown op */",
    }
}
