# Archetype DSL Implementation: BT25 "machine" slice
Date: 2026-06-05
Total cards in pool: 3
Processed this run: 3
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 1
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 2
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT25-062 | Kokuwamon | IMPLEMENT | IMPLEMENTED | self | 12/12 | SOMP free-digivolve self from hand (Machine/Cyborg/TS, memory<=4, optional); inherited +1000 DP; TS Lv2 cost-0 alt-path. `target: source` required (not `self`). |
| BT25-066 | Guardromon | IMPLEMENT | BLOCKED (dsl) | self | 0/0 | Link-card-trash would-leave replacement cost not expressible. |
| BT25-074 | Tankdramon | IMPLEMENT | BLOCKED (dsl) | self | 0/0 | Reveal-then-play-with-cost-reduced-by-5 not expressible (reveal play is free-only). |

## DSL-Vocab-Gap Blocked Cards

### BT25-066 Guardromon  [G-DSL-LINK-TRASH-AS-REPLACEMENT-COST]
- Effect text: "[All Turns] When this Digimon would leave the battle area, by trashing 1 of its link cards, it doesn't leave."
- Missing DSL verb: select + trash one of a permanent's own LINK cards as a `kind: replacement` cost (then cancel the leave). `ReplacementCostBody` supports only `delay_self`; `choose:` supports only `from: hand`.
- Lowers to engine API: a new selection over `source_permanent.linked_cards` + link-card trash (`OnLinkedCardTrashed` already wired in `combat.rs`) + existing `cancel_replacement`. Substrate (`Permanent.linked_cards`, `EffectTiming::OnLinkedCardTrashed`) exists; only DSL vocabulary missing.
- Suggested DSL syntax: `choose: { from: linked_cards, min: 1, max: 1 }` with `outcome: prevent`, or a `trash_linked_card_and_cancel_replacement` step. Replacement must be `optional: true` (DCGO `SetIsSkippable(true)`).
- Other clauses (Blocker, inherited +1000 DP, TS-trait alt-digivolve) are individually expressible; card BLOCKED per no-approximations because the link-trash player choice can't surface.

### BT25-074 Tankdramon  [G-DSL-PLAY-FROM-REVEALED-COST-REDUCED]
- Effect text: "[When Digivolving] [When Attacking] [Once Per Turn] Reveal the top 3 cards of your deck. You may play 1 play cost 12 or lower [D-Brigade] or [ACCEL] trait Digimon card among them with the cost reduced by 5. Trash the rest."
- Missing DSL verb: play a revealed card with play cost REDUCED by N (controller pays the remainder). `play_from_revealed_free` hard-codes `CostDelta::Free`; no `cost_delta` on the reveal-pool play step.
- Lowers to engine API: already present — `play_from_revealed_free` internally calls `Game::play_from_hand_with_cost_result_from_origin(... CostDelta ..., Reveal{..})`, which accepts any `CostDelta`. `enums::CostDelta::Reduce(i16)` exists. Hand analog `play_from_hand` already has `cost_delta` (BT15-096).
- Suggested DSL syntax: add `cost_delta: { reduce: 5 }` to `play_from_revealed_free` (or a new `play_from_revealed` step), threaded into the `from_origin` call.
- Secondary clauses (on_ally_played → opponent CannotDigivolve; inherited [Opponent's Turn] Reboot+Blocker) are individually expressible; card BLOCKED because the main WD/WA clause can't ship.

## New Patterns Discovered
- `effect_initiated_digivolve` with `target: self` silently no-ops (resolves to a Card, not a Permanent). Use `target: source`. Latent bug in 3 shipped cards (AD1-010, BT20-083, EX9-019) — routed to a separate background task. BT25-062 ships the correct `target: source` form.
