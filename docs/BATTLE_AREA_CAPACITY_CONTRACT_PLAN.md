# Battle-area capacity contract work

Status: planned; not implemented. The Examon campaign cannot claim unrestricted
DCGO/official-rules equivalence while this gap remains.

## Reproduction and authority

General rule 3-4-7-2 permits any number of cards in the battle area. The current
standard `Ruleset` limits each battle area to 14 (`rules.rs`), and
`action/space.rs::MAX_FIELD_SLOTS` is also 14. `ACTION_SPEC.md` reserves index 14
for security or breeding in several encodings. `TENSOR_SPEC.md` describes fixed
permanent rows and model compatibility metadata.

The current base DCGO battle scene provides 16 eligible battle frames and one
breeding frame for each player. `Player.Start` rebuilds these lists from the
scene hierarchy; the serialized `fieldCardFrames` list and the default
`FieldPermanents[16]` declaration do not establish runtime capacity.
`FieldCardFrame.isBattleAreaFrameID` excludes only the final breeding frame,
and `CanPlayAsNewPermanent` checks for an empty eligible frame. This establishes
a concrete source-level gap between our 14-slot limit and DCGO's 16 battle
frames, independently of the official rule allowing any number. A live
boundary comparison is still required. Increasing only the card or ruleset
limit cannot repair the action and observation contracts below.

`bt7_063_full_behavior::one_remaining_field_slot_keeps_both_required_picks_but_only_first_play_commits`
exposes the limitation: both named sources are selected, but only one enters
before the original host is deleted. The test documents existing bounded-engine
behavior; it is not evidence that the behavior follows official rules.

Increasing the card's limit or removing its candidate guard alone is unsafe:
the resulting permanents would have missing or colliding actions and could be
invisible to the active observation profile. This is a shared contract change,
not a BT7-063 exception.

The same audit also found bounded hand, trash, source, reveal, security and
ordering choices. The
`HAND_MAIN_LIMIT` and `TRASH_MAIN_LIMIT` ranges truncate otherwise legal
hand/trash candidates, while source selections use a fixed per-permanent
range. Pagination must cover these zones too; moving the battle-area boundary
alone would leave legal BT7-063 source and hand/trash choices unreachable.
Reveal and security helpers also truncate to ten candidates, and the ordering
helper retains only ten cards. Migrate these producers together with their
continuations rather than making excess cards silently unavailable.
Source indices must never spill into a neighboring action range. These are
open contract limitations, not confirmed card behavior.

## Required design

1. Separate permanent identity from compact action/tensor presentation indices.
   Audit the `u8` permanent indices and breeding sentinel before selecting a
   wider storage representation. Preserve incarnation identity across movement,
   clones, queued effects, receipts, saves and replay hashing.
2. Add a typed, paged presentation for main actions and every permanent choice.
   Native attacks need independently addressable attacker and defender choices;
   evolution, blocks, Alliance, field effects, DNA, replacements, source picks,
   simultaneous batches and opponent-controlled selections need equivalent
   coverage. Page navigation must never spend a game action or resolve a trigger.
3. Define the new observation/action version as one contract. A fixed discrete
   action count may be retained only if phase-aware navigation exposes every
   candidate without collisions. Specify page identity, total counts, selected
   physical objects, and current candidate rows in the observation metadata.
   Existing models must not silently interpret the changed action semantics.
4. Update Rust constants/decoding/masks, PyO3 exports, RL wrappers, frontend and
   Tauri selection rendering, inference/export metadata, and compatibility
   gates together. Pin old profiles for archival replay where supported; new
   training or a validated re-export is required for affected live models.
5. Audit physical card allocation separately from field presentation:
   `CardHandle(u16)` and field/source location indices have different bounds.
   Widen serialized identities with checked allocation and an explicit resource
   exhaustion error; neither paging nor a larger field vector fixes allocator
   wraparound. Battle and breeding locations need distinct typed identities.
6. Remove the standard gameplay capacity restriction only after all choice and
   observation paths support the new representation. A separately named bounded
   testing ruleset may retain a limit but cannot advertise official parity.

## Acceptance tests

- Play both BT7-063 sources from the 13-permanent setup and preserve both entry
  events, their legal reactions, and the original host's eventual deletion.
- At least two pages of otherwise identical physical permanents: every legal
  attacker, target, blocker, evolution host, source, effect and replacement is
  reachable exactly once; unavailable choices stay masked.
- Remove and re-enter objects while a later-page decision is pending. Resolve
  captured identities, never a same-slot replacement.
- Clone/resume, save/restore and replay digests agree across navigation and
  continuation boundaries; navigation cannot alter memory, timing or OPT use.
- Observation metadata explains every displayed choice, with no hidden rules
  decisions. Python/Rust/frontend contract fixtures agree, and old model
  artifacts fail compatibility checks clearly.
- Live DCGO comparisons cover the boundary and duplicate physical objects.
- Hand, trash, source, reveal, security and ordering collections wider than
  their current action ranges retain
  every legal physical choice. Page transitions preserve selection order,
  mandatory minimums, decline semantics and already selected cards.

Completion requires an implemented and verified shared contract. A source
review or tests within 14 slots do not close this gap.

## Migration boundaries

The existing numeric count pager in `code/digimon-engine/src/count_choice.rs`
provides a useful navigation precedent: 28 offered values, separate previous
and next operations, and commit-time physical identity checks. It does not
provide paging for permanents or native actions. Reusing its raw IDs is safe
only under an explicitly typed, mutually exclusive presentation state.

Keep these versions separate:

- `action/space.rs::SCHEMA_VERSION` identifies action meanings, independently
  of policy head width. A paged action contract must advance this version even
  if `ACTION_SPACE_SIZE` remains 2192.
- Observation profile IDs, feature schema versions and layout hashes describe
  what a policy observes. A new pager cannot silently replace existing rows.
- Recording schema and verification digest versions describe archival replay.
  They are not substitutes for model action compatibility.

The current Tauri model gate in `code/src-tauri/src/models.rs` forwards tensor
metadata and action-space size into `ModelContract`. Before enabling a paged
contract, export and validate the action semantic version throughout model
metadata, Python inference, ONNX export, hosted manifests and the Tauri gate.
Test rejection of a model with the correct tensor shape and action count but
the old action semantics. Missing new metadata must not imply compatibility.

Implement this as a dedicated contract migration after the current card-effect
repair checkpoint is frozen. First add versioned identity and presentation
types with bounded compatibility tests; then migrate selection and native
action producers and all consumers together. Enable the new profile only when
the complete producer audit and cross-language fixtures pass. Finally remove
the standard field cap and run the overflow cases above, including the live
DCGO 15th and 16th permanent comparisons. A passing intermediate checkpoint
does not close this capacity blocker.
