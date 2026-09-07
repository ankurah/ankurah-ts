//! Where a statement's MOVE FLAGS stand, and what has to stand above them.
//!
//! A flag is written before the statement it belongs to, because after a
//! `return` it would be dead code. That is right for the move itself and wrong
//! for everything the statement evaluates on the way there: an argument that
//! throws leaves the flag set and the moved value released by nobody.

use super::BodyTranslator;

mod operands;
pub(crate) use operands::{evaluates_quietly, text_calls, writes_a_literal};

impl BodyTranslator<'_> {
    /// What a body's own `let`s declare before its first statement: the
    /// formatter's accumulator, and one move FLAG per parameter that has a live
    /// one.
    ///
    /// A formatter composes one string, and every `write!` in it appends to
    /// that. It used to be spliced in by `emit.rs` searching the finished text
    /// for `_result +=`, which found the statement forms and not the tail — so
    /// a `Display` that ended in `write!(f, "b")` answered `"b"` rather than
    /// everything it had written.
    ///
    /// E15: a flag the body never SETS is a `let` nothing assigns beside a test
    /// that is always false. `ownership::wrap` has already dropped it from the
    /// release; this is the declaration going with it.
    pub(crate) fn block_declarations(
        &self,
        owned: &[crate::ownership::Owned],
        body: &str,
    ) -> String {
        let mut declarations = String::new();
        if self.formatter && self.wrote_result.get() {
            declarations.push_str("let _result = '';\n");
        }
        for flag in owned.iter().filter_map(|p| p.flag.as_ref()) {
            if crate::ownership::sets_the_flag(body, flag) {
                declarations.push_str(&format!("let {} = false;\n", flag));
            }
        }
        declarations
    }

    /// Take out the flag declarations nothing assigns, and the guards that
    /// read them.
    ///
    /// E15: a flag says "somebody else owns this now", and a body that never
    /// sets it never hands the value away — so the flag is a `let` nothing
    /// assigns beside a test that is always false. `ownership::wrap` drops such
    /// a guard on its own, and takes the declaration with it where the two
    /// stand in the SAME text.
    ///
    /// N4: for a `let` they do not. The declaration went into the statement
    /// that declared the local, and the release wraps the REST of the block, so
    /// `wrap` — handed only the rest — dropped the guard and left the
    /// declaration standing seventy lines above nothing
    /// (`storage-indexeddb/collection.ts`). Here both streams are in hand, so
    /// the answer is read from both and the declaration comes out of the one it
    /// was written into.
    pub(crate) fn drop_dead_flags(
        &self,
        declarations: &mut String,
        rest: &str,
        owned: &mut [crate::ownership::Owned],
    ) {
        for local in owned.iter_mut() {
            let Some(flag) = local.flag.clone() else { continue };
            if crate::ownership::sets_the_flag(declarations, &flag)
                || crate::ownership::sets_the_flag(rest, &flag)
            {
                continue;
            }
            *declarations = crate::ownership::without_declaration(declarations, &flag);
            local.flag = None;
        }
    }

    /// Give a value a name that stands ABOVE the move flag, in the scope the
    /// expression itself stands in.
    ///
    /// O6: an ordinary HOIST, not a statement-global buffer. `before_flags`
    /// stood above the whole statement, so a call inside a branch, a closure or
    /// an `await` chain could not lift anything at all — its lift would have
    /// landed above the names that block declares — and the statement-tail test
    /// that kept it safe left every such call with its flags above the throw. A
    /// hoist lands wherever hoists land, which is exactly the scope the call is
    /// written in.
    ///
    /// N3: and it carries a release. A temporary lifted here holds a value
    /// Rust had not yet built at all — the argument is evaluated where the call
    /// is — so if the call is never reached, nobody owns what the lift
    /// produced. The release asks the runtime whether the value still has an
    /// owner, exactly as slice 6's refusal-owned prelude does, because the
    /// call may have consumed it.
    fn name_above_the_flag(&self, written: String, owes_a_release: bool, droppable: bool) -> String {
        let name = self.fresh_hoist("_b");
        // S1: the release is read from a flag this frame declares, not from a
        // mark on the value. `markMoved` is protected on `AkObject` and only
        // base's own wrappers call it, so an array, a `Map` or a `Set` — which
        // is what the port writes a `Vec`, a `HashMap` and a `HashSet` as —
        // always answered "nobody has taken it" and the release ran on top of
        // the callee that had just taken the value.
        let flag = owes_a_release.then(|| self.fresh_hoist("_moved"));
        let declaration = match &flag {
            Some(flag) => format!("let {} = false;\nconst {} = {};\n", flag, name, written),
            None => format!("const {} = {};\n", name, written),
        };
        self.own.prelude.borrow_mut().push(crate::ownership::Hoist {
            declaration,
            owned: None,
            temp: Some(name.clone()),
            refused: false,
            released_if_unreached: owes_a_release,
            wrapper: false,
            sets: String::new(),
            droppable,
            flag,
        });
        name
    }

    /// Does the value this expression builds owe a release, so that lifting it
    /// above something that can throw needs one written?
    fn lift_owes_a_release(&self, expr: &syn::Expr) -> bool {
        let Some(tc) = &self.types else { return false };
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(expr)) else { return false };
        crate::ownership::drops_of(&tc.borrow().probe(), &ty).is_droppable()
    }

    /// The arguments of a call that hands a flagged local away, each given a
    /// name that stands above the flag.
    ///
    /// J3: a move flag is written before the statement it belongs to, because
    /// after a `return` it would be dead code. That is right for the move
    /// itself and wrong for everything the statement evaluates on the way
    /// there: an argument that throws leaves the flag set and the callee
    /// released by nobody. Naming the arguments first puts every expression
    /// that can throw above the flag, and a place — a name, a field, a literal
    /// — is left alone, because reading one cannot throw and naming it would
    /// only add noise.
    ///
    /// E10: the rule reached `invoke(..)` alone, because it asked whether the
    /// CALLEE was a path naming a flagged local. Every other call shape kept
    /// the defect: `eat(c, o.unwrap())` on a flagged `c` wrote
    /// `_moved0 = true; eat(c, o.unwrap());`, and an `unwrap` that throws left
    /// the flag set with `c` released by nobody. What decides it is whether
    /// THIS CALL hands a flagged local away, which is the same question the
    /// flag assignment asks, so a plain function call, a method call and a
    /// constructor all take the same placement.
    pub(crate) fn lifted_above_the_flag(
        &self,
        whole: &syn::Expr,
        exprs: &[Option<&syn::Expr>],
        written: Vec<String>,
    ) -> Vec<String> {
        // Only a call that really hands a flagged local away has a flag to
        // stand after; lifting anywhere else would add a name and say nothing.
        if !self.moves_a_flagged_local(whole) {
            return written;
        }
        // §3.9: the arguments go above the flag only where the LIST cannot
        // contain the move. An argument that is not a place is evaluated where
        // it is lifted to, so one that performs the move itself would move
        // BEFORE the flag is set — and an argument after it that throws would
        // then leave a moved value with a flag saying otherwise. All or none:
        // lifting some and not others would also reorder the evaluation Rust
        // wrote, which reading a PLACE cannot observe and evaluating an
        // expression can.
        // W2: what has to stand above the flag is everything that EVALUATES
        // between the flag and the call — asked of the Rust expression and of
        // the text the port wrote for it, because the port writes calls the
        // source does not have. `Event { entity_id: self.id, .. }` reads a
        // field, which cannot panic in Rust, and the port writes
        // `this.deref().id` because the entity is behind a `Deref`: `deref()`
        // on a value somebody dropped throws, and the flag above it said the
        // constructor had already taken the collection clone.
        //
        // A `String::clone` is the identity in the port, so `name.clone()`
        // comes out as `name` and evaluating it cannot throw however the Rust
        // was written — nothing to lift. J1: the EXPRESSION decides first,
        // because whether an operand is a place is a fact the lowering has;
        // the text is read only to catch what the port itself added, and a
        // closure is exempt because its parentheses hold PARAMETERS and call
        // nothing where they stand.
        let evaluates = |index: usize, e: &Option<&syn::Expr>| {
            e.is_some_and(|e| {
                let text = written.get(index).map(String::as_str).unwrap_or("");
                // V5: a value the port writes as a LITERAL builds nothing that
                // can throw, so it does not have to stand above the flag — and
                // lifting it takes it OUT of the position that typed it.
                // `Vec::new()` in a struct-literal field is `[]`, and
                // `const _b2 = [];` is `any[]`, which `noImplicitAny` reports
                // twice at every such site.
                if writes_a_literal(text) {
                    return false;
                }
                !self.writes_a_place(e) || text_calls(e, text)
            })
        };
        if exprs
            .iter()
            .enumerate()
            .filter(|(index, e)| evaluates(*index, e))
            .any(|(_, e)| self.moves_a_flagged_local(e.expect("filtered")))
        {
            return written;
        }
        // Only a lift with something AFTER it that can throw needs a FLAG: the
        // flag assignment cannot throw, and neither can a place, a literal or a
        // closure left where it stands. The LAST lift is therefore free of one
        // — but not of the release itself, which X2 keeps until a transfer the
        // port really wrote discharges it.
        let last_lift = (0..exprs.len()).rev().find(|index| evaluates(*index, &exprs[*index]));
        let lift: Vec<bool> = (0..written.len()).map(|i| evaluates(i, exprs.get(i).unwrap_or(&None))).collect();
        written
            .clone()
            .into_iter()
            .enumerate()
            .map(|(index, text)| match lift.get(index) {
                Some(true) => {
                    let expr = exprs[index].expect("evaluates answered on a Some");
                    let droppable = self.lift_owes_a_release(expr);
                    let owes = Some(index) != last_lift && droppable;
                    self.name_above_the_flag(text, owes, droppable)
                }
                _ => text,
            })
            .collect()
    }

    /// Does the port write this expression as a PLACE — a name or a member
    /// read — whatever Rust's own expression is?
    ///
    /// `evaluates_quietly` answers the Rust question: a path, a field, an
    /// index, a literal, a closure. This answers the port's, which is wider by
    /// exactly the calls the port writes as the identity: `String::clone`,
    /// `to_owned` and `to_string` on a value the port holds as a JavaScript
    /// value are the receiver itself, so `name.clone()` IS `name`. Reading one
    /// cannot throw, so there is nothing to lift above a move flag.
    fn writes_a_place(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Paren(p) => self.writes_a_place(&p.expr),
            syn::Expr::Group(g) => self.writes_a_place(&g.expr),
            syn::Expr::Reference(r) => self.writes_a_place(&r.expr),
            syn::Expr::Unary(syn::ExprUnary { op: syn::UnOp::Deref(_), expr, .. }) => {
                self.writes_a_place(expr)
            }
            // `clone` and `to_owned` on a value the port holds as a JavaScript
            // VALUE are the value itself: the emitted text is the receiver, so
            // the whole expression is still the place the receiver was.
            syn::Expr::MethodCall(call)
                if call.args.is_empty()
                    && matches!(call.method.to_string().as_str(), "clone" | "to_owned") =>
            {
                self.writes_a_place(&call.receiver) && self.copies_by_reading(&call.receiver)
            }
            // R13(e): `to_string` is that only on a `str`, where the emitted
            // text is again the receiver. On a number, a boolean or a `bigint`
            // it BUILDS — `String(n)` — and calling the answer a place was right
            // only by accident, because that particular call cannot throw. The
            // rule is about what the text IS, so it asks about the text.
            syn::Expr::MethodCall(call)
                if call.args.is_empty() && call.method == "to_string" =>
            {
                self.writes_a_place(&call.receiver) && self.is_already_a_string(&call.receiver)
            }
            other => evaluates_quietly(other),
        }
    }

    /// Is this receiver ALREADY a JavaScript string, so that `to_string` on it
    /// writes the receiver and builds nothing?
    fn is_already_a_string(&self, receiver: &syn::Expr) -> bool {
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(receiver)) else { return false };
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        matches!(
            crate::name_map::shape::js_shape(tc.registry, ty.peel_refs()),
            crate::name_map::shape::JsShape::Str
        )
    }

    /// Is this receiver a value the port holds as a JavaScript VALUE, so that
    /// copying it is reading it? A `string`, a number, a boolean and a `bigint`
    /// are; everything with a class of its own has a `clone()` that builds.
    fn copies_by_reading(&self, receiver: &syn::Expr) -> bool {
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(receiver)) else { return false };
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        matches!(
            crate::name_map::shape::js_shape(tc.registry, ty.peel_refs()),
            crate::name_map::shape::JsShape::Str
                | crate::name_map::shape::JsShape::Number
                | crate::name_map::shape::JsShape::Boolean
                | crate::name_map::shape::JsShape::BigInt
        )
    }

    /// The argument expressions of a call, in the order their emitted text was
    /// built, for the placement rule to read.
    pub(crate) fn each_argument<'e>(
        exprs: &'e syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
    ) -> Vec<Option<&'e syn::Expr>> {
        exprs.iter().map(Some).collect()
    }

    /// Does this expression hand away a local the block gave a move FLAG?
    ///
    /// Asked exactly as `flag_sets` asks it — the same shallow move scan over
    /// the same flag map — so the placement and the assignment cannot disagree
    /// about which statements have a flag at all.
    fn moves_a_flagged_local(&self, expr: &syn::Expr) -> bool {
        if self.own.flags.borrow().is_empty() {
            return false;
        }
        let stmt = syn::Stmt::Expr(expr.clone(), None);
        crate::ownership::Scan::new(self)
            .shallow(&stmt)
            .iter()
            .any(|site| self.own.flags.borrow().contains_key(&site.name))
    }
}

/// The operand of every `?` this statement writes at its own level.
///
/// A `?` inside a CLOSURE belongs to the closure, which has its own prelude and
/// its own flags, so the walk stops there.
pub(crate) fn try_operands(stmt: &syn::Stmt) -> Vec<syn::Expr> {
    use syn::visit::Visit;
    #[derive(Default)]
    struct Walk {
        found: Vec<syn::Expr>,
    }
    impl<'ast> Visit<'ast> for Walk {
        fn visit_expr_try(&mut self, node: &'ast syn::ExprTry) {
            self.found.push((*node.expr).clone());
            syn::visit::visit_expr_try(self, node);
        }
        fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
    }
    let mut walk = Walk::default();
    walk.visit_stmt(stmt);
    walk.found
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    const PRELUDE: &str = "pub struct Token(pub u32);\n\
                           impl Drop for Token { fn drop(&mut self) { } }\n\
                           pub struct Sink;\n\
                           impl Sink { pub fn swallow(&self, t: Token, n: u32) -> u32 { n } }\n\
                           pub struct Held { pub t: Token, pub n: u32 }\n\
                           pub fn eat(t: Token, n: u32) -> u32 { n }\n";

    fn body(rust: &str, method: &str) -> String {
        let mut f = Fixture::build(&[("lib.rs", &format!("{}{}", PRELUDE, rust))]);
        f.translated_method("lib.rs", method)
    }

    /// E10: the flag stands after everything the statement evaluates, whatever
    /// SHAPE the call is. It reached `invoke(..)` alone, because the rule asked
    /// whether the CALLEE was a path naming a flagged local.
    #[test]
    fn the_flag_stands_after_the_argument_list_for_every_call_shape() {
        for (rust, method) in [
            (
                "pub fn f(c: Token, o: Option<u32>, e: bool) -> u32 {\n\
                   if e { return 0; }\n\
                   eat(c, o.unwrap())\n\
                 }",
                "f",
            ),
            (
                "pub fn f(s: &Sink, c: Token, o: Option<u32>, e: bool) -> u32 {\n\
                   if e { return 0; }\n\
                   s.swallow(c, o.unwrap())\n\
                 }",
                "f",
            ),
            (
                "pub fn f(c: Token, o: Option<u32>, e: bool) -> Held {\n\
                   if e { return Held { t: Token(0), n: 0 }; }\n\
                   Held { t: c, n: o.unwrap() }\n\
                 }",
                "f",
            ),
        ] {
            let ts = body(rust, method);
            let lift = ts.find("const _b").expect(&format!("nothing was lifted:\n{}", ts));
            let flag = ts.find("_moved0 = true;").expect(&format!("no flag:\n{}", ts));
            assert!(lift < flag, "the flag stands before what can throw:\n{}", ts);
        }
    }

    /// A place, a literal and a closure are left where they stand: evaluating
    /// one cannot throw, and naming it would only add noise —
    /// `const _b2 = false;` and `const _b6 = (v) => v;` above two live
    /// statements is what the first draft wrote.
    #[test]
    fn what_cannot_throw_is_left_where_it_stands() {
        let ts = body(
            "pub fn f(c: Token, n: u32, e: bool) -> u32 {\n\
               if e { return 0; }\n\
               eat(c, 7)\n\
             }",
            "f",
        );
        assert!(!ts.contains("const _b"), "a literal was lifted:\n{}", ts);
        assert!(ts.contains("eat(c, 7)"), "{}", ts);
    }

    /// O6: a call inside a BRANCH lifts too, and what it lifts stands inside
    /// that branch.
    ///
    /// This test used to assert the opposite, and the premise it rested on has
    /// been repealed: the lift went into a statement-global buffer that stood
    /// above the whole statement, so lifting out of a branch would have put the
    /// name above the bindings that branch declares. It is an ordinary HOIST
    /// now, which lands in the scope the call is written in — and the flag
    /// stands below it, which is the whole point. This arm also wrote no flag
    /// at all before, so `eat(c, ..)` handed `c` over and the block's `finally`
    /// released it a second time.
    #[test]
    fn a_call_inside_a_branch_lifts_into_that_branch() {
        let ts = body(
            "pub fn f(c: Token, o: Option<u32>, e: bool) -> u32 {\n\
               if e { return 0; }\n\
               match o { Some(n) => eat(c, n + 1), None => 0 }\n\
             }",
            "f",
        );
        let bind = ts.find("const n = o;").expect(&format!("the arm binds:\n{}", ts));
        let lift = ts.find("const _b").expect(&format!("nothing was lifted:\n{}", ts));
        let flag = ts.find("_moved0 = true;").expect(&format!("no flag:\n{}", ts));
        assert!(bind < lift, "the lift stands inside the arm:\n{}", ts);
        assert!(lift < flag, "and the flag stands below it:\n{}", ts);
        assert!(ts.contains("if (!_moved0) c.drop();"), "the release is guarded:\n{}", ts);
    }

    /// N1: a field's BASE and an index's INDEX decide whether reading it can
    /// throw. `is_place` says yes to any `Expr::Field` and any `Expr::Index`
    /// without looking at either, so the flag was set before `maybe()` and
    /// `which()` ran.
    #[test]
    fn a_field_of_a_call_and_an_index_that_is_a_call_are_lifted() {
        for rust in [
            "pub fn maybe() -> Held { Held { t: Token(0), n: 1 } }\n\
             pub fn f(c: Token, e: bool) -> u32 {\n\
               if e { return 0; }\n\
               eat(c, maybe().n)\n\
             }",
            "pub fn which() -> usize { 0 }\n\
             pub fn f(c: Token, xs: Vec<u32>, e: bool) -> u32 {\n\
               if e { return 0; }\n\
               eat(c, xs[which()])\n\
             }",
        ] {
            let ts = body(rust, "f");
            let flag = ts.find("_moved0 = true;").expect(&format!("no flag:\n{}", ts));
            let call = ts.find("return eat(c,").expect(&format!("no call:\n{}", ts));
            let opens = ts[..flag].contains("maybe()") || ts[..flag].contains("which()");
            assert!(opens, "what can throw stands above the flag:\n{}", ts);
            assert!(flag < call, "and the flag immediately above the transfer:\n{}", ts);
        }
    }

    /// N4: a `let`'s flag declaration goes into the statement that declared the
    /// local, and its guard wraps the REST of the block. Where the body never
    /// sets the flag, `wrap` drops the guard — and, handed only the rest, left
    /// the declaration standing. Live at `storage-indexeddb/collection.ts`,
    /// where a `let _moved0 = false;` stood seventy lines above nothing.
    #[test]
    fn a_dead_flag_takes_its_declaration_with_it() {
        let ts = body(
            "pub fn f(o: Option<Token>) -> u32 {\n\
               let mut held = Vec::new();\n\
               if let Some(t) = o { held.push(t); }\n\
               held.len() as u32\n\
             }",
            "f",
        );
        let declares = ts.contains("= false;");
        let guards = ts.contains("if (!_moved");
        assert_eq!(declares, guards, "a flag is declared exactly where it is read:\n{}", ts);
    }

    /// A field of a NAME is still quiet: reading one cannot throw.
    #[test]
    fn a_field_of_a_name_is_still_left_where_it_stands() {
        let ts = body(
            "pub fn f(c: Token, h: Held, e: bool) -> u32 {\n\
               if e { return 0; }\n\
               eat(c, h.n)\n\
             }",
            "f",
        );
        assert!(!ts.contains("const _b"), "nothing to lift:\n{}", ts);
        assert!(ts.contains("eat(c, h.n)"), "{}", ts);
    }

    /// J1: what is lifted above a move flag is decided from the EXPRESSION and
    /// what the port writes for it, not from the emitted text.
    ///
    /// Read off the text, only a bare NAME was left alone — so
    /// `self.property_name.clone()`, which the port writes as the place
    /// `this.propertyName`, was lifted into a temporary that says nothing.
    /// Live at `core/property/value/lww.ts`.
    #[test]
    fn a_place_the_port_writes_as_itself_is_not_lifted() {
        let ts = body(
            "pub struct Named { pub name: String }\n\
             impl Named {\n\
               pub fn take(&self, s: String, t: Token) -> u32 { 0 }\n\
               pub fn give(&self, t: Token) -> u32 {\n\
                 if true { return self.take(self.name.clone(), t); }\n\
                 0\n\
               }\n\
             }",
            "give",
        );
        assert!(ts.contains("_moved"), "the call really does hand a flagged local away:\n{}", ts);
        assert!(
            !ts.contains("const _b"),
            "`String::clone` is the identity here, so `this.name` is a place and there is \
             nothing to lift:\n{}",
            ts
        );
        // And an argument that CAN throw is still lifted above the flag.
        let throws = body(
            "pub struct Sink2;\n\
             impl Sink2 {\n\
               pub fn take(&self, n: u32, t: Token) -> u32 { n }\n\
               pub fn give(&self, o: Option<u32>, t: Token) -> u32 {\n\
                 if true { return self.take(o.unwrap(), t); }\n\
                 0\n\
               }\n\
             }",
            "give",
        );
        assert!(throws.contains("const _b"), "an `unwrap` can throw:\n{}", throws);
    }
}
