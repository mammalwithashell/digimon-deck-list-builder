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

---

# Slice `splash-digimon` (campaign three-musketeers-1) — 2026-09-13
Pipeline: batch-implement-cards-rust-dsl (single-agent run; scout/implement/review folded)
Slice: BT7-056, BT8-071, BT6-060, BT7-071, BT7-073, BT17-070, BT24-081
(ordered low → high digivolution stage). Sources: official Bandai DB bundles
(`data/card_bundles/<ID>.md`) for printed text / digivolution circles, card
images (`.webp`) for BT7-056 / BT8-071 / BT7-071 / BT7-073 / BT17-070 /
BT24-081 (BT6-060 is not in the local mirror — bundle text used), DCGO
`<ID>.cs` for resolution order.

## Summary
- IMPLEMENTED: 6
- PARTIAL: 0
- BLOCKED (engine): 1 (BT7-056 — `OnAddDigivolutionCards`, the same gap as BT25-005 / EX7-005)
- BLOCKED (dsl): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT7-056 | Dorumon | IMPLEMENT | BLOCKED (engine) | n/a | 0/0 | Inherited "[Your Turn][OPT] when one of your effects places a digivolution card under this Digimon, gain 1 memory" needs `OnAddDigivolutionCards` (+ an effect-controller gate); the [On Play] reveal-3 body is expressible but the card is not authored without its inherited clause |
| BT8-071 | Psychemon | IMPLEMENT | IMPLEMENTED | self-audit | 8/8 | [All Turns] Players can't reduce play costs → `flood_gate CannotReducePlayCost target_player: any` (ST13-08 twin); lock on both players, all turns, lifts on leaving |
| BT6-060 | Deputymon | IMPLEMENT | IMPLEMENTED | self-audit | 13/13 | [On Play] reveal 4 → 1 TM-trait Digimon and/or 1 cost-7 Option to hand, rest TRASHED; [Your Turn] warp INTO a TM Digimon from hand for 6 ignoring requirements (`direction: into` + `condition: your_turn`) |
| BT7-071 | Loweemon | IMPLEMENT | IMPLEMENTED | self-audit | 8/8 | Hybrid: onto one of your purple Tamers as a Lv.3 purple Digimon (`source_treated_as: level_3_purple_digimon`, cost 2 = printed circle) |
| BT7-073 | KaiserLeomon | IMPLEMENT | IMPLEMENTED | self-audit | 10/10 | Purple-Tamer route for 2 + circles Lv.3/3, Lv.4/1; [WD] if a [Hybrid] source or [Koichi Kimura] is under it → <Retaliation> until end of opp's next turn (expiry verified) |
| BT17-070 | Gulfmon | IMPLEMENT | IMPLEMENTED | self-audit | 14/14 | [OP][WD] tuck a Lv.5 [Dark Masters]-text card from hand/trash as bottom source (union cost) → delete 1 opp Lv.≤5; [WA] return 7 cards from opp trash to THEIR deck bottom → unsuspend; Lv.5 w/[Dark Masters]-in-text/3 alt circle |
| BT24-081 | Titamon + SkullBaluchimon | IMPLEMENT | IMPLEMENTED | self-audit | 15/15 | <Rush><Piercing><Execute>; Assembly −6 [Titamon]×[SkullBaluchimon] (full play driven: 8 paid, both under); [OP][WD][WA] trash 1 hand card → delete ALL opp lowest-level Digimon (`for_each` + `lowest_level` aggregate); [On Deletion] may play [Titamon] / Lv.≤5 [Titan] from trash free; (Rule) Demon trait |

Test command (all green, 2026-09-13):
`cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt8_071 bt6_060 bt7_071 bt7_073 bt17_070 bt24_081`
(68 tests). `alt_path_printed_cost_guard` passes for all six cards (its 18
open failures are pre-existing EX12 entries untouched by this slice); the
`dsl` guard binary is green (920 passed).

## Engine-Gap Blocked Cards
### BT7-056 Dorumon
- Effect text (inherited): "[Your Turn] [Once Per Turn] When one of your effects places a digivolution card under this Digimon, gain 1 memory."
- Missing engine API: `OnAddDigivolutionCards` trigger timing (see `docs/RUST_ENGINE_GAPS.md`); DCGO `BT7_056.cs` additionally gates on the placing effect's controller (`cardEffect.EffectSourceCard.Owner == card.Owner`), so the trigger context should carry the effect controller. The [On Play] "reveal 3 → add 1 [X Antibody]-trait card and 1 [Kota Domoto] → bottom the rest" body is the BT18-007 two-bucket shape and needs nothing new.

## DSL-Vocab-Gap Blocked Cards
- none this slice.

## Substrate widened by this slice (rule 28)
- none — every clause lowered with existing vocabulary.

## New Patterns / findings
- **Hybrid Tamer digivolve as an alt-path**: `from: { kind: tamer, color_is: <c> }` + `source_treated_as: level_<n>_<c>_digimon` is the shipping shape for "digivolve onto one of your <colour> Tamers as if the Tamer is a level N <colour> Digimon" (BT7-071 / BT7-073). The `cost:` literal is the printed Tamer-route cost (Loweemon: the circle's 2; KaiserLeomon: the printed "for a memory cost of 2"). The route needs the result card's evo_costs to carry a matching Lv.N/colour row — the bare printed-circle alt-path backfills it for DSL fixtures. First production YAML users of `source_treated_as`.
- **"Delete all … with the lowest level"**: `for_each { over: { of: opponent, zone: [battle_area], kind: digimon, level_matches_aggregate: { selector: lowest_level, of: opponent } } }` + `delete_permanent` (BT23-058 / BT25-093 sibling) — no selection, ties all go.
- **Opponent-trash → opponent-deck-bottom cost**: `select_count_capped_multi { of: opponent, zone: trash, min: N, max: N, optional_zero: false }` + `return_trash_list_to_deck_bottom { of: opponent }` routes to the OPPONENT's deck; keep `optional_zero: false` and let the clause's `outer_prompt` carry the decline, otherwise a 0-pick would run the "unsuspend" tail for free (BT17-070).
- Test fixture gotcha: `attack_player` resolves the security check after a mid-attack [When Attacking] cost, so the checked security card lands in the defender's trash — assert opponent-trash costs by the deck delta, not the trash count. `end_turn` into a player with an empty deck ends the game (deck-out) — give both players decks in expiry tests.
