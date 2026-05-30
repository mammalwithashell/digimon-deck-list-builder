<!-- SCOPE NOTE (2026-05-29): the apply spike found the `grant_triggered_effect`
     step + `ModifierType::GrantedTrigger` slot + dispatch ALREADY EXISTED (from
     the EX10-034 grant-to-binding work). The design assumed a from-scratch build;
     in fact the remaining work was narrow: the OPPONENT-targeting + `<Progress>`
     cause-attribution slice (Q2). Tasks below re-marked to reflect that.
     Q16/Q17 (Lilithmon EX6-057) need a NEW card + the OTHER attribution
     directions — DEFERRED to the card-authoring wave. -->

## 1. Spike — dispatch + cause-attribution shape — DONE (substrate pre-existed)

- [x] 1.1 Trigger-fire path mapped: granted slots are enumerated in `effect_queue.rs::enqueue_from_permanent` (battle) at the granted-entry loop, alongside printed triggers; `granted_triggered_for_timing_with_ids` is the query. The drainer executes `granted_effect_id` bodies (`run_queued_effect_inner`).
- [x] 1.2 Cause-attribution directions confirmed: Q2 (granted effect is the opponent's from the carrier's view → `progress_excludes(carrier, source_player)` covers it) — IMPLEMENTED. Q16 (granted self-delete = carrier's OwnEffect → `<Partition>` skips) — NOT yet wired (needs EX6-057; deferred).
- [x] 1.3 Confirmed: `ModifierType::GrantedTrigger` slot + expiry tick already exist; `end_of_opponents_next_turn` → `Expiry::EndOfOpponentsNextTurn` (expiry_map.rs) is the "until the end of their next turn" mapping EX1-068 uses.

## 2. Engine — granted-trigger modifier slot + dispatch — PRE-EXISTING + Q2 guard added

- [x] 2.1/2.2/2.3 The `ModifierType::GrantedTrigger`-equivalent slot (`GrantedTriggeredEffect`), the install API (`EffectContext::grant_triggered_effect`), and the queue dispatch already existed from EX10-034. NEW work (D4, Q2 direction): the dispatch now skips firing when the carrier is unaffected by the grantor's effects — `effect_queue.rs::enqueue_from_permanent` gates the granted-entry enqueue on `if self.progress_excludes(handle, Some(source_player)) { continue; }`.
- [x] 2.4 Covered by `judge_quiz::a_immunity_scope::q2_...` (fires WhenAttacking with `pending_attack` set; non-Progress control fires, Progress carrier is skipped); EX10-034 granted-effect regression green.

## 3. DSL — `grant_triggered_effect` step — PRE-EXISTING

- [x] 3.1–3.5 The `grant_triggered_effect` step (`target`/`timing`/`body`/`expiry`), its lowering (`dsl_cards/step/grant_triggered.rs`), and snapshot semantics already existed (EX10-034). A predicate `target` (`CompiledModifierTarget::Filter`) walks BOTH battle areas, so `of: opponent` targets opponent Digimon at grant time — no new DSL surface required for EX1-068.

## 4. Cause-attribution + immunity integration — Q2 done; Q16/Q17 deferred

- [x] 4.1c/4.2/4.3 (Q2 direction) `<Progress>` excludes a granted opponent effect on the attacker — implemented via the `progress_excludes` guard in the dispatch; pinned by Q2; `combat` + granted-attack + aura suites green.
- [x] 4.1a (Q16 direction) `<Partition>` does NOT fire when a granted self-delete removes the carrier — implemented by running the granted body with `effect_source_player = carrier.player` (D4/DCGO) at all three dispatch sites, so the deletion is the carrier-controller's OwnEffect and the existing Partition cause-filter skips it. Pinned by Q16. Two `group6_auras` mirror-tests corrected from `gain_memory(-2)` (controller-relative, assumed grantor) to `lose_memory(2)` to reflect the faithful carrier model.
- [x] 4.1b (Q17 direction) a carrier immune to the grantor's effects does not fire the granted clause — implemented by also gating the granted-trigger dispatch on `permanent_is_unaffected_by_effect(carrier, grantor, kind)` at all three sites. DCGO models Magnamon X's immunity as a continuous `CanNotAffectedClass` (affect-time check), matching this fire-time gate. Pinned by Q17 + the `granted_effect_suppressed_when_carrier_immune_to_grantor` synthetic test.

## 5. Author cards + pin judge-quiz scenarios — EX1-068/Q2 done; EX6-057/Q16/Q17 deferred

- [x] 5.1 Authored EX1-068 Ice Wall! `[Main]`: `grant_triggered_effect` (opponent Digimon, `timing: when_attacking`, `body: [lose_memory: 2]`, `expiry: end_of_opponents_next_turn`); `[Security]` clause kept.
- [x] 5.2 Un-ignored `judge_quiz::a_immunity_scope::q2_...` — PASSES (Medusamon loses no memory; non-Progress control loses 2). EX1-068's per-card behavioral tests updated (clause now present + shaped).
- [x] 5.3 Authored EX6-057 Lilithmon (`cards/ex6/EX6-057.yaml`): clause 1 `[OP][WD]` grant "[EoT] Delete this" to a selected opponent Digimon (until end of opponent turn); clause 2 `[All Turns][OPT]` `when_would_leave` cost-replacement (delete a Lv5- Digimon to cancel the leave, not by battle); clause 3 `[Opp Turn][OPT]` on-deletion trash opponent's top security. AND authored BT16-102 Magnamon (X Antibody) (`cards/bt16/BT16-102.yaml`): `<Blocker>`, `<Armor Purge>`, `[When Digivolving]` conditional opponent-effect immunity + +3000 DP + unsuspend, `[All Turns][OPT]` re-activate `[When Digivolving]` on security removal. Per-card tests in `tests/cards_behavioral/ex6/ex6_057.rs` (2) + `bt16/bt16_102.rs` (2). BT21-036 Magnamon NOT authored — its only Q17 role was an Armor-Form digivolution source (staged synthetically); blocks no judge-quiz question.
- [x] 5.4 Q16 (`e_partition_digixros::q16_...`) and Q17 (`a_immunity_scope::q17_...`) un-ignored and PASS.

## 6. Reconcile and verify

- [x] 6.1 Moved the full `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` gap (all three directions Q2/Q16/Q17) from `qa/dsl-vocab-gaps.md` to `qa/resolved-gaps.md`.
- [x] 6.2 Updated `qa/qa-reports/judge-quiz.md` + `card-resolution.md`: Q2/Q16/Q17 → PASS.
- [x] 6.3 Full-suite green gate: `cargo test --features dsl-yaml-loader --no-fail-fast` shows only the 17 PRE-EXISTING failures (confirmed on origin/main); the `dsl`, `judge_quiz`, and granted-effect suites pass. Change is complete (Q2/Q16/Q17 pinned; BT21-036 not needed).
