//! Conditional compilation — `#[cfg(...)]` parsing and evaluation.
//!
//! Rust decides `#[cfg]` from the build: the target triple, the profile, and
//! the feature set Cargo resolved for that build. The port has one build — the
//! browser/Expo one, which is `ankurah` with its `wasm` feature — so the answers
//! are fixed, and they live in `transpile.toml` rather than in this file:
//! `[cfg]` names the target and profile predicates, `[features.<crate>]` names
//! Cargo's resolved feature set for that crate, and `[[feature_overrides]]`
//! records the one class of deliberate departure from it.
//!
//! A predicate nothing decides is a diagnostic, and the item it gates is KEPT.
//! Dropping it would be a silent hole: every type that file declares would go
//! missing, and the report would be dozens of unrelated "no declaration for"
//! lines somewhere else. Keeping it puts the failure where the cause is.
//!
//! Every predicate evaluated is recorded (`decisions()`), so a run can say what
//! it decided and how, which is what the crate inventory pins.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};

/// What a `#[cfg]` is evaluated against: the resolved feature set for one
/// crate, plus the target and profile predicates, which are the same for every
/// crate in one build.
#[derive(Debug, Clone, Default)]
pub struct CfgFeatures {
    /// Features Cargo resolves ON for this crate in the port's build, minus any
    /// recorded override.
    enabled: HashSet<String>,
    /// Name-value predicates that are not features: `target_arch = "wasm32"`,
    /// `target_os`, `target_family`. A name absent here is undecided.
    key_values: BTreeMap<String, String>,
    /// Bare predicates: `debug_assertions`, `test`, `unix`. A name absent here
    /// is undecided.
    flags: BTreeMap<String, bool>,
}

impl CfgFeatures {
    /// The feature set alone. Target and profile predicates stay undecided,
    /// which is what a unit test that only asks about features wants.
    pub fn new(enabled: Vec<String>) -> Self {
        CfgFeatures {
            enabled: enabled.into_iter().collect(),
            key_values: BTreeMap::new(),
            flags: BTreeMap::new(),
        }
    }

    pub fn with_key_values(mut self, kvs: BTreeMap<String, String>) -> Self {
        self.key_values = kvs;
        self
    }

    pub fn with_flags(mut self, flags: BTreeMap<String, bool>) -> Self {
        self.flags = flags;
        self
    }

    /// Check if a feature is enabled
    pub fn is_enabled(&self, feature: &str) -> bool {
        self.enabled.contains(feature)
    }

    /// Read by this module's own tests, which check that the resolved set is
    /// the one the config wrote.
    #[cfg(test)]
    pub fn enabled_names(&self) -> std::collections::BTreeSet<&str> {
        self.enabled.iter().map(|s| s.as_str()).collect()
    }
}

/// Parsed cfg expression
#[derive(Debug, Clone, PartialEq)]
enum CfgExpr {
    /// `cfg(feature = "name")`
    Feature(String),
    /// `cfg(not(...))`
    Not(Box<CfgExpr>),
    /// `cfg(any(..., ...))`
    Any(Vec<CfgExpr>),
    /// `cfg(all(..., ...))`
    All(Vec<CfgExpr>),
    /// `cfg(target_arch = "wasm32")` and every other name-value predicate that
    /// is not `feature`.
    KeyValue(String, String),
    /// `cfg(test)`, `cfg(debug_assertions)`, `cfg(unix)`.
    Flag(String),
}

/// What one predicate answered, and why. `None` means nothing decided it.
type Answer = Option<bool>;

impl CfgExpr {
    /// Evaluate against the configuration. `None` propagates: `all(..)` with an
    /// undecided operand is undecided unless another operand is already false,
    /// and `any(..)` likewise unless another is already true — the same
    /// short-circuit Rust's own evaluation has, so an undecided predicate that
    /// cannot change the answer never gets reported.
    fn eval(&self, cfg: &CfgFeatures) -> Answer {
        match self {
            CfgExpr::Feature(name) => {
                record(format!("feature = \"{}\"", name), Some(cfg.is_enabled(name)));
                Some(cfg.is_enabled(name))
            }
            CfgExpr::Not(inner) => inner.eval(cfg).map(|v| !v),
            CfgExpr::Any(exprs) => {
                let answers: Vec<Answer> = exprs.iter().map(|e| e.eval(cfg)).collect();
                if answers.iter().any(|a| *a == Some(true)) {
                    Some(true)
                } else if answers.iter().all(|a| *a == Some(false)) {
                    Some(false)
                } else {
                    None
                }
            }
            CfgExpr::All(exprs) => {
                let answers: Vec<Answer> = exprs.iter().map(|e| e.eval(cfg)).collect();
                if answers.iter().any(|a| *a == Some(false)) {
                    Some(false)
                } else if answers.iter().all(|a| *a == Some(true)) {
                    Some(true)
                } else {
                    None
                }
            }
            CfgExpr::KeyValue(name, value) => {
                let answer = cfg.key_values.get(name).map(|decided| decided == value);
                record(format!("{} = \"{}\"", name, value), answer);
                answer
            }
            CfgExpr::Flag(name) => {
                let answer = cfg.flags.get(name).copied();
                record(name.clone(), answer);
                answer
            }
        }
    }
}

/// What an item's `#[cfg]` attributes say about whether it is in this build.
#[derive(Debug, Clone, PartialEq)]
pub enum Gate {
    /// No `#[cfg]`, or every one of them true.
    Keep,
    /// A `#[cfg]` evaluated false: the item is not in this build at all.
    Skip,
    /// A predicate nothing decides. The item is kept and the text of the
    /// undecided cfg comes back so the caller can report it at the item.
    Undecided(String),
}

/// Decide whether an item with these attributes is part of the port's build.
pub fn gate(attrs: &[syn::Attribute], cfg: &CfgFeatures) -> Gate {
    let mut undecided = None;
    for attr in attrs {
        if !attr.path().is_ident("cfg") {
            continue;
        }
        let tokens = attr.meta.to_token_stream().to_string();
        // Normalize spaces: syn's tokenizer puts them around punctuation.
        let normalized = tokens.replace(" (", "(").replace(", ", ",");
        match parse_cfg_attr(&normalized) {
            Some(expr) => match expr.eval(cfg) {
                Some(false) => return Gate::Skip,
                Some(true) => {}
                None => {
                    undecided.get_or_insert_with(|| tokens.clone());
                }
            },
            None => {
                undecided.get_or_insert_with(|| tokens.clone());
            }
        };
    }
    match undecided {
        Some(text) => Gate::Undecided(text),
        None => Gate::Keep,
    }
}

/// Check if an item with the given attributes should be skipped (cfg evaluates
/// to false). An undecided predicate keeps the item; `gate` is the form that
/// reports it.
#[cfg(test)]
pub fn should_skip(attrs: &[syn::Attribute], features: &CfgFeatures) -> bool {
    gate(attrs, features) == Gate::Skip
}

/// Parse a `#[cfg(...)]` attribute string into a CfgExpr.
fn parse_cfg_attr(s: &str) -> Option<CfgExpr> {
    let s = s.trim();
    let inner = s.strip_prefix("cfg(")?.strip_suffix(')')?;
    Some(parse_cfg_expr(inner.trim()))
}

/// Parse a cfg expression (the part inside `cfg(...)`).
fn parse_cfg_expr(s: &str) -> CfgExpr {
    let s = s.trim();

    // not(...)
    if let Some(inner) = s.strip_prefix("not(") {
        let inner = inner.strip_suffix(')').unwrap_or(inner);
        return CfgExpr::Not(Box::new(parse_cfg_expr(inner)));
    }

    // any(...) or all(...)
    if let Some(inner) = s.strip_prefix("any(") {
        let inner = inner.strip_suffix(')').unwrap_or(inner);
        let parts = split_cfg_args(inner);
        return CfgExpr::Any(parts.into_iter().map(parse_cfg_expr).collect());
    }
    if let Some(inner) = s.strip_prefix("all(") {
        let inner = inner.strip_suffix(')').unwrap_or(inner);
        let parts = split_cfg_args(inner);
        return CfgExpr::All(parts.into_iter().map(parse_cfg_expr).collect());
    }

    // `<name> = "<value>"`, of which `feature` is one name among several.
    if let Some((name, value)) = s.split_once('=') {
        let name = name.trim().to_string();
        let value = value.trim().trim_matches('"').trim_matches('\'').to_string();
        if name == "feature" {
            return CfgExpr::Feature(value);
        }
        return CfgExpr::KeyValue(name, value);
    }

    CfgExpr::Flag(s.to_string())
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

thread_local! {
    /// Every predicate this run evaluated, and what it answered. The inventory
    /// pins "every cfg the corpus writes is decided"; this is how a run says so.
    static DECISIONS: RefCell<BTreeMap<String, (Answer, usize)>> =
        RefCell::new(BTreeMap::new());
}

fn record(predicate: String, answer: Answer) {
    DECISIONS.with(|d| {
        let mut d = d.borrow_mut();
        let entry = d.entry(predicate).or_insert((answer, 0));
        entry.1 += 1;
        // A predicate answers the same way every time within a run; if it did
        // not, the later answer is the one a reader would be surprised by.
        entry.0 = answer;
    });
}

/// The predicates this run evaluated: text, answer, and how many sites asked.
pub fn decisions() -> Vec<(String, Answer, usize)> {
    DECISIONS.with(|d| {
        d.borrow()
            .iter()
            .map(|(k, (a, n))| (k.clone(), *a, *n))
            .collect()
    })
}

use quote::ToTokens;

#[cfg(test)]
mod tests {
    use super::*;

    fn features(enabled: &[&str]) -> CfgFeatures {
        CfgFeatures::new(enabled.iter().map(|s| s.to_string()).collect())
    }

    fn port_build(enabled: &[&str]) -> CfgFeatures {
        features(enabled)
            .with_key_values(BTreeMap::from([(
                "target_arch".to_string(),
                "wasm32".to_string(),
            )]))
            .with_flags(BTreeMap::from([
                ("debug_assertions".to_string(), true),
                ("test".to_string(), false),
            ]))
    }

    fn eval(src: &str, cfg: &CfgFeatures) -> Answer {
        parse_cfg_attr(src).unwrap().eval(cfg)
    }

    #[test]
    fn test_simple_feature() {
        let f = features(&["singlethread"]);
        assert_eq!(eval("cfg(feature = \"singlethread\")", &f), Some(true));
        assert_eq!(eval("cfg(feature = \"multithread\")", &f), Some(false));
    }

    #[test]
    fn test_not() {
        let f = features(&["singlethread"]);
        assert_eq!(eval("cfg(not(feature = \"multithread\"))", &f), Some(true));
        assert_eq!(eval("cfg(not(feature = \"singlethread\"))", &f), Some(false));
    }

    #[test]
    fn test_any_and_all() {
        let f = features(&["singlethread"]);
        assert_eq!(
            eval(
                "cfg(any(feature = \"singlethread\", not(feature = \"multithread\")))",
                &f
            ),
            Some(true)
        );
        assert_eq!(
            eval(
                "cfg(all(feature = \"multithread\", not(feature = \"singlethread\")))",
                &f
            ),
            Some(false)
        );
    }

    #[test]
    fn target_arch_is_wasm32_in_the_port_build() {
        let f = port_build(&["wasm"]);
        assert_eq!(eval("cfg(target_arch = \"wasm32\")", &f), Some(true));
        assert_eq!(eval("cfg(not(target_arch = \"wasm32\"))", &f), Some(false));
        // core/src/task.rs: the spawn_local arm is in, the tokio arm is out.
    }

    #[test]
    fn debug_assertions_is_true_so_the_prefix_guard_survives() {
        let f = port_build(&[]);
        assert_eq!(eval("cfg(debug_assertions)", &f), Some(true));
        assert_eq!(eval("cfg(not(debug_assertions))", &f), Some(false));
    }

    #[test]
    fn an_undecided_predicate_keeps_the_item_and_names_itself() {
        let f = features(&["wasm"]);
        // Nothing declares `unix`, so nothing decides this.
        let attrs: Vec<syn::Attribute> = syn::parse_quote!(#[cfg(unix)]);
        match gate(&attrs, &f) {
            Gate::Undecided(text) => assert!(text.contains("unix"), "{text}"),
            other => panic!("expected Undecided, got {other:?}"),
        }
        assert!(!should_skip(&attrs, &f), "an undecided cfg keeps its item");
    }

    #[test]
    fn an_undecided_operand_that_cannot_change_the_answer_is_not_undecided() {
        let f = features(&[]);
        // `all(feature = "off", unix)` is false whatever `unix` says.
        let attrs: Vec<syn::Attribute> = syn::parse_quote!(#[cfg(all(feature = "off", unix))]);
        assert_eq!(gate(&attrs, &f), Gate::Skip);
    }

    #[test]
    fn test_wasm_is_on_in_the_port_build() {
        let f = port_build(&["wasm"]);
        assert_eq!(eval("cfg(feature = \"wasm\")", &f), Some(true));
    }

    #[test]
    fn test_context_rs_patterns() {
        let f = features(&["singlethread"]);
        assert_eq!(
            eval(
                "cfg(any(feature = \"singlethread\", not(feature = \"multithread\")))",
                &f
            ),
            Some(true),
            "singlethread branch should be included"
        );
        assert_eq!(
            eval(
                "cfg(all(feature = \"multithread\", not(feature = \"singlethread\")))",
                &f
            ),
            Some(false),
            "multithread branch should be excluded"
        );
    }

    #[test]
    fn decisions_records_what_was_asked() {
        let f = port_build(&["tokio"]);
        let _ = eval("cfg(feature = \"tokio\")", &f);
        let _ = eval("cfg(feature = \"tokio\")", &f);
        let rows = decisions();
        let row = rows
            .iter()
            .find(|(p, _, _)| p == "feature = \"tokio\"")
            .expect("the predicate was recorded");
        assert_eq!(row.1, Some(true));
        assert!(row.2 >= 2, "both sites counted: {}", row.2);
    }
}
