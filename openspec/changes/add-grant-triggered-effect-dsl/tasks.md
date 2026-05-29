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
- [ ] 4.1a/4.1b (Q16/Q17 directions) `<Partition>` does NOT fire when a granted self-delete removes the carrier (granted deletion attributed `OwnEffect` to the carrier); an immune carrier drops the granted slot. DEFERRED — needs EX6-057 Lilithmon to exercise.

## 5. Author cards + pin judge-quiz scenarios — EX1-068/Q2 done; EX6-057/Q16/Q17 deferred

- [x] 5.1 Authored EX1-068 Ice Wall! `[Main]`: `grant_triggered_effect` (opponent Digimon, `timing: when_attacking`, `body: [lose_memory: 2]`, `expiry: end_of_opponents_next_turn`); `[Security]` clause kept.
- [x] 5.2 Un-ignored `judge_quiz::a_immunity_scope::q2_...` — PASSES (Medusamon loses no memory; non-Progress control loses 2). EX1-068's per-card behavioral tests updated (clause now present + shaped).
- [ ] 5.3 Author EX6-057 Lilithmon `[On Play]`/`[When Digivolving]` grant "[EoT] Delete this" — DEFERRED (card-authoring wave).
- [ ] 5.4 Un-ignore Q16 / Q17 — DEFERRED (depend on 5.3 + 4.1a/4.1b).

## 6. Reconcile and verify

- [x] 6.1 Moved the `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` Q2 slice from `qa/dsl-vocab-gaps.md` to `qa/resolved-gaps.md` (Q16/Q17 directions noted as still open).
- [x] 6.2 Updated `qa/qa-reports/judge-quiz.md`: Q2 → PASS (Q16/Q17 remain BLOCKED-CARD on EX6-057).
- [ ] 6.3 Full-suite green gate — run with the sibling changes before archiving (Q16/Q17 still open, so this change is not fully complete).
