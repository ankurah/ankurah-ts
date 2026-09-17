//! The operands the emitter parses out of a macro this port has a hook for.
//!
//! For: the walk that types a body and the walk that writes it read one list.
//! Where the writing walk parses an operand the typing walk never saw, that
//! operand's constraints arrive after the statements above it are written, and
//! one local ends up with two answers.

use proc_macro2::TokenStream;
use syn::Expr;

use super::{format_call, items, logging, parse_exprs_from_tokens, select_futures};

/// Every operand of a supported macro, in the shape the emitter reads it.
///
/// A macro whose tokens the emitter passes through unchanged — `dbg!`,
/// `stringify!`, `cfg!` — parses nothing and types nothing, so it answers with
/// no operands.
pub fn macro_argument_exprs(mac: &syn::Macro) -> Vec<Expr> {
    let name = mac
        .path
        .segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default();
    // `tracing::warn!` and a bare `warn!` behind a `use` are one macro, so the
    // leaf name decides, exactly as the emitter's own dispatch does.
    if logging::level(&name).is_some() {
        return formatting_operands(&mac.tokens);
    }
    match name.as_str() {
        "select" => select_futures(&mac.tokens),
        "vec" => vec_operands(mac),
        "format" | "println" | "eprintln" | "print" | "eprint" | "panic" | "unreachable" => {
            formatting_operands(&mac.tokens)
        }
        "write" | "writeln" => written_operands(&mac.tokens),
        "params" | "assert" | "debug_assert" | "assert_eq" | "assert_ne" => {
            parse_exprs_from_tokens(&mac.tokens).unwrap_or_default()
        }
        "matches" => items::matches_operands(&mac.tokens),
        "anyhow" | "bail" => items::anyhow_operands(&mac.tokens),
        "hashmap" => items::hashmap_operands(&mac.tokens),
        "json" => items::json_operands(&mac.tokens),
        _ => Vec::new(),
    }
}

/// The values a formatting macro's placeholders read, positional then named.
/// The format string itself is a literal and types nothing.
pub(super) fn formatting_operands(tokens: &TokenStream) -> Vec<Expr> {
    let Some(written) = format_call::written(tokens) else {
        return Vec::new();
    };
    written
        .positional
        .into_iter()
        .chain(written.named.into_iter().map(|(_, value)| value))
        .collect()
}

/// `write!(f, "..", ..)`: the formatter is dropped by the lowering and never
/// typed, so the operands are those of the format call behind it.
fn written_operands(tokens: &TokenStream) -> Vec<Expr> {
    match super::write_arguments(tokens) {
        Some(rest) => formatting_operands(&rest),
        None => Vec::new(),
    }
}

/// `vec![a, b]` lists its elements and `vec![v; n]` repeats one: the count is
/// an operand too, so the walk that types the body sees the same expressions
/// the emitter writes.
fn vec_operands(mac: &syn::Macro) -> Vec<Expr> {
    if let Some(repeat) = super::repeat_form(&mac.tokens) {
        return vec![*repeat.expr, *repeat.len];
    }
    parse_exprs_from_tokens(&mac.tokens).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::macro_argument_exprs;
    use quote::quote;

    fn operands(tokens: proc_macro2::TokenStream) -> Vec<String> {
        let mac: syn::Macro = syn::parse2(tokens).expect("a macro invocation");
        macro_argument_exprs(&mac)
            .iter()
            .map(|e| quote!(#e).to_string())
            .collect()
    }

    /// Every macro the emitter parses into expressions hands the same
    /// expressions to the walk that types the body; every macro it passes
    /// through unchanged hands none.
    #[test]
    fn the_emitters_operands_and_the_typing_walks_are_one_list() {
        let cases: Vec<(proc_macro2::TokenStream, Vec<&str>)> = vec![
            (quote!(tracing::info!("{}", a)), vec!["a"]),
            (quote!(warn!("{} {}", a, b)), vec!["a", "b"]),
            (quote!(debug!("{n}", n = a)), vec!["a"]),
            (quote!(error!("{}", a)), vec!["a"]),
            (quote!(trace!("{}", a)), vec!["a"]),
            (quote!(format!("{}", a)), vec!["a"]),
            (quote!(println!("{}", a)), vec!["a"]),
            (quote!(eprintln!("{}", a)), vec!["a"]),
            (quote!(print!("{}", a)), vec!["a"]),
            (quote!(eprint!("{}", a)), vec!["a"]),
            (quote!(panic!("{}", a)), vec!["a"]),
            (quote!(unreachable!("{}", a)), vec!["a"]),
            (quote!(write!(f, "{}", a)), vec!["a"]),
            (quote!(writeln!(f, "{}", a)), vec!["a"]),
            (quote!(vec![a, b]), vec!["a", "b"]),
            (quote!(vec![a; n]), vec!["a", "n"]),
            (quote!(params![a, b]), vec!["a", "b"]),
            (quote!(assert!(a)), vec!["a"]),
            (quote!(debug_assert!(a)), vec!["a"]),
            (quote!(assert_eq!(a, b)), vec!["a", "b"]),
            (quote!(assert_ne!(a, b)), vec!["a", "b"]),
            (quote!(matches!(a, Some(_))), vec!["a"]),
            (quote!(matches!(a, Some(x) if b)), vec!["a", "b"]),
            (quote!(anyhow!(a)), vec!["a"]),
            (quote!(anyhow!("{}", a)), vec!["a"]),
            (quote!(bail!(a)), vec!["a"]),
            (quote!(hashmap! { a => b }), vec!["a", "b"]),
            (quote!(json!(a)), vec!["a"]),
            (quote!(select! { x = a => {} }), vec!["a"]),
            (quote!(dbg!(a)), vec![]),
            (quote!(stringify!(a)), vec![]),
            (quote!(cfg!(a)), vec![]),
            (quote!(todo!()), vec![]),
            (quote!(unimplemented!()), vec![]),
            (quote!(include_str!(a)), vec![]),
        ];
        for (invocation, want) in cases {
            let written = invocation.to_string();
            assert_eq!(operands(invocation), want, "operands of {}", written);
        }
    }
}
