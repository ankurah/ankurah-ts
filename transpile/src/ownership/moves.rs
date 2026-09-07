//! Which locals were handed to somebody else before the block ended.
//!
//! Rust's compiler tracks this to decide where drop glue runs; the emitter has
//! to reach the same answer, because releasing a value its new owner also
//! releases is a double drop, and Rust's own answer is not written down at the
//! use site.
//!
//! The analysis is syntactic, which is enough because it only ever runs on
//! values that have drop glue — and a droppable value is not `Copy`, so Rust's
//! rule is exactly the syntax: a bare name in a position that takes a value
//! moves; a `&x` borrows. The one question the syntax cannot answer is whether
//! a method takes its receiver by value, and the caller supplies that from the
//! impl table.
//!
//! Where the answer is not clean, the default is "moved": a value that was
//! moved and is dropped anyway is released under a live owner, which corrupts
//! the program; a value that was kept and is not dropped is a leak the
//! registry reports. The first is worse, and the memo's rule follows.

pub(super) use super::dispositions::collect_pattern_names;


/// What the block should do with a local when it ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Nothing took it: release it in the block's `finally`.
    Kept,
    /// It was handed away on every path that reaches here: release nothing.
    Moved,
    /// It was handed away on some paths. Rust compiles a drop flag for this,
    /// and so does the emitter: a boolean set at the move site and read by the
    /// `finally`.
    Flagged,
    /// It was handed away somewhere the emitter cannot place the flag — inside
    /// a `?:` branch or a short-circuit operand. Reported, and not released.
    Unsure,
}

/// Where a move was written, relative to the block that declared the local.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Where {
    /// A statement of the declaring block itself, outside any branch: the value
    /// is gone by the time the block ends, on every path.
    Straight,
    /// Inside a nested block — a branch, a loop body, a match arm. The block
    /// that holds it emits the flag as one of its own statements.
    Branch,
    /// In THIS statement, with something the statement has still to evaluate
    /// standing between the move and the call that performs it.
    ///
    /// X5: `take2(token, o.unwrap())` moves `token` on every path the SOURCE
    /// has, so the site read as straight-line and the block wrote no release at
    /// all — and `unwrap` on a `None` throws with the token handed to nobody,
    /// which Rust drops while it unwinds. It is a conditional move like any
    /// other, and it is one the statement itself writes the flag for, which is
    /// what separates it from `Branch`.
    Evaluated,
    /// Inside a closure. Rust captures by value to make the move possible, so
    /// the closure owns the value from here.
    Closure,
    /// Somewhere no statement can be written: a `?:` branch, an operand of
    /// `&&` or `||`.
    Unwritable,
}

/// A move the scan found, with the position that decides how it is handled.
#[derive(Debug, Clone)]
pub struct Site {
    pub name: String,
    pub(super) at: Where,
    pub span: proc_macro2::Span,
}

/// Reads the one thing the syntax cannot say: whether a method call takes its
/// receiver by value. `Result::unwrap`, `Option::take` and the `into_*` family
/// all do, and the impl table knows which.
pub trait Consumes {
    fn consumes_receiver(&self, call: &syn::ExprMethodCall) -> bool;

    /// Is this call one the engine REFUSES, so that a hole stands where the
    /// whole call would have?
    ///
    /// J4: a hole throws before anything the call would have consumed reaches a
    /// new owner, so the receiver and every argument are still the block's.
    /// Counting them as moved left the block releasing nothing and the values
    /// to the leak check — a leak on the refusal path, which is the one path a
    /// reported gap is supposed to make safe.
    fn refuses_call(&self, call: &syn::ExprMethodCall) -> bool;

    /// Whether a `match` hands its subject's payload to an arm. Rust moves the
    /// subject there, and the emitted `intoMatch` leaves it moved, so the block
    /// that declared it must not release it as well.
    fn consumes_scrutinee(&self, m: &syn::ExprMatch) -> bool;

    /// The same for the pattern of an `if let` or a `while let`, which takes
    /// its subject apart exactly as an arm does.
    fn consumes_let_scrutinee(&self, let_expr: &syn::ExprLet) -> bool;

    /// Which operands an overloaded operator takes by value. Rust's operator
    /// traits are written `fn add(self, rhs: Rhs)`, so `a + b` between two
    /// ported types releases both — and the syntax of `+` says nothing about
    /// it, which is why the impl table has to.
    fn consumes_operands(&self, bin: &syn::ExprBinary) -> (bool, bool);

    /// Does calling this closure consume it? A closure whose body hands a
    /// capture away is an `FnOnce`: Rust lets it run once, and the runtime's
    /// `callOnce` marks it moved before the body runs.
    fn consumes_callee(&self, call: &syn::ExprCall) -> bool;

    /// Is `*expr` a deref that MOVES — the `Box` deref-move — rather than the
    /// `Deref` trait's borrow?
    ///
    /// `*boxed` takes the value out of the box and the box goes with it;
    /// `*guard` on a `MutexGuard` reads through it and takes nothing. Only a
    /// `Box` has the first, and the port erases it, so the emitted text is the
    /// same either way and the syntax alone cannot tell them apart.
    fn derefs_by_value(&self, expr: &syn::Expr) -> bool;

    /// Does the impl behind a unary operator take its operand by value?
    /// `impl Neg for Weight` is `fn neg(self) -> Weight`, so `-weight` releases
    /// the weight exactly as `a + b` releases both of its operands.
    fn consumes_unary_operand(&self, unary: &syn::ExprUnary) -> bool;
}

mod walk;

pub struct Scan<'c> {
    pub consumes: &'c dyn Consumes,
    /// Names a nested block declared, innermost frame last. A move of one of
    /// those is a move of *that* binding, not of the outer local it shadows:
    /// `let sub = { let counter = counter.clone(); move || use(counter) }`
    /// hands the clone to the closure and leaves the outer `counter` alone.
    shadowed: std::cell::RefCell<Vec<Vec<String>>>,
}

impl<'c> Scan<'c> {
    pub fn new(consumes: &'c dyn Consumes) -> Scan<'c> {
        Scan {
            consumes,
            shadowed: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Every move written in `stmts`, in source order.
    pub fn block(&self, stmts: &[syn::Stmt]) -> Vec<Site> {
        let mut out = Vec::new();
        self.statements(stmts, Where::Straight, &mut out);
        out
    }

    /// The same, with each site tagged by the statement that wrote it, so a
    /// block can attribute a move to the declaration in scope where it stands.
    pub fn block_indexed(&self, stmts: &[syn::Stmt]) -> Vec<(usize, Site)> {
        let mut out = Vec::new();
        let mut reachable = Where::Straight;
        for (index, stmt) in stmts.iter().enumerate() {
            let mut sites = Vec::new();
            self.stmt(stmt, reachable, &mut sites);
            out.extend(sites.into_iter().map(|site| (index, site)));
            if reachable == Where::Straight && leaves_the_block(stmt) {
                reachable = Where::Branch;
            }
        }
        out
    }

    /// Walk a nested block's statements in a scope of their own, so that a name
    /// it declares stops standing for the one outside it.
    pub(super) fn nested_block(&self, stmts: &[syn::Stmt], at: Where, out: &mut Vec<Site>) {
        self.shadowed.borrow_mut().push(Vec::new());
        self.statements(stmts, nested(at), out);
        self.shadowed.borrow_mut().pop();
    }

    /// A run of statements, in source order.
    ///
    /// A move written straight-line is on every path *until something above it
    /// can leave the block*. After a `return`, a `break`, a `continue` or a `?`,
    /// the statements below it are conditional, and a move there needs the drop
    /// flag a branch would get — otherwise the early exit leaves the value with
    /// nothing to release it.
    pub(super) fn statements(&self, stmts: &[syn::Stmt], at: Where, out: &mut Vec<Site>) {
        let mut reachable = at;
        for stmt in stmts {
            self.stmt(stmt, reachable, out);
            if reachable == Where::Straight && leaves_the_block(stmt) {
                reachable = Where::Branch;
            }
        }
    }

    /// Is this name one a nested scope declared for itself?
    pub(super) fn is_shadowed(&self, name: &str) -> bool {
        self.shadowed
            .borrow()
            .iter()
            .any(|frame| frame.iter().any(|n| n == name))
    }

    /// The moves this one statement writes *itself* — not the ones inside a
    /// nested block, which that block emits as it translates. This is what a
    /// statement's flag assignments are written from.
    pub fn shallow(&self, stmt: &syn::Stmt) -> Vec<Site> {
        let mut out = Vec::new();
        self.stmt(stmt, Where::Straight, &mut out);
        out.retain(|s| {
            matches!(s.at, Where::Straight | Where::Closure | Where::Evaluated)
        });
        out
    }

    pub(super) fn stmt(&self, stmt: &syn::Stmt, at: Where, out: &mut Vec<Site>) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    // `let y = x;` hands x to y.
                    self.moved(&init.expr, at, out);
                    self.walk(&init.expr, at, out);
                    if let Some((_, diverge)) = &init.diverge {
                        self.walk(diverge, nested(at), out);
                    }
                }
                // From here on this name means what this `let` bound. Only a
                // nested frame records it: the declaring block's own locals are
                // exactly what the scan is asking about.
                if let Some(frame) = self.shadowed.borrow_mut().last_mut() {
                    let mut names = Vec::new();
                    collect_pattern_names(&local.pat, &mut names);
                    frame.extend(names);
                }
            }
            // A block's trailing expression is its value, so a bare name there
            // is handed to whoever asked for the block — which is a move. A
            // statement with a semicolon throws its value away instead.
            syn::Stmt::Expr(expr, None) => self.tail(expr, at, out),
            syn::Stmt::Expr(expr, Some(_)) => self.walk(expr, at, out),
            syn::Stmt::Macro(mac) => self.macro_args(&mac.mac, at, out),
            syn::Stmt::Item(_) => {}
        }
    }

    /// A block's value.
    pub(super) fn tail(&self, expr: &syn::Expr, at: Where, out: &mut Vec<Site>) {
        self.moved(expr, at, out);
        self.walk(expr, at, out);
    }

    /// Record `expr` as moved where it is a bare name, and nothing otherwise.
    ///
    /// Only a *name* moves. `&x` borrows, `x.field` is a partial move the
    /// caller reports separately, and a call's result is a fresh value.
    pub(super) fn moved(&self, expr: &syn::Expr, at: Where, out: &mut Vec<Site>) {
        match expr {
            syn::Expr::Path(path) => {
                if let Some(name) = local_name(path) {
                    if self.is_shadowed(&name) {
                        return;
                    }
                    out.push(Site {
                        name,
                        at,
                        span: syn::spanned::Spanned::span(path),
                    });
                }
            }
            // `(x, y)`, `[x]` and `Foo { a: x }` each take their parts by
            // value — left to right, so a part with something after it that
            // can throw is moved under that (X5).
            syn::Expr::Tuple(tuple) => {
                let elems: Vec<&syn::Expr> = tuple.elems.iter().collect();
                for (index, elem) in elems.iter().enumerate() {
                    self.moved(elem, evaluating(at, &elems, index + 1), out);
                }
            }
            syn::Expr::Array(array) => {
                let elems: Vec<&syn::Expr> = array.elems.iter().collect();
                for (index, elem) in elems.iter().enumerate() {
                    self.moved(elem, evaluating(at, &elems, index + 1), out);
                }
            }
            syn::Expr::Struct(s) => {
                let fields: Vec<&syn::Expr> = s.fields.iter().map(|f| &f.expr).collect();
                for (index, field) in fields.iter().enumerate() {
                    self.moved(field, evaluating(at, &fields, index + 1), out);
                }
            }
            syn::Expr::Paren(p) => self.moved(&p.expr, at, out),
            syn::Expr::Group(g) => self.moved(&g.expr, at, out),
            // Y3: `*boxed` MOVES what the box held, and the box with it —
            // Rust's deref-move, which only a `Box` has — so
            // `if let Predicate::And(l, r) = *left` takes `left`. Without this,
            // the arm that consumed `left` released it again on the way out and
            // the runtime reported it used after being moved.
            //
            // Only a `Box`. `*guard` on a `MutexGuard`, a `Ref` or an `Arc` is
            // the `Deref` trait, which BORROWS: reading through one takes
            // nothing, and counting it as a move left
            // `let g = self.inner.lock().await; *g` with the guard released by
            // nobody.
            syn::Expr::Unary(syn::ExprUnary { op: syn::UnOp::Deref(_), expr, .. })
                if self.consumes.derefs_by_value(expr) =>
            {
                self.moved(expr, at, out)
            }
            // `Some(x)`, `Foo::Bar(x)` and every other call take their
            // arguments by value; the argument walk records those.
            _ => {}
        }
    }

    /// Every local this `move` closure names, which is everything it captured.
    pub fn captures(&self, closure: &syn::ExprClosure) -> Vec<Site> {
        let mut params = Vec::new();
        for input in &closure.inputs {
            collect_pattern_names(input, &mut params);
        }
        self.shadowed.borrow_mut().push(params);
        let mut out = Vec::new();
        self.mentions(&closure.body, &mut out);
        self.shadowed.borrow_mut().pop();
        out
    }

    /// Every local this closure's body hands away.
    ///
    /// A closure written without `move` still captures by value whatever its
    /// body moves — Rust infers the mode per capture — so a non-`move` closure
    /// that hands a value to a callee owns it exactly as a `move` one does. It
    /// is also what says a capture is *consumed* by the body rather than merely
    /// held, which is the difference between an `FnOnce` and an `Fn`.
    pub fn moved_captures(&self, closure: &syn::ExprClosure) -> Vec<Site> {
        let mut params = Vec::new();
        for input in &closure.inputs {
            collect_pattern_names(input, &mut params);
        }
        self.shadowed.borrow_mut().push(params);
        let mut out = Vec::new();
        self.walk(&closure.body, Where::Closure, &mut out);
        self.tail(&closure.body, Where::Closure, &mut out);
        self.shadowed.borrow_mut().pop();
        out
    }

    /// Every local a `move` closure names, which is everything it captured.
    fn mentions(&self, body: &syn::Expr, out: &mut Vec<Site>) {
        struct Names<'o> {
            out: &'o mut Vec<Site>,
            shadowed: &'o [Vec<String>],
        }
        impl syn::visit::Visit<'_> for Names<'_> {
            fn visit_expr_path(&mut self, path: &syn::ExprPath) {
                if let Some(name) = local_name(path) {
                    if self.shadowed.iter().any(|frame| frame.iter().any(|n| *n == name)) {
                        return;
                    }
                    self.out.push(Site {
                        name,
                        at: Where::Closure,
                        span: syn::spanned::Spanned::span(path),
                    });
                }
            }
        }
        let shadowed = self.shadowed.borrow();
        syn::visit::Visit::visit_expr(
            &mut Names {
                out,
                shadowed: &shadowed,
            },
            body,
        );
    }
}

/// Everything under a branch, a loop or an arm is one step further from
/// straight-line, and stays there.
/// Where a move stands when something the call still has to EVALUATE can leave
/// the frame first.
///
/// X5: `take2(token, o.unwrap())` moves `token` on every path the source has,
/// so the disposition was `Moved` and the block wrote no release at all — and
/// `unwrap` on a `None` throws with `token` handed to nobody, which Rust drops
/// while it unwinds. The same holds for a `?` standing in a later field of the
/// struct literal that moves the value: ankql's
/// `Predicate::Comparison { left: Box::new(left.populate_recursive(values)?),
/// operator, .. }` left `operator` unreleased on the error path.
///
/// So a move with anything after it that can throw is a move under a BRANCH:
/// the block declares a flag, `lifted_above_the_flag` lifts those later
/// operands above it, and the flag stands immediately before the call.
/// `rest` is what is still to be evaluated after this position.
pub(super) fn evaluating(at: Where, rest: &[&syn::Expr], from: usize) -> Where {
    let can_throw = rest.iter().skip(from).any(|e| !crate::body::flags::evaluates_quietly(e));
    match (at, can_throw) {
        (Where::Straight, true) => Where::Evaluated,
        _ => at,
    }
}

pub(super) fn nested(at: Where) -> Where {
    match at {
        Where::Straight | Where::Branch | Where::Evaluated => Where::Branch,
        other => other,
    }
}

/// The local a path names, or nothing where the path is not a plain name.
pub(crate) fn local_name(path: &syn::ExprPath) -> Option<String> {
    if path.qself.is_some() || path.path.segments.len() != 1 {
        return None;
    }
    let ident = path.path.segments[0].ident.to_string();
    // A method that takes `self` by value owns the receiver like any other
    // by-value parameter, and `self` in a value position hands it on. It is
    // written `this`, which is the name the release is emitted against.
    if ident == "self" {
        return Some("this".to_string());
    }
    if matches!(ident.as_str(), "Self" | "None" | "true" | "false") {
        return None;
    }
    // A unit enum variant and a constant are written like a local and are not
    // one. Both are capitalised by convention, which the extractor already
    // relies on everywhere it reads a path.
    if ident.starts_with(|c: char| c.is_uppercase()) {
        return None;
    }
    Some(crate::name_map::to_camel_case(&ident))
}


/// Can this statement leave the block it stands in, before the statements below
/// it run?
///
/// A `return` and a `?` leave the function; a `break` and a `continue` leave the
/// enclosing loop, which is only this block when the loop is not inside the
/// statement itself. A closure's `return` leaves the closure and is not one.
fn leaves_the_block(stmt: &syn::Stmt) -> bool {
    struct Exits {
        found: bool,
    }
    impl syn::visit::Visit<'_> for Exits {
        fn visit_expr(&mut self, expr: &syn::Expr) {
            match expr {
                syn::Expr::Return(_) | syn::Expr::Try(_) => {
                    self.found = true;
                }
                syn::Expr::Break(_) | syn::Expr::Continue(_) => {
                    self.found = true;
                }
                // A loop written here catches its own `break` and `continue`;
                // only a `return` or a `?` inside it reaches past this block.
                syn::Expr::ForLoop(_) | syn::Expr::While(_) | syn::Expr::Loop(_) => {
                    let mut inner = Returns { found: false };
                    syn::visit::visit_expr(&mut inner, expr);
                    self.found |= inner.found;
                    return;
                }
                // A closure's own exits belong to the closure.
                syn::Expr::Closure(_) => return,
                _ => {}
            }
            syn::visit::visit_expr(self, expr);
        }
    }
    struct Returns {
        found: bool,
    }
    impl syn::visit::Visit<'_> for Returns {
        fn visit_expr(&mut self, expr: &syn::Expr) {
            match expr {
                syn::Expr::Return(_) | syn::Expr::Try(_) => self.found = true,
                syn::Expr::Closure(_) => return,
                _ => {}
            }
            syn::visit::visit_expr(self, expr);
        }
    }
    let mut exits = Exits { found: false };
    syn::visit::Visit::visit_stmt(&mut exits, stmt);
    exits.found
}
