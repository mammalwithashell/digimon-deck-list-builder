## 1. Gating spike — resolve the corpus

- [x] 1.1 Per-question card-resolution table frozen in [`card-resolution.md`](./card-resolution.md) — all 30 questions' exact `card_id`s read off the PDF card images, with board state + judge answer. ≈70 distinct cards; no `BLOCKED-DATA` scenarios (Q18 = BT17-077 confirmed). (Done during exploration.)
- [x] 1.2 Q4 resolved: Aldamon **AD1-002** + Atomic Inferno **BT4-098** (Sec.A.+1) + Holy Flame **ST3-15** (Sec.A.−1). (Done.)
- [x] 1.3 Implementation status re-derived against the authoritative IDs (filesystem scan, 2026-05-28). Result in `card-resolution.md` §"Implementation status": **27 of 79 implemented (DSL), 52 to author, raw_rust empty**. Discovery-wave-ready (zero authoring): **Q2, Q5, Q22, Q23**. Per-cluster authoring load tabulated there. (NOTE: "YAML present" still needs an AUDIT-mode faithfulness pass per card before a scenario is pinned; BT7-107 lacks a test.)
- [ ] 1.4 For each question, record the `RULES_CONTEXT.md` citation and the `DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs` reference for the processing-order detail
- [x] 1.5 Loader smoke test `loader::all_implemented_quiz_cards_load_from_embedded_pack` asserts all 27 implemented quiz cards load via `DebugRunner::dsl_card` — PASSES (2026-05-28). No packaging gaps; the discovery-wave scenarios can compose these cards by chaining `.dsl_card(id)`.
- [x] 1.6 Scaffolded `code/digimon-engine/tests/judge_quiz/` (main.rs + 7 cluster modules a_immunity_scope…g_zone_keyword, each with a documented question/card/judge-answer header) + `loader.rs`; registered `[[test]] name = "judge_quiz"` in `Cargo.toml` (required-features dsl-yaml-loader). Compiles clean; `cargo test --test judge_quiz` green (1 test).

## 2. Discovery wave — pin/expose the already-implementable scenarios (no authoring)

- [x] 2.1 ALL 30 questions encoded as test entries across the 7 cluster modules — Q2/Q5/Q22 audited with real findings, Q23 a documented candidate, the other 26 BLOCKED-CARD `#[ignore]` stubs naming exact missing cards. Each docstring cites question + judge answer + cards (+ DCGO where audited).
- [x] 2.2 `cargo test --test judge_quiz` → 1 passed (loader), 30 ignored, 0 failed. Q22 was run WITHOUT ignore first and confirmed FAILING (real evidence) before cite-and-ignore.
<!-- DISCOVERY-WAVE LOG (in progress, 2026-05-28):
  Q2 (cluster A) — BLOCKED on engine primitive G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT
     (EX1-068 Ice Wall [Main] grant omitted). Progress immunity itself IS implemented.
     `#[ignore]`d test written; consumer logged on the gap in qa/dsl-vocab-gaps.md.
  Q5 (cluster C) — BLOCKED-DATA: AD1-025 [Assembly] absent from cards.json (real card has it,
     DCGO AD1_025.cs:214-255). assembly alt-path KIND supported (BT18-102). `#[ignore]`d test written.
  Q22 (cluster F) — DISCOVERED ENGINE BUG, PROVEN: return_trash_cards_to_deck_bottom
     (effect_context/mod.rs:5554) inserts Digi-Eggs into the MAIN deck, no digitama routing. Focused
     test ran and FAILED (digitama_deck 0, expected 1); now `#[ignore]`d citing
     G-RETURN-TRASH-DIGI-EGG-ROUTING (engine-gaps.md). Small fix: branch on CardKind::DigiEgg.
  Q23 (cluster D/F) — candidate, same return verb + "remain-in-trash to resolve" gating; audit pending.
  RULE-LEVEL PROBES (beyond per-scenario stubs — probe the engine rule each cluster needs,
  independent of card authoring):
  - Cluster B root rule — G-NO-GENERAL-ZERO-DP-RULES-CHECK (PROVEN). No general state-based ≤0-DP
    deletion; only run_rule_check_after_arts (Arts-only). Synthetic probe
    `zero_dp_probe_reduced_digimon_deleted_after_effect_resolves` confirmed failing. Systemic —
    blocks Q6/Q8/Q13/Q14/Q24 at the engine level (in addition to their card-authoring blocks).
    Highest-impact fix. Companion `zero_dp_probe_healthy_digimon_survives_drain` passes.
  - Cluster D/F root rule — G-ON-TRASH-OBSERVER-SYNCHRONOUS (CONFIRMED+probed). Q21/Q23 need
    on-trash inherited effects to DEFER and re-check remain-in-trash; `fire_digivolution_card_trashed`
    (game_actions.rs:3308) enqueues + immediately drains (synchronous, intentional for EX10-036) →
    can't defer → Q23 over-counts (+3 vs +1). Design tension, not a trivial fix. Probe
    `cluster_d_on_trash_observer_fires_synchronously_not_deferred` characterizes it (passes).
    Q21 (OnDeletion path) probable but needs separate verification.
  - Cluster E Partition cause-filter — PRESENT (not a gap): keyword_effects.rs:839 skips
    Battle | OwnEffect. (Q16's granted-effect-counts-as-own attribution still needs the Q2 grant
    primitive + cause-attribution verification once authorable.)
  - Cluster A immunity machinery — PRESENT (not a gap): permanent_is_unaffected_by_effect
    (game.rs:3468) supports EffectControllerFilter::{Any,OpponentOnly,OwnOnly} + source_kind. Q18's
    "immune to ALL incl own" = Any. Probe `cluster_a_self_immunity_blocks_own_controller_effect`
    passes. Q18 still BLOCKED-CARD on Quantumon LM-020 + needs the <Blast Digivolve> path to consult
    can_affect_permanent (card-gated). Q17 also needs the Q2 grant primitive.
  - Cluster F (Q10/Q11 OPT) + Cluster G (Q3 breeding-inactivity, Q4 security net-count) — NOT deeply
    probed: engine has once_per_turn tracking, an `in_breeding` predicate (predicate.rs:432), and
    existing `mid_attack_security_attack_recompute` coverage for Q4's net-strike rule. Remaining
    scenarios are card-specific; verify when the cards are authored.
  Pattern: 5 distinct gaps surfaced (missing rules-check / engine bug / sync-vs-defer tension /
    engine primitive / source data) + 2 machinery areas confirmed PRESENT (Partition cause-filter,
    immunity controller-filter). Suite: 4 passed, 31 ignored, 0 failed. -->

- [x] 2.3 The one genuine-bug FAIL (Q22) logged to `engine-gaps.md` as `G-RETURN-TRASH-DIGI-EGG-ROUTING`. Q2 consumer logged on the existing `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` entry in `dsl-vocab-gaps.md`. Q5 data-gap documented in card-resolution.md + the Q5 test.
- [x] 2.4 Discovery summary produced: `qa/qa-reports/judge-quiz.md` (per-question verdict ledger) + `card-resolution.md` §"Implementation status". Tally: 0 PASS, 1 BUG, 1 CANDIDATE, 1 BLOCKED-PRIMITIVE, 1 BLOCKED-DATA, 26 BLOCKED-CARD. 3 distinct gaps + 52 cards to author.

<!-- Cluster phases are ordered by gap-likelihood (A→B→C→D→E→F→G), per design D1:
     the primary goal is finding & patching engine gaps. The §2 discovery wave runs first
     across all clusters and may re-order these phases by observed divergence; a cluster the
     wave shows the engine already handles can be deprioritized. -->

## 3. Cluster A — immunity scope (Q1, Q2, Q17, Q18, Q28)

- [x] 3.1 Author the missing cards — ALL DONE: BT13-088 Belphemon: Sleep Mode + BT16-102 Magnamon (X Antibody) + BT20-059 Gankoomon (X Antibody) (prior waves), LM-020 Quantumon (2026-06-05), **EX5-060 Dragomon (IMPLEMENTED — 2026-06-11)**; BT17-077 Imperialdramon: PM ACE unblocked by 1.2.
- [x] 3.2 Write Q1, Q2, Q17, Q18, Q28 scenario tests — **all five PASS** (Q28 done 2026-06-11 with a no-protection control).
- [x] 3.3 Gaps fixed: Q1/Q2/Q17 (grant-triggered-effect waves), Q18 (immunity + security-return verbs), **Q28 (2026-06-11): protection-gated [On Play] suppression (`fire_play_event_triggers` consults `permanent_is_unaffected_by_effect` vs the recorded suppressor), opponent-side trash play, suspended entry (`G-PLAY-ENTERS-SUSPENDED`), `event_target_level` formula, and the CONTINUOUS `grant_effect_immunity` form (`G-DSL-CONTINUOUS-CONTROLLED-IMMUNITY-AURA` — floating immunity covering later entrants; BT20-059 re-authored)**.
- [x] 3.4 Cluster A green — Q1/Q2/Q17/Q18/Q28 all PASS (judge_quiz 39/0/3 as of 2026-06-11).

## 4. Cluster B — deferred rules-check (Q6, Q7, Q8, Q13, Q14, Q24)

- [~] 4.1 Author the missing cards: **Kokomon EX6-004 (IMPLEMENTED — done 2026-06-10, with G-SUSPEND-EFFECT-INITIATED)**; Flame Hellscythe, Pillomon, Eye of the Gorgon, Rapidmon (X Antibody), Hudiemon, Tentomon done in prior waves. Still TODO for Q8: the Burst stack is implemented but blocked on `add-burst-digivolve` (G-BURST-ON-TURN-END-NOT-EXECUTED — greenfield burst resolution path, see `.claude/plans/rust-engine-gaps-burst-digivolve.md`).
- [~] 4.2 Write Q6, Q7, Q8, Q13, Q14, Q24 scenario tests asserting the judge answers — **Q6/Q7/Q13/Q14/Q24 PASS** (Q24 done 2026-06-10: `b::q24_hudiemon_alliance_partner_deleted_by_rules_check_before_trigger`); Q8 `#[ignore]` on the burst gap.
- [~] 4.3 Fix any surfaced gap — Q24 (2026-06-10) closed `G-SUSPEND-EFFECT-INITIATED` and surfaced+fixed four more: `<Alliance>` keyword was modeled on the ALLY (now attacker-side per DCGO `AllianceSelfEffect`); Alliance resolution order (suspend via chokepoint in a deferred-drain scope, read ally DP AFTER suspension); `effective_dp` floors at 0 (rules 17-1-3-1 / DCGO); outermost drain runs the state-based rules check BEFORE activating parked triggers. Incidental: `<Armor Purge>` accept dialog subject-scoped (was offered on neighbors' deletions); suspend chokepoint re-ticks declarative auras. Q8's burst gap remains (own change).
- [ ] 4.4 Confirm cluster B tests green; archive closed gaps (Q6/Q7/Q13/Q14/Q24 green; Q8 pending `add-burst-digivolve`)

## 5. Cluster C — declare-then-pay cost window (Q5, Q26, Q27, Q30)

- [ ] 5.1 Author the missing cards: (Omnimon AD1-025 already implemented), Dorbickmon, MedievalGallantmon, Imperialdramon: Dragon Mode, Chaosmon: Valdur Arm, BanchoLeomon (Miraculous Mega Knight already implemented)
- [x] 5.2 Q5/Q26/Q27 done in prior waves. **Q30 DONE 2026-06-11**: `c::q30_partition_interruptive_suspends_both_with_cost_reduction` PASSES — full board from Flamedramon's inherited [EoT] DNA digivolve; legal suspend set EXACTLY {Imperialdramon: Dragon Mode, Chaosmon: Valdur Arm}. Wave: BT20-036, EX3-063, BT16-077, EX3-008 authored (IMPLEMENTED); EX8-074 suspend-2 re-audited to ANY battle-area Digimon. Engine: `<Partition>` re-timed to an interruptive (optional, non-cancelling) WhenWouldLeaveBattleArea replacement + `run_after_selections_drain` sequencing + `granted_keyword` on partition granters. New OPEN gap: G-NESTED-PARKED-REPLACEMENT (engine-gaps.md).
- [ ] 5.3 Fix any surfaced gap: declare-a-play-whose-cost-becomes-payable-after-declaration window (Q5); cost recomputed unpayable after a mid-resolution DNA-evolution ⇒ return to hand, 0 memory (Q26/Q27) — TDD
- [ ] 5.4 Confirm cluster C tests green; archive closed gaps

## 6. Cluster D — trigger activation site (Q9, Q19, Q20, Q21, Q23)

- [x] 6.1 Author the missing cards: **Gatomon BT15-037 (PARTIAL — done 2026-06-06; G-DSL-ON-DISCARD-SECURITY-TRIGGER), Mastemon BT23-102 (PARTIAL — done 2026-06-06; G-TRASH-SECURITY-BATCH-INTERRUPTED-BY-OBSERVER)**; Eyesmon: Scatter Mode / Back for Revenge! / Q19-Q23 cards done in prior waves.
- [x] 6.2 Write Q9, Q19, Q20, Q21, Q23 scenario tests asserting the judge answers — **all PASS** (Q9 done 2026-06-06: `d::q9_gatomon_not_in_battle_area_during_removal_no_memory`).
- [x] 6.3 Fix any surfaced gap: Q19/Q21/Q23 resolved in prior waves; **Q9 needed no engine change** — the not-in-battle-area suppression of `[All Turns]` falls out of the existing trigger-dispatch (only battle-area permanents' triggers fire). Two incidental gaps logged (controller-trim batch-abort; on-discard-security DSL trigger), neither blocks Q9.
- [x] 6.4 Confirm cluster D tests green; archive closed gaps — Q9/Q19/Q20/Q21/Q23 all PASS.

## 7. Cluster E — `<Partition>` / DigiXros departure / de-digivolve (Q15, Q16, Q25, Q29; Q30 shared)

- [x] 7.1 Author the missing cards: Lilithmon EX6-057 done (grant wave); Q15 wave done 2026-06-11 (BT19-073, BT17-016, BT12-016, EX3-057 + EX8-073 re-authored with a real `effect_immunity` aura); **Q29 wave done 2026-06-11: BT10-093 Yuu Amano (PARTIAL — clause 1 needs `G-DSL-ON-CARD-PLACED-UNDER-TRIGGER`), EX10-039 ChuuChuumon (IMPLEMENTED), EX10-044 Damemon (IMPLEMENTED), EX10-031 DarkKnightmon (PARTIAL — `G-DSL-WOULD-LEAVE-TRIGGERED-OBSERVER`), EX10-056 Bagramon (PARTIAL ×2), EX10-059 DarknessBagramon (PARTIAL ×3; DigiXros path faithful).** (The earlier "three need G-DSL-HAND-MAIN-SELF-PLAY-REDUCED" note was wrong — none of these cards carry that clause; verified against card images.) Paildramon BT16-025 already implemented.
- [x] 7.2 **Q15 DONE 2026-06-11** (`e::q15_sequential_de_digivolve_halted_by_x_antibody_immunity`); Q16 + Q25 done in prior waves. **Q29 DONE 2026-06-11**: `e::q29_legal_digixros_stack_orderings_with_yuu_amano` + `e::q29_single_under_tamer_card_yields_third_legal_stack` PASS — real `r.play()` DigiXros transaction; Yuu's hook BEFORE materials; placed cards on top in pick order; recipe materials at bottom in spec order; cost 16 −3 −3 −2×N.
- [x] 7.3 Sequential de-digivolve respecting newly-acquired immunity: pinned (Q15). DigiXros placement-order legality (Q29): pinned; engine widening — `preattach_digixros_material` no longer recipe-validates (slot-independent `pre_attach_extra_material` fallback, DCGO `AddDigivolutionCardInfos` parity).
- [x] 7.4 Cluster E tests green (Q15/Q16/Q25/Q29). New OPEN gaps logged: `G-DSL-ON-CARD-PLACED-UNDER-TRIGGER`, `G-DSL-WOULD-LEAVE-TRIGGERED-OBSERVER`, `G-DSL-PLACE-PERMANENT-AS-SOURCE`, `G-DSL-BLIND-OPP-HAND-PLACE`, `G-DSL-GAIN-ALL-TURNS-FROM-SOURCES` (qa/dsl-vocab-gaps.md) + `G-TRIGGER-CONTEXT-CLOBBERED-BY-COST-REDUCTION-INTERRUPT` (engine-gaps.md).

## 8. Cluster F — token lifecycle & memory arithmetic (Q10, Q11, Q12, Q22)

- [ ] 8.1 Author the missing cards: Mental Training, Akihiro Kurata, MirageGaogamon, Sharkmon (Gravity Crush, Venusmon, Medusamon already implemented; Petrification token exists in `src/cards/tokens/`)
- [ ] 8.2 Write Q10, Q11 (multi-effect memory math + Once-Per-Turn re-trigger), Q12 (token placeable as digivolution card ⇒ unsuspend), Q22 (Digi-Eggs to egg deck still satisfy "send 2 to bottom" ⇒ 2 tokens) scenario tests
- [ ] 8.3 Fix any surfaced gap: token can be targeted/placed as a digivolution card though it doesn't remain (Q12); egg-deck routing still satisfies a send-to-bottom cost (Q22); OPT vs non-OPT re-trigger arithmetic (Q11) — TDD
- [ ] 8.4 Confirm cluster F tests green; archive closed gaps

## 9. Cluster G — zone/keyword scoping (Q3, Q4)

- [x] 9.1 Author the missing cards: **Aldamon AD1-002 (PARTIAL — done 2026-06-05; alt-path gap G-DSL-DIGISOURCE-TRAIT-COUNT-GTE), Atomic Inferno BT4-098 (IMPLEMENTED — done 2026-06-05)**; Holy Flame ST3-15 already impl. **Puppetmon EX10-020 (PARTIAL — done 2026-06-10; G-DSL-HAND-MAIN-SELF-PLAY-REDUCED + G-DSL-SECURITY-WAS-FACE-UP-GATE, both incidental to Q3) + Quartzmon BT12-057 (IMPLEMENTED — done 2026-06-10)**.
- [x] 9.2 **Q4 DONE 2026-06-05** (`g::q4_security_attack_net_modifiers_one_check` + control). **Q3 DONE 2026-06-10** — `g::q3_breeding_area_effect_inactive_allows_digivolve` PASSES: the `[All Turns]` restriction is the new `modifier_name` aura install of `CanOnlyDigivolveInto` (DSL slice of G-DIGIVOLVE-TARGET-RESTRICTION landed), battle-area-sourced ⇒ breeding-inactive for free; battle-area control in `ex10/ex10_020.rs` proves no false-pass.
- [~] 9.3 **Q3 (2026-06-10)** landed the `modifier_name` aura widening (Name-payload modifier installs — closes the deferred DSL slice of G-DIGIVOLVE-TARGET-RESTRICTION), a `color_is` arm on `no_face_up_security_named`, and surfaced+fixed a real engine gap: the turn-start bulk unsuspend ignored `CannotUnsuspend` (Quartzmon "[All Turns] don't unsuspend" TDD) — now honored at the phase site (game_phases.rs). Two new DSL vocab gaps logged (hand-main self-play; security was-face-up gate). Q4 surfaced no NEW engine gap on the modifier recompute path (it already reads live net Security Attack value, judge-correct). NOTE: discovered + **FIXED (test-only) 2026-06-05** a RED in `mid_attack_security_attack_recompute`. Root cause (bisected to #582 `G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT`): the recompute is correct (loop checks exactly 2); the test's blanket driver auto-activated Medusamon's optional `[End of Attack]` delete on a now-targetable Petrification token, whose `[On Deletion]` trashes the defender's top security (a faithful but unrelated cascade). Driver now scoped to the security loop (`drive_security_loop_to_completion`); no engine change. Q3's breeding-area-inactivity gap still pending its cards.
- [ ] 9.4 Confirm cluster G tests green; archive closed gaps (Q4 green; Q3 pending)

## 10. Reconcile and verify

- [x] 10.1 Created `qa/qa-reports/judge-quiz.md`: per-question verdict ledger (all 30 — cluster, judge answer, verdict, blocker/gap, test fn) + tally + gaps-surfaced + key lesson. (Will be re-finalized as scenarios flip from BLOCKED to PASS during authoring.)
- [ ] 10.2 Update `qa/qa-reports/validated_cards_dsl.json` with verified verdicts for every newly-authored card
- [ ] 10.3 Move every gap closed by this change from `engine-gaps.md` / `dsl-vocab-gaps.md` to `qa/resolved-gaps.md` with resolution note + test command; leave only genuinely-open gaps, each confirmed against current engine source
- [ ] 10.4 Confirm no `judge_quiz` test carries an `#[ignore]` except those citing a verified `BLOCKED-DATA` / unimplemented-card blocker
- [ ] 10.5 Run the full `cargo test --manifest-path code/digimon-engine/Cargo.toml` suite — green, no regressions; run `combat` and `option_flow` suites as the hot-path gate for any combat/cost gap fix
