# Judge-Quiz Faithfulness Suite — Verdict Ledger

Per-question coverage of the TCG-Judges' Discord rules quiz ("ZXavier's Digi Rulings", 30 Q),
reproduced as Rust behavioral tests under `code/digimon-engine/tests/judge_quiz/`.
Change: `openspec/changes/add-judge-quiz-faithfulness-suite/`. Authoritative card IDs + board states:
[`card-resolution.md`](../../openspec/changes/add-judge-quiz-faithfulness-suite/card-resolution.md).

**Discipline:** discover-then-pin. Each test asserts the official judge-correct answer. A test that
*could* pass for the wrong reason (no-op effect, wrong play path, surface outcome that ignores the
rule) is NOT written that way. An `#[ignore]` always cites a specific blocker; it never hides a
known-wrong result.

Coverage as of 2026-06-05: **30/30 questions have a test entry; 19 PASS.** `cargo test --test judge_quiz`
→ 28 passed (19 question pins + loader/probe/analogs), 11 ignored, 0 failed.

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
| 1 | A | YES (Progress guards Digimon, not battle) | **PASS** | Fixed by `batch-implement-cards-rust-dsl` first wave: BT13-088 Belphemon: Sleep Mode authored; pinned — Medusamon's `<Progress>` is live (would block an affecting opponent effect) yet Belphemon's `[Opp Turn]` end-attack succeeds (ends the battle, doesn't affect the Digimon) | `a::q1_belphemon_opp_turn_ends_attack_through_progress` |
| 2 | A | NO memory loss | **PASS** | Fixed by `add-grant-triggered-effect-dsl`: EX1-068 `[Main]` grant authored + granted-trigger dispatch consults `progress_excludes` (G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT, Q2 slice resolved) | `a::q2_medusamon_progress_blocks_ice_wall_memory_loss` |
| 3 | G | YES | BLOCKED-CARD | EX10-020, BT12-057 | `g::q3_breeding_area_effect_inactive_allows_digivolve` |
| 4 | G | NO another check (+1/−1 net) | BLOCKED-CARD | AD1-002, BT4-098, ST3-15 | `g::q4_security_attack_net_modifiers_one_check` |
| 5 | C | YES (declare if cost can be made payable) | **PASS** | Fixed by `fix-ad1-025-assembly-data`: engine Assembly executor (G-ASSEMBLY-PLAY-EXECUTION) + `[Assembly]` restored to AD1-025 data/YAML. Declare-then-pay mask offers the play at memory 0. | `c::q5_assembly_declaration_legal_when_cost_can_be_made_payable` |
| 6 | B | NO | **PASS** | Pinned 2026-05-30: BT8-109 Flame Hellscythe authored; Pillomon (BT9-033) reduced to ≤0 DP by sub-effect 1 is NOT deleted mid-effect, so its `CannotPlayDigimonByEffect` floodgate persists and sub-effect 2's trash-play is blocked (contrast Q7, where the delete clears it). Pillomon deleted only by the post-resolution rules-check. | `b::q6_pillomon_zero_dp_not_deleted_until_flame_hellscythe_resolves` |
| 7 | B | YES | **PASS** | Fixed by `batch-implement-cards-rust-dsl` first wave: BT9-108 Eye of the Gorgon authored; pinned — sub-effect 1 deletes Pillomon (clearing its `CannotPlayDigimonByEffect` floodgate), sub-effect 2 then plays the Lv3 (control proves the floodgate was real → no false-pass) | `b::q7_eye_of_the_gorgon_sequential_delete_then_play` |
| 8 | B | Agumon trashed → Koromon trashed | BLOCKED-PRIMITIVE | All cards implemented, but `G-BURST-ON-TURN-END-NOT-EXECUTED` (discovered): the Burst `on_burst_turn_end` (trash top at end of burst turn) is compiled but never executed — `BurstDigivolve` is lowered only to a blast-counter marker — so "Agumon trashed → Koromon trashed" can't occur. (Also needs the DP-less-can't-remain rule + a DebugRunner burst driver.) | `b::q8_burst_digivolve_dp_less_digimon_trash_chain_at_eot` |
| 9 | D | After both trashed; NO memory | BLOCKED-CARD | BT23-102, BT15-037 | `d::q9_gatomon_not_in_battle_area_during_removal_no_memory` |
| 10 | F | 0 | **PASS** | RESOLVED 2026-06-04 (`G-ON-ADD-TO-HAND-OBSERVER`). The earlier `G-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT` co-block was a MISCHARACTERIZATION — the judge line uses Akihiro Kurata's **[End of Opp Turn] draw/trash** (clause 2, implemented), NOT his Belphemon cost-reduction (clause 1, still blocked but unused here). MirageGaogamon's observer now fires for real off Kurata's effect-draw and gains floor(15/4)=3 → 0. Mental Training cost + Gravity Crush −2 staged as memory deltas. | `f::q10_multi_effect_memory_arithmetic_ends_at_zero` |
| 11 | F | 4 (Gravity Crush not OPT) | **PASS** | RESOLVED 2026-06-04. Follow-up from Q10: a 2nd Mental Training (−2) + Gravity Crush's non-OPT end-of-turn −2 → 4 on Player A's side. (Turns on Gravity Crush being non-OPT, not the observer.) | `f::q11_non_opt_gravity_crush_refires_memory_four` |
| 12 | F | YES, unsuspends | **PASS** | RESOLVED 2026-06-02 (`G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT`): `kind_matches_field` now coalesces `Token` into `Digimon`, and `eval_permanent_fields` defers permanent-kind to the (token-aware) `synth_identity` check — so a REAL Petrification token is a legal placement pick. Test un-ignored with the real `TOKEN_PETRIFICATION` permanent (not a Digimon stand-in). Regression-clean. | `f::q12_token_placeable_as_digivolution_card_unsuspends` |
| 13 | B | −6000 DP | **PASS** | Pinned 2026-05-30: BT22-042 Nyabootmon's `-3000 × (your Digimon)` is counted (2: Nyabootmon + ShoeShoemon) BEFORE ShoeShoemon (P-165)'s deferred `[On Play]` adds a Familiar token → −6000 (token not counted; a count of 3 would give −9000). | `b::q13_nyabootmon_dp_minus_measured_before_shoeshoemon_on_play` |
| 14 | B | −6000 DP | **PASS** | RESOLVED 2026-06-02 (`G-CONTINUOUS-MASS-DP-DEBUFF`): EX4-074 re-authored `continuous: true` → a source-independent floating mass modifier catches the later-played ShoeShoemon at ≤0 DP, which is still counted → Nyabootmon's debuff is −6000 (count 2). Asserts the per-application debuff value (the ruling) — the net Ruin DP additionally reflects Nyabootmon's faithful OPTIONAL `[On Any Deletion]` recursion. Focused substrate pin: `b::q14_ruin_mode_mass_debuff_is_continuous_catches_later_entrant`. | `b::q14_nyabootmon_dp_minus_vs_shinegreymon_ruin_mode` |
| 15 | E | Gallantmon (X Antibody) topmost | BLOCKED-CARD | BT19-073, BT17-016, BT12-016, EX3-057 | `e::q15_sequential_de_digivolve_halted_by_x_antibody_immunity` |
| 16 | E | NO (`<Partition>` not triggered) | **PASS** | Fixed by `add-grant-triggered-effect-dsl`: EX6-057 Lilithmon authored + granted body runs as the carrier's own effect (D4/DCGO), so the granted self-delete is OwnEffect → `<Partition>` cause-filter skips it | `e::q16_partition_not_triggered_when_leaving_by_own_granted_effect` |
| 17 | A | NO | **PASS** | Fixed by `add-grant-triggered-effect-dsl`: BT16-102 Magnamon X + EX6-057 authored; the granted-trigger dispatch suppresses a granted opponent effect when the carrier is immune to the grantor (`permanent_is_unaffected_by_effect`). BT21-036 not needed (its only role was an Armor-Form source — staged synthetically) | `a::q17_magnamon_x_immunity_removes_granted_eot_delete` |
| 18 | A | NO | BLOCKED-PRIMITIVE | LM-020 attempted (first wave): the `[Start of Opp Turn]` category-immunity (Q18-relevant) is implementable + the Blast-Digivolve immunity substrate is done, but LM-020 is BLOCKED on `G-DSL-RETURN-SELECTED-SECURITY-TO-DECK` (its `[When Digivolving]` clause). | `a::q18_quantumon_self_immunity_blocks_own_blast_digivolve` |
| 19 | D | 0 draws | PASS | `G-ON-DELETION-RESOLVES-MID-EFFECT` RESOLVED 2026-06-05. Part A: top-most-card-in-trash gate on the OnDeletion bundle (`run_queued_effect_inner` + snapshot `is_token`). Part B: `drain_batch_on_any_deletion`'s post-deletion trigger drain gated on `maybe_drain_effect_queue` so the bundle resolves only after CFtD's return-to-hand settles → Eyesmon left trash → all suppressed → 0 draws. (Q20 stays 8.) | `d::q19_on_deletion_suppressed_when_returned_to_hand` |
| 20 | D | 8 draws | **PASS** | Cards authored 2026-06-03; the Eyesmon stack (own Draw3 + inherited Gabumon 2 + DemiMeramon 1 + Pumpkinmon 2) deleted-to-trash fires all [On Deletion] → 8 draws. Engine matches. | `d::q20_all_on_deletion_fire_when_eyesmon_stays_in_trash` |
| 21 | D | 0 draws | BLOCKED-CARD | BT3-109 (Back for Revenge!) BLOCKED on `G-DSL-DELETED-SELF-TRASH-BINDING` (no DSL binding for the just-deleted carrier in trash); would also hit `G-ON-DELETION-RESOLVES-MID-EFFECT`. | `d::q21_remaining_on_deletion_suppressed_when_played_from_trash` |
| 22 | F | YES, 2 tokens | **PASS** | Fixed by `fix-judge-quiz-engine-gaps` (Gap 2): `move_card_to_deck` routes a Digi-Egg returned from trash to the digitama deck (G-RETURN-TRASH-DIGI-EGG-ROUTING, resolved) | `f::q22_digi_egg_returned_to_deck_bottom_routes_to_digitama_deck` |
| 23 | D/F | 1 memory | PASS | Engine already correct (2026-05-30, run to completion): trashing 3 Tumblemon mid-effect enqueues 3 mandatory `OnDigivolutionCardTrashed` observers → multi-trigger `TriggerOrder` selection PARKS them past Medusamon's return-2; on resolution each clause condition is re-evaluated, dropping the 2 returned cards → only the 1 still in trash fires (+1). The earlier `G-ON-TRASH-OBSERVER-SYNCHRONOUS` "+3 over-count" was a mischaracterization (single-source probe + abstract reasoning, never run end-to-end). No engine change needed. | `d::q23_inherited_trash_memory_gated_on_remaining_in_trash` |
| 24 | B | Hudiemon DP 3000 | BLOCKED-PRIMITIVE | BT23-101, BT23-037, BT16-101, ST17-07 implemented; needs EX6-004 (Kokomon), itself BLOCKED on `G-SUSPEND-EFFECT-INITIATED` (suspend event carries no by_effect bit, so Kokomon's "when an EFFECT suspends" is un-gatable). | `b::q24_hudiemon_alliance_partner_deleted_by_rules_check_before_trigger` |
| 25 | E | YES (DigiXros departure ≠ battle) | **PASS** | RESOLVED 2026-06-03 (`G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER`): added `ReplacementCause::DigiXros` + fire the `WhenWouldLeaveBattleArea` replacement window per battle-area material before consuming. BT17-095's `[All Turns]` leave observer now fires on a DigiXros departure. | `e::q25_all_turns_fires_on_digixros_departure_not_battle` |
| 26 | C | Returns to hand | **PASS** | RESOLVED 2026-06-03 (`G-DIGIXROS-REDIRECT-EXTRACTION`): added the leaving/limbo holding slot (`Game::digixros_leaving_limbo`). The leave window parks BT17-095's `<Delay>` without committing the host; WarGreymon is held in limbo (resolvable + extractable); the accepted DNA-evo re-materializes it into Omnimon; finalize finds the DigiXros recipe dropped below `min` and returns Dorbickmon to hand. Supporting fixes: identity re-resolution of the parked replacement's source/subject after the `battle_area` index shift, and excluding the in-flight host from DNA-partner candidates. | `c::q26_dorbickmon_returns_to_hand_when_cost_unpayable_after_dna_evo` |
| 27 | C | Pays 0 memory | **PASS** | RESOLVED 2026-06-03 (`G-DIGIXROS-UNPAYABLE-RETURN-TO-HAND`): the host commits only at `finalize_digixros_play_after_leave_windows`; when a pruned material leaves the recipe unsatisfied the play is abandoned with 0 memory paid. | `c::q27_dorbickmon_pays_zero_memory_when_returned_to_hand` |
| 28 | A | YES, plays AND activates | BLOCKED-PRIMITIVE | BT20-059 Gankoomon X authored (first wave; protection verified to dodge the lock via `can_affect_permanent`); EX5-060 Dragomon BLOCKED on `G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED` + `G-EVENT-PLAYED-LEVEL-FORMULA` | `a::q28_gankoomon_x_protection_beats_dragomon_on_play_lock` |
| 29 | E | 3 legal stacks | BLOCKED-CARD | BT10-093, EX10-039, EX10-044, EX10-059, EX10-056, EX10-031 | `e::q29_legal_digixros_stack_orderings_with_yuu_amano` |
| 30 | C/E | Suspend both w/ cost reduction | BLOCKED-CARD | BT20-037, BT20-036, EX3-063, BT16-077, EX3-008 | `c::q30_partition_interruptive_suspends_both_with_cost_reduction` |

## Verdict tally

| Verdict | Count | Questions |
|---------|-------|-----------|
| BLOCKED-CARD | 7 | 3, 4, 9, 15, 21, 29, 30 |
| BLOCKED-PRIMITIVE | 4 | 8, 18, 24, 28 |
| CANDIDATE | 0 | — |
| PASS | 19 | 1, 2, 5, 6, 7, 10, 11, 12, 13, 14, 16, 17, 19, 20, 22, 23, 25, 26, 27 |

(Counts: 7 + 4 + 0 + 19 = 30 of 30.)

Q19 → **PASS** (2026-06-05): `G-ON-DELETION-RESOLVES-MID-EFFECT` RESOLVED. Two-part
fix — (A) a top-most-card-in-trash gate on the `[On Deletion]` bundle in
`run_queued_effect_inner` (DCGO `CanActivateOnDeletion`; snapshot gains `is_token`),
and (B) gating `drain_batch_on_any_deletion`'s post-deletion trigger drain on
`maybe_drain_effect_queue` so the bundle resolves only after the deleting effect's
later steps (Calling From the Darkness' return-to-hand) settle. The (b) half proved
surgical, not the feared deletion-model restructuring: the premature drain was an
*unconditional* `drain_effect_queue()` in the OnAnyDeletion stage, not the OnDeletion
stage's already-deferred flush. A cause-slot follow-on (deferred handlers read the
cause from the snapshot, not the restored live slot) keeps `deletion_cause()` faithful
(rule 25). Q20 stays 8. Regression: full engine suite green except 4 pre-existing
`cost_hooks` failures (confirmed at pre-Q19 990de2d5). See engine-gaps.md.

Q25 → **PASS** (2026-06-03): the DigiXros leave-trigger gap
(`G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER`) is closed — `ReplacementCause::DigiXros` +
firing the `WhenWouldLeaveBattleArea` window per battle-area material make BT17-095's
`[All Turns]` leave observer fire on a DigiXros departure. Q26/Q27's
recompute/return-to-hand machinery is built and gated on the residual
`G-DIGIXROS-REDIRECT-EXTRACTION` (a leaving-material limbo slot).

Q10/Q11 (F-cluster memory arithmetic) processed 2026-06-03: authored the 3 cards
(P-104 Mental Training IMPLEMENTED; BT13-103 Akihiro Kurata + BT11-033 MirageGaogamon
PARTIAL). Discover-then-pin surfaced two engine gaps that block the arithmetic —
`G-ON-ADD-TO-HAND-OBSERVER` (no OnAddToHand trigger for MirageGaogamon's
memory-per-4-cards observer) and `G-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT`
(Akihiro Kurata's "by deleting X reduce cost by X's cost"). Q10/Q11 → BLOCKED-PRIMITIVE.

Q19/Q20/Q21 (D-cluster [On Deletion] activation-site) processed 2026-06-03: authored
the 4 Eyesmon-stack cards (BT7-069/BT2-069/BT3-006/BT2-076, DSL — purple [On Deletion]
Draw-then-trash). **Q20 → PASS** (stack deletion fires all [On Deletion] = 8 draws).
**Q19 → BLOCKED-PRIMITIVE** `G-ON-DELETION-RESOLVES-MID-EFFECT` (On-Deletion resolves
nested in the delete step, before the return-to-hand). **Q21 stays BLOCKED-CARD** on
BT3-109 (`G-DSL-DELETED-SELF-TRASH-BINDING`). 5th card BT3-109 BLOCKED (no faithful
play-this-card-from-trash binding).

Q25/Q26/Q27 moved BLOCKED-CARD → **BLOCKED-PRIMITIVE** (2026-06-03): EX3-014 Dorbickmon
was authored (DSL — closing 2 DSL substrate gaps: per-source-stack-count-filtered
formula + `trait_contains` substring predicate), unblocking the cards. Staging the
three scenarios through real DigiXros play then discovered the *true* blockers are
two DigiXros engine gaps — `G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER` (material removal
fires no leave trigger) and `G-DIGIXROS-UNPAYABLE-RETURN-TO-HAND` (no
declare-then-pay recompute/return when a material vanishes mid-resolution). The
discover-then-pin pin is the campaign's core value: authoring the card surfaced the
real engine work.

Q12 + Q14 moved BLOCKED-PRIMITIVE → **PASS** (2026-06-02, judge-quiz engine-gaps
change): `G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT` closed (field `kind: digimon` now
matches battle-area tokens) and `G-CONTINUOUS-MASS-DP-DEBUFF` closed (new
source-independent floating continuous mass-modifier substrate + `add_modifier
continuous: true`; EX4-074 re-authored).

Q23 moved BLOCKED-PRIMITIVE → PASS (2026-05-30): the documented
`G-ON-TRASH-OBSERVER-SYNCHRONOUS` "+3 over-count" was a MISCHARACTERIZATION. Running
the real 3-source-trash-then-return-2 scenario to completion shows the engine
already produces the judge-correct +1: ≥2 mandatory `OnDigivolutionCardTrashed`
observers form a multi-trigger bundle → a `TriggerOrder` selection PARKS them past
the trashing effect, and on resolution each observer's clause condition is
re-evaluated, dropping the cards that were returned in the meantime. Pinned by
`d::q23_inherited_trash_memory_gated_on_remaining_in_trash` (synthetic Medusamon
driver over real EX8-051/EX8-005). The prior "fix seam" / deferral analysis was
predicated on the over-count and is retired; no engine change was needed.

Cluster-B pin wave (2026-05-30): after authoring cluster B's cards, Q6 + Q13 moved
BLOCKED-CARD → **PASS** (deferred-deletion floodgate timing; debuff counted before a
deferred On-Play token). Three more turned out BLOCKED-PRIMITIVE — each pin attempt
*discovered* a real engine gap rather than passing:
- Q8 → `G-BURST-ON-TURN-END-NOT-EXECUTED` (the Burst `on_burst_turn_end` top-trash is
  compiled but never executed — `BurstDigivolve` lowers only to a blast-counter marker;
  also blocks BT13-020/BT13-060's EoT self-trash).
- Q14 → `G-CONTINUOUS-MASS-DP-DEBUFF` (EX4-074's mass −5000 is a one-time snapshot, not a
  continuous effect; doesn't catch a later-played Digimon. Faithful pin body written + ignored).
- Q24 → needs EX6-004, itself BLOCKED on `G-SUSPEND-EFFECT-INITIATED` (suspend event has no
  by_effect bit).

Q1 + Q7 moved BLOCKED-CARD → PASS on 2026-05-29 (`batch-implement-cards-rust-dsl`
first wave): BT13-088 (Belphemon: Sleep Mode) and BT9-108 (Eye of the Gorgon)
authored + pinned. Q12 moved BLOCKED-CARD → BLOCKED-PRIMITIVE same wave: BT24-059
authored but `G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT` (token excluded from
`kind: digimon` field-select) blocks the faithful pin. Q18 / Q28 moved
BLOCKED-CARD → BLOCKED-PRIMITIVE: their cards were attempted but LM-020
(`G-DSL-RETURN-SELECTED-SECURITY-TO-DECK`) and EX5-060
(`G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED` + `G-EVENT-PLAYED-LEVEL-FORMULA`)
BLOCKED; BT20-059 (Gankoomon X) IS authored.

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
the granted "[EoT] Delete this" is OwnEffect → `<Partition>` skips it.

Q17 moved BLOCKED-CARD → PASS on 2026-05-29 (same change): BT16-102 Magnamon X
authored; the granted-trigger dispatch also gates on
`permanent_is_unaffected_by_effect`, so a carrier immune to the grantor's
effects suppresses the granted "[EoT] Delete this". BT21-036 was not needed
(Armor-Form source staged synthetically).

## Gaps surfaced (the discovery-wave yield)

0. **G-NO-GENERAL-ZERO-DP-RULES-CHECK** (cluster B root: Q6, Q8, Q13, Q14, Q24) — **RESOLVED
   2026-05-29** (change `fix-judge-quiz-engine-gaps`, Gap 1). The engine now has a general
   state-based ≤0-DP rules-check (`Game::run_state_based_rules_check`) invoked at the outermost
   `drain_effect_queue` boundary — between each top-level queued effect (Q24 interleave) and a final
   fixpoint sweep, never mid-effect (Q6/Q13/Q14); the unfaithful inline mid-effect deletion in
   `add_modifier` was removed. Probe `zero_dp_probe_reduced_digimon_deleted_after_effect_resolves`
   + synthetic `q6_analog_*`/`q24_analog_*` pin it; full suite regression-clean. Cluster-B per-card
   scenarios (Q6/Q8/Q13/Q14/Q24) flip to PASS as their cards are authored. Moved to
   [`resolved-gaps.md`](../resolved-gaps.md).
0b. **G-ON-TRASH-OBSERVER-SYNCHRONOUS** (Q23) — **WITHDRAWN / MISCHARACTERIZED 2026-05-30.** The
   logged "+3 over-count" was wrong for the real Q23 (multi-source) shape. Running the 3-Tumblemon-
   trash-then-return-2 scenario to completion shows the engine ALREADY produces the judge-correct +1:
   ≥2 mandatory `OnDigivolutionCardTrashed` observers form a multi-trigger bundle → the drainer
   installs a `TriggerOrder` selection that PARKS them past the trashing effect (the return-2 runs
   first); on resolution each observer's clause condition is RE-EVALUATED, and the cards returned in
   the meantime fail (no longer in trash) → dropped, leaving only the 1 remaining (+1). The earlier
   probe only ran the SINGLE-source synchronous case + reasoned about deferral abstractly, never end-
   to-end. **Q23 → PASS; no engine change needed; the deferral "fix seam" / split-out follow-up is
   retired.** Residual narrow open question (no known card, not a blocker): a SINGLE source trashed
   then returned WITHIN one effect would still fire synchronously. Gap moved to
   [`resolved-gaps.md`](../resolved-gaps.md) as mischaracterized. Q21 is `[On Deletion]` (different
   mechanic) and stays BLOCKED-CARD on its unauthored cards.
0c. **G-BLAST-DIGIVOLVE-IMMUNITY** (Q18) — **RESOLVED 2026-05-29** (change
   `fix-judge-quiz-cluster-wiring-gaps`). Blast Digivolve / Blast DNA now consult
   `permanent_is_unaffected_by_effect` so a Digimon immune to all Digimon effects incl. its own
   (Quantumon LM-020) can't blast-digivolve. Substrate closed + pinned by
   `counter_interrupt::blast_target_immune_to_own_effects_is_not_a_counter_candidate`; Q18 stays
   BLOCKED-CARD on LM-020 for the end-to-end pin.
0d. **G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER** (Q25) + **G-DIGIVOLVE-TARGET-RESTRICTION** (Q3) — NEW gaps,
   probed + scoped 2026-05-29. Both confirmed via read-only audit (DigiXros material consumption fires
   no leave trigger; no digivolve-target-restriction `ModifierType`). DEFERRED to authoring-time
   (rule 28): the API card text for EX3-014 lacks the `[All Turns]` the judge-quiz mapping implies, and
   EX10-020's restriction is a self-"can only digivolve into [Apocalymon]" — both need DCGO to pin the
   exact shape before building. Fix shapes logged in [`engine-gaps.md`](../archetype-qa/engine-gaps.md).
1. **G-RETURN-TRASH-DIGI-EGG-ROUTING** (Q22) — RESOLVED 2026-05-29 (change
   `fix-judge-quiz-engine-gaps`, Gap 2). Was: `return_trash_cards_to_deck_bottom` inserted Digi-Eggs
   into the main deck. Fixed via `EffectContext::move_card_to_deck` routing all four trash→deck movers
   (`CardKind::DigiEgg` → digitama deck). Q22 → PASS. Gap moved to
   [`resolved-gaps.md`](../resolved-gaps.md). (Q23's remain-in-trash gating turned out to be already
   handled by the engine's multi-trigger TriggerOrder parking + condition re-evaluation —
   G-ON-TRASH-OBSERVER-SYNCHRONOUS was withdrawn as mischaracterized; Q23 → PASS.)
2. **AD1-025 `[Assembly]` gap** (Q5) — RESOLVED 2026-05-29 (change `fix-ad1-025-assembly-data`). This
   was a TWO-layer gap: (a) `data/cards.json` missing the `[Assembly]` keyword the real card carries
   (DCGO AD1_025.cs:214-255), AND (b) no engine Assembly executor at all (the alt-path KIND compiled
   but was matched in no play path — G-ASSEMBLY-PLAY-EXECUTION). Fixed by implementing the executor
   (eligibility-from-trash, surfaced per-element selection, bottom placement, reduced cost,
   declare-then-pay mask), restoring `[Assembly]` to `card_overrides.json`, and authoring the
   `assembly` alt_path in `cards/ad1/AD1-025.yaml`. Q5 → PASS. Gap moved to
   [`resolved-gaps.md`](../resolved-gaps.md).
3. **G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT** (Q2, Q16, Q17) — FULLY RESOLVED 2026-05-29 (change
   `add-grant-triggered-effect-dsl`). The `grant_triggered_effect` step + `GrantedTrigger` slot
   already existed (EX10-034). All three cause-attribution directions landed: **Q2** — opponent-set
   targeting + dispatch consults `progress_excludes` so a `<Progress>` carrier doesn't fire the grant
   (EX1-068). **Q16** — the granted body runs as the carrier's OWN effect (D4/DCGO), so a granted
   self-delete is OwnEffect → `<Partition>` skips it (EX6-057 Lilithmon). **Q17** — the dispatch also
   consults `permanent_is_unaffected_by_effect`, so a carrier immune to the grantor's effects
   (Magnamon X's "isn't affected by your opponent's effects") suppresses the granted clause
   (BT16-102). Q2/Q16/Q17 → PASS; gap moved to [`resolved-gaps.md`](../resolved-gaps.md). BT21-036
   Magnamon was NOT needed (its only role was an Armor-Form digivolution source, staged synthetically).
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
data / engine bug). A second lesson (Q23, 2026-05-30): a logged "gap" can be a mischaracterization —
the "+3 over-count" survived a probe + abstract reasoning but evaporated once the scenario was run to
completion (resolving the mandatory TriggerOrder selection). RUN-TO-COMPLETION before logging a gap;
a probe that stops at the first `pending_selection` can miss that the engine already does the right
thing. Gaps live at the engine-primitive, card-YAML-clause, and source-data layers, and only an
AUDIT-before-asserting pass — run end-to-end — distinguishes them, which is exactly what this suite
institutionalizes.
