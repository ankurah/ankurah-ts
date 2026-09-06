//! Expression type resolution — the type of any `syn::Expr` the translator asks about.
//!
//! The Rust source declares every type; this walks the AST and looks each one
//! up through the registry, the impl table and the scope stack. What it cannot
//! answer it refuses, with a diagnostic naming the position, and the translator
//! decides whether it has a fallback for that site.

use syn::spanned::Spanned;

use super::expected;
use super::scope::ScopeStack;
use crate::diag::{Diag, DiagSink};
use crate::name_map;
use crate::registry::{
    resolve_type, Def, ModuleId, Ns, Probe, TypeEnv, TypeRegistry,
};
use crate::ty::subst::Subst;
use crate::ty::{unify, Prim, TraitRef, Ty};

pub struct TypeContext<'a> {
    pub registry: &'a TypeRegistry,
    pub scopes: ScopeStack,
    /// The module whose imports and declarations names resolve through.
    pub module: ModuleId,
    /// Generic parameters in scope, so `T` in an annotation is a parameter.
    pub params: Vec<String>,
    /// What each of those parameters is known to implement. A call on `T`, and
    /// a call on `self` inside a trait's own default body, dispatch through
    /// these and nothing else.
    pub param_bounds: Vec<(String, TraitRef)>,
    /// What `Self` means in the enclosing impl.
    pub self_ty: Option<Ty>,
    /// The parameters of the closure whose body is being typed right now,
    /// innermost frame last.
    ///
    /// Asking a closure's tail for its type needs the closure's parameters
    /// visible, and `resolve_expr` takes `&self` because it answers questions
    /// rather than translating. This is that one scope, opened for the length
    /// of the question and closed after it.
    closure_params: std::cell::RefCell<Vec<Vec<(String, Option<Ty>)>>>,
    pub sink: &'a DiagSink,
}

impl<'a> TypeContext<'a> {
    pub fn new(
        registry: &'a TypeRegistry,
        module: ModuleId,
        self_ty: Option<Ty>,
        params: Vec<String>,
        sink: &'a DiagSink,
    ) -> Self {
        let mut scopes = ScopeStack::new();
        // The module frame holds the file's constants.
        scopes.push_module();
        if let Some(ty) = &self_ty {
            scopes.push_impl(ty.clone());
        }
        TypeContext {
            registry,
            scopes,
            module,
            params,
            param_bounds: Vec::new(),
            self_ty,
            closure_params: std::cell::RefCell::new(Vec::new()),
            sink,
        }
    }

    /// Ask something with a closure's parameters in scope, then take them back
    /// out again whatever the answer was.
    pub(super) fn with_closure_params<T>(
        &self,
        params: &[(String, Option<Ty>)],
        ask: impl FnOnce() -> T,
    ) -> T {
        self.closure_params.borrow_mut().push(params.to_vec());
        let answer = ask();
        self.closure_params.borrow_mut().pop();
        answer
    }

    /// What a closure parameter opened by `with_closure_params` holds, and
    /// whether the name is one of them at all.
    fn closure_param(&self, name: &str) -> Option<Option<Ty>> {
        self.closure_params
            .borrow()
            .iter()
            .rev()
            .find_map(|frame| {
                frame
                    .iter()
                    .find(|(param, _)| param == name)
                    .map(|(_, ty)| ty.clone())
            })
    }

    /// The type of a block's tail expression, which is the block's own type.
    pub fn block_tail_type(&self, block: &syn::Block) -> Result<Ty, Diag> {
        self.resolve_block(block)
    }

    /// Is this the declared `Option`? Asked by identity, so a crate type that
    /// happens to be called `Option` is its own type.
    pub fn is_option(&self, ty: &Ty) -> bool {
        ty.peel_refs()
            .id()
            .is_some_and(|id| self.is_system(id, "std::option::Option"))
    }

    /// What a `Result` or an `Option` carries, or the type itself where it
    /// carries nothing.
    ///
    /// A deserialisation writes the value; the wrapper around it belongs to the
    /// `?` or the `unwrap` that follows. So `let id: EntityId = ..` and `let
    /// id: Result<EntityId, _> = ..` both name `EntityId` as the type that
    /// reads itself out.
    pub fn unwrapped_payload(&self, ty: &Ty) -> Ty {
        self.try_payload(ty).unwrap_or_else(|| ty.clone())
    }

    /// A call's result with whatever the position wanted written into the parts
    /// the call itself left open.
    ///
    /// `s.parse()`, `x.into()` and `it.collect()` all return one of their own
    /// type parameters, and the call says nothing about which type that is —
    /// `let id: EntityId = s.parse()` says it. Where the result holds no open
    /// parameter there is nothing to close and the expectation is ignored: it
    /// is a hint, never an override of a type the source settled.
    fn close_with_expectation(&self, ret: Ty, expected: Option<&Ty>) -> Ty {
        let Some(want) = expected else { return ret };
        let open = open_params(&ret);
        if open.is_empty() && !expected::has_infer(&ret) {
            return ret;
        }
        let filled = expected::fill_infer(&ret, want);
        let mut subst = Subst::new();
        match unify(&open, &filled, want.peel_refs(), &mut subst) {
            Ok(()) => filled.substitute(&subst),
            // The two disagree somewhere. They may still agree where it
            // counts: `s.parse()` returns `Result<F, F::Err>` and a `?`
            // handing an expectation inward says what the payload is and
            // leaves the error slot a hole, so matching the two whole fails on
            // the error and loses the payload with it. Take the bindings the
            // parts that do agree produced, and where no part agrees the
            // call's own answer stands.
            Err(_) => {
                let mut subst = Subst::new();
                expected::partial_bindings(&open, &filled, want.peel_refs(), &mut subst);
                filled.substitute(&subst)
            }
        }
    }

    /// What a `?` operand has to be, given what the `?` itself has to produce.
    ///
    /// The error slot is left a hole: no position ever says what a `?` throws
    /// away, and naming one would put a type the source never wrote into the
    /// match.
    pub fn try_operand_expectation(&self, expected: Option<&Ty>) -> Option<Ty> {
        let want = expected?;
        let id = self.registry.system_type("std::result::Result")?;
        Some(Ty::Named {
            id,
            args: vec![want.clone(), Ty::Infer],
        })
    }

    /// The callable type a closure has: `impl Fn(A, B) -> R` with what the
    /// signature settled, which is the type Rust gives a closure everywhere a
    /// closure's type is asked for.
    ///
    /// A parameter or a result the engine could not settle is refused rather
    /// than filled with a guess, because a wrong `Fn` bound would pick a wrong
    /// impl at every call that takes this closure.
    fn callable_type(
        &self,
        closure: &syn::ExprClosure,
        sig: &super::closures::ClosureSig,
    ) -> Result<Ty, Diag> {
        let untyped = sig.untyped_params();
        if !untyped.is_empty() {
            return Err(self.refuse(
                closure.span(),
                format!(
                    "this closure's parameter{} {} typed by nothing the engine can read: \
                     neither an annotation on the closure nor the position it stands in says \
                     what {} hold{}",
                    if untyped.len() == 1 { " is" } else { "s are" },
                    untyped
                        .iter()
                        .map(|n| format!("`{}`", n))
                        .collect::<Vec<_>>()
                        .join(", "),
                    if untyped.len() == 1 { "it" } else { "they" },
                    if untyped.len() == 1 { "s" } else { "" },
                ),
            ));
        }
        let Some(id) = self.registry.system_type("std::ops::Fn") else {
            return Err(self.refuse(closure.span(), "`Fn` is not declared"));
        };
        let inputs: Vec<Ty> = sig.params.iter().filter_map(|(_, ty)| ty.clone()).collect();
        Ok(Ty::ImplTrait {
            bounds: vec![TraitRef {
                id,
                args: vec![if inputs.is_empty() {
                    Ty::Unit
                } else {
                    Ty::Tuple(inputs)
                }],
                bindings: vec![(
                    "Output".to_string(),
                    sig.ret.clone().unwrap_or(Ty::Unit),
                )],
            }],
        })
    }

    /// The impl table, asked from the module that wrote the call and with the
    /// bounds this body's parameters carry.
    pub fn probe(&self) -> Probe<'_> {
        Probe::new(self.registry, self.module).with_bounds(&self.param_bounds)
    }

    /// The type a name has here, where the scope stack knows one.
    ///
    /// A closure parameter opened for the length of one question shadows the
    /// stack, exactly as the closure's own scope would if this were a
    /// translation rather than a question.
    pub fn lookup(&self, name: &str) -> Option<Ty> {
        match self.closure_param(name) {
            Some(ty) => ty,
            None => self.scopes.resolve(name).cloned(),
        }
    }

    pub fn bind(&mut self, name: &str, ty: Ty) {
        self.scopes.bind(name.to_string(), ty);
    }

    pub fn push_block(&mut self) {
        self.scopes.push_block();
    }

    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    pub fn push_fn(&mut self, params: Vec<(String, Ty)>) {
        self.scopes.push_fn(params);
    }

    pub fn push_closure(&mut self, params: Vec<(String, Ty)>) {
        self.scopes.push_closure(params);
    }

    /// Is this name bound in scope at all, whether or not the engine could
    /// type what it holds?
    pub fn is_bound(&self, name: &str) -> bool {
        self.closure_param(name).is_some() || self.scopes.is_bound(name)
    }

    /// Would a `let` of this name here be a redeclaration JavaScript refuses?
    pub fn redeclares(&self, name: &str) -> bool {
        self.scopes.redeclares(name)
    }

    /// The identifier a bound name is emitted under.
    pub fn emitted_name(&self, name: &str) -> Option<String> {
        self.scopes.emitted_name(name)
    }

    /// Take a fresh identifier for a shadow and emit that name under it.
    pub fn shadow(&mut self, name: &str) -> String {
        let fresh = self.scopes.fresh_name(name);
        self.scopes.rename(name, fresh.clone());
        fresh
    }

    pub fn bind_untyped(&mut self, name: &str) {
        self.scopes.bind_untyped(name.to_string());
    }

    pub(super) fn refuse(&self, span: proc_macro2::Span, message: impl Into<String>) -> Diag {
        Diag::at(&self.sink.file(), span, message)
    }

    /// Resolve a written type in this module, with the generics in scope.
    pub fn resolve_written_type(&self, ty: &syn::Type) -> Result<Ty, Diag> {
        let env = TypeEnv::new(self.registry, self.module, self.sink)
            .with_params(&self.params)
            .with_self(self.self_ty.as_ref());
        resolve_type(ty, &env)
    }

    /// The type of an expression, or the reason the engine cannot say.
    pub fn resolve_expr(&self, expr: &syn::Expr) -> Result<Ty, Diag> {
        self.resolve_expr_expecting(expr, None)
    }

    /// The same, where the position the expression stands in says what type it
    /// has to be (spec 4.6).
    ///
    /// The expectation settles what the expression alone leaves open: the width
    /// of an integer literal, the target of an `.into()` or a `.parse()`, the
    /// parameters of a closure, and whether a sequence literal is a sequence of
    /// bytes. It is a hint about this one position and travels no further than
    /// the sub-expressions that inherit it directly.
    pub fn resolve_expr_expecting(
        &self,
        expr: &syn::Expr,
        expected: Option<&Ty>,
    ) -> Result<Ty, Diag> {
        match expr {
            syn::Expr::Path(path) if path.path.is_ident("self") => self
                .scopes
                .self_type()
                .cloned()
                .ok_or_else(|| self.refuse(expr.span(), "`self` outside an impl")),

            syn::Expr::Path(path) => self.resolve_path_expr(path),

            syn::Expr::Field(field) => {
                let member = member_name(&field.member);
                self.resolve_field_access(&field.base, &member)
                    .map(|found| found.ty)
            }

            syn::Expr::MethodCall(call) => {
                let method = call.method.to_string();
                self.resolve_method_call_with(&call.receiver, &method, call.turbofish.as_ref())
                    .map(|found| self.close_with_expectation(found.ret, expected))
            }

            syn::Expr::Call(call) => self
                .resolve_call(call)
                .map(|ret| self.close_with_expectation(ret, expected)),

            syn::Expr::Struct(lit) => self.resolve_struct_literal(lit),

            syn::Expr::Reference(r) => self.resolve_expr(&r.expr),

            // `loop { .. break n; }` is an expression whose type is what its
            // `break`s carry; a `loop` with no such `break` never ends and has
            // no value. Untyped, the local it was bound to was untyped too, and
            // every operator on that local fell back.
            syn::Expr::Loop(loop_expr) => {
                let carried = break_value(&loop_expr.body);
                match carried {
                    Some(value) => self.resolve_expr_expecting(value, expected),
                    None => Ok(Ty::Never),
                }
            }

            // `-x` and `!x`. On a primitive both answer the operand's own type,
            // which is what tells the operator around them that they are
            // integer arithmetic: with `-1` unresolved, `x / -1` was written as
            // JavaScript's `/` — a float division where Rust truncates, and
            // with `i32::MIN / -1`'s panic missed. On anything else the answer
            // is the impl's `Output`.
            syn::Expr::Unary(unary)
                if matches!(unary.op, syn::UnOp::Neg(_) | syn::UnOp::Not(_)) =>
            {
                let operand = self.resolve_expr_expecting(&unary.expr, expected)?;
                if matches!(operand.peel_refs(), Ty::Prim(_)) {
                    return Ok(operand.peel_refs().clone());
                }
                let trait_path = match unary.op {
                    syn::UnOp::Neg(_) => "std::ops::Neg",
                    _ => "std::ops::Not",
                };
                self.project_through(&operand, trait_path, "Output")
                    .ok_or_else(|| {
                        self.refuse(
                            expr.span(),
                            format!(
                                "`{}` has no `{}` impl in reach",
                                self.registry.describe(&operand),
                                trait_path
                            ),
                        )
                    })
            }

            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                let inner_ty = self.resolve_expr(&unary.expr)?;
                // A `*` the port has nothing to take is one the port ALREADY
                // took: `Box<T>` is `T` here, so `&**expr` over a
                // `&Box<Expr>` has one reference to peel and no box behind it,
                // and the second `*` was refused — which left `Predicate::
                // IsNull`'s inner match untyped, so its catch-all could not be
                // expanded and `expr.match({ Path: .. })` reached the runtime's
                // fatal for every `Expr` that is not a `Path`. The erasure is
                // the port's own, made once by the type mapping, so the
                // resolution agrees with it here rather than reporting it at
                // every mention — the same reading fixpass3 §4.12 gave a const
                // generic argument. `deref_accessor_of` still refuses, because
                // `*x = y` has to name a field and a value that is its own
                // dereference has none.
                Ok(self
                    .probe()
                    .deref_once(&inner_ty)
                    .map(|step| step.to)
                    .unwrap_or(inner_ty))
            }

            // The port models an async function as returning the type it
            // writes: `#[async_trait]` is ignored and no `Future` is wrapped
            // around anything (spec 4.10). So awaiting one yields exactly what
            // the call already had.
            syn::Expr::Await(await_expr) => self.resolve_expr(&await_expr.base),

            // `e?` is `T` whether or not the error type has to be converted.
            // What the position wants of the `?` is what it wants of the
            // payload, so it is handed inward wrapped in a `Result`: that is
            // what types `let id: EntityId = s.parse()?`, whose call says
            // nothing about which type it parses.
            syn::Expr::Try(try_expr) => {
                let want = self.try_operand_expectation(expected);
                let inner = self.resolve_expr_expecting(&try_expr.expr, want.as_ref())?;
                self.try_payload(&inner).ok_or_else(|| {
                    self.refuse(
                        expr.span(),
                        format!(
                            "`?` on `{}`, which is neither a Result nor an Option",
                            self.registry.describe(&inner)
                        ),
                    )
                })
            }

            syn::Expr::Cast(cast) => self.resolve_written_type(&cast.ty),

            syn::Expr::Tuple(t) if t.elems.is_empty() => Ok(Ty::Unit),
            syn::Expr::Tuple(t) => {
                let elems = t
                    .elems
                    .iter()
                    .map(|e| self.resolve_expr(e))
                    .collect::<Result<_, _>>()?;
                Ok(Ty::Tuple(elems))
            }

            syn::Expr::Range(range) => self.range_type(range).ok_or_else(|| {
                self.refuse(expr.span(), "the range types in `std::ops` are not declared")
            }),

            syn::Expr::Paren(p) => self.resolve_expr_expecting(&p.expr, expected),
            syn::Expr::Group(g) => self.resolve_expr_expecting(&g.expr, expected),

            syn::Expr::Lit(lit) => self.literal_type(&lit.lit, expected),

            // A closure has a type once its parameters and its result do: the
            // callable Rust gives it. Saying so is what lets a `let` bind one,
            // and what a later position reads the parameter types back out of.
            syn::Expr::Closure(closure) => {
                let sig = self.closure_signature(closure, expected);
                self.callable_type(closure, &sig)
            }

            // Every element of an array literal has the sequence's element
            // type, and it is that — not the first element's own default —
            // that decides the widths written into the wire format.
            syn::Expr::Array(array) => {
                let elem_want = expected.and_then(|ty| expected::element_of(self.registry, ty));
                let elem = match array.elems.first() {
                    Some(first) => self.resolve_expr_expecting(first, elem_want.as_ref())?,
                    None => elem_want.clone().ok_or_else(|| {
                        self.refuse(
                            expr.span(),
                            "an empty array literal has no element type, and the position it \
                             stands in does not say one",
                        )
                    })?,
                };
                Ok(Ty::Array {
                    elem: Box::new(elem),
                    len: crate::ty::ArrayLen::Lit(array.elems.len() as u64),
                })
            }

            syn::Expr::Binary(bin) => self.binary_type(bin, expected),

            syn::Expr::Index(idx) => {
                let base = self.resolve_expr(&idx.expr)?;
                self.index_result(&base, &idx.index).ok_or_else(|| {
                    self.refuse(
                        expr.span(),
                        format!(
                            "no `Index` impl reaches `{}` with the index written here",
                            self.registry.describe(&base)
                        ),
                    )
                })
            }

            syn::Expr::Repeat(repeat) => {
                let elem_want = expected.and_then(|ty| expected::element_of(self.registry, ty));
                Ok(Ty::Array {
                    elem: Box::new(self.resolve_expr_expecting(&repeat.expr, elem_want.as_ref())?),
                    len: crate::ty::ArrayLen::Named("_".to_string()),
                })
            }

            // Every arm of a `match` and both branches of an `if` have the same
            // type in Rust, so the first one that is not a divergence answers
            // for all of them.
            syn::Expr::Match(m) => m
                .arms
                .iter()
                .find_map(|arm| {
                    self.resolve_expr_expecting(&arm.body, expected)
                        .ok()
                        .filter(|t| *t != Ty::Never)
                })
                .ok_or_else(|| {
                    self.refuse(expr.span(), "no arm of this match has a type the engine could read")
                }),

            syn::Expr::If(if_expr) => {
                let then = self.resolve_block_expecting(&if_expr.then_branch, expected);
                if let Ok(ty) = &then {
                    if *ty != Ty::Never {
                        return then;
                    }
                }
                match &if_expr.else_branch {
                    Some((_, other)) => self.resolve_expr_expecting(other, expected),
                    // An `if` with no `else` is the unit type.
                    None => Ok(Ty::Unit),
                }
            }

            syn::Expr::Macro(mac) => self.macro_type(&mac.mac, expected),

            syn::Expr::Block(b) => match b.block.stmts.last() {
                Some(syn::Stmt::Expr(tail, None)) => self.resolve_expr_expecting(tail, expected),
                _ => Err(self.refuse(
                    expr.span(),
                    "block has no tail expression to take a type from",
                )),
            },

            other => Err(self.refuse(
                other.span(),
                format!("`{}` expressions are not typed yet", expr_form(other)),
            )),
        }
    }

    fn resolve_block(&self, block: &syn::Block) -> Result<Ty, Diag> {
        self.resolve_block_expecting(block, None)
    }

    fn resolve_block_expecting(
        &self,
        block: &syn::Block,
        expected: Option<&Ty>,
    ) -> Result<Ty, Diag> {
        match block.stmts.last() {
            Some(syn::Stmt::Expr(tail, None)) => self.resolve_expr_expecting(tail, expected),
            _ => Ok(Ty::Unit),
        }
    }

    /// A literal's type. An integer or float written without a suffix takes the
    /// width the position wants, and Rust's own default — `i32` and `f64` —
    /// only where the position wants nothing.
    fn literal_type(&self, lit: &syn::Lit, expected: Option<&Ty>) -> Result<Ty, Diag> {
        Ok(match lit {
            syn::Lit::Str(_) => Ty::Ref {
                mutable: false,
                inner: Box::new(Ty::Str),
            },
            syn::Lit::ByteStr(_) => Ty::Ref {
                mutable: false,
                inner: Box::new(Ty::Slice(Box::new(Ty::Prim(Prim::U8)))),
            },
            syn::Lit::Byte(_) => Ty::Prim(Prim::U8),
            syn::Lit::Char(_) => Ty::Prim(Prim::Char),
            syn::Lit::Bool(_) => Ty::Prim(Prim::Bool),
            syn::Lit::Int(int) => match Prim::from_rust_name(int.suffix()) {
                Some(prim) => Ty::Prim(prim),
                None => Ty::Prim(
                    expected
                        .and_then(expected::integer_width)
                        .unwrap_or(Prim::I32),
                ),
            },
            syn::Lit::Float(float) => match Prim::from_rust_name(float.suffix()) {
                Some(prim) => Ty::Prim(prim),
                None => {
                    Ty::Prim(expected.and_then(expected::float_width).unwrap_or(Prim::F64))
                }
            },
            other => {
                return Err(self.refuse(
                    syn::spanned::Spanned::span(other),
                    "literal form is not typed yet",
                ))
            }
        })
    }

    /// A binary operator's result. Comparison and logical operators are `bool`
    /// whatever they are applied to; arithmetic on primitives is the primitive.
    /// An operator on anything else resolves through its trait's `Output`, which
    /// is the operators step.
    fn binary_type(&self, bin: &syn::ExprBinary, expected: Option<&Ty>) -> Result<Ty, Diag> {
        use syn::BinOp::*;
        match bin.op {
            Eq(_) | Ne(_) | Lt(_) | Le(_) | Gt(_) | Ge(_) | And(_) | Or(_) => {
                return Ok(Ty::Prim(Prim::Bool))
            }
            AddAssign(_) | SubAssign(_) | MulAssign(_) | DivAssign(_) | RemAssign(_)
            | BitXorAssign(_) | BitAndAssign(_) | BitOrAssign(_) | ShlAssign(_)
            | ShrAssign(_) => return Ok(Ty::Unit),
            _ => {}
        }
        // An unsuffixed integer literal takes the type of whatever it is written
        // against — `n + 1` where `n: usize` is `usize` arithmetic, not a type
        // mismatch between `usize` and the literal's default `i32`.
        // A shift is the exception: its right operand has a type of its own —
        // `1u64 << 63i32` is a `u64` — so reading the whole expression off the
        // shift amount answered `i32` for what the position said was 64-bit.
        if !matches!(bin.op, Shl(_) | Shr(_)) {
            if let Some(ty) = self.literal_against(&bin.left, &bin.right)? {
                return Ok(ty);
            }
        }
        // Where both operands are literals the whole expression takes its type
        // from the position, which is how `bits ^ (1 << 63)` beside a `u64` is
        // 64-bit arithmetic rather than the literal's default `i32`.
        let left = self.resolve_expr_expecting(&bin.left, expected)?;
        let Ty::Prim(prim) = left.peel_refs() else {
            // An operator between ported types is a call, and its impl says
            // what the call answers.
            if let Some(output) = self.overloaded_result(bin, &left) {
                return Ok(output);
            }
            return Err(self.refuse(
                syn::spanned::Spanned::span(bin),
                format!(
                    "`{}` overloads this operator; which impl it takes is the operators step",
                    self.registry.describe(&left)
                ),
            ));
        };
        // A shift's right operand has its own type; every other arithmetic
        // operator on primitives takes two of the same and gives that back.
        if matches!(bin.op, Shl(_) | Shr(_)) {
            return Ok(Ty::Prim(*prim));
        }
        let right = self.resolve_expr(&bin.right)?;
        if right.peel_refs() == &Ty::Prim(*prim) {
            Ok(Ty::Prim(*prim))
        } else {
            Err(self.refuse(
                syn::spanned::Spanned::span(bin),
                "the two sides of this operator are different types",
            ))
        }
    }

    /// What an overloaded operator answers: its impl's `Output`.
    ///
    /// Only an impl with no parameters of its own is read. A generic one —
    /// `impl<T> Add for Wrapper<T>` — writes its `Output` in terms of those
    /// parameters, and the impl table hands back the impl it matched without
    /// the substitution that matched it, so there is nothing here to put in
    /// their place. Asking is not translating: what the right operand could not
    /// say is reported where the operator is written.
    fn overloaded_result(&self, bin: &syn::ExprBinary, left: &Ty) -> Option<Ty> {
        let trait_path = crate::operators::operator_trait(&bin.op)?;
        let mark = self.sink.mark();
        let right = self.resolve_expr(&bin.right);
        self.sink.rewind(mark);
        // Rust's operator traits default `Rhs` to `Self`, which every operator
        // impl in the corpus takes.
        let right = right.unwrap_or_else(|_| left.clone());
        let found = self.probe().operator_impl(&trait_path, left, &right).ok()?;
        let def = self.registry.impl_def(found.impl_id);
        // A generic impl says what it answers in terms of its own parameters —
        // `impl<T> Add for Generic<T> { type Output = Generic<T>; }` — and the
        // match that selected it is what says which `T` this site has. Refusing
        // every generic impl left the local a `+` was bound to untyped, so
        // nothing released what it held.
        Some(def.assoc_types.get("Output")?.substitute(&found.args))
    }

    /// The type of an arithmetic operator where one side is an unsuffixed
    /// integer literal: the other side's, since that is what Rust infers.
    fn literal_against(
        &self,
        left: &syn::Expr,
        right: &syn::Expr,
    ) -> Result<Option<Ty>, Diag> {
        // Not only a bare literal: `bits ^ (1 << 63)` writes an operand built
        // entirely out of unsuffixed literals, and Rust gives the whole of it
        // the other side's type. Reading `1 << 63` as the literal's default
        // `i32` made it a `number` beside a `bigint`, which JavaScript refuses
        // to combine at all.
        fn unsuffixed(e: &syn::Expr) -> bool {
            match e {
                syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(i), .. }) => {
                    i.suffix().is_empty()
                }
                syn::Expr::Paren(p) => unsuffixed(&p.expr),
                syn::Expr::Group(g) => unsuffixed(&g.expr),
                syn::Expr::Unary(u) => {
                    matches!(u.op, syn::UnOp::Neg(_) | syn::UnOp::Not(_)) && unsuffixed(&u.expr)
                }
                // A shift's right operand has a type of its own, so only the
                // left one carries the whole expression's.
                syn::Expr::Binary(b) if matches!(b.op, syn::BinOp::Shl(_) | syn::BinOp::Shr(_)) => {
                    unsuffixed(&b.left)
                }
                syn::Expr::Binary(b) => unsuffixed(&b.left) && unsuffixed(&b.right),
                _ => false,
            }
        }
        let other = if unsuffixed(left) {
            right
        } else if unsuffixed(right) {
            left
        } else {
            return Ok(None);
        };
        let ty = self.resolve_expr(other)?;
        Ok(match ty.peel_refs() {
            Ty::Prim(prim) if prim.is_integer() => Some(Ty::Prim(*prim)),
            _ => None,
        })
    }

    /// What `base[index]` hands back: `<base as Index<I>>::Output`, where `I` is
    /// the type of the index expression.
    ///
    /// The std surface declares the whole family — `Index<I> for Vec<T>` and
    /// `for [T]` through `SliceIndex`, `Index<&Q> for HashMap<K, V>` through
    /// `Borrow` — so which of them applies, and whether the answer is an element
    /// or a slice, follows from the index's type rather than from a list of
    /// container names.
    #[cfg(test)]
    pub fn index_of(&self, base: &Ty, index_src: &str) -> Option<Ty> {
        let index: syn::Expr = syn::parse_str(index_src).expect("parses as an expression");
        self.index_result(base, &index)
    }

    fn index_result(&self, base: &Ty, index: &syn::Expr) -> Option<Ty> {
        let index_ty = self.index_type(index)?;
        let trait_id = self.registry.system_type("std::ops::Index")?;
        // `HashMap` is indexed by a borrowed key, and the deref chain is what
        // walks a `Vec` down to the `[T]` its `Index` is written for.
        for candidate in std::iter::once(base.clone())
            .chain(self.probe().deref_chain(base).ok()?.into_iter().map(|s| s.to))
        {
            let found = self.project_with(
                &candidate,
                TraitRef {
                    id: trait_id,
                    args: vec![index_ty.clone()],
                    bindings: Vec::new(),
                },
                "Output",
            );
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// The type of the expression between the brackets.
    ///
    /// An unsuffixed integer literal is read as `usize` here. That is not a
    /// guess about inference: `SliceIndex` is implemented for `usize` and for
    /// ranges of `usize` and for nothing else numeric, so `usize` is the only
    /// type such a literal can have in this position. Where the index is
    /// anything else the ordinary rules answer, and a `HashMap` indexed by
    /// `&key` goes through `Borrow` like any other lookup.
    fn index_type(&self, index: &syn::Expr) -> Option<Ty> {
        if is_unsuffixed_int(index) {
            return Some(Ty::Prim(Prim::Usize));
        }
        if let syn::Expr::Range(range) = index {
            return self.range_type(range);
        }
        self.resolve_expr(index).ok()
    }

    /// `a..b` is a `Range<A>`, `a..` a `RangeFrom<A>`, `..b` a `RangeTo<A>`,
    /// `..` a `RangeFull` and `a..=b` a `RangeInclusive<A>`, each declared in
    /// `std::ops`.
    fn range_type(&self, range: &syn::ExprRange) -> Option<Ty> {
        let closed = matches!(range.limits, syn::RangeLimits::Closed(_));
        let end_ty = |e: &Option<Box<syn::Expr>>| e.as_deref().and_then(|e| self.index_type(e));
        let (path, arg) = match (&range.start, &range.end) {
            (Some(start), Some(_)) if closed => ("std::ops::RangeInclusive", self.index_type(start)),
            (Some(start), Some(_)) => ("std::ops::Range", self.index_type(start)),
            (Some(start), None) => ("std::ops::RangeFrom", self.index_type(start)),
            (None, Some(_)) if closed => ("std::ops::RangeToInclusive", end_ty(&range.end)),
            (None, Some(_)) => ("std::ops::RangeTo", end_ty(&range.end)),
            (None, None) => ("std::ops::RangeFull", None),
        };
        let id = self.registry.system_type(path)?;
        Some(Ty::Named {
            id,
            args: arg.into_iter().collect(),
        })
    }

    /// What a macro invocation produces (spec 4.10). The transpiler never
    /// expands one, so each supported macro's type is stated here and every
    /// other macro is refused at the invocation.
    fn macro_type(&self, mac: &syn::Macro, expected: Option<&Ty>) -> Result<Ty, Diag> {
        let span = syn::spanned::Spanned::span(mac);
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        match name.as_str() {
            // `vec![..]` is a `Vec` of whatever its elements are, and its
            // elements are whatever the position wants: `vec![1, 2]` behind a
            // `Vec<u8>` holds bytes, not the `i32`s a bare literal defaults to.
            "vec" => {
                let elem_want = expected.and_then(|ty| expected::element_of(self.registry, ty));
                let id = self.registry.system_type("std::vec::Vec").ok_or_else(|| {
                    self.refuse(span, "`vec!` yields a Vec, which is not declared")
                })?;
                let elems = crate::macros::vec_macro_elements(mac);
                let elem = match (elems.first(), elem_want) {
                    (Some(first), want) => self.resolve_expr_expecting(first, want.as_ref())?,
                    (None, Some(want)) => want,
                    (None, None) => {
                        return Err(self.refuse(
                            span,
                            "an empty `vec![]` has no element type, and the position it stands \
                             in does not say one",
                        ))
                    }
                };
                Ok(Ty::Named {
                    id,
                    args: vec![elem],
                })
            }
            "format" => self
                .registry
                .system_type("std::string::String")
                .map(|id| Ty::Named { id, args: Vec::new() })
                .ok_or_else(|| self.refuse(span, "`format!` yields a String, which is not declared")),
            "panic" | "todo" | "unimplemented" | "unreachable" => Ok(Ty::Never),
            "assert" | "assert_eq" | "assert_ne" | "debug_assert" | "debug_assert_eq"
            | "debug_assert_ne" => Ok(Ty::Unit),
            "matches" => Ok(Ty::Prim(Prim::Bool)),
            "trace" | "debug" | "info" | "warn" | "error" => Ok(Ty::Unit),
            other => Err(self.refuse(
                span,
                format!("`{}!` has no declared type", other),
            )),
        }
    }

    /// What `?` hands back: a `Result`'s first argument or an `Option`'s only
    /// one. The conversion the error takes on the way out is the conversions
    /// step's business, and does not change this type.
    fn try_payload(&self, ty: &Ty) -> Option<Ty> {
        let Ty::Named { id, args } = ty.peel_refs() else {
            return None;
        };
        if self.is_system(*id, "std::result::Result") || self.is_system(*id, "std::option::Option") {
            return args.first().cloned();
        }
        None
    }

    /// Is this id the type declared at that std path? Asked by identity, so a
    /// crate type called `Result` is its own type and `std::fmt::Result` — an
    /// alias for something else entirely — is not this one either.
    pub(super) fn is_system(&self, id: crate::ty::TypeId, path: &str) -> bool {
        self.registry.system_type(path) == Some(id)
    }



    pub(super) fn resolve_struct_literal(&self, lit: &syn::ExprStruct) -> Result<Ty, Diag> {
        let ty = syn::Type::Path(syn::TypePath {
            qself: lit.qself.clone(),
            path: lit.path.clone(),
        });
        // A struct literal may also name an enum variant: `Signal::Memo { .. }`.
        let segments: Vec<String> = lit
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        if let Some((id, _)) = self.registry.lookup_variant(self.module, &segments) {
            let params = self
                .registry
                .def(id)
                .map(|d| d.type_params.clone())
                .unwrap_or_default();
            return Ok(Ty::Named {
                id,
                args: params.into_iter().map(Ty::Param).collect(),
            });
        }

        let resolved = self.resolve_written_type(&ty)?;
        let Ty::Named { id, args } = &resolved else {
            return Ok(resolved);
        };
        let Some(def) = self.registry.def(*id) else {
            return Ok(resolved);
        };
        // A literal written without arguments — `Foo { .. }` for `Foo<T>` —
        // takes them from the fields it was given.
        if !args.is_empty() || def.type_params.is_empty() {
            return Ok(resolved);
        }
        let params = def.type_params.clone();
        let fields = def.fields.clone();
        let mut subst = Subst::new();
        for field in &lit.fields {
            let name = member_name(&field.member);
            let Some((_, declared)) = fields.iter().find(|(n, _)| *n == name) else {
                continue;
            };
            if let Ok(actual) = self.resolve_expr(&field.expr) {
                let _ = unify(&params, declared, &actual, &mut subst);
            }
        }
        Ok(Ty::Named {
            id: *id,
            args: params
                .iter()
                .map(|p| subst.get(p).cloned().unwrap_or(Ty::Param(p.clone())))
                .collect(),
        })
    }

    /// A path in expression position: a local, a parameter, or a constant
    /// reached through the module's own scope.
    fn resolve_path_expr(&self, path: &syn::ExprPath) -> Result<Ty, Diag> {
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let written = segments.join("::");

        if segments.len() == 1 {
            let ident = &segments[0];
            // Locals and parameters are bound under their TypeScript name;
            // constants keep their Rust SCREAMING_CASE.
            let camel = name_map::to_camel_case(ident);
            if let Some(ty) = self.lookup(&camel).or_else(|| self.lookup(ident)) {
                return Ok(ty);
            }
            // A name that is bound but has no type is a different failure from
            // a name nothing binds, and says where to look.
            if self.is_bound(&camel) || self.is_bound(ident) {
                return Err(self.refuse(
                    path.span(),
                    format!("`{}` is bound here but the engine could not type it", written),
                ));
            }
        }

        // A primitive's associated constant has that primitive's type, which is
        // what makes a method call on it reach the number translations at all
        // (`ty::prim_consts`).
        if let Some(prim) = crate::ty::prim_consts::type_of_path(&segments) {
            return Ok(Ty::Prim(prim));
        }

        // A unit enum variant written as a path is a value of its enum.
        if let Some((id, _)) = self.registry.lookup_variant(self.module, &segments) {
            let params = self
                .registry
                .def(id)
                .map(|d| d.type_params.clone())
                .unwrap_or_default();
            return Ok(Ty::Named {
                id,
                args: params.into_iter().map(Ty::Param).collect(),
            });
        }

        match self.registry.lookup(self.module, Ns::Value, &segments) {
            Ok(Some(Def::Value(id))) => match self.registry.value(id).and_then(|v| v.ty.clone()) {
                Some(ty) => Ok(ty),
                None => Err(self.refuse(
                    path.span(),
                    format!("`{}` has no type the engine could read", written),
                )),
            },
            Err(err) => Err(self.refuse(path.span(), err.message)),
            _ => Err(self.refuse(
                path.span(),
                format!("`{}` does not name a value here", written),
            )),
        }
    }

    /// The type of a `let` binding: its annotation if it has one, otherwise
    /// the type of what initialises it.
    pub fn resolve_local_type(&self, local: &syn::Local) -> Result<Ty, Diag> {
        let annotated = self.local_annotation(local);
        match (annotated, &local.init) {
            // A `let x: Vec<_> = ..` says most of the type and leaves a hole
            // for the initialiser to close.
            (Some(written), Some(init)) if expected::has_infer(&written) => {
                let filled = self.resolve_expr_expecting(&init.expr, Some(&written))?;
                Ok(expected::fill_infer(&written, &filled))
            }
            (Some(written), _) => Ok(written),
            (None, Some(init)) => self.resolve_expr(&init.expr),
            (None, None) => Err(self.refuse(
                local.span(),
                "binding has neither a type nor an initialiser",
            )),
        }
    }

    /// The type a `let` writes for itself, which is what its initialiser is
    /// expected to produce.
    pub fn local_annotation(&self, local: &syn::Local) -> Option<Ty> {
        match &local.pat {
            syn::Pat::Type(pat_type) => self.resolve_written_type(&pat_type.ty).ok(),
            _ => None,
        }
    }

    // ── Queries the body translator asks before emitting ──────────────

    /// The accessor `*expr` reaches through, or why the engine cannot say.
    /// `*x = y` has to write something; this is what decides whether it writes
    /// the accessor the type declares or the one the translator assumes.
    pub fn deref_accessor_of(&self, expr: &syn::Expr) -> Result<String, Diag> {
        let ty = self.resolve_expr(expr)?;
        match self.probe().deref_once(&ty) {
            Some(step) => step
                .accessor
                .as_ref()
                .and_then(|a| a.field())
                .map(|f| f.to_string())
                .ok_or_else(|| {
                    self.refuse(
                        expr.span(),
                        format!(
                            "`{}` dereferences without a field to assign through",
                            self.registry.describe(&ty)
                        ),
                    )
                }),
            None => Err(self.refuse(
                expr.span(),
                format!("`{}` does not dereference", self.registry.describe(&ty)),
            )),
        }
    }

    /// Is `Type::Variant` an enum variant, as opposed to an associated
    /// function? The enum is resolved through its own path, never by the last
    /// segment of it.
    pub fn is_variant(&self, type_path: &str, variant: &str) -> bool {
        let mut segments: Vec<String> = type_path
            .split('.')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        if segments.is_empty() {
            return false;
        }
        segments.push(variant.to_string());
        if self.registry.lookup_variant(self.module, &segments).is_some() {
            return true;
        }
        // The emitted name is the LEAF, because the port flattens a crate's
        // module tree into a package's exports: `ast::Literal::I64` is written
        // `Literal.I64` and imported from `./ast`. A module that says only
        // `use crate::ast;` has no `Literal` in scope, so asking from there
        // answers no — and the call was then written as an associated function
        // of a class, not as the variant it is. The crate root is where the
        // flattened surface lives, so it is asked second.
        let root = self.registry.crate_root_of(self.module);
        self.registry
            .modules()
            .ids()
            .filter(|m| self.registry.modules().is_within(*m, root))
            .any(|m| m != self.module && self.registry.lookup_variant(m, &segments).is_some())
    }

    /// The enum and variant a path names, where it names a *unit* variant of an
    /// enum this crate emits a class for.
    ///
    /// A unit variant in expression position is a value that has to be built —
    /// `new ParseError('Empty', {})` — exactly as a payload-carrying one is.
    /// Writing it as a member of the class instead named a static nothing
    /// declares, which reads `undefined` and compares unequal to every variant
    /// the same file constructs properly.
    pub fn unit_variant_of_emitted_enum(&self, segments: &[String]) -> Option<(String, String)> {
        let (id, variant) = self.registry.lookup_variant(self.module, segments)?;
        let ty = Ty::Named {
            id,
            args: Vec::new(),
        };
        if !crate::emit_impls::has_emitted_class(self.registry, &ty) {
            return None;
        }
        let def = self.registry.def(id)?;
        let crate::registry::TypeKind::Enum { variants } = &def.kind else {
            return None;
        };
        let found = variants.iter().find(|v| v.name == variant)?;
        if !found.fields.is_empty() {
            return None;
        }
        Some((self.registry.name_of(id), variant))
    }

    /// The enum and variant a path names, where the path names a variant of an
    /// enum this crate emits a class for — whether or not it carries fields.
    ///
    /// `unit_variant_of_emitted_enum` answers only for a variant with no
    /// payload, because a path in expression position is a VALUE only then. A
    /// struct-variant LITERAL — `Predicate::Comparison { left, .. }` — names a
    /// variant that does carry fields and is built the same way.
    pub fn variant_of_emitted_enum(&self, segments: &[String]) -> Option<(String, String)> {
        // `Self::Add { .. }` inside `impl WatcherChange` names the same variant
        // `WatcherChange::Add` does. `Self` is not a name the registry holds,
        // so the path was looked up, found nothing, and fell through to the
        // struct-literal writing, which emitted `new WatcherChange.Add(..)` —
        // not a constructor.
        let (id, variant) = match (segments.first().map(String::as_str), self.self_ty.as_ref()) {
            (Some("Self"), Some(Ty::Named { id, .. })) if segments.len() == 2 => {
                let variant = segments[1].clone();
                if !self.registry.is_variant_of(*id, &variant) {
                    return None;
                }
                (*id, variant)
            }
            _ => self.registry.lookup_variant(self.module, segments)?,
        };
        let ty = Ty::Named { id, args: Vec::new() };
        if !crate::emit_impls::has_emitted_class(self.registry, &ty) {
            return None;
        }
        Some((self.registry.name_of(id), variant))
    }

    /// Is this the `Result` the transpiler emits a real `unwrap` for?
    ///
    /// A `LockResult` is not, even though it is a `Result`: the port's
    /// `Mutex::lock` and `RwLock::read` hand back the guard itself rather than a
    /// `Result` around one, so the `unwrap` the Rust source writes has nothing
    /// left to do by the time the TypeScript runs. That is an emission fact
    /// about the runtime, not a claim about Rust — the engine typed the call as
    /// `Result::unwrap` on a `LockResult<Guard>` and got the guard, which is
    /// exactly what Rust says.
    pub fn is_result(&self, ty: &Ty) -> bool {
        // By identity, not by leaf name: a crate type called `Result` is its own
        // type and does not have the runtime `Result`'s `unwrap`.
        let ty = ty.peel_refs();
        ty.id()
            .is_some_and(|id| self.is_system(id, "std::result::Result"))
            && !self.is_lock_result(ty)
    }

    /// `LockResult<G>` is `Result<G, PoisonError<G>>`, and its alias is expanded
    /// by the time the engine sees it, so it is recognised by that error type.
    pub fn is_lock_result(&self, ty: &Ty) -> bool {
        let Ty::Named { args, .. } = ty.peel_refs() else {
            return false;
        };
        args.get(1)
            .and_then(|e| e.id())
            .is_some_and(|id| self.is_system(id, "std::sync::PoisonError"))
    }

    /// The std functions whose port version hands back the guard itself.
    ///
    /// Rust wraps a guard in a `LockResult` because another thread can poison
    /// a lock; there are no other threads here, so `@ankurah/base` returns the
    /// guard and the `unwrap` the source writes has nothing left to do. This is
    /// a fact about *these functions*, not about the type: keying it on
    /// `Result<_, PoisonError<_>>` also swallowed the `unwrap` on any other
    /// `Result` that happened to carry a `PoisonError`.
    const LOCK_CALLS: [(&'static str, &'static str); 8] = [
        ("std::sync::Mutex", "lock"),
        ("std::sync::Mutex", "try_lock"),
        ("std::sync::RwLock", "read"),
        ("std::sync::RwLock", "write"),
        ("std::sync::RwLock", "try_read"),
        ("std::sync::RwLock", "try_write"),
        ("std::sync::Mutex", "get_mut"),
        ("std::sync::RwLock", "get_mut"),
    ];

    /// The free functions the port writes as something that is not the
    /// `Result` Rust returns.
    ///
    /// `bincode::serialize` becomes a `BincodeWriter` whose `finish()` hands
    /// back the bytes; `bincode::deserialize` becomes `T.decode(reader)`, which
    /// hands back the `T`; `serde_json::to_string` becomes `JSON.stringify`,
    /// which hands back the string. In each case there is no `Result` for the
    /// `unwrap` the source writes to take apart. `serde_json::from_str` is
    /// deliberately absent: the port's `T.fromJson` does return a `Result`,
    /// because a malformed value is a real failure there.
    /// Calls the port writes as the VALUE Rust wraps in a `Result`, so that the
    /// `unwrap` written after one has nothing left to unwrap.
    ///
    /// `serde_json::to_string` was here while it was emitted as
    /// `JSON.stringify(x)`. It answers a `Result` now, exactly as Rust does, so
    /// the `unwrap` after it is a real one — and dropping it was what left
    /// `test_event_id_json_serialization` comparing a `Result` to a string.
    const VALUE_NOT_RESULT: [&'static str; 2] =
        ["bincode::serialize", "bincode::deserialize"];

    /// Did this expression come from a call the port writes as the value
    /// itself, so that the `unwrap` written after it has nothing to unwrap?
    pub fn is_lock_call(&self, expr: &syn::Expr) -> bool {
        if let syn::Expr::Call(call) = expr {
            if let syn::Expr::Path(path) = &*call.func {
                let written: Vec<String> = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                return Self::VALUE_NOT_RESULT.contains(&written.join("::").as_str());
            }
        }
        let syn::Expr::MethodCall(call) = expr else {
            return false;
        };
        let method = call.method.to_string();
        if !Self::LOCK_CALLS.iter().any(|(_, name)| *name == method) {
            return false;
        }
        // Asking is not translating: the resolution files the questions it
        // deferred, and the call is translated separately.
        let mark = self.sink.mark();
        let found =
            self.resolve_method_call_with(&call.receiver, &method, call.turbofish.as_ref());
        self.sink.rewind(mark);
        let Ok(found) = found else { return false };
        let Some(impl_id) = found.callee.impl_id() else {
            return false;
        };
        let Some(owner) = self.registry.impl_def(impl_id).self_ty.peel_refs().id() else {
            return false;
        };
        Self::LOCK_CALLS
            .iter()
            .any(|(path, name)| *name == method && self.is_system(owner, path))
    }

}

/// The type-parameter names still standing in a type.
///
/// A call whose result is one of them — `Into::into` returning its trait's `T`,
/// `FromStr::from_str` returning `Self` — has left that part of its answer to
/// the position it stands in, and these are the names the position gets to
/// bind.
pub(super) fn open_params(ty: &Ty) -> Vec<String> {
    let mut names = Vec::new();
    collect_params(ty, &mut names);
    names
}

fn collect_params(ty: &Ty, out: &mut Vec<String>) {
    match ty {
        Ty::Param(name) => {
            if !out.iter().any(|n| n == name) {
                out.push(name.clone());
            }
        }
        Ty::Named { args, .. } => args.iter().for_each(|a| collect_params(a, out)),
        Ty::Tuple(elems) => elems.iter().for_each(|e| collect_params(e, out)),
        Ty::Slice(inner) | Ty::Array { elem: inner, .. } | Ty::Ref { inner, .. } => {
            collect_params(inner, out)
        }
        Ty::Assoc { base, .. } => collect_params(base, out),
        Ty::ImplTrait { bounds } | Ty::Dyn { traits: bounds } => {
            for bound in bounds {
                bound.args.iter().for_each(|a| collect_params(a, out));
                bound.bindings.iter().for_each(|(_, t)| collect_params(t, out));
            }
        }
        _ => {}
    }
}

/// A primitive's own name, for the rare place a literal's type is asked for.
#[allow(dead_code)]
pub fn prim_name(p: Prim) -> String {
    format!("{:?}", p).to_lowercase()
}

/// The name of an expression form, so a refusal says which one it could not
/// read rather than only that it could not.
pub fn expr_form(expr: &syn::Expr) -> &'static str {
    match expr {
        syn::Expr::Array(_) => "array",
        syn::Expr::Assign(_) => "assignment",
        syn::Expr::Binary(_) => "binary operator",
        syn::Expr::Break(_) => "break",
        syn::Expr::Closure(_) => "closure",
        syn::Expr::Const(_) => "const block",
        syn::Expr::Continue(_) => "continue",
        syn::Expr::ForLoop(_) => "for loop",
        syn::Expr::If(_) => "if",
        syn::Expr::Index(_) => "index",
        syn::Expr::Let(_) => "let condition",
        syn::Expr::Lit(_) => "literal",
        syn::Expr::Loop(_) => "loop",
        syn::Expr::Macro(_) => "macro invocation",
        syn::Expr::Match(_) => "match",
        syn::Expr::Range(_) => "range",
        syn::Expr::Repeat(_) => "array repeat",
        syn::Expr::Return(_) => "return",
        syn::Expr::Unary(_) => "unary operator",
        syn::Expr::Unsafe(_) => "unsafe block",
        syn::Expr::While(_) => "while loop",
        syn::Expr::Yield(_) => "yield",
        syn::Expr::Async(_) => "async block",
        _ => "this",
    }
}

pub fn member_name(member: &syn::Member) -> String {
    match member {
        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
    }
}

/// Is this an integer literal written without a suffix?
///
/// Rust infers such a literal's type from where it stands. In index position it
/// can only be a `usize`, because that and the ranges of it are what
/// `SliceIndex` is implemented for; nothing else is being decided here.
fn is_unsuffixed_int(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(int),
            ..
        }) if int.suffix().is_empty()
    )
}

/// The first value a `break` inside this loop's body carries, which is what the
/// loop's own type is.
///
/// A `break` in a loop written INSIDE the body belongs to that loop, and a
/// closure carries its own control flow, so neither is looked into.
fn break_value(block: &syn::Block) -> Option<&syn::Expr> {
    fn in_expr(expr: &syn::Expr) -> Option<&syn::Expr> {
        match expr {
            syn::Expr::Break(brk) if brk.label.is_none() => brk.expr.as_deref(),
            syn::Expr::Loop(_)
            | syn::Expr::While(_)
            | syn::Expr::ForLoop(_)
            | syn::Expr::Closure(_)
            | syn::Expr::Async(_) => None,
            syn::Expr::Block(b) => in_block(&b.block),
            syn::Expr::Unsafe(b) => in_block(&b.block),
            syn::Expr::If(if_expr) => in_block(&if_expr.then_branch).or_else(|| {
                if_expr.else_branch.as_ref().and_then(|(_, other)| in_expr(other))
            }),
            syn::Expr::Match(m) => m.arms.iter().find_map(|arm| in_expr(&arm.body)),
            _ => None,
        }
    }
    fn in_block(block: &syn::Block) -> Option<&syn::Expr> {
        block.stmts.iter().find_map(|stmt| match stmt {
            syn::Stmt::Expr(expr, _) => in_expr(expr),
            syn::Stmt::Local(local) => local.init.as_ref().and_then(|init| in_expr(&init.expr)),
            _ => None,
        })
    }
    in_block(block)
}
