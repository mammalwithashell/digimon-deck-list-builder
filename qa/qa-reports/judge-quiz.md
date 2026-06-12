# Judge-Quiz Faithfulness Suite — Verdict Ledger

Per-question coverage of the TCG-Judges' Discord rules quiz ("ZXavier's Digi Rulings", 30 Q),
reproduced as Rust behavioral tests under `code/digimon-engine/tests/judge_quiz/`.
Change: `openspec/changes/add-judge-quiz-faithfulness-suite/`. Authoritative card IDs + board states:
[`card-resolution.md`](../../openspec/changes/add-judge-quiz-faithfulness-suite/card-resolution.md).

**Discipline:** discover-then-pin. Each test asserts the official judge-correct answer. A test that
*could* pass for the wrong reason (no-op effect, wrong play path, surface outcome that ignores the
rule) is NOT written that way. An `#[ignore]` always cites a specific blocker; it never hides a
known-wrong result.

Coverage as of 2026-06-11: **30/30 questions have a test entry; 29 PASS.** `cargo test --test judge_quiz`
→ 42 passed (29 question pins + loader/probe/analogs + controls + the Q29 single-card variant), 1 ignored (Q8), 0 failed.

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
| 3 | G | YES | **PASS** | RESOLVED 2026-06-10: EX10-020 Puppetmon (PARTIAL — `[Hand][Main]` self-play `G-DSL-HAND-MAIN-SELF-PLAY-REDUCED` + `[Security]` was-face-up gate `G-DSL-SECURITY-WAS-FACE-UP-GATE`, both incidental to Q3) + BT12-057 Quartzmon (IMPLEMENTED) authored. The `[All Turns]` restriction is the new `modifier_name` aura install of `CanOnlyDigivolveInto` (G-DIGIVOLVE-TARGET-RESTRICTION DSL slice landed) — a battle-area-sourced declarative aura, so it is INACTIVE in the breeding area for free, exactly the judge rule. Battle-area control (`ex10_020_battle_area_restriction_blocks_non_apocalymon_digivolve`) proves no false-pass. Incidental engine fix: the turn-start bulk unsuspend ignored `CannotUnsuspend` (`Player::unsuspend_all`) — Quartzmon's "[All Turns] don't unsuspend" TDD caught it; now honored at the phase site. | `g::q3_breeding_area_effect_inactive_allows_digivolve` |
| 4 | G | NO another check (+1/−1 net) | **PASS** | AD1-002 Aldamon + BT4-098 Atomic Inferno authored 2026-06-05 (ST3-15 Holy Flame already impl). The live-card form of the *reduction* case `mid_attack_security_attack_recompute.rs` Test 3 deferred: Aldamon attacks with Atomic Inferno's `<Security A. +1>` (would check 2); Holy Flame on top of P1 security applies `<Security A. −1>` on its check; the engine re-reads net strike (=1) and STOPS after 1 check. Control `q4_control_atomic_inferno_plus_one_alone_checks_two` proves the +1 genuinely extends the loop (no false-pass). | `g::q4_security_attack_net_modifiers_one_check` |
| 5 | C | YES (declare if cost can be made payable) | **PASS** | Fixed by `fix-ad1-025-assembly-data`: engine Assembly executor (G-ASSEMBLY-PLAY-EXECUTION) + `[Assembly]` restored to AD1-025 data/YAML. Declare-then-pay mask offers the play at memory 0. | `c::q5_assembly_declaration_legal_when_cost_can_be_made_payable` |
| 6 | B | NO | **PASS** | Pinned 2026-05-30: BT8-109 Flame Hellscythe authored; Pillomon (BT9-033) reduced to ≤0 DP by sub-effect 1 is NOT deleted mid-effect, so its `CannotPlayDigimonByEffect` floodgate persists and sub-effect 2's trash-play is blocked (contrast Q7, where the delete clears it). Pillomon deleted only by the post-resolution rules-check. | `b::q6_pillomon_zero_dp_not_deleted_until_flame_hellscythe_resolves` |
| 7 | B | YES | **PASS** | Fixed by `batch-implement-cards-rust-dsl` first wave: BT9-108 Eye of the Gorgon authored; pinned — sub-effect 1 deletes Pillomon (clearing its `CannotPlayDigimonByEffect` floodgate), sub-effect 2 then plays the Lv3 (control proves the floodgate was real → no false-pass) | `b::q7_eye_of_the_gorgon_sequential_delete_then_play` |
| 8 | B | Agumon trashed → Koromon trashed | BLOCKED-PRIMITIVE | All cards implemented, but `G-BURST-ON-TURN-END-NOT-EXECUTED` (discovered): the Burst `on_burst_turn_end` (trash top at end of burst turn) is compiled but never executed — `BurstDigivolve` is lowered only to a blast-counter marker — so "Agumon trashed → Koromon trashed" can't occur. (Also needs the DP-less-can't-remain rule + a DebugRunner burst driver.) | `b::q8_burst_digivolve_dp_less_digimon_trash_chain_at_eot` |
| 9 | D | After both trashed; NO memory | **PASS** | BT23-102 Mastemon + BT15-037 Gatomon authored 2026-06-06 (both PARTIAL on incidental clauses). Activation-site ruling: Gatomon's `[All Turns]` "gain 1 memory when your security is removed" is battle-area-gated (DCGO `IsExistOnBattleArea`); when Mastemon's `[When Digivolving]` trim trashes Gatomon FROM the opponent's security, Gatomon is not a battle-area trigger source → 0 memory. Engine enforces this by construction (only battle-area permanents' triggers dispatch). Control: same Gatomon on the field DOES gain +1 (`bt15_037_on_field_gains_memory_when_own_security_removed`). | `d::q9_gatomon_not_in_battle_area_during_removal_no_memory` |
| 10 | F | 0 | **PASS** | RESOLVED 2026-06-04 (`G-ON-ADD-TO-HAND-OBSERVER`). The earlier `G-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT` co-block was a MISCHARACTERIZATION — the judge line uses Akihiro Kurata's **[End of Opp Turn] draw/trash** (clause 2, implemented), NOT his Belphemon cost-reduction (clause 1, still blocked but unused here). MirageGaogamon's observer now fires for real off Kurata's effect-draw and gains floor(15/4)=3 → 0. Mental Training cost + Gravity Crush −2 staged as memory deltas. | `f::q10_multi_effect_memory_arithmetic_ends_at_zero` |
| 11 | F | 4 (Gravity Crush not OPT) | **PASS** | RESOLVED 2026-06-04. Follow-up from Q10: a 2nd Mental Training (−2) + Gravity Crush's non-OPT end-of-turn −2 → 4 on Player A's side. (Turns on Gravity Crush being non-OPT, not the observer.) | `f::q11_non_opt_gravity_crush_refires_memory_four` |
| 12 | F | YES, unsuspends | **PASS** | RESOLVED 2026-06-02 (`G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT`): `kind_matches_field` now coalesces `Token` into `Digimon`, and `eval_permanent_fields` defers permanent-kind to the (token-aware) `synth_identity` check — so a REAL Petrification token is a legal placement pick. Test un-ignored with the real `TOKEN_PETRIFICATION` permanent (not a Digimon stand-in). Regression-clean. | `f::q12_token_placeable_as_digivolution_card_unsuspends` |
| 13 | B | −6000 DP | **PASS** | Pinned 2026-05-30: BT22-042 Nyabootmon's `-3000 × (your Digimon)` is counted (2: Nyabootmon + ShoeShoemon) BEFORE ShoeShoemon (P-165)'s deferred `[On Play]` adds a Familiar token → −6000 (token not counted; a count of 3 would give −9000). | `b::q13_nyabootmon_dp_minus_measured_before_shoeshoemon_on_play` |
| 14 | B | −6000 DP | **PASS** | RESOLVED 2026-06-02 (`G-CONTINUOUS-MASS-DP-DEBUFF`): EX4-074 re-authored `continuous: true` → a source-independent floating mass modifier catches the later-played ShoeShoemon at ≤0 DP, which is still counted → Nyabootmon's debuff is −6000 (count 2). Asserts the per-application debuff value (the ruling) — the net Ruin DP additionally reflects Nyabootmon's faithful OPTIONAL `[On Any Deletion]` recursion. Focused substrate pin: `b::q14_ruin_mode_mass_debuff_is_continuous_catches_later_entrant`. | `b::q14_nyabootmon_dp_minus_vs_shinegreymon_ruin_mode` |
| 15 | E | Gallantmon (X Antibody) topmost | **PASS** | RESOLVED 2026-06-11: BT19-073 LordKnightmon (X Antibody), BT17-016 Gallantmon, BT12-016 WarGrowlmon, EX3-057 Growlmon authored (all IMPLEMENTED); EX8-073 re-authored so its `[All Turns]` 0-or-less-memory immunity is a REAL `effect_immunity` aura (new DSL aura slot landed with this slice). BT19-073's `[When Digivolving]` runs N SEPARATE `<De-Digivolve 1>` instances against the picked stack with the immunity re-checked per trashed card (DCGO `IDegeneration` loop) — after the first trash exposes Gallantmon (X Antibody), its live immunity halts the remaining instances. Pin asserts topmost = EX8-073, exactly 1 card trashed, and the full remaining stack order. | `e::q15_sequential_de_digivolve_halted_by_x_antibody_immunity` |
| 16 | E | NO (`<Partition>` not triggered) | **PASS** | Fixed by `add-grant-triggered-effect-dsl`: EX6-057 Lilithmon authored + granted body runs as the carrier's own effect (D4/DCGO), so the granted self-delete is OwnEffect → `<Partition>` cause-filter skips it | `e::q16_partition_not_triggered_when_leaving_by_own_granted_effect` |
| 17 | A | NO | **PASS** | Fixed by `add-grant-triggered-effect-dsl`: BT16-102 Magnamon X + EX6-057 authored; the granted-trigger dispatch suppresses a granted opponent effect when the carrier is immune to the grantor (`permanent_is_unaffected_by_effect`). BT21-036 not needed (its only role was an Armor-Form source — staged synthetically) | `a::q17_magnamon_x_immunity_removes_granted_eot_delete` |
| 18 | A | NO | PASS | LM-020 Quantumon fully authored 2026-06-05. `[Start of Opp Turn]` declares a category (`select_effect_choice`) + reveals opp deck-top + grants category immunity (`grant_effect_immunity` source_controller `any` → covers OWN effects) gated on the new `binding_card_kind` predicate. Self-inclusive Digimon immunity blocks Quantumon's own `<Blast Digivolve>` (combat gates blast candidacy/execution on `permanent_is_unaffected_by_effect(.., own, Digimon)`). `[When Digivolving]` clause uses the new `return_selected_security_to_deck` verb (`G-DSL-RETURN-SELECTED-SECURITY-TO-DECK` CLOSED). | `a::q18_quantumon_self_immunity_blocks_own_blast_digivolve` |
| 19 | D | 0 draws | PASS | `G-ON-DELETION-RESOLVES-MID-EFFECT` RESOLVED 2026-06-05. Part A: top-most-card-in-trash gate on the OnDeletion bundle (`run_queued_effect_inner` + snapshot `is_token`). Part B: `drain_batch_on_any_deletion`'s post-deletion trigger drain gated on `maybe_drain_effect_queue` so the bundle resolves only after CFtD's return-to-hand settles → Eyesmon left trash → all suppressed → 0 draws. (Q20 stays 8.) | `d::q19_on_deletion_suppressed_when_returned_to_hand` |
| 20 | D | 8 draws | **PASS** | Cards authored 2026-06-03; the Eyesmon stack (own Draw3 + inherited Gabumon 2 + DemiMeramon 1 + Pumpkinmon 2) deleted-to-trash fires all [On Deletion] → 8 draws. Engine matches. | `d::q20_all_on_deletion_fire_when_eyesmon_stays_in_trash` |
| 21 | D | 0 draws | PASS | `G-DSL-DELETED-SELF-TRASH-BINDING` CLOSED 2026-06-05 + BT3-109 authored. `event_card` already resolved the deleted-self top card in trash; the only fix was making `play_from_trash_free` accept a `Card`-handle binding. Back for Revenge! grants Eyesmon `[On Deletion]` self-replay-from-trash; replaying it leaves the trash, so the remaining draw bundle is suppressed by the Q19 top-card-in-trash gate ⇒ 0 draws. (Q19's co-blocker on this row resolved 2026-06-05.) | `d::q21_remaining_on_deletion_suppressed_when_played_from_trash` |
| 22 | F | YES, 2 tokens | **PASS** | Fixed by `fix-judge-quiz-engine-gaps` (Gap 2): `move_card_to_deck` routes a Digi-Egg returned from trash to the digitama deck (G-RETURN-TRASH-DIGI-EGG-ROUTING, resolved) | `f::q22_digi_egg_returned_to_deck_bottom_routes_to_digitama_deck` |
| 23 | D/F | 1 memory | PASS | Engine already correct (2026-05-30, run to completion): trashing 3 Tumblemon mid-effect enqueues 3 mandatory `OnDigivolutionCardTrashed` observers → multi-trigger `TriggerOrder` selection PARKS them past Medusamon's return-2; on resolution each clause condition is re-evaluated, dropping the 2 returned cards → only the 1 still in trash fires (+1). The earlier `G-ON-TRASH-OBSERVER-SYNCHRONOUS` "+3 over-count" was a mischaracterization (single-source probe + abstract reasoning, never run end-to-end). No engine change needed. | `d::q23_inherited_trash_memory_gated_on_remaining_in_trash` |
| 24 | B | Hudiemon DP 3000 | **PASS** | RESOLVED 2026-06-10: `G-SUSPEND-EFFECT-INITIATED` closed (suspend/unsuspend events carry `effect_initiated`; `EffectContext::suspend` tags true) + EX6-004 Kokomon authored. The pin surfaced FOUR more engine fixes: (1) `<Alliance>` keyword was modeled on the ALLY — moved to the ATTACKER (DCGO `AllianceSelfEffect`); (2) Alliance resolution now suspends through the canonical chokepoint inside a deferred-drain scope and reads the ally's DP AFTER suspension (DCGO `AllianceProcess` order); (3) `effective_dp` floors at 0 (rules 17-1-3-1 / DCGO `Permanent.DP`) so the debuffed ally contributes +0; (4) the outermost drain runs the state-based rules check BEFORE activating parked triggers (official: rule processing precedes trigger activation), so Tentomon dies before Kokomon's inherited +2000 can activate. Bonus-tip pinned: Sec.A.+1 retained → 2 checks. Also fixed: `<Armor Purge>`'s accept prompt offered on a NEIGHBOR's deletion (now subject-scoped upstream). | `b::q24_hudiemon_alliance_partner_deleted_by_rules_check_before_trigger` |
| 25 | E | YES (DigiXros departure ≠ battle) | **PASS** | RESOLVED 2026-06-03 (`G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER`): added `ReplacementCause::DigiXros` + fire the `WhenWouldLeaveBattleArea` replacement window per battle-area material before consuming. BT17-095's `[All Turns]` leave observer now fires on a DigiXros departure. | `e::q25_all_turns_fires_on_digixros_departure_not_battle` |
| 26 | C | Returns to hand | **PASS** | RESOLVED 2026-06-03 (`G-DIGIXROS-REDIRECT-EXTRACTION`): added the leaving/limbo holding slot (`Game::digixros_leaving_limbo`). The leave window parks BT17-095's `<Delay>` without committing the host; WarGreymon is held in limbo (resolvable + extractable); the accepted DNA-evo re-materializes it into Omnimon; finalize finds the DigiXros recipe dropped below `min` and returns Dorbickmon to hand. Supporting fixes: identity re-resolution of the parked replacement's source/subject after the `battle_area` index shift, and excluding the in-flight host from DNA-partner candidates. | `c::q26_dorbickmon_returns_to_hand_when_cost_unpayable_after_dna_evo` |
| 27 | C | Pays 0 memory | **PASS** | RESOLVED 2026-06-03 (`G-DIGIXROS-UNPAYABLE-RETURN-TO-HAND`): the host commits only at `finalize_digixros_play_after_leave_windows`; when a pruned material leaves the recipe unsatisfied the play is abandoned with 0 memory paid. | `c::q27_dorbickmon_pays_zero_memory_when_returned_to_hand` |
| 28 | A | YES, plays AND activates | **PASS** | RESOLVED 2026-06-11: EX5-060 Dragomon authored (IMPLEMENTED) on four substrate widenings — (1) `play_from_trash_free` plays for the BINDING OWNER's side (`G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED`: "your opponent plays… from THEIR trash" enters THEIR battle area); (2) `suspended: true` (`G-PLAY-ENTERS-SUSPENDED`: the permanent ENTERS suspended, before play-event observers); (3) `event_target_level` formula (`G-EVENT-PLAYED-LEVEL-FORMULA`: "level less than or equal to IT"); (4) the `[On Play]` suppress rider is now CONSULT-GATED on `permanent_is_unaffected_by_effect` vs the recorded suppressor (the rider is an effect of Dragomon on the played Digimon). BT20-059's protection re-authored as the CONTINUOUS `grant_effect_immunity` form (`G-DSL-CONTINUOUS-CONTROLLED-IMMUNITY-AURA` resolved via the floating-modifier substrate + immunity payload) — the prior snapshot form missed the later-played Ciel; the quiz board has Ciel in the TRASH at grant time (the first-wave note claiming otherwise misread the board). Control pins the unsuppressed-rider false-pass. | `a::q28_gankoomon_x_protection_beats_dragomon_on_play_lock` (+ `a::q28_control_no_protection_on_play_suppressed`) |
| 29 | E | 3 legal stacks | **PASS** | RESOLVED 2026-06-11: the full 6-card Bagra cluster authored — BT10-093 Yuu Amano (PARTIAL: clause 1 needs `G-DSL-ON-CARD-PLACED-UNDER-TRIGGER`), EX10-039 ChuuChuumon (IMPLEMENTED), EX10-044 Damemon (IMPLEMENTED), EX10-031 DarkKnightmon (PARTIAL: would-leave replay needs `G-DSL-WOULD-LEAVE-TRIGGERED-OBSERVER`), EX10-056 Bagramon (PARTIAL ×2), EX10-059 DarknessBagramon (PARTIAL ×3 — DigiXros path faithful). Engine fix: `preattach_digixros_material` recipe-validated pre-attached cards and silently dropped non-recipe ones — Yuu Amano's hook places arbitrary purple Digimon (DCGO `AddDigivolutionCardInfos` does not recipe-validate); added slot-independent `pre_attach_extra_material` fallback. Pins assert TWO of the 3 legal stacks (both purple cards in pick order; single-card variant) bottom→top exactly, the cost stack 16 −3 −3 −2×N, that Yuu's hook resolves BEFORE material selection, and that recipe materials commit at the bottom in spec order. | `e::q29_legal_digixros_stack_orderings_with_yuu_amano` (+ `e::q29_single_under_tamer_card_yields_third_legal_stack`) |
| 30 | C/E | Suspend both w/ cost reduction | **PASS** | RESOLVED 2026-06-11: the Valdur Arm wave authored — BT20-036 BanchoLeomon, EX3-063 Imperialdramon: Dragon Mode, BT16-077 Dinobeemon, EX3-008 Flamedramon (all IMPLEMENTED; BT20-037 + EX8-074 pre-existing, EX8-074's suspend-2 re-audited to ANY battle-area Digimon per DCGO). **Engine re-timing: `<Partition>` moved from post-trash `OnDeletion` to an OPTIONAL, NON-CANCELLING `WhenWouldLeaveBattleArea` replacement** — the printed reminder reads "would leave" and the judge calls it interruptive (the carrier is still on field while the partition plays + their would-play interrupts resolve, so Chaosmon itself is a legal suspend target). Both picks extract from the live stack BEFORE either plays (no on-trash event — the cards transit, not trash), and the second play chains via the new `Game::run_after_selections_drain` so it starts only after the first's interrupt chain settles (the judge's "played out simultaneously": BanchoLeomon is not in the battle area during Medieval's suspend picks). DSL fix: `kind: partition` granters now carry `granted_keyword` so the replacement candidate walk synthesizes the keyword auto-effect. The pin reproduces the whole line from Flamedramon's inherited [EoT] DNA digivolve and asserts the legal suspend set is EXACTLY {Imperialdramon: Dragon Mode, Chaosmon: Valdur Arm}. | `c::q30_partition_interruptive_suspends_both_with_cost_reduction` |

## Verdict tally

| Verdict | Count | Questions |
|---------|-------|-----------|
| BLOCKED-CARD | 0 | — |
| BLOCKED-PRIMITIVE | 1 | 8 |
| CANDIDATE | 0 | — |
| PASS | 29 | 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30 |

(Counts: 0 + 1 + 0 + 29 = 30 of 30.)

Q29 → **PASS** (2026-06-11): the 6-card Bagra DigiXros cluster authored
(BT10-093 Yuu Amano, EX10-039, EX10-044, EX10-031, EX10-056, EX10-059).
The two pins reproduce the judge's stack-legality ruling end-to-end through
a real `r.play()` DigiXros transaction: Yuu Amano's would-play hook resolves
BEFORE material selection; its placed cards sit on TOP of the digivolution
cards in pick order; the recipe materials commit at the BOTTOM in
requirement-spec order ([Bagramon] above [DarkKnightmon]); and the cost
stacks 16 −3 −3 −2×placed. Engine widening: pre-attached materials no
longer have to satisfy a recipe slot (DCGO `AddDigivolutionCardInfos`
parity) — `DigiXrosTransaction::pre_attach_extra_material`. Side-discovery
while QA'ing EX10-044: a mid-tail free play that fires a would-play
cost-reduction interrupt clobbers the outer deletion trigger context
(`G-TRIGGER-CONTEXT-CLOBBERED-BY-COST-REDUCTION-INTERRUPT`, engine-gaps.md).

Q28 → **PASS** (2026-06-11): EX5-060 Dragomon authored with four substrate
widenings (opponent-side trash play; suspended entry; event-level formula;
protection-gated [On Play] suppression) and BT20-059's "none of your
Digimon are affected…" re-authored as a CONTINUOUS controlled immunity
(floating descriptor + immunity payload — covers the later-played Ciel,
matching the judge's "persistent effect" language; the prior per-permanent
snapshot was wrong for the quiz's own board, which has Ciel in the trash at
grant time). One faithful side-discovery while pinning: an unsuppressed
[On Play] memory gain that crosses the gauge ENDS the turn (rule 8-3) and
the new turn's unsuspend phase would mask the suspended-entry assertion —
the pin stages gauge headroom; the real-game line where B is at 0 memory
genuinely passes the turn.

Q15 → **PASS** (2026-06-11): the four-card Q15 wave (BT19-073 / BT17-016 /
BT12-016 / EX3-057, all IMPLEMENTED) landed via a background authoring agent
(its run stalled overnight against build-lock collisions; the on-disk work
was salvaged, verified, and finished by the orchestrator). EX8-073 was
re-authored so the "[All Turns] while you have 0 or less memory, this
Digimon isn't affected by your opponent's Digimon's effects" rides a new
`effect_immunity` aura slot instead of an approximation. The sequential
De-Digivolve rule pins: BT19-073's `[When Digivolving]` is N separate
`<De-Digivolve 1>` instances (DCGO `IDegeneration` per-instance loop), each
re-checking the target's immunity after every trashed card — so exposing
Gallantmon (X Antibody) mid-sequence halts the rest (judge: it stays
topmost; exactly one card trashed).

Q3 → **PASS** (2026-06-10): EX10-020 Puppetmon + BT12-057 Quartzmon authored.
The restriction clause landed the **`modifier_name` aura widening** — a DSL
aura can now install a Name-payload modifier (`CanOnlyDigivolveInto
"Apocalymon"`), completing the deferred DSL slice of
G-DIGIVOLVE-TARGET-RESTRICTION. Because declarative auras tick from
battle-area sources only, the restriction is automatically inactive in the
breeding area — the Q3 ruling falls out by construction (DCGO
`IsExistOnBattleArea` agrees). Two incidental EX10-020 clauses are PARTIAL
on new vocab gaps (hand-main self-play; security was-face-up gate — see
qa/dsl-vocab-gaps.md), both irrelevant to Q3. TDD on Quartzmon's
"[All Turns] all other Digimon and Tamers don't unsuspend" exposed and fixed
a real engine gap: the turn-start bulk unsuspend (`Player::unsuspend_all`)
never consulted `CannotUnsuspend` (only Reboot's path did) — the unsuspend
phase now skips locked permanents. A small predicate widening also landed:
`no_face_up_security_named` gained a `color_is` arm (EX10-020's "no GREEN
face-up security" On-Deletion gate).

Q24 → **PASS** (2026-06-10): `G-SUSPEND-EFFECT-INITIATED` closed — the suspend /
unsuspend events now carry an `effect_initiated` bit (`EffectContext::suspend`
tags `true`; attack/blocker/cost suspends `false`), feeding the existing DSL
`event_is_effect_initiated` predicate — and EX6-004 Kokomon authored (single
inherited `[Your Turn][OPT]` clause). Discover-then-pin yield was unusually
rich: the scenario exposed FOUR adjacent engine divergences, all fixed:
1. **`<Alliance>` keyword side inversion** — the engine gated the window on
   the ALLY carrying Alliance; officially (and DCGO `AllianceSelfEffect`) the
   keyword is on the ATTACKER and any other unsuspended own Digimon is a
   legal ally. `try_enter_alliance` corrected; `combat/alliance_interrupt.rs`
   re-pinned to the attacker-side model (+ a new negative pin).
2. **Alliance resolution order** — the callback suspended by raw flag-set
   (no OnSuspend observers, no aura re-tick) and read the ally's DP BEFORE
   suspension. Now: suspend through `Game::suspend_with_cause(ally, true)`
   inside a deferred-drain scope, read DP AFTER (DCGO `AllianceProcess`),
   grant +DP/Sec.A.+1, then flush.
3. **DP floor at 0** — `effective_dp` returned negative values; rules
   17-1-3-1 + DCGO `Permanent.DP` floor at 0. A suspended Tentomon at
   1000−4000 reads 0 → Alliance adds +0 ("won't give any DP").
4. **Rule processing before trigger activation** — the outermost
   `drain_effect_queue` now runs `run_state_based_rules_check` BEFORE
   resolving queued triggers, so the ≤0-DP Tentomon is deleted before
   Kokomon's parked inherited trigger activates (the judge's exact line:
   "deleted due to rules check before the inherited effect has a chance to
   activate").
Plus an incidental: `<Armor Purge>`'s optional accept dialog was offered on
a NEIGHBOR's deletion (candidate collection enumerates all permanents);
now subject-scoped upstream via `replacement_condition`. Suspending also
re-ticks declarative auras at the chokepoint so suspension-keyed auras
(BT16-101) materialize immediately. Q14's pin was adjusted: the earlier
rules check means the blanket driver now ACCEPTS Nyabootmon's faithful
optional `[On Any Deletion]` recursion, so the assertion pins the FIRST
application (−6000) and tolerates the −3000 recursion entry.

Q9 → **PASS** (2026-06-06): authored BT23-102 Mastemon (PARTIAL — controller-side
trim blocked by `G-TRASH-SECURITY-BATCH-INTERRUPTED-BY-OBSERVER`) + BT15-037
Gatomon (PARTIAL — play-from-security-when-trashed blocked by
`G-DSL-ON-DISCARD-SECURITY-TRIGGER`). Cluster-D activation-site ruling: Gatomon's
`[All Turns]` memory is gated on the carrier being in the battle area (DCGO
`IsExistOnBattleArea`); trashed FROM the opponent's security by Mastemon's trim it
never fires → 0 memory. The engine enforces this by construction (triggered effects
dispatch only for battle-area permanents), so no engine change was needed — the pin
+ the field control (`bt15_037_on_field_gains_memory...`) jointly prove it isn't a
no-op false-pass. Pinned to the OPPONENT-side trim to avoid the controller-trim
gap. Both cards' YAML was recovered from the authoring sub-agent's transcript after
its untracked output was removed by cross-worktree `git clean` activity, then
staged/committed immediately.

Q4 → **PASS** (2026-06-05): authored AD1-002 Aldamon (PARTIAL — alt-digivolve
≥2-Hybrid-sources count is `G-DSL-DIGISOURCE-TRAIT-COUNT-GTE`, incidental to Q4) +
BT4-098 Atomic Inferno (IMPLEMENTED — all 3 clauses; latent engine over-fire
`G-ENGINE-GRANTED-ONBLOCK-CARRIER-GATE` on the granted on_block trigger, positive
case correct). Q4 is the live-card realization of the **reduction** case that
`mid_attack_security_attack_recompute.rs` Test 3 deferred: a Security-Attack-reducing
*security* effect (Holy Flame's `[Security]` `<Security A. −1>`) flips mid-attack and
the engine's per-iteration `current_security_strike` recompute correctly drops the
remaining checks (net +1−1 = 1 check). Pinned by
`g::q4_security_attack_net_modifiers_one_check` + the false-pass control
`g::q4_control_atomic_inferno_plus_one_alone_checks_two`. The **modifier** path of
the recompute is proven correct.

Recompute-suite RED — **RESOLVED 2026-06-05 (test-only fix).** While running the
combat hot-path gate, the `mid_attack_security_attack_recompute` suite was found
RED (2/3, `left:0 right:1`). Systematic root-cause: the recompute is CORRECT — the
loop checks exactly 2 cards and leaves 1 (verified by instrumenting
`current_security_strike` + the dispose loop: strike reads 2 every iteration). The
failure was a **test artifact**, bisected to `053a06ad` (#582,
`G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT`): once tokens became valid `kind: digimon`
field-select targets, the Petrification token that Medusamon's
`on_opponent_security_removed` clause plays for the defender became a legal target
for Medusamon's OPTIONAL `[End of Attack]` "delete 1 of opponent's lowest-DP
Digimon". The test's blanket `drive_to_completion` auto-activated that optional →
token deleted → token `[On Deletion]` trashes the defender's TOP security → a 3rd,
unrelated security loss. The engine is faithful (the cascade is correct Medusamon
behavior); the test simply must not opt into it. Fix: the driver is now scoped to
the in-flight security loop (`drive_security_loop_to_completion`, stops when
`security_resolution` clears), so the check count is asserted at loop completion
before the post-loop `[End of Attack]` optional. No engine change. Suite green
(2 passed, 1 ignored).

Q18 → **PASS** (2026-06-05): LM-020 Quantumon fully authored (both clauses). The
ledger's "only the security→deck clause is blocked" was an undercount — the card
needed several primitives. Landed two reusable ones: the `return_selected_security_to_deck`
verb + `EffectContext::return_security_card_to_deck` engine primitive
(`G-DSL-RETURN-SELECTED-SECURITY-TO-DECK` CLOSED), and a `binding_card_kind`
predicate (compare a bound card's category to a declared one). Clause 2
([Start of Opp Turn]) declares a category via `select_effect_choice`, reveals the
opponent deck-top, and grants `grant_effect_immunity` with `source_controller: any`
(covers OWN effects) when the kinds match. Because `<Blast Digivolve>` is a Digimon
effect and combat gates blast on `permanent_is_unaffected_by_effect(.., own, Digimon)`,
declaring Digimon makes Quantumon unable to blast-digivolve into Imperialdramon: PM
ACE (BT17-077). Q18 test pins the gate from the REAL clause-2 effect with a control.
Clause 1 ([When Digivolving]) place-Digimon-on-security cost + security→deck-top +
shuffle uses the new verb. Regression: judge_quiz 30, cards_behavioral 3900,
dsl 760, combat 213, effect_context 145, lib 212 — green.

Q21 → **PASS** (2026-06-05): `G-DSL-DELETED-SELF-TRASH-BINDING` CLOSED + BT3-109
authored. The gap premise was mostly wrong — the `event_card`/`event_target`
bindings ALREADY resolve the just-deleted carrier's top card in trash
(`DeletedObjectSnapshot.top_card`). The only real missing link: `play_from_trash_free`
accepted a trash-index binding but not a card-handle binding, so `event_card`
couldn't feed it. Fixed generally by making the `PlayFromTrashFree` step arm accept
`ResolvedBinding::Card(h)` (engine call self-guards the handle is in trash). BT3-109
Back for Revenge! grants a chosen Digimon `[On Deletion]` self-replay-from-trash;
replaying the carrier leaves the trash, so the remaining draw bundle is suppressed
by the Q19 top-most-card-in-trash gate ⇒ 0 draws. Composes cleanly with Q19 — both
D-cluster activation-site questions now PASS. Behavioral test:
`tests/cards_behavioral/bt3/bt3_109.rs`; regression: cards_behavioral 3896, full
engine suite green except the 4 pre-existing cost_hooks failures.

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
