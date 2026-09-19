//! Which statements above this one can leave the block before it runs.
//!
//! A move written straight-line is on every path only while nothing above it
//! can leave: a `return`, a `break`, a `continue`, a `?` and anything that can
//! throw all end the block early, and a move below one of those is conditional.
//! Read as unconditional, the block wrote no release at all, and the early exit
//! left the value with nothing to release it.

use super::{Site, Where};

/// What a run of statements has learned about the ones above the current
/// statement: where the block still is, which statements can throw, and where
/// each name was bound.
///
/// The binding's position matters as much as the throw's. `let a = Owned::new(1);
/// return take(a);` throws in the statement that BINDS `a`, and on that path
/// there is no `a` to release: called conditional it grew a flag and a `finally`
/// that guard nothing.
pub(super) struct Throws {
    pub(super) reachable: Where,
    /// The statements above this one that can throw, by index.
    throwing: Vec<usize>,
    /// Where each name this block binds was bound. A name that is not here was
    /// bound above the block — a parameter, or an outer local — and is owned
    /// before any statement of it runs.
    bound: std::collections::HashMap<String, usize>,
}

impl Throws {
    pub(super) fn new(at: Where) -> Self {
        Throws { reachable: at, throwing: Vec::new(), bound: std::collections::HashMap::new() }
    }

    /// The site as it stands, or under a BRANCH because something above it that
    /// the block had already given a value to can throw.
    pub(super) fn conditional(&self, index: usize, site: Site) -> Site {
        if site.at != Where::Straight {
            return site;
        }
        let owned_before = |throw: &usize| match self.bound.get(&site.name) {
            Some(bound) => bound < throw,
            None => true,
        };
        match self.throwing.iter().any(|k| *k < index && owned_before(k)) {
            true => Site { at: Where::Branch, ..site },
            false => site,
        }
    }

    /// What this statement leaves behind for the ones below it.
    pub(super) fn after(&mut self, index: usize, stmt: &syn::Stmt) {
        if let syn::Stmt::Local(local) = stmt {
            for name in crate::body::pattern_names(&local.pat) {
                self.bound.insert(name, index);
            }
        }
        if throws(stmt) {
            self.throwing.push(index);
        }
        if self.reachable == Where::Straight && exits_the_block(stmt) {
            self.reachable = Where::Branch;
        }
    }
}

/// Can EVALUATING this statement throw, so that the statements below it do not
/// run?
///
/// Asked of what the statement evaluates in Rust — the same `evaluates_quietly`
/// the within-a-statement rule asks of an operand — because the two halves have
/// to agree about what "can throw" means. A `let` with no initialiser, an item,
/// and a statement built only out of names and literals are quiet.
fn throws(stmt: &syn::Stmt) -> bool {
    let expr = match stmt {
        syn::Stmt::Expr(expr, _) => expr,
        syn::Stmt::Local(local) => match local.init.as_ref() {
            Some(init) => &init.expr,
            None => return false,
        },
        syn::Stmt::Item(_) | syn::Stmt::Macro(_) => return false,
    };
    !crate::body::flags::evaluates_quietly(expr)
}

/// Can this statement leave the block it stands in, before the statements below
/// it run?
///
/// A `return` and a `?` leave the function; a `break` and a `continue` leave the
/// enclosing loop, which is only this block when the loop is not inside the
/// statement itself. A closure's `return` leaves the closure and is not one.
fn exits_the_block(stmt: &syn::Stmt) -> bool {
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
