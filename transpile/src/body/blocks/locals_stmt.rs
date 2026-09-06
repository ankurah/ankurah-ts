//! What a `let` statement becomes.
//!
//! Split out of `blocks.rs`, which had grown past the 600-line rule. A `let` is
//! the one statement that puts a NAME in scope for everything after it, so it
//! is also the one whose ownership question — who releases what this binds, and
//! for how long — is answered here rather than in the block walk.

use crate::body::BodyTranslator;
use crate::body::places::EntryFinish;
use crate::ownership;

use crate::body::{as_move_closure, is_mut_binding, pattern_names, references_var};

impl BodyTranslator<'_> {
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
        // be asked before the binding is made — and of EVERY name the pattern
        // binds, because `let [queryId, ..] = ..` shadows each of them on its
        // own.
        let mut shadowing: Vec<String> = pattern_names(&local.pat)
            .into_iter()
            .filter(|name| self.redeclares_here(name))
            .collect();
        // `let _ = expr;` binds nothing in Rust and `const _ = expr;` binds a
        // variable called `_` here, so a second one in the same scope — beside
        // a closure parameter the source also wrote `_` — is a duplicate
        // declaration. It takes a fresh name for the same reason a shadow does.
        if pattern_names(&local.pat).is_empty() && self.redeclares_here(&pat) {
            shadowing.push(pat.clone());
        }
        let already_in_scope = !shadowing.is_empty();

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
        let entry_slot = self.finishes_an_entry(&init.expr);
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
            None => self.expecting(&init.expr, annotation.as_ref(), || match entry_slot {
                // R1: the `let` answers the same question the `*` does.
                EntryFinish::Slot => {
                    self.through_place(&init.expr, || self.moved_value(&init.expr))
                }
                EntryFinish::Hole | EntryFinish::Neither => self.moved_value(&init.expr),
            }),
        };

        // Only now does the name mean the new value. A `let` may take one
        // apart — `let (a, b) = ...`, `let Foo { x } = ...` — so every name
        // the pattern writes is bound, each typed from its own position.
        if self.types.is_some() {
            self.bind_pattern_here(&local.pat, ty.as_ref());
        }

        // `let PAT = e else { .. };` — the pattern is REFUTABLE, and the else
        // block runs when it does not match. Both the test and the else block
        // were dropped: `let ScanState::Scanning { .. } = state else { return
        // None };` came out as the destructuring alone, so the variant was
        // never tested and the `return None` was gone. Twelve sites.
        if let Some((_tok, diverge)) = &init.diverge {
            let subject = self.fresh_temp();
            let (test, bind) = self.pattern_test(&subject, &local.pat);
            // The else block diverges — Rust requires it — so it is written as
            // statements, where a `return`, a `break` and a `throw` all mean
            // what they meant in the source.
            let otherwise = self.statements(diverge);
            let bind = self.freshen_bindings(bind, &shadowing);
            return format!(
                "const {} = {};\nif (!({})) {{\n{}}}\n{}",
                subject,
                expr,
                test,
                crate::body::indent(&format!("{}\n", otherwise.trim_end())),
                bind
            );
        }

        // C1: a local this body hands out as `&mut` and whose type the port
        // writes as a JavaScript VALUE lives in a cell, because a number, a
        // string and a boolean are copied at the call and the callee's writes
        // would go nowhere. Decided here, where the type is known.
        let wants_a_cell = self.cell_candidates.borrow().iter().any(|c| *c == pat)
            && ty
                .as_ref()
                .is_some_and(|ty| match &self.types {
                    Some(tc) => crate::is_value_spelling(&crate::name_map::map_ty(
                        tc.borrow().registry,
                        ty,
                    )),
                    None => false,
                });
        // A finisher the engine had to REFUSE wrote a hole, not a slot, and
        // reading `.value` off a hole says nothing the hole does not already.
        // The disposition comes from the LOWERING (I1): reading it off the
        // rendered text meant an initialiser whose value carried the characters
        // `unsupported(` for any other reason stopped binding the slot.
        let entry_slot = entry_slot == EntryFinish::Slot;
        if wants_a_cell || entry_slot {
            // `freshen` hands out a NEW name each time it is asked, so the
            // rename is computed once, here, and not again below.
            let name = self.freshened_pattern(&local.pat, &shadowing);
            self.hold_in_a_cell(&name);
            // A finisher already answers the runtime's write-through slot; a
            // value local needs one built around it.
            let held = match entry_slot {
                true => expr,
                false => format!("new BorrowMut({})", expr),
            };
            return format!("const {} = {};\n", name, held);
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
            self.freshened_pattern(&local.pat, &shadowing)
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

}
