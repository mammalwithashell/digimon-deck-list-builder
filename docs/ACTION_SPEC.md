# Action Decoder Specification


Owned physical union menus page the complete hand/trash/material candidate pool through the same card-choice aliases. Typed zone codes are 1 battle source, 2 breeding source, 3 security, 4 hand, 5 trash, and 6 battle top. Hand identities remain private to their owner; trash identities are public. Commits resolve the pinned card in its original zone/carrier, including after compaction.

## Physical source and security paging migration (implementation in progress)

The coordinated change `support-complete-digivolution-source-choices` reserves
actions 30–57 for 28 physical card rows, 58/59 for previous/next page, and 62
for an allowed decline. Action capacity remains 2192; activation requires
action schema version 2 across every consumer. Rust now exports version 2;
version 1 models are incompatible. Remaining selector, binding and runtime
qualification is tracked in the change; the worktree is not yet certified
for training admission.

The pending prompt owns the full filtered candidate vector and physical pins.
A row commits the pinned card after checking its current carrier and zone;
positions and action IDs do not identify cards across pages. Navigation
changes only page state and generation, without spending costs or once-per-turn
uses, moving cards, invoking effect continuations, or consuming RNG. Masks
remain the legality authority. Count declarations retain their separate typed
contract.

A security search presents all cards in its stack, including ineligible cards.
Typed `inspect_card` rows have no legal commit bit; they remain visible to the
inspection recipient and can be browsed with the same page navigation. A
nonempty search with no eligible matches presents inspection and Done/decline,
then runs its no-match/outer continuation. Inspection rows occupy presentation
positions in `cardChoice.rows` without joining `valid_action_ids`.

Asynchronous submissions must include the expected prompt ID and page generation
as exact unsigned decimal strings. Reject mismatches before any mutation.
Trusted serial engine callers may resolve the currently installed prompt.
A security search grants inspection only to its selecting seat; other recipients
must not receive hidden card IDs or registry indices. A security reorder or
shuffle invalidates the old physical menu, including shuffles that happen to
retain order. Mandatory invalidation must explicitly rebuild or finish the
operation rather than strand an empty mask.

A stale mandatory security menu exposes `refresh_page` on action 59 when a
live eligible card remains. Refresh reevaluates the original predicate against
the current stack, rebuilds its physical pins and increments page generation.
It invokes no callback, payment, lifecycle hook, movement or RNG. Raw selectors
retain a read-only `Send + Sync` predicate; DSL selectors retain typed predicate,
bindings and provenance data. A positional filter such as index zero therefore
selects the current top card after refresh. An optional stale security prompt
retains its existing PASS behavior.

An impossible mandatory physical menu exposes `finish_impossible` on action 62.
This completes an unavailable operation, rather than paying or declining a
still-available printed choice. The engine abandons its callback and paid DSL
tails, releases only its armed typed finalizers, rejects current payment credit
and resumes enclosing lifecycle continuations. Required prefixes count only
sources still under their original carrier. Their masks also preserve named-role
matching and distinctness feasibility; a partial prefix cannot earn a reward.
Raw Partition menus additionally retain a read-only completion predicate over
their original physical pool, so losing a required printed role offers Finish
even when enough unrelated source cards remain.

The tensor allocation, compatibility migration, and full producer inventory
are specified in TENSOR_SPEC.md and the change's contract-inventory.md.


The engine exposes `2192` discrete action IDs. Legal actions are provided by `get_action_mask(player_id)` and executed by `decode_action(action_id, player_id)`.

## Global Action Ranges

| Range | Count | Meaning | Formula |
|---|---:|---|---|
| `0-29` | 30 | Play hand card | `hand_idx` |
| `30-59` | 30 | [Hand][Main] effect (Main) / Trash hand card (selection) | `30 + hand_idx` |
| `60` | 1 | Hatch |
| `61` | 1 | Move from breeding |
| `62` | 1 | Pass / decline |
| `63-92` | 30 | Initiate DNA Digivolve | `63 + hand_idx` |
| `93` | 1 | Explicit concede command; suppressed from agent masks |
| `94` | 1 | Play first (BO3 `SelectPlayOrder` only) |
| `95` | 1 | Play second (BO3 `SelectPlayOrder` only) |
| `96-99` | 4 | Unused |
| `100-399` | 300 | Attack / block / alliance (phase-dependent) | `100 + slot * 15 + target` |
| `400-999` | 600 | Digivolve | `400 + hand_idx * 15 + field_idx` |
| `1000-1999` | 1000 | Effect activation | `1000 + perm_idx * 10 + effect_idx` |
| `2000-2167` | 168 | Source selection (battle-area carrier) | `2000 + field_idx * 12 + source_idx` |
| `2168-2191` | 24 | Source selection (breeding-area carrier) | `2168 + carrier_owner * 12 + source_idx` |

## Key Constants

| Name | Value | Notes |
|---|---:|---|
| `FIELD_SLOTS` | 14 | Battle area slots per player |
| `MAX_SOURCES` | 11 | Max digivolution stack depth |
| `SECURITY_TARGET` | 14 | Attack target index for security (`= FIELD_SLOTS`) |
| `BREEDING_SLOT` | 14 | Virtual field index for breeding (`= FIELD_SLOTS`) |
| `TARGETS_PER_ATTACKER` | 15 | Stride for attack formula |
| `FIELDS_PER_HAND` | 15 | Stride for digivolve formula |
| `EFFECTS_PER_PERM` | 10 | Stride for effect formula |
| `SOURCES_PER_FIELD` | 12 | Stride for source selection formula |
| `BREEDING_SOURCE_CARRIERS` | 2 | Breeding carriers addressable for source selection (one stack per player) |
| `BREEDING_SOURCE_SELECT_START` | 2168 | First breeding-carrier source-selection action ID |

## Phase-Aware Meaning

Action IDs are intentionally reused across phases.

### Mulligan

- `0`: keep opening hand
- `1`: mulligan opening hand (redraw 5)

### Main

- `0-29`: play card from hand
- `30-59`: activate [Hand][Main] effect on hand card (`30 + hand_idx`)
- `62`: pass turn
- `63-92`: initiate DNA digivolve
- `100-399`: attack (`target=14` means security)
- `400-999`: digivolve (`field_idx=14` means breeding-area digivolve)
- `1000-1999`: effect activations currently used for training/delay style actions

### Breeding

- `60`: hatch
- `61`: move from breeding
- `62`: pass breeding
- `1000-1999`: breeding-side training activation (virtual slot 14)

### BlockTiming

- `100-113`: choose blocker (`100 + blocker_slot`)
- `62`: decline block (unless rules force block, for example Collision state)

### CounterTiming

- `400-999`: blast digivolve candidates
- `62`: decline counter

### EndOfTurnAction (`GamePhase=15`)

- `62`: decline / finish end-of-turn action window
- `100-...`: end-of-turn attack selections granted by `MayAttack` / `ForceAttack`
- `63-...`: DNA digivolve routes registered for the end of turn (hand index)

The end-of-turn attack keywords (<Engage>, <Execute>, <Vortex>, <Overclock>)
are not offered here: they are trigger-type effects (Comprehensive Rules 4.3
§16-44-2, §16-38-2, §16-33-2, §16-34-2) whose decision is a pending selection
inside the `[End of Your Turn]` pool — the attack declaration in the attack
range `100-399` plus `62` (decline), or for <Overclock> its cost pick
(`encode_attack(0, field_idx)`) plus `62`. The former Overclock field-effect
bit (`1000 + perm_idx * 10 + 0`) is never legal.

### AllianceTiming (`GamePhase=16`)

- `100-113`: choose ally to suspend for Alliance bonus
- `62`: decline Alliance

### SelectPlayOrder (BO3 match training only)

- `94`: play first in the next game
- `95`: play second in the next game

Entered between games of a best-of-three match by `Game::request_play_order_selection(loser_id)`. Only the loser of the previous game is the chooser; the mask reports `94`/`95` legal only for that player. The chosen `PlayOrder` is written to `Game::last_play_order_choice` for the Python `MatchEnv` wrapper to consume.

### Concede (explicit command)

- `93`: explicit concede command. The current engine intentionally suppresses this action from agent masks. Count navigation preserves that policy; it does not introduce a new concede choice.

### Selection Phases

- Generic selection phases (`SelectTarget`, `SelectMaterial`, `SelectHand`, `SelectReveal`, `SelectEffectChoice`, `SelectSecurity`) use `pending_selection.valid_indices`.
- `SelectTrash`: uses `pending_selection.valid_indices` when available, otherwise falls back to `130-179` (`130 + trash_idx`). Optional selections allow decline with `62`.
- `SelectSource` / `SelectMaterial`: uses `pending_selection.valid_indices` when available, otherwise falls back to `2000-2167` (`2000 + field_idx * 12 + source_idx`) for a battle-area carrier, or `2168-2191` (`2168 + carrier_owner * 12 + source_idx`) for a breeding-area carrier (King Drasil). Optional selections allow decline with `62`.
- `SelectBudgeted`: uses field-target action IDs for opponent battle-area permanents and allows decline with `62` once the minimum pick count is satisfied.
- `SelectBreedingPermanent`: uses phase-scoped breeding selection IDs (`14` for player 0, `15` for player 1) for breeding-area permanent choices.
- Optional selections allow decline with `62`.

### Selection Primitive Reuse

Group 2 selection primitives reuse existing action ranges.

- Cross-permanent source selections (battle-area carriers) use `SOURCE_SELECT_START..SOURCE_SELECT_END`.
- Breeding-area carrier source selections use `BREEDING_SOURCE_SELECT_START..BREEDING_SOURCE_SELECT_END` (`2168..2192`), keyed by the carrier's owning player. This sub-range was appended in Task S1.3, raising `ACTION_SPACE_SIZE` from `2168` to `2192` — a deliberate action-space version bump (existing trained RL models must be retrained).
- Up-to-N source selections expose `PASS` only after the minimum pick count is satisfied.
- DP-budget permanent selections reuse field-target action IDs during `SelectBudgeted`.
- Breeding permanent selections use phase-scoped breeding selection IDs only while `SelectBreedingPermanent` is pending.

Any future expansion that requires more source slots, more breeding targets, or additional simultaneous selection surfaces must update this document, Rust constants, PyO3 constants, and RL environment constants in the same change.

## Selection Conventions

| Index Range | Selection Meaning |
|---|---|
| `0-29` | Hand card index |
| `30-39` | Revealed-card index |
| `40-49` | Own security index |
| `50-59` | Opponent security index |
| `62` | Decline optional selection |
| `14/15` | Own breeding permanent during `SelectBreedingPermanent` |
| `100-113` | Own battle-area permanent |
| `114-127` | Opponent battle-area permanent |
| `130-179` | Trash-card index |
| `1000-1009` | Effect branch choice |

Arts Digivolve uses `99` for the controller's breeding-area target during its optional selection prompt.

## Attack Formula

`action_id = 100 + attacker_slot * 15 + target`

Target mapping:

- `0-13`: opponent battle-area slots
- `14`: opponent security (`SECURITY_TARGET`)

## Digivolve Formula

`action_id = 400 + hand_idx * 15 + field_idx`

- `field_idx=0..13`: battle area
- `field_idx=14`: breeding area (`BREEDING_SLOT`)

## Effect Formula

`action_id = 1000 + perm_idx * 10 + effect_idx`

Current engine usage includes training and delay effect hooks, depending on phase and card state.

## Source Selection Formula

Battle-area carrier:

`action_id = 2000 + field_idx * 12 + source_idx`

Breeding-area carrier (King Drasil — Task S1.3):

`action_id = 2168 + carrier_owner * 12 + source_idx`

Used in `SelectSource` / `SelectMaterial` when effects need a specific card from a digivolution stack. A breeding-area carrier holds exactly one digivolving stack per player, so it is addressed by the carrier's owning player rather than a field index.

## Masking Contract

- Mask size is always `2192`
- `1.0` means legal, `0.0` illegal
- Frontend and RL agents must select only masked-legal actions
- Backend decoder is phase-aware and resolves semantics from current `GamePhase`


## Dedicated paged count choices

An installed count prompt uses `SelectEffectChoice` / `EffectChoice` and owns a
typed page in its continuation. These are phase-aware aliases of existing IDs:

| Action | Meaning inside a count prompt |
|---|---|
| 30–57 | Commit the corresponding one of up to 28 presented counts |
| 58 | Previous page, only when a previous page exists |
| 59 | Next page, only when a next page exists |
| 62 | Decline only when the prompt is optional |

The engine retains the complete count list, including choices beyond 30 or 300.
Counts must be strictly increasing, payable cannot exceed declared, and declared
cannot exceed i64::MAX (the existing DSL Literal binding domain). Every offered
commit action binds the exact declared value, never its page index or ordinal.
Navigation changes only the owned prompt page and its observation/digest. It
does not pay a cost, consume OPT, move cards, resolve another effect, or advance
the turn. Foreign-seat and masked actions remain rejected. Clone continuations
own independent page state.

Each menu entry includes typed `count_choice` metadata in desktop/debug DTOs,
and `countChoice` in the web state envelope. Its operation is `commit_count`,
`previous_page`, or `next_page`; exact count/page/identity values are decimal
strings. UI buttons dispatch the supplied action ID and obey the action mask.
The metadata explains choices and never grants legality. See TENSOR_SPEC for
the coordinated observation schema/hash update and model compatibility gate.


### Model action semantic compatibility

Model training contracts require `action_schema_version` in addition to the six observation/action shape fields. Its sole authority is Rust `action::space::SCHEMA_VERSION`, exported by PyO3 as `ACTION_SCHEMA_VERSION`. Equal shapes do not imply compatible actions. Missing, malformed, or mismatched action semantics reject inference, checkpoint resume/export, publication, download and loading. Upload parsers preserve declared artifact values; they never stamp current semantics onto old weights. Legacy database rows remain null and incompatible. New training and newly initialized random exports may capture the current contract; re-export requires the authentic compatible training tuple. SCHEMA_VERSION is now 2 for owned physical card pages; old action semantics are incompatible even when ACTION_SPACE_SIZE remains 2192. Full selector and runtime qualification is still required before admitting the blocked cards.
