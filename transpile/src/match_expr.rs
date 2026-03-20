//! Match expression translation — Rust match → TS patterns
//!
//! Handles Option match (Some/None → null checks), Result match (Ok/Err → try/catch),
//! and enum match (variants → .match({}) pattern).

use crate::name_map;
use crate::body::{translate_expr, translate_pat, indent};

/// Translate a match expression in return position (adds return to each arm)
pub fn translate_match_returning(match_expr: &syn::ExprMatch) -> String {
    let scrutinee = translate_expr(&match_expr.expr);

    if is_option_match(&match_expr.arms) {
        return translate_option_match_returning(&scrutinee, &match_expr.arms);
    }

    // For enum and other matches, the standard translation + return wrapping works
    let ts = translate_match(match_expr);
    if ts.contains(".match(") {
        format!("return {};", ts)
    } else {
        ts
    }
}

/// Option match with return in each branch
fn translate_option_match_returning(scrutinee: &str, arms: &[syn::Arm]) -> String {
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
                some_arm = Some((var_name, translate_expr(&arm.body)));
            }
            "None" | "_" => { none_arm = Some(translate_expr(&arm.body)); }
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
pub fn translate_match(match_expr: &syn::ExprMatch) -> String {
    let scrutinee = translate_expr(&match_expr.expr);

    if is_option_match(&match_expr.arms) {
        return translate_option_match(&scrutinee, &match_expr.arms);
    }
    if is_result_match(&match_expr.arms) {
        return translate_result_match(&scrutinee, &match_expr.arms);
    }
    if looks_like_enum_match(&match_expr.arms) {
        return translate_enum_match(&scrutinee, &match_expr.arms);
    }

    // Fallback: if/else chain
    let mut out = String::new();
    for (i, arm) in match_expr.arms.iter().enumerate() {
        let pat = translate_pat(&arm.pat);
        let body = translate_expr(&arm.body);
        let guard = arm.guard.as_ref().map(|(_, g)| format!(" && {}", translate_expr(g))).unwrap_or_default();

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

fn translate_option_match(scrutinee: &str, arms: &[syn::Arm]) -> String {
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
                some_arm = Some((var_name, translate_expr(&arm.body)));
            }
            "None" | "_" => {
                none_arm = Some(translate_expr(&arm.body));
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

fn translate_result_match(scrutinee: &str, arms: &[syn::Arm]) -> String {
    for arm in arms {
        if let syn::Pat::TupleStruct(ts) = &arm.pat {
            let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            if name == "Ok" {
                let var = ts.elems.first().map(translate_pat).unwrap_or_else(|| "v".to_string());
                let body = translate_expr(&arm.body);
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

fn translate_enum_match(scrutinee: &str, arms: &[syn::Arm]) -> String {
    let mut out = format!("{}.match({{\n", scrutinee);

    for arm in arms {
        // Collect field mappings: local_name → v.fieldName
        let (variant_name, field_mappings) = match &arm.pat {
            syn::Pat::TupleStruct(ts) => {
                let variant = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                let mappings: Vec<(String, String)> = ts.elems.iter().enumerate().map(|(i, pat)| {
                    let local = translate_pat(pat);
                    let accessor = format!("v._{}", i);
                    (local, accessor)
                }).collect();
                (variant, mappings)
            }
            syn::Pat::Struct(s) => {
                let variant = s.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                let mappings: Vec<(String, String)> = s.fields.iter().map(|f| {
                    let field_name = match &f.member {
                        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    // The local variable name may differ if renamed: `selection: query`
                    let local = translate_pat(&f.pat);
                    let accessor = format!("v.{}", field_name);
                    (local, accessor)
                }).collect();
                (variant, mappings)
            }
            syn::Pat::Path(p) => {
                let variant = p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                (variant, Vec::new())
            }
            syn::Pat::Or(or_pat) => {
                // OR pattern: And(l,r) | Or(l,r) => body
                // Emit separate arms for each alternative with the same body
                for case in &or_pat.cases {
                    let (vname, fmaps) = match case {
                        syn::Pat::TupleStruct(ts) => {
                            let v = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                            let m: Vec<(String, String)> = ts.elems.iter().enumerate().map(|(i, pat)| {
                                (translate_pat(pat), format!("v._{}", i))
                            }).collect();
                            (v, m)
                        }
                        syn::Pat::Struct(s) => {
                            let v = s.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                            let m: Vec<(String, String)> = s.fields.iter().map(|f| {
                                let field_name = match &f.member {
                                    syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                                    syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                                };
                                (translate_pat(&f.pat), format!("v.{}", field_name))
                            }).collect();
                            (v, m)
                        }
                        syn::Pat::Path(p) => {
                            let v = p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                            (v, Vec::new())
                        }
                        _ => continue,
                    };
                    let mut b = translate_expr(&arm.body);
                    let mut sorted = fmaps.clone();
                    sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
                    for (local, accessor) in &sorted {
                        b = replace_identifier(&b, local, accessor);
                    }
                    let b = if b.starts_with('[') { format!("{} as any", b) } else { b };
                    if fmaps.is_empty() {
                        out.push_str(&format!("  {}: () => {},\n", vname, b));
                    } else {
                        out.push_str(&format!("  {}: (v) => {},\n", vname, b));
                    }
                }
                continue;
            }
            syn::Pat::Wild(_) => continue,
            _ => (translate_pat(&arm.pat), Vec::new()),
        };

        let mut body = translate_expr(&arm.body);

        // Replace field references: local_name → v.fieldName
        // Process longer names first to avoid partial replacements
        let mut sorted_mappings = field_mappings.clone();
        sorted_mappings.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        for (local, accessor) in &sorted_mappings {
            body = replace_identifier(&body, local, accessor);
        }

        // Add type cast for array/tuple returns to avoid TS inference issues
        let body = if body.starts_with('[') { format!("{} as any", body) } else { body };

        if field_mappings.is_empty() {
            out.push_str(&format!("  {}: () => {},\n", variant_name, body));
        } else {
            out.push_str(&format!("  {}: (v) => {},\n", variant_name, body));
        }
    }

    out.push_str("})");
    out
}

/// Replace standalone identifier occurrences (not part of longer identifiers or property access)
fn replace_identifier(text: &str, from: &str, to: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    let bytes = text.as_bytes();

    while i < bytes.len() {
        if i + from.len() <= bytes.len() && &text[i..i + from.len()] == from {
            // Check that it's a standalone identifier (not part of a longer word)
            let before_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let after_ok = i + from.len() >= bytes.len() || !is_ident_char(bytes[i + from.len()]);
            // Skip if preceded by '.' (property access) but NOT '...' (spread)
            let not_property = i == 0 || bytes[i - 1] != b'.'
                || (i >= 3 && bytes[i-3] == b'.' && bytes[i-2] == b'.' && bytes[i-1] == b'.');

            if before_ok && after_ok && not_property {
                result.push_str(to);
                i += from.len();
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
