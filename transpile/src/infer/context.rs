//! Expression type resolution — the type of any `syn::Expr` the translator asks about.
//!
//! The Rust source declares every type; this walks the AST and looks each one
//! up through the registry, the impl table and the scope stack. What it cannot
//! answer it refuses, with a diagnostic naming the position, and the translator
//! decides whether it has a fallback for that site.

use syn::spanned::Spanned;

use super::scope::ScopeStack;
use crate::diag::{Diag, DiagSink};
use crate::name_map;
use crate::registry::{
    resolve_type, Def, FieldResolution, MethodResolution, ModuleId, Ns, Probe, TypeEnv,
    TypeRegistry,
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
            sink,
        }
    }

    /// The impl table, asked from the module that wrote the call and with the
    /// bounds this body's parameters carry.
    pub fn probe(&self) -> Probe<'_> {
        Probe::new(self.registry, self.module).with_bounds(&self.param_bounds)
    }

    /// The type a name has here, where the scope stack knows one.
    pub fn lookup(&self, name: &str) -> Option<Ty> {
        self.scopes.resolve(name).cloned()
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
        self.scopes.is_bound(name)
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

    fn refuse(&self, span: proc_macro2::Span, message: impl Into<String>) -> Diag {
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
                    .map(|found| found.ret)
            }

            syn::Expr::Call(call) => self.resolve_call(call),

            syn::Expr::Struct(lit) => self.resolve_struct_literal(lit),

            syn::Expr::Reference(r) => self.resolve_expr(&r.expr),

            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                let inner_ty = self.resolve_expr(&unary.expr)?;
                self.probe()
                    .deref_once(&inner_ty)
                    .map(|step| step.to)
                    .ok_or_else(|| {
                        self.refuse(
                            expr.span(),
                            format!(
                                "`{}` does not dereference",
                                self.registry.describe(&inner_ty)
                            ),
                        )
                    })
            }

            // The port models an async function as returning the type it
            // writes: `#[async_trait]` is ignored and no `Future` is wrapped
            // around anything (spec 4.10). So awaiting one yields exactly what
            // the call already had.
            syn::Expr::Await(await_expr) => self.resolve_expr(&await_expr.base),

            // `e?` is `T` whether or not the error type has to be converted;
            // which `From` performs the conversion is the conversions step.
            syn::Expr::Try(try_expr) => {
                let inner = self.resolve_expr(&try_expr.expr)?;
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

            syn::Expr::Paren(p) => self.resolve_expr(&p.expr),
            syn::Expr::Group(g) => self.resolve_expr(&g.expr),

            syn::Expr::Lit(lit) => self.literal_type(&lit.lit),

            syn::Expr::Binary(bin) => self.binary_type(bin),

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

            syn::Expr::Repeat(repeat) => Ok(Ty::Array {
                elem: Box::new(self.resolve_expr(&repeat.expr)?),
                len: crate::ty::ArrayLen::Named("_".to_string()),
            }),

            // Every arm of a `match` and both branches of an `if` have the same
            // type in Rust, so the first one that is not a divergence answers
            // for all of them.
            syn::Expr::Match(m) => m
                .arms
                .iter()
                .find_map(|arm| self.resolve_expr(&arm.body).ok().filter(|t| *t != Ty::Never))
                .ok_or_else(|| {
                    self.refuse(expr.span(), "no arm of this match has a type the engine could read")
                }),

            syn::Expr::If(if_expr) => {
                let then = self.resolve_block(&if_expr.then_branch);
                if let Ok(ty) = &then {
                    if *ty != Ty::Never {
                        return then;
                    }
                }
                match &if_expr.else_branch {
                    Some((_, other)) => self.resolve_expr(other),
                    // An `if` with no `else` is the unit type.
                    None => Ok(Ty::Unit),
                }
            }

            syn::Expr::Macro(mac) => self.macro_type(&mac.mac),

            syn::Expr::Block(b) => match b.block.stmts.last() {
                Some(syn::Stmt::Expr(tail, None)) => self.resolve_expr(tail),
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
        match block.stmts.last() {
            Some(syn::Stmt::Expr(tail, None)) => self.resolve_expr(tail),
            _ => Ok(Ty::Unit),
        }
    }

    /// A literal's type. An integer or float written without a suffix takes
    /// Rust's own default — `i32` and `f64` — which is what rustc gives it when
    /// nothing else constrains it.
    fn literal_type(&self, lit: &syn::Lit) -> Result<Ty, Diag> {
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
                None => Ty::Prim(Prim::I32),
            },
            syn::Lit::Float(float) => match Prim::from_rust_name(float.suffix()) {
                Some(prim) => Ty::Prim(prim),
                None => Ty::Prim(Prim::F64),
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
    fn binary_type(&self, bin: &syn::ExprBinary) -> Result<Ty, Diag> {
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
        if let Some(ty) = self.literal_against(&bin.left, &bin.right)? {
            return Ok(ty);
        }
        let left = self.resolve_expr(&bin.left)?;
        let Ty::Prim(prim) = left.peel_refs() else {
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

    /// The type of an arithmetic operator where one side is an unsuffixed
    /// integer literal: the other side's, since that is what Rust infers.
    fn literal_against(
        &self,
        left: &syn::Expr,
        right: &syn::Expr,
    ) -> Result<Option<Ty>, Diag> {
        let unsuffixed = |e: &syn::Expr| {
            matches!(e, syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(i), .. })
                if i.suffix().is_empty())
        };
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
    fn macro_type(&self, mac: &syn::Macro) -> Result<Ty, Diag> {
        let span = syn::spanned::Spanned::span(mac);
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        match name.as_str() {
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

    // ── Calls ──────────────────────────────────────────────────────────

    /// Which function `receiver.name(..)` calls, and what it hands back.

    /// The same, with the type arguments a turbofish wrote. `collect::<Vec<_>>()`
    /// says what the call produces where nothing else does.
    pub fn resolve_method_call_with(
        &self,
        receiver: &syn::Expr,
        method: &str,
        turbofish: Option<&syn::AngleBracketedGenericArguments>,
    ) -> Result<MethodResolution, Diag> {
        let receiver_ty = self.resolve_expr(receiver)?;
        let probe = self.probe();

        let mut explicit = Vec::new();
        for arg in turbofish.iter().flat_map(|t| t.args.iter()) {
            match arg {
                syn::GenericArgument::Type(ty) => explicit.push(self.resolve_written_type(ty)?),
                syn::GenericArgument::Lifetime(_) => {}
                other => {
                    return Err(self.refuse(
                        syn::spanned::Spanned::span(other),
                        "only type arguments are read from a turbofish",
                    ))
                }
            }
        }

        let found = probe
            .resolve_method_with(&receiver_ty, method, &explicit)
            .map_err(|err| self.refuse(receiver.span(), err.describe(self.registry, method)))?;

        // A resolution that rests on a question nobody answered is still an
        // answer the translator uses, so the question is filed where the call is.
        // Step 4 discharges these (spec 4.5); until it does they are part of the
        // measure of what the engine cannot yet decide.
        for obligation in &found.obligations {
            self.sink.report(
                receiver.span(),
                format!(
                    "obligation deferred: `{}: {}` ({})",
                    self.registry.describe(&obligation.subject),
                    self.registry.name_of(obligation.bound.id),
                    match obligation.reason {
                        crate::registry::Undecided::NoDeclaration =>
                            "the trait has no declaration here",
                        crate::registry::Undecided::OpenSubject =>
                            "the subject is still a type parameter",
                        crate::registry::Undecided::DepthLimit => "the search ran too deep",
                    }
                ),
            );
        }
        if let Some(trait_id) = found.out_of_scope {
            self.sink.report(
                receiver.span(),
                format!(
                    "method `{}` resolved through trait `{}`, which is not in scope here",
                    method,
                    self.registry.name_of(trait_id)
                ),
            );
        }
        Ok(found)
    }

    /// Where a field lives, and what has to be written to reach it.
    pub fn resolve_field_access(
        &self,
        base: &syn::Expr,
        member: &str,
    ) -> Result<FieldResolution, Diag> {
        let base_ty = self.resolve_expr(base)?;
        self.probe().resolve_field(&base_ty, member).ok_or_else(|| {
            self.refuse(
                base.span(),
                format!(
                    "no field `{}` on `{}`",
                    member,
                    self.registry.describe(&base_ty)
                ),
            )
        })
    }

    /// A `Type::function(..)`, an enum variant carrying a payload, or a plain
    /// function call.
    fn resolve_call(&self, call: &syn::ExprCall) -> Result<Ty, Diag> {
        let syn::Expr::Path(path) = &*call.func else {
            return Err(self.refuse(
                call.func.span(),
                "the callee is not a path, so nothing names what it calls",
            ));
        };
        let span = path.span();
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();

        // `Signal::Constant(v)` builds the enum, with whatever its payload says
        // about the enum's own parameters.
        if let Some((id, variant)) = self.registry.lookup_variant(self.module, &segments) {
            return Ok(self.variant_type(id, &variant, &call.args));
        }

        // `Self::new(..)`, `EntityId::from_bytes(..)`: the type named by
        // everything but the last segment, and an associated function on it.
        if segments.len() >= 2 {
            let name = segments.last().cloned().unwrap_or_default();
            if let Some(ty) = self.type_of_prefix(path) {
                if let Some(ret) = self.assoc_fn_return(&ty, &name) {
                    return Ok(ret);
                }
                return Err(self.refuse(
                    span,
                    format!(
                        "no associated function `{}` on `{}`",
                        name,
                        self.registry.describe(&ty)
                    ),
                ));
            }
        }

        // A free function declared in reach.
        match self.registry.lookup(self.module, Ns::Value, &segments) {
            Ok(Some(Def::Value(id))) => match self.registry.value(id).and_then(|v| v.ty.clone()) {
                Some(ty) => Ok(ty),
                None => Err(self.refuse(
                    span,
                    format!("`{}` has no return type the engine could read", segments.join("::")),
                )),
            },
            Err(err) => Err(self.refuse(span, err.message)),
            _ => Err(self.refuse(
                span,
                format!("`{}` does not name a function here", segments.join("::")),
            )),
        }
    }

    /// The enum a variant belongs to, with its parameters bound from whatever
    /// the payload was given. A parameter the payload says nothing about is
    /// left standing as a parameter, which is the truth about it.
    fn variant_type(
        &self,
        id: crate::ty::TypeId,
        variant: &str,
        args: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
    ) -> Ty {
        let params = self
            .registry
            .def(id)
            .map(|d| d.type_params.clone())
            .unwrap_or_default();
        let mut subst = Subst::new();
        if let Some(fields) = self.registry.variant_fields(id, variant) {
            for ((_, declared), arg) in fields.iter().zip(args) {
                if let Ok(actual) = self.resolve_expr(arg) {
                    let _ = unify(&params, declared, &actual, &mut subst);
                }
            }
        }
        Ty::Named {
            id,
            args: params
                .iter()
                .map(|p| subst.get(p).cloned().unwrap_or(Ty::Param(p.clone())))
                .collect(),
        }
    }

    /// The type everything but a path's last segment names.
    fn type_of_prefix(&self, path: &syn::ExprPath) -> Option<Ty> {
        let mut prefix = path.path.clone();
        prefix.segments.pop();
        // `pop` leaves the trailing separator behind, which syn will print.
        while prefix.segments.trailing_punct() {
            let last = prefix.segments.pop()?.into_value();
            prefix.segments.push_value(last);
        }
        if prefix.segments.is_empty() {
            return None;
        }
        let ty = syn::Type::Path(syn::TypePath {
            qself: path.qself.clone(),
            path: prefix,
        });
        self.resolve_written_type(&ty).ok()
    }

    /// What an associated function on this type returns. Inherent impls first,
    /// then trait impls; two answers is no answer.
    fn assoc_fn_return(&self, ty: &Ty, name: &str) -> Option<Ty> {
        let probe = self.probe();
        let mut inherent: Option<Ty> = None;
        let mut from_trait: Option<Ty> = None;
        let mut trait_count = 0;
        for id in self.registry.impls_for(ty) {
            let def = self.registry.impl_def(id);
            let Some(sig) = def.methods.get(name) else {
                continue;
            };
            if !sig.is_static() {
                continue;
            }
            let Some(subst) = def.match_self(ty) else {
                continue;
            };
            let ret = probe.normalize(&sig.ret.substitute(&subst));
            if def.is_inherent() {
                if inherent.is_some() {
                    return None;
                }
                inherent = Some(ret);
            } else {
                trait_count += 1;
                from_trait = Some(ret);
            }
        }
        inherent.or(if trait_count == 1 { from_trait } else { None })
    }

    /// `Foo { a, b }` builds a `Foo`, with its parameters read off the fields
    /// it was given.
    fn resolve_struct_literal(&self, lit: &syn::ExprStruct) -> Result<Ty, Diag> {
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
            if let Some(ty) = self
                .scopes
                .resolve(&camel)
                .or_else(|| self.scopes.resolve(ident))
            {
                return Ok(ty.clone());
            }
            // A name that is bound but has no type is a different failure from
            // a name nothing binds, and says where to look.
            if self.scopes.is_bound(&camel) || self.scopes.is_bound(ident) {
                return Err(self.refuse(
                    path.span(),
                    format!("`{}` is bound here but the engine could not type it", written),
                ));
            }
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
        if let syn::Pat::Type(pat_type) = &local.pat {
            return self.resolve_written_type(&pat_type.ty);
        }
        match &local.init {
            Some(init) => self.resolve_expr(&init.expr),
            None => Err(self.refuse(
                local.span(),
                "binding has neither a type nor an initialiser",
            )),
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
        self.registry
            .lookup_variant(self.module, &segments)
            .is_some()
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

    /// Did this expression come from one of those calls?
    pub fn is_lock_call(&self, expr: &syn::Expr) -> bool {
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

    /// The types a callee's closure parameter takes.
    ///
    /// Step 2 still only knows `LocalKey<T>::with` — what `thread_local!`
    /// declares, and what the port calls `ThreadLocal`; typing a closure from
    /// the callee's `Fn` bound is the closures step, which reads the bound off
    /// the resolved signature the impl table now supplies.
    pub fn resolve_closure_param_types(
        &self,
        receiver_expr: &syn::Expr,
        method: &str,
        closure: &syn::ExprClosure,
    ) -> Vec<(String, Ty)> {
        let mut result = Vec::new();
        let Ok(receiver_ty) = self.resolve_expr(receiver_expr) else {
            return result;
        };
        let Ty::Named { id, args } = receiver_ty.peel_refs() else {
            return result;
        };
        if self.is_system(*id, "std::thread::LocalKey")
            && method == "with"
            && !args.is_empty()
        {
            if let Some(param) = closure.inputs.first() {
                let param_name = crate::body::BodyTranslator::pat_static(param);
                result.push((param_name, args[0].clone()));
            }
        }
        result
    }
}




/// A primitive's own name, for the rare place a literal's type is asked for.
#[allow(dead_code)]
pub fn prim_name(p: Prim) -> String {
    format!("{:?}", p).to_lowercase()
}

/// The name of an expression form, so a refusal says which one it could not
/// read rather than only that it could not.
fn expr_form(expr: &syn::Expr) -> &'static str {
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

pub(super) fn member_name(member: &syn::Member) -> String {
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
