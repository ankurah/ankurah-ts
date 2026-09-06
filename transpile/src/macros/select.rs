//! `tokio::select!`, as the runtime's arbiter.
//!
//! Split out of `macros.rs`, which was over the 600-line rule. What is here is
//! one macro whose lowering has nothing in common with the rest: it parses arms
//! of its own shape and writes a race, where every other macro writes a call or
//! a literal.

use super::ensure_newline;
use crate::body::{indent, BodyTranslator};
use crate::control_flow;
use syn::Expr;

/// One arm of a `select!`: what it waits for, and what it does when that arm wins.
struct SelectArm {
    pat: syn::Pat,
    future: syn::Expr,
    body: syn::Expr,
}

/// `tokio::select! { pat = fut => body, .. }` as the runtime's arbiter.
///
/// `select!` drops every branch when it returns — the winner and the losers
/// alike — and for a `Notified`, a `oneshot::Receiver` or a `JoinHandle` that
/// drop is the cancellation. So the branches are named once, raced once, and
/// released in a `finally` whichever arm won and whether or not one of them
/// threw. Erasing the macro to a comment ran none of the arms at all.
///
/// The winning arm's value is the select's value, and a macro is spliced into
/// whatever position it was written in. So the arbitration goes inside an async
/// arrow function called on the spot: the arm ends in a `return`, the call is
/// an expression a `let` or an argument can hold, and the branch release stays
/// in the `finally` the arrow function's `try` carries. Writing the arbitration
/// as bare statements threw the winning arm's value away wherever the select
/// stood, and did not parse at all where something bound it.
///
/// An arm that leaves what encloses the select — a `return`, a `break`, a
/// `continue`, a `?` — keeps the statement form instead. Inside the arrow
/// function that exit would land on the arrow function rather than on the
/// function or the loop the source wrote it against; as statements it lands
/// where Rust puts it. Such an arm can only stand where the select is a
/// statement, and a statement's value is thrown away in Rust too, so nothing
/// is lost by not producing one — but the select is reported all the same,
/// because the two forms are not the same lowering.
pub(super) fn translate_select(tokens: &proc_macro2::TokenStream, t: &BodyTranslator) -> String {
    let Some(arms) = parse_select(tokens) else {
        t.report_select_gap(tokens, "its arms are not `pattern = future => body`");
        return format!("undefined /* select!({}) */", tokens);
    };
    if arms.is_empty() {
        t.report_select_gap(
            tokens,
            "it is written with no arms, so there is nothing to race and no arm to take a value \
             from",
        );
        return "undefined /* select! with no arms */".to_string();
    }

    // What every arm does when it wins decides which of the two forms carries
    // the whole select, so it is settled before any arm is written.
    let escape = arms.iter().find_map(|arm| arm_leaves_the_select(&arm.body));
    if let Some(what) = escape {
        t.report_select_gap(
            tokens,
            &format!(
                "an arm {}, which an arrow function cannot carry — the exit would land on the \
                 arrow function rather than where the source wrote it — so the arbitration is \
                 written as statements and the select produces no value",
                what
            ),
        );
    }
    let produces_value = escape.is_none();

    let branches = t.fresh_temp();
    let outcome = t.fresh_temp();
    let mut list = String::new();
    for (i, arm) in arms.iter().enumerate() {
        list.push_str(&format!(
            "  {{ tag: '_{}', promise: {} }},\n",
            i,
            t.expr(&arm.future)
        ));
    }
    let mut body = format!("const {} = await select({});\n", outcome, branches);
    for (i, arm) in arms.iter().enumerate() {
        let subject = format!("{}.value", outcome);
        let _bindings = t.enter_pattern(&arm.pat, None);
        let (test, bind) = t.pattern_test(&subject, &arm.pat);
        let arm_ts = if produces_value {
            // A declaration lifted out of an arm cannot stand outside the arrow
            // function, where the arm's own bindings are not in scope, so it
            // comes back with the arm's text and is written inside it.
            let (written, lifted) = t.with_own_hoists(|| match &arm.body {
                // An arm written as a block already has the `if` above it for
                // its braces, and its own last expression becomes the `return`
                // through the block's tail. Asking for it in return position
                // instead put a second pair of braces inside the first.
                syn::Expr::Block(block) => t.translate_block(&block.block),
                other => control_flow::translate_expr_in_return_position(other, t),
            });
            crate::ownership::hoisted(&written, &lifted)
        } else {
            t.statements(&arm.body)
        };
        drop(_bindings);
        if test != "true" {
            t.report_select_gap(
                tokens,
                "an arm's pattern can fail to match, and tokio then disables that branch and \
                 keeps waiting; this lowering takes the arm anyway",
            );
        }
        let head = if i == 0 { "if" } else { "} else if" };
        body.push_str(&format!(
            "{} ({}.tag === '_{}') {{\n{}",
            head,
            outcome,
            i,
            indent(&format!("{}{}", bind, ensure_newline(&arm_ts)))
        ));
    }
    // The arbiter answers with one of the tags it was handed, so the chain
    // above covers every outcome — but only the code here knows that, and the
    // enclosing function has to produce a value on every path or the caller
    // reads it as possibly undefined. The last branch says so in the one way a
    // reader and a type checker both understand. H1: written for the escaping
    // form too, whose arms leave the function around the select — as the last
    // expression of a body that returns something, the chain fell off its end
    // and `tsc` said so.
    body.push_str("} else {\n  throw new Error('select: the arbiter answered with a tag no arm wrote');\n");
    body.push_str("}\n");
    let held = t.fresh_temp();
    let raced = format!(
        "const {branches} = [\n{list}];\ntry {{\n{body}}} finally {{\n  \
         for (const {held} of {branches}) dropOwned({held}.promise);\n}}",
        branches = branches,
        list = list,
        body = indent(&body),
        held = held,
    );
    t.own.select_wrote_statements.set(!produces_value);
    if produces_value {
        format!("await (async () => {{\n{}}})()", indent(&ensure_newline(&raced)))
    } else {
        raced
    }
}

/// How an arm body leaves what encloses the select, where it does.
///
/// A `return` and a `?` leave the function; a `break` and a `continue` leave
/// the loop around the select, unless the arm writes that loop itself. A
/// closure or an async block written inside the arm keeps its own exits, so
/// neither is looked into. The answer names the construct, because it goes
/// into the diagnostic a reader has to act on.
fn arm_leaves_the_select(body: &Expr) -> Option<&'static str> {
    struct Exits {
        found: Option<&'static str>,
    }
    impl syn::visit::Visit<'_> for Exits {
        fn visit_expr(&mut self, expr: &Expr) {
            match expr {
                Expr::Return(_) => self.found = self.found.or(Some("returns from the function")),
                Expr::Try(_) => {
                    self.found = self.found.or(Some("hands an error on with `?`"));
                }
                Expr::Break(_) => self.found = self.found.or(Some("breaks out of the loop")),
                Expr::Continue(_) => {
                    self.found = self.found.or(Some("continues the loop"));
                }
                // A loop written inside the arm catches its own `break` and
                // `continue`; only a `return` or a `?` inside it reaches past
                // the select.
                Expr::ForLoop(_) | Expr::While(_) | Expr::Loop(_) => {
                    let mut inner = Returns { found: None };
                    syn::visit::visit_expr(&mut inner, expr);
                    self.found = self.found.or(inner.found);
                    return;
                }
                Expr::Closure(_) | Expr::Async(_) => return,
                _ => {}
            }
            syn::visit::visit_expr(self, expr);
        }
    }
    struct Returns {
        found: Option<&'static str>,
    }
    impl syn::visit::Visit<'_> for Returns {
        fn visit_expr(&mut self, expr: &Expr) {
            match expr {
                Expr::Return(_) => self.found = self.found.or(Some("returns from the function")),
                Expr::Try(_) => {
                    self.found = self.found.or(Some("hands an error on with `?`"));
                }
                Expr::Closure(_) | Expr::Async(_) => return,
                _ => {}
            }
            syn::visit::visit_expr(self, expr);
        }
    }
    let mut exits = Exits { found: None };
    syn::visit::Visit::visit_expr(&mut exits, body);
    exits.found
}

/// The futures a `select!` waits on, for the move scan: each of them is taken
/// by value, and `select!` drops every one when it returns.
pub fn select_futures(tokens: &proc_macro2::TokenStream) -> Vec<syn::Expr> {
    parse_select(tokens)
        .unwrap_or_default()
        .into_iter()
        .map(|arm| arm.future)
        .collect()
}

fn parse_select(tokens: &proc_macro2::TokenStream) -> Option<Vec<SelectArm>> {
    let parser = |input: syn::parse::ParseStream| -> syn::Result<Vec<SelectArm>> {
        let mut arms = Vec::new();
        // `biased;` asks tokio for source order, which is what this lowering
        // does either way.
        if input.peek(syn::Ident) && input.peek2(syn::Token![;]) {
            input.parse::<syn::Ident>()?;
            input.parse::<syn::Token![;]>()?;
        }
        while !input.is_empty() {
            let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
            input.parse::<syn::Token![=]>()?;
            let future: syn::Expr = input.parse()?;
            input.parse::<syn::Token![=>]>()?;
            let body: syn::Expr = input.parse()?;
            let _ = input.parse::<syn::Token![,]>();
            arms.push(SelectArm { pat, future, body });
        }
        Ok(arms)
    };
    syn::parse::Parser::parse2(parser, tokens.clone()).ok()
}

