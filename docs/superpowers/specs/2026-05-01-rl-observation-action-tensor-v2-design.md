# RL Observation And Action Tensor V2 Design

**Goal:** Replace the compact v1 RL tensor with a fair-information, mechanically annotated v2 observation contract that gives pilot agents reusable action and pending-choice semantics without asking them to infer every effect from card identity alone.

## Context

The current tensor is a flat `1375`-float observation documented in `docs/TENSOR_SPEC.md` and written by `code/digimon-engine/src/tensor.rs`.

It is compact, but it has three important limitations for pilot training:

- It exposes very little decision metadata. The current selection context is only five floats: phase, legal-count, selecting player, and two reserved slots.
- It does not explain pending-selection choices. This is especially painful for `SelectionKind::TriggerOrder`, where multiple legal action IDs can all mean "resolve a queued effect" but differ strategically.
- It still reflects older layout assumptions, including a separate breeding-area block and a Python legacy tensor-layout import in the RL feature extractor.

Recent `main` changes make a richer contract more feasible:

- `PendingSelection` now preserves `source_kind: EffectSourceKind`.
- `QueuedEffect` carries `source_kind`, so DUAL cards, inherited effects, and security effects can be classified without guessing from raw `CardKind`.
- `BREEDING_SELECTION_TARGET = 99` is now an explicit selection convention.
- Action IDs remain phase-aware and reused, so raw action ID ranges are not sufficient to explain semantics.

No trained pilot models need to be preserved, so v2 can be a breaking observation-shape change.

## Design Principles

1. The action mask remains the legality oracle.
2. The observation may summarize mechanics, but it must not drive rules behavior.
3. Metadata must come from structured engine state, effect descriptors, DSL lowering, or explicit card-effect annotations, not prompt-string parsing.
4. The default pilot observation is fair-information. Hidden opponent hand cards and face-down security cards are not encoded by identity.
5. Card identity and mechanical semantics both matter. Card embeddings represent identity; typed features represent reusable mechanics.
6. Breeding and battle-area objects share permanent-stack structure, but the tensor must preserve their different rules semantics.
7. The v2 contract should remain a flat `Box(float32)` for SB3/ONNX simplicity, while being organized into table-shaped sections that a custom extractor can process structurally later.

## Non-Goals

- Do not change `ACTION_SPACE_SIZE` in this design.
- Do not replace `ActionMasker` or maskable PPO.
- Do not implement an action-conditioned policy head in the first pass.
- Do not make observation metadata authoritative for game rules.
- Do not parse printed card text or human-readable prompts at tensor-write time.
- Do not add an omniscient training tensor as the default pilot observation.

## High-Level Shape

V2 remains a single flat `float32` vector, but its sections are table-shaped:

```text
global_features[64]
player_summary[2][32]
permanent_slots[2][15][96]
own_hand[30][32]
known_zone_cards[120][8]
decision_context[64]
pending_choice_features[32][96]
action_id_features[2168][16]
reserved[256]
```

The proposed total size is:

```text
TENSOR_SIZE_V2 = 43008
```

This is intentionally larger than v1. The action-id table accounts for most of the size. It is shallow by design; rich effect-choice semantics live in `pending_choice_features`.

## Top-Level Layout

| Section | Shape | Size | Notes |
|---|---:|---:|---|
| `global_features` | `[64]` | `64` | Phase, turn, memory, format-level state |
| `player_summary` | `[2][32]` | `64` | Observer first, opponent second |
| `permanent_slots` | `[2][15][96]` | `2880` | 14 battle slots plus breeding slot |
| `own_hand` | `[30][32]` | `960` | One row per action-addressable hand slot |
| `known_zone_cards` | `[120][8]` | `960` | Trash, known security, revealed cards |
| `decision_context` | `[64]` | `64` | Current decision player and prompt summary |
| `pending_choice_features` | `[32][96]` | `3072` | Rich metadata for current prompt rows |
| `action_id_features` | `[2168][16]` | `34688` | Shallow metadata aligned to raw action IDs |
| `reserved` | `[256]` | `256` | Zero-filled until assigned |

## Perspective And Hidden Information

All sections are written from the observing player's perspective:

- Row group `0` means observer.
- Row group `1` means opponent.
- Memory is observer-relative.
- Controller features are observer-relative: self, opponent, either, none.

Default pilot observation must not leak hidden information:

- Own hand card IDs are visible.
- Opponent hand card IDs are not visible. Opponent hand count is encoded in `player_summary`.
- Face-down security card IDs are not visible, including the observer's own security.
- Face-up security cards and revealed cards are visible by card ID.
- Trash, battle area, breeding area, and public linked/source cards are visible by card ID.

If future experiments need perfect-information training, add an explicit `build_tensor_v2_omniscient` helper and keep it out of the default `DigimonEnv`.

## Global Features

`global_features[64]` should include:

- tensor version marker: `2.0`
- turn count normalized
- memory normalized and clipped
- current phase one-hot or stable phase flags
- turn player relative to observer
- current decision player relative to observer
- first-player flag if relevant
- game-over flag
- winner relative to observer if terminal
- format/game mode if available
- small reserved tail

Use one-hot flags for low-cardinality categories where practical. Avoid ordinal enum scalars for phase if a one-hot fits.

## Player Summary

Each `player_summary[player][32]` row should include public counts and coarse state:

- deck count
- digitama deck count
- hand count
- security count
- trash count
- battle-area permanent count
- breeding occupied flag
- revealed-card count controlled/owned by that player if available
- memory-controller flags relative to current memory
- has pending security resolution flag
- has pending option resolution flag
- aggregate keyword/protection summary where cheap and public
- reserved tail

This row is where opponent hand information belongs: count only, no identities.

## Permanent Slots

Use one permanent slot table for both battle area and breeding:

```text
permanent_slots[player][slot]

slot 0-13 = battle area slots
slot 14   = breeding area
```

This is a **permanent slot table**, not a "field" table. The breeding slot shares the row shape because it is also a stack-like permanent object. It does not share battle-area rules semantics.

### Why Breeding Shares The Row Shape

Breeding-area objects and battle-area objects share the information the model needs for many decisions:

- top card identity
- card kind/level/color/DP
- source stack
- inherited/source-card information
- digivolution eligibility
- OPT/source state where available

The action space already treats breeding as a virtual permanent location in several places:

- digivolve uses `field_idx = 14`
- field-effect slots can address the breeding virtual slot
- selection now reserves `BREEDING_SELECTION_TARGET = 99`

Keeping breeding in a separate one-off tensor block would force the model and the action metadata writer to special-case a structurally similar object. A unified permanent table lets action metadata refer to `(player, slot)` consistently.

### Breeding Safety Rules

The row must still mark breeding clearly:

- `zone_battle = 0`
- `zone_breeding = 1`
- `can_attack_now = 0`
- `can_be_attacked_now = 0`
- `can_block_now = 0`
- battle-only targeting flags are `0`
- move/digivolve-related flags may be populated normally

The action mask still decides legality. These fields only help the model understand why slot `14` behaves differently.

### Permanent Row Groups

`PERMANENT_SLOT_SIZE = 96`.

Recommended groups:

| Group | Count | Notes |
|---|---:|---|
| Presence and zone | `8` | active, controller, battle/breeding, slot index |
| Top-card identity | `1` | card ID registry index |
| Top-card static data | `12` | kind, colors, level, DP, play cost, dual flag |
| State | `12` | suspended, source count, linked count, option/training/delay state |
| Current legal affordances | `10` | can attack, block, move, digivolve-on, targetable by current prompt |
| Keywords | `12` | common native/granted keywords as flags/buckets |
| Protections/floodgates | `8` | immunity/protection state visible on the permanent |
| Source entries | `11 * 3 = 33` | card ID, OPT state, DP contribution |
| Reserved | `0` | row is exactly filled |

If breeding lacks a stable `PermanentHandle` for some fields during initial implementation, write the shared fields and zero the handle-dependent ones. Long term, introduce a board-location handle that can represent both battle and breeding.

## Own Hand Rows

`own_hand[30][32]` aligns with the hand-addressable action ranges:

- `0-29` play hand card
- `30-59` hand main effect
- `63-92` DNA digivolve from hand
- `400-999` digivolve from hand

Each row should include:

- active flag
- card ID
- card kind flags, including DUAL
- color flags
- level bucket
- DP bucket
- play cost / option use cost
- has legal play action now
- has legal hand-main effect now
- has legal digivolve action now
- has legal DNA action now
- has legal counter/blast action now
- printed keyword summary where cheap
- reserved tail

Do not add opponent hand rows by identity. Opponent hand count lives in `player_summary`.

## Known Zone Cards

`known_zone_cards[120][8]` stores card rows for public or currently known zones:

```text
0-44    own trash
45-89   opponent trash
90-99   own security slots, card ID only if face-up/known
100-109 opponent security slots, card ID only if face-up/known
110-119 revealed cards
```

Each row:

- active/known flag
- card ID, `0` if unknown or padding
- owner relative flag
- zone enum/flags
- index normalized
- card kind
- level or `0`
- cost/DP bucket depending on card kind

Face-down security rows can still encode slot presence/count through active/count fields, but card ID remains `0`.

## Decision Context

`decision_context[64]` describes the current decision surface:

- current phase one-hot
- decision player relative to observer
- current turn player relative to observer
- pending selection present
- pending selection kind one-hot
- pending selection optional flag
- PASS legal flag
- number of legal choices normalized
- previous phase one-hot or compact phase flags
- source kind of the prompt if available
- source origin zone if available
- current attack-state flags if an attack is in flight
- current security-resolution phase flags if a security check is in flight
- reserved tail

This replaces the v1 five-float selection context.

## Pending Choice Features

`pending_choice_features[32][96]` is the rich table for currently installed `PendingSelection` choices.

Rows are aligned to prompt presentation order, not raw action IDs:

```text
row i = pending_selection.valid_action_ids[i]
```

If `PASS` is legal, include it as an additional row after valid choices when space permits. If there are more than `32` prompt choices, fill the first `32` rows in presentation order and expose truncation/count flags in `decision_context`. The action mask remains complete even if metadata rows are capped.

### Why Not Align Rows To `30-59`

Action IDs are reused across phases:

- `30-59` can mean hand effects, reveal/security selections, or trigger-order picks.
- `59` can be replacement accept in an effect-choice prompt.
- `99` now means breeding target in selection prompts.
- `1000-1009` can mean effect branches.
- `2000-2167` can mean source selections.

Prompt-row alignment is stable across all selection kinds and does not pretend that one raw range has universal semantics.

### Pending Choice Row Groups

Recommended row groups:

| Group | Count | Notes |
|---|---:|---|
| Row/action identity | `8` | active, legal, raw action ID, row index, total choices |
| Selection shape | `10` | trigger order, target, card, mode, source, security, replacement, multi-pick |
| Optionality | `4` | mandatory, optional, PASS/decline, decline-all |
| Timing | `12` | when attacking, when digivolving, on play, main, security, end of attack, end of turn, etc. |
| Source provenance | `10` | source kind, source origin zone, inherited/security flags, controller |
| Source card | `1` | card ID registry index |
| Effect categories | `16` | delete, suspend, unsuspend, bounce, bottom deck, DP +/- , draw/search, memory, play, digivolve, recover, trash security, grant keyword, grant immunity, protection |
| Target profile | `12` | target controller, target kind, zone, suspended/unsuspended, lowest/highest DP/level, count bucket |
| Duration/protection | `7` | immediate, until EOT, until opponent EOT, while condition, blocks source kinds/removal modes |
| Numeric buckets | `8` | DP amount, memory amount, draw/search count, level/play-cost/DP threshold |
| Reserved | `8` | zero-filled |

### Trigger Order Metadata

For `SelectionKind::TriggerOrder`, each row describes the queued effect that would be resolved by selecting that action:

- queued effect source card ID
- `QueuedEffect.source_kind`
- queued effect controller
- `QueuedEffect.timing`
- optional/mandatory
- source permanent location if any
- effect slot
- structured effect category tags

The current `EffectChoiceEntry.label` is useful for debugging but must not be parsed for tensor data.

## Action ID Features

`action_id_features[2168][16]` is a shallow action-aligned table.

Rows are aligned directly to raw action IDs:

```text
row action_id = metadata for that action ID in the current phase/state
```

This table does not replace the action mask. It gives the policy a small amount of action-local context:

| Offset | Field |
|---:|---|
| `0` | legal flag, matching `get_action_mask` |
| `1` | raw action ID normalized |
| `2` | action family enum/normalized bucket |
| `3` | phase family enum/normalized bucket |
| `4` | source zone enum/normalized bucket |
| `5` | source index normalized |
| `6` | target zone enum/normalized bucket |
| `7` | target index normalized |
| `8` | source permanent slot normalized, if any |
| `9` | target permanent slot normalized, if any |
| `10` | memory/cost delta bucket when known |
| `11` | DP/security/count bucket when known |
| `12` | uses hand card flag |
| `13` | uses permanent flag |
| `14` | prompt/selection action flag |
| `15` | reserved |

For illegal actions, static decode fields may still be populated, but state-dependent fields should be zero unless meaningful. The legal flag must always agree with the mask exposed through `info["action_mask"]`.

This table is intentionally small. Rich card/effect meaning belongs in the hand/permanent/pending-choice sections.

## Effect Metadata Contract

Pending-choice effect semantics require a structured effect summary model.

Add an observation-only descriptor, likely attached to `Effect` or returned alongside `Effect`:

```rust
pub struct EffectObservationMetadata {
    pub categories: EffectCategoryFlags,
    pub target_profile: TargetProfileFlags,
    pub duration: EffectDurationKind,
    pub numeric_buckets: EffectNumericBuckets,
}
```

The exact Rust shape can use bitflags, small enums, or fixed arrays, but it must be explicit and non-authoritative.

Sources:

- DSL-lowered effects should derive metadata from compiled steps where possible.
- Raw Rust effects should set metadata through builder methods.
- Keyword auto-effects should define metadata centrally.
- Unknown metadata should be encoded as unknown/zero, not guessed from prompt text.

Rules behavior must never depend on these observation tags. If a metadata tag is wrong, the agent may learn worse, but the engine must still resolve the card correctly.

## Card ID Positions

The v2 feature extractor should still split card IDs from scalar values for embedding.

Card ID positions include:

- permanent top-card IDs
- permanent source-card IDs
- own hand card IDs
- known zone card IDs
- pending choice source-card IDs

`action_id_features` should not contain card IDs in v2. It references source/target zones and indices instead.

The Rust engine should export v2 card/scalar positions through PyO3 or a generated Rust-owned layout file. `code/digimon_gym/agents/features_extractor.py` should stop importing tensor layout from `engine_py_legacy`.

## Implementation Surfaces

Expected engine/RL surfaces to update:

- `code/digimon-engine/src/tensor.rs`
- `docs/TENSOR_SPEC.md`
- `docs/ACTION_SPEC.md` if action feature semantics need cross-reference
- `code/digimon-engine-py/src/lib.rs` for exported constants/layout metadata
- `code/digimon_gym/digimon_gym.py` observation space
- `code/digimon_gym/agents/features_extractor.py`
- any ONNX export assumptions about input size
- tests under `code/digimon-engine/tests/mask_and_tensor/`
- RL smoke tests under `code/tests/rl/`

Preferred implementation shape:

- Introduce `build_tensor_v2`.
- Keep `build_tensor` as an alias only if the migration needs a short transition.
- Export `TENSOR_VERSION = 2`.
- Export `TENSOR_SIZE = TENSOR_SIZE_V2` once `DigimonEnv` has switched.
- Add `compute_positions_v2`.

## Invariants

1. `action_id_features[action_id][0] == get_action_mask(player)[action_id]`.
2. Every nonzero card ID field is listed in `CARD_ID_POSITIONS_V2`.
3. No scalar field is listed in `CARD_ID_POSITIONS_V2`.
4. `CARD_ID_POSITIONS_V2` and `SCALAR_POSITIONS_V2` cover every tensor index exactly once.
5. Opponent hidden hand identities never appear in the default v2 observation.
6. Face-down security identities never appear in the default v2 observation.
7. Breeding slot `14` has `zone_breeding = 1` and all battle-only affordances set false.
8. Prompt rows are ordered by `pending_selection.valid_action_ids`, plus optional PASS row when applicable.
9. `TriggerOrder` choice metadata describes the queued effect selected by that row.
10. `source_kind` and source origin are separate features.

## Testing Requirements

Add focused tests before or alongside implementation:

- Tensor size is `43008`.
- Layout position coverage is exact.
- Opponent hand IDs are absent from observer tensor.
- Face-down security IDs are absent for both players.
- Face-up security IDs are present when revealed.
- Battle slot and breeding slot share row shape.
- Breeding slot has battle-only affordances set to zero.
- Digivolve onto breeding points action metadata at permanent slot `14`.
- `BREEDING_SELECTION_TARGET = 99` points action/pending metadata at permanent slot `14`.
- `TriggerOrder` installs one pending-choice row per offered queued effect.
- `TriggerOrder` rows include source card, source kind, timing, optionality, and effect category metadata.
- DUAL Option use encodes `source_kind = Option`.
- DUAL after Arts Digivolve encodes `source_kind = Digimon` for stack effects.
- Action legal bits match the engine action mask.
- Python `DigimonEnv.observation_space.shape` matches Rust `TENSOR_SIZE`.
- `CardEmbeddingExtractor` consumes Rust-owned v2 positions, not legacy Python layout.

## Migration Plan Sketch

This design intentionally stops short of a task-by-task implementation plan. The next step should be a separate implementation plan that covers:

1. Add v2 layout constants and tests.
2. Write the v2 tensor builder without switching `DigimonEnv`.
3. Export v2 constants and card/scalar positions through PyO3.
4. Update Python feature extraction to read Rust-owned positions.
5. Add action-id feature table generation.
6. Add pending-choice metadata plumbing, starting with `TriggerOrder`.
7. Switch `DigimonEnv` to v2 once Rust and Python smoke tests pass.
8. Update docs and remove v1-only assumptions.

## Open Risks

- `action_id_features[2168][16]` increases input size substantially. It is acceptable for first training runs, but a later action-conditioned extractor may use it more efficiently.
- Some effect metadata will be unknown until raw Rust effects are annotated or DSL lowering can infer categories.
- Breeding handle-dependent fields may be incomplete until the engine has a stable location handle for breeding permanents.
- Full legal-action metadata is useful, but the existing SB3 policy head still emits raw action logits. The table prepares for a better extractor/head without requiring one immediately.

## Acceptance Criteria

The design is ready to implement when reviewers agree on:

- `TENSOR_SIZE_V2 = 43008` as the first v2 target.
- Unified `permanent_slots[2][15]` with breeding at slot `14`.
- Default fair-information observation.
- Rich `pending_choice_features[32][96]` row-aligned to prompt order.
- Shallow `action_id_features[2168][16]` row-aligned to action IDs.
- Structured effect observation metadata as the source for effect-category features.
