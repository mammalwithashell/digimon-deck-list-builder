# Mechanics gap digest: rules-conformance campaign, Plan 1 (2026-10)

**Campaign:** engine rules-conformance validation, Plan 1 (foundation + mechanics suite), Tasks 1-14.
**Date:** 2026-10-16. **Manual:** Comprehensive Rules Ver. 4.3 (2026-09-18), vendored at `qa/rules/aegis/comprehensive-4.3.md`.
**Design:** `docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md` (§5b defines the mechanics set and what "blocks training" means: the divergence can change a game outcome or put an illegal choice in, or a legal choice out of, the action space).
**Purpose:** the input to Plan 3 (fixes), and the batched owner-rulings worksheet for the cases the PDF does not settle (design §6).

## Denominator

From the regenerated `qa/rules/cr_obligations.json` (1,119 obligations, Ver. 4.3):

| Scope | Obligations | Proven | Not-testable | Gap (blocking) | Gap (non-blocking) | Gap total |
|---|---|---|---|---|---|---|
| All chapters | 1,119 | 201 | 246 | 63 | 609 | 672 |
| Mechanics set (§5b) | 715 | 182 | 152 | 51 | 330 | 381 |

"Gap (non-blocking)" is mostly the untriaged default: an id absent from the sidecar is `gap`, `blocks_training: false`. Only 3 non-blocking gaps carry a test and a gap id (listed below). The 12 blocking obligations outside the mechanics set (§4-6-6-2, §4-9-5, §4-19-5 ×3, §18-1-2, §18-3-x ×6) still block training on their own merits; they are tagged `terminology-and-identity` / `other-information` in Aegis's taxonomy.

Regenerate and check:

```bash
PYTHONPATH=code python -m tools.rules.build_obligations          # writes qa/rules/cr_obligations.json from the sidecar
PYTHONPATH=code python -m tools.rules.build_obligations --check
PYTHONPATH=code python -m tools.rules.coverage_report            # the per-chapter table below
PYTHONPATH=code python -m tools.rules.coverage_report --check
cargo test --manifest-path code/digimon-engine/Cargo.toml --test rules_conformance -- --include-ignored   # every red test
```

Statuses live in `qa/rules/cr_obligations_status.yaml` (hand-maintained sidecar); gap entries in `docs/RUST_ENGINE_GAPS.md` (section "Findings from the 2026-10-15 rules-conformance investigation", plus the older entries named below).

### Per chapter

| Ch | Chapter | Total | Proven | Not-testable | Gap (blocking) | Gap |
|---|---|---|---|---|---|---|
| 1 | Game Overview | 55 | 0 | 11 | 0 | 44 |
| 2 | Card Information | 115 | 0 | 30 | 0 | 85 |
| 3 | Game Areas | 66 | 0 | 14 | 0 | 52 |
| 4 | Basic Terminology | 139 | 15 | 34 | 5 | 90 |
| 5 | Game Preparation | 14 | 0 | 2 | 0 | 12 |
| 6 | Game Procedures | 45 | 1 | 10 | 0 | 34 |
| 7 | Playing a Card | 49 | 0 | 6 | 0 | 43 |
| 8 | Digivolution | 55 | 0 | 8 | 0 | 47 |
| 9 | Using Cards | 13 | 1 | 1 | 1 | 11 |
| 10 | Link | 10 | 0 | 2 | 0 | 8 |
| 11 | Attacking | 40 | 2 | 8 | 5 | 30 |
| 12 | Blocking | 9 | 4 | 1 | 0 | 4 |
| 13 | Security Checks | 18 | 7 | 3 | 0 | 8 |
| 14 | Battles | 10 | 7 | 0 | 0 | 3 |
| 15 | Effect Rules | 244 | 27 | 66 | 7 | 151 |
| 16 | Keyword Effects | 206 | 128 | 44 | 32 | 34 |
| 17 | Rule Checks | 16 | 5 | 3 | 6 | 8 |
| 18 | Other Information | 15 | 4 | 3 | 7 | 8 |
| **Total** | | **1,119** | **201** | **246** | **63** | **672** |

"Gap" includes "Gap (blocking)". Plan 1 covered chapters 11-18 (mechanics-first, design §0); chapters 1-10 were touched only where a mechanic depended on them (§4 DUAL / Link / Overflow, §9-1-5). Chapters 6-9 and 1-5/10 are deferred to a follow-up plan.

## Blocking gaps

**29 gap ids** blocked training at the audit (63 obligations); after the Plan 4 fixes through Task 22, **20** remain (42 obligations; `qa/rules/cr_obligations.json`, 2026-10-17). **All 29 are `PDF-explicit`**: the 4.3 text (with, where noted, an official Q&A) settles the expected behaviour, so Plan 3 fixes them without asking. **None is `PDF-silent`.** Six explicit gaps carry a worksheet question that does not hold up their fix: Vortex target (W2), the end-of-turn attack keywords (W3), DUAL (W5 and W11), infinite loops (W10), Overflow (W11) and Link-at-limit (W12).

**Update 2026-10-16 (Plan 4 Task 24):** two blocking ids were added, `G-ENGINE-PROGRESS-DOES-NOT-HIDE-EARLIER-OPPONENT-EFFECTS` (§16-39-1) and `G-ENGINE-TOKEN-EFFECT-SOURCE-KIND-IS-RULE` (§15-2-1-1), both `PDF-explicit`; `G-ENGINE-UNAFFECTED-SKIPS-INSTEAD-OF-SUPPRESSES` (§15-15-5-1/-2/-4) was found and resolved in the same task (see "Retracted or resolved"). With the Plan 1 ids already resolved by earlier Plan 4 tasks, the regenerated ledger on this branch has **24 open blocking gap ids (51 obligations)**; the 29/63 above is the Plan 1 baseline.

Grouped by the mechanism the defect lives in; when the obligation's Aegis taxonomy tag differs, it is given after the rule id. The tests are those the sidecar lists for the gap's obligations: the red ones are `#[ignore]`d under the gap id and fail with `--include-ignored`; the green ones (other halves of the same obligations) are marked "(green)". "DCGO" and "Aegis" give each reference engine's stance on the rule as the gap entry and Aegis's vendored scenario index (`qa/rules/aegis/conformance-scenario-index.md`) / engine audits (`docs/audits/engine/*.md` at Aegis `ec0cd22f0`) record it.

### effects-and-ordering

**G-ENGINE-TOKEN-EFFECT-SOURCE-KIND-IS-RULE**: `PDF-explicit` — filed 2026-10-16 (Plan 4 Task 24 fix round 1)
- Rules: §15-2-1-1.
- Test: `ch15_15_unaffected::digimon_only_immunity_blocks_a_digimon_tokens_effect`.
- Cause: token effects are classified `EffectSourceKind::Rule` at four `source_kind_for_card_kind` sites, so a Digimon-effect-only immunity does not block them.
- DCGO: agrees with the rule (`AutoProcessing.cs` marks token effects `IsDigimonEffect`).
- Fix: map `CardKind::Token` to `Digimon`; handle the tensor `source_kind_bucket` shift, the opponent-Digimon-effect attributions, and the five `source_kind: rule` immunity grants (BT13-088, BT16-102, EX11-046, EX13-062, EX6-011) in the same change.

**G-ENGINE-ALLIANCE-OUTSIDE-TRIGGER-POOL**: `PDF-explicit`
- Rules: §15-4-3-2; §16-24-2 (keywords, <Alliance>); §16-23-2 (keywords, <Raid>).
- Tests: `ch15_04_effect_states::when_attacking_and_alliance_share_one_pool`, `ch16_attack_keywords::alliance_kind_matches_rule`, `ch16_attack_keywords::raid_kind_matches_rule`, `ch16_attack_keywords::raid_timing_matches_rule` (green).
- Cause: <Alliance> and <Raid> are attack-state windows (`AllianceOpen`, `RaidOpen`) reached only after the `[When Attacking]` drain, not trigger-type entries in the same pool, so the turn player can never order them before a `[When Attacking]`.
- DCGO: agrees with the rule (both are `ActivateClass`es stacked in the one `OnAllyAttack` batch).
- Aegis: agrees (`interaction-trigger-timing.test.ts`: "resolves EX6 Shoutmon's Alliance and When Attacking before MetalEtemon…"; `raid-lifecycle.md`: Raid "triggers from a Digimon attack").
- Fix: enqueue <Alliance>/<Raid> as trigger-type `QueuedEffect`s at the attack declaration, in the `[When Attacking]` batch; retire the separate windows (keep the ally pick / switch pick as the body). Lands with the F4 single-pool work.

**G-ENGINE-WHEN-ATTACKING-PARK-DROPS-ALLY-ATTACK-OBSERVERS**: `PDF-explicit`
- Rules: §15-4-3-5-1, §15-4-3-5-2.
- Test: `ch15_04_effect_states::turn_player_orders_first_then_non_turn_player`.
- Cause: `fire_on_attack` returns as soon as the attacker's `[When Attacking]` drain parks, so the `OnAllyAttack` / `OnOpponentAttack` fan-outs are never enqueued and those observers are lost for the attack.
- DCGO: agrees (one `StackSkillInfos(..., OnAllyAttack)` batch).
- Aegis: agrees (`ch15-02`: "ALL of the turn player's simultaneously-triggered effects resolve before ANY of the non-turn player's"; `trigger-ordering.md`: "When Attacking with opponent attack watchers").
- Fix: enqueue OnAttack / WhenAttacking / OnSuspend / OnAllyAttack / OnOpponentAttack before the first drain, as one bundle.

**G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS**: `PDF-explicit` (owner rulings 2026-10-10 #1 and #5 already decided it)
- Rules: §15-8-3-2; §16-12-7 (keywords, with §15-4-2-4).
- Tests: `ch15_08_effect_categories::trigger_type_waits_for_causing_effect_to_finish`, `ch16_processing_keywords::de_digivolve_separate_instances_processed_separately`.
- Cause: `fire_digivolution_card_trashed` drains the queue inline, so when the trasher is itself a queued trigger the observer resolves before the trasher finishes; inside an Option body (deferred) two separate trash events coalesce into one trigger.
- DCGO: agrees (stacks the trigger, resolves it after the trashing effect).
- Aegis: agrees (`trigger-ordering.md`: "Checks remain deferred while an Option or effect body is still processing").
- Fix: route source-trash observers through the normal pending queue (PR #726's `maybe_drain_effect_queue`), then fix #726's rule-4 (once per condition) and rule-5 (again per event) misses; re-derive EX10-036 from DCGO + oracle.

**G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED**: `PDF-explicit` (Ver. 4.3 reversed 3.6; owner ruling 2026-10-10 #6)
- Rules: §15-7-4, §15-8-4-4-1.
- Tests: `ch15_07_optional_conditions::unpayable_optional_condition_cannot_be_chosen`, `ch15_08_effect_categories::activation_type_declarable_only_when_optional_condition_payable` (green).
- Cause: a "by X, Y" clause is `optional: true` with X as leading steps; nothing marks X as the condition, so neither the trigger prompt nor the `[Main]` mask checks that X can be executed, and accepting runs Y for free.
- DCGO: agrees (per-card `CanActivateCondition`).
- Aegis: agrees (`tamer-suspend-cost-consent`: "does not offer the unpayable triggered clause"; `keyword-detach-lifecycle`: "does not offer the processing choice with unpayable links"; "does not declare an impossible activation-type compound payment").
- Fix: per-step-kind payability guards for a leading cost step (`trash_top_security`, memory, `suspend`, …) ANDed into `outer_optional_guard`, `needs_pre_cost_prompt` and the `[Main]` mask; long term the DSL `by:` block (`G-DSL-OPTIONAL-PROCESSING-CONDITION-BLOCK`). Then retire per-card hand-authored gates.

**G-ENGINE-REFIRED-EFFECT-SKIPS-OPTIONAL-DECISION**: `PDF-explicit`
- Rule: §15-15-7-4.
- Test: `ch15_07_optional_conditions::mandatory_invoker_does_not_force_invoked_optional_condition`.
- Cause: `run_refired_effect` calls `run_queued_effect` directly, bypassing the drain's pre-cost / outer-optional prompt, so a borrowed effect's optional condition is executed without asking.
- DCGO: agrees (`Activate_Optional_Effect_Execute` shows the borrowed effect's yes/no).
- Aegis: agrees ("lets the player decline BT23-045's borrowed optional placement cost").
- Fix: factor the drain's single-trigger optional gate into a helper that `run_refired_effect` also calls (with the payability guard above).

**G-ENGINE-PENDING-PROCESSING-OUTSIDE-TRIGGER-POOL**: `PDF-explicit`
- Rule: §18-1-2 (other-information).
- Test: `ch18_pending_overwrite::pending_processing_orders_with_simultaneous_triggers`.
- Cause: pending processing is a `ScheduledEffect` run by `fire_scheduled_for_timing` after the timing's printed triggers have drained, with no prompt, instead of joining their simultaneous pool.
- DCGO: agrees (player effects are stacked in the same `StackSkillInfos(null, OnEndTurn)` batch).
- Aegis: none found.
- Fix: materialise the timing's `ScheduledEffect`s as `QueuedEffect`s in the same batch as the printed triggers, before the drain, at every `fire_scheduled_for_timing` site.

**G-ENGINE-INFINITE-LOOP-SILENTLY-CLEARED**: `PDF-explicit` (representation question W10)
- Rules: §18-3-1, §18-3-2, §18-3-3, §18-3-3-1, §18-3-3-2, §18-3-3-3 (other-information).
- Tests: `ch18_pending_overwrite::infinite_loop_is_stopped_not_silently_cleared`, `ch18_pending_overwrite::stoppable_loop_repeats_until_its_owner_stops_it` (none for §18-3-3-1/-2).
- Cause: `MAX_CHAIN_DEPTH = 50` clears the whole effect queue silently (no event, no draw, no stop prompt), discarding unrelated pending triggers; prompted iterations count toward the cap.
- DCGO: not applicable (no loop detection; a mandatory loop stalls the client).
- Aegis: partial (`ch18`: a loop nobody can stop ends in a draw; a stoppable loop is stopped; "§18-3-3's … 'declare a repeat count' stop procedure has no protocol representation").
- Fix: emit an event; draw when no optional choice exists in the cycle (§18-3-2); otherwise a repeat-count declaration (or per-repetition optional prompt), never clearing unrelated entries; reset the depth counter on player decisions; detect cycles by signature.

### attack-and-timing

**G-ENGINE-ATTACK-TIMINGS-SKIPPED-WHEN-ATTACKER-LEAVES**: `PDF-explicit`
- Rule: §11-1-5 (its example names the counter and block timings).
- Tests: `ch11_14_attack_procedure::attack_timings_continue_when_attacker_leaves_mid_resolution`, `ch16_attack_keywords::vortex_attack_has_block_timing`.
- Cause: `commit_attack_declaration` / `advance_pending_attack` return `cleanup_attack(Invalid)` as soon as the attacker is gone, so the counter timing never occurs.
- DCGO: agrees (`CounterTiming()` checks only `IsEndAttack`; block skipped when the attacker is gone).
- Aegis: agrees (`ch11`: "11-1-5: once an attack is declared, the later timings still occur even if the attacker leaves").
- Fix: continue to `CounterOpen` on the stored attack context (`try_enter_counter` must not index the vanished attacker), then skip block/battle to `cleanup_attack`; leave `cancelled` attacks alone.

**G-ENGINE-REMOVED-ATTACK-TARGET-SLOT-REUSED**: `PDF-explicit`
- Rule: §11-2-6.
- Test: `ch11_14_attack_procedure::removed_target_stays_the_target_and_attack_fails`.
- Cause: the attack target is a battle-area slot handle; after the target is removed the next permanent shifts into the slot and is battled.
- DCGO: agrees (holds the defender as a `Permanent` object).
- Aegis: agrees (`ch11`: "11-2-6: … that target remains the attack target and the attack simply fails").
- Fix: a `target_removed` flag (generalising `effective_target_removed_by_dna`) read by `resolve_pending_battle`, `try_enter_block`, the Raid rider and `live_battle_opponent_of`, or key the target by `battle_identity`; audit the `blocker` handle the same way.

**G-ENGINE-ATTACK-ENDS-BEFORE-END-OF-ATTACK-EFFECTS-RESOLVE**: `PDF-explicit`
- Rules: §11-6-2; §16-38-1, §16-38-4 (keywords, <Execute>).
- Tests: `ch11_14_attack_procedure::attack_does_not_end_until_all_processing_resolved`, `ch16_attack_keywords::execute_optional_eot_attack_then_pending_deletion_at_end_of_attack`.
- Cause: `cleanup_attack` drains `EndOfAttack`, then (even if the drain parked) expires end-of-attack modifiers and clears `pending_attack`, so "for the attack" effects, `<Progress>` and <Execute>'s self-deletion lose the attack.
- DCGO: agrees (`EndAttack()` yields on the stack before `CleanUp`).
- Aegis: agrees (`ch11`: "11-6-2: the attack-in-progress flag drops only once end-of-attack processing has fully resolved").
- Fix: split `cleanup_attack` at the drain; on a park keep `pending_attack` in an end-of-attack state and resume through `advance_pending_attack`.

**G-ENGINE-VORTEX-ATTACK-SKIPS-COUNTER-AND-BLOCK**: `PDF-explicit` (+ Q3869)
- Rule: §11-4-1 (also listed under §11-1-5 and §16-33-1).
- Test: `ch16_attack_keywords::vortex_attack_has_block_timing`.
- Cause: `begin_attack_open` short-circuits a Vortex attack straight to `Battle`, skipping Raid, Alliance, counter and block.
- DCGO: agrees (`SetIsVortex()` only relaxes the played-this-turn check).
- Aegis: none found.
- Fix: drop the short-circuit; keep `AttackInitiator::Vortex` only for the same-turn exemption; invert `tests/combat/counter_interrupt.rs::vortex_bypasses_counter`.
- **Fixed** (Plan 4 Task 13, `4967a8120`): §11-4-1 `proven`.

**G-ENGINE-VORTEX-ATTACKS-UNSUSPENDED-DIGIMON**: `PDF-explicit` (confirmation question W2)
- Rules: §11-2-7-1; §16-33-1 (keywords).
- Test: `ch16_attack_keywords::vortex_attack_target_is_a_suspended_digimon` (§16-33-1 also lists `vortex_optional_eot_attack_on_digimon_and_same_turn_played` (green) and `vortex_attack_has_block_timing`).
- Cause: the end-of-turn mask and `end_of_turn_attack_vortex_flag` offer a Vortex attack on every opponent Digimon, unsuspended included.
- DCGO: suspended targets only (`CanAttackTargetDigimon` requires `IsSuspended || isExecute`).
- Aegis: mixed (scenario "16-33-1: <Vortex> does not bypass the normal suspended-target requirement"; the 2026-09-05 `vortex-timing.md` audit describes allowing "unsuspended opposing Digimon").
- Classification: §11-2-7-1 names the legal targets ("the opponent or 1 of the opponent's suspended Digimon"); §16-33-1 grants an attack on a Digimon but no target permission, and the manual spells such a permission out wherever it exists (§16-38-1 <Execute>, the §15-1-3 example).
- Fix: apply the normal declaration-time target legality (suspended, or a `CanAttackUnsuspended` grant) to Vortex-only attacks in the mask and the decoder; keep the same-turn exemption. The engine's declaration legality also still carries a <Raid> highest-DP arm; that arm is not 4.3 behaviour (<Raid> switches the target after the declaration, §16-23-1/-2) and is tracked as `G-ENGINE-RAID-DECLARATION-TARGET-UNSUSPENDED`.
- **Fixed** (Plan 4 Task 13, `4967a8120`): §11-2-7-1, §16-33-1 `proven`.

**G-ENGINE-RAID-DECLARATION-TARGET-UNSUSPENDED**: `PDF-explicit` — filed 2026-10-17 (Plan 4 Task 13 review round 1) — **fixed (Plan 4 Task 33, `d5b90d8fb`)**: the declaration legality has no <Raid> arm at any of the four sites; `ch16_attack_keywords::raid_attacker_declares_on_a_legal_target_then_may_switch` is green and listed under §16-23-1 / §11-2-7-1; the pinning tests are inverted.
- Rules: §11-2-7-1; §16-23-1, §16-23-2 (keywords, <Raid>).
- Tests: `ch16_attack_keywords::raid_attacker_declares_on_a_legal_target_then_may_switch` (green; listed under 16-23-1 and 11-2-7-1, both `proven`). The former pins are inverted: `mask_main_parity::mask_raid_adds_no_unsuspended_declaration_target` and `::mask_raid_tied_highest_are_not_declaration_targets` (were `mask_raid_targets_highest_dp_unsuspended` / `mask_raid_allows_all_tied_for_highest`), `mask_end_of_turn_parity::vortex_with_raid_declares_on_a_legal_target_then_may_switch` (was `vortex_with_raid_declares_on_highest_dp_unsuspended_today`), plus `force_attack_mask`, `bt20_044_optional_attacker`, `bt20_040`, `bt24_011`.
- Cause: the declaration-time target legality in `action/mask.rs` (Main-phase, end-of-turn and effect-granted attack arms) and `action/decode.rs` `end_of_turn_attack_vortex_flag` lets a <Raid> attacker declare on the opponent's highest-DP unsuspended Digimon, although <Raid> only switches the target after the declaration (implemented in the attack's trigger pool since Task 1).
- DCGO: agrees with the rule (`Permanent.CanAttackTargetDigimon` has no <Raid> arm; `Raid.cs` is an `OnAllyAttack` `SwitchDefender`).
- Fix: drop the <Raid> arm from the declaration-time legality at all four sites; invert the three pinning tests.

### battle

**G-ENGINE-BATTLE-WIN-OBSERVERS-SEQUENCED-AFTER-ON-DELETION**: `PDF-explicit` — filed 2026-10-17 (Plan 4 Task 27 review round 1) — **fixed (Plan 4 Task 28, `055806250`)**: battle-result triggers and <Piercing> (`EffectTiming::BattleResult`) pend with the loser's [On Deletion], turn player first; `ch15_04_effect_states::battle_result_trigger_pools_with_the_losers_on_deletion_turn_player_first` and `::piercing_triggers_with_the_battle_deletion_pool` are green.
- Rules: §14-2-4; §15-4-3-2, §15-4-3-5-1 (effects-and-ordering); §16-7-4 (keywords, <Piercing>); official Q&A Q3931 (EX8-045).
- Test: `dsl::battle_deleter::on_deletion_choice_finishes_before_end_of_battle_observer_after_clone` pins the wrong order (P0 is the turn player, yet P1's [On Deletion] choice comes first); no conformance test is red yet, and the obligations keep their status.
- Cause: the battle deletion's trigger flush drains the loser's [On Deletion] / `OnAnyDeletion` pool to completion before `resume_pending_battle_resolutions` enqueues the `EndOfBattle` battle-result observers ("wins a battle", "deletes in battle"), so the turn player's battle-result triggers activate after the non-turn player's [On Deletion].
- DCGO: agrees with the rule (one `AutoProcessCheck` over the `OnDestroyedAnyone` and `OnEndBattle` stacks).
- Fix: enqueue the battle-result observers into the deletion batch's single `TriggerFlush` pool (with the §18-1-2 end-of-battle pending processing); invert the pinning test.

**G-ENGINE-BATTLE-RESULT-TRIGGER-CONDITION-NOT-CHECKED-AT-TRIGGER**: `PDF-explicit` — filed 2026-10-17 (Plan 4 Task 28 review round)
- Rules: §15-8-3-1, §15-8-3-8, §15-4-4-5.
- Test: `dsl::battle_deleter::winner_predicates_follow_ordinary_evolution_during_opponents_on_deletion` pins the wrong outcome (P1-turn staging: the observer activates after the loser's [On Deletion] made the winner [Evolved]); no conformance test yet.
- Cause: `BattleResult` entries are queued without evaluating their trigger condition; the drainer's transient non-firing filter lets one whose condition failed at the battle activate once a sibling makes it true.
- DCGO: agrees with the rule (`StackSkillInfos` evaluates `CanTrigger` when the battle stacks it).
- Fix: evaluate the trigger condition at the battle's enqueue and drop entries that did not trigger; keep the §15-4-4-5 activation re-check.

**G-ENGINE-BATTLE-WINNER-TRAIT-READS-DIGIVOLUTION-CARDS**: `needs-reading` — filed 2026-10-17 (Plan 4 Task 28 review round) — **fixed (Plan 4 Task 31, `8834bec70`)**: a Digimon's traits are its top card's (controller ruling, provisional; §4-7-7, Q3225, DCGO `permanent.TopCard`); `Permanent::has_trait` (now `has_printed_trait`) reads the top card and the winner / host-Digimon trait readers use `has_trait_for_rules`; the pinning test is un-ignored and `ch15_08_effect_categories::battle_winner_trait_is_the_top_cards_trait` / `::permanent_trait_is_the_top_cards_trait` are green. §4-7-7 stays `gap` (tests listed; they pin the characteristics reading, not the rule's text).
- Rules: §4-7-7 (bottom cards are part of the top card's information) vs DCGO's top-card trait read (`BT25_020.cs`).
- Test: `dsl::battle_deleter::turn_players_observer_reads_the_winners_top_card_before_de_digivolution` (ignored).
- Cause: `TriggerContext::battle_winner_has_trait` reads the live winner through `Permanent::has_trait`, which scans every card in the stack.
- Fix: read the winner's synthesized identity (top card + trait modifiers), once the reading is confirmed.

**G-DSL-CARRIER-NAME-READS-DIGIVOLUTION-CARDS**: `needs-reading` (same ruling as the trait gap above) — filed 2026-10-17 (Plan 4 Task 31 review round)
- Rules: §4-7-7 + Q3225 (a Digimon "without ... [Imperialdramon] in its name" once its top card is trashed); DCGO `TopCard.ContainsCardName`.
- Cards: BT12-059, EX1-014, BT20-059, BT17-102 read "this Digimon's name / is [X]" through `self_digivolution_contains_name` (top card + every digivolution card); BT16-027 / BT20-020 read "in its digivolution cards" through the same scan (harmless today).
- Cause: `Permanent::contains_card_name` scans the whole stack.
- Fix: re-author those cards onto a top-card name predicate (`source_name_contains`; BT17-102 on the synthesized names) and BT16-027 / BT20-020 onto `self_digivolution_sources_contain_name`; leave `contains_card_name` as is.

(§14 is 7 of 10 proven.)

### blocking

No blocking gap tagged `blocking`; the forced-block defect is `G-ENGINE-COLLISION-BLOCKER-NOT-GRANTED` (keywords) and the skipped block timing is `G-ENGINE-VORTEX-ATTACK-SKIPS-COUNTER-AND-BLOCK` (attack-and-timing).

### rule-checks

**G-ENGINE-RULE-CHECK-TARGETS-INCOMPLETE**: `PDF-explicit` (+ Q6370)
- Rules: §17-1-3-2-3, -4, -5, -6, -7.
- Tests: `ch17_rule_checks::non_digimon_in_breeding_area_is_trashed`, `face_down_card_on_field_is_trashed`, `excess_link_cards_trashed_player_chooses_which`, `excess_link_cards_over_any_limit_prompt_before_trash`, `link_card_failing_requirement_is_trashed`, `linked_card_of_unlisted_category_is_trashed`.
- Cause: `run_state_based_rules_check` scans only battle-area tops for 0 DP / no DP / orphan Options; breeding-area non-Digimon, face-down cards, over-limit link cards (player's choice), unqualified link cards and wrong-category link cards are never trashed.
- DCGO: agrees (`AutoProcessing.RuleProcess` has each sub-process; `DigimonLackLinkMaxCountProcess` lets the owner choose).
- Aegis: partial (`ch17`: the orphan-Option target; other targets none found).
- Fix: a breeding-area pass (needs a "placed by an effect" marker), a face-down top check, and a per-host link step (trim excess by the controller's resumable choice; re-evaluate `LinkCondition` and category), all in the simultaneous pass below.

**G-ENGINE-RULE-CHECK-TARGETS-NOT-SIMULTANEOUS**: `PDF-explicit`
- Rule: §17-1-3.
- Test: `ch17_rule_checks::all_rule_check_targets_processed_simultaneously`.
- Cause: each target goes through its own one-element `delete_permanents_batch`, whose `[On Deletion]` drains before the next target is removed; no `TriggerOrder` for two 0 DP Digimon.
- DCGO: partly disagrees (its sub-steps run in sequence, but all 0 DP Digimon are deleted in one batch); the PDF governs.
- Aegis: agrees (`trigger-ordering.md`: "Rule-check deletion joins the timing's other triggers; controller order"; `interaction-rule-check-triggers.test.ts`).
- Fix: collect every target, run all trashes plus ONE `delete_permanents_batch(all_zero_dp, Rule)` in one deferred-drain scope, flush once; keep the parked-deletion resume guard.

### turn-flow

No blocking gap. (Plan 1 did not cover chapter 6.)

### digivolution-and-costs

**G-ENGINE-KEYWORD-DIGISORPTION-UNIMPLEMENTED**: `PDF-explicit`
- Rules: §16-10-1, -2, -3, -4, -5 (keywords).
- Tests: `ch16_processing_keywords::digisorption_optional_suspend_reduces_cost_floors_at_zero_and_stacks`, `digisorption_kind_matches_rule`, `digisorption_timing_matches_rule`, `digisorption_optionality_matches_rule`.
- Cause: no `Keyword` variant, parse arm, DSL vocabulary or authored card; digivolving into a <Digisorption> card pays the full cost with no offer.
- DCGO: agrees (per-card `BeforePayCost` `ActivateClass`: suspend 1 Digimon, base included, then reduce).
- Aegis: agrees (`digisorption-lifecycle.md`: consent before payment, exact payer pick, refusal pays full cost).
- Fix: `Keyword::Digisorption(n)` with an optional before-pay-cost reducer on digivolution into the carrier (suspend any own unsuspended Digimon incl. the base, -N floored at 0, one prompt per instance); then author BT2/BT5/BT10/BT11. Reachable by no deck today (no authored card).

**G-ENGINE-BLAST-DNA-IGNORES-DNA-REQUIREMENTS**: `PDF-explicit`
- Rule: §16-31-5 (keywords).
- Test: `ch16_processing_keywords::blast_dna_cannot_ignore_dna_requirements`.
- Cause: the DSL-registered blast route is offered when its own name filters match; the card's DNA requirements are never consulted on that path.
- DCGO: agrees (`CanPlayJogress(true)` checks the DNA requirement).
- Aegis: none found (its BT20-045 proof uses requirement-meeting materials).
- Fix: on the registered path also require that (field, hand) satisfy one of the result card's DNA requirements before offering the route.

### play-and-costs

No blocking gap. (Plan 1 did not cover chapter 7.)

### card-use-and-costs

**F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE** (third half): `PDF-explicit`
- Rule: §9-1-5.
- Test: `ch18_pending_overwrite::used_option_trashed_immediately_after_first_main_uninterrupted`.
- Cause: a used Option's trash is offered as a `TriggerOrder` branch beside triggered effects, and a rule check runs between the `[Main]` body and the trash.
- DCGO: agrees (`UseOptionClass.UseOption` calls `AddTrashCard` straight after the skill).
- Aegis: agrees (`ch09`: "9-1-5: a used Option … is trashed as pending processing once its [Main] effect resolves").
- Fix: dispose the Option in the step that finishes its `[Main]` body, before `rules_check_between_queued_effects` and any trigger; drop the trash-vs-trigger `TriggerOrder` and `option_trash_order_deferred`; retire `support::trash_used_option_first` / `settle_after_option`. Reverses the earlier `G-ENGINE-OPTION-TRASH-TURN-PLAYER-ORDER` reading (W8).
- **Fixed** (Plan 4 Task 7, `c2b25d0b5`): §9-1-5 `proven`.

### link

**G-ENGINE-LINK-MAX-BASE**: `PDF-explicit`
- Rule: §4-9-5 (terminology-and-identity; outside the mechanics set).
- Test: `ch16_processing_keywords::link_max_base_is_one`.
- Cause: `link_host_candidates` / `can_insert_plug_in_at` use a base link limit of 5 (`5 + link_max_delta`), not 1.
- DCGO: agrees with the rule (`LinkedMax` starts at 1).
- Aegis: agrees (`link.md`: printed Link +1 "raising the base capacity from one to two").
- Fix: base 1; lands in one change with the replacement below (fixing the base alone turns the tests into refusals), in Plan 3 step 1 beside the §17-1-3-2-5 rule-check trim.

**G-ENGINE-LINK-AT-LIMIT-REFUSED-NOT-REPLACED**: `PDF-explicit` (chooser question W12)
- Rules: §16-40-1, -2, -3 (keywords), with §4-9-5.
- Tests: `ch16_processing_keywords::link_plus_adds_to_max_link_cards_and_instances_add`, `link_plus_kind_matches_rule`.
- Cause: hosts at their limit are dropped from link candidates, so a link never replaces an existing link card.
- DCGO: agrees with the rule (removes `(Count + 1) - LinkedMax` link cards at link time, the owner choosing).
- Aegis: none found (its proof stays below the limit).
- Fix: keep at-limit hosts as candidates; the linking player chooses the link card(s) to trash (resumable selection) in the same link step; keep the ch17 trim for hosts that go over later.

### keywords

**G-ENGINE-PROGRESS-DOES-NOT-HIDE-EARLIER-OPPONENT-EFFECTS**: `PDF-explicit` (+ Q6178) — filed 2026-10-16 (Plan 4 Task 24 fix round 1) — **FIXED** (`20646307f`, Plan 4 Task 25; see "Retracted or resolved")
- Rules: §16-39-1 (<Progress>), §15-15-5-1.
- Test: `ch15_15_unaffected::progress_attacker_not_affected_by_earlier_opponent_effect_while_attacking`; `progress_attacker_effect_applied_during_attack_applies_after_it` (green: an opponent's effect applied DURING the attack is held and applies after it).
- Cause: `<Progress>` is decided at install time (`held_while_attacking`); `ModifierRegistry::get` cannot read the attack state, so an opponent's modifier installed before the attack keeps applying while the Digimon attacks (a 6000-DP attacker under an earlier -3000 battles at 3000).
- DCGO: agrees with the rule (read-time `CanNotBeAffected`).
- Fix: a `Game`-maintained attack shield on the registry that `get` honours for opponent-controlled, effect-tagged entries.


**G-ENGINE-ENGAGE-ATTACK-AFTER-END-OF-TURN-TRIGGERS**: `PDF-explicit` (+ Q3868/Q3869; confirmation question W3)
- Rules: §16-44-2 (<Engage>), §16-38-2 (<Execute>), §16-33-2 (<Vortex>), §16-34-2 (<Overclock>).
- Tests: `ch16_new_keywords::engage_kind_matches_rule`, `engage_timing_matches_rule` (green); `ch16_attack_keywords::execute_kind_matches_rule`, `execute_timing_matches_rule` (green), `vortex_kind_matches_rule`, `vortex_timing_matches_rule` (green), `overclock_kind_matches_rule`, `overclock_timing_matches_rule` (green). The kind tests pin declaration < other `[End of Your Turn]` effect < counter timing / security check. The order follows from §11-1-4 ("After declaring an attack, the counter timing won't begin until any triggered [When Attacking] effects and other triggered effects have been resolved"), which holds for any end-of-turn "may attack" trigger, and Q3869 rules it for <Vortex>.
- Cause: choosing <Engage>/<Execute> first only grants `MayAttack`, so the other end-of-turn effect resolves before the attack is even declared (post-drain `EndOfTurnAction` window); <Vortex>/<Overclock> are not triggers at all.
- DCGO: partly disagrees (pools and orders them, as the rule requires, but runs the whole attack inside the activating coroutine, so the other effect waits for the whole attack, not just the declaration).
- Aegis: partial (`engage-lifecycle.md`: optional End of Your Turn self-attack; `vortex-timing.md`: Vortex is an optional EndOfYourTurn effect ordered against Shoto's end-turn effect both ways; the Q3869 sub-order is not evidenced).
- Fix: each keyword body declares the attack as its own resumable decision inside the drain; the remaining end-of-turn entries resolve after the declaration and before the counter timing, then the attack continues. Update the end-of-turn tests that treat decline as PASS.

**G-ENGINE-BLITZ-TURN-ENDS-BEFORE-ATTACK**: `PDF-explicit` (+ Q602)
- Rules: §16-16-1, -2, -3, -4, -5.
- Tests: `ch16_attack_keywords::blitz_optionality_matches_rule`, `blitz_kind_matches_rule`, `blitz_attack_proceeds_after_memory_returns_to_zero_or_more`, `blitz_cannot_attack_when_memory_already_zero_or_more`.
- Cause: <Blitz> has no processing; `digivolve_from_hand` ends with `check_turn_end`, so the turn passes before the attack the mask carve-out would allow.
- DCGO: agrees (`BlitzProcess` runs the attack inside the activating effect).
- Aegis: none found (only the glossary negative "if the Digimon is suspended … <Blitz> won't enable it to attack").
- Fix: a <Blitz> body (gate: opponent memory ≥ 1 at activation) that offers the attack as its own resumable decision, deferring `check_turn_end` to the attack's cleanup; only the attack and the decline are legal in that window; retire the mask carve-out.

**G-ENGINE-COLLISION-BLOCKER-NOT-GRANTED**: `PDF-explicit`
- Rules: §16-30-1, -2, -4.
- Tests: `ch16_attack_keywords::collision_forces_block_whenever_possible` (green), `collision_kind_matches_rule`, `collision_blocker_grant_affects_digimon_forced_block_affects_player`.
- Cause: `try_enter_block` special-cases Collision for candidacy instead of granting <Blocker>; an effect-immune Digimon is forced to block, and "with <Blocker>" readers miss the grant.
- DCGO: agrees (`HasBlocker` grant guarded by `CanNotBeAffected`).
- Aegis: agrees on the grant (`collision-iceclad.md`: "all opponent Digimon gain Blocker only while the Collision attacker is attacking"); immunity not evidenced.
- Fix: declarative grant: while the carrier attacks, `has_keyword(h, Blocker)` is true for each opponent Digimon not unaffected by the attacker's effects; drop the candidacy special case.

**G-ENGINE-ON-DELETION-DRAINED-BEFORE-DELETION-OBSERVERS**: `PDF-explicit`
- Rule: §16-13-2 (with §15-4-3-2 / §15-4-3-5).
- Tests: `ch16_attack_keywords::retaliation_kind_matches_rule`, `retaliation_timing_matches_rule` (green).
- Cause: `run_deletion_batch_stages` drains `OnDeletion` (stage 5) before enqueuing `OnAnyDeletion` / `OnLeaveField` (stage 6), so every `[On Deletion]` (incl. <Retaliation>) resolves before the "when a Digimon is deleted" observers with no ordering choice.
- DCGO: agrees (one `OnDestroyedAnyone` stack).
- Aegis: agrees (`trigger-ordering.md`: "native On Deletion with deletion watchers" in one order request).
- Fix: enqueue both stages before one flush; keep the post-trash snapshot threading.

**G-ENGINE-DEDIGIVOLVE-LEVEL-FLOOR-READS-NEXT-CARD**: `PDF-explicit`
- Rule: §16-12-4.
- Tests: `ch16_processing_keywords::de_digivolve_cannot_trash_level_3_or_lower`, `de_digivolve_player_declares_count_mandatory_at_least_one_stops_at_level_3`.
- Cause: `de_digivolve_core` tests the level of the card below the top, not the top card being trashed.
- DCGO: agrees (stops on the permanent's own level 3).
- Aegis: agrees ("does not trash a level 3-only stack"; "trashes three top cards without removing the level 3 card").
- Fix: test the top card's level before each pop.

**G-DEDIGIVOLVE-NO-DECLARATION**: `PDF-explicit`
- Rule: §16-12-1 (with §16-12-3).
- Test: `ch16_processing_keywords::de_digivolve_player_declares_count_mandatory_at_least_one_stops_at_level_3`.
- Cause: <De-Digivolve N> always trashes the maximum; the player never declares 1..N.
- DCGO: agrees (`SelectCountEffect`, `CanNoSelect: false`).
- Aegis: agrees ("uses the public number choice to declare one of a printed De-Digivolve 2").
- Fix: a non-optional count selection (1..min(N, trashable)) when at least 2 are trashable.

**G-ENGINE-DEDIGIVOLVE-FIRES-DIGIVOLUTION-CARD-TRASHED**: `PDF-explicit` (by the §4-8-1 definition)
- Rule: §16-12-1 (the obligation's `gap_id` is `G-DEDIGIVOLVE-NO-DECLARATION`; this gap is in its note).
- Test: `ch16_processing_keywords::de_digivolve_trashes_top_cards_not_digivolution_cards`.
- Cause: `de_digivolve_core` (and <Armor Purge>) fire `OnDigivolutionCardTrashed` for each popped top card.
- DCGO: agrees (stacks `WhenTopCardTrashed`, never `OnDigivolutionCardDiscarded`).
- Aegis: none found.
- Fix: a new `EffectTiming::OnTopCardTrashed` (DSL `on_top_card_trashed`, `G-DSL-TOP-CARD-TRASHED-TIMING`) fired once per instance; stop firing the digivolution-card timing; re-author `OBS_TOP_TRASH_MEM` on it and re-verify §16-12-5/-7.

**G-ENGINE-MATERIAL-SAVE-AFTER-DELETION**: `PDF-explicit`
- Rule: §16-21-2.
- Test: `ch16_processing_keywords::material_save_kind_matches_rule`.
- Cause: <Material Save> is an `on_deletion` effect working from the trash, not an immediate-type effect offered while the Digimon stands.
- DCGO: agrees (`WhenPermanentWouldBeDeleted`).
- Aegis: none found (its proofs check placement, not timing).
- Fix: re-time as a non-cancelling `WhenWouldBeDeleted` immediate effect (the <Partition> shape), moving the sources before the deletion.

**G-ENGINE-MATERIAL-SAVE-MAY-PLACE-FEWER**: `PDF-explicit`
- Rule: §16-21-3.
- Tests: `ch16_processing_keywords::material_save_optionality_matches_rule` (green), `material_save_places_the_full_number_whenever_possible`.
- Cause: the card pick is `min: 0, max: n` and stays optional after each pick.
- DCGO: agrees (`canEndNotMax: false`).
- Aegis: agrees in its tests ("saves four of six eligible sources"; "exactly the printed number … are placed"), though its audit prose says "up to".
- Fix: after the Tamer pick, require exactly `min(n, eligible)`.

**G-ENGINE-OVERFLOW-SIMULTANEOUS-ORDER-PER-PERMANENT**: `PDF-explicit` (blocks_training judgement W11)
- Rules: §4-19-5, §4-19-5-1, §4-19-5-2 (terminology-and-identity; outside the mechanics set).
- Test: `ch16_new_keywords::overflow_turn_player_orders_then_non_turn_player`.
- Cause: a deletion batch applies each stack's <Overflow> as it is trashed, in removal order (player descending), so the non-turn player's instances go first.
- DCGO: disagrees with the rule (per removed permanent, removal order).
- Aegis: agrees ("4-19-5-1/4-19-5-2: simultaneous Overflow is charged turn-player-first … order changes the clamped result").
- Fix: collect every leaving card of a simultaneous batch and call `apply_ace_overflow_for_sources` once, before any drain.

**G-ENGINE-DUAL-OPTION-INFO-NOT-REFERENCED-IN-CARD-SEARCH**: `PDF-explicit` (unqualified-reference question W5; blocks_training judgement W11)
- Rule: §4-6-6-2 (terminology-and-identity; outside the mechanics set).
- Tests: `ch16_new_keywords::dual_option_information_referenced_as_option_card` (§4-6-6-2 also lists the green `dual_use_declares_digimon_or_option_information`).
- Cause: card-zone predicates read a DUAL's colours from its Digimon information even when the search references it as an Option card, so "red Option card" offers a purple-Option DUAL and "purple Option card" hides it.
- DCGO: unknown (not recorded in the gap entry).
- Aegis: agrees ("4-6-6-1/4-6-6-2: a DUAL card's Option information is a DISTINCT field"; `dual-play-or-use.md`: Option colour requirements apply).
- Fix: with `kind: option` evaluate colour/cost from `dual.option` only; with `kind: digimon` from `dual.digimon` only; never the union.

## Non-blocking gaps

- `G-ENGINE-MIND-LINK-PLACEMENT-OPTIONAL` (§16-28-3): the placement pick can be declined after activation; outcome-equal to not declaring. DCGO also has `canNoSelect: true`; the rule governs.
- `G-ENGINE-KEYWORD-REFERENCE-MATCHES-ONE-VALUE` (§16-2): "with <Fragment>" matches only <Fragment (1)>; no production YAML references a parametric keyword this way yet.
- `G-DSL-OPTIONAL-PROCESSING-CONDITION-BLOCK` (§15-7-3, DSL vocabulary): a compound optional condition can be paid in part; production card BT18-019 is affected and needs a both-names gate.
- `G-ENGINE-OPTION-TRASH-NO-EVENT` (§9-1-5, no obligation of its own): a used Option's trash emits no `GameEvent`. **Fixed** (Plan 4 Task 7, `c2b25d0b5`): `GameEvent::Trash` is logged.
- `G-ENGINE-ENGAGE-EOT-PASS-ONLY-STEP` (§16-44-1, no obligation of its own): after an <Engage> attack the turn parks once more with PASS as the only legal action.
- `G-ENGINE-UNAFFECTED-KEYWORD-GRANT-INSTALL-SKIP` (§15-15-5-2 / §15-15-5-4): a keyword granted (`grant_keyword`) to an immune Digimon is skipped instead of suppressed, so it never applies after the immunity ends; no authored opponent-side keyword grant (BT22-029 grants <Blocker> to its own Puppet Digimon); reachable only as an own grant under an own-effect immunity (the Quantumon shape), in both directions (granted while immune is lost; granted before keeps applying, against Q6178). Modifier-backed gains and granted triggered effects are fixed (`G-ENGINE-UNAFFECTED-SKIPS-INSTEAD-OF-SUPPRESSES`).
- `G-ENGINE-DEPARTED-TOKEN-CONTEXT-FALLS-BACK-TO-RULE` (§15-2-1-1, no obligation of its own; filed by Plan 4 Task 26 review): a context rebuilt by handle for a token that has left play (`dsl_cards/step/mod.rs` ~L273 tail rewrap) resolves as a rule effect, because the kind fallback looks the token up by zone; no authored token reaches it (all token effects are raw_rust and carry their kind).
- `G-ENGINE-PROGRESS-SHIELD-BYPASSED-BY-TIMING-DISABLE-AND-FORMULA-AURAS` (§16-39-1 / §15-15-5-1): the `<Progress>` shield and the immunity filter cover `ModifierRegistry::get` only; `is_timing_disabled`, `permanent_modifiers_iter` / `any_with_type`, the live formula-aura sum inside `effective_dp` (`live_declarative_formula_sum`) and raw registry installs from card scripts bypass both. Latent: no authored opponent-side reach.
- `G-TEST-DEBUGRUNNER-IGNORES-ACTIVE-WHEN-ON-OWN-GRANT-KEYWORD` (test harness, no obligation of its own): `DebugRunner::card_data_from_compiled` folds a card's own conditional `grant_keyword` into its printed keywords, so a DebugRunner test can never see a "while X, this Digimon gains <KW>" keyword off.
- `G-ENGINE-STANDING-LINK-ABSORB-DRAINS-MID-PROCESS` (§4-9-5, no obligation of its own; filed by Plan 4 Task 30 review): `absorb_standing_digimon_as_link` drains the effect queue after each trashed digivolution card and after the source's own link cards, so on the §4-9-5 choice path the replaced link cards' `OnLinkedCardTrashed` triggers resolve before the new card attaches (and a trigger removing a lower-index permanent could leave the host handle stale); fix: queue only and drain once after `OnLink`. Shared by the declared standing link and the effect relink (only `_examples/EX11-027` authors it).
- `G-ENGINE-RESIDUAL-INLINE-DRAINS-OUTSIDE-SOURCE-TRASH` (§15-8-3-2, no obligation of its own): `fire_digivolution_card_returned_to_deck_bottom`, `digivolve_replacement_subject_without_cost`, `play_token` and `place_self_as_delay_option_permanent` still drain the queue inline; the source-trash paths were fixed under `G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS`.

- `G-ENGINE-REPLACED-TRAIT-STILL-COUNTS-FROM-PRINTED-TOP-CARD` (no obligation of its own; filed by the Plan 4 Task 31 review): `has_trait_for_rules` and the general `trait_has` OR the synthesized identity with the printed top card, so a replacing `ChangeTraits` / non-additive `TreatAsDigimon` cannot remove a printed trait, and the live winner read can disagree with its snapshot.

The other 330 non-blocking mechanics-set gaps (and 279 outside it) are untriaged defaults with no test, mostly chapters 1-10.

## Owner-rulings worksheet

Each item gives the question, the candidate readings, what DCGO does, and what the suite assumes today. None holds up a blocking fix: W2, W3, W5, W10, W11 and W12 attach to `PDF-explicit` gaps whose fix proceeds on the suite's reading.

**W1. §15-8-5 (15-8-5-1 / -5-4 / -5-5-2): prevention after a prevention.** After one immediate-type effect has prevented a deletion, may the carrier's other pending prevention effects still be offered in the same window? (TODO in the 15-8-5-4 sidecar note, `4609ca892`.)
- (a) Yes: each pending immediate effect activates if its condition holds at activation (§15-8-5-5-2); the engine does this today (the window keeps advancing with `cancelled = true`). The 15-8-5-4 sidecar note leans this way (§15-8-5-1 / -5-4 / -5-5-2).
- (b) No: the deletion is no longer about to happen, so the remaining "would be deleted" effects have no cause left and lapse.
- DCGO: not checked for this case (BT11-041's once-per-cause record is a stub). Suite: unasserted either way.

**W2. Confirmation: §16-33-1 <Vortex> attack target.** (`G-ENGINE-VORTEX-ATTACKS-UNSUSPENDED-DIGIMON`, classified `PDF-explicit`; does not hold up the fix.)
- (a) Suspended Digimon only: §11-2-7-1 names the legal targets; §16-33-1 grants the attack, not a target permission, while §16-38-1 <Execute> and the §15-1-3 example spell such a permission out. DCGO and Aegis's current scenario agree. Suite and fix assume (a).
- (b) Any opponent Digimon, reading §16-33-1's "attack an opponent's Digimon" as unqualified. Aegis's 2026-09-05 audit once implemented (b); the engine did (b) until Task 13.
- **Fixed (Task 13) on reading (a); owner may override.** Task 13 of Plan 4 (`4967a8120`) applies the normal declaration-time target legality (suspended, or a `CanAttackUnsuspended` grant; the engine's leftover <Raid> arm is `G-ENGINE-RAID-DECLARATION-TARGET-UNSUSPENDED`) to the <Vortex> attack in the mask and the decoder; `ch16_attack_keywords::vortex_attack_target_is_a_suspended_digimon` is un-ignored and §11-2-7-1 / §16-33-1 are `proven`. Reading (b) would invert that test's second assertion and `mask_end_of_turn_parity::mask_vortex_does_not_target_unsuspended_digimon`.

**W3. Confirmation: end-of-turn attack order for <Engage>, <Execute>, <Overclock>.** Q3868/Q3869 are rulings on <Vortex> (EX7-064): another end-of-turn effect ordered after it resolves after the attack declaration and before the counter timing. For the other three this is not analogy alone: §11-1-4 ("After declaring an attack, the counter timing won't begin until any triggered [When Attacking] effects and other triggered effects have been resolved") gives the same order for any pending trigger once an attack is declared. (Does not hold up the fix.)
- (a) The same order for every trigger-type end-of-turn "may attack" keyword (declaration < other effect < counter timing), per §11-1-4 + Q3869. Suite assumes (a) for all four.
- (b) For the others the other effect waits for the whole attack (DCGO's coroutine shape for all of them, including <Vortex>, contrary to Q3869).

**W4. §16-46-2 <Detach>: trigger-type or immediate-type?** The rule says "trigger-type" but defines the trigger as "would leave the battle area" and refers to §15-8-5.
- (a) Immediate-type: offered while the Digimon is still on the field with its link card. Suite assumes (a) (`detach_kind_matches_rule`, `detach_timing_matches_rule`, proven).
- (b) Trigger-type as worded: resolves after the Digimon left.
- DCGO: not recorded.

**W5. DUAL card referenced without a category.** §4-6-5-2 / §4-6-6-2 say which information applies when a DUAL is referenced as a Digimon or as an Option. Which applies to an unqualified reference such as "Trash 1 purple card from your hand"?
- (a) Either information (matches if either face matches).
- (b) The Digimon information only (the card's default, top-level face), as the engine does for every reference today.
- (c) Neither colour unless the effect names the category.
- DCGO: not checked. Suite: unasserted; the fix must pin whichever is chosen.
- *Superseded by the owner ruling below.* History: Task 18 of Plan 4 pinned provisional reading (a) (an unqualified reference matched if the whole reference held through either information), while `kind: option` / `kind: digimon` read that information only (§4-6-5-2, §4-6-6-2) and on-field digivolution cards read the Digimon information only (§4-6-3); those three still hold.
- **Owner ruled (b) 2026-10-17: Digimon information.** Fixed in Task 29 (421e95064). An unqualified reference reads the DUAL card's Digimon information only; the Option information is read only under `kind: option` (§4-6-6-2). `dual_unqualified_colour_reference_reads_digimon_information` (renamed from `..._matches_either_information`) and `dual_unqualified_reference_holds_through_one_information` are re-pinned: "red card" finds the red-Digimon / purple-Option DUAL; "purple card", "purple Lv.6 card" and "card that is not red" do not.

**W6. Sign-off: §15-8-3-9-4 versus Q1009** (15-8-3-9-2 sidecar note, `05d39e4cb`). §15-8-3-9-4 covers the removed card's own last-known state; references to general game state (e.g. your security count) are made when processing is performed (§15-8-3-9-3), as Q1009 rules for BT2-034. Proven on that reading (`pending_effect_whose_condition_lapsed_is_dropped`).

**W7. Sign-off: §15-8-5-4 "cause" is the interrupting event** (15-8-5-4 sidecar note, `b20297ec9`, which retracted `G-ENGINE-IMMEDIATE-ONCE-PER-CAUSE-SCOPED-TO-EVENT`). Two deletions from one `[Main]` are two causes; an immediate-type effect may activate once for each. The alternative (the resolving `[Main]` is the cause) would make it once per `[Main]`. DCGO enforces neither for BT11-041.

**W8. Sign-off: §9-1-5 reverses `G-ENGINE-OPTION-TRASH-TURN-PLAYER-ORDER`.** That earlier engine fix read §18-1-2 as making a used Option's trash orderable among its controller's triggered effects; the 4.3 §9-1-5 text ("Other effects and rule checks don't interrupt the period between [Main] effect activation and trashing") makes it uninterrupted. DCGO agrees with §9-1-5. The suite assumes the 4.3 reading.
- **Fixed on the 4.3 reading.** Task 7 of Plan 4 (`c2b25d0b5`) trashes the used Option in the step that finishes its `[Main]`, before the rules check and any trigger, with a `GameEvent::Trash`; the ordering prompt is gone. `ch18_pending_overwrite::used_option_trashed_immediately_after_first_main_uninterrupted` is un-ignored and §9-1-5 is `proven`; `G-ENGINE-OPTION-TRASH-NO-EVENT` is resolved with it.

**W9. Sign-off: rule text versus DCGO for <Mind Link> (§16-28-3).** The rule says the processing is mandatory once activated; DCGO lets the pick be declined. Filed as a non-blocking gap on the rule's reading (the PDF governs).

**W10. §18-3-3 loop representation in the action space.** §18-3-3 prescribes a repeat-count declaration (turn player, then non-turn player). Is a per-repetition optional prompt (keep accepting, then decline) an acceptable equivalent for the RL action space, or must the declaration be modelled as a bounded count action? The conformance test accepts either. A second design point: how the engine decides "neither player has the ability to stop it" (§18-3-2): no optional choice anywhere in the repeating cycle.
- **Fixed on the per-repetition reading; owner may override.** Task 22 of Plan 4 keeps one optional prompt per repetition (no repeat-count action): `ch18_pending_overwrite::stoppable_loop_repeats_until_its_owner_stops_it` repeats the loop 40 times and stops it (§18-3-3, §18-3-3-3 `proven`); §18-3-3-1 / §18-3-3-2 stay `gap`, non-blocking. "Neither player can stop it" (§18-3-2) is read as the same queued entries over the same board (zones' card handles, suspension, memory) recurring 3 times with no declinable decision in between (a forced pick does not count): `infinite_loop_is_stopped_not_silently_cleared` and `forced_prompt_loop_still_ends_in_a_draw` end in `GameEvent::InfiniteLoop` + a `GameOver` draw, and `long_finite_chain_is_not_an_infinite_loop` runs to its end (`89db1771c`, `28db41b16`). Review round 1 (`3e1614560`): only optional prompts and the DSL choose-one count as declinable (trigger / replacement order, count and refire prompts are forced), the draw is final inside an attack, and the signature also hashes effective DP and per-turn activation counts.

**W11. `blocks_training` judgement calls.** Overflow (`G-ENGINE-OVERFLOW-SIMULTANEOUS-ORDER-PER-PERMANENT`: the outcome differs only when the memory marker hits ±10 mid-sequence) and DUAL search (`G-ENGINE-DUAL-OPTION-INFO-NOT-REFERENCED-IN-CARD-SEARCH`) are flagged blocking (`173375296`). Keep, or downgrade to non-blocking?

**W12. Link-time replacement chooser (§4-9-5).** §4-9-5 says the same number of existing link cards are trashed but not who chooses which. Q6370 gives the choice to "the player" for the rule-check trim. Reading: the linking player (who controls the host, §10-1-3-1) chooses. DCGO: the owner chooses when the maximum is above 1. Confirmation only.
- **Confirmed by owner 2026-10-17** (linking player chooses; implemented by Task 3).

## Retracted or resolved during the campaign

- `G-ENGINE-IMMEDIATE-ONCE-PER-CAUSE-SCOPED-TO-EVENT`: retracted (`b20297ec9`). The engine's per-event `replacement_fired` scope is right; `ch15_08_effect_categories::immediate_effect_once_per_interrupting_cause` now proves §15-8-5-4 (W7).
- `G-ENGINE-COMPOUND-OPTIONAL-CONDITION-PARTIAL-PAYMENT`: retracted as an engine gap (`ed7a9a9cb`); refiled as DSL vocabulary `G-DSL-OPTIONAL-PROCESSING-CONDITION-BLOCK` (non-blocking list).
- `G-REPLACEMENT-NO-ORDER-PROMPT`: no longer reproduces (checked 2026-10-16): `replacement_window.rs` installs an `EffectChoice` order for simultaneous immediate-type effects; `ch16_replacement_keywords::two_simultaneous_replacements_ordered_by_controller` proves §15-8-5-3.
- `G-ENGINE-SECURITY-CHECK-OBSERVERS-AFTER-BATTLE`: resolved by the F2 merge (`b41f3f5f1`, the §13-1-8 reorder `SecuritySkillDrain → OnSecurityCheckDrain → BattleResolved`); `ch11_14_attack_procedure::check_triggered_effects_resolve_before_security_battle` is un-ignored and green, and §13-1-8 / §13-1-8-2-1 / §13-1-8-3-1 are `proven`.
- `G-ENGINE-DNA-RESULT-KEEPS-BASE-ORIENTATION`: resolved (§8-2-2-1-1, design F3). Fix `98d4937a6` (PR #737); the per-clause conformance tests `code/digimon-engine/tests/dna_result_new_card.rs` landed with the F3 merge and are green. The tracker entry in `docs/RUST_ENGINE_GAPS.md` is RESOLVED.
- `G-ENGINE-UNAFFECTED-SKIPS-INSTEAD-OF-SUPPRESSES`: found and resolved in Plan 4 Task 24 (§15-15-5-1 / -2 / -4, `1e3fc9d9e`). Modifiers and granted triggered effects applied to an immune Digimon are recorded and hidden at read time, so they apply once the immunity ends; `tests/rules_conformance/ch15_15_unaffected.rs` (10 tests) is green and 15-15-5-1..4 are `proven`.
- `G-ENGINE-PROGRESS-DOES-NOT-HIDE-EARLIER-OPPONENT-EFFECTS`: resolved in Plan 4 Task 25 (§16-39-1 / §15-15-5-1 / §15-15-5-2, `20646307f`). `<Progress>` is a read-time shield on the modifier registry (`ModifierRegistry::progress_shield`, keyed to the attacker by identity), so an opponent's effect applied before the attack stops applying while it attacks and applies again the moment it ends or the attacker loses `<Progress>`; `ch15_15_unaffected` progress tests green, 16-39-1 `proven`.
- `G-ENGINE-BATTLE-WIN-OBSERVERS-SEQUENCED-AFTER-ON-DELETION`: found in the Task 27 review and resolved in Plan 4 Task 28 (§14-2-4, §15-4-3-5-1, §16-7-4; `055806250`). The battle-result triggers and <Piercing> pend with the loser's [On Deletion] in one pool, the turn player's first (Q3931); the end-of-battle timing (§14-2-5) follows the pool.
- `G-ENGINE-BATTLE-WINNER-TRAIT-READS-DIGIVOLUTION-CARDS`: found in the Task 28 review and resolved in Plan 4 Task 31 (§4-7-7; `8834bec70`). A Digimon's traits are its top card's (controller ruling, provisional; official Q&A Q3225; DCGO reads `permanent.TopCard`): the shared `Permanent::has_trait` (now `has_printed_trait`) no longer scans the digivolution cards, so the battle-winner qualifier and the host-Digimon trait predicate read the top card.
- `G-ENGINE-SCHEDULED-DELAY-MANDATORY-SCAN`: resolved (fixed 2026-08-24 in `73733429d`, confirmed by `delay_scheduled_window_is_optional`); entry moved to `qa/resolved-gaps.md`.

## Vocabulary gaps filed (`qa/dsl-vocab-gaps.md`, 2026-10-16)

- `G-DSL-OPTIONAL-PROCESSING-CONDITION-BLOCK`: a first-class "by X, Y" condition block (§15-7); also the long-term fix for the unpayable-condition and refire gaps; BT18-019.
- `G-KEYWORD-MATRIX-VER4-ROWS`: `qa/rules/keyword_semantics_matrix.json` lacks Ver. 4.x rows (<Use Req.>, <Detach>, …) and sources four rows from `engine_extension` instead of §16-42..§16-45 / §4-20.
- `G-DSL-REPLACEMENT-KEYWORD-EXCLUSIONS-LINT`: proposed `dsl-lint` guard that `<Partition>` / `<Decode>` clauses carry their rule exclusions and optionality (several §16-29 / §16-36 proofs hold only for that authoring path).
- `G-DSL-TOP-CARD-TRASHED-TIMING`: no `on_top_card_trashed` timing (BT21-094, BT8-110, EX10-010); the §16-12-5 observer witness is authored on a stand-in timing until it exists.

## Suggested fix order for Plan 3

By blast radius: substrate first, so later fixes land on the corrected loop.

1. **Trigger-loop pooling** (one simultaneous pool per event, §15-4-3):
   - `G-ENGINE-ALLIANCE-OUTSIDE-TRIGGER-POOL` (Alliance + Raid windows) with `G-ENGINE-WHEN-ATTACKING-PARK-DROPS-ALLY-ATTACK-OBSERVERS`;
   - `G-ENGINE-PENDING-PROCESSING-OUTSIDE-TRIGGER-POOL`;
   - `G-ENGINE-RULE-CHECK-TARGETS-INCOMPLETE` + `G-ENGINE-RULE-CHECK-TARGETS-NOT-SIMULTANEOUS`, with `G-ENGINE-LINK-MAX-BASE` + `G-ENGINE-LINK-AT-LIMIT-REFUSED-NOT-REPLACED` landing together in one change (§17-1-3-2-5 needs the base limit of 1, and the base alone turns the link tests into refusals);
   - `G-ENGINE-ON-DELETION-DRAINED-BEFORE-DELETION-OBSERVERS`;
   - `G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS` (#726 rework) and `F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE` (§9-1-5).
2. **Optional-condition legality:** `G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED`, then `G-ENGINE-REFIRED-EFFECT-SKIPS-OPTIONAL-DECISION` (shares the gate).
3. **Attack procedure:** `G-ENGINE-ATTACK-TIMINGS-SKIPPED-WHEN-ATTACKER-LEAVES`, `G-ENGINE-REMOVED-ATTACK-TARGET-SLOT-REUSED`, `G-ENGINE-ATTACK-ENDS-BEFORE-END-OF-ATTACK-EFFECTS-RESOLVE`, `G-ENGINE-BLITZ-TURN-ENDS-BEFORE-ATTACK`, `G-ENGINE-VORTEX-ATTACK-SKIPS-COUNTER-AND-BLOCK`, `G-ENGINE-VORTEX-ATTACKS-UNSUSPENDED-DIGIMON`, `G-ENGINE-COLLISION-BLOCKER-NOT-GRANTED`, `G-ENGINE-RAID-DECLARATION-TARGET-UNSUSPENDED` (filed 2026-10-17).
4. **Keyword implementations:** `G-ENGINE-KEYWORD-DIGISORPTION-UNIMPLEMENTED`; `G-ENGINE-MATERIAL-SAVE-AFTER-DELETION` + `G-ENGINE-MATERIAL-SAVE-MAY-PLACE-FEWER`; `G-ENGINE-BLAST-DNA-IGNORES-DNA-REQUIREMENTS`; De-Digivolve ×3 (`G-ENGINE-DEDIGIVOLVE-LEVEL-FLOOR-READS-NEXT-CARD`, `G-DEDIGIVOLVE-NO-DECLARATION`, `G-ENGINE-DEDIGIVOLVE-FIRES-DIGIVOLUTION-CARD-TRASHED` with `G-DSL-TOP-CARD-TRASHED-TIMING`); `G-ENGINE-OVERFLOW-SIMULTANEOUS-ORDER-PER-PERMANENT`; `G-ENGINE-DUAL-OPTION-INFO-NOT-REFERENCED-IN-CARD-SEARCH` (the fix pins whichever reading W5 confirms); `G-ENGINE-ENGAGE-ATTACK-AFTER-END-OF-TURN-TRIGGERS` (Engage / Execute / Vortex / Overclock end-of-turn ordering, after the attack-procedure fixes).
5. **Loops:** `G-ENGINE-INFINITE-LOOP-SILENTLY-CLEARED` (either prompt shape in W10 passes the test).

Each fix un-ignores its tests, flips the sidecar entries to `proven`, regenerates `qa/rules/cr_obligations.json`, and re-triages any DCGO exam that becomes `diverged` as a cited `dcgo_quirk` in the same PR (design §6).
