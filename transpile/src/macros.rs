//! Macro translation — Rust macros → TS expressions

use crate::name_map;

/// Translate a macro invocation to TS
pub fn translate_macro(mac: &syn::Macro) -> String {
    let name = mac.path.segments.last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default();

    let tokens = mac.tokens.to_string();

    match name.as_str() {
        "vec" => {
            // vec![a, b, c] → [a, b, c]
            format!("[{}]", tokens)
        }
        "format" => translate_format_macro(&tokens),
        "println" | "eprintln" => format!("console.log({})", translate_format_macro(&tokens)),
        "dbg" => format!("console.log({})", tokens),
        "write" | "writeln" => {
            // write!(f, "...", args) → skip the formatter, just format
            let without_f = tokens.trim_start_matches("f ,").trim_start_matches("f,").trim();
            translate_format_macro(without_f)
        }
        "panic" | "unreachable" => format!("throw new Error({})", translate_format_macro(&tokens)),
        "assert" => {
            format!("if (!({})) throw new Error('assertion failed')", tokens)
        }
        "assert_eq" => {
            let parts: Vec<&str> = tokens.splitn(2, ',').collect();
            if parts.len() == 2 {
                format!("expect({}).toEqual({})", parts[0].trim(), parts[1].trim())
            } else {
                format!("/* assert_eq!({}) */", tokens)
            }
        }
        "assert_ne" => {
            let parts: Vec<&str> = tokens.splitn(2, ',').collect();
            if parts.len() == 2 {
                format!("expect({}).not.toEqual({})", parts[0].trim(), parts[1].trim())
            } else {
                format!("/* assert_ne!({}) */", tokens)
            }
        }
        "todo" => "throw new Error('TODO')".to_string(),
        "unimplemented" => "throw new Error('unimplemented')".to_string(),
        _ => format!("/* {}!({}) */", name, tokens),
    }
}

/// Translate format!("...", args) to template literal
/// Parses the macro tokens properly using syn to handle complex expressions
pub fn translate_format_macro(tokens: &str) -> String {
    // Try to parse using syn's macro token parsing
    if let Ok(parsed) = parse_format_args(tokens) {
        return parsed;
    }

    // Fallback: simple string
    format!("'{}'", tokens.replace('\'', "\\'"))
}

/// Parse format!("fmt", arg1, arg2) into a template literal
fn parse_format_args(tokens: &str) -> Result<String, ()> {
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
    let rest = tokens[i + 1..].trim().trim_start_matches(',').trim();

    if rest.is_empty() && !fmt_str.contains('{') {
        return Ok(format!("'{}'", fmt_str));
    }

    // Parse arguments as syn expressions
    let args: Vec<String> = if rest.is_empty() {
        Vec::new()
    } else {
        parse_comma_separated_exprs(rest)
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
fn parse_comma_separated_exprs(input: &str) -> Vec<String> {
    // Try to parse as syn expressions using a helper wrapper
    let wrapped = format!("fn _args_() {{ let _x_ = ({},); }}", input);
    if let Ok(file) = syn::parse_file(&wrapped) {
        if let Some(syn::Item::Fn(func)) = file.items.first() {
            if let Some(syn::Stmt::Local(local)) = func.block.stmts.first() {
                if let Some(init) = &local.init {
                    if let syn::Expr::Tuple(tuple) = &*init.expr {
                        return tuple.elems.iter()
                            .map(|e| crate::body::translate_expr(e))
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
