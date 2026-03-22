//! Conditional compilation — #[cfg(...)] expression parsing and evaluation
//!
//! Evaluates Rust cfg expressions against a declared feature set.
//! Used to select which items to transpile (e.g., singlethread vs multithread).

use std::collections::HashSet;

/// Feature configuration for conditional compilation.
/// Declares which features are enabled for transpilation.
#[derive(Debug, Clone)]
pub struct CfgFeatures {
    /// Enabled features (e.g., "singlethread")
    enabled: HashSet<String>,
}

impl CfgFeatures {
    pub fn new(enabled: Vec<String>) -> Self {
        CfgFeatures {
            enabled: enabled.into_iter().collect(),
        }
    }

    /// Check if a feature is enabled
    pub fn is_enabled(&self, feature: &str) -> bool {
        self.enabled.contains(feature)
    }
}

/// Parsed cfg expression
#[derive(Debug, Clone, PartialEq)]
enum CfgExpr {
    /// cfg(feature = "name")
    Feature(String),
    /// cfg(not(...))
    Not(Box<CfgExpr>),
    /// cfg(any(..., ...))
    Any(Vec<CfgExpr>),
    /// cfg(all(..., ...))
    All(Vec<CfgExpr>),
    /// cfg(test), cfg(debug_assertions), etc. — non-feature predicates
    Predicate(String),
}

impl CfgExpr {
    /// Evaluate this cfg expression against the feature set.
    /// Non-feature predicates (test, debug_assertions) evaluate to false.
    fn eval(&self, features: &CfgFeatures) -> bool {
        match self {
            CfgExpr::Feature(name) => features.is_enabled(name),
            CfgExpr::Not(inner) => !inner.eval(features),
            CfgExpr::Any(exprs) => exprs.iter().any(|e| e.eval(features)),
            CfgExpr::All(exprs) => exprs.iter().all(|e| e.eval(features)),
            CfgExpr::Predicate(name) => {
                // Non-feature predicates: test=false, debug_assertions=false for transpilation
                match name.as_str() {
                    "test" => false,
                    "debug_assertions" => false,
                    _ => false,
                }
            }
        }
    }
}

/// Check if an item with the given attributes should be skipped (cfg evaluates to false).
/// Returns true if the item should be EXCLUDED from transpilation.
pub fn should_skip(attrs: &[syn::Attribute], features: &CfgFeatures) -> bool {
    for attr in attrs {
        if attr.path().is_ident("cfg") {
            let tokens = attr.meta.to_token_stream().to_string();
            // Normalize spaces: syn tokenizer adds spaces around punctuation
            let normalized = tokens.replace(" (", "(").replace(", ", ",");
            if let Some(expr) = parse_cfg_attr(&normalized) {
                if !expr.eval(features) {
                    return true;
                }
            } else {
                eprintln!("  [cfg] UNPARSED: {}", tokens);
            }
        }
    }
    false
}

/// Parse a #[cfg(...)] attribute string into a CfgExpr.
/// Input format: `cfg(...)` where ... is the cfg expression.
fn parse_cfg_attr(s: &str) -> Option<CfgExpr> {
    let s = s.trim();
    // Strip outer "cfg(...)"
    let inner = s.strip_prefix("cfg(")?.strip_suffix(')')?;
    Some(parse_cfg_expr(inner.trim()))
}

/// Parse a cfg expression (the part inside cfg(...)).
fn parse_cfg_expr(s: &str) -> CfgExpr {
    let s = s.trim();

    // feature = "name"
    if let Some(rest) = s.strip_prefix("feature") {
        let rest = rest.trim().strip_prefix('=').unwrap_or(rest).trim();
        let name = rest.trim_matches('"').trim_matches('\'');
        return CfgExpr::Feature(name.to_string());
    }

    // not(...)
    if let Some(inner) = s.strip_prefix("not(") {
        let inner = inner.strip_suffix(')').unwrap_or(inner);
        return CfgExpr::Not(Box::new(parse_cfg_expr(inner)));
    }

    // any(...) or all(...)
    if let Some(inner) = s.strip_prefix("any(") {
        let inner = inner.strip_suffix(')').unwrap_or(inner);
        let parts = split_cfg_args(inner);
        return CfgExpr::Any(parts.into_iter().map(|p| parse_cfg_expr(p)).collect());
    }
    if let Some(inner) = s.strip_prefix("all(") {
        let inner = inner.strip_suffix(')').unwrap_or(inner);
        let parts = split_cfg_args(inner);
        return CfgExpr::All(parts.into_iter().map(|p| parse_cfg_expr(p)).collect());
    }

    // Bare predicate: test, debug_assertions, unix, windows, etc.
    CfgExpr::Predicate(s.to_string())
}

/// Split comma-separated cfg arguments, respecting nested parentheses.
fn split_cfg_args(s: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                let arg = s[start..i].trim();
                if !arg.is_empty() {
                    args.push(arg);
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = s[start..].trim();
    if !last.is_empty() {
        args.push(last);
    }
    args
}

use quote::ToTokens;

#[cfg(test)]
mod tests {
    use super::*;

    fn features(enabled: &[&str]) -> CfgFeatures {
        CfgFeatures::new(enabled.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn test_simple_feature() {
        let f = features(&["singlethread"]);
        let expr = parse_cfg_attr("cfg(feature = \"singlethread\")").unwrap();
        assert!(expr.eval(&f));

        let expr = parse_cfg_attr("cfg(feature = \"multithread\")").unwrap();
        assert!(!expr.eval(&f));
    }

    #[test]
    fn test_not() {
        let f = features(&["singlethread"]);
        let expr = parse_cfg_attr("cfg(not(feature = \"multithread\"))").unwrap();
        assert!(expr.eval(&f));

        let expr = parse_cfg_attr("cfg(not(feature = \"singlethread\"))").unwrap();
        assert!(!expr.eval(&f));
    }

    #[test]
    fn test_any() {
        let f = features(&["singlethread"]);
        // any(feature = "singlethread", not(feature = "multithread")) → true
        let expr = parse_cfg_attr("cfg(any(feature = \"singlethread\", not(feature = \"multithread\")))").unwrap();
        assert!(expr.eval(&f));
    }

    #[test]
    fn test_all() {
        let f = features(&["singlethread"]);
        // all(feature = "multithread", not(feature = "singlethread")) → false
        let expr = parse_cfg_attr("cfg(all(feature = \"multithread\", not(feature = \"singlethread\")))").unwrap();
        assert!(!expr.eval(&f));
    }

    #[test]
    fn test_context_rs_patterns() {
        let f = features(&["singlethread"]);

        // The singlethread branch: any(feature = "singlethread", not(feature = "multithread"))
        let expr = parse_cfg_attr("cfg(any(feature = \"singlethread\", not(feature = \"multithread\")))").unwrap();
        assert!(expr.eval(&f), "singlethread branch should be included");

        // The multithread branch: all(feature = "multithread", not(feature = "singlethread"))
        let expr = parse_cfg_attr("cfg(all(feature = \"multithread\", not(feature = \"singlethread\")))").unwrap();
        assert!(!expr.eval(&f), "multithread branch should be excluded");
    }

    #[test]
    fn test_wasm_excluded() {
        let f = features(&["singlethread"]);
        let expr = parse_cfg_attr("cfg(feature = \"wasm\")").unwrap();
        assert!(!expr.eval(&f));
    }

    #[test]
    fn test_bare_predicates() {
        let f = features(&[]);
        let expr = parse_cfg_attr("cfg(test)").unwrap();
        assert!(!expr.eval(&f));

        let expr = parse_cfg_attr("cfg(debug_assertions)").unwrap();
        assert!(!expr.eval(&f));
    }

    #[test]
    fn test_no_features_multithread_excluded() {
        // With no features enabled, singlethread is the default behavior
        let f = features(&[]);

        // any(feature = "singlethread", not(feature = "multithread"))
        // → false OR true → true (singlethread is default when neither is set)
        let expr = parse_cfg_attr("cfg(any(feature = \"singlethread\", not(feature = \"multithread\")))").unwrap();
        assert!(expr.eval(&f));

        // all(feature = "multithread", not(feature = "singlethread"))
        // → false AND true → false
        let expr = parse_cfg_attr("cfg(all(feature = \"multithread\", not(feature = \"singlethread\")))").unwrap();
        assert!(!expr.eval(&f));
    }
}
