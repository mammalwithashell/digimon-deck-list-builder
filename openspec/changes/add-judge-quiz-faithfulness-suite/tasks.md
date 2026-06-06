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

- [ ] 3.1 Author the missing cards: Belphemon: Sleep Mode, Magnamon (X Antibody), Quantumon, (Imperialdramon: PM ACE if 1.2 unblocks), Gankoomon (X Antibody), Dragomon — full text, per-card TDD tests via the batch pipeline
- [ ] 3.2 Write Q1, Q2, Q17, Q18, Q28 scenario tests asserting the judge answers
- [ ] 3.3 Fix any surfaced gap: "affects me vs affects the battle" immunity scope (Q1/Q2), granted-effect ownership removal under immunity (Q17), self-immunity blocking own effect (Q18), protection beating "[On Play] don't activate" (Q28) — TDD, minimal primitive each
- [ ] 3.4 Confirm cluster A tests green; archive closed gaps

## 4. Cluster B — deferred rules-check (Q6, Q7, Q8, Q13, Q14, Q24)

- [ ] 4.1 Author the missing cards: Flame Hellscythe, Pillomon, Eye of the Gorgon, Agumon (Burst Digivolve), Koromon, Rapidmon (X Antibody), Hudiemon, Tentomon, Kokomon (Nyabootmon/ShoeShoemon/ShineGreymon: Ruin Mode already implemented)
- [ ] 4.2 Write Q6, Q7, Q8, Q13, Q14, Q24 scenario tests asserting the judge answers
- [ ] 4.3 Fix any surfaced gap: rules-check deferred until the ongoing effect fully resolves; DP measured at the right moment; sequential sub-effect ordering (delete-then-play); DP-less / Burst-Digivolve trash chain — TDD
- [ ] 4.4 Confirm cluster B tests green; archive closed gaps

## 5. Cluster C — declare-then-pay cost window (Q5, Q26, Q27, Q30)

- [ ] 5.1 Author the missing cards: (Omnimon AD1-025 already implemented), Dorbickmon, MedievalGallantmon, Imperialdramon: Dragon Mode, Chaosmon: Valdur Arm, BanchoLeomon (Miraculous Mega Knight already implemented)
- [ ] 5.2 Write Q5, Q26, Q27 scenario tests; write Q30 (shared with cluster E) for the interruptive-`<Partition>` + cost-reduction outcome
- [ ] 5.3 Fix any surfaced gap: declare-a-play-whose-cost-becomes-payable-after-declaration window (Q5); cost recomputed unpayable after a mid-resolution DNA-evolution ⇒ return to hand, 0 memory (Q26/Q27) — TDD
- [ ] 5.4 Confirm cluster C tests green; archive closed gaps

## 6. Cluster D — trigger activation site (Q9, Q19, Q20, Q21, Q23)

- [ ] 6.1 Author the missing cards: Gatomon, Mastemon, Eyesmon: Scatter Mode, Back for Revenge!, plus the On-Deletion/return-to-hand cards Q19/Q23 resolve to in the spike
- [ ] 6.2 Write Q9, Q19, Q20, Q21, Q23 scenario tests asserting the judge answers (draw counts / memory)
- [ ] 6.3 Fix any surfaced gap: [On Deletion] activates only from trash (return-to-hand suppresses it, Q19); remaining-in-trash gates inherited [On Deletion] (Q23); play-from-trash mid-resolution suppresses remaining effects (Q21); not-in-battle-area suppresses [All Turns] (Q9) — TDD. Cross-check CLAUDE.md §25 (OnDeletion post-trash contract)
- [ ] 6.4 Confirm cluster D tests green; archive closed gaps

## 7. Cluster E — `<Partition>` / DigiXros departure / de-digivolve (Q15, Q16, Q25, Q29; Q30 shared)

- [ ] 7.1 Author the missing cards: Lilithmon, Paildramon, Gallantmon (X Antibody), DarknessBagramon, Damemon, ChuuChuumon, Bagramon, DarkKnightmon, Yuu Amano (LordKnightmon (X Ant.) and Miraculous Mega Knight already implemented)
- [ ] 7.2 Write Q15 (sequential de-digivolve halted by mid-sequence X-Antibody immunity), Q16 (`<Partition>` not triggering on leave-by-own-effect), Q25 (`[All Turns]` fires on DigiXros departure), Q29 (legal DigiXros stack orderings) scenario tests
- [ ] 7.3 Fix any surfaced gap: `<Partition>` departure-cause discrimination (battle vs own-effect vs DigiXros); sequential de-digivolve respecting newly-acquired immunity on the new top card; DigiXros placement-order legality — TDD
- [ ] 7.4 Confirm cluster E tests green; archive closed gaps

## 8. Cluster F — token lifecycle & memory arithmetic (Q10, Q11, Q12, Q22)

- [ ] 8.1 Author the missing cards: Mental Training, Akihiro Kurata, MirageGaogamon, Sharkmon (Gravity Crush, Venusmon, Medusamon already implemented; Petrification token exists in `src/cards/tokens/`)
- [ ] 8.2 Write Q10, Q11 (multi-effect memory math + Once-Per-Turn re-trigger), Q12 (token placeable as digivolution card ⇒ unsuspend), Q22 (Digi-Eggs to egg deck still satisfy "send 2 to bottom" ⇒ 2 tokens) scenario tests
- [ ] 8.3 Fix any surfaced gap: token can be targeted/placed as a digivolution card though it doesn't remain (Q12); egg-deck routing still satisfies a send-to-bottom cost (Q22); OPT vs non-OPT re-trigger arithmetic (Q11) — TDD
- [ ] 8.4 Confirm cluster F tests green; archive closed gaps

## 9. Cluster G — zone/keyword scoping (Q3, Q4)

- [~] 9.1 Author the missing cards: **Aldamon AD1-002 (PARTIAL — done 2026-06-05; alt-path gap G-DSL-DIGISOURCE-TRAIT-COUNT-GTE), Atomic Inferno BT4-098 (IMPLEMENTED — done 2026-06-05)**; Holy Flame ST3-15 already impl. Still TODO for Q3: Puppetmon EX10-020, Quartzmon BT12-057.
- [~] 9.2 **Q4 (Security Attack count net +1/−1 ⇒ one check) DONE 2026-06-05** — `g::q4_security_attack_net_modifiers_one_check` + false-pass control `g::q4_control_atomic_inferno_plus_one_alone_checks_two`; it is the live-card realization of the `mid_attack_security_attack_recompute.rs` Test-3 *reduction* case. Q3 (breeding-area effect inactivity) still BLOCKED-CARD.
- [~] 9.3 Q4 surfaced no NEW engine gap on the modifier recompute path (it already reads live net Security Attack value, judge-correct). NOTE: discovered + **FIXED (test-only) 2026-06-05** a RED in `mid_attack_security_attack_recompute`. Root cause (bisected to #582 `G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT`): the recompute is correct (loop checks exactly 2); the test's blanket driver auto-activated Medusamon's optional `[End of Attack]` delete on a now-targetable Petrification token, whose `[On Deletion]` trashes the defender's top security (a faithful but unrelated cascade). Driver now scoped to the security loop (`drive_security_loop_to_completion`); no engine change. Q3's breeding-area-inactivity gap still pending its cards.
- [ ] 9.4 Confirm cluster G tests green; archive closed gaps (Q4 green; Q3 pending)

## 10. Reconcile and verify

- [x] 10.1 Created `qa/qa-reports/judge-quiz.md`: per-question verdict ledger (all 30 — cluster, judge answer, verdict, blocker/gap, test fn) + tally + gaps-surfaced + key lesson. (Will be re-finalized as scenarios flip from BLOCKED to PASS during authoring.)
- [ ] 10.2 Update `qa/qa-reports/validated_cards_dsl.json` with verified verdicts for every newly-authored card
- [ ] 10.3 Move every gap closed by this change from `engine-gaps.md` / `dsl-vocab-gaps.md` to `qa/resolved-gaps.md` with resolution note + test command; leave only genuinely-open gaps, each confirmed against current engine source
- [ ] 10.4 Confirm no `judge_quiz` test carries an `#[ignore]` except those citing a verified `BLOCKED-DATA` / unimplemented-card blocker
- [ ] 10.5 Run the full `cargo test --manifest-path code/digimon-engine/Cargo.toml` suite — green, no regressions; run `combat` and `option_flow` suites as the hot-path gate for any combat/cost gap fix
