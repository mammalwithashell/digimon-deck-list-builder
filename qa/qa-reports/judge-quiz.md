# Judge-Quiz Faithfulness Suite — Verdict Ledger

Per-question coverage of the TCG-Judges' Discord rules quiz ("ZXavier's Digi Rulings", 30 Q),
reproduced as Rust behavioral tests under `code/digimon-engine/tests/judge_quiz/`.
Change: `openspec/changes/add-judge-quiz-faithfulness-suite/`. Authoritative card IDs + board states:
[`card-resolution.md`](../../openspec/changes/add-judge-quiz-faithfulness-suite/card-resolution.md).

**Discipline:** discover-then-pin. Each test asserts the official judge-correct answer. A test that
*could* pass for the wrong reason (no-op effect, wrong play path, surface outcome that ignores the
rule) is NOT written that way. An `#[ignore]` always cites a specific blocker; it never hides a
known-wrong result.

Coverage as of 2026-05-29: **30/30 questions have a test entry.** `cargo test --test judge_quiz`
→ 1 passed (loader), 30 ignored, 0 failed.

## Verdict legend

- **PASS** — faithful test written and green (judge answer reproduced).
- **BUG** — faithful test written and *confirmed failing* against current engine; gap logged; test `#[ignore]`-d citing the gap pending the fix.
- **CANDIDATE** — all referenced cards implemented; real test pending (complexity / dependency noted).
- **BLOCKED-CARD** — needs ≥1 unimplemented card authored (the bulk; routes to cluster authoring §3–§9).
- **BLOCKED-DATA** — referenced card's data is incomplete in `data/cards.json`.
- **BLOCKED-PRIMITIVE** — needs a missing engine/DSL primitive.

## Per-question verdicts

| Q | Cluster | Judge answer | Verdict | Blocker / gap | Test fn |
|---|---------|--------------|---------|---------------|---------|
| 1 | A | YES (Progress guards Digimon, not battle) | BLOCKED-CARD | BT13-088 | `a::q1_belphemon_opp_turn_ends_attack_through_progress` |
| 2 | A | NO memory loss | **PASS** | Fixed by `add-grant-triggered-effect-dsl`: EX1-068 `[Main]` grant authored + granted-trigger dispatch consults `progress_excludes` (G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT, Q2 slice resolved) | `a::q2_medusamon_progress_blocks_ice_wall_memory_loss` |
| 3 | G | YES | BLOCKED-CARD | EX10-020, BT12-057 | `g::q3_breeding_area_effect_inactive_allows_digivolve` |
| 4 | G | NO another check (+1/−1 net) | BLOCKED-CARD | AD1-002, BT4-098, ST3-15 | `g::q4_security_attack_net_modifiers_one_check` |
| 5 | C | YES (declare if cost can be made payable) | **PASS** | Fixed by `fix-ad1-025-assembly-data`: engine Assembly executor (G-ASSEMBLY-PLAY-EXECUTION) + `[Assembly]` restored to AD1-025 data/YAML. Declare-then-pay mask offers the play at memory 0. | `c::q5_assembly_declaration_legal_when_cost_can_be_made_payable` |
| 6 | B | NO | BLOCKED-CARD | BT8-109 | `b::q6_pillomon_zero_dp_not_deleted_until_flame_hellscythe_resolves` |
| 7 | B | YES | BLOCKED-CARD | BT9-108 | `b::q7_eye_of_the_gorgon_sequential_delete_then_play` |
| 8 | B | Agumon trashed → Koromon trashed | BLOCKED-CARD | BT13-020, AD1-016, BT21-044, BT21-042, EX4-005, BT21-004 | `b::q8_burst_digivolve_dp_less_digimon_trash_chain_at_eot` |
| 9 | D | After both trashed; NO memory | BLOCKED-CARD | BT23-102, BT15-037 | `d::q9_gatomon_not_in_battle_area_during_removal_no_memory` |
| 10 | F | 0 | BLOCKED-CARD | BT13-103, BT11-033, P-104 | `f::q10_multi_effect_memory_arithmetic_ends_at_zero` |
| 11 | F | 4 (Gravity Crush not OPT) | BLOCKED-CARD | BT13-103, BT11-033, P-104 | `f::q11_non_opt_gravity_crush_refires_memory_four` |
| 12 | F | YES, unsuspends | BLOCKED-CARD | BT24-059 | `f::q12_token_placeable_as_digivolution_card_unsuspends` |
| 13 | B | −6000 DP | BLOCKED-CARD | BT16-101, ST17-07 | `b::q13_nyabootmon_dp_minus_measured_before_shoeshoemon_on_play` |
| 14 | B | −6000 DP | BLOCKED-CARD | BT16-101 (1 away) | `b::q14_nyabootmon_dp_minus_vs_shinegreymon_ruin_mode` |
| 15 | E | Gallantmon (X Antibody) topmost | BLOCKED-CARD | BT19-073, BT17-016, BT12-016, EX3-057 | `e::q15_sequential_de_digivolve_halted_by_x_antibody_immunity` |
| 16 | E | NO (`<Partition>` not triggered) | **PASS** | Fixed by `add-grant-triggered-effect-dsl`: EX6-057 Lilithmon authored + granted body runs as the carrier's own effect (D4/DCGO), so the granted self-delete is OwnEffect → `<Partition>` cause-filter skips it | `e::q16_partition_not_triggered_when_leaving_by_own_granted_effect` |
| 17 | A | NO | BLOCKED-CARD | BT16-102, BT21-036, EX6-057 | `a::q17_magnamon_x_immunity_removes_granted_eot_delete` |
| 18 | A | NO | BLOCKED-CARD | LM-020 (1 away) | `a::q18_quantumon_self_immunity_blocks_own_blast_digivolve` |
| 19 | D | 0 draws | BLOCKED-CARD | BT7-069, BT2-069, BT3-006 | `d::q19_on_deletion_suppressed_when_returned_to_hand` |
| 20 | D | 8 draws | BLOCKED-CARD | + BT2-076 | `d::q20_all_on_deletion_fire_when_eyesmon_stays_in_trash` |
| 21 | D | 0 draws | BLOCKED-CARD | + BT3-109 | `d::q21_remaining_on_deletion_suppressed_when_played_from_trash` |
| 22 | F | YES, 2 tokens | **PASS** | Fixed by `fix-judge-quiz-engine-gaps` (Gap 2): `move_card_to_deck` routes a Digi-Egg returned from trash to the digitama deck (G-RETURN-TRASH-DIGI-EGG-ROUTING, resolved) | `f::q22_digi_egg_returned_to_deck_bottom_routes_to_digitama_deck` |
| 23 | D/F | 1 memory | **CANDIDATE** | all cards impl; needs full chain + remain-in-trash gating check; downstream of Q22 | `d::q23_inherited_trash_memory_gated_on_remaining_in_trash` |
| 24 | B | Hudiemon DP 3000 | BLOCKED-CARD | BT23-101, BT23-037, EX6-004, BT16-101, ST17-07 | `b::q24_hudiemon_alliance_partner_deleted_by_rules_check_before_trigger` |
| 25 | E | YES (DigiXros departure ≠ battle) | BLOCKED-CARD | EX3-014 (1 away) | `e::q25_all_turns_fires_on_digixros_departure_not_battle` |
| 26 | C | Returns to hand | BLOCKED-CARD | EX3-014 (1 away) | `c::q26_dorbickmon_returns_to_hand_when_cost_unpayable_after_dna_evo` |
| 27 | C | Pays 0 memory | BLOCKED-CARD | EX3-014 (1 away) | `c::q27_dorbickmon_pays_zero_memory_when_returned_to_hand` |
| 28 | A | YES, plays AND activates | BLOCKED-CARD | BT20-059, EX5-060 | `a::q28_gankoomon_x_protection_beats_dragomon_on_play_lock` |
| 29 | E | 3 legal stacks | BLOCKED-CARD | BT10-093, EX10-039, EX10-044, EX10-059, EX10-056, EX10-031 | `e::q29_legal_digixros_stack_orderings_with_yuu_amano` |
| 30 | C/E | Suspend both w/ cost reduction | BLOCKED-CARD | BT20-037, BT20-036, EX3-063, BT16-077, EX3-008 | `c::q30_partition_interruptive_suspends_both_with_cost_reduction` |

## Verdict tally

| Verdict | Count | Questions |
|---------|-------|-----------|
| BLOCKED-CARD | 25 | 1, 3, 4, 6–15, 17–21, 24–30 |
| CANDIDATE | 1 | 23 |
| PASS | 4 | 2, 5, 16, 22 |

Q5 moved BLOCKED-DATA → PASS on 2026-05-29 (change `fix-ad1-025-assembly-data`):
the engine Assembly executor was implemented (G-ASSEMBLY-PLAY-EXECUTION,
resolved) and `[Assembly]` restored to AD1-025 in data + YAML.

Q22 moved BUG (proven) → PASS on 2026-05-29 (change `fix-judge-quiz-engine-gaps`,
Gap 2): Digi-Egg routing on return-to-deck fixed (G-RETURN-TRASH-DIGI-EGG-ROUTING,
resolved).

Q2 moved BLOCKED-PRIMITIVE → PASS on 2026-05-29 (change `add-grant-triggered-effect-dsl`):
EX1-068's `[Main]` grant authored + the granted-trigger dispatch now consults
`progress_excludes` so a `<Progress>` opponent Digimon doesn't fire the grant.

Q16 moved BLOCKED-CARD → PASS on 2026-05-29 (same change): EX6-057 Lilithmon
authored + the granted body now runs as the carrier's OWN effect (D4/DCGO), so
the granted "[EoT] Delete this" is OwnEffect → `<Partition>` skips it. Q17 still
needs BT16-102 + BT21-036 + an immunity-removes-granted-slot mechanic.

## Gaps surfaced (the discovery-wave yield)

0. **G-NO-GENERAL-ZERO-DP-RULES-CHECK** (cluster B root: Q6, Q8, Q13, Q14, Q24) — NEW engine gap,
   test-proven. The engine has no general state-based ≤0-DP rules-check; the only ≤0-DP deletion is
   Arts-digivolve-specific (`run_rule_check_after_arts`, game_actions.rs:1607). A Digimon reduced to
   ≤0 DP by a non-Arts effect is never deleted. Probe
   `zero_dp_probe_reduced_digimon_deleted_after_effect_resolves` confirmed failing (battle_area 1,
   expected 0). Logged in [`engine-gaps.md`](../archetype-qa/engine-gaps.md). **Systemic** — bigger
   than the quiz; affects any DP-reduction-to-0 effect. **Highest-impact fix.**
0b. **G-ON-TRASH-OBSERVER-SYNCHRONOUS** (Q23 confirmed; Q21 probable) — NEW gap/tension,
   code-confirmed + probed. `on_digivolution_card_trashed` fires synchronously at trash-time
   (`fire_digivolution_card_trashed` enqueues + immediately drains, intentional for EX10-036), so
   inherited on-trash effects can't DEFER and re-check remain-in-trash → Q23 over-counts (+3 vs the
   judge's +1). Design tension (EX10-036 needs synchrony; Q21/Q23 need deferral) — not a trivial fix.
   Probe `cluster_d_on_trash_observer_fires_synchronously_not_deferred` characterizes it. Logged in
   [`engine-gaps.md`](../archetype-qa/engine-gaps.md).
1. **G-RETURN-TRASH-DIGI-EGG-ROUTING** (Q22) — RESOLVED 2026-05-29 (change
   `fix-judge-quiz-engine-gaps`, Gap 2). Was: `return_trash_cards_to_deck_bottom` inserted Digi-Eggs
   into the main deck. Fixed via `EffectContext::move_card_to_deck` routing all four trash→deck movers
   (`CardKind::DigiEgg` → digitama deck). Q22 → PASS. Gap moved to
   [`resolved-gaps.md`](../resolved-gaps.md). (Q23's remain-in-trash gating is a separate gap —
   G-ON-TRASH-OBSERVER-SYNCHRONOUS — still open.)
2. **AD1-025 `[Assembly]` gap** (Q5) — RESOLVED 2026-05-29 (change `fix-ad1-025-assembly-data`). This
   was a TWO-layer gap: (a) `data/cards.json` missing the `[Assembly]` keyword the real card carries
   (DCGO AD1_025.cs:214-255), AND (b) no engine Assembly executor at all (the alt-path KIND compiled
   but was matched in no play path — G-ASSEMBLY-PLAY-EXECUTION). Fixed by implementing the executor
   (eligibility-from-trash, surfaced per-element selection, bottom placement, reduced cost,
   declare-then-pay mask), restoring `[Assembly]` to `card_overrides.json`, and authoring the
   `assembly` alt_path in `cards/ad1/AD1-025.yaml`. Q5 → PASS. Gap moved to
   [`resolved-gaps.md`](../resolved-gaps.md).
3. **G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT** (Q2, Q16) — RESOLVED 2026-05-29 (change
   `add-grant-triggered-effect-dsl`). The `grant_triggered_effect` step + `GrantedTrigger` slot
   already existed (EX10-034). Two attribution directions landed: **Q2** — opponent-set targeting +
   the dispatch consulting `progress_excludes` so a `<Progress>` carrier doesn't fire the grant
   (EX1-068 `[Main]` authored). **Q16** — the granted body now runs as the carrier's OWN effect
   (D4/DCGO: granted ActivateClass sourced from the carrier), so a granted self-delete is OwnEffect →
   `<Partition>` skips it (EX6-057 Lilithmon authored). Q2 + Q16 → PASS; gap moved to
   [`resolved-gaps.md`](../resolved-gaps.md). **Q17 still open** — needs BT16-102 Magnamon X +
   BT21-036 Magnamon authored AND an "immunity removes a granted slot" mechanic (card-wave work).
4. **52 cards to author** — the BLOCKED-CARD bulk. Per-cluster authoring load is tabulated in
   `card-resolution.md`; cluster A is the smallest (7 cards) and three scenarios (Q14, Q16, Q18, Q25,
   Q26, Q27) are a *single* card away.

## Engine machinery confirmed PRESENT (rule-level probes — not gaps)

Probing the engine rule each cluster needs (independent of card authoring) also *cleared* areas:

- **`<Partition>` cause-filter** (Q16/Q25/Q30 rule) — skips `Battle | OwnEffect`
  (keyword_effects.rs:839). Present. (Q16's granted-self-delete attribution still needs the Q2 grant
  primitive once authorable.)
- **Immunity controller-filter** (Q17/Q18/Q28 rule) — `permanent_is_unaffected_by_effect`
  (game.rs:3468) supports `Any | OpponentOnly | OwnOnly` + `source_kind`; Q18's "immune to ALL incl
  own" = `Any`. Probe `cluster_a_self_immunity_blocks_own_controller_effect` passes. (Q18 still needs
  Quantumon LM-020 + the `<Blast Digivolve>` path to consult `can_affect_permanent`.)
- **Security-attack net count** (Q4 rule) — `current_security_strike` (combat.rs:2489) computes
  `raw = base(1) + sum(SecurityAttackChange) + …`; the sum includes negatives, so `+1` and `−1` net
  to base 1 check. Present by construction (fills the gap the deferred Test 3 of
  `mid_attack_security_attack_recompute.rs` left). Q4 still BLOCKED-CARD on AD1-002/BT4-098/ST3-15.
- **Non-OPT delayed-effect stacking** (Q11 rule) — `schedule_delayed_with_runtime` (effect_context/mod.rs:969)
  does an unconditional `scheduled_effects.push` with no dedup/OPT gate, so two Gravity Crush (BT1-090)
  plays schedule two end-of-turn `−2` losses (`−4` total). Present by construction. Q10/Q11 still
  BLOCKED-CARD on Mental Training (P-104)/MirageGaogamon (BT11-033)/Akihiro Kurata (BT13-103).
- **Cluster G breeding-area inactivity** (Q3) — the only rule NOT cleanly probeable at the engine
  level: it turns on a Puppetmon-specific `[All Turns]` digivolve-target restriction not applying in
  the breeding area. The engine has an `in_breeding` predicate (predicate.rs:432); verify when
  EX10-020 is authored.

## Key lesson

"DSL YAML present" is a weak proxy for "faithful." Of the 4 scenarios that looked discovery-ready
(all cards had YAML), Q2/Q5/Q22 were each blocked at a *different* layer (engine primitive / source
data / engine bug) and Q23 needs auditing. Gaps live at the engine-primitive, card-YAML-clause, and
source-data layers, and only an AUDIT-before-asserting pass distinguishes them — which is exactly
what this suite institutionalizes.
