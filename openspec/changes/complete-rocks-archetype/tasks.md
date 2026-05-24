## 1. Calibration spike (gates the plan)

- [x] 1.1 Resolve the Rocks pool from `data/deck_library.json`; produce the current verdict table (IMPLEMENTED / PARTIAL / BLOCKED) by reading `validated_cards_dsl.json` AND running `cargo test --test cards_behavioral` per card — record tracker-vs-reality divergences
- [x] 1.2 Re-audit 5 representative PARTIAL cards via `/batch-implement-cards-rust-dsl` AUDIT mode (sample across source-trash, Delay, reveal/search, and security clusters)
- [x] 1.3 Classify the sample: `AUDITED-OK` (stale, already done) vs needs-a-clause-authored vs genuinely substrate-blocked; compute the stale-vs-genuine ratio
- [x] 1.4 Confirm EX10-003 flips to `IMPLEMENTED` (tracked BLOCKED, tests pass) — the canonical stale-verdict proof
- [x] 1.5 Resolve design Open Questions: source-check `predicate.rs` for an existing carrier-trait leaf (B1); check whether any in-flight track owns face-up security lifecycle (B4)

<!-- SPIKE RESULT (2026-05-22):
  1.1 — 47-card pool: cargo test = 239 passed / 0 failed / 9 ignored. Tracker claims 2 BLOCKED + 30 PARTIAL.
        Massive divergence: every card has passing tests; only 4 cards carry ignored tests.
  1.2/1.3 — 5 spike cards (EX10-032, P-039, P-107, EX8-067, EX7-049): 5/5 are genuinely-incomplete
        (real omitted clauses), 0/5 stale-already-done. BUT every omitted clause's substrate has
        landed → Phase A is REAL AUTHORING WORK (~28 cards, 1-2 clauses each), not reclassification.
  1.4 — EX10-003 confirmed done (cancel_attack ships, 1 passed / 0 ignored). Verdict flips BLOCKED→IMPLEMENTED.
  1.5a — B1 LIKELY NOT A GAP: `source_permanent_trait_has` already exists (predicate.rs:369) and
         resolves against the carrier for inherited clauses. BT21-021's ignored test is probably a
         stale marker / authoring fix, not a substrate gap. -> see proposed artifact update below.
  1.5b — G-PLACE-SELF-AS-OPTION-PERMANENT resolved 2026-05-02; P-039/P-107 empty security clauses now authorable.
  1.5c — B4 face-up security overlaps `.claude/plans/rust-engine-gaps-dark-masters.md`; `Player.face_up_security`
         substrate partially exists. Confirm ownership before building B4.
-->


## 2. Phase A — authoring re-audit of PARTIAL cards

- [x] 2.1 Run `/batch-implement-cards-rust-dsl` AUDIT mode over all remaining PARTIAL Rocks cards in batches of 4
- [x] 2.2 For each `AUDITED-OK` card, update its `validated_cards_dsl.json` verdict to `IMPLEMENTED` with verified test count
- [x] 2.3 For each card with an omitted clause whose substrate has already landed, author the missing clause TDD-style (failing test first) and bring the card to `IMPLEMENTED`
- [x] 2.4 Prune every `#[ignore]` marker on a Rocks test that cites a gap confirmed closed against current engine source; re-enable and confirm the test passes
- [x] 2.5 Produce the precise list of genuinely-substrate-blocked cards/clauses remaining after Phase A
- [x] 2.6 Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral` — confirm green, no regressions

<!-- PHASE A RESULT (2026-05-22): 26 PARTIAL cards processed in 5 batches of ~6 agents.
  Rocks pool went 15/30/2 (IMPL/PARTIAL/BLOCKED) -> 37/9/1.
  23 cards newly IMPLEMENTED; 5 reclassified already-done (EX10-003, EX7-074, BT9-103, EX8-047, P-167).
  Full engine suite green: all 39 test binaries pass, 0 failures. One regression (option_flow link
  test, from the ST22-11 game_actions.rs change) found and fixed.
  Genuinely-substrate-blocked remaining (10 cards), with the gap each needs:
    B2  P-130     — G-MOVE-BREEDING-DSL (move_from_breeding verb)
    B3  EX11-065  — G-DSL-SELECT-OWN-SOURCES-FILTER union (hand u digivolution-source cost)
    B3  EX11-038  — same union gap
    B4  BT20-055  — face-up security lifecycle (flip + checks-face-up observer)
    B5  BT23-096  — G-DSL-DELAY-ON-ATTACK-EVENT
    NEW BT23-059  — G-ON-OPTION-TRASHED-DSL (DSL Timing enum lacks on_option_trashed)
    NEW BT8-094   — G-EVENT-TARGET-LEVEL-LTE (event_target_level_lte/eq/gte predicate)
    NEW EX10-034  — G-DSL-GRANT-TRIGGERED-EFFECT-TO-BINDING
    NEW EX11-044  — G-HIGHEST-PLAY-COST-SELECTOR + event_host_permanent_is_source
    NEW EX8-050   — play-from-reveal-free sub-step
  Phase A surfaced 5 NEW small gaps not in the original B1-B5 scope — see task group 10. -->

## 3. B1 — BT21-021 inherited Rush (collapsed: authoring, not substrate)

<!-- Spike outcome: `source_permanent_trait_has` already exists (predicate.rs:369).
     B1 is no longer a Phase B substrate slice — done in Phase A. -->

- [x] 3.1 Verify the `kind: aura` evaluation path consults `source_permanent_trait_has` for an inherited clause's condition (read the aura-eval code path); if it does not, file the narrow wiring fix
- [x] 3.2 Re-author BT21-021's inherited `[Your Turn]` Rush aura with `source_permanent_trait_has: "Xros Heart"` as its condition; re-enable `bt21_021_inherited_rush_only_if_carrier_has_xros_heart` and confirm it passes

## 4. B2 — move-from-breeding DSL verb (P-130)

- [x] 4.1 Write a failing DSL/behavioral test for a `move_from_breeding` step with an optional accept/decline prompt and a breeding-permanent `filter`
- [x] 4.2 Add the `move_from_breeding` step (`StepSpec` → `CompiledStep` → lowering) in `code/digimon-dsl/src/` over the existing `EffectContext::move_from_breeding_by_effect`
- [x] 4.3 Implement the optional level-filtered prompt wrapper so the choice surfaces through `pending_selection` (CLAUDE.md §17); pair it with the already-wired `SelectOwnBreedingPermanentArgs::filter`
- [x] 4.4 Confirm tests pass; add regression coverage under `tests/dsl/` / `tests/selection/`

## 5. B3 — union-zone cost selector (EX11-065)

- [x] 5.1 Write a failing test for a cost selector spanning hand ∪ own digivolution-card sources with a single trait filter, trashing the chosen card
- [x] 5.2 Decide whether the union selector composes existing `SelectionKind`s or needs a new kind; if new, append an additive pending-selection action sub-range (never renumber) and update `docs/ACTION_SPEC.md`
- [x] 5.3 Implement the union-zone cost selector in `code/digimon-engine/src/effect_context/` parameterised by zone set (hand∪source for EX11-065; also expressible as hand∪trash for Royal Knights `G-UNION-HAND-TRASH-SOURCE-COST`)
- [x] 5.4 Wire DSL parse → compile → lowering for the union-zone cost verb
- [x] 5.5 Confirm tests pass (including the no-eligible-card negative case); add regression coverage

## 6. B4 — face-up security lifecycle (BT20-055)

- [x] 6.1 Write failing tests for (a) a no-choice `flip_security_face_up` step and (b) a "when your Digimon checks a face-up security card" observer timing
- [x] 6.2 Implement the `flip_security_face_up` primitive in the engine security module + its DSL step
- [x] 6.3 Implement the face-up-security-check observer timing: dispatch wiring + DSL `when:` token
- [x] 6.4 Confirm tests pass; move the closed face-up-security entries from `docs/RUST_ENGINE_GAPS.md` to `qa/resolved-gaps.md`

## 7. B5 — Delay-on-attack trigger support (BT23-096)

- [x] 7.1 Write failing tests for a `<Delay>` clause armed by an attack event, including the attacker-trait condition
- [x] 7.2 Extend `lower_delay.rs` to map attack timings (`OnAllyAttack` et al.) to an `OnEvent` `DelayTrigger` instead of degrading to `EndOfYourNextTurn`
- [x] 7.3 Extend combat dispatch (`combat.rs` / `effect_queue.rs`) to fan attack events out to event-gated delayed options
- [x] 7.4 Add a delay-context attacker predicate so the activation condition can read the attacking Digimon's traits for a plain attack
- [x] 7.5 Confirm tests pass; run the full `combat` and `option_flow` suites as the hot-path regression gate

## 8. Phase C — author the substrate-blocked clauses

- [x] 8.1 BT21-021 — handled in task group 3 (B1 collapsed to authoring); no Phase C work remains
- [x] 8.2 P-130 — author the `[On Play]` move-from-breeding clause with the optional level-3+ prompt; remove the stale `#[ignore]`s
- [x] 8.3 EX11-065 — author Clause 0 `[Start of Your Main Phase]` with the union-zone cost selector; remove the stale `#[ignore]`s
- [x] 8.4 EX11-038 — author the `[When Moving]/[On Play]` union-zone Draw clause (same B3 selector as EX11-065)
- [x] 8.5 BT20-055 — author the face-up-security-flip rider and the omitted `[Your Turn]` self-security-placement clause; add the previously-missing behavioral tests
- [x] 8.6 BT23-096 — author the `[Your Turn]` CS-attack `<Delay>` clause; remove the stale `#[ignore]`

## 10. NEW small gaps surfaced by Phase A (substrate slices)

<!-- These 5 gaps were discovered while authoring Phase A clauses. Each blocks one Rocks
     card and is an independent substrate slice (any order). Same pattern as B2-B5:
     failing test -> primitive -> green test -> author the card clause -> archive the gap. -->

- [x] 10.1 `G-EVENT-TARGET-LEVEL-LTE` — add `event_target_level_lte/eq/gte` predicate family to `PredicateSpec` (`code/digimon-dsl/src/predicate.rs`), wire through compile/lowering/evaluator; then author both BT8-094 clauses
- [x] 10.2 `G-ON-OPTION-TRASHED-DSL` — add `Timing::OnOptionTrashed` to the DSL `Timing` enum (`code/digimon-dsl/src/clause.rs`) + lowering to the engine's existing `EffectTiming::OnOptionTrashed`; then author BT23-059 Clause B
- [x] 10.3 `G-HIGHEST-PLAY-COST-SELECTOR` — add the `HighestPlayCost` sibling to `FieldSelector`/`CompiledFieldSelector` + evaluator (mirrors the resolved `LowestPlayCost`); then author EX11-044 Clause A
- [x] 10.4 `event_host_permanent_is_source` — add the DSL predicate gating `OnDigivolutionCardTrashed` to the observer-is-host case; then author EX11-044 Clause B
- [x] 10.5 `G-DSL-GRANT-TRIGGERED-EFFECT-TO-BINDING` — add a DSL verb that grants a printed triggered ability ("[Start of Your Main Phase] This Digimon attacks") to a selected permanent; then author EX10-034 Clause A
- [x] 10.6 EX8-050 play-from-reveal-free — confirm/close the gap blocking the "you may play 1 from the revealed cards free" sub-step; then author that sub-step

## 9. Phase D — reconcile and verify

- [x] 9.1 Finalize `qa/qa-reports/validated_cards_dsl.json`: every Rocks card has a verified verdict matching `cargo test` reality
- [x] 9.2 Move all gaps closed by B1–B5 from `qa/dsl-vocab-gaps.md` / `qa/archetype-qa/engine-gaps.md` to `qa/resolved-gaps.md` with resolution notes + test commands
- [x] 9.3 Update `qa/archetype-qa/dsl/rocks.md` with the final per-card verdict table
- [x] 9.4 Confirm no Rocks behavioral test carries an `#[ignore]` citing a closed gap
- [x] 9.5 Run the full `cargo test --manifest-path code/digimon-engine/Cargo.toml` suite — must be green with no regressions
