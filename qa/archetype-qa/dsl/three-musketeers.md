# Archetype DSL Implementation: Three Musketeers
Date: 2026-09-12
Total cards in pool (this slice): 8
Processed this run: 8
Pipeline: batch-implement-cards-rust-dsl (campaign three-musketeers-1, slice `ex7-line`)

Slice: EX7-005, EX7-008, EX7-040, EX7-051, EX7-010, EX7-043, EX7-011, EX7-044
(ordered low → high digivolution stage). Prior EX7 [Three Musketeers] cards
(EX7-048 Gundramon, EX7-070 Der Blitz, EX7-071 Hurricane Screw Shot, EX7-073
BeelStarmon (X Antibody)) were already IMPLEMENTED and are used as real
fixtures by this slice's tests.

Sources: official Bandai DB bundles (`data/card_bundles/EX7-0xx.md`) for
printed text / digivolution circles (note: cards.json drops Deputymon's
printed [Once Per Turn]); card images EX7-005/008/010/040/044/051 (`.webp`);
DCGO `EX7_0xx.cs` for resolution order.

## Summary
- IMPLEMENTED: 7
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 1
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| EX7-005 | Kapurimon | IMPLEMENT | BLOCKED (engine) | n/a | 0/0 | Needs the `OnAddDigivolutionCards` trigger timing (open gap, shared with BT25-005 Pagumon) |
| EX7-008 | ToyAgumon (Red) | IMPLEMENT | IMPLEMENTED | self-audit | 9/9 | Reveal 3, two mandatory buckets (TM-in-text / cost-6 Option), bottom rest; inherited [Your Turn] +2000; 2 alt-paths |
| EX7-040 | ToyAgumon (Black) | IMPLEMENT | IMPLEMENTED | self-audit | 11/11 | Optional trash-TM-hand-card cost → Draw 2; inherited <Reboot>; 2 alt-paths |
| EX7-051 | Sparrowmon | IMPLEMENT | IMPLEMENTED | self-audit | 12/12 | [SoMP] union (hand/trash) TM Option under own non-token Digimon → Draw 1; inherited <Retaliation>; 2 alt-paths |
| EX7-010 | Deputymon | IMPLEMENT | IMPLEMENTED | self-audit | 13/13 | Shared [WD][WA][OPT] trash 1 Option from ANY Digimon's sources; [Your Turn] trait grant via NEW `grant_traits`; inherited +2000; 2 alt-paths |
| EX7-043 | Tankmon | IMPLEMENT | IMPLEMENTED | self-audit | 12/12 | Return 3 TM cards (hand/trash mix) to deck top via NEW `return_union_bound_to_deck` → De-Digivolve 1; inherited <Reboot>; 2 alt-paths |
| EX7-011 | Megadramon | IMPLEMENT | IMPLEMENTED | self-audit | 11/11 | Union TM Option placed under self → delete opp Digimon ≤6000 DP; inherited <Piercing>; 2 alt-paths |
| EX7-044 | Gigadramon | IMPLEMENT | IMPLEMENTED | self-audit | 9/9 | Reveal 4, mandatory TM Option placed under self, rest top/bottom, if placed delete opp Digimon/Tamer cost ≤3; inherited <Collision>; 2 alt-paths |

Test command (all green, 2026-09-12):
`cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex7_0`

## Engine-Gap Blocked Cards
### EX7-005 Kapurimon
- Effect text (inherited): "[Your Turn] [Once Per Turn] When effects place Option cards with the [Three Musketeers] trait in this Digimon's digivolution cards, gain 1 memory."
- Missing engine API: an `OnAddDigivolutionCards` trigger timing fired from the stack-source-placement path (place_as_bottom_source / place_as_top_source / material placement) carrying the host permanent + the added cards + the placing effect (DCGO `CanTriggerOnAddDigivolutionCard(hashtable, permanentCondition, cardEffectCondition, cardCondition)`).
- Suggested addition: `Timing::OnAddDigivolutionCards` (DSL `when: on_add_digivolution_cards`) → `EffectTiming::OnAddDigivolutionCards`, with `event_target_is_source` / `event_card_trait_has` / `event_card_kind` gating. Body is then `gain_memory: 1` with `once_per_turn: true`, `scope: inherited`, `active_when: your_turn`. Tracked in `docs/RUST_ENGINE_GAPS.md` § "`OnAddDigivolutionCards` trigger timing" (card list updated).

## DSL-Vocab-Gap Blocked Cards
(none — the two vocabulary gaps this slice hit were closed in-slice, see below)

## Substrate widened by this slice (rule 28)
- **`grant_traits` on `kind: aura`** (G-DSL-AURA-GRANT-TRAITS, `qa/resolved-gaps.md`): continuous "gains the [X] trait" — lowers to `ChangeTraits` + `Traits { add, replace: false }`. Driver EX7-010.
- **`return_union_bound_to_deck: { binding, position }`** (G-DSL-RETURN-UNION-BOUND-TO-DECK, `qa/resolved-gaps.md`): return a `select_union_zone` pick to the top/bottom of its owner's deck from hand, trash or a stack; new `EffectContext::return_hand_card_to_deck`. Driver EX7-043.
- Validator: `place_as_bottom_source.bind_placed_as` is now declared as an optional binding so an "If this effect placed" gate (`if { binding_present }`) validates. Driver EX7-044.

## New Patterns Discovered
- "Trash 1 Option from ANY 1 Digimon's digivolution cards" (either owner): `select_any_permanent` over `{ kind: digimon, source_count: { filter: { kind: option }, at_least: 1 } }` then owner-routed `if binding_owner` → `select_own_sources` / `select_opponent_sources { target: host, min: 1, max: 1 }` → `trash_selected_sources` (EX7-010). Worth a RUST_DSL_TEST_API.md §8 note.
- "Return N cards from hand or trash to the top of the deck" cost: N sequential `select_union_zone` picks (first `optional: true, cost: true`, the rest mandatory) each followed by `return_union_bound_to_deck { position: top }`; pick order = deck order (EX7-043).
