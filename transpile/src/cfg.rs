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

use quote::ToTokens;
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
    /// Every feature this crate's own `Cargo.toml` declares, implicit ones
    /// included. A `#[cfg(feature = "x")]` naming something absent from this
    /// set is not FALSE — it is a question nothing in the corpus can answer,
    /// which is what an undecided predicate means everywhere else. `None` here
    /// is "nobody said", which is what a unit test that only lists the enabled
    /// features wants.
    declared: Option<HashSet<String>>,
}

impl CfgFeatures {
    /// The feature set alone. Target and profile predicates stay undecided,
    /// which is what a unit test that only asks about features wants.
    pub fn new(enabled: Vec<String>) -> Self {
        CfgFeatures {
            enabled: enabled.into_iter().collect(),
            key_values: BTreeMap::new(),
            flags: BTreeMap::new(),
            declared: None,
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

    /// The same set, with `test` ON.
    ///
    /// D6: inside a `#[cfg(test)] mod tests`, the compiler is already building
    /// with `test` true, so a `#[cfg(test)]` on an item there keeps it. Walking
    /// the module with the crate's own set — where `test` is false — dropped
    /// every one of those items, and the emitted suite then called a helper
    /// nothing declared.
    pub fn under_test(&self) -> CfgFeatures {
        let mut under = self.clone();
        under.flags.insert("test".to_string(), true);
        under
    }

    /// The features the crate's own `Cargo.toml` declares.
    pub fn with_declared(mut self, declared: Vec<String>) -> Self {
        self.declared = Some(declared.into_iter().collect());
        self
    }

    /// Is this feature on in the port's build? Read by the config's own tests,
    /// which check that the resolved set is the one Cargo would resolve;
    /// evaluation goes through `answer_feature`, which can also say "nobody
    /// decided".
    #[cfg(test)]
    pub fn is_enabled(&self, feature: &str) -> bool {
        self.enabled.contains(feature)
    }

    /// What this build says about a feature: on, off, or a question nothing
    /// answers.
    ///
    /// A feature name used to be the ONE predicate that could never be
    /// undecided — `is_enabled` answered false for everything the config did
    /// not list, so `#[cfg(feature = "never-declared")]` dropped its item in
    /// silence. It can now be undecided for the reason every other predicate
    /// can: the crate does not declare it, so nothing in the corpus says what
    /// Cargo would resolve it to, and a typo in either the source or
    /// `[features.<crate>]` reads exactly like a feature that is off.
    fn answer_feature(&self, name: &str) -> Answer {
        match &self.declared {
            Some(declared) if !declared.contains(name) => None,
            _ => Some(self.enabled.contains(name)),
        }
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
                let answer = cfg.answer_feature(name);
                record(format!("feature = \"{}\"", name), answer);
                answer
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

/// `#[cfg_attr(P, a, b)]` as the attributes it really is.
///
/// Rust reads it as `#[a] #[b]` where `P` holds and as nothing where it does
/// not. Ignoring it entirely gave the right answer for every corpus site, whose
/// predicates are all features the port turns off — and the wrong answer for
/// any site whose predicate is TRUE, where a derive, a serde rename or a
/// `#[test]` would have been dropped in silence.
///
/// A predicate nothing decides leaves the attributes out, which is what the
/// port did before; `undecided` collects those so the caller can say so.
pub fn expand_cfg_attrs(
    attrs: &[syn::Attribute],
    cfg: &CfgFeatures,
    undecided: &mut Vec<String>,
) -> Vec<syn::Attribute> {
    let mut out = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("cfg_attr") {
            out.push(attr.clone());
            continue;
        }
        let Ok(list) = attr.meta.require_list() else {
            out.push(attr.clone());
            continue;
        };
        let Ok(parts) = list.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            undecided.push(attr.meta.to_token_stream().to_string());
            continue;
        };
        let mut parts = parts.into_iter();
        let Some(predicate) = parts.next() else { continue };
        let text = format!("cfg({})", predicate.to_token_stream());
        let normalized = text.replace(" (", "(").replace(", ", ",");
        match parse_cfg_attr(&normalized).and_then(|expr| expr.eval(cfg)) {
            Some(true) => out.extend(parts.map(|meta| syn::Attribute {
                pound_token: attr.pound_token,
                style: attr.style,
                bracket_token: attr.bracket_token,
                meta,
            })),
            Some(false) => {}
            None => undecided.push(attr.meta.to_token_stream().to_string()),
        }
    }
    out
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

/// Decide the `#[cfg]` written INSIDE a body: on a statement, on a `let`, on a
/// match arm, on a field of a struct literal, and on an item declared in a
/// block.
///
/// Rust drops these exactly as it drops a top-level item, and the emitter used
/// to carry every one of them into the output unevaluated. Two `let`s written
/// as `#[cfg(debug_assertions)]` and `#[cfg(not(debug_assertions))]` both came
/// out, the shadowing rename gave the second a fresh name so the file still
/// compiled, and the scanner then read the release branch — inverting the
/// `debug_assertions = true` ruling in the one place it was made for
/// (`storage/indexeddb-wasm/src/collection.rs:345,352`).
///
/// The block is pruned in place before anything translates it, so nothing
/// downstream has to ask again.
pub fn prune_block(block: &mut syn::Block, cfg: &CfgFeatures) {
    let mut pruner = Pruner { cfg };
    syn::visit_mut::VisitMut::visit_block_mut(&mut pruner, block);
}

struct Pruner<'a> {
    cfg: &'a CfgFeatures,
}

impl Pruner<'_> {
    /// Is what carries these attributes in this build? An undecided predicate
    /// keeps it and says so, exactly as it does for a top-level item.
    fn keeps(&self, attrs: &[syn::Attribute], span: proc_macro2::Span) -> bool {
        match gate(attrs, self.cfg) {
            Gate::Keep => true,
            Gate::Skip => false,
            Gate::Undecided(text) => {
                crate::extract::report_undecided_cfg(span, &text);
                true
            }
        }
    }
}

impl syn::visit_mut::VisitMut for Pruner<'_> {
    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        block.stmts.retain(|stmt| {
            let span = syn::spanned::Spanned::span(stmt);
            match stmt {
                syn::Stmt::Local(local) => self.keeps(&local.attrs, span),
                syn::Stmt::Item(item) => self.keeps(item_attrs(item), span),
                syn::Stmt::Expr(expr, _) => self.keeps(expr_attrs(expr), span),
                syn::Stmt::Macro(mac) => self.keeps(&mac.attrs, span),
            }
        });
        syn::visit_mut::visit_block_mut(self, block);
    }

    fn visit_expr_match_mut(&mut self, node: &mut syn::ExprMatch) {
        node.arms
            .retain(|arm| self.keeps(&arm.attrs, syn::spanned::Spanned::span(arm)));
        syn::visit_mut::visit_expr_match_mut(self, node);
    }

    fn visit_expr_struct_mut(&mut self, node: &mut syn::ExprStruct) {
        let kept: syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma> = node
            .fields
            .iter()
            .filter(|field| self.keeps(&field.attrs, syn::spanned::Spanned::span(field)))
            .cloned()
            .collect();
        node.fields = kept;
        syn::visit_mut::visit_expr_struct_mut(self, node);
    }
}

/// The attributes on any expression form. `syn` gives every one an `attrs`
/// field and no trait to read it through.
fn expr_attrs(expr: &syn::Expr) -> &[syn::Attribute] {
    macro_rules! arms {
        ($($variant:ident),* $(,)?) => {
            match expr {
                $(syn::Expr::$variant(e) => &e.attrs,)*
                _ => &[],
            }
        };
    }
    arms!(
        Array, Assign, Async, Await, Binary, Block, Break, Call, Cast, Closure, Const, Continue,
        Field, ForLoop, Group, If, Index, Infer, Let, Lit, Loop, Macro, Match, MethodCall, Paren,
        Path, Range, Reference, Repeat, Return, Struct, Try, TryBlock, Tuple, Unary, Unsafe, While,
        Yield,
    )
}

/// The attributes on any item form, for an item declared inside a block.
fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    macro_rules! arms {
        ($($variant:ident),* $(,)?) => {
            match item {
                $(syn::Item::$variant(i) => &i.attrs,)*
                _ => &[],
            }
        };
    }
    arms!(
        Const, Enum, ExternCrate, Fn, ForeignMod, Impl, Macro, Mod, Static, Struct, Trait,
        TraitAlias, Type, Union, Use,
    )
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

    /// R6: a `#[cfg]` written anywhere an attribute can sit is evaluated.
    /// Before this, only a top-level item was asked, so both branches of the
    /// indexeddb prefix guard were emitted and the release one was read.
    #[test]
    fn a_cfg_on_a_statement_decides_whether_the_statement_is_in_this_build() {
        let f = port_build(&[]);
        let mut block: syn::Block = syn::parse_quote!({
            #[cfg(debug_assertions)]
            let n = 1;
            #[cfg(not(debug_assertions))]
            let n = 2;
            n
        });
        prune_block(&mut block, &f);
        assert_eq!(block.stmts.len(), 2, "one `let` and the tail");
        let written = quote::ToTokens::to_token_stream(&block).to_string();
        assert!(written.contains("let n = 1"), "{written}");
        assert!(!written.contains("let n = 2"), "{written}");
    }

    /// A match arm and a struct-literal field carry `#[cfg]` too:
    /// `signals/src/react.rs:98` builds an `Inner` whose `name` field exists
    /// only in a debug build.
    #[test]
    fn a_cfg_on_a_match_arm_and_on_a_literal_field_decides_them_too() {
        let f = port_build(&[]);
        let mut block: syn::Block = syn::parse_quote!({
            let inner = Inner {
                #[cfg(debug_assertions)]
                name: OnceLock::new(),
                #[cfg(not(debug_assertions))]
                shadow: 0,
                version,
            };
            match x {
                #[cfg(not(debug_assertions))]
                A => 1,
                B => 2,
            }
        });
        prune_block(&mut block, &f);
        let written = quote::ToTokens::to_token_stream(&block).to_string();
        assert!(written.contains("name :"), "{written}");
        assert!(!written.contains("shadow"), "{written}");
        assert!(!written.contains("A =>"), "{written}");
        assert!(written.contains("B =>"), "{written}");
    }

    /// A feature the crate does not declare is NOT false. It used to be the one
    /// predicate that could never be undecided, so a typo — in the source or in
    /// `[features.<crate>]` — dropped its item in silence.
    #[test]
    fn a_feature_the_crate_never_declares_is_undecided_not_false() {
        let declared = port_build(&["wasm"]).with_declared(vec![
            "wasm".to_string(),
            "uniffi".to_string(),
        ]);
        assert_eq!(eval("cfg(feature = \"wasm\")", &declared), Some(true));
        assert_eq!(eval("cfg(feature = \"uniffi\")", &declared), Some(false));
        assert_eq!(eval("cfg(feature = \"never-declared\")", &declared), None);
        let attrs: Vec<syn::Attribute> = syn::parse_quote!(#[cfg(feature = "never-declared")]);
        match gate(&attrs, &declared) {
            Gate::Undecided(text) => assert!(text.contains("never-declared"), "{text}"),
            other => panic!("expected Undecided, got {other:?}"),
        }
        // With nothing declared, the question is answered from the enabled set
        // alone — which is what a unit fixture wants.
        assert_eq!(eval("cfg(feature = \"never-declared\")", &port_build(&[])), Some(false));
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

    /// D6: inside a `#[cfg(test)] mod tests` the compiler is already building
    /// with `test` true, so a `#[cfg(test)]` on an item there KEEPS it. Walked
    /// with the crate's own set, every such item was dropped and the emitted
    /// suite called a helper nothing declared.
    #[test]
    fn a_cfg_test_item_is_kept_inside_a_test_module() {
        let outside = port_build(&[]);
        let item: syn::ItemFn = syn::parse_str("#[cfg(test)] fn helper() -> u32 { 1 }").unwrap();
        assert!(should_skip(&item.attrs, &outside), "outside, it is not in the build");
        assert!(
            !should_skip(&item.attrs, &outside.under_test()),
            "inside a test module, it is"
        );
    }

    /// D8: `#[cfg_attr(P, ..)]` is the attributes it carries where `P` holds and
    /// nothing where it does not. Ignored entirely, a derive gated on a
    /// predicate that HOLDS was dropped in silence.
    #[test]
    fn a_cfg_attr_is_the_attributes_it_carries() {
        let cfg = port_build(&["singlethread"]);
        let mut undecided = Vec::new();

        let held: syn::ItemStruct =
            syn::parse_str("#[cfg_attr(feature = \"singlethread\", derive(Clone), serde(transparent))] struct S;")
                .unwrap();
        let expanded = expand_cfg_attrs(&held.attrs, &cfg, &mut undecided);
        let written: Vec<String> = expanded
            .iter()
            .map(|a| a.meta.to_token_stream().to_string().replace(" (", "("))
            .collect();
        assert_eq!(written, vec!["derive(Clone)", "serde(transparent)"]);
        assert!(undecided.is_empty(), "{undecided:?}");

        let absent: syn::ItemStruct =
            syn::parse_str("#[cfg_attr(feature = \"uniffi\", derive(Clone))] struct S;").unwrap();
        assert!(expand_cfg_attrs(&absent.attrs, &cfg, &mut undecided).is_empty());
        assert!(undecided.is_empty(), "a decided predicate is not undecided");

        // A predicate nothing decides leaves the attributes out AND says so.
        let unknown: syn::ItemStruct =
            syn::parse_str("#[cfg_attr(target_os = \"redox\", derive(Clone))] struct S;").unwrap();
        assert!(expand_cfg_attrs(&unknown.attrs, &cfg, &mut undecided).is_empty());
        assert_eq!(undecided.len(), 1, "{undecided:?}");

        // Everything that is not a `cfg_attr` travels through untouched.
        let plain: syn::ItemStruct = syn::parse_str("#[derive(Debug)] struct S;").unwrap();
        assert_eq!(expand_cfg_attrs(&plain.attrs, &cfg, &mut undecided).len(), 1);
    }
}

