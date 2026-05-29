## 1. Spike — dispatch + cause-attribution shape

- [ ] 1.1 Read the trigger-fire path (`enqueue_from_permanent` / `effect_queue.rs`) and pin the hook point where a permanent's granted-trigger slots are enumerated alongside its printed triggers (ordering, `[Once Per Turn]` accounting)
- [ ] 1.2 Confirm both cause-attribution directions hold for "controller = carrier": Q2 (granted effect is the opponent's from the attacker's view → `progress_excludes` covers it) AND Q16 (granted self-delete is the carrier's OwnEffect → `<Partition>` cause-filter skips it). Write throwaway probes if needed
- [ ] 1.3 Confirm the existing `Expiry` tick drains a `GrantedTrigger`-bearing `ModifierEntry`; identify the `EndOfOpponentsTurn` expiry mapping for "until the end of their next turn"

## 2. Engine — granted-trigger modifier slot + dispatch

- [ ] 2.1 Write a failing engine unit test: install a `GrantedTrigger` slot (e.g. `WhenAttacking` → lose 2 memory) on a permanent; firing that timing resolves the body against the carrier; the slot expires on the declared boundary
- [ ] 2.2 Add `ModifierType::GrantedTrigger` + `ModifierPayload::GrantedTrigger { clause: CompiledTriggeredClause }` (`enums.rs`, `modifiers.rs`)
- [ ] 2.3 Wire dispatch (per 1.1) to enumerate + enqueue granted-clause bodies when a matching timing fires on the carrier, with controller/cause = carrier (D4)
- [ ] 2.4 Confirm tests pass; add expiry + no-target + multi-grant unit coverage

## 3. DSL — `grant_triggered_effect` step

- [ ] 3.1 Write a failing DSL/behavioral test for a `grant_triggered_effect` step (target selector + `when` + inline `process` + `expiry`) installing the slot on the snapshot target set
- [ ] 3.2 Add the step shape (`step.rs`), compile to a payload carrying an inline `CompiledTriggeredClause` (`compile.rs`, `compiled.rs`)
- [ ] 3.3 Lower against the GRANTED permanent (not the source), installing the `GrantedTrigger` modifier on each snapshot target with the declared expiry
- [ ] 3.4 Confirm the target set is snapshotted at grant time (later-played Digimon don't carry it — DCGO parity); add the negative test
- [ ] 3.5 Run `cargo test --test dsl` green

## 4. Cause-attribution + immunity integration

- [ ] 4.1 Write failing tests: (a) `<Partition>` does NOT fire when a granted self-delete removes the carrier (Q16 rule); (b) a carrier that becomes immune to opponent effects loses the granted slot (Q17 rule); (c) `<Progress>` excludes a granted opponent effect on the attacker (Q2 rule)
- [ ] 4.2 Implement the attribution so granted-effect deletions read `ReplacementCause::OwnEffect` for the carrier; ensure `permanent_is_unaffected_by_effect` suppresses/removes the granted slot under immunity
- [ ] 4.3 Confirm all three pass; run `combat` suite (Partition lives there)

## 5. Author cards + pin judge-quiz scenarios

- [ ] 5.1 Author EX1-068 Ice Wall! `[Main]`: `grant_triggered_effect` (opponent Digimon, `when_attacking`, `lose_memory: 2`, `expiry: end_of_opponents_turn`); keep the `[Security]` clause
- [ ] 5.2 Un-ignore `judge_quiz::a_immunity_scope::q2_...`; it passes (Medusamon loses no memory). Add EX1-068's per-card behavioral test
- [ ] 5.3 Author EX6-057 Lilithmon `[On Play]`/`[When Digivolving]`: grant "[End of Your Turn] Delete this Digimon" to 1 opponent Digimon (+ its other printed clauses); add the per-card behavioral test
- [ ] 5.4 Un-ignore the Q16 (`<Partition>` not triggered) and Q17 (immunity removes granted delete) judge-quiz tests; both pass

## 6. Reconcile and verify

- [ ] 6.1 Move `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` from `qa/dsl-vocab-gaps.md` to `qa/resolved-gaps.md` with resolution note + test commands
- [ ] 6.2 Update `qa/qa-reports/judge-quiz.md`: Q2/Q16/Q17 → PASS
- [ ] 6.3 Run the full `cargo test --manifest-path code/digimon-engine/Cargo.toml` suite — green, no regressions (incl. `combat`, `option_flow`, `dsl`)
