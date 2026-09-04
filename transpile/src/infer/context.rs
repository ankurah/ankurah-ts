//! Expression type resolution — the type of any `syn::Expr` the translator asks about.
//!
//! The Rust source declares every type; this walks the AST and looks each one
//! up through the registry and the scope stack. What it cannot answer it
//! refuses, with a diagnostic naming the position, and the translator decides
//! whether it has a fallback for that site.

use syn::spanned::Spanned;

use super::scope::ScopeStack;
use crate::diag::{Diag, DiagSink};
use crate::name_map;
use crate::registry::{resolve_type, Def, ModuleId, Ns, TypeEnv, TypeRegistry};
use crate::ty::Ty;

pub struct TypeContext<'a> {
    pub registry: &'a TypeRegistry,
    pub scopes: ScopeStack,
    /// The module whose imports and declarations names resolve through.
    pub module: ModuleId,
    /// Generic parameters in scope, so `T` in an annotation is a parameter.
    pub params: Vec<String>,
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
            self_ty,
            sink,
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

    /// Is this name bound in some enclosing scope, whether or not its type is
    /// known? Shadowing is a question about names, not about types.
    pub fn is_bound(&self, name: &str) -> bool {
        self.scopes.is_bound(name)
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
                let base_ty = self.resolve_expr(&field.base)?;
                let member = member_name(&field.member);
                self.registry
                    .resolve_field(&base_ty, &member)
                    .map(|(ty, _accessor)| ty)
                    .ok_or_else(|| {
                        self.refuse(expr.span(), format!("no field `{}` on this type", member))
                    })
            }

            syn::Expr::MethodCall(call) => {
                let receiver_ty = self.resolve_expr(&call.receiver)?;
                let method = call.method.to_string();
                // `unwrap` and `expect` on a lock guard. Rust's `RwLock::read`
                // yields a `LockResult`, but the polyfill this port declares
                // yields the guard itself, so the `unwrap` the source writes
                // has nothing left to do and the value keeps its type. This is
                // decided before method lookup, because looking it up would
                // reach through the guard and find the `unwrap` of whatever it
                // is holding — one level too many.
                //
                // Named one by one rather than "anything with an accessor", so
                // that giving crate types deref accessors cannot widen it.
                // `RefCell::borrow` is not here: it yields the guard in Rust
                // too, so no `unwrap` is ever written on it.
                if matches!(method.as_str(), "unwrap" | "expect")
                    && self.is_lock_guard(&receiver_ty)
                {
                    return Ok(receiver_ty);
                }
                // Otherwise the declared method answers, so `Option::expect` is
                // `T` and `Arc::clone` is `Arc<T>`.
                if let Some(ret) = self.registry.resolve_method(&receiver_ty, &method) {
                    return Ok(ret);
                }
                // Not on the wrapper — look at what it wraps.
                if let Some(accessor) = self.registry.deref_field(&receiver_ty) {
                    if !accessor.is_empty() {
                        if let Some(inner) = self.registry.deref_target(&receiver_ty) {
                            if let Some(ret) = self.registry.resolve_method(&inner, &method) {
                                return Ok(ret);
                            }
                        }
                    }
                }
                // `Clone::clone` returns the receiver's own type, for every
                // impl of it. Which impl is selected is the impl table's job;
                // what it returns is not in doubt.
                if method == "clone" {
                    return Ok(receiver_ty);
                }
                Err(self.refuse(expr.span(), format!("no method `{}` on this type", method)))
            }

            syn::Expr::Reference(r) => self.resolve_expr(&r.expr),

            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                let inner_ty = self.resolve_expr(&unary.expr)?;
                match self.registry.deref_field(&inner_ty) {
                    Some(accessor) if !accessor.is_empty() => self
                        .registry
                        .deref_target(&inner_ty)
                        .ok_or_else(|| self.refuse(expr.span(), "nothing to dereference to")),
                    _ => {
                        Err(self.refuse(expr.span(), "this type is not a dereferenceable wrapper"))
                    }
                }
            }

            syn::Expr::Paren(p) => self.resolve_expr(&p.expr),

            syn::Expr::Block(b) => match b.block.stmts.last() {
                Some(syn::Stmt::Expr(tail, None)) => self.resolve_expr(tail),
                _ => Err(self.refuse(
                    expr.span(),
                    "block has no tail expression to take a type from",
                )),
            },

            other => Err(self.refuse(other.span(), "expression form is not typed yet")),
        }
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

    /// The accessor to emit to reach through a wrapper, e.g. `value` for `Arc`.
    pub fn deref_accessor(&self, ty: &Ty) -> Option<String> {
        let accessor = self.registry.deref_field(ty)?;
        if accessor.is_empty() {
            None
        } else {
            Some(accessor.to_string())
        }
    }

    /// The accessor `*expr` reaches through, or why the engine cannot say.
    /// `*x = y` has to write something; this is what decides whether it writes
    /// the accessor the type declares or the one the translator assumes.
    pub fn deref_accessor_of(&self, expr: &syn::Expr) -> Result<String, Diag> {
        let ty = self.resolve_expr(expr)?;
        self.deref_accessor(&ty).ok_or_else(|| {
            self.refuse(expr.span(), "this type declares no wrapper accessor to assign through")
        })
    }

    /// What a wrapper wraps: `Arc<Inner<T>>` to `Inner<T>`.
    pub fn deref_inner_type(&self, ty: &Ty) -> Option<Ty> {
        self.registry.deref_target(ty)
    }

    /// Does reading this field need an accessor emitted first?
    pub fn field_deref(&self, base_expr: &syn::Expr, member: &str) -> Option<(String, Ty)> {
        let base_ty = self.resolve_expr(base_expr).ok()?;
        let (field_ty, accessor) = self.registry.resolve_field(&base_ty, member)?;
        let accessor = accessor?;
        if accessor.is_empty() {
            None
        } else {
            Some((accessor, field_ty))
        }
    }

    /// Does calling this method need an accessor emitted first?
    pub fn method_deref(&self, receiver_expr: &syn::Expr, method: &str) -> Option<String> {
        let receiver_ty = self.resolve_expr(receiver_expr).ok()?;
        if self.registry.is_own_method(&receiver_ty, method) {
            return None;
        }
        self.deref_accessor(&receiver_ty)
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

    /// The guards whose Rust counterparts are reached through a `LockResult`.
    fn is_lock_guard(&self, ty: &Ty) -> bool {
        let Some(id) = ty.peel_refs().id() else { return false };
        matches!(
            self.registry.system_name(id).as_deref(),
            Some("MutexGuard" | "RwLockReadGuard" | "RwLockWriteGuard")
        )
    }

    /// Is this the `Result` the transpiler emits a real `unwrap` for?
    pub fn is_result(&self, ty: &Ty) -> bool {
        ty.peel_refs()
            .id()
            .is_some_and(|id| self.registry.name_of(id) == "Result")
    }

    /// The types a callee's closure parameter takes.
    ///
    /// Step 1 still only knows `ThreadLocal<T>::with`; typing a closure from the
    /// callee's `Fn` bound is the closures step, which needs the impl table.
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
        if self.registry.system_name(*id).as_deref() == Some("ThreadLocal")
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

fn member_name(member: &syn::Member) -> String {
    match member {
        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
    }
}
