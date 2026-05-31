## Why

The judge-quiz faithfulness discovery wave (`add-judge-quiz-faithfulness-suite`) surfaced three engine-level gaps, two proven by failing tests and one by a characterizing probe + code reading. All three are real rules-correctness issues that reach far beyond the quiz scenarios that exposed them:

- **No general state-based ≤0-DP rules-check (`G-NO-GENERAL-ZERO-DP-RULES-CHECK`).** The only ≤0-DP deletion site in the engine is `run_rule_check_after_arts`, invoked from exactly one place — the Arts-digivolve flow. `add_dp_modifier`/`add_modifier` store the modifier without a deletion check, and `drain_effect_queue` runs no sweep. So a Digimon reduced to ≤0 DP by any non-Arts effect (DP-minus is ubiquitous) is **never deleted**. Proven: `zero_dp_probe_reduced_digimon_deleted_after_effect_resolves` (a synthetic Digimon at −1000 DP survives the post-effect drain). **Highest impact** — a systemic gameplay-correctness bug.
- **Digi-Egg routing ignored on return-to-deck (`G-RETURN-TRASH-DIGI-EGG-ROUTING`).** `return_trash_cards_to_deck_bottom` unconditionally inserts every returned card into the main deck; a `CardKind::DigiEgg` returned this way lands in the main deck instead of the Digi-Egg (digitama) deck — an illegal state. Proven: `q22_digi_egg_returned_to_deck_bottom_routes_to_digitama_deck` (digitama deck stays empty).
- **On-trash inherited effects fire synchronously, blocking remain-in-trash gating (`G-ON-TRASH-OBSERVER-SYNCHRONOUS`).** `fire_digivolution_card_trashed` enqueues the trigger then immediately drains — intentionally synchronous so EX10-036's secondary clauses see just-trashed cards. But that prevents inherited on-trash triggers (e.g. Tumblemon's "gain 1 memory") from deferring and re-checking trash-presence, so they over-count when a later effect removes the card first. Characterized: `cluster_d_on_trash_observer_fires_synchronously_not_deferred`.

These gaps currently block judge-quiz Q6/Q8/Q13/Q14/Q24 (gap 1), Q22 (gap 2), and Q23/Q21 (gap 3) at the engine level — independent of card authoring. Fixing them turns those scenarios from BLOCKED to pinnable and removes real correctness bugs from live play and RL training.

## What Changes

- **Gap 1 — generalize the state-based rules-check.** Rename/promote `run_rule_check_after_arts` into a general `Game::run_state_based_rules_check` that deletes every battle-area Digimon at `effective_dp ≤ 0`, and invoke it at the DCGO-parity points: after each effect resolution (post `drain_effect_queue`), after combat DP changes, and at phase boundaries — never mid-effect (the judge rule: rule checks don't run until an ongoing effect or rule action finishes). Sequence so the deferred-timing scenarios (Q13/Q14: ShoeShoemon not deleted until Nyabootmon's `[When Digivolving]` fully resolves; Q24: Tentomon deleted by the rules-check before Kokomon's `[Your Turn]` trigger contributes) resolve correctly.
- **Gap 2 — route Digi-Eggs on return-to-deck.** Branch `return_trash_cards_to_deck_bottom` (and audit the sibling `return_trash_cards_to_deck_top` and any other "to deck" movers) on `CardKind::DigiEgg` → `digitama_deck` (bottom = index 0). The `moved` list still counts the card so dependent costs (Medusamon's "return 2") stay satisfied.
- **Gap 3 — defer inherited on-trash triggers while preserving EX10-036's synchronous intra-effect observers.** Distinguish (a) intra-effect observers a secondary clause of the SAME resolving effect consumes (stay synchronous) from (b) inherited/printed triggered effects that go to the pending queue and resolve AFTER the current effect with a zone-presence (remain-in-trash) re-check. Verify EX10-036's exact dependency before changing dispatch.
- **Tests:** flip each gap's `#[ignore]`-d judge-quiz test to a real pass; add focused engine regression tests; keep `EX10-036` and the existing combat/security suites green.

## Capabilities

### New Capabilities
- `judge-quiz-engine-gap-fixes`: The three engine corrections above — a general state-based ≤0-DP rules-check invoked after effect resolution / combat / phase boundaries (not mid-effect); Digi-Egg routing to the digitama deck on return-to-deck movements; and deferral + remain-in-trash re-check for inherited on-trash triggered effects while preserving synchronous intra-effect observer firing.

### Modified Capabilities
<!-- Behaviors here are NEW (the checks/routing/deferral do not exist today), so they are ADDED under the new capability. If the spike finds an existing capability (e.g. permanent-deletion-semantics, zombie-permanent-cleanup) whose requirements must change rather than extend, a MODIFIED delta to that capability's spec will be added at that time. -->

## Impact

- **Engine (Rust):** `code/digimon-engine/src/game_actions.rs` (generalize `run_rule_check_after_arts`; new invocation sites), `code/digimon-engine/src/effect_context/mod.rs` (`return_trash_cards_to_deck_bottom`/`_top` Digi-Egg routing; on-trash deferral), `code/digimon-engine/src/game_actions.rs::fire_digivolution_card_trashed` + `effect_queue.rs` (defer vs synchronous), `code/digimon-engine/src/combat.rs` (post-combat rules-check site).
- **Tests:** `code/digimon-engine/tests/judge_quiz/*` (un-ignore Q22 + the cluster-B/Q23 probes; add Q6/Q8/Q13/Q14/Q24 once cards exist), plus focused regression tests under `tests/deletion_batching/`, `tests/effect_context/`, `tests/selection/`. Hot-path gates: `combat`, `option_flow`, and the EX10-036 behavioral test.
- **Trackers:** move closed gaps from `qa/archetype-qa/engine-gaps.md` to `qa/resolved-gaps.md`; update `qa/qa-reports/judge-quiz.md` verdicts.
- **No RL contract change** expected (no action-space/tensor change). The ≤0-DP rules-check changes WHEN existing deletions happen, not the action surface; if any new pending-selection arises it is handled additively per the existing contract.
- **Risk:** gap 1 changes a hot path (every effect/combat resolution gains a sweep) and gap 3 touches the trigger-dispatch shared by EX10-036 — both need careful sequencing and the existing suites as regression gates.

## Non-Goals

- **AD1-025 `[Assembly]` data-ingest gap (judge-quiz Q5).** That is a `data/cards.json` / `card_overrides.json` correction (the `[Assembly]` keyword is missing from card data) plus YAML alt-path authoring — a data/card-content change, not an engine fix. Separate change.
- **`G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` (judge-quiz Q2/Q16/Q17).** A large, pre-existing DSL-vocabulary feature (new `ModifierType::GrantedTrigger` + `CompiledStep::GrantTriggeredEffect` + lowering), tracked since 2026-05-03 in `qa/dsl-vocab-gaps.md`. Its own change.
- Authoring the 52 BLOCKED-CARD judge-quiz scenarios — that is `/batch-implement-cards-rust-dsl` work.
