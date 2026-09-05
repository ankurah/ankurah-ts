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

use std::collections::HashMap;

use syn::parse::Parser;

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
enum Where {
    /// A statement of the declaring block itself, outside any branch: the value
    /// is gone by the time the block ends, on every path.
    Straight,
    /// Inside a nested block — a branch, a loop body, a match arm. The block
    /// that holds it emits the flag as one of its own statements.
    Branch,
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
    at: Where,
    pub span: proc_macro2::Span,
}


/// Reads the one thing the syntax cannot say: whether a method call takes its
/// receiver by value. `Result::unwrap`, `Option::take` and the `into_*` family
/// all do, and the impl table knows which.
pub trait Consumes {
    fn consumes_receiver(&self, call: &syn::ExprMethodCall) -> bool;

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

    /// Does the impl behind a unary operator take its operand by value?
    /// `impl Neg for Weight` is `fn neg(self) -> Weight`, so `-weight` releases
    /// the weight exactly as `a + b` releases both of its operands.
    fn consumes_unary_operand(&self, unary: &syn::ExprUnary) -> bool;
}

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
    fn nested_block(&self, stmts: &[syn::Stmt], at: Where, out: &mut Vec<Site>) {
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
    fn statements(&self, stmts: &[syn::Stmt], at: Where, out: &mut Vec<Site>) {
        let mut reachable = at;
        for stmt in stmts {
            self.stmt(stmt, reachable, out);
            if reachable == Where::Straight && leaves_the_block(stmt) {
                reachable = Where::Branch;
            }
        }
    }

    /// Is this name one a nested scope declared for itself?
    fn is_shadowed(&self, name: &str) -> bool {
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
        out.retain(|s| s.at == Where::Straight || s.at == Where::Closure);
        out
    }

    fn stmt(&self, stmt: &syn::Stmt, at: Where, out: &mut Vec<Site>) {
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
    fn tail(&self, expr: &syn::Expr, at: Where, out: &mut Vec<Site>) {
        self.moved(expr, at, out);
        self.walk(expr, at, out);
    }

    /// Record `expr` as moved where it is a bare name, and nothing otherwise.
    ///
    /// Only a *name* moves. `&x` borrows, `x.field` is a partial move the
    /// caller reports separately, and a call's result is a fresh value.
    fn moved(&self, expr: &syn::Expr, at: Where, out: &mut Vec<Site>) {
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
            // `(x, y)`, `[x]` and `Foo { a: x }` each take their parts by value.
            syn::Expr::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.moved(elem, at, out);
                }
            }
            syn::Expr::Array(array) => {
                for elem in &array.elems {
                    self.moved(elem, at, out);
                }
            }
            syn::Expr::Struct(s) => {
                for field in &s.fields {
                    self.moved(&field.expr, at, out);
                }
            }
            syn::Expr::Paren(p) => self.moved(&p.expr, at, out),
            syn::Expr::Group(g) => self.moved(&g.expr, at, out),
            // `Some(x)`, `Foo::Bar(x)` and every other call take their
            // arguments by value; the argument walk records those.
            _ => {}
        }
    }

    /// Walk an expression for moves, in the positions that take a value.
    fn walk(&self, expr: &syn::Expr, at: Where, out: &mut Vec<Site>) {
        match expr {
            syn::Expr::Call(call) => {
                if self.consumes.consumes_callee(call) {
                    self.moved(&call.func, at, out);
                }
                for arg in &call.args {
                    self.moved(arg, at, out);
                    self.walk(arg, at, out);
                }
                self.walk(&call.func, at, out);
            }

            syn::Expr::MethodCall(call) => {
                // A method taking `self` consumes the receiver: `r.unwrap()`,
                // `v.into_iter()`, `opt.take()`.
                if self.consumes.consumes_receiver(call) {
                    self.moved(&call.receiver, at, out);
                }
                self.walk(&call.receiver, at, out);
                for arg in &call.args {
                    self.moved(arg, at, out);
                    self.walk(arg, at, out);
                }
            }

            // Awaiting a named future takes it by value, exactly as a
            // self-taking method does, and the emitter releases nothing after.
            syn::Expr::Await(await_expr) => {
                self.moved(&await_expr.base, at, out);
                self.walk(&await_expr.base, at, out);
            }

            syn::Expr::Assign(assign) => {
                self.moved(&assign.right, at, out);
                self.walk(&assign.right, at, out);
                self.walk(&assign.left, at, out);
            }

            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.moved(value, at, out);
                    self.walk(value, at, out);
                }
            }

            syn::Expr::Break(brk) => {
                if let Some(value) = &brk.expr {
                    self.moved(value, at, out);
                    self.walk(value, at, out);
                }
            }

            syn::Expr::Struct(s) => {
                for field in &s.fields {
                    self.moved(&field.expr, at, out);
                    self.walk(&field.expr, at, out);
                }
                if let Some(rest) = &s.rest {
                    self.walk(rest, at, out);
                }
            }

            syn::Expr::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.moved(elem, at, out);
                    self.walk(elem, at, out);
                }
            }

            syn::Expr::Array(array) => {
                for elem in &array.elems {
                    self.moved(elem, at, out);
                    self.walk(elem, at, out);
                }
            }

            // A closure that moves a value out of its environment captured it
            // by value, `move` written or not — Rust has no other way to
            // compile the body. A `move` closure claims everything it mentions.
            syn::Expr::Closure(closure) => {
                let mut params = Vec::new();
                for input in &closure.inputs {
                    collect_pattern_names(input, &mut params);
                }
                self.shadowed.borrow_mut().push(params);
                if closure.capture.is_some() {
                    self.mentions(&closure.body, out);
                }
                self.walk(&closure.body, Where::Closure, out);
                self.tail(&closure.body, Where::Closure, out);
                self.shadowed.borrow_mut().pop();
            }

            // The branches of a `?:` and the operands of `&&`/`||` have nowhere
            // to put a statement. A move there is reported rather than flagged.
            syn::Expr::Binary(bin) => {
                let short_circuit =
                    matches!(bin.op, syn::BinOp::And(_) | syn::BinOp::Or(_));
                let operand = if short_circuit { Where::Unwritable } else { at };
                // An overloaded operator is a method call whose impl takes its
                // operands by value, and the call releases them.
                let (left, right) = self.consumes.consumes_operands(bin);
                if left {
                    self.moved(&bin.left, at, out);
                }
                if right {
                    self.moved(&bin.right, operand, out);
                }
                self.walk(&bin.left, at, out);
                self.walk(&bin.right, operand, out);
            }

            syn::Expr::Unary(unary) => {
                if self.consumes.consumes_unary_operand(unary) {
                    self.moved(&unary.expr, at, out);
                }
                self.walk(&unary.expr, at, out);
            }

            syn::Expr::If(if_expr) => {
                self.walk(&if_expr.cond, at, out);
                self.nested_block(&if_expr.then_branch.stmts, at, out);
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.walk(else_expr, nested(at), out);
                    self.tail(else_expr, nested(at), out);
                }
            }

            syn::Expr::Match(m) => {
                if self.consumes.consumes_scrutinee(m) {
                    self.moved(&m.expr, at, out);
                }
                self.walk(&m.expr, at, out);
                for arm in &m.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.walk(guard, Where::Unwritable, out);
                    }
                    // `other => ..` takes the whole subject into its own name,
                    // and only where that arm runs: the block keeps a flag and
                    // releases the subject on the paths the other arms took.
                    if crate::ownership::scrutinee::binds_whole_subject(&arm.pat) {
                        self.moved(&m.expr, nested(at), out);
                    }
                    self.walk(&arm.body, nested(at), out);
                    self.tail(&arm.body, nested(at), out);
                }
            }

            syn::Expr::Block(block) => self.nested_block(&block.block.stmts, at, out),

            syn::Expr::ForLoop(for_loop) => {
                self.moved(&for_loop.expr, at, out);
                self.walk(&for_loop.expr, at, out);
                self.nested_block(&for_loop.body.stmts, at, out);
            }

            syn::Expr::While(w) => {
                self.walk(&w.cond, at, out);
                self.nested_block(&w.body.stmts, at, out);
            }

            syn::Expr::Loop(l) => self.nested_block(&l.body.stmts, at, out),

            // `if let Some(payload) = value` moves out of `value` exactly as
            // a `match` arm does, and the block that owned it stops owning it.
            syn::Expr::Let(let_expr) => {
                if self.consumes.consumes_let_scrutinee(let_expr) {
                    self.moved(&let_expr.expr, at, out);
                }
                self.walk(&let_expr.expr, at, out)
            }
            syn::Expr::Async(block) => self.nested_block(&block.block.stmts, at, out),
            syn::Expr::Unsafe(block) => self.nested_block(&block.block.stmts, at, out),

            syn::Expr::Try(t) => self.walk(&t.expr, at, out),
            syn::Expr::Paren(p) => self.walk(&p.expr, at, out),
            syn::Expr::Group(g) => self.walk(&g.expr, at, out),
            syn::Expr::Reference(r) => self.walk(&r.expr, at, out),
            syn::Expr::Cast(c) => self.walk(&c.expr, at, out),
            syn::Expr::Field(f) => self.walk(&f.base, at, out),
            syn::Expr::Index(i) => {
                self.walk(&i.expr, at, out);
                self.walk(&i.index, at, out);
            }
            syn::Expr::Range(r) => {
                if let Some(from) = &r.start {
                    self.walk(from, at, out);
                }
                if let Some(to) = &r.end {
                    self.walk(to, at, out);
                }
            }
            syn::Expr::Repeat(r) => self.walk(&r.expr, at, out),

            // A macro's arguments are Rust written in this body, and the
            // supported ones each take theirs the way an ordinary call would:
            // `vec![a, b]` by value, `format!`/`assert!`/`write!` by reference.
            // Leaving them out of the scan let `Bag { items: vec![value] }`
            // release `value` here and in the `Bag` that now held it.
            syn::Expr::Macro(mac) => self.macro_args(&mac.mac, at, out),

            _ => {}
        }
    }

    /// The moves a supported macro's arguments write.
    ///
    /// An unsupported macro's tokens are not Rust the emitter reads, so nothing
    /// is claimed about them; the translation of the macro is what reports it.
    fn macro_args(&self, mac: &syn::Macro, at: Where, out: &mut Vec<Site>) {
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        // `select!` takes each arm's future by value and drops every one of
        // them when it returns, so the block that named one must not release it
        // as well.
        if name == "select" {
            for future in crate::macros::select_futures(&mac.tokens) {
                self.moved(&future, at, out);
                self.walk(&future, at, out);
            }
            return;
        }
        let by_value = match name.as_str() {
            "vec" => true,
            "format" | "println" | "eprintln" | "write" | "writeln" | "panic" | "unreachable"
            | "assert" | "debug_assert" | "assert_eq" | "assert_ne" => false,
            _ => return,
        };
        let parse = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        let Ok(args) = parse.parse2(mac.tokens.clone()) else {
            return;
        };
        for arg in &args {
            if by_value {
                self.moved(arg, at, out);
            }
            self.walk(arg, at, out);
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
fn nested(at: Where) -> Where {
    match at {
        Where::Straight | Where::Branch => Where::Branch,
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

/// What each declared local's block should do with it.
///
/// Sites are attributed to the declaration that was in scope where they were
/// written, so `let staged = ..; use(staged); let staged = ..;` reads the first
/// binding as moved and the second as kept.
#[derive(Debug, Default)]
pub struct Dispositions {
    /// Keyed by the name Rust wrote and which declaration of it this is,
    /// counting from one.
    by_declaration: HashMap<(String, usize), Disposition>,
    /// The move sites that took a value into a closure, so the capture can be
    /// reported once at the site rather than once per use.
    pub captures: Vec<Site>,
    /// The sites the emitter could not write a flag for.
    pub unwritable: Vec<Site>,
}

impl Dispositions {
    pub fn of(&self, name: &str, ordinal: usize) -> Disposition {
        self.by_declaration
            .get(&(name.to_string(), ordinal))
            .copied()
            .unwrap_or(Disposition::Kept)
    }

    /// Attribute each site to the declaration it was written under.
    ///
    /// `declarations` is, in source order, the statement index of each `let`
    /// and the names it binds. A site in statement j belongs to the last
    /// declaration of that name before j.
    pub fn build(declarations: &[(usize, Vec<String>)], sites: Vec<(usize, Site)>) -> Dispositions {
        let mut result = Dispositions::default();
        for (stmt_index, site) in sites {
            let ordinal = declarations
                .iter()
                .filter(|(at, names)| *at < stmt_index && names.iter().any(|n| *n == site.name))
                .count();
            if ordinal == 0 {
                // Not one of this block's locals: a parameter, an outer local,
                // or a name from a pattern. The block that owns it decides.
                continue;
            }
            let key = (site.name.clone(), ordinal);
            let disposition = match site.at {
                Where::Straight | Where::Closure => Disposition::Moved,
                Where::Branch => Disposition::Flagged,
                Where::Unwritable => Disposition::Unsure,
            };
            if site.at == Where::Closure {
                result.captures.push(site.clone());
            }
            if site.at == Where::Unwritable {
                result.unwritable.push(site.clone());
            }
            // A local moved on a straight-line path is gone whatever else
            // happened to it; a flag would only ask a question already
            // answered. Otherwise the strongest claim wins.
            let existing = result.by_declaration.entry(key).or_insert(Disposition::Kept);
            *existing = stronger(*existing, disposition);
        }
        result
    }
}

/// Which of two claims about one local stands. "Gone" beats "sometimes gone"
/// beats "kept", because releasing a value somebody else owns is the failure
/// this analysis exists to prevent.
fn stronger(a: Disposition, b: Disposition) -> Disposition {
    let rank = |d: Disposition| match d {
        Disposition::Kept => 0,
        Disposition::Flagged => 1,
        Disposition::Unsure => 2,
        Disposition::Moved => 3,
    };
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

/// Every name a pattern binds, in the TypeScript spelling the sites use.
fn collect_pattern_names(pat: &syn::Pat, out: &mut Vec<String>) {
    match pat {
        syn::Pat::Ident(ident) => {
            out.push(crate::name_map::to_camel_case(&ident.ident.to_string()));
            if let Some((_, sub)) = &ident.subpat {
                collect_pattern_names(sub, out);
            }
        }
        syn::Pat::Tuple(t) => t.elems.iter().for_each(|p| collect_pattern_names(p, out)),
        syn::Pat::TupleStruct(t) => t.elems.iter().for_each(|p| collect_pattern_names(p, out)),
        syn::Pat::Slice(s) => s.elems.iter().for_each(|p| collect_pattern_names(p, out)),
        syn::Pat::Struct(s) => s.fields.iter().for_each(|f| collect_pattern_names(&f.pat, out)),
        syn::Pat::Reference(r) => collect_pattern_names(&r.pat, out),
        syn::Pat::Type(t) => collect_pattern_names(&t.pat, out),
        syn::Pat::Paren(p) => collect_pattern_names(&p.pat, out),
        syn::Pat::Or(or) => or.cases.iter().for_each(|p| collect_pattern_names(p, out)),
        _ => {}
    }
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
