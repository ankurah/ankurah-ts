# Rust Fixture Completeness Assessment

> **HISTORICAL — a coverage assessment of 2026-03-14, superseded by the fixtures
> themselves.** The counts here (15 bincode fixtures, 7 yrs) have since grown to
> 24 bincode fixtures with per-item offset and length sidecars, 12 yrs V2
> documents, 10 core LWW cases, 91 ankql parse cases and 26 planner cases, and
> each fixture set now carries its own README on the `ts-port-support` branch.
> The gaps this file identifies were the reason for that work. Read it as the
> argument that produced the fixtures, not as their inventory. Historical text
> follows unchanged.

Date: 2026-03-14
Branch: `ts-port-support`
Worktree: `/Users/daniel/ak/ankurah-ts-support/`

## Test Results

All fixture tests pass:
- **Bincode fixtures**: 15/15 passed
- **Yrs V2 fixtures**: 7/7 passed

## Bincode Fixture Coverage

### ID Types
| Type | Covered | Test |
|------|---------|------|
| EntityId | Yes | test_ids_fixture |
| EventId | Yes | test_ids_fixture |
| TransactionId | Yes | test_ids_fixture |
| RequestId | Yes | test_ids_fixture |
| QueryId | Yes | test_ids_fixture |
| UpdateId | Yes | test_ids_fixture |
| CollectionId | Yes | test_ids_fixture |

### Clock
| Variant | Covered | Test |
|---------|---------|------|
| Empty (0 events) | Yes | test_clock_fixture |
| Single event | Yes | test_clock_fixture |
| Multiple events | Yes | test_clock_fixture |

### Auth Types
| Type | Covered | Test |
|------|---------|------|
| AuthData (with bytes) | Yes | test_auth_fixture |
| AuthData (empty) | Yes | test_auth_fixture |
| Attestation | Yes | test_auth_fixture |
| AttestationSet (empty) | Yes | test_auth_fixture |
| AttestationSet (two items) | Yes | test_auth_fixture |
| Attested\<EntityState\> | Yes | test_auth_fixture |
| Attested\<Event\> | Yes | test_attested_event_fixture (NEW) |
| Attested\<Event\> (empty attestations) | Yes | test_attested_event_fixture (NEW) |
| Principal | Yes | test_principal_fixture (NEW) |

### Data Types
| Type | Covered | Test |
|------|---------|------|
| Operation | Yes | test_data_fixture |
| OperationSet | Yes | test_data_fixture |
| StateBuffers | Yes | test_data_fixture |
| State | Yes | test_data_fixture |
| StateFragment | Yes | test_data_fixture |
| Event | Yes | test_data_fixture |
| EventFragment | Yes | test_data_fixture |
| EntityState | Yes | test_data_fixture |

### Request Types
| Type/Variant | Covered | Test |
|--------------|---------|------|
| NodeRequest | Yes | test_request_fixture |
| NodeRequestBody::CommitTransaction | Yes | test_request_fixture |
| NodeRequestBody::Get | Yes | test_request_fixture |
| NodeRequestBody::GetEvents | Yes | test_request_fixture |
| NodeRequestBody::Fetch | Yes | test_request_fixture |
| NodeRequestBody::SubscribeQuery | Yes | test_request_fixture |

### Response Types
| Type/Variant | Covered | Test |
|--------------|---------|------|
| NodeResponse | Yes | test_response_fixture |
| NodeResponseBody::CommitComplete | Yes | test_response_fixture |
| NodeResponseBody::Fetch | Yes | test_response_fixture |
| NodeResponseBody::Get | Yes | test_response_fixture |
| NodeResponseBody::GetEvents | Yes | test_response_fixture |
| NodeResponseBody::QuerySubscribed | Yes | test_response_fixture |
| NodeResponseBody::Success | Yes | test_response_fixture |
| NodeResponseBody::Error | Yes | test_response_fixture |

### Causal Types
| Type/Variant | Covered | Test |
|--------------|---------|------|
| CausalRelation::Equal | Yes | test_causal_fixture |
| CausalRelation::StrictDescends | Yes | test_causal_fixture |
| CausalRelation::StrictAscends | Yes | test_causal_fixture |
| CausalRelation::DivergedSince | Yes | test_causal_fixture |
| CausalRelation::Disjoint (Some gca) | Yes | test_causal_fixture |
| CausalRelation::Disjoint (None gca) | Yes | test_causal_fixture |
| CausalRelation::BudgetExceeded | Yes | test_causal_fixture |
| CausalAssertionFragment | Yes | test_causal_fixture |
| CausalAssertion | Yes | test_causal_assertion_fixture (NEW) |

### Delta Types
| Type/Variant | Covered | Test |
|--------------|---------|------|
| DeltaContent::StateSnapshot | Yes | test_delta_fixture |
| DeltaContent::EventBridge | Yes | test_delta_fixture |
| DeltaContent::StateAndRelation | Yes | test_delta_fixture |
| EntityDelta (all 3 content variants) | Yes | test_delta_fixture |
| KnownEntity | Yes | test_delta_fixture |

### Update Types
| Type/Variant | Covered | Test |
|--------------|---------|------|
| NodeUpdate | Yes | test_update_fixture |
| NodeUpdateBody::SubscriptionUpdate | Yes | test_update_fixture |
| SubscriptionUpdateItem | Yes | test_update_fixture |
| UpdateContent::EventOnly | Yes | test_update_fixture |
| UpdateContent::StateAndEvent | Yes | test_update_fixture |
| MembershipChange::Initial | Yes | test_update_fixture |
| MembershipChange::Add | Yes | test_update_fixture |
| MembershipChange::Remove | Yes | test_update_fixture |
| NodeUpdateAck | Yes | test_update_fixture |
| NodeUpdateAckBody::Success | Yes | test_update_fixture |
| NodeUpdateAckBody::Error | Yes | test_update_fixture |

### Message Types
| Type/Variant | Covered | Test |
|--------------|---------|------|
| Message::Presence | Yes | test_message_fixture |
| Message::PeerMessage | Yes | test_message_fixture |
| NodeMessage::Request | Yes | test_message_fixture |
| NodeMessage::Response | Yes | test_message_fixture |
| NodeMessage::Update | Yes | test_message_fixture |
| NodeMessage::UpdateAck | Yes | test_message_fixture |
| NodeMessage::UnsubscribeQuery | Yes | test_message_fixture |

### Other Types
| Type/Variant | Covered | Test |
|--------------|---------|------|
| Presence (durable, no root) | Yes | test_presence_fixture |
| Presence (ephemeral, with root) | Yes | test_presence_fixture |
| sys::Item::SysRoot | Yes | test_system_fixture |
| sys::Item::Collection | Yes | test_system_fixture |
| subscription::QueryId | Yes | test_ids_fixture |

### Notes on sys::Item::Other
`sys::Item::Other` has `#[serde(other)]` which makes it a deserialization-only fallback variant (it catches unknown enum variants during deserialization). It cannot be directly serialized as a named variant. This is by design and does not need a dedicated fixture -- the TS port should implement it as a deserialization fallback as well.

## Yrs V2 Fixture Coverage

| Scenario | Covered | Test |
|----------|---------|------|
| Empty document | Yes | test_empty_doc |
| Simple text creation | Yes | test_simple_text |
| Multi-field documents | Yes | test_multifield |
| Text with edits (insert, delete, re-insert) | Yes | test_text_with_edits |
| Incremental base state | Yes | test_incremental_base |
| Incremental diff (state vector delta) | Yes | test_incremental_diff |
| Concurrent edits / merge | Yes | test_concurrent_merge (NEW) |

## Changes Made

Three new bincode fixture tests were added:
1. `test_causal_assertion_fixture` -- covers `CausalAssertion` struct (was missing)
2. `test_principal_fixture` -- covers `Principal` struct (was missing)
3. `test_attested_event_fixture` -- covers `Attested<Event>` standalone (was only tested inside other structures)

One new Yrs V2 fixture test was added:
4. `test_concurrent_merge` -- two docs with different client_ids make concurrent text insertions, merge via update exchange, verify convergence

New fixture files generated:
- `proto/test_fixtures/causal_assertion.bin`
- `proto/test_fixtures/principal.bin`
- `proto/test_fixtures/attested_event.bin`
- `proto/test_fixtures/yrs_v2/concurrent_merge.bin`

## Conclusion

All public serializable types and enum variants in `ankurah-proto` are now covered by fixture tests. The fixture infrastructure is complete and ready for cross-platform validation against the TypeScript port.
