# DSL Vocabulary Gaps Tracker

This file accumulates `BLOCKED` verdicts whose `gap_kind` is `dsl` (the engine has the primitive but the DSL lacks a verb that lowers to it). Entries are appended by `/batch-implement-cards-rust-dsl`.

Format per entry:

```
## <CARD_ID> — <clause name>
- Effect text: "..."
- Missing DSL verb / step kind / predicate: ...
- Lowers to engine API: <method on EffectContext that already exists>
- Suggested DSL syntax: <YAML shape>
- First reported: YYYY-MM-DD
```

## BT22-008 / BT22-017 — inherited end-of-turn DNA digivolve registration
- Effect text: "[End of Your Turn] This Digimon and another of your Digimon may DNA digivolve into a Digimon card in your hand."
- Missing DSL verb / step kind / predicate: Lowering for existing `alt_path_registration` declarative clauses with `kind: dna_digivolve`. YAML examples can spell the clause, but `code/digimon-engine/src/dsl_cards/mod.rs` currently leaves this declarative form in the unlowered catch-all branch.
- Lowers to engine API: the same alternate-path registration and action-mask channel used by normal DNA digivolve costs, producing a player-visible pending/action path rather than an automatic end-of-turn digivolve.
- Suggested DSL syntax: keep the existing `alt_path_registration` shape and require lowering for inherited clauses, including `timing: end_of_your_turn`, `kind: dna_digivolve`, material filters, target hand-card filter, and cost override.
- First reported: 2026-04-28

## BT22-015 — grant "this Digimon may attack" after When Digivolving
- Effect text: "[When Digivolving] ... Then, this Digimon may attack."
- Missing DSL verb / step kind / predicate: `ModifierType::MayAttack` / immediate attack permission is not exposed by the DSL modifier map, and there is no declarative step that lowers to the engine's attack-permission helper once the effect resolves.
- Lowers to engine API: `ModifierType::MayAttack` / `ModifierType::CanAttackUnsuspended` or the force-follow-up attack helper tracked in `docs/RUST_ENGINE_GAPS.md`.
- Suggested DSL syntax: `grant_attack_permission: { target: self, scope: player_or_digimon, expiry: end_of_turn }` for persistent permission, and a distinct `offer_follow_up_attack: { target: self }` when the printed text creates an immediate action prompt.
- First reported: 2026-04-28

## BT22-015 — count same-level pairs in own stack
- Effect text: "[When Digivolving] For every 2 cards with the same level in this Digimon's digivolution cards, return 1 of your opponent's Digimon to the bottom of the deck."
- Missing DSL verb / step kind / predicate: Formula support for "number of same-level pairs in this Digimon's digivolution cards" and repeat-count target selection derived from that formula.
- Lowers to engine API: stack inspection plus repeated `return_to_deck(..., DeckEnd::Bottom)` after each player-visible target selection.
- Suggested DSL syntax: `formula: { aggregate: same_level_pairs, zone: self_sources }` feeding `repeat: <formula>` around a `select_opponent_permanent` + `return_to_deck_bottom` step.
- First reported: 2026-04-28

## BT17-078 — bottom-deck all opponent Digimon sharing chosen level
- Effect text: "[On Play] [When Digivolving] ... place all of your opponent's Digimon with the same level as 1 of their Digimon at the bottom of the deck."
- Missing DSL verb / step kind / predicate: Binding one selected opponent Digimon's level and applying a mass same-level filter to every opponent permanent. The DSL has selection and aggregate helpers, but lacks a reusable "bind selected property, then for-each matching permanents" pattern.
- Lowers to engine API: select opponent permanent, read selected level, then call `return_to_deck(..., DeckEnd::Bottom)` for each opponent permanent whose top card has that level.
- Suggested DSL syntax: `bind_selected: { name: chosen_level, selector: opponent_digimon, property: level }` followed by `for_each_opponent_permanent: { where: { level_eq: "$chosen_level" }, do: return_to_deck_bottom }`.
- First reported: 2026-04-28
