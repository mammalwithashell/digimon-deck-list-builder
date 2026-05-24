# zombie-permanent-cleanup Specification

## Purpose

Define the engine's contract that any code path which empties a `Permanent`'s
`card_sources` Vec — moving the carrier's only card into another zone or onto
another permanent — MUST remove the carrier slot from `Player::battle_area`
before any trigger fan-out can observe it. Pair this Layer 1 mutation-side
cleanup with Layer 2 read-side defensive guards on trigger-queue callers that
iterate `battle_area`, so a transient zombie permanent never crashes the
engine. This capability eliminates the `Permanent must have at least one card`
panic family across material-extraction sibling sites and locks tracking
artifacts to the actual fix scope.

## Requirements

### Requirement: Material extraction MUST NOT leave a zombie carrier in battle_area

Any engine code path that mutates a `Permanent`'s `card_sources` Vec in a way that can empty it — moving the carrier's only card into another zone or onto another permanent — SHALL invoke `Game::soft_remove_if_emptied(carrier_handle)` after the mutation, and SHALL pair with `Game::shift_handle_after_soft_remove` to adjust any in-flight `PermanentHandle` for the resulting index shift, before any trigger fan-out can observe the carrier.

This requirement applies to the following currently-known sibling sites; the engine MUST satisfy each:

1. `EffectContext::play_from_materials_suppress_on_play` — when the played source was the carrier's only card AND the underlying `play_from_hand_with_cost_result_from_origin_suppress` call returned `Played(_)`.
2. `Game::place_as_bottom_source_observed` — when `source` is `CardSourceRef::Material(carrier, _)` and the take emptied the carrier.
3. The replacement-redirect-to-Trash branch in the `WhenWouldPlaceInSecurity` handler (`game_actions.rs` ~ line 6141).
4. The place-into-security from material path (`game_actions.rs` ~ line 6192).
5. `Game::trash_source_ref` — when the source ref resolved to the carrier's only remaining card.
6. `EffectContext::trash_card_source` — when the by-handle position resolved to the carrier's only remaining card.

For each site, a behavioral regression test SHALL exist that places a single-source carrier, exercises the sibling operation, and asserts no `Permanent` remains in any `Player::battle_area` with `card_sources.is_empty()`.

#### Scenario: play_from_materials on a single-source carrier removes the carrier slot

- **WHEN** an effect calls `EffectContext::play_from_materials(carrier, 0, CostDelta::Free)` and `carrier.card_sources.len() == 1` at call time
- **AND** the underlying play succeeds (returns `Played(_)`)
- **THEN** the carrier's slot SHALL be removed from `Player::battle_area`
- **AND** no `Permanent` in any player's `battle_area` SHALL have an empty `card_sources` Vec
- **AND** the played card SHALL appear as its own new permanent in the controller's `battle_area`

#### Scenario: play_from_materials Pending branch defers cleanup until resume

- **WHEN** `play_from_materials` enters the `Pending` branch (parked selection)
- **THEN** the carrier MAY temporarily have empty `card_sources` for the duration of the parked selection
- **AND** the Layer 2 read-side guards SHALL prevent any panic during the parked window
- **AND** upon the parked selection's resume, the engine SHALL either remove the carrier slot (if play succeeds) or restore the source to the carrier (if play fails / rollback)

#### Scenario: play_from_materials Failed rollback restores the source and does NOT remove the carrier

- **WHEN** `play_from_materials` enters the `Failed` branch (e.g., battle area full)
- **THEN** the source SHALL be reinserted at its original `source_index` in the carrier's `card_sources`
- **AND** the carrier SHALL remain in `battle_area` with its `card_sources` Vec non-empty

#### Scenario: place_as_bottom_source_observed with Material source from single-source carrier removes the carrier slot

- **WHEN** `Game::place_as_bottom_source_observed(CardSourceRef::Material(carrier, 0), target, _)` is called
- **AND** `carrier.card_sources.len() == 1` at call time
- **AND** the take + push-under-target succeeds
- **THEN** the carrier's slot SHALL be removed from `battle_area`
- **AND** the `target` `PermanentHandle` SHALL be adjusted via `shift_handle_after_soft_remove` if the removal shifts target's index
- **AND** the target's new bottom source SHALL be the card taken from the carrier

#### Scenario: trash_source_ref of the carrier's only remaining source removes the carrier slot

- **WHEN** `Game::trash_source_ref(SourceSelectionRef { permanent: carrier, card })` is called
- **AND** `card` is the carrier's only entry in `card_sources`
- **THEN** the card SHALL be moved to the carrier-owner's `trash`
- **AND** the carrier's slot SHALL be removed from `battle_area`
- **AND** no `Permanent` SHALL have empty `card_sources`

#### Scenario: trash_card_source by handle of the carrier's only remaining source removes the carrier slot

- **WHEN** `EffectContext::trash_card_source(carrier, card)` is called
- **AND** `card` is the carrier's only entry in `card_sources`
- **THEN** the card SHALL be moved to the card-owner's `trash`
- **AND** `fire_digivolution_card_trashed` SHALL be enqueued before the slot removal so observer dispatch sees the source-trash event
- **AND** the carrier's slot SHALL be removed from `battle_area`

#### Scenario: replacement-redirect-to-Trash from Material on single-source carrier removes the carrier slot

- **WHEN** a `WhenWouldPlaceInSecurity` replacement handler returns `ReplacementOutcome::Redirected(Trash(...))` with the original source being `CardSourceRef::Material(carrier, 0)` of a single-source carrier
- **AND** the take routes the card to trash
- **THEN** the carrier's slot SHALL be removed from `battle_area`

#### Scenario: place-into-security from Material on single-source carrier removes the carrier slot

- **WHEN** the place-into-security path takes `CardSourceRef::Material(carrier, 0)` of a single-source carrier and pushes it to `player.security`
- **THEN** the carrier's slot SHALL be removed from `battle_area`

### Requirement: Linked cards on a soft-removed carrier flow to trash and fire OnLinkedCardTrashed

When `Game::soft_remove_if_emptied(handle)` removes a carrier slot whose `linked_cards` Vec is non-empty, each linked card SHALL be routed to the carrier-owner's `trash`, and `EffectTiming::OnLinkedCardTrashed` SHALL be enqueued per player so observers see the linked-card movement. This mirrors `combat::finalize_permanent_deletion`'s linked-card handling and is already implemented in the helper; this requirement locks it in for the broader sibling-extraction surface.

#### Scenario: carrier with linked cards is soft-removed during material extraction

- **WHEN** any material-extraction sibling empties a carrier that has 2 linked cards
- **AND** the carrier's slot is removed
- **THEN** both linked cards SHALL appear in the carrier-owner's `trash`
- **AND** `OnLinkedCardTrashed` SHALL be enqueued for each player exactly once

### Requirement: Effect-queue battle_area iterators MUST tolerate zombie permanents

Read-side functions in `effect_queue.rs` that iterate `Player::battle_area` and read `Permanent::top_card()` SHALL guard against empty `card_sources`. A `Permanent` with empty `card_sources` SHALL be skipped (`continue` in a loop, `None` returned from a lookup) rather than panicking. This is the Layer 2 defensive guarantee — independent of the Layer 1 mutation-side cleanup — that a transient zombie permanent (e.g., during a `Pending` `play_from_materials` parked selection) does not crash the engine.

The Layer 2 guards SHALL cover at minimum:
- `Game::find_event_gated_delay_permanent` (battle_area iter looking up by `top_card().card_index`)
- `Game::event_gated_delay_source` (reads `perm.top_card().handle()` for source-card match)
- (already covered by PR #533: `Game::enqueue_from_permanent`, `Game::enqueue_from_breeding_permanent`, `Game::queued_effect_source_is_live`, `Game::top_card_handle`)

#### Scenario: find_event_gated_delay_permanent skips a zombie permanent during scan

- **WHEN** a player's `battle_area` contains a zombie `Permanent` (`card_sources.is_empty()`)
- **AND** `Game::find_event_gated_delay_permanent(owner, card_index, timing)` is called
- **THEN** the function SHALL NOT panic
- **AND** the function SHALL skip the zombie slot and continue scanning subsequent slots
- **AND** the function SHALL return the first non-zombie matching delayed-Option permanent, or `None` if none matches

#### Scenario: event_gated_delay_source returns None on a zombie source permanent

- **WHEN** a `QueuedEffect` references a `source_permanent` whose `card_sources` is empty
- **AND** `Game::event_gated_delay_source(&qe)` is called
- **THEN** the function SHALL NOT panic
- **AND** the function SHALL return `None`

### Requirement: Tracking artifacts MUST reflect the actual fix scope

`qa/archetype-qa/engine-gaps.md` SHALL distinguish between the digivolve-from-material variant (resolved by PR #533) and the broader material-extraction siblings (resolved by this change). `qa/archetype-qa/panic-families.json` SHALL list one `family_id` per distinct resolved-/open-state group, with `status: "open"` until the corresponding siblings close.

#### Scenario: gaps.md and panic-families.json are consistent post-change

- **WHEN** this change lands
- **THEN** `qa/archetype-qa/engine-gaps.md` SHALL contain a `G-PERMANENT-EMPTY-DIGIVOLVE-FROM-MATERIAL` entry marked resolved with PR #533, and a `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` entry marked resolved with this change's PR
- **AND** `qa/archetype-qa/panic-families.json` SHALL have both `family_id`s with matching resolved status
- **AND** no entry SHALL contradict another (e.g., `RESOLVED` in markdown while `"status": "open"` in JSON)
