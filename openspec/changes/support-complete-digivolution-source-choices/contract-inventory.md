# Frozen implementation contract

This descriptor governs the coordinated migration. Rust now implements action
schema two and 128-wide pending rows; remaining consumers and selector families
still need verification. It does not certify the blocked cards.

| Operation | ID |
| --- | --- |
| Commit visible card row 0 through 27 | 30 through 57 |
| Previous page | 58 |
| Next page | 59 |
| Decline, when permitted by the owning operation | 62 |

Page capacity is 28. ACTION_SPACE_SIZE remains 2192. These aliases are legal
only while the new typed card-choice owner is installed. CountChoice retains
its separate owner and numeric payload. DNA material prompts retain their
existing semantics. No decoder may infer card identity from these IDs alone.

An owned prompt contains the complete presentation pool and legal subset, an immutable
prompt ID, page generation, page index, selected prefix and physical pins.
Every navigation increments the page generation with checked arithmetic. A
submission carries the expected prompt ID and page generation. Invalid or
stale submissions leave the owner unchanged. Navigation moves no cards,
spends no cost, consumes no OPT and uses no RNG. Resolution revalidates the
physical card and carrier; a position is presentation data, never identity.
Source positions use usize internally, with checked wire conversions.

Source pins identify battle versus breeding, owner, stable carrier identity
and physical CardHandle. Security pins identify owner, physical card and
security-order generation. Security rows expose printed metadata only when
the selecting seat has inspection permission. Internal physical handles
remain private and must not become card-ID observation features.

Action schema version two and a new active observation feature schema/hash
are required before installing these prompts in production. The active
standard_lite_v2 / standard_lite_deck_v2 Rust sizes are now 9434 / 9874;
standard_compact_v1 remains explicit engine-only compatibility. New row
metadata must be allocated through exported layout metadata; the active
sizes/hashes are not changed by this planning descriptor.

### Observation allocation for the migration

Use a 128-float pending row, retaining existing offsets 0 through 85. This
avoids overloading the count-choice fields or treating physical cards as
numeric declarations. With 32 rows, the additional 1024 floats make the
Rust lite/deck sizes 9434/9874. Export all fields in layout metadata;
FEATURE_SCHEMA_VERSION and the layout hashes must change atomically with
the active decoder. The source constants and writers now use these sizes; binding/runtime verification remains pending.

| Pending row offset | Field |
| --- | --- |
| 86 | Card-page row present |
| 87 / 88 / 89 | Commit / previous / next flags |
| 90 | Typed zone (battle source, breeding source, security, other supported origin) |
| 91 | Owner relative to observer |
| 92 | Current carrier slot, when present |
| 93 | Permitted printed registry card ID; hidden and navigation rows use zero |
| 94..97 | Prompt ID, four normalized 16-bit limbs |
| 98..101 | Page generation, four limbs |
| 102..105 | Page index, four limbs |
| 106..109 | Page count, four limbs |
| 110..113 | Current zone/source position, four limbs |
| 114..117 | Stable carrier identity, four limbs; absent for security |
| 118 | Carrier present |
| 119 | Card identity permitted to this observer |
| 120 | Decline operation flag |
| 121 | Inspection-only card row, never a legal commit |
| 122..127 | Reserved, zero |

Do not expose physical CardHandle values in any row. Wire u64 values are
decimal strings, including the expected prompt/page token on submissions.
Navigation and decline use typed operations with no fake card identity.
The operation owner provides current source/security positions after live pin
resolution; those positions are never used to bind a committed card.

## Migration inventory

| Producer / consumer | Existing limitation or assumption | Required migration |
| --- | --- | --- |
| EffectContext::select_material | min(source_count, 12), position callback | Complete battle/breeding candidates, owned physical resolution |
| select_own_sources / select_opponent_sources, SourceMultiStep | Encoded SourceSelectionRef, u8 position | Complete physical candidates and feasible-prefix masks |
| source_multi_candidates / DSL source candidate collection | encode_source_select rejects depth >= 12; positions narrow to u8 | Filter complete vectors before paging, retain physical pins and usize positions |
| select_union_zone / union_zone_action_ids / union_zone_candidates | Source/breeding candidates pass through the legacy encoders, origins narrow to u8 | Preserve hand/trash/source origin and complete physical source candidates in the owned chooser |
| DigiXros digixros_material_candidates_for / pending material decoder | Under-Tamer origins pass through the 12-slot source encoder | Retain typed DigiXros origin and requirement feasibility while paging physical candidates |
| CountCappedZone::Material | Debug assertion of at most 12 sources | Shared owned chooser, preserve min/max and continuation |
| partition_source_refs and Partition keyword activation | Both truncate to 12 | Complete candidate and requirement matching |
| select_security | min(security_len, 10), 40/50 offset callback | Owned security candidates with visibility and order generation |
| DSL zone_moves security placement | Legacy security index extraction | Typed choice binding and completed payment receipt |
| runners/selection_resolve | Reconstructs material/count candidates from 12-slot band | Resolve typed rows from the installed owner |
| ResumeSelectKind / SourceMultiState / clone and state hash | Captured legacy position geometry | Preserve complete owner, generation and physical pins |
| Generic selection dispatch and async step transports | Current action ID only | Validate prompt/page token before taking prompt or running hooks |
| UI JSON, frontend selection rendering and action submission | Fixed source/security slot decoding | Export descriptor and typed row metadata, navigation and generation |
| PyO3 / DigimonEnv / action masking / tensor encoders | Existing schema and pending-row metadata | Export descriptor, complete row features, authoritative mask |
| Replays / model manifests / inference / export | Same-size semantic mismatch possible | Reject incompatible action schema and observation hash |

The fixed v1 observation security slots are display limits, not game limits.
Do not remove those constants blindly: active pending rows provide complete
choices. Legacy source action encode/decode helpers remain compatibility
APIs until all producers and consumers above have migrated together.

Security inspection amendment: search pages include ineligible cards as typed
inspect_card rows (mask zero). Owned page tensor rows follow cardChoice.rows,
while other prompts retain valid_action_ids order. No-match nonempty searches
still permit full inspection and an explicit finish before shuffle/continuation.
