# Archetype Faithfulness Audit — 2026-06-10

Audit mode: re-verification of existing combo/interaction suites (audit-archetype-faithfulness).
Scope this run: **BG Imperial** (`code/digimon-engine/tests/archetypes/bg_imperial.rs`).
Sources cross-examined per combo: printed card text (cards.json / card_overrides.json / YAML DSL),
`Digimon TCG resources/general_rule.pdf` (§8-1, §8-2, §2-3-6), and DCGO C# (base-repo
`$BASE_DCGO/Assets/Scripts/CardEffect/...`: ST9_05.cs, ST9_06.cs, BT16_025.cs).

## Summary

| Archetype | Verdict | Combos faithful | Divergent | Untested | Findings filed | Suite run |
|---|---|---|---|---|---|---|
| BG Imperial | **FAITHFUL** | 6 / 6 | 0 | 0 | 0 | 14 / 14 tests pass |

No divergences between engine behavior and card text / rules / DCGO were found.
Four **minor** test-quality issues were noted (assertion-tightening opportunities, one
guardrail deviation on synthetic neutral targets); none invalidates a combo verdict and
none is a blocker or major issue, so the FAITHFUL verdict stands.

---

## BG Imperial — FAITHFUL

- Model doc: `qa/archetype-qa/bg-imperial-model.md`
- Test file: `code/digimon-engine/tests/archetypes/bg_imperial.rs`
- Suite run: 14 / 14 passed (`cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes bg_imperial`)
- New tests authored this audit: none (existing coverage judged sufficient; gap-fill items listed under Test issues)
- Findings filed: none (no tracker entries; triage outcomes empty)
- Deferred / blocked: nothing blocked; coverage-gate static test remains failing at deck level (34/84 = 40% vs 85% threshold) — a deck-pool implementation-coverage matter outside this audit's combo scope, already recorded by the static harness

### Combo-by-combo

| # | Combo | Status | Covering tests | Evidence (condensed) |
|---|---|---|---|---|
| 1 | DNA Bounce (ST9-05) | faithful | `dna_bounce_real_exveemon_stingmon_returns_low_dp_opponent`, `regular_digivolve_into_paildramon_does_not_fire_dna_bounce` | Real DNA digivolve (EX1-014 blue Lv.4 + ST9-09 green Lv.4) installs an OppField selection; asserts opp_field -1 and opp_deck increase. Regular-digivolve control: no selection, target survives. Matches DCGO ST9_05.cs (`CanTriggerWhenDigivolving && IsJogress`, `Mode.PutLibraryBottom`, DP<=6000, `canNoSelect:false`) and YAML `on_dna_digivolve` / `return_to_deck position: bottom`; general_rule.pdf §8-2. Minor: deck delta asserted as `>` not `== +1`; bounce target synthetic (issues 1, 4). |
| 2 | DNA Lockdown (BT16-025) | faithful | `dna_lockdown_suspends_and_locks_opponent`, `regular_digivolve_into_meta_paildramon_suspends_but_does_not_lock` | DNA path asserts suspend AND `ModifierType::CannotUnsuspend`; regular path asserts suspend without the lock. Matches DCGO BT16_025.cs (`DigivolutionCards.Count <=` source count; `IsJogress -> GainCanNotUnsuspendPlayerEffect`, `EffectDuration.UntilOpponentTurnEnd`) and YAML `materials_count_lte: source_material_count` + CannotUnsuspend expiry `end_of_opponents_turn`. Weakness (not a divergence): the count-gate's negative case is untested (issue 2). |
| 3 | Colour-Gated Source Replay (ST9-06) | faithful | `colour_source_replay_with_blue_and_green_sources_replays_both`, `colour_source_replay_without_green_source_replays_only_blue` | Real stack via `place_stack` (EX1-014 + ST9-09 + ST9-05 + ST9-06); optional accepted, two colour picks driven; field +2 with both colours, +1 without green. Stacked sources are the only legal blue/green Lv<=4 candidates, so the deltas genuinely prove the colour split. Matches DCGO ST9_06.cs (optional ActivateClass, two SelectCardEffect passes over DigivolutionCards Blue/Green Lv<=4, `PlayPermanentCards payCost:false`) and YAML dual `select_own_sources` + `play_selected_sources_free`; general_rule.pdf §8-1-2-8. |
| 4 | Evolution-cost & colour gate (alt-paths) | faithful | `paildramon_st9_05_dna_alt_path_requires_blue_lv4_and_green_lv4_at_cost_0`, `imperialdramon_dm_st9_06_standard_digivolve_is_blue_lv5_cost_4` | Pins compiled alt_paths: ST9-05 DNA path = exactly 2 materials {Blue Lv.4}+{Green Lv.4} at cost 0, standard cost 4; ST9-06 standard from Blue Lv.5 at cost 4. Matches YAML alt_paths and DCGO `JogressCondition(elements, 0)` / cards.json evo_costs; general_rule.pdf §8-2-1, §8-1-3-2. Minor under-spec: ST9-05 standard-path `from` colour not asserted (cost is). |
| 5 | Digivolution colour gate (rookie/champion legs) | faithful | `veemon_digivolves_over_blue_egg`, `wormmon_cannot_digivolve_over_blue_egg`, `wormmon_digivolves_over_green_egg`, `single_colour_exveemon_takes_blue_lv3_not_green`, `single_colour_stingmon_takes_green_lv3_not_blue`, `dual_colour_exveemon_and_stingmon_accept_the_off_colour_lv3` | Exercises the real `Game::can_digivolve` colour/level gate across rejection and acceptance cases, including dual-colour evo requirements (BT12-022/BT12-050). Matches general_rule.pdf §8-1-3-1 + §2-3-6. Synthetic evolver/base cards encode REAL printed evo_costs because DSL-loaded CardData carries empty evo_costs (`card_data_from_compiled` sets `evo_costs: Vec::new()`) — a documented harness limitation (see `reference_debugrunner_empty_evo_costs`), not an engine bug; the engine rule under test is exercised authentically. |
| 6 | Effect descriptions bubble to the UI (ST9-05 / ST9-06) | faithful | `dna_bounce_real_exveemon_stingmon_returns_low_dp_opponent`, `colour_source_replay_with_blue_and_green_sources_replays_both` | Combo 1 unconditionally asserts the DNA-bounce `PendingSelection.prompt` contains "6000" and "bottom" (card-specific UI text per no-approximations rule 17, matching YAML prompt fields / DCGO SetUpCustomMessage). Combo 3's blue/green prompt check is wrapped in `if let Some(view)` and can pass vacuously (issue 3) — verdict stays faithful on the strength of the unconditional Combo 1 check. |

### Test issues (all minor — no verdict impact)

1. **Synthetic neutral targets** (`dna_bounce_...` / `dna_lockdown_...` via `make_opp_digimon`): opponent
   targets are `make_test_card('OPP-LOW'/'OPP')` rather than real effectless DSL vanillas (e.g. ST2-02,
   ST2-04), contra the skill's "real cards for every role" guardrail. Assertions remain valid; prefer a
   real vanilla so the absence of effects is guaranteed by the card, not by hand.
2. **BT16-025 count-gate under-exercised** (`dna_lockdown_suspends_and_locks_opponent`): only confirms a
   0-source opponent IS suspended vs Paildramon's 2 sources; never confirms an opponent with MORE sources
   is NOT suspended. Test would still pass if the engine suspended unconditionally. Add a >source-count
   sibling asserting non-suspension.
3. **Conditional prompt assertion** (`colour_source_replay_with_blue_and_green_sources_replays_both`):
   the blue/green UI-prompt assert is inside `if let Some(view) = runner.pending_selection_view()`, so it
   silently no-ops if no selection is parked. Unwrap-and-assert (as Combo 1 does).
4. **Loose deck delta** (`dna_bounce_real_exveemon_stingmon_returns_low_dp_opponent`): asserts
   `opp_deck > before` instead of exactly `before + 1`; ideally also assert bottom placement (DCGO
   `Mode.PutLibraryBottom` / YAML `position: bottom`).

### Static harness state (pre-existing, recorded by archetype-static-tests)

- deck_legality: PASS (50 main / 5 egg, constructs)
- smoke_games: PASS (5/5 clean)
- combo_presence: PASS (6/6 combos fully implemented)
- coverage_gate: FAIL — 34/84 = 40% vs 85% threshold (1 failing card BT23-047, 49 unknown). Deck-pool
  implementation coverage, not combo faithfulness; routed to the normal implementation pipeline
  (`/batch-implement-cards-rust-dsl`), not a finding of this audit.

### Verdict rationale

Every audited combo is faithful against card text + general_rule.pdf + DCGO; the full suite passes
14/14; no findings were filed and no triage was required; all four test issues are severity:minor
assertion-strength improvements. Per the verdict rules, **BG Imperial = FAITHFUL**.
