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

# Slice `ex7-tops` (campaign three-musketeers-1) — 2026-09-13
Pipeline: batch-implement-cards-rust-dsl (single-agent run; scout/implement/review folded)
Slice: EX7-013, EX7-059, EX7-066, BT25-005, BT25-085, P-170, BT21-054, BT21-074
(ordered low → high digivolution stage). Sources: official Bandai DB bundles
(`data/card_bundles/<ID>.md`) for printed text / digivolution circles, card
images (`.webp`) for EX7-013 / EX7-059 / BT25-085 / BT21-074 (the image
carries Satellamon's `Sup. 4` circle the DB bundle omits), DCGO `<ID>.cs` for
resolution order.

## Summary
- IMPLEMENTED: 7
- PARTIAL: 0
- BLOCKED (engine): 1 (BT25-005 — re-confirmed, `OnAddDigivolutionCards`)
- BLOCKED (dsl): 0 (BT25-085's prior dsl block closed in-slice)
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| EX7-013 | MagnaKidmon | IMPLEMENT | IMPLEMENTED | self-audit | 14/14 | [OP][WD] may use TM Option free then draw until 6 (NEW formula `draw.count`); [EoYT][OPT] trash Option from own sources → TM Digimon gains SA+1 for the turn + force_attack |
| EX7-059 | BeelStarmon ACE | IMPLEMENT | IMPLEMENTED | self-audit | 13/13 | <Blast Digivolve>, Overflow -4; [OP][WD] return 1 Option from trash then may use TM Option free; [WA][OPT] trash own Option source → may use TM Option free |
| EX7-066 | Chaos Triangular | IMPLEMENT | IMPLEMENTED | self-audit | 10/10 | Inherited effect-trash → +3000 DP until opp EoT; use_requirement; [Main] delete ≤9000 +3000 per differently-named TM Digimon (NEW `distinct_names_count`) then place self under a TM Digimon; [Security] delete ≤12000 |
| BT25-005 | Pagumon | IMPLEMENT | BLOCKED (engine) | n/a | 0/0 | `OnAddDigivolutionCards` trigger timing still absent (re-confirmed) |
| BT25-085 | BeelStarmon (DUAL) | IMPLEMENT | IMPLEMENTED | self-audit | 18/18 | Prior dsl block closed: use TM/TS Option from hand OR own sources (union + `use_option_bound` Material origin); trash Option from any own stack/link cards → unsuspend ([WD][WA][Counter]); Option face Use Req + [Main] highest-level delete + optional TM bottom-source; Arts Digivolve |
| P-170 | AvengeKidmon | IMPLEMENT | IMPLEMENTED | self-audit | 10/10 | Rule trait; when-played −6 by returning 3 TM-text cards (optional, interactive pay_cost); Raid/Blocker/Retaliation; [On Deletion] may play TM Digimon cost ≤12 from hand/trash free |
| BT21-054 | Shotmon | IMPLEMENT | IMPLEMENTED | self-audit | 9/9 | [On Play] optional trash Appmon/TM source from any own Digimon → De-Digivolve 1; Link cost 1 + link box +2000; [When Linking] delete cost ≤3 |
| BT21-074 | Satellamon | IMPLEMENT | IMPLEMENTED | self-audit | 13/13 | [OP][WD] union tuck → return/De-Digivolve protection until opp EoT; [WD][WA][OPT] trash Appmon/TM source → De-Digivolve 1; Link cost 3 + link box +4000; [When Linking] delete level ≤4; Sup. circle |

Test command (all green, 2026-09-13):
`cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex7_013 ex7_059 ex7_066 p_170 bt21_054 bt21_074 bt25_085`

## Engine-Gap Blocked Cards
### BT25-005 Pagumon
- Effect text (inherited): "[Your Turn] [Once Per Turn] When [Three Musketeers] trait cards are placed in this Digimon's digivolution cards, it may digivolve into a Digimon card with [Three Musketeers] in its text or the [TS] trait in the hand with the cost reduced by 2."
- Missing engine API: `OnAddDigivolutionCards` trigger timing (see `docs/RUST_ENGINE_GAPS.md`); the downstream body (`effect_initiated_digivolve` from hand, `cost: { reduce: 2 }`) is expressible once the trigger exists.

## Substrate widened by this slice (rule 28)
- **Formula-valued `draw.count`** (G-DSL-DRAW-FORMULA-COUNT): `draw: { of, count: <formula> }` compiles to the new `CompiledStep::DrawFn` (literal counts keep `Draw`). Driver EX7-013 "draw until you have 6 in your hand" = `max(6 - hand, 0)`.
- **`distinct_names_count` per-selector** (G-DSL-FORMULA-DISTINCT-NAMES-COUNT): `per: { distinct_names_count: { of, zone, filter } }` — synth-identity-aware distinct card names (same normalisation as `distinct_named_count_gte`). Driver EX7-066's per-different-name DP cap.
- **`use_option_bound` Material origin** (G-DSL-USE-OPTION-FROM-SOURCES facet): a `select_union_zone` pick from a carrier's digivolution cards now routes through `OptionSource::Source { host, card }` instead of silently no-op'ing. Driver BT25-085.
- **`link_card_count` predicate leaf** (G-DSL-LINK-CARD-COUNT-FILTERED): the link-card sibling of `source_count` (`{ filter, at_least }` over `Permanent.linked_cards`). Driver BT25-085's "digivolution cards OR link cards" activation gate.

## New Patterns / findings
- `source_count`'s nested filter is evaluated with the card-leaf evaluator (no `any_of`/`all_of` combinators) — express an OR as two `any_permanent { source_count }` legs under a top-level `any_of` (BT21-054 / BT21-074 conditions).
- A trigger whose first body step is an optional selection gets no separate outer gate; add a `condition` mirroring DCGO's `CanActivateCondition` so a no-candidate trigger is not queued (otherwise a 2-clause timing still surfaces a spurious `TriggerOrder`).
- DCGO `RemoveUse` (OPT not spent when nothing executed): `refund_opt` on `binding_absent` after an optional pick (BT25-085 use clause), or a clause-level optional gate (declining it refunds the OPT — G-OPT-REFUND-ON-DECLINE; BT25-085 unsuspend clause).
- `runner.play(0, i)` on an Option seats it as a field permanent — drive Option [Main] tests through `game.play_option_from_hand` (BT25-043 idiom); EX7-070's tests still use `play` and never assert the post-use trash (latent).
- A digivolution-source trash emits no `GameEvent::Trash`; assert the trash zone instead.

---

# Slice `tamers-options` (campaign three-musketeers-1) — 2026-09-15
Pipeline: batch-implement-cards-rust-dsl (single-agent run; scout/implement/review
folded). Authoring agent crashed mid-verification (session limit); the slice was
finished, audited and committed by the resume stage on 2026-09-15.
Slice: BT18-093, BT3-096, P-212, BT3-105, BT6-105, P-108 (the archetype's Tamers
and Options). Sources: card images (`.webp`) + official Bandai DB bundles
(`data/card_bundles/<ID>.md`) for printed text and the official Q&A, DCGO
`<ID>.cs` for resolution order.

## Summary
- IMPLEMENTED: 6
- PARTIAL: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT18-093 | Violet Inboots | IMPLEMENT | IMPLEMENTED | resume-stage audit | 8/8 | [SoYT] memory ≤2 → 3; [SoYMP] optional trash Option/[Ghost]/[TM] from hand (cost, declinable) → Draw 1, gated on an eligible hand card (DCGO CanActivate); [Security] play self free |
| BT3-096 | Mimi Tachikawa | IMPLEMENT | IMPLEMENTED | resume-stage audit | 6/6 | [All Turns] `on_use_option` (NEW timing) optional suspend-self → +1 memory, fires on either player's Option use, never offered while suspended; [Security] play self free. **Engine fix**: OnUseOption now fires AFTER the used Option's body |
| P-212 | Asuna Shiroki | IMPLEMENT | IMPLEMENTED | resume-stage audit | 9/9 | [SoYMP] +1 memory if opp has a Digimon; [On Play] Draw 1 → mandatory hand trash → `trashed_hand_card_matching` (NEW result predicate) TM/TS → delete 1 opp Lv.3; [Security] play self free |
| BT3-105 | Breath of the Gods | IMPLEMENT | IMPLEMENTED | resume-stage audit | 7/7 | [Main] 1 own Digimon gains Reboot + DP-minus immunity + no-return-to-hand/deck until end of opp's next turn; [Security] continuous opp-wide CannotAttackPlayer for the turn |
| BT6-105 | Gewalt Schwärmer | IMPLEMENT | IMPLEMENTED | resume-stage audit | 7/7 | `use_requirement` colour bypass with a [TM] battle-area Digimon; [Main] `delete_all_permanents` (NEW step) both sides play cost ≤7, tokens excluded, one batch; [Security] add self to hand |
| P-108 | Wisdom Training | IMPLEMENT | IMPLEMENTED | resume-stage audit | 9/9 | [Main] reveal 2 → add 1 purple → bottom the rest (ordering choice) → seat as Delay; [Main]<Delay> optional: 1 Digimon digivolves into a purple hand card, cost −2; inherited [Security] place self as Delay |

Test command (all green, 2026-09-15 — 46/46):
`cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt18_093 bt3_096 p_212 bt3_105 bt6_105 p_108`

## Resume-stage audit — what the crashed slice had wrong (9 failing tests → 0)
All nine failures were in this slice; every ex7-tops test was already green.
- **BT3-096 (3 tests) — ENGINE ordering bug, fixed.** `Game::use_option_from` enqueued
  `OnUseOption` (both battle areas) together with the Option's own `OptionMain` and
  drained once, so Mimi's trigger and Death Claw's `[Main]` landed in one
  same-controller bundle → a spurious non-optional `TriggerOrder` prompt (and on the
  opponent's turn the Option's body simply parked first). Evidence for the correct
  order: DCGO `UseOptionClass.UseOption` calls `StackSkillInfos(hashtable,
  OnUseOption)` (→ `PutStackedSkill`, deferred) and then runs the `OptionSkill`
  effects inline via `ActivateEffectProcess`; the official BT3-096 Q&A reads "It can
  be activated after activating the used Option card's [Main] effect." Fix:
  `Game::on_use_option_armed` is set when the use cost is paid;
  `fire_on_use_option_observers` consumes it AFTER the body's drain (synchronous
  path) or from `advance_pending_option` (selection-resume path, including a body
  that CLAIMED `pending_option` such as `place_self_under_permanent`), moving the
  pending Option into the new `OptionResolutionPhase::OnUseOptionDrain` so an
  observer's parked prompt resumes into arts/disposal without re-firing. Test 3 now
  declines Death Claw's own optional pick first, then answers Mimi.
- **BT3-105 (1 test) — test bug.** `.security(0, &[CARD_ID, "FILL"])` lists bottom →
  top (combat reveals via `security.pop()`), so the attack revealed FILL. Reordered.
- **BT6-105 (1 test) — YAML re-authored.** The colour bypass was a `kind: flood_gate`
  granting `IgnoreColorRequirement` (BT22-099 / P-151 / BT23-096 shape). That
  declarative is only ticked for permanents on the field — on a hand-resident Option
  it never reaches `option_use_requirement_or_color_available`, and the sibling
  authorings are asserted structurally only (no live `play_option_from_hand`).
  Re-authored as the top-level `use_requirement:` (EX7-066 / EX7-071 idiom — the
  identical printed clause), which lowers to the [Main] effect's
  `option_color_requirement_bypass` that the use-time check consults. Positive and
  negative live checks pass. (Latent finding for the three sibling cards — logged
  below, not changed in this slice.)
- **P-108 (4 tests) — test bugs.** `.deck()` lists bottom → top (`Player::draw` pops
  the last element) and `deck_bottom_ids` read the Vec tail (= the top); the three
  reveal tests had their decks reversed. The structural check for the Delay body's
  `effect_initiated_digivolve` did not recurse into the `if binding_exists` wrapper.

## Substrate widened by this slice (rule 28)
- **`when: on_use_option` timing** (G-DSL-ON-USE-OPTION-TIMING — RESOLVED): lowers to
  `EffectTiming::OnUseOption`, the board-wide "when a player uses an Option card"
  observer; scope the actor with `active_when: { your_turn | all_turns }`. Driver
  BT3-096; also unblocks BT25-091 Monica Simmons clause 3 (still authored as BLOCKED
  in its YAML — follow-up).
- **Engine: OnUseOption fires after the Option body** (see audit above) —
  `Game::on_use_option_armed`, `fire_on_use_option_observers`,
  `OptionResolutionPhase::OnUseOptionDrain`, `finish_option_after_body`.
- **`delete_all_permanents { over }` step** (G-DSL-DELETE-ALL-PERMANENTS — RESOLVED):
  one up-front scan of both battle areas → `Game::delete_permanents_batch` (rule 25),
  so "Delete all …" leaves the field as ONE simultaneous deletion. Driver BT6-105.
- **`trashed_hand_card_matching { <card predicate> }` result predicate** and the
  bare-bool `effect_trashed_any_hand_card` (G-DSL-EFFECT-TRASHED-HAND-CARD —
  RESOLVED): read the per-effect hand-trash log written by
  `trash_from_hand_by_index` / hand-origin `trash_union_bound`. Driver P-212.

## New Patterns / findings
- DebugRunner list orientation: `.deck(...)` AND `.security(...)` are bottom → top
  (last element = the card drawn / revealed first). Assert the bottom of the deck
  via `deck[..n]`, not the tail.
- Structural clause checks must recurse into `CompiledStep::If { then, else_branch }`
  when the body nests a mandatory step under an optional pick's `if binding_exists`.
- LATENT (not changed here): BT22-099, P-151 (has a `use_requirement` too, so it
  works), BT23-096, BT19-093, BT21-097 author the colour bypass as a hand-Option
  `flood_gate` + `IgnoreColorRequirement`; their tests assert the clause shape only.
  Any card relying on that shape alone (BT22-099, BT23-096) is likely NOT bypassing
  colour at use time — worth a live-use regression pass and a `use_requirement`
  re-author.

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
