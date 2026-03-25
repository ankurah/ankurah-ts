//! Expression type resolution — resolves the type of any syn::Expr.
//!
//! This is the transpiler's type system. Every expression has a deterministic
//! type in Rust, and this module resolves it by walking the AST and looking up
//! types through the TypeRegistry and ScopeStack.
//!
//! The Rust source fully declares all types — this is pure resolution, not inference.

use crate::name_map;
use crate::resolve::{ResolvedType, ScopeStack, Scope, ScopeKind, TypeRegistry};

/// Expression type resolver. Combines the TypeRegistry (crate-wide type definitions)
/// with a ScopeStack (local variable bindings) to resolve the type of any expression.
pub struct TypeContext<'a> {
    pub registry: &'a TypeRegistry,
    pub scopes: ScopeStack,
    pub module: String,
}

impl<'a> TypeContext<'a> {
    pub fn new(registry: &'a TypeRegistry, module: &str, self_type: &str) -> Self {
        let mut scopes = ScopeStack::new();
        // Always push a base scope for module-level constants
        scopes.push_block();
        if !self_type.is_empty() && self_type != "Self" {
            let self_ty = crate::resolve::parse_type_string(self_type);
            scopes.push_impl(self_ty);
        }
        TypeContext {
            registry,
            scopes,
            module: module.to_string(),
        }
    }

    /// Register a variable binding in the current scope.
    pub fn bind(&mut self, name: &str, ty: ResolvedType) {
        self.scopes.bind(name.to_string(), ty);
    }

    /// Push a new block scope.
    pub fn push_block(&mut self) {
        self.scopes.push_block();
    }

    /// Pop the current scope.
    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    /// Push a function scope with parameter bindings.
    pub fn push_fn(&mut self, params: Vec<(String, ResolvedType)>) {
        self.scopes.push_fn(params);
    }

    /// Push a closure scope with parameter bindings.
    pub fn push_closure(&mut self, params: Vec<(String, ResolvedType)>) {
        let bindings = params.into_iter().collect();
        self.scopes.push(Scope {
            kind: ScopeKind::Closure,
            bindings,
        });
    }

    /// Resolve a variable name to its type.
    pub fn resolve_var(&self, name: &str) -> Option<ResolvedType> {
        self.scopes.resolve(name).cloned()
    }

    /// Resolve the type of any expression.
    ///
    /// This is the core of the type system. It handles:
    /// - Variables (local, param, constant) via scope lookup
    /// - self / this via Impl scope
    /// - Field access via TypeRegistry field resolution
    /// - Method calls via TypeRegistry method resolution
    /// - Unary deref (*x) via TypeRegistry deref_field
    /// - References (&x) as transparent
    /// - unwrap/expect as identity for non-Result types
    pub fn resolve_expr(&self, expr: &syn::Expr) -> Option<ResolvedType> {
        match expr {
            // `self` → self_type from Impl scope
            syn::Expr::Path(path) if path.path.is_ident("self") => {
                self.scopes.self_type().cloned()
            }

            // Single-ident path → variable/constant lookup
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = name_map::to_camel_case(&path.path.segments[0].ident.to_string());
                // Try scope stack first (locals, params, constants)
                if let Some(ty) = self.scopes.resolve(&name) {
                    return Some(ty.clone());
                }
                // Try the original Rust name (constants are often UPPER_SNAKE_CASE)
                let rust_name = path.path.segments[0].ident.to_string();
                self.scopes.resolve(&rust_name).cloned()
            }

            // self.field → resolve self_type, look up field
            syn::Expr::Field(field) => {
                let base_ty = self.resolve_expr(&field.base)?;
                let member = match &field.member {
                    syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                    syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                };
                // Resolve field, walking through deref if needed
                self.registry.resolve_field_in_module(&base_ty, &member, &self.module)
                    .map(|(ty, _accessor)| ty)
            }

            // receiver.method(args) → resolve receiver, look up method return type
            syn::Expr::MethodCall(call) => {
                let receiver_ty = self.resolve_expr(&call.receiver)?;
                let method = call.method.to_string();

                // unwrap/expect on non-Result → identity (type doesn't change)
                if matches!(method.as_str(), "unwrap" | "expect") {
                    if !matches!(&receiver_ty, ResolvedType::Named { name, .. } if name == "Result") {
                        return Some(receiver_ty);
                    }
                }

                // clone → same type
                if method == "clone" {
                    return Some(receiver_ty);
                }

                // Try direct method lookup
                if let Some(ret) = self.registry.resolve_method_in_module(&receiver_ty, &method, &self.module) {
                    return Some(ret);
                }

                // Try deref: if method isn't on the wrapper, resolve through deref
                if let Some(accessor) = self.registry.deref_field(&receiver_ty) {
                    if !accessor.is_empty() {
                        // Get the inner type (first generic arg)
                        if let ResolvedType::Named { args, .. } = &receiver_ty {
                            if let Some(inner) = args.first() {
                                return self.registry.resolve_method_in_module(inner, &method, &self.module);
                            }
                        }
                    }
                }

                None
            }

            // &expr → transparent (same type as inner)
            syn::Expr::Reference(r) => self.resolve_expr(&r.expr),

            // *expr → deref target type
            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                let inner_ty = self.resolve_expr(&unary.expr)?;
                if let Some(accessor) = self.registry.deref_field(&inner_ty) {
                    if !accessor.is_empty() {
                        // Inner type is first generic arg
                        if let ResolvedType::Named { args, .. } = &inner_ty {
                            return args.first().cloned();
                        }
                    }
                }
                None
            }

            // (expr) → transparent
            syn::Expr::Paren(p) => self.resolve_expr(&p.expr),

            // Block → type of last expression (if it's a tail expr)
            syn::Expr::Block(b) => {
                if let Some(syn::Stmt::Expr(expr, None)) = b.block.stmts.last() {
                    self.resolve_expr(expr)
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    /// Resolve the type of a local variable declaration.
    /// Prefers explicit type annotation, falls back to init expression.
    pub fn resolve_local_type(&self, local: &syn::Local) -> Option<ResolvedType> {
        // 1. Explicit type annotation: `let x: Type = ...`
        if let syn::Pat::Type(pat_type) = &local.pat {
            let ty_str = name_map::map_type(&pat_type.ty);
            let resolved = crate::resolve::parse_type_string(&ty_str);
            if !matches!(resolved, ResolvedType::Unknown) {
                return Some(resolved);
            }
        }
        // 2. Resolve from init expression
        if let Some(init) = &local.init {
            return self.resolve_expr(&init.expr);
        }
        None
    }

    // ── Type-aware code generation queries ────────────────────────────
    //
    // These methods answer questions body.rs needs for code emission:
    // deref insertion, method dispatch, enum variant checks.

    /// Get the deref accessor for a type (e.g., "value" for Arc, RwLockWriteGuard).
    /// Returns None if the type doesn't deref or isn't resolvable.
    pub fn deref_accessor(&self, ty: &ResolvedType) -> Option<String> {
        let accessor = self.registry.deref_field(ty)?;
        if accessor.is_empty() { None } else { Some(accessor.to_string()) }
    }

    /// Check if a method is directly on the type (not inherited through deref).
    pub fn is_own_method(&self, ty: &ResolvedType, method: &str) -> bool {
        self.registry.is_own_method(ty, method)
    }

    /// Get the inner type after dereferencing (first generic arg).
    /// E.g., Arc<Inner<T>> → Inner<T>, RwLockWriteGuard<Map<K,V>> → Map<K,V>
    pub fn deref_inner_type(&self, ty: &ResolvedType) -> Option<ResolvedType> {
        if let ResolvedType::Named { args, .. } = ty {
            args.first().cloned()
        } else {
            None
        }
    }

    /// Check if a field access on an expression needs deref insertion.
    /// Returns Some((accessor, field_type)) if deref is needed, None otherwise.
    pub fn field_deref(&self, base_expr: &syn::Expr, member: &str) -> Option<(String, ResolvedType)> {
        let base_ty = self.resolve_expr(base_expr)?;
        if let Some((field_ty, Some(accessor))) = self.registry.resolve_field_in_module(&base_ty, member, &self.module) {
            if !accessor.is_empty() {
                return Some((accessor, field_ty));
            }
        }
        None
    }

    /// Check if a method call on an expression needs deref insertion.
    /// Returns Some(accessor) if the method isn't on the wrapper type itself.
    pub fn method_deref(&self, receiver_expr: &syn::Expr, method: &str) -> Option<String> {
        let receiver_ty = self.resolve_expr(receiver_expr)?;
        if !self.registry.is_own_method(&receiver_ty, method) {
            self.deref_accessor(&receiver_ty)
        } else {
            None
        }
    }

    /// Check if a name is an enum variant (for path resolution).
    pub fn is_variant(&self, type_name: &str, variant: &str) -> bool {
        self.registry.is_variant(type_name, variant)
    }

    /// Resolve closure parameter types from the calling method's signature.
    /// Given a method call `receiver.method(|param| ...)`, resolves what type
    /// the callback parameter should be.
    ///
    /// Currently handles:
    /// - ThreadLocal<T>.with(|t| ...) → t is T
    /// - General pattern: if the method accepts Fn(T) -> R, resolve T
    pub fn resolve_closure_param_types(
        &self,
        receiver_expr: &syn::Expr,
        method: &str,
        closure: &syn::ExprClosure,
    ) -> Vec<(String, ResolvedType)> {
        let mut result = Vec::new();
        if let Some(receiver_ty) = self.resolve_expr(receiver_expr) {
            if let ResolvedType::Named { name, args } = &receiver_ty {
                // ThreadLocal<T>.with(callback) → callback receives T
                if name == "ThreadLocal" && method == "with" && !args.is_empty() {
                    if let Some(param) = closure.inputs.first() {
                        let param_name = crate::body::BodyTranslator::pat_static(param);
                        result.push((param_name, args[0].clone()));
                    }
                }
                // RefCell<T>.borrow_mut() is handled through method return type,
                // not closure params. But if we see patterns like
                // vec.iter().for_each(|item| ...) we'd add them here.
            }
        }
        result
    }
}
