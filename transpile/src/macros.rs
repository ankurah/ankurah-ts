//! Macro translation — Rust macros → TS expressions

use crate::body::{indent, BodyTranslator};
use crate::name_map;

/// Translate a macro invocation to TS.
///
/// The arguments are Rust expressions written in the body the macro sits in, so
/// they are translated through *that* body's translator: a fresh one knows
/// nothing of the closure parameters and inferred types around it, and emitted
/// `${[...].map(..)}` — a spread of a name it could not see — where the source
/// had a closure parameter.
pub fn translate_macro(mac: &syn::Macro, t: &BodyTranslator) -> String {
    let name = mac.path.segments.last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default();

    match name.as_str() {
        "vec" => {
            // vec![a, b, c] → parse elements via syn
            if let Ok(args) = parse_exprs_from_tokens(&mac.tokens) {
                let translated: Vec<String> =
                    args.iter().map(|e| t.moved_value(e)).collect();
                format!("[{}]", translated.join(", "))
            } else {
                format!("[{}]", mac.tokens)
            }
        }
        "format" => translate_format_from_tokens(&mac.tokens, t),
        "println" | "eprintln" => format!("console.log({})", translate_format_from_tokens(&mac.tokens, t)),
        "dbg" => format!("console.log({})", mac.tokens),
        "write" | "writeln" => {
            // write!(f, "...", args) → parse tokens, skip formatter, format the rest
            translate_write_from_tokens(&mac.tokens, t)
        }
        "panic" | "unreachable" => format!("throw new Error({})", translate_format_from_tokens(&mac.tokens, t)),
        // The condition is Rust, so it is translated as Rust. Printing the
        // token stream put the source back out verbatim — `event_ids . contains
        // (& 7)` — which is neither TypeScript nor what the assertion meant.
        "assert" | "debug_assert" => match parse_exprs_from_tokens(&mac.tokens) {
            Ok(args) if !args.is_empty() => {
                let condition = t.expr_value(&args[0]);
                // `assert!(c, "..", x)` carries its own message; without one the
                // failure says only that the assertion failed, as Rust's does.
                let message = if args.len() > 1 {
                    let tail = args[1..].iter().map(|e| quote::quote!(#e));
                    translate_format_from_tokens(&quote::quote!(#(#tail),*), t)
                } else {
                    "'assertion failed'".to_string()
                };
                format!("if (!({})) throw new Error({})", condition, message)
            }
            _ => format!("/* {}!({}) */", name, mac.tokens),
        },
        "assert_eq" => {
            if let Ok(args) = parse_exprs_from_tokens(&mac.tokens) {
                if args.len() >= 2 {
                    format!("expect({}).toEqual({})", t.expr_value(&args[0]), t.expr_value(&args[1]))
                } else {
                    format!("/* assert_eq!({}) */", mac.tokens)
                }
            } else {
                format!("/* assert_eq!({}) */", mac.tokens)
            }
        }
        "assert_ne" => {
            if let Ok(args) = parse_exprs_from_tokens(&mac.tokens) {
                if args.len() >= 2 {
                    format!("expect({}).not.toEqual({})", t.expr_value(&args[0]), t.expr_value(&args[1]))
                } else {
                    format!("/* assert_ne!({}) */", mac.tokens)
                }
            } else {
                format!("/* assert_ne!({}) */", mac.tokens)
            }
        }
        // A macro cannot be a function, so the arms stay with the emitter and
        // only the arbitration goes to the runtime.
        "select" => translate_select(&mac.tokens, t),
        "todo" => "throw new Error('TODO')".to_string(),
        "unimplemented" => "throw new Error('unimplemented')".to_string(),
        // A macro nothing here expands is emitted as a comment, so whatever its
        // arguments hand away goes nowhere. Where one of them is a value the
        // block owns, that is a leak, and the site says so.
        _ => {
            t.report_unsupported_macro(mac, &name);
            format!("/* {}!({}) */", name, mac.tokens)
        }
    }
}

/// Translate format!("...", args) to template literal
/// Parses the macro tokens properly using syn to handle complex expressions
pub fn translate_format_macro(tokens: &str, t: &BodyTranslator) -> String {
    // Try to parse using syn's macro token parsing
    if let Ok(parsed) = parse_format_args(tokens, t) {
        return parsed;
    }

    // Fallback: simple string
    format!("'{}'", tokens.replace('\'', "\\'"))
}

/// Parse format!("fmt", arg1, arg2) into a template literal
fn parse_format_args(tokens: &str, t: &BodyTranslator) -> Result<String, ()> {
    let tokens = tokens.trim();

    // Extract the format string (first quoted string)
    if !tokens.starts_with('"') { return Err(()); }

    // Find end of format string, handling escaped quotes
    let mut i = 1;
    let bytes = tokens.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2; // skip escaped char
            continue;
        }
        if bytes[i] == b'"' {
            break;
        }
        i += 1;
    }
    if i >= bytes.len() { return Err(()); }

    let fmt_str = &tokens[1..i];
    let rest = tokens[i + 1..].trim().trim_start_matches(',').trim().trim_end_matches(',').trim();

    if rest.is_empty() && !fmt_str.contains('{') {
        return Ok(format!("'{}'", fmt_str));
    }

    // Parse arguments as syn expressions
    let args: Vec<String> = if rest.is_empty() {
        Vec::new()
    } else {
        parse_comma_separated_exprs(rest, t)
    };

    // Build template literal
    let mut result = String::from("`");
    let mut arg_idx = 0;

    let mut chars = fmt_str.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            if let Some(&'}') = chars.peek() {
                // {} — simple placeholder
                chars.next();
                if arg_idx < args.len() {
                    result.push_str(&format!("${{{}}}", args[arg_idx]));
                    arg_idx += 1;
                }
            } else {
                // {name} or {:format} — consume until }
                let mut spec = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '}' { break; }
                    spec.push(next);
                }
                // Check if it's a named arg like {name} or a format spec like {:?}
                if spec.starts_with(':') || spec.starts_with('#') {
                    // Format spec — use the next positional arg
                    if arg_idx < args.len() {
                        result.push_str(&format!("${{{}}}", args[arg_idx]));
                        arg_idx += 1;
                    }
                } else if !spec.is_empty() {
                    // Named arg like {name} — translate as variable
                    result.push_str(&format!("${{{}}}", name_map::to_camel_case(&spec)));
                }
            }
        } else if c == '\\' {
            if let Some(&next) = chars.peek() {
                chars.next();
                result.push(c);
                result.push(next);
            }
        } else {
            result.push(c);
        }
    }
    result.push('`');
    Ok(result)
}

/// Parse comma-separated expressions, respecting nesting
/// This handles cases like: `self.0.iter().map(|id| id.to_string()).join(",")`
fn parse_comma_separated_exprs(input: &str, t: &BodyTranslator) -> Vec<String> {
    // Try to parse as syn expressions using a helper wrapper
    let wrapped = format!("fn _args_() {{ let _x_ = ({},); }}", input);
    if let Ok(file) = syn::parse_file(&wrapped) {
        if let Some(syn::Item::Fn(func)) = file.items.first() {
            if let Some(syn::Stmt::Local(local)) = func.block.stmts.first() {
                if let Some(init) = &local.init {
                    if let syn::Expr::Tuple(tuple) = &*init.expr {
                        return tuple.elems.iter()
                            // A format argument is read through, not handed
                            // over: `Display` takes `&self`, so `{}` on a field
                            // is a borrow and not a partial move.
                            .map(|e| t.expr_value(e))
                            .collect();
                    }
                }
            }
        }
    }

    // Fallback: split on top-level commas (respecting parens/brackets)
    split_respecting_nesting(input)
}

/// Split on commas, respecting parentheses and bracket nesting
fn split_respecting_nesting(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;

    for c in input.chars() {
        match c {
            '(' | '[' | '{' | '<' => { depth += 1; current.push(c); }
            ')' | ']' | '}' | '>' => { depth -= 1; current.push(c); }
            ',' if depth == 0 => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    parts.push(trimmed);
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        parts.push(trimmed);
    }
    parts
}

// ── Token-based macro parsing (avoids string round-trip) ────────────

use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitStr, Token};
use proc_macro2::TokenStream;

/// Parse comma-separated expressions from a TokenStream
fn parse_exprs_from_tokens(tokens: &TokenStream) -> Result<Vec<Expr>, syn::Error> {
    struct ExprList(Vec<Expr>);
    impl Parse for ExprList {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut exprs = Vec::new();
            while !input.is_empty() {
                exprs.push(input.parse()?);
                if input.peek(Token![,]) { input.parse::<Token![,]>()?; }
            }
            Ok(ExprList(exprs))
        }
    }
    syn::parse2::<ExprList>(tokens.clone()).map(|el| el.0)
}

/// Parse format!("fmt", args...) directly from TokenStream
fn translate_format_from_tokens(tokens: &TokenStream, t: &BodyTranslator) -> String {
    struct FormatArgs { fmt: LitStr, args: Vec<Expr> }
    impl Parse for FormatArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let fmt: LitStr = input.parse()?;
            let mut args = Vec::new();
            while input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                if input.is_empty() { break; } // trailing comma
                args.push(input.parse()?);
            }
            Ok(FormatArgs { fmt, args })
        }
    }
    match syn::parse2::<FormatArgs>(tokens.clone()) {
        Ok(parsed) => {
            let translated_args: Vec<String> = parsed.args.iter()
                .map(|e| t.expr_value(e))
                .collect();
            build_template_literal(&parsed.fmt.value(), &translated_args)
        }
        Err(_) => {
            // Fallback to string-based parsing
            translate_format_macro(&tokens.to_string(), t)
        }
    }
}

/// Parse write!(f, "fmt", args...) directly from TokenStream
fn translate_write_from_tokens(tokens: &TokenStream, t: &BodyTranslator) -> String {
    struct WriteArgs { _formatter: Expr, fmt: LitStr, args: Vec<Expr> }
    impl Parse for WriteArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let formatter: Expr = input.parse()?;
            input.parse::<Token![,]>()?;
            let fmt: LitStr = input.parse()?;
            let mut args = Vec::new();
            while input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                if input.is_empty() { break; }
                args.push(input.parse()?);
            }
            Ok(WriteArgs { _formatter: formatter, fmt, args })
        }
    }
    match syn::parse2::<WriteArgs>(tokens.clone()) {
        Ok(parsed) => {
            let translated_args: Vec<String> = parsed.args.iter()
                .map(|e| t.expr_value(e))
                .collect();
            build_template_literal(&parsed.fmt.value(), &translated_args)
        }
        Err(_) => {
            // Fallback
            let s = tokens.to_string();
            let without_f = s.trim_start_matches("f ,").trim_start_matches("f,").trim();
            translate_format_macro(without_f, t)
        }
    }
}

/// Build a TS template literal from a format string and translated args
fn build_template_literal(fmt_str: &str, args: &[String]) -> String {
    let mut result = String::from("`");
    let mut arg_idx = 0;

    let mut chars = fmt_str.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            if let Some(&'}') = chars.peek() {
                chars.next();
                if arg_idx < args.len() {
                    result.push_str(&format!("${{{}}}", args[arg_idx]));
                    arg_idx += 1;
                }
            } else {
                let mut spec = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '}' { break; }
                    spec.push(next);
                }
                if spec.starts_with(':') || spec.starts_with('#') {
                    if arg_idx < args.len() {
                        result.push_str(&format!("${{{}}}", args[arg_idx]));
                        arg_idx += 1;
                    }
                } else if !spec.is_empty() {
                    result.push_str(&format!("${{{}}}", name_map::to_camel_case(&spec)));
                }
            }
        } else if c == '\\' {
            if let Some(&next) = chars.peek() {
                chars.next();
                result.push(c);
                result.push(next);
            }
        } else {
            result.push(c);
        }
    }
    result.push('`');
    result
}


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
fn translate_select(tokens: &proc_macro2::TokenStream, t: &BodyTranslator) -> String {
    let Some(arms) = parse_select(tokens) else {
        t.report_select_gap(tokens, "its arms are not `pattern = future => body`");
        return format!("/* select!({}) */", tokens);
    };
    if arms.is_empty() {
        return "/* select! with no arms */".to_string();
    }
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
        let arm_ts = t.statements(&arm.body);
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
    body.push_str("}\n");
    let held = t.fresh_temp();
    format!(
        "const {branches} = [\n{list}];\ntry {{\n{body}}} finally {{\n  \
         for (const {held} of {branches}) dropOwned({held}.promise);\n}}",
        branches = branches,
        list = list,
        body = indent(&body),
        held = held,
    )
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

/// The text with exactly one newline at its end, so a block body that already
/// ends in one does not open a blank line inside the arm.
fn ensure_newline(text: &str) -> String {
    format!("{}\n", text.trim_end())
}
