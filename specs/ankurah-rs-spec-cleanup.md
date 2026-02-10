# ankurah-rs Spec Cleanup - Low Priority Action Items

**Created**: 2026-02-10
**Source**: `_agent-work/spec-cross-check.md` analysis
**Priority**: Low - these are correctness and housekeeping issues, not blocking active development.

---

## 1. Specs with Incorrect V1 References

The Rust codebase exclusively uses **V2 encoding** for all Yrs operations. Every V1 reference below must be corrected to V2.

- [ ] `yrs-yjs-interop-validation.md` line 12: "V1 encoding: Original format, wider compatibility" -- misleadingly implies V1 is what ankurah uses
- [ ] `yrs-yjs-interop-validation.md` line 40: "V1 encoding: Compatible (this is the primary interop format)" -- V2 is the actual interop format
- [ ] `yrs-yjs-interop-validation.md` lines 243-244: Sample Rust code shows `txn.encode_state_as_update_v1` -- must be `encode_state_as_update_v2`
- [ ] `yrs-yjs-interop-validation.md` line 251: Sample Rust code shows `Update::decode_v1(buffer)?` -- must be `Update::decode_v2`
- [ ] `yrs-yjs-interop-validation.md` line 275: "ankurah currently uses **V1 encoding** (`encode_state_as_update_v1`). The TS port should also use V1 for maximum compatibility." -- entirely wrong; must state V2
- [ ] `architectural-decisions.md` line 30: "Use V1 encoding (not V2) for maximum cross-implementation compatibility" -- must be reversed: use V2
- [ ] `ecosystem-research.md` line 39: "ankurah's Yrs (Rust) is a port OF Yjs, so state encoding should be compatible (V1 format)" -- must say V2 format
- [ ] `continue-implementation.md` line 92: "Yrs/Yjs state: Yjs V1 update encoding (should be Yrs-compatible, needs validation)" -- must say V2

---

## 2. Specs That Duplicate Proto Struct Definitions

The Rust proto source at `ankurah/proto/src/` is authoritative. Spec sections that paraphrase wire protocol types by describing struct shapes should be deleted or replaced with a reference to the Rust source. Paraphrased definitions have already drifted from reality.

- [ ] `wire-format-interop.md` lines 72-80: Fake fixture generation code showing `NodeMessage { id, from, to, body: NodeMessageBody::Fetch { ... } }` -- this struct shape does not exist. `NodeMessage` is an enum, not a struct with `id/from/to/body` fields.
- [ ] `wire-format-interop.md` lines 329-339: Full `Operation` struct definition with `{ backend: String, data: Vec<u8> }` -- wrong shape (see Section 3 below). Delete and reference `proto/src/data.rs`.
- [ ] `architecture.md` lines 113-114: Type mapping table rows listing `NodeMessage` as a simple type and `NodeMessageBody` as a discriminated union -- `NodeMessageBody` does not exist in the Rust codebase.
- [ ] `ecosystem-research.md` lines 136-138: Paraphrased `NodeMessage -> NodeRequest / NodeResponse / NodeUpdate / NodeUpdateAck` and `NodeRequestBody` list -- omits the actual top-level `Message` enum, omits `UnsubscribeQuery` variant, omits `auth: Vec<AuthData>` on `NodeMessage::Request`.
- [ ] `initial-porting-workflow.md` line 76: "`update.rs -> update.ts`: NodeMessage, NodeMessageBody, SubscriptionUpdateItem, UpdateContent" -- `NodeMessageBody` does not exist; the file is `message.rs` not `update.rs` for `NodeMessage`.

---

## 3. Incorrect Type Descriptions

These are concrete factual errors in struct/enum shapes that differ from the actual Rust code.

- [ ] `wire-format-interop.md` lines 330-333: `Operation { backend: String, data: Vec<u8> }` -- actual is `Operation { diff: Vec<u8> }`. The backend name is the key in `OperationSet(BTreeMap<String, Vec<Operation>>)`, not a field on `Operation`. Source: `proto/src/data.rs` lines 157, 178-181.
- [ ] `wire-format-interop.md` line 339: "decode operations by checking the `backend` field" -- there is no `backend` field on `Operation`.
- [ ] `wire-format-interop.md` line 13: "`Event.operations` bincode (Vec<Operation>)" -- actual type is `OperationSet`, not `Vec<Operation>`.
- [ ] `architecture.md` line 114: "`NodeMessageBody` (discriminated union)" -- this type does not exist anywhere in the codebase. The actual wire-level type is `Message` (wrapping `Presence` or `PeerMessage(NodeMessage)`), and `NodeMessage` is itself an enum with variants `Request`, `Response`, `Update`, `UpdateAck`, `UnsubscribeQuery`.
- [ ] `ecosystem-research.md` lines 136-137: Describes `NodeMessage` as directly containing `NodeRequest / NodeResponse / NodeUpdate / NodeUpdateAck` -- missing the outer `Message` enum and the `Presence` variant.
- [ ] `wire-format-interop.md` lines 73-78: Sample code constructs `NodeMessage { id, from, to, body }` as a flat struct -- `NodeMessage` is an enum; the `{id, from, to, body}` fields belong to `NodeRequest` (a separate struct nested inside `NodeMessage::Request`).

---

## 4. Stale File Inventories

Spec file lists significantly undercount the actual Rust source files. Each needs updating.

- [ ] `continue-implementation.md` lines 58-63 and `initial-porting-workflow.md` lines 73-78 (Phase 1): Proto module inventory lists ~7 files (`data`, `message`, `request`, `update`, `clock`, `id`, `sys`). Actual proto has 14 modules: `auth`, `clock`, `collection`, `data`, `error`, `human_id`, `id`, `message`, `peering`, `request`, `subscription`, `sys`, `transaction`, `update`. Missing 7 modules including `auth.rs` (critical for `Attested<T>`, `AuthData`), `peering.rs` (`Presence`), `subscription.rs` (`QueryId`).
- [ ] `initial-porting-workflow.md` lines 104-110 (Phase 2): Signals file list has 7 entries. Actual `signals/src/` has 16+ files including `context.rs`, `reactive_graph.rs`, `porcelain/subscribe.rs`, `porcelain/wait.rs`, `signal/map.rs`, `value.rs`, `react.rs`, `react_native.rs`. More than double the estimated scope.
- [ ] `initial-porting-workflow.md` line 228 (Phase 8): Reactor listed as single `reactor.rs -> reactor.ts` with 7 files total in the phase. Actual reactor is a module root with 8+ sub-files: `watcherset.rs`, `property_path.rs`, `update.rs`, `candidate_changes.rs`, `comparison_index.rs`, `fetch_gap.rs`, `subscription.rs`, `subscription_state.rs`.
- [ ] `initial-porting-workflow.md` lines 155-159 (Phase 4): Storage common described as "a small package - just trait definitions" with 1 file (`lib.rs -> index.ts`). Actual `storage/common/src/` has 7 modules: `bounds`, `filtering`, `planner`, `predicate`, `sorting`, `traits`, `types`. Includes a query planner and filtering engine.
- [ ] `structural-mapping-analysis.md` line 301: Claims "~88% of files are directly or near-directly mappable" but only counted ~80 files. Actual codebase has 120+ files. The 88% applies to a subset, not the whole.

---

## 5. Scope Corrections

The de-scoped items lists in `continue-implementation.md` and `architecture.md` need updating.

- [ ] `continue-implementation.md` line 24: De-scoped list says "lineage attestation" -- this is too broad. Only **cryptographic verification** is de-scoped. The `Attested<T>` wrapper type and `AttestationSet` must still be ported (as wire protocol types), and `CausalRelation`, `CausalAssertion`, `CausalAssertionFragment`, `KnownEntity` must be at least stub-ported for bincode compatibility.
- [ ] `architecture.md` lines 227-235: De-scoped list says "Lineage attestation / cryptographic verification" -- same issue. The types are wire protocol types and must exist even if verification logic is stubbed.
- [ ] `architecture.md` line 224: In-scope list includes "CLI code generator for typed wrappers" but `continue-implementation.md` line 26 and `architectural-decisions.md` lines 23-25 say "No code generation initially." This contradiction should be resolved -- clarify that CLI codegen is Phase 11 (later), not Phase 1.
- [ ] Only truly de-scoped items: PostgreSQL storage, Sled storage, WebSocket **server** (client is in scope), PN Counter backend. Everything else mentioned in de-scope lists (`Attested<T>`, lineage types, policy agent traits) needs at least stub implementations.

---

## 6. TODO Spec Files

These spec files are listed in `continue-implementation.md` (lines 112-114) as TODO and have not been written.

- [ ] `rust-changes-required.md` (line 112): "Minimal Rust-side changes needed. **Not yet written** - was drafted but rejected (had JSON wire format). Needs rewrite: complete PR #236, schema export CLI, bincode fixtures. No JSON wire format." Status: TODO. Note: the `_agent-work/rust-architecture-findings.md` partially covers Rust-side analysis but does not replace this spec.
- [ ] `codebase-organization.md` (line 113): "How to organize TS source into 1:1 mapped / completely different / bridge zones with inline annotations. **Not yet written.**" Status: TODO.
- [ ] `domcorder-patterns.md` (line 114): "Reference patterns from `~/code/domcorder` for TS bincode implementation. **Not yet written** - agent couldn't access directory (permission issue)." Status: TODO. Note: `_agent-work/domcorder-analysis.md` now exists and covers the domcorder bincode patterns extensively. This TODO may be satisfiable by promoting that analysis or referencing it.
