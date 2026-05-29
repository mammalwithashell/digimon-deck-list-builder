## 1. Gap 2 — Digi-Egg routing on return-to-deck (smallest, isolated)

- [ ] 1.1 Confirm the failing test: `cargo test --test judge_quiz q22` shows `q22_digi_egg_returned_to_deck_bottom_routes_to_digitama_deck` failing (digitama empty)
- [ ] 1.2 Add a private `move_card_to_deck(player, card, position)` helper in `effect_context/mod.rs` (or `game.rs`) routing `CardKind::DigiEgg` → `digitama_deck` (bottom = `insert(0)`, top = `push`), everything else → `deck`
- [ ] 1.3 Re-point `return_trash_cards_to_deck_bottom` (mod.rs:5538) and `return_trash_cards_to_deck_top` (mod.rs:5570) through the helper; grep for any other trash→deck / bounce→deck movers and route them too
- [ ] 1.4 Confirm the `moved` Vec still counts the routed card (dependent costs unaffected)
- [ ] 1.5 Un-ignore the Q22 test; add a focused regression test for `_to_deck_top` with a Digi-Egg; both pass
- [ ] 1.6 Run `cargo test --test judge_quiz` + `--test deletion_batching` green

## 2. Gap 1 — general state-based ≤0-DP rules-check (highest impact)

- [ ] 2.1 Confirm the failing probe: `zero_dp_probe_reduced_digimon_deleted_after_effect_resolves` fails; `..._healthy_digimon_survives_drain` passes
- [ ] 2.2 Promote `run_rule_check_after_arts` (game_actions.rs:1701) to `Game::run_state_based_rules_check`: delete every battle-area Digimon with `effective_dp ≤ 0` via the batched deletion flow (`delete_permanents_batch`), looping to a fixpoint (re-check after each pass; bounded by battle-area size); idempotent on already-gone handles
- [ ] 2.3 Invoke it at the top-level resolution boundaries: end of `drain_effect_queue` once the queue is empty (effect_queue.rs:697), after combat DP changes resolve (combat.rs — AFTER combat's own loser deletions), and at phase transitions. Keep the existing Arts call site (now delegating to the general fn)
- [ ] 2.4 Guarantee it never runs mid-effect (only when the top-level resolution has finished) — verify against the deferred-timing cases
- [ ] 2.5 Un-ignore `zero_dp_probe_reduced_digimon_deleted_after_effect_resolves`; it passes
- [ ] 2.6 Add synthetic regression tests: (a) Q6-analog — Digimon at 0 DP mid-effect not deleted until the effect resolves; (b) Q24-analog — a suspended Digimon driven to ≤0 by an `[All Turns]` is deleted by the rules-check before a separate queued `[Your Turn]` trigger resolves (pins the D3 interleave)
- [ ] 2.7 Hot-path regression gate: full `combat`, `option_flow`, `deletion_batching` suites + an RL smoke run — no double-deletion, no perf regression, no zombie 0-DP permanents

## 3. Gap 3 — defer inherited on-trash triggers (spike-gated, riskiest)

- [ ] 3.1 **Calibration spike:** read EX10-036 + its behavioral test `ex10_036_clause_a_after_source_trash_prompts_opp_field_delete`; pin EXACTLY what its secondary clause needs from the synchronous drain in `fire_digivolution_card_trashed` (game_actions.rs:3308). Decide go/split.
- [ ] 3.2 If go: separate intra-effect observer consumption (stays synchronous) from inherited triggered effects on the trashed card (enqueue to the pending queue, resolve at the §2 resolution boundary with a remain-in-trash activation re-check)
- [ ] 3.3 Add the remain-in-trash activation gate to the inherited-on-trash resolution path
- [ ] 3.4 Add a synthetic regression test: 2 sources with an inherited "when trashed, gain memory" are trashed; one is removed from trash before resolution; only the remaining one's effect fires
- [ ] 3.5 Convert the `cluster_d_on_trash_observer_fires_synchronously_not_deferred` characterization into a deferral assertion (or replace with the deferred-behavior test); EX10-036 behavioral test stays green
- [ ] 3.6 If the spike shows the two needs can't be cleanly separated within this change's scope, split gap 3 into a follow-up change, leave Q23/Q21 BLOCKED on it, and record the decision here

## 4. Reconcile and verify

- [ ] 4.1 Move closed gaps (`G-RETURN-TRASH-DIGI-EGG-ROUTING`, `G-NO-GENERAL-ZERO-DP-RULES-CHECK`, and `G-ON-TRASH-OBSERVER-SYNCHRONOUS` if §3 lands) from `qa/archetype-qa/engine-gaps.md` to `qa/resolved-gaps.md` with resolution notes + test commands
- [ ] 4.2 Update `qa/qa-reports/judge-quiz.md` verdicts: Q22 → PASS; the cluster-B / Q23 entries note the engine fix landed (per-card scenarios flip to PASS as their cards are authored)
- [ ] 4.3 Confirm no judge-quiz test carries an `#[ignore]` for a gap this change closed
- [ ] 4.4 Run the full `cargo test --manifest-path code/digimon-engine/Cargo.toml` suite — green, no regressions
