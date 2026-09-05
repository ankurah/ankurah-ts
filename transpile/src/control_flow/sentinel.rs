//! The protocol a LIFTED body uses to hand a jump back to whoever reads it.
//!
//! For: several Rust expressions are written here as a function — an arm of
//! `match`/`intoMatch`, a block standing where a value is wanted. A `return`, a
//! `?`, a `break` and a `continue` written in such an expression mean the
//! enclosing FUNCTION or the enclosing LOOP, and none of those is something an
//! arrow can perform: `return break` does not parse, and a `break` inside an
//! arrow is a SyntaxError. So the arrow hands the jump back as a value — the
//! sentinel — and the statement that reads the arrow's value performs the jump
//! before anything looks at that value.
//!
//! Everything the protocol needs to agree on lives here: what a lifted body
//! hands back, which loops a lifted `break` can still reach, and the reader the
//! caller writes. Both places that lift a body — `BodyTranslator::value_through_a_jump`
//! for a block, `match_expr::jump_through_a_value` for a consuming match — ask
//! for that reader here, so there is one description of the shape rather than
//! two that drift apart.

use crate::body::BodyTranslator;

/// What a lifted body hands back in place of a `break` or a `continue`.
pub(crate) fn jump_marker(kind: &str, label: &Option<syn::Lifetime>) -> String {
    match label {
        Some(label) => format!("return {{ $jump: '{}', $label: '{}' }}", kind, label.ident),
        None => format!("return {{ $jump: '{}' }}", kind),
    }
}

/// What a lifted body hands back in place of a `return` it cannot perform.
///
/// The value the function was going to answer with travels out as `$value`
/// beside the `$jump` marker, and the statement that reads the lifted value
/// performs the real `return` before anything looks at it.
pub(crate) fn return_marker(value: &str) -> String {
    format!("return {{ $jump: 'return', $value: {} }}", value)
}

/// Does a `break`/`continue` written HERE have to travel out as a sentinel?
///
/// Only when the loop it names was written outside the body being lifted. A
/// `for` written inside a lifted arm is an ordinary JavaScript loop in the same
/// arrow, so a `continue` bound to it is an ordinary `continue`: ankql's
/// `generate_expr_sql` skips a NUL byte that way, and handing that `continue`
/// back as a sentinel left the String arm on the first NUL — an unterminated
/// SQL literal.
pub(crate) fn jump_leaves_the_lift(t: &BodyTranslator, label: &Option<syn::Lifetime>) -> bool {
    if !t.jump_as_value.get() {
        return false;
    }
    !reaches_a_loop_in_this_lift(t, label)
}

/// Is the loop this jump names one of the loops written inside the current
/// lift, so the code being written can jump to it directly?
fn reaches_a_loop_in_this_lift(t: &BodyTranslator, label: &Option<syn::Lifetime>) -> bool {
    let loops = t.loops_in_lift.borrow();
    match label {
        // A bare `break` leaves the innermost loop, so any loop inside the lift
        // answers it.
        None => !loops.is_empty(),
        Some(lifetime) => {
            let named = lifetime.ident.to_string();
            loops.iter().any(|carried| carried.as_deref() == Some(named.as_str()))
        }
    }
}

/// Write a body with the enclosing lift's jump handling put aside, and its own
/// loops counted from nothing.
///
/// The guard restores both, so a lift inside a lift sees only its own loops.
pub(crate) fn lifting<R>(t: &BodyTranslator, write: impl FnOnce() -> R) -> (R, bool) {
    let enclosing = t.jump_as_value.replace(true);
    let loops = std::mem::take(&mut *t.loops_in_lift.borrow_mut());
    let written = write();
    t.jump_as_value.set(enclosing);
    *t.loops_in_lift.borrow_mut() = loops;
    (written, enclosing)
}

/// Write the body of a loop, with that loop noted as one the code inside can
/// jump to without a sentinel.
///
/// `value` is the name a `break` carrying a value assigns to and the label it
/// leaves, where this loop stands in value position; every other loop has none.
pub(crate) fn inside_a_loop<R>(
    t: &BodyTranslator,
    label: &Option<syn::Label>,
    write: impl FnOnce() -> R,
) -> R {
    inside_a_loop_for(t, label, None, write)
}

pub(crate) fn inside_a_loop_for<R>(
    t: &BodyTranslator,
    label: &Option<syn::Label>,
    value: Option<(String, String)>,
    write: impl FnOnce() -> R,
) -> R {
    let name = label.as_ref().map(|l| l.name.ident.to_string());
    t.loops_in_lift.borrow_mut().push(name.clone());
    t.loop_frames.borrow_mut().push(crate::body::LoopFrame { label: name, value });
    let written = write();
    t.loop_frames.borrow_mut().pop();
    t.loops_in_lift.borrow_mut().pop();
    written
}

/// The loop a `break` carrying a value leaves, where that loop is one whose
/// value the code around it wanted.
///
/// A `break` with no label leaves the innermost loop, whatever kind it is; one
/// with a label leaves the loop that carries it.
pub(crate) fn value_loop_for(
    t: &BodyTranslator,
    label: &Option<syn::Lifetime>,
) -> Option<(String, String)> {
    let frames = t.loop_frames.borrow();
    let frame = match label {
        None => frames.last()?,
        Some(lifetime) => {
            let named = lifetime.ident.to_string();
            frames.iter().rev().find(|f| f.label.as_deref() == Some(named.as_str()))?
        }
    };
    frame.value.clone()
}

/// One jump a lifted body can hand back: `break`, `continue`, or either of them
/// naming a loop.
///
/// The kinds travel as `word` or `word#label`, which is how the two lifting
/// sites collect them from the syntax.
pub(crate) struct Handed<'k> {
    /// Does the lifted body exit the whole function — a `return`, or a `?`?
    pub returns: bool,
    /// Every `break`/`continue` it performs, spelled `word` or `word#label`.
    pub jumps: &'k [String],
}

/// The tests the statement after a lifted body writes: one per jump the body
/// can hand back, in the order that keeps the function's own exit first.
///
/// `held` is the name the lifted value was hoisted to. Where the reader itself
/// stands inside another lift, a jump it cannot perform is RE-RAISED whole
/// rather than unwrapped: unwrapping a `return` there would hand the enclosing
/// arrow a bare `Result.Err`, which the test above it does not recognise as an
/// exit, and the error would be read as the expression's value — core's
/// `reactor/fetch_gap.ts` handed a failed `build_continuation_predicate` on as
/// the gap selection. A `break` re-raised the same way reaches the reader that
/// can perform it.
pub(crate) fn reader(t: &BodyTranslator, held: &str, handed: Handed<'_>) -> String {
    let inside_a_lift = t.jump_as_value.get();
    let mut out = String::new();
    // The function's own exit comes first: the body left with an error, and
    // nothing below may read that error as if it were the expression's value.
    if handed.returns {
        let performed = if inside_a_lift {
            format!("return {held}", held = held)
        } else {
            format!("return ({held} as any).$value", held = held)
        };
        out.push_str(&format!(
            "if (({held} as any)?.$jump === 'return') {performed};\n",
            held = held,
            performed = performed
        ));
    }
    for kind in handed.jumps {
        let (word, label) = match kind.split_once('#') {
            Some((word, label)) => (word, Some(label.to_string())),
            None => (kind.as_str(), None),
        };
        let lifetime = label.as_ref().map(|name| {
            syn::Lifetime::new(&format!("'{}", name), proc_macro2::Span::call_site())
        });
        // `?.` rather than a truthiness test: a body that produces nothing
        // makes the value's inferred type `void`, which TypeScript refuses to
        // test for truth.
        let matched = match &label {
            Some(name) => format!(
                "({held} as any)?.$jump === '{word}' && ({held} as any)?.$label === '{name}'",
                held = held,
                word = word,
                name = name
            ),
            None => format!("({held} as any)?.$jump === '{word}'", held = held, word = word),
        };
        let performed = if jump_leaves_the_lift(t, &lifetime) {
            // This reader cannot perform the jump either — the loop is further
            // out still — so the sentinel travels on whole.
            format!("return {held}", held = held)
        } else {
            match &label {
                Some(name) => format!("{word} {name}", word = word, name = name),
                None => word.to_string(),
            }
        };
        out.push_str(&format!("if ({matched}) {performed};\n", matched = matched, performed = performed));
    }
    out
}

// ── Which jumps this expression hands out ─────────────────────────────
//
// The emitter and the analysis have to agree on one rule, or the reader tests
// for a jump nobody handed back and the arrow writes a `break` nothing can
// perform. The rule is the one `jump_leaves_the_lift` applies while writing:
// a `break` or a `continue` belongs to the innermost loop it names, and only a
// jump that reaches PAST every loop written inside the expression leaves it.

/// Every jump this expression hands out, as `break`, `continue`, or either with
/// `#label` after it.
pub(crate) fn jumps_out_of(expr: &syn::Expr) -> Vec<String> {
    let mut out = Vec::new();
    collect_jumps(expr, &mut Vec::new(), &mut out);
    out.dedup();
    out
}

/// Every jump an arm's body hands out, in the same spelling.
pub(crate) fn jumps_in(expr: &syn::Expr) -> Vec<String> {
    let mut out = Vec::new();
    collect_jumps(expr, &mut Vec::new(), &mut out);
    out
}

/// `enclosing` is the loops written inside the expression being examined, each
/// as the label it carries — the same stack `loops_in_lift` keeps while the
/// body is written.
fn collect_jumps(expr: &syn::Expr, enclosing: &mut Vec<Option<String>>, out: &mut Vec<String>) {
    match expr {
        syn::Expr::Break(brk) => {
            if !caught_here(enclosing, &brk.label) {
                out.push(match &brk.label {
                    Some(label) => format!("break#{}", label.ident),
                    None => "break".to_string(),
                });
            }
            // `break 'a f(break 'b)` is legal Rust, and the payload's own jump
            // leaves before this one does.
            if let Some(value) = &brk.expr {
                collect_jumps(value, enclosing, out);
            }
        }
        syn::Expr::Continue(cont) => {
            if !caught_here(enclosing, &cont.label) {
                out.push(match &cont.label {
                    Some(label) => format!("continue#{}", label.ident),
                    None => "continue".to_string(),
                });
            }
        }
        // A loop of the expression's own catches a jump that names it — and
        // only that jump. A `break 'outer` written inside it still leaves, and
        // dropping it here left the arm writing `break outer` inside an arrow,
        // which is a SyntaxError.
        syn::Expr::Loop(l) => within(l.label.as_ref(), enclosing, |e| {
            l.body.stmts.iter().for_each(|s| collect_jumps_stmt(s, e, out))
        }),
        // A `while` evaluates its condition on every turn, INSIDE the loop, so
        // a jump written there names this loop like one in the body.
        syn::Expr::While(w) => within(w.label.as_ref(), enclosing, |e| {
            collect_jumps(&w.cond, e, out);
            w.body.stmts.iter().for_each(|s| collect_jumps_stmt(s, e, out))
        }),
        // A `for` evaluates its sequence ONCE, before the loop, so a jump
        // written there names an enclosing loop and not this one.
        syn::Expr::ForLoop(f) => {
            collect_jumps(&f.expr, enclosing, out);
            within(f.label.as_ref(), enclosing, |e| {
                f.body.stmts.iter().for_each(|s| collect_jumps_stmt(s, e, out))
            })
        }
        // A closure carries its own control flow, and Rust does not let a jump
        // cross into one.
        syn::Expr::Closure(_) | syn::Expr::Async(_) => {}
        syn::Expr::Block(block) => block
            .block
            .stmts
            .iter()
            .for_each(|s| collect_jumps_stmt(s, enclosing, out)),
        syn::Expr::Unsafe(block) => block
            .block
            .stmts
            .iter()
            .for_each(|s| collect_jumps_stmt(s, enclosing, out)),
        syn::Expr::If(if_expr) => {
            collect_jumps(&if_expr.cond, enclosing, out);
            if_expr
                .then_branch
                .stmts
                .iter()
                .for_each(|s| collect_jumps_stmt(s, enclosing, out));
            if let Some((_, other)) = &if_expr.else_branch {
                collect_jumps(other, enclosing, out);
            }
        }
        syn::Expr::Match(m) => {
            collect_jumps(&m.expr, enclosing, out);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    collect_jumps(guard, enclosing, out);
                }
                collect_jumps(&arm.body, enclosing, out);
            }
        }
        // Everything else: a jump may be written below ANY expression — a call
        // argument, a method argument, a binary operand, a tuple element, an
        // index, an `await`, a `?`. The catch-all used to stop here, so the
        // reader analysis never saw those and emission wrote a bare `break`
        // inside an arrow, which a JavaScript engine refuses to parse. `syn`'s
        // own walk visits every child expression, and each of them comes back
        // through this same dispatch.
        other => {
            let mut walk = Children { enclosing, out };
            syn::visit::visit_expr(&mut walk, other);
        }
    }
}

/// `syn`'s default walk, routed back through `collect_jumps` at every child.
///
/// For: enumerating the expression kinds that may hold a jump is a list nobody
/// can keep complete, and the one it was written as missed every ordinary
/// expression. This asks `syn` for the children instead, and the kinds that
/// need their own treatment — the loops, the closures, the blocks — are the
/// ones `collect_jumps` still names.
struct Children<'a> {
    enclosing: &'a mut Vec<Option<String>>,
    out: &'a mut Vec<String>,
}

impl syn::visit::Visit<'_> for Children<'_> {
    fn visit_expr(&mut self, expr: &syn::Expr) {
        collect_jumps(expr, self.enclosing, self.out);
    }
}

fn within<R>(
    label: Option<&syn::Label>,
    enclosing: &mut Vec<Option<String>>,
    walk: impl FnOnce(&mut Vec<Option<String>>) -> R,
) -> R {
    enclosing.push(label.map(|l| l.name.ident.to_string()));
    let answer = walk(enclosing);
    enclosing.pop();
    answer
}

/// Does one of the loops written inside the expression catch this jump?
fn caught_here(enclosing: &[Option<String>], label: &Option<syn::Lifetime>) -> bool {
    match label {
        None => !enclosing.is_empty(),
        Some(lifetime) => {
            let named = lifetime.ident.to_string();
            enclosing.iter().any(|carried| carried.as_deref() == Some(named.as_str()))
        }
    }
}

fn collect_jumps_stmt(stmt: &syn::Stmt, enclosing: &mut Vec<Option<String>>, out: &mut Vec<String>) {
    match stmt {
        syn::Stmt::Expr(expr, _) => collect_jumps(expr, enclosing, out),
        syn::Stmt::Local(local) => {
            if let Some(init) = &local.init {
                collect_jumps(&init.expr, enclosing, out);
                // `let Some(x) = e else { break };` — the else block is where
                // the jump is written, and it is the whole point of the form.
                if let Some((_, diverge)) = &init.diverge {
                    collect_jumps(diverge, enclosing, out);
                }
            }
        }
        _ => {}
    }
}

/// Does this expression leave the FUNCTION — through a `return`, or through
/// the early exit a `?` performs?
///
/// The emitter lifts a block, a value-position `match` and an `if` used as a
/// value into an arrow function, and a `return` inside an arrow returns from
/// the arrow. So an expression that leaves the function cannot be lifted as it
/// stands: the exit is handed back as a value the statement below performs,
/// exactly as a `break` is. A closure, an `async` block and a nested item
/// carry their own exits and are not this expression's.
pub(crate) fn leaves_the_function(expr: &syn::Expr) -> bool {
    struct Exits {
        found: bool,
    }
    impl syn::visit::Visit<'_> for Exits {
        fn visit_expr(&mut self, expr: &syn::Expr) {
            match expr {
                // `write!(f, "..")?` is not an exit here: a formatter body
                // APPENDS, so the emitter writes the whole thing as a string
                // and there is no `Result` to leave with.
                syn::Expr::Try(t) if crate::body::as_write_macro(&t.expr).is_some() => return,
                syn::Expr::Return(_) | syn::Expr::Try(_) => {
                    self.found = true;
                    return;
                }
                // A closure's `return` leaves the closure; an `async` block's
                // leaves the future. Neither reaches this function.
                syn::Expr::Closure(_) | syn::Expr::Async(_) => return,
                _ => {}
            }
            syn::visit::visit_expr(self, expr);
        }
        fn visit_item(&mut self, _: &syn::Item) {}
    }
    let mut exits = Exits { found: false };
    syn::visit::Visit::visit_expr(&mut exits, expr);
    exits.found
}

/// Does this expression, or an arm of it, leave a loop that stands outside it?
pub(crate) fn jumps_out_of_a_loop(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Match(m) => m.arms.iter().any(|arm| jumps_out(&arm.body)),
        other => jumps_out(other),
    }
}

/// Does this expression leave a loop that stands outside it?
pub(crate) fn jumps_out(expr: &syn::Expr) -> bool {
    !jumps_out_of(expr).is_empty()
}
