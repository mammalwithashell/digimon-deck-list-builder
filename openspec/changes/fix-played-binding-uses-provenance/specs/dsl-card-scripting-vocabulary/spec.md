## ADDED Requirements

### Requirement: `bind_as` on play verbs binds a stable card identity, not a positional slot

When the DSL author writes `bind_as: <name>` on a play verb that creates a battle-area permanent (`play_from_hand_free`, `play_from_revealed_free`, `play_from_materials`, `play_union_bound_free`, `play_token`, and any future play-permanent verbs), the engine SHALL store the binding as a stable card identity (a `ProvenanceToken` keyed to the played card's `CardHandle`), NOT as a positional `PermanentHandle { player, index }`. Downstream verbs that consume the binding via the strict resolver SHALL resolve the token at consume time and treat resolution failure as a silent no-op.

A play-verb binding survives stack-changing events (regular digivolve, DNA digivolve, deletion, area swap) only insofar as the played card remains the top card of a battle-area permanent. Once the played card is no longer a top card — because the permanent was deleted, the played card became a digivolution card under a different top, or the permanent was consumed in DNA digivolve — the strict resolver fails to resolve and any downstream verb that targets the binding through that path silently skips.

This contract applies whether the binding is consumed inside the same effect resolution as the play step, or whether the binding is captured by `schedule_delayed` and consumed at a future drain boundary. The token-resolve cost is paid uniformly.

#### Scenario: Played Digimon stays on field through opponent's turn, delayed return-to-hand fires correctly

- **WHEN** a DSL clause uses `play_from_hand_free: { bind_as: played }` followed by `schedule_delayed: { when: end_of_opponents_next_turn, body: [{ return_to_hand: { target: { permanent: played } } }] }`, the played Digimon stays on the field with no stack changes until the end of the opponent's turn
- **THEN** at end of the opponent's turn the scheduled `return_to_hand` resolves the `played` binding to the same permanent and bounces it normally (top card to owner's hand, digivolution cards to trash)

#### Scenario: Played Digimon is consumed in DNA digivolve before opponent's EOT

- **WHEN** the same `play_from_hand_free` + `schedule_delayed` pattern runs, then before the scheduled drain the played Digimon is consumed as one of the two materials in a DNA digivolve (the played card becomes a digivolution card under the merged stack's new top)
- **THEN** at end of the opponent's turn the scheduled `return_to_hand` resolves the `played` binding via the provenance token, finds that the played card is no longer a battle-area top, and silently skips with no return-to-hand and no trashing of the merged stack's digivolution cards

#### Scenario: Played Digimon is regularly digivolved before opponent's EOT

- **WHEN** the same `play_from_hand_free` + `schedule_delayed` pattern runs, then before the scheduled drain the played Digimon is regularly digivolved (a new top card is pushed onto its `card_sources` and the played card becomes a digivolution card)
- **THEN** at end of the opponent's turn the scheduled `return_to_hand` resolves the `played` binding via the provenance token, finds that the played card is no longer a battle-area top, and silently skips with no return-to-hand event fired

#### Scenario: Played Digimon is deleted by another effect before opponent's EOT

- **WHEN** the same `play_from_hand_free` + `schedule_delayed` pattern runs, then before the scheduled drain the played Digimon is deleted by an opponent's effect or own-effect
- **THEN** at end of the opponent's turn the scheduled `return_to_hand` resolves the `played` binding via the provenance token, finds no battle-area permanent for it, and silently skips with no further side-effects

#### Scenario: Play verb fails to produce a permanent

- **WHEN** a `play_from_hand_free: { bind_as: played }` step runs but the play does not succeed (the hand card has no valid target frame, or a replacement effect cancels the play)
- **THEN** no binding is inserted under `played`, and downstream consumers see no binding and silently skip

#### Scenario: Binding consumed synchronously in same resolution

- **WHEN** a play-verb `bind_as` is consumed by a follow-up step within the same effect resolution (not via `schedule_delayed`)
- **THEN** the consume site resolves the token, the played permanent is still the battle-area top, and the verb executes against the resolved handle exactly as if a positional handle had been bound

### Requirement: `schedule_delete_played_at_turn_end` preserves permissive carrier-deletion semantics

The DSL verb `schedule_delete_played_at_turn_end` SHALL resolve a play-verb `bind_as` binding via a **permissive** carrier-aware resolver, not the strict top-card resolver used by other consumers. The permissive resolver SHALL yield the carrier `PermanentHandle` whenever the played card is anywhere in a battle-area permanent's `card_sources` — top card OR digivolution card.

This preserves the "delete the Digimon this effect played" semantics for cards like EX11-022 Karakurumon, EX11-061 Mirai Kinosaki, and P-165 ShoeShoemon, where the rules-intent is to delete the carrier even after a digivolve buries the played card.

#### Scenario: Played Digimon stays as top card

- **WHEN** `play_from_hand_free: { bind_as: puppet_played }` is followed by `schedule_delete_played_at_turn_end: { binding: puppet_played }` and the played Digimon remains the top card of its permanent until end of turn
- **THEN** at end of turn the scheduled deletion fires against the played Digimon's permanent

#### Scenario: Played Digimon was digivolved before end of turn

- **WHEN** the played Digimon is digivolved into something else before end of turn (the played card becomes a digivolution card)
- **THEN** at end of turn the scheduled deletion fires against the CARRIER permanent (the new top) — the permissive resolver yields the carrier handle

#### Scenario: Played Digimon has left play

- **WHEN** the played Digimon has been returned to hand, returned to deck, trashed, or otherwise left the battle area before end of turn
- **THEN** at end of turn the scheduled deletion is a silent no-op — the provenance token does not resolve to any battle-area permanent

### Requirement: Selection-produced `bind_as` bindings remain positional

`bind_as` on selection verbs (`select_own_permanent`, `select_opponent_permanent`, `select_own_breeding_permanent`, and any other intra-resolution selection) SHALL continue to produce positional `PermanentHandle` bindings. These bindings are scoped to a single effect resolution and do not benefit from identity tracking.

#### Scenario: Selection binding consumed in same resolution

- **WHEN** `select_own_permanent: { bind_as: tgt }` runs and a follow-up step in the same effect body consumes `tgt`
- **THEN** the binding resolves directly to a `PermanentHandle` with no provenance lookup

#### Scenario: Selection binding semantics unchanged from prior behavior

- **WHEN** the spec is applied to existing DSL cards
- **THEN** no card YAML that uses `select_*: { bind_as: ... }` requires any change to its behavior or its compiled output
