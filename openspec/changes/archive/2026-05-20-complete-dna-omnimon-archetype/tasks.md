# Tasks — complete-dna-omnimon-archetype

> **Scope update 2026-05-19:** Phase A's reconciliation sweep verified all 87 ignored
> tests against engine code. Result: 8 STALE, 14 AUTHORING-ONLY, 1 CROSS-TRACK,
> **18 OPEN-SUBSTRATE** gaps. Verdict ledger: 34 IMPLEMENTED / 25 PARTIAL / 5 BLOCKED.
> Per-item evidence: `.workdata/classification.json`. User elected FULL SCOPE — close
> all 18 substrate gaps. Sections 3–5 below are the expanded Phase C.
>
> Every substrate task is TDD (CLAUDE.md §18): failing test first, then parser →
> compiled → lowering → engine wiring → `dsl_eval_arm_coverage` lint. Every card
> clause is verified against printed `data/cards.json` text. No-approximations §17.

## 1. Phase A — Reconciliation sweep

- [x] 1.1 Resolve the canonical DNA Omnimon card pool (64 cards) + deck-frequency.
- [x] 1.2 Enumerate every `#[ignore]` marker in DNA Omnimon behavioral tests.
- [x] 1.3 Classify all 41 gap items STALE / OPEN-SUBSTRATE / AUTHORING-ONLY / CROSS-TRACK against current code.
- [x] 1.7 Create `qa/qa-reports/validated_cards_dsl.json` verdict ledger (64 cards).
- [x] 1.5 Rewrite each OPEN-SUBSTRATE test's `#[ignore]` reason to cite the verified-open gap accurately (done per-gap inside sections 3–5).
- [x] 1.6 Fix the EX4-003 CROSS-TRACK item: give `make_lv4_digimon` non-empty `evo_costs` in the test helper; re-enable both EX4-003 inherited tests.
- [x] 1.8 After all sections land, run `cards_behavioral`, `dsl`, `dna_digivolve`, `digivolve`, `dsl_eval_arm_coverage` — no regressions.

## 2. Phase A — Re-enable STALE tests (8 items, no substrate needed)

- [x] 2.1 G-COUNT-AGGREGATE — re-enable `ex4_061` count_lte test; confirm pass.
- [x] 2.2 G-DECLARATIVE-KEYWORD — re-enable `ad1_001` + `bt20_102` grant_keyword tests; confirm pass.
- [x] 2.3 G-DSL-EVENT-TARGET-IS-SELF — add `event_permanent_is_source: true` to BT15-101 `on_suspend` clause; re-enable test.
- [x] 2.4 BT22-089 count_gte — re-enable `bt22_089` clause-2 no-eligible-trait test; confirm pass.
- [x] 2.5 EX10-010 — delete the 2 ignored CannotBeAffected tests duplicated by the passing SECTION 8 regression tests (or re-enable if not exact dupes).
- [x] 2.6 P-206 select_reveal — re-enable the 2 `p_206` Main reveal-filter tests; confirm pass.

## 3. Phase C — Small substrate gaps (6)

- [x] 3.1 `G-DSL-PREDICATE-TEXT-CONTAINS` — add `effect_text_contains` predicate leaf (parser/compiled/compile/engine eval); author BT22-017 bucket-1 clause; behavioral test.
- [x] 3.2 `G-EVENT-TARGET-NAME-CONTAINS` — add `event_target_name_contains` predicate leaf using the `event_target_card` resolver; author EX4-061 clause-2 name filter; test.
- [x] 3.3 `G-FORMULA-SOURCE-DP` — add a `source_dp` FormulaSpec variant (or `binding_dp: source`); author P-182 [When Digivolving] clause; test.
- [x] 3.4 `G-PLAY-COST-AGGREGATE` — add `LowestPlayCost` to `AggregateSelector` + `FieldSelector`; engine eval; (consumed by EX4-073 clause C in 5.x).
- [x] 3.5 `G-SELF-DIGIVOLUTION-CONTAINS-NAME` (sources-only) — add a predicate that scans digivolution sources only (excluding carrier top card); author BT20-102 negative-case clause; test.
- [x] 3.6 `G-ALT-PATH-DIRECTION-INTO` companion — add `distinct_tamer_colours_gte` BoolPredicate leaf (formula `distinct_colors_count` already exists); author ST20-10 warp-into-WarGreymon alt-path clause; test.

## 4. Phase C — Medium substrate gaps (9)

- [x] 4.1 `G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH` — add `min:N` to `select_count_capped_multi`, a trash-list→deck-bottom step verb, and an atomic cost-then-cancel guard in `lower_replacement.rs`; author EX5-015 Clause C; test.
- [x] 4.2 `G-DSL-UNION-PLAY-FREE` — widen `select_union_zone` to bind a zone-tagged index (HandIndex/TrashIndex) so `play_from_*_free` can consume it; author BT17-095 union-play clause; test.
- [x] 4.3 `G-IGNORE-COLOR-MASK` — add a from-hand installation path for "ignore THIS card's color requirement"; author P-206 color-bypass; test.
- [x] 4.4 `G-MULTI-SELECT-OPP-PLAY-COST-SUM` — add a `PlayCostBudget` selection kind mirroring `DpBudget`; (consumed by EX4-073 clause B arm 2 in 5.x); test.
- [x] 4.5 `G-OPT-MULTI-TIMING-SHARED-LOCKOUT` — add a shared OPT key so a multi-timing cluster shares one once-per-turn counter; author AD1-014 cluster; test.
- [x] 4.6 `G-OUTER-OPTIONAL-NOT-INSTALLED` — install an outer accept/decline `PendingSelection` for a lone optional triggered effect; covers BT22-084 + BT5-092; test.
- [x] 4.7 `G-PRED-NO-FACE-UP-SECURITY-NAMED` — add a DSL predicate filtering security cards by face-up state; author ST20-15 color-bypass gate; test.
- [x] 4.8 `G-SELECT-EMPTY-OUTER-TAIL` — fix `select_hand` empty-candidate path to drain the outer tail (port the `select_material` fix); re-enable AD1-009 optional-clause test.
- [x] 4.9 BT5-092 cost-reduction trigger — add a `when_*_digivolves_into` target-keyed trigger to `CostReductionBody`; author BT5-092 clause 2; test.

## 5. Phase C — Deep substrate gaps (3)

- [x] 5.1 `G-DSL-DNA-FROM-HAND-PARTNER` — new engine API + DSL step variant where one DNA material is a hand `CardHandle`; author BT17-095 Clause B DNA reward; test.
- [x] 5.2 `G-FOR-EACH-DELETE-INDEX-SHIFT` — make `ForEach` over deletion targets stable (reverse iteration or stable permanent IDs); author/fix BT8-097; test.
- [x] 5.3 AD1-012 defender-side effect-initiated DNA mid-attack-interrupt — support effect DNA during the attack interrupt with attack-flow resume; author AD1-012 Clause 4; test.

## 6. Phase C — Authoring-only card clauses (14, substrate already present)

- [x] 6.1 G-COLOR-MATCH-AGAINST-BOARD — add `color_matches_any_field_digimon` to P-206 Delay `select_hand` filter; tests.
- [x] 6.2 G-DSL-DISTINCT-TAMER-COLORS-FORMULA — author AD1-014 lock branch + ST20-11 immunity clause with `select_count_capped_multi` + `floor_div(distinct_colors_count,2)`; tests.
- [x] 6.3 G-DSL-SELECT-OWN-SOURCES-FILTER — author EX4-073 clause C with `select_own_sources.filter: { level_gte: 6 }` (uses 3.4); tests.
- [x] 6.4 G-DSL-SELF-NAME-CONTAINS — author AD1-014 inherited [When Attacking] clause with `source_name_contains`; tests.
- [x] 6.5 G-EVENT-CARD-TAMER-PLAY — author AD1-010 + EX9-012 test bodies (YAML observers already present); confirm pass.
- [x] 6.6 G-EVENT-TARGET-NOT-SOURCE — add `event_permanent_is_source: false` to EX4-039 inherited clause; test.
- [x] 6.7 G-OPT-TRIGGERED — author the 4 placeholder test bodies (BT17-081, BT20-102, EX4-039, EX4-061); confirm OPT lockout passes.
- [x] 6.8 G-PLAY-COST-GTE — add `play_cost_gte: 4` to BT22-089 clause-1 filter; re-enable test.
- [x] 6.9 G-PLAY-COST-LTE — author EX10-010 test body + add `play_cost_lte: 3` to P-206 inherited-security filters; tests.
- [x] 6.10 G-SECURITY-ZONE-AURA-SOURCE — author ST20-15 `kind: aura, scope: security` clause + 3 test bodies.
- [x] 6.11 G-ADD-OPTION-SELF-TO-HAND — author P-206 inherited-security test body (+ DebugRunner security-effect harness if needed).
- [x] 6.12 G-PLACE-SELF-AS-OPTION-PERMANENT — author P-206 Delay-activation clause + 3 test bodies (+ Delay-activation harness if needed).
- [x] 6.13 AD1-025 — write + register the `ad1_025_on_play_process` raw_rust fn (DRIFT 1) and author the omitted `[All Turns][OPT]` clause (DRIFT 2); 6 test bodies.
- [x] 6.14 BT16-082 — author the 2 OPT move-trigger test bodies; confirm pass.

## 7. Phase B — Missing-YAML card authoring (TDD, frequency-ordered)

- [x] 7.1 BT22-084 Nokia Shiramine (63 decks) — behavioral tests from printed text incl. Tamer security route; production YAML (depends on 4.6); confirm pass.
- [x] 7.2 BT17-007 Agumon (9 decks) — behavioral tests; YAML; confirm pass.
- [x] 7.3 ST2-13 Hammer Spark (4 decks) — behavioral tests; YAML (Option `[Main]` flow — land PARTIAL + file follow-up if blocked).
- [x] 7.4 BT5-093 Tai & Matt (2 decks) — behavioral tests; YAML; confirm pass.
- [x] 7.5 AD1-019 (empty placeholder) — minimal YAML + test.
- [x] 7.6 Update `validated_cards_dsl.json` for all five cards.

## 8. Phase D — raw_rust minimization

- [x] 8.1 Review each `raw_rust` escape in AD1-025, BT13-012, BT20-102, BT22-099, BT23-096, EX4-073, EX5-015, P-206 against current DSL vocabulary.
- [x] 8.2 Migrate now-expressible escapes to pure DSL; keep behavioral tests as regression guards.
- [x] 8.3 Document each retained escape with the reason the DSL cannot express it.

## 9. Phase E — Tracker reconciliation + verification

- [x] 9.1 Move every gap verified closed during the change to `qa/resolved-gaps.md` (dated section).
- [x] 9.2 Annotate `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md` with closure outcomes.
- [x] 9.3 Reconcile `qa/dsl-vocab-gaps.md` and `docs/RUST_ENGINE_GAPS.md` — no closed gap left marked open; record any newly-filed follow-ups.
- [x] 9.4 Confirm no DNA Omnimon test carries an `#[ignore]` citing an already-closed gap.
- [x] 9.5 Run full engine suite + `dsl_eval_arm_coverage` + `DIGIMON_BACKEND=rust` parity test; no regressions.
- [x] 9.6 Finalize `validated_cards_dsl.json`: every card has a verdict; every PARTIAL/BLOCKED names its open gap.
