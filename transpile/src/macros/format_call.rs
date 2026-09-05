//! A formatting macro's arguments, read from the invocation.
//!
//! For: `format!`, `write!`, `println!`, `panic!`, the assertions and the
//! tracing calls all take a format string followed by the values it names, and
//! all of them have to render those values the way `std::fmt` renders them. This
//! supplies those values to the shared renderer, translated through the body the
//! macro sits in so that a closure parameter or a local is the name it has here.
//!
//! Rust 2021 lets a placeholder name a variable directly — `warn!("{self} did
//! {event}")` — with no argument list at all, so a name the argument list does
//! not carry is looked up as an expression in the enclosing body.

use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitStr, Token};

use crate::body::BodyTranslator;
use crate::derives::debug_fmt::debug_expr;
use crate::macros::format_emit::{render, Operands};
use crate::macros::format_spec::{parse, ArgRef};
use crate::ty::Ty;

/// A formatting macro's arguments as written: the string, the positional
/// expressions, and the `name = expr` ones.
pub struct Written {
    pub fmt: LitStr,
    pub positional: Vec<Expr>,
    pub named: Vec<(String, Expr)>,
}

impl Parse for Written {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fmt: LitStr = input.parse()?;
        let mut positional = Vec::new();
        let mut named = Vec::new();
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            // `name = expr` binds a placeholder by name; everything else is
            // positional. `syn` parses the former as an assignment expression.
            let expr: Expr = input.parse()?;
            match expr {
                Expr::Assign(assign) => match *assign.left {
                    Expr::Path(path) if path.path.get_ident().is_some() => {
                        let name = path.path.get_ident().expect("checked").to_string();
                        named.push((name, *assign.right));
                    }
                    other => positional.push(Expr::Assign(syn::ExprAssign {
                        left: Box::new(other),
                        ..assign
                    })),
                },
                other => positional.push(other),
            }
        }
        Ok(Written { fmt, positional, named })
    }
}

/// Read a macro's tokens as a format string and its arguments.
pub fn written(tokens: &TokenStream) -> Option<Written> {
    syn::parse2::<Written>(tokens.clone()).ok()
}

/// Render one formatting macro as a TypeScript string expression, reporting
/// whatever the port could not carry over at `at`.
pub fn format_string(
    written: &Written,
    t: &BodyTranslator,
    at: proc_macro2::Span,
) -> Option<String> {
    let pieces = match parse(&written.fmt.value()) {
        Ok(pieces) => pieces,
        Err(why) => {
            t.fallback(
                at,
                format!(
                    "this format string has {}, so it is written out as the literal it is",
                    why
                ),
            );
            return None;
        }
    };
    let mut operands = Arguments { written, t, at, next: 0 };
    Some(render(&pieces, &mut operands))
}

struct Arguments<'a, 'b> {
    written: &'a Written,
    t: &'a BodyTranslator<'b>,
    at: proc_macro2::Span,
    next: usize,
}

impl Arguments<'_, '_> {
    /// The expression a placeholder names, translated, and typed where the
    /// rendering depends on the type.
    fn value(&self, expr: &Expr, needs_type: bool) -> (String, Option<Ty>) {
        let ty = needs_type.then(|| self.t.resolve_expr_type(expr).ok()).flatten();
        (self.t.expr_value(expr), ty)
    }
}

impl Operands for Arguments<'_, '_> {
    fn operand(&mut self, which: &ArgRef, needs_type: bool) -> Option<(String, Option<Ty>)> {
        match which {
            ArgRef::Next => {
                let at = self.next;
                self.next += 1;
                self.written.positional.get(at).map(|e| self.value(e, needs_type))
            }
            ArgRef::Positional(index) => self
                .written
                .positional
                .get(*index)
                .map(|e| self.value(e, needs_type)),
            ArgRef::Named(name) => {
                if let Some((_, expr)) = self.written.named.iter().find(|(n, _)| n == name) {
                    return Some(self.value(expr, needs_type));
                }
                // Rust 2021 captures the variable of that name from the
                // enclosing scope, so the name is an expression in this body —
                // and rustc refuses the macro where there is none. A name
                // nothing in scope answers came out as itself, so the emitted
                // template read a binding that is not there and the line threw
                // `ReferenceError` rather than printing.
                let expr: Expr = syn::parse_str(name).ok()?;
                if !self.t.names_something(&expr) {
                    self.t.fallback(
                        self.at,
                        format!(
                            "the format string captures `{}`, and nothing of that name is in \
                             scope here, so the placeholder is written as `undefined`",
                            name
                        ),
                    );
                    return Some(("undefined".to_string(), None));
                }
                Some(self.value(&expr, needs_type))
            }
        }
    }

    fn display(&mut self, expr: &str, ty: Option<&Ty>, alternate: bool) -> String {
        if !alternate {
            return expr.to_string();
        }
        // `{:#}` on an `anyhow::Error` prints every message in the chain, which
        // the runtime's stand-in offers under its own name. Every other type
        // decides the alternate form inside its `Display`, which the port's
        // `toString` does not carry.
        if is_anyhow(self.t, ty) {
            return format!("{}.toStringAlternate()", expr);
        }
        self.t.fallback(
            self.at,
            format!(
                "`{}` is printed with `{{:#}}`, and the port's `toString` has no alternate form, \
                 so it is printed the ordinary way",
                expr
            ),
        );
        expr.to_string()
    }

    fn debug(&mut self, expr: &str, ty: Option<&Ty>, alternate: bool) -> String {
        if alternate {
            self.t.fallback(
                self.at,
                format!(
                    "`{}` is printed with `{{:#?}}`, whose indented layout the port does not \
                     write, so it is printed on one line",
                    expr
                ),
            );
        }
        let Some(reg) = self.t.registry() else {
            self.t.fallback(
                self.at,
                format!(
                    "`{}` is printed with `{{:?}}` on a translation path with no type context, \
                     so it prints as whatever its `toString` says",
                    expr
                ),
            );
            return expr.to_string();
        };
        match debug_expr(reg, ty, expr) {
            Ok(text) => text,
            Err(why) => {
                self.t.fallback(
                    self.at,
                    format!(
                        "`{}` is printed with `{{:?}}`, and it prints as whatever its `toString` \
                         says, because {}",
                        expr, why
                    ),
                );
                expr.to_string()
            }
        }
    }

    fn report(&mut self, what: String) {
        self.t.fallback(self.at, what);
    }
}

/// Is this the runtime's `anyhow::Error` stand-in?
fn is_anyhow(t: &BodyTranslator, ty: Option<&Ty>) -> bool {
    let Some(Ty::Named { id, .. }) = ty.map(|ty| ty.peel_refs()) else {
        return false;
    };
    t.registry()
        .is_some_and(|reg| reg.name_of(*id).ends_with("anyhow::Error"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(src: &str) -> Written {
        let tokens: TokenStream = syn::parse_str(src).expect("tokens");
        written(&tokens).expect("reads as a format call")
    }

    #[test]
    fn positional_arguments_are_read_in_order() {
        let w = parsed(r#""{} {}", a, b"#);
        assert_eq!(w.positional.len(), 2);
        assert!(w.named.is_empty());
    }

    #[test]
    fn a_named_argument_is_kept_by_its_name() {
        let w = parsed(r#""{width}", width = 8"#);
        assert!(w.positional.is_empty());
        assert_eq!(w.named.len(), 1);
        assert_eq!(w.named[0].0, "width");
    }

    #[test]
    fn an_assignment_that_is_not_a_binding_stays_positional() {
        let w = parsed(r#""{}", *x = 1"#);
        assert_eq!(w.positional.len(), 1);
        assert!(w.named.is_empty());
    }

    #[test]
    fn a_trailing_comma_adds_no_argument() {
        let w = parsed(r#""{}", a,"#);
        assert_eq!(w.positional.len(), 1);
    }
}
