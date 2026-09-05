//! Which emitted arrow functions have to be `async`.
//!
//! Rust's `.await` belongs to the enclosing `async fn`, and the emitter often
//! puts the code holding it inside an arrow function of its own — a match arm,
//! a block standing where a value is wanted. JavaScript's `await` belongs to
//! the *nearest* function, so such an arrow has to be `async` and its result
//! has to be awaited where it stands. Without that, TypeScript reported
//! "'await' expressions are only allowed within async functions" at 45 sites in
//! core, and every one of them stopped the file's semantic phase.
//!
//! The question is asked of the Rust, not of the emitted text: an `.await`
//! written inside a nested closure or `async` block belongs to *that* function,
//! and a text scan cannot tell the two apart.

use syn::visit::{self, Visit};

/// Does this expression `.await` something that belongs to the function it is
/// written in — rather than to a closure or `async` block inside it?
pub fn awaits(expr: &syn::Expr) -> bool {
    let mut finder = Finder { found: false };
    finder.visit_expr(expr);
    finder.found
}

/// The same question of a block.
pub fn block_awaits(block: &syn::Block) -> bool {
    let mut finder = Finder { found: false };
    finder.visit_block(block);
    finder.found
}

struct Finder {
    found: bool,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_expr_await(&mut self, node: &'ast syn::ExprAwait) {
        self.found = true;
        visit::visit_expr_await(self, node);
    }

    // A closure is its own function: an `.await` inside it is that closure's,
    // and the closure is `async` in its own right.
    fn visit_expr_closure(&mut self, _node: &'ast syn::ExprClosure) {}

    // `async { .. }` is a future, and awaiting inside it is the future's own
    // business; the block is not awaited until somebody awaits the value.
    fn visit_expr_async(&mut self, _node: &'ast syn::ExprAsync) {}

    // An item written inside a body — a nested `fn` — is not this function.
    fn visit_item(&mut self, _node: &'ast syn::Item) {}

    // A macro's arguments are Rust written in this function, and syn keeps them
    // as tokens rather than as expressions. `assert_eq!(compare(..).await, ..)`
    // is nine of core's remaining `await` sites, and a walk that stops at the
    // macro sees none of them.
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if tokens_await(&node.tokens) {
            self.found = true;
        }
    }
}

/// Does this macro's argument list `.await` something?
///
/// The arguments are parsed where they can be, so an `await` inside a closure
/// written as a macro argument still belongs to that closure. Where they cannot
/// be parsed — a macro whose body is not an expression list — the `await`
/// keyword in the token stream is the answer, which can only over-report.
fn tokens_await(tokens: &proc_macro2::TokenStream) -> bool {
    use syn::parse::Parser as _;
    let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
    if let Ok(args) = parser.parse2(tokens.clone()) {
        return args.iter().any(awaits);
    }
    has_await_token(tokens)
}

fn has_await_token(tokens: &proc_macro2::TokenStream) -> bool {
    tokens.clone().into_iter().any(|tt| match tt {
        proc_macro2::TokenTree::Ident(id) => id == "await",
        proc_macro2::TokenTree::Group(g) => has_await_token(&g.stream()),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expr(src: &str) -> syn::Expr {
        syn::parse_str(src).unwrap()
    }

    #[test]
    fn an_await_in_the_expression_counts() {
        assert!(awaits(&expr("foo().await")));
        assert!(awaits(&expr("if x { a().await } else { b }")));
        assert!(awaits(&expr(
            "match e { A(v) => v.apply(r).await, B => 0 }"
        )));
    }

    #[test]
    fn an_await_inside_a_nested_function_does_not() {
        assert!(!awaits(&expr("|x| async move { f(x).await }")));
        assert!(!awaits(&expr("spawn(async { f().await })")));
        assert!(!awaits(&expr("items.map(|i| i.load().await)")));
    }

    #[test]
    fn a_macro_argument_counts() {
        // Nine of core's sites are `assert_eq!(f().await, ..)` in a test block.
        assert!(awaits(&expr("assert_eq!(compare(s, a, b).await, Ordering::Less)")));
        assert!(!awaits(&expr("assert_eq!(a, b)")));
        assert!(!awaits(&expr("spawn!(async { f().await })")));
    }

    #[test]
    fn nothing_to_await() {
        assert!(!awaits(&expr("a + b")));
        assert!(!awaits(&expr("match e { A => 1, B => 2 }")));
    }
}
