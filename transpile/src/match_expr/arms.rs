//! What one arm of a `.match({..})` is made of.
//!
//! For: an arm has to be assembled the same way wherever it comes from — the
//! variant the source named, the variant a catch-all stands for, or one link of
//! a chain — so the pieces and the decisions live here rather than in each
//! caller. `enum_match_over` used to format its arms one way and `catch_all`
//! another, and the catch-all's arms lost the match's value in every position
//! but the enclosing function's return.

use crate::body::{indent, BodyTranslator};
use crate::name_map;
use super::owing::{guard_release, hole_in_an_arm, release_before_a_hole_in_the_bindings, release_of};
use super::{is_statements, translate_pat};

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
) -> (String, Vec<String>, Vec<String>) {
    let walked =
        super::payload::payload_walk(pat, param, fields, t, super::payload::Tests::Reported, match_expr)
            .expect("a reported walk has no test to be a hole");
    (walked.text, walked.bound_keys, walked.names)
}

/// The variant a pattern names and the payload slots it takes out of it, as
/// (local name, accessor within the arm's payload).
pub(super) fn payload_of(pat: &syn::Pat) -> Option<(String, Vec<(String, String)>)> {
    match pat {
        syn::Pat::TupleStruct(ts) => {
            let variant = ts.path.segments.last()?.ident.to_string();
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

/// One arm of a `.match({..})`, as the pieces the two callers assemble it from.
///
/// `enum_match_over` writes an arm the source named; `catch_all` writes one per
/// variant the source left to its `_`. Both need the same decisions made the
/// same way — whether the arm's value needs a `return`, whether its releases
/// need a block, whether it is `async` — so both build one of these and hand it
/// to `render_arm`. The catch-all used to format its arms itself and lost the
/// match's value in every position but the enclosing function's return.
pub(super) struct ArmParts<'a> {
    /// The key the runtime's match dispatches on.
    pub variant: &'a str,
    /// What the arm declares before its body: the drop flags a hand-away owes,
    /// and the names the arm takes out of the value it was given.
    pub bindings: String,
    /// The payload parameter, where the arm takes one.
    pub param: Option<String>,
    pub body: &'a str,
    pub owned: &'a [crate::ownership::Owned],
    pub lifted: &'a [crate::ownership::Hoist],
    /// Whether the match hands a value back at all.
    pub produces: bool,
    pub is_async: bool,
    /// What this arm's outermost `finally` says about the parts of the payload
    /// no name took. A consuming arm owns the whole payload from the moment it
    /// is called — `intoMatch` releases nothing of its own, on any path — so an
    /// arm that binds only some of it releases the rest here.
    pub release_rest: String,
}

/// One arm of a `.match({..})`.
///
/// The payload's names are declared inside the arm, from the value the arm is
/// handed. They used to be substituted into the rendered TypeScript by walking
/// its characters, which could not tell a binding from the same word inside a
/// string literal or a comment, and knew nothing of a name shadowed further in.
pub(super) fn render_arm(parts: ArmParts<'_>, t: &BodyTranslator) -> String {
    let ArmParts { variant, bindings, param, body, owned, lifted, produces, is_async, release_rest } = parts;
    // An arm is an arrow function, and JavaScript's `await` belongs to the
    // nearest one — so an arm that awaits is `async`, and the whole `.match`
    // is awaited where it stands.
    let keyword = if is_async { "async " } else { "" };
    let head = match &param {
        Some(param) => format!("  {}: {}({}) => ", variant, keyword, param),
        None => format!("  {}: {}() => ", variant, keyword),
    };
    if owned.is_empty() && lifted.is_empty() && release_rest.is_empty() {
        return format!("{}{},\n", head, as_arm_value(body, &bindings, produces));
    }
    // An arm that owns what it was handed, that lifted a declaration out of its
    // own body, or that owes the payload a release, is always a block: the
    // release goes in a `finally`, so the arm cannot be the bare expression
    // form.
    let inner = arm_block(
        ArmParts { variant, bindings, param: None, body, owned, lifted, produces, is_async, release_rest },
        t,
    );
    format!("{}{{\n{}  }},\n", head, indent(&indent(&inner)))
}

/// One arm's body as STATEMENTS: what `render_arm` puts inside the arrow.
///
/// A link of a chain has already been handed the payload by the key around it,
/// so it needs the same statements without an arrow of its own.
pub(super) fn arm_block(parts: ArmParts<'_>, t: &BodyTranslator) -> String {
    let (bindings, inner) = arm_block_parts(parts, t);
    format!("{}{}", bindings, inner)
}

/// The same, with the DECLARATIONS kept apart from the body.
///
/// A guarded link needs the two separately: the names the pattern took have to
/// stand before the guard, because the guard reads them, and the body has to
/// stand inside the `if` the guard opens.
pub(super) fn arm_block_parts(parts: ArmParts<'_>, t: &BodyTranslator) -> (String, String) {
    let ArmParts { bindings, body, owned, lifted, produces, release_rest, .. } = parts;
    let mut inner = t.wrap_bindings(
        owned,
        crate::ownership::hoisted(&arm_statements(body, produces), lifted),
    );
    if !release_rest.is_empty() {
        inner = format!("try {{\n{}}} finally {{\n{}}}\n", indent(&inner), indent(&release_rest));
    }
    (bindings, inner)
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
pub(super) fn arm_statements(body: &str, produces: bool) -> String {
    if body.trim().is_empty() {
        return String::new();
    }
    if is_statements(body) {
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
pub(super) fn as_arm_value(body: &str, bindings: &str, produces: bool) -> String {
    // An arm whose body is nothing — `Self::Large(_) => { /* Vec drops itself */ }`
    // — still has to be a function. `Large: (v) => ,` is not one.
    if body.trim().is_empty() {
        if bindings.trim().is_empty() {
            return "{}".to_string();
        }
        return format!("{{\n{}  }}", indent(&indent(bindings)));
    }
    let statements = is_statements(body);
    // An arm that produces nothing is always written as statements: the bare
    // expression form would make the arrow hand back whatever the expression
    // produced, and the match's own value is `()`.
    if !produces {
        return format!(
            "{{\n{}  }}",
            indent(&indent(&format!("{}{}\n", bindings, arm_statements(body, false).trim_end())))
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
    let (payload, bound, declared) = arm_declarations(case, &param, fields, t, match_expr);
    // A consuming arm owns every part of the payload, including the parts its
    // pattern wrote `_` for: `intoMatch` releases nothing of its own on any path
    // out, so an unowned part is a leak. Asked inside the pattern's own scope,
    // because the answer turns on what the names it bound are.
    let release_rest = release_of(case, &param, &bound, takes, t);
    let Body { body, lifted, owned, flags } =
        translate_body(arm, &declared, takes, t, match_expr, position);
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
    let Some(super::payload::Payload { test, text: payload, bound_keys: bound, names: declared }) =
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
    let guard = arm.guard.as_ref().map(|(_, guard)| {
        let (test, lifted) = t.with_own_hoists(|| t.expr(guard));
        super::chain::tried::Guard {
            test,
            lifted,
            release: guard_release(&declared, &release_rest, takes, t),
        }
    });
    let Body { body, lifted, owned, flags } =
        translate_body(arm, &declared, takes, t, match_expr, position);
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
            is_async,
            release_rest,
        },
        t,
    );
    super::chain::Link {
        test,
        bindings,
        guard,
        block,
        leaves: leaves_by_itself(&body, produces),
    }
}

/// Does this arm's body leave the arrow by itself?
///
/// Only a chain with a GUARD asks: an arm whose guard failed falls into the arm
/// below it, so an arm that ran has to say it is finished — and a body that
/// returns or throws has said so already.
fn leaves_by_itself(body: &str, produces: bool) -> bool {
    super::leaves_the_arm(&arm_statements(body, produces))
}

/// An arm's body, and what the arm owes around it.
struct Body {
    body: String,
    lifted: Vec<crate::ownership::Hoist>,
    owned: Vec<crate::ownership::Owned>,
    flags: String,
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
    let (body, lifted) = t.with_own_hoists(|| t.statements(&arm.body));
    let body = body.trim_end().to_string();
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
    Body { body, lifted, owned, flags }
}

