//! What one arm of a `.match({..})` is made of.
//!
//! For: an arm has to be assembled the same way wherever it comes from — the
//! variant the source named, the variant a catch-all stands for, or one link of
//! a chain — so the pieces and the decisions live here rather than in each
//! caller. `enum_match_over` used to format its arms one way and `catch_all`
//! another, and the catch-all's arms lost the match's value in every position
//! but the enclosing function's return.

use crate::body::{indent, BodyTranslator};
use super::Position;
use crate::name_map;
use super::owing::{guard_release, hole_in_an_arm, release_before_a_hole_in_the_bindings, release_of};
use super::translate_pat;
pub(super) use super::rendering::{arm_block, arm_block_parts, render_arm, ArmParts};

/// A name for the arm's parameter that nothing else in the arm answers to.
pub(super) fn arm_parameter(fields: &[(String, String)], body: &syn::Expr) -> String {
    shared_parameter(fields, &[body])
}

/// A name for the parameter one or more arms share, which none of their
/// patterns and none of their bodies answers to.
///
/// A chain's arms are branches of ONE arrow, so they take one parameter between
/// them, and a `const v` in any of their bodies — or in the catch-all's, which
/// stands as the chain's last `else` — would be declared beside it.
pub(super) fn shared_parameter(fields: &[(String, String)], bodies: &[&syn::Expr]) -> String {
    let taken: Vec<String> = bodies.iter().flat_map(|body| declared_in(body)).collect();
    let clashes = |name: &str| {
        fields.iter().any(|(local, _)| local == name) || taken.iter().any(|n| n == name)
    };
    if !clashes("v") {
        return "v".to_string();
    }
    for n in 0.. {
        let candidate = if n == 0 { "_v".to_string() } else { format!("_v{}", n) };
        if !clashes(&candidate) {
            return candidate;
        }
    }
    unreachable!("the search is unbounded")
}

/// Every name a `let` inside this expression binds.
///
/// A superset: a `let` in a nested BLOCK opens its own scope and could reuse
/// the parameter's name legally. Naming the parameter around it costs one
/// underscore and gets the question right without tracking scopes.
pub(super) fn declared_in(body: &syn::Expr) -> Vec<String> {
    struct Bound {
        names: Vec<String>,
    }
    impl syn::visit::Visit<'_> for Bound {
        fn visit_local(&mut self, local: &syn::Local) {
            self.names.extend(crate::body::pattern_names(&local.pat));
            syn::visit::visit_local(self, local);
        }
        // A closure has a scope of its own, and a name inside it shadows
        // legally.
        fn visit_expr_closure(&mut self, _: &syn::ExprClosure) {}
        fn visit_item(&mut self, _: &syn::Item) {}
    }
    let mut bound = Bound { names: Vec::new() };
    syn::visit::Visit::visit_expr(&mut bound, body);
    bound.names
}

/// The same walk, for a key that stands ALONE.
///
/// A variant only one arm names, with no arm below it, has nothing to fall
/// through to — so the inner test cannot be made at all and the site says so.
/// Everything else is what a chain link does.
pub(super) fn arm_declarations(
    pat: &syn::Pat,
    param: &str,
    fields: &[(String, String)],
    t: &BodyTranslator,
    match_expr: &syn::ExprMatch,
) -> Option<(String, Vec<String>, Vec<String>, Option<String>)> {
    let walked =
        super::payload::payload_walk(pat, param, fields, t, super::payload::Tests::Reported, match_expr)?;
    Some((walked.text, walked.bound_keys, walked.names, walked.refused))
}

/// The variant a pattern names and the payload slots it takes out of it, as
/// (local name, accessor within the arm's payload).
pub(super) fn payload_of(pat: &syn::Pat) -> Option<(String, Vec<(String, String)>)> {
    match pat {
        syn::Pat::TupleStruct(ts) => {
            let variant = ts.path.segments.last()?.ident.to_string();
            // K9: each element takes the member at its own position, so a
            // trailing `..` shifts nothing — that is the shape the rule is
            // written for, and `payload_walk` refuses one written anywhere else.
            let fields = ts
                .elems
                .iter()
                .enumerate()
                .map(|(i, p)| (translate_pat(p), format!("_{}", i)))
                .collect();
            Some((variant, fields))
        }
        syn::Pat::Struct(s) => {
            let variant = s.path.segments.last()?.ident.to_string();
            let fields = s
                .fields
                .iter()
                .map(|f| {
                    let member = match &f.member {
                        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    (translate_pat(&f.pat), member)
                })
                .collect();
            Some((variant, fields))
        }
        syn::Pat::Path(p) => Some((p.path.segments.last()?.ident.to_string(), Vec::new())),
        _ => None,
    }
}

/// A pattern's alternatives: an `|` pattern writes one body for several
/// variants, and each of them is an arm of its own with the same body.
pub(super) fn cases_of(pat: &syn::Pat) -> Vec<&syn::Pat> {
    match pat {
        syn::Pat::Or(or_pat) => or_pat.cases.iter().collect(),
        other => vec![other],
    }
}

/// An arm body as statements: its own control flow where it has some, and
/// otherwise the `return` that makes its value the arm's.
pub(super) fn arm_statements(body: &str, produces: bool, value: bool) -> String {
    if body.trim().is_empty() {
        return String::new();
    }
    if !value {
        return format!("{}\n", body);
    }
    // An arm of a match whose Rust value is `()` produces nothing. Handing back
    // what the port's own spelling produced — `Array.prototype.push` answers a
    // number where `Vec::push` answers nothing — gave the match a value its
    // position could not take.
    if !produces {
        return format!("{};\n", body);
    }
    format!("return {};\n", body)
}

/// An arm's value: its declarations, then the body. A body that is already a
/// sequence of statements keeps its own control flow; anything else is the
/// value the arm produces.
pub(super) fn as_arm_value(body: &str, bindings: &str, produces: bool, value: bool) -> String {
    // An arm whose body is nothing — `Self::Large(_) => { /* Vec drops itself */ }`
    // — still has to be a function. `Large: (v) => ,` is not one.
    if body.trim().is_empty() {
        if bindings.trim().is_empty() {
            return "{}".to_string();
        }
        return format!("{{\n{}  }}", indent(&indent(bindings)));
    }
    let statements = !value;
    // An arm that produces nothing is always written as statements: the bare
    // expression form would make the arrow hand back whatever the expression
    // produced, and the match's own value is `()`.
    if !produces {
        return format!(
            "{{\n{}  }}",
            indent(&indent(&format!("{}{}\n", bindings, arm_statements(body, false, value).trim_end())))
        );
    }
    // A tuple literal confuses TypeScript's inference across arms: `match`
    // takes its result type from the first arm it reads, and `[null, events]`
    // makes every later arm an error against it.
    let value = if body.starts_with('[') {
        format!("{} as any", body)
    } else {
        body.to_string()
    };
    if bindings.is_empty() && !statements {
        return value;
    }
    let tail = if statements {
        body.to_string()
    } else {
        format!("return {};", value)
    };
    // The arm sits two spaces in, so its block's contents sit at four and the
    // closing brace lines up with the arm.
    format!(
        "{{\n{}  }}",
        indent(&indent(&format!("{}{}\n", bindings, tail)))
    )
}

/// One arm of the runtime's own match: the key, and everything under it.
///
/// The arm's parameter must not collide with a name the pattern binds — nor
/// with one the arm's own BODY declares. `A(n) => { let v = f(n)?; .. }` wrote
/// `const v` beside the parameter `v`, which is a redeclaration: the emitted
/// arrow threw a TDZ `ReferenceError` before it read either.
#[allow(clippy::too_many_arguments)]
pub(super) fn translate_arm(
    case: &syn::Pat,
    variant: &str,
    fields: &[(String, String)],
    arm: &syn::Arm,
    t: &BodyTranslator,
    match_expr: &syn::ExprMatch,
    position: super::Position,
    produces: bool,
    takes: crate::ownership::scrutinee::Takes,
    scrutinee_ty: Option<&crate::ty::Ty>,
    is_async: bool,
) -> String {
    let param = arm_parameter(fields, &arm.body);
    let _bindings = t.enter_pattern(case, scrutinee_ty);
    let Some((payload, bound, declared, refused)) =
        arm_declarations(case, &param, fields, t, match_expr)
    else {
        // The pattern is one `pattern_test` cannot read back, and the key IS
        // the test for a plain arm, so there is nothing to keep.
        drop(_bindings);
        let what = format!(
            "an arm naming `{}` tests the payload with a pattern the translator cannot read \
             back, so this arm is not written",
            variant
        );
        t.report_match_gap(match_expr, what.clone());
        let keyword = if is_async { "async " } else { "" };
        let head = if fields.is_empty() {
            format!("  {}: {}() => ", variant, keyword)
        } else {
            format!("  {}: {}({}) => ", variant, keyword, param)
        };
        let block = hole_in_an_arm(&what, &param, !fields.is_empty(), takes);
        return format!("{}{{\n{}  }},\n", head, indent(&indent(&block)));
    };
    // A consuming arm owns every part of the payload, including the parts its
    // pattern wrote `_` for: `intoMatch` releases nothing of its own on any path
    // out, so an unowned part is a leak. Asked inside the pattern's own scope,
    // because the answer turns on what the names it bound are.
    let release_rest = release_of(case, &param, &bound, takes, t);
    if let Some(what) = refused {
        // K4: the pattern took a droppable name OUT of a member and left the
        // rest. A plain key has no arm below it, so the whole arm is the hole.
        drop(_bindings);
        let keyword = if is_async { "async " } else { "" };
        let head = if fields.is_empty() {
            format!("  {}: {}() => ", variant, keyword)
        } else {
            format!("  {}: {}({}) => ", variant, keyword, param)
        };
        let block = hole_in_an_arm(&what, &param, !fields.is_empty(), takes);
        return format!("{}{{\n{}  }},\n", head, indent(&indent(&block)));
    }
    let Body { body, lifted, owned, flags, value, .. } =
        translate_body(arm, &declared, takes, t, match_expr, position, produces);
    drop(_bindings);
    let refusing = release_before_a_hole_in_the_bindings(&payload, &param, !fields.is_empty(), takes);
    render_arm(
        ArmParts {
            variant,
            bindings: format!("{}{}{}", refusing, flags, payload),
            param: (!fields.is_empty() || !release_rest.is_empty()).then(|| param.clone()),
            body: &body,
            owned: &owned,
            lifted: &lifted,
            produces,
            value,
            is_async,
            release_rest,
        },
        t,
    )
}

/// One link of a chain: the same arm, written as the branch a test guards.
#[allow(clippy::too_many_arguments)]
pub(super) fn translate_link(
    case: &syn::Pat,
    variant: &str,
    param: &str,
    fields: &[(String, String)],
    arm: &syn::Arm,
    t: &BodyTranslator,
    match_expr: &syn::ExprMatch,
    position: super::Position,
    produces: bool,
    takes: crate::ownership::scrutinee::Takes,
    scrutinee_ty: Option<&crate::ty::Ty>,
    is_async: bool,
) -> super::chain::Link {
    let _bindings = t.enter_pattern(case, scrutinee_ty);
    let Some(super::payload::Payload {
        test,
        text: payload,
        bound_keys: bound,
        names: declared,
        refused,
    }) =
        super::payload::payload_walk(case, param, fields, t, super::payload::Tests::Kept, match_expr)
    else {
        // R12: the arm's pattern is one the translator cannot read back, so
        // neither the test nor the arms below it can be written, and the hole
        // stands where they would have run.
        drop(_bindings);
        let what = format!(
            "an arm naming `{}` tests the payload with a pattern the translator cannot read \
             back, so this arm and the arms below it are not written",
            variant
        );
        t.report_match_gap(match_expr, what.clone());
        return super::chain::Link {
            test: None,
            bindings: String::new(),
            guard: None,
            block: hole_in_an_arm(&what, param, !fields.is_empty(), takes),
            leaves: true,
        };
    };
    // The guard is translated inside the pattern's scope, because it reads the
    // names the pattern bound — and it is written before the body, so its
    // temporaries are numbered where Rust evaluates them. Its own declarations
    // are held here rather than lifted out of the match: Rust makes this test
    // after the variant dispatch, so a guard that takes a lock takes it only on
    // the path where the variant matched.
    let release_rest = release_of(case, param, &bound, takes, t);
    if let Some(what) = refused {
        // K4, and fixpass4's D2: the TEST still decides, so the refusal stands
        // in the branch and a value it does not match reaches the arm below.
        drop(_bindings);
        return super::chain::Link {
            test,
            bindings: String::new(),
            guard: None,
            block: hole_in_an_arm(&what, param, !fields.is_empty(), takes),
            leaves: true,
        };
    }
    let guard = arm.guard.as_ref().map(|(_, guard)| {
        let (test, lifted) = t.with_own_hoists(|| t.expr(guard));
        super::chain::tried::Guard {
            test,
            lifted,
            release: guard_release(&declared, &release_rest, takes, t),
        }
    });
    let Body { body, lifted, owned, flags, value, leaves } =
        translate_body(arm, &declared, takes, t, match_expr, position, produces);
    drop(_bindings);
    let refusing = release_before_a_hole_in_the_bindings(&payload, param, !fields.is_empty(), takes);
    let (bindings, block) = arm_block_parts(
        ArmParts {
            variant,
            bindings: format!("{}{}{}", refusing, flags, payload),
            param: None,
            body: &body,
            owned: &owned,
            lifted: &lifted,
            produces,
            value,
            is_async,
            release_rest,
        },
        t,
    );
    super::chain::Link { test, bindings, guard, block, leaves }
}

/// An arm's body, written for what the arm's arrow owes, and which form the
/// lowering wrote.
///
/// An arm IS an arrow function, so a block body is written as that arrow's own
/// statements: wrapping it in a block of its own would add a scope the arrow
/// already provides. Everything else goes to the position that wants the
/// value, which is what puts a `return` on each branch of a nested match
/// instead of leaving it standing as a statement (K2).
pub(super) fn body_of_an_arm(body: &syn::Expr, produces: bool, t: &BodyTranslator) -> (String, bool) {
    use crate::control_flow::Wrote;
    if let syn::Expr::Block(block) = body {
        if block.label.is_none() {
            return (t.translate_block(&block.block), false);
        }
    }
    if produces {
        let (text, wrote) = crate::control_flow::in_value_position(body, t);
        return (text, wrote == Wrote::Value);
    }
    (t.statements(body), !crate::control_flow::form::writes_statements(body, t))
}

/// An arm's body, and what the arm owes around it.
struct Body {
    body: String,
    lifted: Vec<crate::ownership::Hoist>,
    owned: Vec<crate::ownership::Owned>,
    flags: String,
    /// Is `body` one EXPRESSION whose value the arm's arrow still has to hand
    /// back, or a run of statements that has already done it?
    value: bool,
    /// Does every path out of the arm leave the arrow — so that a chain need
    /// write no jump after it?
    leaves: bool,
}

/// The body both forms of arm share: what it declares, what it owns, and what
/// it says about leaving early.
fn translate_body(
    arm: &syn::Arm,
    declared: &[String],
    takes: crate::ownership::scrutinee::Takes,
    t: &BodyTranslator,
    match_expr: &syn::ExprMatch,
    position: super::Position,
    produces: bool,
) -> Body {
    // Where the enum handed its payload over, the arm owns what the pattern
    // named and releases it however the arm is left.
    let names: Vec<String> = match takes {
        crate::ownership::scrutinee::Takes::Payload => declared.to_vec(),
        crate::ownership::scrutinee::Takes::Nothing => Vec::new(),
    };
    let owned = t.claim_bindings(
        &names,
        std::slice::from_ref(&syn::Stmt::Expr(arm.body.as_ref().clone(), None)),
    );
    // An arm is an arrow function, so what the arm's own expression lifted out
    // of itself stays inside it: the declaration names values the arm's payload
    // produced, which do not exist outside. A block body is written as the
    // arrow's own statements, with the `return` on its tail; as an
    // immediately-called function it computed the arm's value and threw it
    // away, and a `return` written inside it left the inner function rather
    // than the enclosing one.
    //
    // K2: where the match hands a value back, the body is written for the
    // position that WANTS one, so a nested match — `Expr::Placeholder =>
    // match values.next() { Some(v) => Ok(..), None => Err(..) }` — puts a
    // `return` on each of its own branches instead of standing there as a
    // statement whose value nobody takes. `ankql/ast.ts`'s
    // `Expr.populateRecursive` answered `undefined` for exactly that arm.
    let ((body, value), lifted) = t.with_own_hoists(|| body_of_an_arm(&arm.body, produces, t));
    let body = body.trim_end().to_string();
    // Where the match hands a value back, EVERY path out of the arm hands one
    // back too — that is what Rust's type for the arm says — so the lowering
    // wrote a `return` on each of them and the arm leaves. Where the match's
    // own value is `()`, nothing was returned and the arm leaves only where
    // the Rust does: an `if` with NO `else` runs on when its test fails, which
    // reading the last line of the text backwards could not tell (K2).
    let leaves = produces || crate::control_flow::form::always_leaves(&arm.body);
    // An arm is an arrow function, so a `?` inside one returns from the arm.
    // Where the match is the enclosing function's value that is exactly right —
    // the arm's `Result` is what the function returns — and where it is a
    // statement it is not, and nobody sees the error. `leaves_the_loop` routes
    // such a match through the sentinel, which sets `jump_as_value`; anything
    // still here has no route.
    if position == super::Position::Statement
        && !t.jump_as_value.get()
        && super::leaves_the_function(&arm.body)
    {
        t.report_match_gap(
            match_expr,
            "an arm leaves early, and the arm is an arrow function whose `return` leaves the \
             arm rather than the function, so nobody sees the error it left with",
        );
    }
    // An arm is an arrow function, so a local this arm hands away sets its drop
    // flag here — the same line the enclosing block would have written had the
    // arm been a statement of it.
    let flags = t.flag_sets_for(&arm.body);
    Body { body, lifted, owned, flags, value, leaves }
}

/// One arm's body, written for the position the match stands in.
///
/// F3: whether a value is wanted comes from the POSITION the lowering chose,
/// never from anything read off the generated text or off an expectation left
/// standing. An arm of a statement match produces nothing, so its block is a
/// run of statements — asked as an expression, `{ if n == 0 { return .. } .. }`
/// came back an arrow function whose value was then written as a statement of
/// its own.
pub(super) fn arm_body(body: &syn::Expr, t: &BodyTranslator, position: Position) -> (String, bool) {
    // K2: whether the arm LEAVES is answered from the Rust, on every path — an
    // `if` with no `else` runs on when its test fails, which reading the last
    // line of the text backwards could not tell.
    let leaves = |text: String| (text, crate::control_flow::form::always_leaves(body));
    let returning = |text: String| {
        (text, crate::control_flow::form::leaves_in_return_position(body, t))
    };
    match position {
        Position::Statement => match body {
            syn::Expr::Block(block) if block.label.is_none() => {
                leaves(t.translate_block(&block.block).trim_end().to_string())
            }
            other => leaves(t.expr(other)),
        },
        // Whatever this arm produces IS what the function answers, so the
        // function's return type is the arm's expectation — re-keyed onto the
        // arm's own span, because an expectation is matched by the span of the
        // expression it was written for. Without it `match f { true => Ok(xs
        // .collect()), .. }` had nothing saying what `collect` built, where the
        // same `Ok(..)` written as the function's tail did. Live at
        // `core/indexing/encoding.rs`, three arms of one match.
        Position::Returning => {
            let want = t.fn_return.clone();
            let (text, wrote) = t.expecting(body, want.as_ref(), || {
                crate::control_flow::in_value_position(body, t)
            });
            match wrote {
                // The `return` the position owes goes on here, and a `return`
                // leaves.
                crate::control_flow::Wrote::Value => (format!("return {};", text), true),
                crate::control_flow::Wrote::Statements => returning(text),
            }
        }
    }
}
