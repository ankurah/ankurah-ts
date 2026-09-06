//! The emitted `<T, U extends X>` list, as text.
//!
//! Generics reach emission as the string Rust wrote them in, and three
//! questions are asked of it: which parameters it names, how a bound gathered
//! elsewhere is merged into it, and how a Rust default (`S = RandomState`) is
//! taken off — TypeScript reads a default there as a different thing.

use std::collections::HashMap;

/// Merge impl block generic bounds into a class's generic declaration.
/// E.g., `<Upstream, Input, Output, Transform>` with bounds
/// `{Upstream: [Signal, With<Input>, Clone], Transform: [Clone]}` becomes
/// `<Upstream extends Signal & With<Input> & Clone, Input, Output, Transform extends Clone>`
/// The parameters written inside a generic list.
///
/// Two things have to be read the way TypeScript reads them. The list ends at
/// ONE `>`, however many the last parameter's own type ends with — taking every
/// trailing `>` off took the list's terminator with them, and the class then
/// read `class Reactor<E extends .., Ev extends Clone = Attested<Event> extends
/// Struct {`, which swallowed the rest of the file. And a comma inside a type
/// argument belongs to that argument: `<A, B<C, D>>` declares two parameters.
fn generic_params(generics: &str) -> Vec<String> {
    let inner = generics
        .strip_prefix('<')
        .and_then(|rest| rest.strip_suffix('>'))
        .unwrap_or(generics);
    let mut params = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for c in inner.chars() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                params.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() {
        params.push(current);
    }
    params
}

pub(super) fn merge_bounds_into_generics(generics: &str, bounds: &HashMap<String, Vec<String>>) -> String {
    if generics.is_empty() || bounds.is_empty() { return generics.to_string(); }
    let params = generic_params(generics);
    let merged: Vec<String> = params.iter().map(|p| {
        let p = p.trim();
        // Extract existing param name (before any `extends` or `=`)
        let param_name = p.split_whitespace().next().unwrap_or(p);
        // Check if there are impl bounds for this param
        if let Some(impl_bounds) = bounds.get(param_name) {
            // Check if param already has `extends` constraints
            if p.contains(" extends ") {
                // Extract existing bounds and merge
                let extends_pos = p.find(" extends ").unwrap();
                let existing_part = &p[extends_pos + 9..]; // after " extends "
                // Split on default " = " if present
                let (existing_bounds_str, default_part) = if let Some(eq_pos) = existing_part.find(" = ") {
                    (&existing_part[..eq_pos], &existing_part[eq_pos..])
                } else {
                    (existing_part, "")
                };
                let existing_bounds: Vec<&str> = existing_bounds_str.split(" & ").map(|s| s.trim()).collect();
                let mut all_bounds: Vec<String> = existing_bounds.iter().map(|s| s.to_string()).collect();
                for b in impl_bounds {
                    if !all_bounds.iter().any(|eb| eb == b) {
                        all_bounds.push(b.clone());
                    }
                }
                format!("{} extends {}{}", param_name, all_bounds.join(" & "), default_part)
            } else {
                // No existing extends — check for default
                let (base, default_part) = if let Some(eq_pos) = p.find(" = ") {
                    (&p[..eq_pos], &p[eq_pos..])
                } else {
                    (p, "")
                };
                let _ = base; // unused, param_name is what we need
                format!("{} extends {}{}", param_name, impl_bounds.join(" & "), default_part)
            }
        } else {
            p.to_string()
        }
    }).collect();
    format!("<{}>", merged.join(", "))
}

/// Strip bounds and defaults from generic params for use in type references.
/// `<T = void>` → `<T>`, `<T extends Foo = void>` → `<T>`, `<T extends Signal & Clone>` → `<T>`
pub(super) fn strip_generic_defaults(generics: &str) -> String {
    if generics.is_empty() { return generics.to_string(); }
    let params = generic_params(generics);
    let stripped: Vec<String> = params.iter().map(|p| {
        let p = p.trim();
        // Extract just the param name (before any `extends` or `=`)
        p.split_whitespace().next().unwrap_or(p).to_string()
    }).collect();
    format!("<{}>", stripped.join(", "))
}
