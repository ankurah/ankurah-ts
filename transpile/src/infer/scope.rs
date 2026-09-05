//! Local bindings, innermost first.
//!
//! Module-level names are not here: those come from the registry's module tree,
//! which knows what the file imported. This stack holds what a body binds —
//! constants pulled into the module frame, `self`, parameters, `let`s and
//! closure parameters — and lets an inner binding shadow an outer one the way
//! Rust does.

use std::collections::{HashMap, HashSet};

use crate::ty::Ty;

#[derive(Debug)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

#[derive(Debug)]
pub struct Scope {
    pub kind: ScopeKind,
    pub bindings: HashMap<String, Ty>,
    /// Names this scope emits under a different identifier than the source
    /// wrote. A Rust shadow is a new variable, and JavaScript cannot declare one
    /// twice in a scope, so the second one is emitted under a fresh name and
    /// every later use of it follows.
    pub renames: HashMap<String, String>,
    /// Names bound here whose type the engine could not read. They still
    /// shadow, because shadowing is about names; a diagnostic was filed where
    /// the type was needed.
    pub untyped: HashSet<String>,
}

#[derive(Debug)]
pub enum ScopeKind {
    /// The file's own constants and statics.
    Module,
    /// An impl block: `self` is bound to the type the impl is written for.
    Impl { self_type: Ty },
    /// A function's parameters.
    Fn,
    /// A `{ }` block's `let` bindings.
    Block,
    /// A closure's parameters; captures resolve through the enclosing scopes.
    Closure,
}

impl Scope {
    fn empty(kind: ScopeKind) -> Scope {
        Scope {
            kind,
            bindings: HashMap::new(),
            renames: HashMap::new(),
            untyped: HashSet::new(),
        }
    }
}

impl ScopeStack {
    pub fn new() -> Self {
        ScopeStack { scopes: Vec::new() }
    }

    pub fn pop(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }

    pub fn push_module(&mut self) {
        self.scopes.push(Scope::empty(ScopeKind::Module));
    }

    pub fn push_block(&mut self) {
        self.scopes.push(Scope::empty(ScopeKind::Block));
    }

    /// Enter an impl block, binding `this` to the type it is written for.
    pub fn push_impl(&mut self, self_type: Ty) {
        let mut bindings = HashMap::new();
        bindings.insert("this".to_string(), self_type.clone());
        self.scopes.push(Scope {
            kind: ScopeKind::Impl { self_type },
            bindings,
            renames: HashMap::new(),
            untyped: HashSet::new(),
        });
    }

    pub fn push_fn(&mut self, params: Vec<(String, Ty)>) {
        self.scopes.push(Scope {
            kind: ScopeKind::Fn,
            bindings: params.into_iter().collect(),
            renames: HashMap::new(),
            untyped: HashSet::new(),
        });
    }

    pub fn push_closure(&mut self, params: Vec<(String, Ty)>) {
        self.scopes.push(Scope {
            kind: ScopeKind::Closure,
            bindings: params.into_iter().collect(),
            renames: HashMap::new(),
            untyped: HashSet::new(),
        });
    }

    /// The innermost binding of this name, which is the one Rust means.
    pub fn resolve(&self, name: &str) -> Option<&Ty> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.bindings.get(name))
    }

    /// The type of `self` in the nearest enclosing impl block.
    pub fn self_type(&self) -> Option<&Ty> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| match &scope.kind {
                ScopeKind::Impl { self_type } => Some(self_type),
                _ => None,
            })
    }

    /// The identifier this name is emitted under, innermost binding first.
    pub fn emitted_name(&self, name: &str) -> Option<String> {
        for scope in self.scopes.iter().rev() {
            if let Some(fresh) = scope.renames.get(name) {
                return Some(fresh.clone());
            }
            if scope.bindings.contains_key(name) || scope.untyped.contains(name) {
                return Some(name.to_string());
            }
        }
        None
    }

    /// An identifier nothing in scope is using, for a shadow that cannot be
    /// declared twice.
    pub fn fresh_name(&self, base: &str) -> String {
        let taken = |candidate: &str| {
            self.scopes.iter().any(|scope| {
                scope.bindings.contains_key(candidate)
                    || scope.untyped.contains(candidate)
                    || scope.renames.values().any(|v| v == candidate)
            })
        };
        for n in 1.. {
            let candidate = format!("{}_{}", base, n);
            if !taken(&candidate) {
                return candidate;
            }
        }
        unreachable!("there is always a free suffix")
    }

    /// Emit this name under `fresh` from here on.
    pub fn rename(&mut self, name: &str, fresh: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.renames.insert(name.to_string(), fresh);
        }
    }

    /// Is this name bound at all, typed or not? Shadowing is a question about
    /// names, not about types.
    pub fn is_bound(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope.bindings.contains_key(name) || scope.untyped.contains(name))
    }

    /// Would a `let` of this name here be a redeclaration JavaScript refuses?
    ///
    /// Every scope out to and including the enclosing function or closure
    /// counts. A Rust scope is not always an emitted brace: a match arm pushes
    /// one scope for the pattern's names and another for the arm's body block,
    /// and emission writes ONE arrow-function body for both — so
    /// `P::And(left, right) => { let left = left.norm(cols); .. }` put two
    /// `const left` in one block and the file did not parse. The scope frames
    /// alone cannot say which of them ends up as a brace, and this is the side
    /// to be wrong on: a fresh name where JavaScript would have allowed the
    /// shadow costs a rename that every later use follows, while a missed one
    /// costs a file.
    ///
    /// The function or closure's parameters count for a second reason, whatever
    /// the blocks in between: `(x) => { let x = ... }` is a syntax error even
    /// though the parameters are not written in the block.
    pub fn redeclares(&self, name: &str) -> bool {
        let holds = |scope: &Scope| scope.bindings.contains_key(name) || scope.untyped.contains(name);
        for scope in self.scopes.iter().rev() {
            if holds(scope) {
                return true;
            }
            if matches!(scope.kind, ScopeKind::Fn | ScopeKind::Closure) {
                return false;
            }
        }
        false
    }

    /// Bind a name in the innermost scope, replacing any binding of the same
    /// name there — Rust's `let x = ...; let x = ...` shadowing.
    pub fn bind(&mut self, name: String, ty: Ty) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.untyped.remove(&name);
            scope.bindings.insert(name, ty);
        }
    }

    /// Bind a name whose type the engine could not read. It shadows and it
    /// suppresses a redeclaration; it answers no type question.
    pub fn bind_untyped(&mut self, name: String) {
        if let Some(scope) = self.scopes.last_mut() {
            if !scope.bindings.contains_key(&name) {
                scope.untyped.insert(name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::Prim;

    #[test]
    fn an_inner_binding_shadows_an_outer_one_until_its_scope_ends() {
        let mut stack = ScopeStack::new();
        stack.push_fn(vec![("x".into(), Ty::Prim(Prim::U32))]);
        stack.push_block();
        stack.bind("x".into(), Ty::Str);
        assert_eq!(stack.resolve("x"), Some(&Ty::Str));
        stack.pop();
        assert_eq!(stack.resolve("x"), Some(&Ty::Prim(Prim::U32)));
    }
}
