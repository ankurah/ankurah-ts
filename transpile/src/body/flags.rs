//! Where a statement's MOVE FLAGS stand, and what has to stand above them.
//!
//! A flag is written before the statement it belongs to, because after a
//! `return` it would be dead code. That is right for the move itself and wrong
//! for everything the statement evaluates on the way there: an argument that
//! throws leaves the flag set and the moved value released by nobody.

use super::BodyTranslator;

/// One expression's extent, as a comparable key. Two clones of an expression
/// carry the same span, which is what lets a lowering ask "is this the
/// expression the statement is about" without holding a borrow of it.
pub(crate) fn span_key(expr: &syn::Expr) -> (usize, usize, usize, usize) {
    let span = syn::spanned::Spanned::span(expr);
    let (start, end) = (span.start(), span.end());
    (start.line, start.column, end.line, end.column)
}

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

    /// Give a value a name that stands ABOVE the statement's move flags.
    fn name_above_the_flag(&self, written: String) -> String {
        let name = self.fresh_hoist("_b");
        self.own
            .before_flags
            .borrow_mut()
            .push(format!("const {} = {};\n", name, written));
        name
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
        // And only one standing where the STATEMENT's own expression stands.
        // `before_flags` is written above the whole statement, so lifting an
        // argument of a call nested inside a branch, a closure or an IIFE the
        // statement writes would put it above the names that block declares —
        // `storage-indexeddb`'s `collection.ts` reads a `limitVal` bound inside
        // the `if` the call sits in, and `signals`' `calculated.ts` is the same
        // shape one level shallower.
        if self.own.statement_tail.get() != Some(span_key(whole)) {
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
        let lifts = |e: &&Option<&syn::Expr>| e.is_some_and(|e| !evaluates_quietly(e));
        if exprs.iter().filter(lifts).any(|e| self.moves_a_flagged_local(e.expect("filtered"))) {
            return written;
        }
        written
            .into_iter()
            .enumerate()
            .map(|(index, text)| match exprs.get(index) {
                // A `String::clone` is the identity in the port, so
                // `name.clone()` comes out as `name` and evaluating it cannot
                // throw however the Rust was written — nothing to lift. J1:
                // asked of the EXPRESSION and what the port writes for it,
                // rather than of the emitted text, because whether an operand
                // is a place is a fact the lowering has.
                Some(e) if lifts(&e) && !self.writes_a_place(e.expect("filtered"))
                    => self.name_above_the_flag(text),
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
            syn::Expr::MethodCall(call)
                if call.args.is_empty()
                    && matches!(
                        call.method.to_string().as_str(),
                        "clone" | "to_owned" | "to_string"
                    ) =>
            {
                self.writes_a_place(&call.receiver) && self.copies_by_reading(&call.receiver)
            }
            other => evaluates_quietly(other),
        }
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

    /// The statement's own outermost expression, for the placement rule above.
    /// A `return e` and a `break e` carry the expression the statement is
    /// really about, and a `let x = e` is the same question about its
    /// initialiser.
    pub(crate) fn statement_tail_key(stmt: &syn::Stmt) -> Option<(usize, usize, usize, usize)> {
        fn through(expr: &syn::Expr) -> &syn::Expr {
            match expr {
                syn::Expr::Return(r) => r.expr.as_deref().map(through).unwrap_or(expr),
                syn::Expr::Break(b) => b.expr.as_deref().map(through).unwrap_or(expr),
                syn::Expr::Paren(p) => through(&p.expr),
                syn::Expr::Group(g) => through(&g.expr),
                other => other,
            }
        }
        match stmt {
            syn::Stmt::Expr(expr, _) => Some(span_key(through(expr))),
            syn::Stmt::Local(local) => local
                .init
                .as_ref()
                .map(|init| span_key(through(&init.expr))),
            _ => None,
        }
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

/// Can this argument be left where it stands, because evaluating it cannot
/// throw?
///
/// A place — a name, a field, an index — is the original answer: reading one
/// cannot fail, and naming it would only add noise. A LITERAL and a CLOSURE are
/// the same: `f(c, false)` and `opt.map(|v| v)` build a value out of nothing,
/// and lifting them wrote `const _b2 = false;` and `const _b6 = (v) => v;`
/// above two live statements.
fn evaluates_quietly(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Lit(_) | syn::Expr::Closure(_)) || crate::body::is_place(expr)
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

    /// A call the statement does not evaluate at its own top level is left
    /// alone: `before_flags` stands above the WHOLE statement, so lifting an
    /// argument out of a branch would put it above the names that branch
    /// declares.
    #[test]
    fn a_call_inside_a_branch_of_the_statement_is_left_alone() {
        let ts = body(
            "pub fn f(c: Token, o: Option<u32>, e: bool) -> u32 {\n\
               if e { return 0; }\n\
               match o { Some(n) => eat(c, n + 1), None => 0 }\n\
             }",
            "f",
        );
        assert!(!ts.contains("const _b"), "an argument was lifted out of an arm:\n{}", ts);
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
