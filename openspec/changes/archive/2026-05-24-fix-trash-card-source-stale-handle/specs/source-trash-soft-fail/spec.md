## ADDED Requirements

### Requirement: `trash_card_source` SHALL soft-fail on invalidated handles instead of panicking

`EffectContext::trash_card_source(perm: PermanentHandle, card: CardHandle)` MUST return `bool` and MUST NOT panic for any of the following rules-natural conditions:
- the carrier slot `perm.player.battle_area[perm.index]` is absent (e.g., the permanent was soft-removed or deleted between handle capture and trash attempt);
- the carrier slot exists but `card_sources` is empty (no body, no sources — i.e., a zombie slot mid-cleanup);
- `card_sources` is non-empty but no source has `handle() == card` (the captured card has already been trashed, returned to hand/deck, or extracted as material).

In each of those conditions the function MUST return `false` with no mutation, no observer dispatch, no `soft_remove_if_emptied` side-effects, and no logged error. When the card IS present, the function MUST trash it (move to its owner's trash), fire `OnDigivolutionCardTrashed`, drain the observer queue, run `soft_remove_if_emptied(perm)`, and return `true`.

This mirrors DCGO `ITrashDigivolutionCards.TrashDigivolutionCards()` ([DCGO/Assets/Scripts/Script/CardController.cs:5181](DCGO/Assets/Scripts/Script/CardController.cs:5181)): guard chain at entry, filter targets against the live `DigivolutionCards` list, silently yield-break on empty/invalid input.

#### Scenario: Stale CardHandle silently no-ops
- **WHEN** an effect calls `ctx.trash_card_source(perm, card)` where `perm` is a live permanent but `card` is no longer in `perm.card_sources`
- **THEN** the function returns `false`, no card moves to trash, no observer fires, the existing `card_sources` is unchanged, and no panic occurs

#### Scenario: Missing carrier slot silently no-ops
- **WHEN** an effect calls `ctx.trash_card_source(perm, card)` where `perm.index >= battle_area.len()` (the permanent was removed by a sibling effect)
- **THEN** the function returns `false` with no mutation and no panic

#### Scenario: Empty-stack zombie carrier silently no-ops
- **WHEN** an effect calls `ctx.trash_card_source(perm, card)` and `battle_area[perm.index].card_sources.is_empty()`
- **THEN** the function returns `false`; `top_card()` is NOT called on the empty stack; no panic

#### Scenario: Valid trash returns true and dispatches observers
- **WHEN** an effect calls `ctx.trash_card_source(perm, card)` and `card` is present in `perm.card_sources`
- **THEN** the function removes the source, pushes it to `removed.owner`'s trash, fires `OnDigivolutionCardTrashed` with the correct host card / source card / event cause, drains the observer queue, calls `soft_remove_if_emptied(perm)`, and returns `true`

#### Scenario: Picked source vanishes between submit and trash (replay reproducer)
- **WHEN** a SourceMulti's picked `source_ref` was valid at install time but an intervening observer effect trashed the underlying source before the submit callback's tail step runs `trash_card_source(source_ref.permanent, source_ref.card)`
- **THEN** the call returns `false`, the resolution proceeds (no panic), and a `per_selected` iteration over surviving picks sees only the actually-trashed entries

### Requirement: `install_source_multi_selection` SHALL revalidate picks at submit time

`install_source_multi_selection`'s pick callback ([code/digimon-engine/src/effect_context/selections.rs:2586](code/digimon-engine/src/effect_context/selections.rs:2586)) MUST verify that the picked `source_ref.card` is still present in `game.player(source_ref.permanent.player).battle_area[source_ref.permanent.index].card_sources` BEFORE adding the ref to `next_picked`. If the card is no longer present, the callback MUST re-install the SourceMulti pending with the unchanged `picked` set (preserving prior valid picks) and refreshed candidates from `source_multi_candidates`, NOT add the stale ref to `next_picked`.

This mirrors DCGO `SelectCardEffect.SetUp(... customRootCardList: selectedPermanent.DigivolutionCards ...)` ([DCGO/Assets/Scripts/Script/CardEffectCommons/TrashDigivolutionCards.cs:125](DCGO/Assets/Scripts/Script/CardEffectCommons/TrashDigivolutionCards.cs:125)): the picker reads the live list at display time, so a vanished candidate is never advertised.

#### Scenario: Picked card vanished — re-install with refreshed candidates
- **WHEN** a SourceMulti pending is open with snapshot candidate `(slot_i, source_idx_j) ↔ card_handle_X` and an intervening observer drains `card_handle_X` from slot_i's stack, then the agent submits the action selecting card_handle_X
- **THEN** the callback detects the staleness, does NOT add card_handle_X to `next_picked`, and re-invokes `install_source_multi_selection` with the unchanged prior `picked` set and a freshly enumerated candidate list

#### Scenario: Picked card still present — captured normally
- **WHEN** the agent submits an action whose `source_ref` is still in its permanent's `card_sources`
- **THEN** the callback adds the ref to `next_picked` and recurses; behavior identical to current implementation

#### Scenario: Final callback receives only valid picks
- **WHEN** `picked.len() == max` triggers the final callback
- **THEN** every `SourceSelectionRef` in `picked` was validated against the live stack at the moment of its own submit (no guarantee about validity at final-callback time — that's `trash_card_source`'s job)

### Requirement: DSL `TrashSelectedSources` and `TrashUnionBound` SHALL iterate picks with per-call soft-fail

The DSL step handlers for `CompiledStep::TrashSelectedSources` and `CompiledStep::TrashUnionBound` ([code/digimon-engine/src/dsl_cards/step/zone_moves.rs:211](code/digimon-engine/src/dsl_cards/step/zone_moves.rs:211), [:300](code/digimon-engine/src/dsl_cards/step/zone_moves.rs:300)) MUST iterate their bound `SourceSelectionRef`s and call `ctx.trash_card_source(ref.permanent, ref.card)` per ref. The bool return MUST be ignored for the purpose of this change (no surface DSL behavior change). The loop MUST NOT abort on a `false` return — subsequent refs in the same binding still get a trash attempt.

This mirrors DCGO `ITrashDigivolutionCards`'s "filter then trash survivors" shape: each requested target is attempted; failures are silent.

#### Scenario: Cross-carrier picks with one stale entry
- **WHEN** bound `source_refs = [(carrier_A, card_X), (carrier_B, card_Y)]`, `card_X` was trashed by a prior observer between SourceMulti submit and `TrashSelectedSources` running, and `card_Y` is still live
- **THEN** the loop calls `trash_card_source(carrier_A, card_X)` (returns `false`, no-op), then `trash_card_source(carrier_B, card_Y)` (returns `true`, card_Y trashed). No panic.

#### Scenario: All picks survive
- **WHEN** every bound `source_ref` is still present in its carrier
- **THEN** the loop trashes every ref, fires observers per trash, and produces identical behavior to today

#### Scenario: All picks stale
- **WHEN** every bound `source_ref` is stale (an observer cascade trashed all picked sources before the DSL step runs)
- **THEN** the loop iterates and silently no-ops; the surrounding clause (e.g., `select_opponent_permanent` for cost reduction in EX10-033 Clause B) still runs per current semantics
