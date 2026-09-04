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

/// Ignored on purpose. It activates when the engine's method resolution lands
/// (SYMBOL-TABLE-SPEC.md section 6.3, "rust-analyzer as a one-time oracle"):
/// at that point the engine can be asked, for each site below, which method the
/// call resolves to and what the receiver's type is, and the answers must equal
/// rust-analyzer's. Until then there is nothing to ask, so running this only
/// proves the data loads — which `oracle_loads` above already does.
///
/// Run it once the engine exists with:
///     cd transpile && cargo test --test oracle -- --ignored
#[test]
#[ignore = "activates when the engine's method resolution lands (spec 6.3)"]
fn engine_matches_oracle() {
    let calls = load::<MethodCall>("method_calls.json");
    let derefs = load::<OverloadedDeref>("overloaded_derefs.json");

    // The shape the comparison takes. Each `resolve_method_call` below is the
    // engine entry point that does not exist yet; wiring it up is the whole of
    // activating this test.
    for site in &calls.sites {
        let _ = (&site.file, site.line, site.col, &site.callee, &site.receiver_type);
    }
    for site in &derefs.sites {
        let _ = (&site.file, site.line, &site.steps);
    }

    panic!(
        "the engine has no method-resolution entry point to compare against yet; \
         {} call sites and {} deref chains are waiting in tests/oracle/",
        calls.sites.len(),
        derefs.sites.len()
    );
}
