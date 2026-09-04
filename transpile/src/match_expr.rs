//! Match expression translation — Rust match → TS patterns
//!
//! Handles Option match (Some/None → null checks), Result match (Ok/Err → try/catch),
//! and enum match (variants → .match({}) pattern).

use crate::name_map;
use crate::body::{translate_pat, indent, BodyTranslator};

/// Translate a match expression in return position (adds return to each arm)
pub fn translate_match_returning(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let scrutinee = t.expr(&match_expr.expr);

    if is_option_match(&match_expr.arms) {
        return translate_option_match_returning(&scrutinee, match_expr, t);
    }

    // For enum and other matches, the standard translation + return wrapping works
    let ts = translate_match(match_expr, t);
    if ts.contains(".match(") {
        format!("return {};", ts)
    } else {
        ts
    }
}

/// Option match with return in each branch
fn translate_option_match_returning(scrutinee: &str, match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let arms = &match_expr.arms;
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    let mut some_arm = None;
    let mut none_arm = None;

    for arm in arms {
        let arm_type = match &arm.pat {
            syn::Pat::TupleStruct(ts) => ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
            syn::Pat::Ident(ident) => ident.ident.to_string(),
            syn::Pat::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
            syn::Pat::Wild(_) => "_".to_string(),
            _ => String::new(),
        };

        match arm_type.as_str() {
            "Some" => {
                let var_name = if let syn::Pat::TupleStruct(ts) = &arm.pat {
                    ts.elems.first().map(translate_pat).unwrap_or_else(|| "v".to_string())
                } else { "v".to_string() };
                let _arm = t.enter_pattern(&arm.pat, scrutinee_ty.as_ref());
                some_arm = Some((var_name, t.expr(&arm.body)));
            }
            "None" | "_" => { none_arm = Some(t.expr(&arm.body)); }
            _ => {}
        }
    }

    match (some_arm, none_arm) {
        (Some((var, some_body)), Some(none_body)) => {
            format!("if ({} != null) {{\n  const {} = {};\n  return {};\n}} else {{\n  return {};\n}}",
                scrutinee, var, scrutinee, some_body, none_body)
        }
        (Some((var, some_body)), None) => {
            format!("if ({} != null) {{\n  const {} = {};\n  return {};\n}}",
                scrutinee, var, scrutinee, some_body)
        }
        (None, Some(none_body)) => {
            format!("if ({} == null) {{\n  return {};\n}}", scrutinee, none_body)
        }
        _ => format!("return /* match {} */;", scrutinee),
    }
}

/// Translate a match expression
pub fn translate_match(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let scrutinee = t.expr(&match_expr.expr);

    if is_option_match(&match_expr.arms) {
        return translate_option_match(&scrutinee, match_expr, t);
    }
    if is_result_match(&match_expr.arms) {
        return translate_result_match(&scrutinee, match_expr, t);
    }
    if looks_like_enum_match(&match_expr.arms) {
        return translate_enum_match(&scrutinee, match_expr, t);
    }

    // Fallback: if/else chain
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    let mut out = String::new();
    for (i, arm) in match_expr.arms.iter().enumerate() {
        let pat = translate_pat(&arm.pat);
        let _arm = t.enter_pattern(&arm.pat, scrutinee_ty.as_ref());
        let body = t.expr(&arm.body);
        let guard = arm.guard.as_ref().map(|(_, g)| format!(" && {}", t.expr(g))).unwrap_or_default();
        drop(_arm);

        if i == 0 {
            out.push_str(&format!("if ({}{}) {{\n{}}}", pat, guard, indent(&format!("{}\n", body))));
        } else if pat == "_" {
            out.push_str(&format!(" else {{\n{}}}", indent(&format!("{}\n", body))));
        } else {
            out.push_str(&format!(" else if ({}{}) {{\n{}}}", pat, guard, indent(&format!("{}\n", body))));
        }
    }
    out
}

fn is_option_match(arms: &[syn::Arm]) -> bool {
    arms.iter().any(|arm| {
        match &arm.pat {
            syn::Pat::TupleStruct(ts) => {
                let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                name == "Some" || name == "None"
            }
            syn::Pat::Ident(ident) => ident.ident == "None",
            syn::Pat::Path(path) => {
                path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default() == "None"
            }
            _ => false,
        }
    })
}

fn translate_option_match(scrutinee: &str, match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let arms = &match_expr.arms;
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    let mut some_arm = None;
    let mut none_arm = None;

    for arm in arms {
        let arm_type = match &arm.pat {
            syn::Pat::TupleStruct(ts) => ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
            syn::Pat::Ident(ident) => ident.ident.to_string(),
            syn::Pat::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
            syn::Pat::Wild(_) => "_".to_string(),
            _ => String::new(),
        };

        match arm_type.as_str() {
            "Some" => {
                let var_name = if let syn::Pat::TupleStruct(ts) = &arm.pat {
                    ts.elems.first().map(translate_pat).unwrap_or_else(|| "v".to_string())
                } else {
                    "v".to_string()
                };
                let _arm = t.enter_pattern(&arm.pat, scrutinee_ty.as_ref());
                some_arm = Some((var_name, t.expr(&arm.body)));
            }
            "None" | "_" => {
                none_arm = Some(t.expr(&arm.body));
            }
            _ => {}
        }
    }

    match (some_arm, none_arm) {
        (Some((var, some_body)), Some(none_body)) => {
            format!("if ({} != null) {{\n  const {} = {};\n{}\n}} else {{\n{}\n}}",
                scrutinee, var, scrutinee, indent(&some_body), indent(&none_body))
        }
        (Some((var, some_body)), None) => {
            format!("if ({} != null) {{\n  const {} = {};\n{}\n}}",
                scrutinee, var, scrutinee, indent(&some_body))
        }
        (None, Some(none_body)) => {
            format!("if ({} == null) {{\n{}\n}}", scrutinee, indent(&none_body))
        }
        _ => format!("/* match {} */", scrutinee),
    }
}

fn is_result_match(arms: &[syn::Arm]) -> bool {
    arms.iter().any(|arm| {
        if let syn::Pat::TupleStruct(ts) = &arm.pat {
            let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            name == "Ok" || name == "Err"
        } else {
            false
        }
    })
}

fn translate_result_match(scrutinee: &str, match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    for arm in &match_expr.arms {
        if let syn::Pat::TupleStruct(ts) = &arm.pat {
            let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            if name == "Ok" {
                let var = ts.elems.first().map(translate_pat).unwrap_or_else(|| "v".to_string());
                let _arm = t.enter_pattern(&arm.pat, scrutinee_ty.as_ref());
                let body = t.expr(&arm.body);
                drop(_arm);
                return format!("const {} = {};\n{}", var, scrutinee, body);
            }
        }
    }
    format!("/* match {} */", scrutinee)
}

fn looks_like_enum_match(arms: &[syn::Arm]) -> bool {
    arms.iter().any(|arm| {
        match &arm.pat {
            syn::Pat::TupleStruct(ts) => {
                let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                name != "Some" && name != "None" && name != "Ok" && name != "Err"
                    && name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            }
            syn::Pat::Struct(s) => {
                let name = s.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            }
            syn::Pat::Path(p) => p.path.segments.len() >= 2,
            _ => false,
        }
    })
}

fn translate_enum_match(scrutinee: &str, match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    let mut out = format!("{}.match({{\n", scrutinee);

    for arm in &match_expr.arms {
        // An `|` pattern writes one body for several variants; each gets its own
        // arm with the same body, bound through its own payload.
        let cases: Vec<&syn::Pat> = match &arm.pat {
            syn::Pat::Or(or_pat) => or_pat.cases.iter().collect(),
            other => vec![other],
        };
        for case in cases {
            let Some((variant, fields)) = payload_of(case) else {
                continue;
            };
            let _bindings = t.enter_pattern(case, scrutinee_ty.as_ref());
            let body = t.expr(&arm.body);
            drop(_bindings);
            out.push_str(&render_arm(&variant, &fields, &body));
        }
    }

    out.push_str("})");
    out
}

/// The variant a pattern names and the payload slots it takes out of it, as
/// (local name, accessor within the arm's payload).
fn payload_of(pat: &syn::Pat) -> Option<(String, Vec<(String, String)>)> {
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

/// One arm of a `.match({..})`.
///
/// The payload's names are declared inside the arm, from the value the arm is
/// handed. They used to be substituted into the rendered TypeScript by walking
/// its characters, which could not tell a binding from the same word inside a
/// string literal or a comment, and knew nothing of a name shadowed further in.
fn render_arm(variant: &str, fields: &[(String, String)], body: &str) -> String {
    if fields.is_empty() {
        return format!("  {}: () => {},\n", variant, as_arm_value(body, ""));
    }
    // The arm's parameter must not collide with a name the pattern binds.
    let param = if fields.iter().any(|(local, _)| local == "v") { "_v" } else { "v" };
    let mut bindings = String::new();
    for (local, accessor) in fields {
        if local == "_" {
            continue;
        }
        bindings.push_str(&format!("const {} = {}.{};\n", local, param, accessor));
    }
    format!("  {}: ({}) => {},\n", variant, param, as_arm_value(body, &bindings))
}

/// An arm's value: its declarations, then the body. A body that is already a
/// sequence of statements keeps its own control flow; anything else is the
/// value the arm produces.
fn as_arm_value(body: &str, bindings: &str) -> String {
    let statements = body.starts_with("if ")
        || body.starts_with("for ")
        || body.starts_with("while ")
        || body.starts_with("return ")
        || body.starts_with("throw ")
        || body.starts_with('{')
        || body.contains(";\n");
    if bindings.is_empty() && !statements {
        // A tuple literal confuses TypeScript's inference across arms.
        return if body.starts_with('[') {
            format!("{} as any", body)
        } else {
            body.to_string()
        };
    }
    let tail = if statements {
        body.to_string()
    } else {
        format!("return {};", body)
    };
    // The arm sits two spaces in, so its block's contents sit at four and the
    // closing brace lines up with the arm.
    format!(
        "{{\n{}  }}",
        indent(&indent(&format!("{}{}\n", bindings, tail)))
    )
}
