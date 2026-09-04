//! rust-analyzer's answers about the corpus, as checked-in data (spec 6.3).
//!
//! On 2026-09-02 a throwaway spike ran rust-analyzer over the support checkout
//! and wrote down, for a sample of sites, the receiver type it inferred, the
//! method it resolved the call to, the deref and borrow steps it inserted, the
//! type it gave each closure's return, and the error conversion each `?`
//! performs. `tests/oracle/*.json` is that text converted to JSON; the schema is
//! `tests/oracle/SCHEMA.md`.
//!
//! Nothing from rust-analyzer is a dependency of the transpiler. This is data in
//! the repository, regenerated only deliberately by rebuilding the out-of-tree
//! spike on nightly and re-running `tests/oracle/convert_spike_txt.py`.

mod common;

use common::transpile_dir;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct Sites<T> {
    /// Which spike output file the records came from.
    pub source: String,
    pub sites: Vec<T>,
}

/// One method call: what rust-analyzer thought the receiver was, what it
/// adjusted the receiver to, and which function the call landed on.
#[derive(Deserialize)]
pub struct MethodCall {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
    pub receiver_type: String,
    pub receiver_type_adjusted: String,
    pub callee: String,
    pub callee_kind: String,
    pub result_type: String,
}

/// The deref and borrow steps inserted at one site, in order.
#[derive(Deserialize)]
pub struct AdjustmentChain {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
    pub steps: Vec<AdjustmentStep>,
}

#[derive(Deserialize)]
pub struct AdjustmentStep {
    /// rust-analyzer's own word for the step, e.g. `Deref(None)`,
    /// `Borrow(Ref(Mut))`, `Pointer(Unsize)`.
    pub adjustment: String,
    pub from: String,
    pub to: String,
}

/// A closure with the type rust-analyzer gave it and the return it inferred.
#[derive(Deserialize)]
pub struct ClosureType {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
    pub closure_type: String,
    pub inferred_return: String,
}

/// A `?` whose error type had to be converted, and the function that does it.
#[derive(Deserialize)]
pub struct TryConversion {
    pub file: String,
    pub line: u32,
    pub from_error: String,
    pub to_error: String,
    pub conversion: String,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
}

/// A call that resolved to a trait's own declaration rather than to an impl:
/// blanket or generic dispatch, the case a syntactic transpiler cannot guess.
#[derive(Deserialize)]
pub struct TraitGenericCall {
    pub file: String,
    pub line: u32,
    pub callee: String,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
}

/// A receiver that went through a user or std `Deref` impl before the call.
#[derive(Deserialize)]
pub struct OverloadedDeref {
    pub file: String,
    pub line: u32,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
    pub steps: Vec<DerefStep>,
}

#[derive(Deserialize)]
pub struct DerefStep {
    pub from: String,
    pub to: String,
}

/// A closure and the return type rust-analyzer inferred for it, from the
/// whole-corpus pass (no column, unlike `ClosureType`).
#[derive(Deserialize)]
pub struct ClosureReturn {
    pub file: String,
    pub line: u32,
    pub return_type: String,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
}

/// An expression rust-analyzer could not type. Present so the gap is visible;
/// these sites are covered by the corpus inventory test instead.
#[derive(Deserialize)]
pub struct UntypedExpr {
    pub file: String,
    pub line: u32,
    pub expr: String,
}

#[derive(Deserialize)]
pub struct CrateCounts {
    pub source: String,
    pub crates: BTreeMap<String, BTreeMap<String, u64>>,
}

pub fn load<T: serde::de::DeserializeOwned>(name: &str) -> Sites<T> {
    let path = transpile_dir().join("tests/oracle").join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} does not match the schema: {e}", path.display()))
}

pub fn load_crate_counts() -> CrateCounts {
    let path = transpile_dir().join("tests/oracle/crate_counts.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} does not match the schema: {e}", path.display()))
}

/// The oracle files are hand-converted text, so this guards the conversion:
/// every file parses into the structs above and holds the number of records the
/// spike produced. A count that moves means somebody regenerated the oracle,
/// which is a reviewable event.
#[test]
fn oracle_loads() {
    assert_eq!(load::<MethodCall>("method_calls.json").sites.len(), 14);
    assert_eq!(load::<AdjustmentChain>("adjustment_chains.json").sites.len(), 6);
    assert_eq!(load::<ClosureType>("closure_types.json").sites.len(), 6);
    // The spike's `?` sites all came from core/src/context.rs, which sits under
    // `#[async_trait]` and stayed untyped, so this file is deliberately empty.
    assert_eq!(load::<TryConversion>("try_sites.json").sites.len(), 0);

    assert_eq!(load::<TraitGenericCall>("trait_generic_calls.json").sites.len(), 76);
    assert_eq!(load::<OverloadedDeref>("overloaded_derefs.json").sites.len(), 27);
    assert_eq!(load::<ClosureReturn>("closure_returns.json").sites.len(), 52);
    assert_eq!(load::<TryConversion>("try_conversions.json").sites.len(), 57);
    assert_eq!(load::<UntypedExpr>("untyped_expressions.json").sites.len(), 188);

    let counts = load_crate_counts();
    let names: Vec<&str> = counts.crates.keys().map(|s| s.as_str()).collect();
    assert_eq!(names, ["ankql", "core", "proto", "signals"]);
    assert_eq!(counts.crates["ankql"]["method_calls"], 367);
}

/// One site the engine is expected to keep resolving.
#[derive(Deserialize)]
struct Pinned {
    file: String,
    line: u32,
    method: String,
    /// Set where the port declares a different signature from Rust's, so the
    /// two result types are not expected to agree.
    #[serde(default)]
    result_differs: Option<String>,
}

#[derive(Deserialize)]
struct PinnedSites {
    site: Vec<Pinned>,
}

fn pinned_sites() -> Vec<Pinned> {
    let path = transpile_dir().join("tests/oracle/covered_sites.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let parsed: PinnedSites = toml::from_str(&text)
        .unwrap_or_else(|e| panic!("{} does not match the schema: {e}", path.display()));
    parsed.site
}

/// The engine's answers, against rust-analyzer's, on the sites both cover.
///
/// The two do not name types the same way — rust-analyzer writes
/// `Vec<EventId, Global>` and `&mut DebugStruct<'_, '_>`, the engine writes
/// `Vec<EventId>` — so the comparison is on the shape that decides the emitted
/// TypeScript: the borrow taken, the outermost type constructor of the adjusted
/// receiver, which function was selected including the trait it came through,
/// the result type, and the ordered chain of dereferences.
///
/// Every site in `covered_sites.toml` must resolve. A site the engine does not
/// cover *and* is not pinned is a gap, printed by category, which is the
/// std-surface step's work list.
#[test]
fn engine_matches_oracle() {
    let engine = common::run_resolve("signals", "signals/src");
    let calls = load::<MethodCall>("method_calls.json");
    let pinned = pinned_sites();

    let mut mismatches = Vec::new();
    let mut uncovered: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut covered = 0usize;

    for site in &calls.sites {
        let method = callee_method(&site.callee);
        let pin = pinned
            .iter()
            .find(|p| p.file == site.file && p.line == site.line && p.method == method);
        let Some(row) = engine
            .iter()
            .find(|r| r.file == site.file && r.line == site.line && r.method == method)
        else {
            if pin.is_some() {
                mismatches.push(format!(
                    "{}:{} {}: pinned as covered, but the engine no longer resolves it",
                    site.file, site.line, method
                ));
            } else {
                uncovered
                    .entry(callee_owner(&site.callee))
                    .or_default()
                    .push(format!("{}:{} {}", site.file, site.line, site.callee));
            }
            continue;
        };
        covered += 1;

        // The receiver, after the dereferences and the borrow.
        if site.receiver_type_adjusted != "(none)"
            && shape(&site.receiver_type_adjusted) != shape(&row.adjusted)
        {
            mismatches.push(format!(
                "{}:{} {}: receiver is `{}`, rust-analyzer says `{}`",
                site.file, site.line, method, row.adjusted, site.receiver_type_adjusted
            ));
        }

        // Which function, including the trait it came through: selecting the
        // right receiver through the wrong trait is a different answer.
        if callee_identity(&site.callee, &site.callee_kind) != engine_callee_identity(&row.callee) {
            mismatches.push(format!(
                "{}:{} {}: callee is `{}`, rust-analyzer says `{}` ({})",
                site.file, site.line, method, row.callee, site.callee, site.callee_kind
            ));
        }

        // And what the call produced.
        match pin.and_then(|p| p.result_differs.as_ref()) {
            Some(_) => {}
            None if shape(&site.result_type) != shape(&row.result) => mismatches.push(format!(
                "{}:{} {}: result is `{}`, rust-analyzer says `{}`",
                site.file, site.line, method, row.result, site.result_type
            )),
            None => {}
        }
    }

    for pin in &pinned {
        let seen = calls.sites.iter().any(|s| {
            s.file == pin.file && s.line == pin.line && callee_method(&s.callee) == pin.method
        });
        if !seen {
            mismatches.push(format!(
                "{}:{} {}: pinned, but the oracle has no such site",
                pin.file, pin.line, pin.method
            ));
        }
    }

    // The deref chains, from the whole-corpus pass: the same steps, in the same
    // order, from and to the same types.
    let derefs = load::<OverloadedDeref>("overloaded_derefs.json");
    let ours: Vec<&OverloadedDeref> = derefs
        .sites
        .iter()
        .filter(|s| s.file.starts_with("signals/"))
        .collect();
    let mut deref_covered = 0usize;
    for site in &ours {
        let expected: Vec<(String, String)> = site
            .steps
            .iter()
            .map(|s| (head(&s.from), head(&s.to)))
            .collect();
        let matched = engine.iter().any(|r| {
            r.file == site.file && r.line == site.line && adjustment_chain(r) == expected
        });
        if matched {
            deref_covered += 1;
        } else {
            uncovered
                .entry("deref chain not reproduced".to_string())
                .or_default()
                .push(format!("{}:{} {}", site.file, site.line, site.expr));
        }
    }

    let mut report = String::new();
    for (category, sites) in &uncovered {
        report.push_str(&format!("\n  {} ({} sites)\n", category, sites.len()));
        for site in sites {
            report.push_str(&format!("    {}\n", site));
        }
    }
    eprintln!(
        "oracle: {} of {} method-call sites covered ({} pinned), {} of {} deref chains; \
         the rest are gaps, by what is missing:{}",
        covered,
        calls.sites.len(),
        pinned.len(),
        deref_covered,
        ours.len(),
        report
    );

    assert!(
        mismatches.is_empty(),
        "the engine disagrees with rust-analyzer on {} site(s):\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
    assert!(
        covered >= pinned.len(),
        "every pinned site has to be covered"
    );
}

/// The engine's whole adjustment chain at one site, in rust-analyzer's terms.
///
/// rust-analyzer writes the auto-ref as one more step — `HashMap` to `&mut
/// HashMap` — where the engine keeps the dereferences and the borrow apart.
/// They are the same chain, so the borrow is put back on the end before the two
/// are compared.
fn adjustment_chain(row: &common::Resolved) -> Vec<(String, String)> {
    let mut chain: Vec<(String, String)> =
        row.steps.iter().map(|(from, to)| (head(from), head(to))).collect();
    let last = chain
        .last()
        .map(|(_, to)| to.clone())
        .unwrap_or_else(|| head(&row.receiver));
    let (borrow, adjusted) = shape(&row.adjusted);
    if !borrow.is_empty() {
        chain.push((last, adjusted));
    }
    chain
}

/// `HashMap<K, V, S, A>::len` gives `len`.
fn callee_method(callee: &str) -> String {
    callee.rsplit("::").next().unwrap_or(callee).to_string()
}

/// `HashMap<K, V, S, A>::len` gives `HashMap<K, V, S, A>`; `<Foo as Bar>::baz`
/// gives `Foo`.
fn callee_owner(callee: &str) -> String {
    let owner = match callee.rfind("::") {
        Some(at) => &callee[..at],
        None => callee,
    };
    let owner = owner.trim_start_matches('<');
    match owner.find(" as ") {
        Some(at) => owner[..at].to_string(),
        None => owner.trim_end_matches('>').to_string(),
    }
}

/// Which function was selected, as a pair the two sides can both produce: the
/// trait it came through if it came through one, otherwise the type it is an
/// inherent method of, plus the method's name.
fn callee_identity(callee: &str, _kind: &str) -> (String, String) {
    // rust-analyzer writes a trait method as `Trait::method` and an inherent one
    // as `Type<..>::method`; either way the owner is what identifies it.
    (head(&callee_owner(callee)), callee_method(callee))
}

/// The engine writes a trait callee as `<Self as Trait>::method`.
fn engine_callee_identity(callee: &str) -> (String, String) {
    let name = callee_method(callee);
    let owner = match callee.rfind("::") {
        Some(at) => &callee[..at],
        None => callee,
    };
    if let Some(at) = owner.find(" as ") {
        let trait_name = owner[at + 4..].trim_end_matches('>');
        return (head(trait_name), name);
    }
    (head(owner.trim_start_matches('<')), name)
}

/// The borrow a type is behind and the constructor at its head, which is what
/// the emitted TypeScript turns on. Lifetimes and the allocator and hasher
/// parameters rust-analyzer prints are not part of that.
fn shape(ty: &str) -> (String, String) {
    let ty = ty.trim();
    let (borrow, rest) = if let Some(rest) = ty.strip_prefix("&mut ") {
        ("&mut", rest)
    } else if let Some(rest) = ty.strip_prefix('&') {
        ("&", rest)
    } else {
        ("", ty)
    };
    (borrow.to_string(), head(rest))
}

fn head(ty: &str) -> String {
    let ty = ty.trim().trim_start_matches('&').trim_start_matches("mut ").trim();
    if ty.starts_with('[') {
        return "[]".to_string();
    }
    let name = ty.split(['<', ' ']).next().unwrap_or(ty);
    name.rsplit("::").next().unwrap_or(name).to_string()
}
