//! Values an expression produced that nothing in the source names.
//!
//! `self.entries.lock().unwrap().len()` builds a guard on its way to a number.
//! Rust gives that guard a lifetime — it lives to the end of the statement and
//! is dropped there — and the emitted TypeScript has nothing that can happen
//! to it at all: the expression names it nowhere, so nothing releases it and
//! the lock is held for the life of the program.
//!
//! So a temporary that owes a release is lifted out of the expression that
//! built it: it is given a name in a declaration standing above the statement,
//! the expression is written against that name, and the release is placed
//! where Rust places it. A condition is the one place that is not the end of
//! the statement — Rust drops a condition's temporaries as soon as the
//! condition has been evaluated, before the branch is taken — and
//! `settle_condition` is what writes that.

use crate::body::{indent, BodyTranslator};
use crate::ownership;

impl<'a> BodyTranslator<'a> {
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

    /// An operand a short circuit may skip, with whatever it took to evaluate
    /// itself declared and released inside it.
    ///
    /// `left && right` gives `right` nowhere to put a statement: a temporary
    /// lifted out of it stood above the whole expression and was taken whether
    /// or not `left` allowed the second test to run, so
    /// `false && *cell.lock().unwrap() == 0` locked the mutex the program never
    /// asked it to lock. A function called on the spot puts those statements
    /// back inside the branch the short circuit guards.
    ///
    /// An operand that leaves the function — a `?` inside it — or that awaits
    /// cannot be moved into one: the `return` would leave the wrapper rather
    /// than the body, and the await would need the wrapper to be async and the
    /// operand to be a promise. Those are reported and left where they stood.
    pub(crate) fn short_circuit_operand(
        &self,
        operand: &syn::Expr,
        written: String,
        lifted: Vec<ownership::Hoist>,
    ) -> String {
        if lifted.is_empty() {
            return written;
        }
        let leaves = lifted
            .iter()
            .any(|hoist| hoist.declaration.contains("return ") || hoist.declaration.contains("await "));
        if leaves {
            self.fallback(
                syn::spanned::Spanned::span(operand),
                "this operand takes a value of its own and leaves the function or awaits, so \
                 what it took is declared before the short circuit that decides whether the \
                 operand runs at all",
            );
            self.own.prelude.borrow_mut().extend(lifted);
            return written;
        }
        // The wrapper is a function, and JavaScript's `await` belongs to the
        // nearest one, so an operand that awaits gets an `async` wrapper and the
        // call is awaited where the operand stood.
        let body = ownership::hoisted(&format!("return {};\n", written), &lifted);
        if crate::control_flow::awaiting::awaits(operand) {
            format!("await (async () => {{\n{}}})()", indent(&body))
        } else {
            format!("(() => {{\n{}}})()", indent(&body))
        }
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
        // `Iterator::next` is declared `&mut self`, so the impl table says the
        // receiver survives the call — and for Rust it does, with the tail
        // still in it. The port has no cursor: on a receiver nobody else holds
        // it hands back the head and releases the rest, so the whole sequence
        // goes INTO the call and a second release here would drop that tail
        // twice (Q1).
        if self.lowering_takes_the_whole_sequence(call) {
            return written;
        }
        self.hoist_produced(&call.receiver, written)
    }

    /// Does this call's lowering take the sequence its receiver produced, so
    /// that the statement owes it nothing?
    fn lowering_takes_the_whole_sequence(&self, call: &syn::ExprMethodCall) -> bool {
        call.method == "next"
            && call.args.is_empty()
            && self.builds_its_own_sequence(&call.receiver)
            && self.adaptor_owns_its_elements(call)
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
        // A cascade is lifted too. An array, a `Map` or a `T | null` the
        // expression built owns what is inside it and has no `drop()` of its
        // own, so leaving it in place left everything it held with nothing to
        // release it: `look(&vec![Owned::new()])` leaked the whole vector.
        if !matches!(
            drops,
            ownership::Drops::Guard | ownership::Drops::Own | ownership::Drops::Cascade
        ) {
            return written;
        }
        self.hoist_temporary(written, drops)
    }

    /// Give an expression a name that stands before the statement it is written
    /// in, with no release of its own.
    ///
    /// For a value that is read more than once by the text being written —
    /// matched on, and then named again by an arm — where Rust evaluates it
    /// once. Who releases it is decided elsewhere, by whatever owns the value
    /// the expression produced, so this adds nothing to that.
    pub(crate) fn hoist_name(&self, written: String) -> String {
        let name = self.fresh_hoist("_m");
        self.own.prelude.borrow_mut().push(ownership::Hoist {
            declaration: format!("const {} = {};\n", name, written),
            owned: None,
            temp: None,
            refused: false,
            released_if_unreached: false,
        });
        name
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
            temp: None,
            refused: false,
            released_if_unreached: false,
        });
        name
    }
}
