# Rules Conformance 4 — Engine Fixes for the 29 Blocking Gaps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the 29 blocking rules-conformance gaps (63 obligations) that Plan 1 found, in the digest's blast-radius order, so every `#[ignore]`d `rules_conformance` test that names one of them runs green and its obligations flip to `proven`.

**Architecture:** Substrate first. The trigger loop becomes one simultaneous pool per event (attack declaration, timing-scheduled pending processing, rule-check sweep, deletion batch, source-trash observers, used-Option disposal); then optional-condition legality; then the attack state machine (attacker leaves, target removed, end-of-attack park, Vortex, Collision, Blitz); then keyword implementations on the corrected loop; finally infinite-loop handling. Every task is TDD against tests that already exist: un-ignore, run red, fix, run green, flip the sidecar, regenerate the ledger.

**Tech Stack:** Rust (`code/digimon-engine`), `digimon-dsl` crate, the `rules_conformance` test binary (`cite!` fingerprints, `DebugRunner`), `code/tools/rules/` Python tooling (`build_obligations`, `coverage_report`), `qa/rules/cr_obligations_status.yaml` sidecar.

**Spec:** `docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md` §5 Phase 3, §6 (decisions: PDF governs over DCGO's loop; no training-pool shrinkage, re-triage flipped exams as `dcgo_quirk`; owner rulings 1–6 are the pin), §F4, §F5. **Findings digest (the per-gap requirements):** `qa/rules/mechanics-gaps-2026-10.md` ("Blocking gaps" and "Suggested fix order for Plan 3"; this plan is that fix plan). **Gap entries (cause, DCGO reading, fix shape, exact engine sites):** `docs/RUST_ENGINE_GAPS.md` §"Findings from the 2026-10-15 rules-conformance investigation" (~L5259–5632), plus `G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS` (~L4602), `G-ENGINE-LINK-MAX-BASE` (~L4707), `F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE` (~L3278), `G-DEDIGIVOLVE-NO-DECLARATION` (~L2628). Rules text: `docs/digimon-rules/cr-4.3.txt` (grep `^<id>`).

## Global Constraints

- **Canon:** Comprehensive Rules Ver. 4.3. Cite 4.3 ids in code comments and commit messages. When DCGO's loop contradicts explicit 4.3 text, the PDF governs (spec §6); the digest already classified all 29 gaps `PDF-explicit`, so no fix waits on the owner.
- **Discover-then-pin:** never weaken, delete or re-write a `rules_conformance` assertion to make it pass. The only permitted edits to a conformance test are: removing its `#[ignore = …]` attribute; updating a test that the gap entry explicitly says must change with the fix (listed per task); fixing a vacuous assertion the reviewer proves vacuous. `cite!` pins are verified statically by `meta::every_pinned_cite_in_source_matches`; never touch a fingerprint.
- **No approximations (CLAUDE.md rules 17, 28):** every new player decision goes through `pending_selection` via the resumable VM (`crate::resume`), never a closure callback. No auto-selection, no tail trims where the rules give the player a choice.
- **Clone safety:** `Game: Clone` must keep working; new parked state is plain data.
- **Build isolation:** every cargo command uses `CARGO_TARGET_DIR='D:\cargo-target-wt\digimon-simulator-validation-4dd340'` (CLAUDE.md rule 31). Before a full `cards_behavioral` run, kill stray `cards_behavioral-*.exe` / `cargo` processes (rule 33).
- **Test invocations:**
  - conformance: `CARGO_TARGET_DIR=… cargo test --manifest-path code/digimon-engine/Cargo.toml --test rules_conformance -- <filter>`; a red check uses `-- --include-ignored <filter>` before the ignore is removed, or removes the ignore first and runs plainly.
  - full card suite (phase-end tasks only): `RUST_MIN_STACK=268435456 CARGO_TARGET_DIR=… cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- --test-threads=8`.
  - other engine binaries named per task: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test <name>`.
  - ledger: `PYTHONPATH=code python -m tools.rules.build_obligations && PYTHONPATH=code python -m tools.rules.build_obligations --check && PYTHONPATH=code python -m tools.rules.coverage_report --check` (requires every `proven` obligation to have a live, non-ignored citing test).
- **Sidecar flips:** a task flips exactly the obligations whose tests it un-ignores to `status: proven` in `qa/rules/cr_obligations_status.yaml` (remove `blocks_training`, `gap_id`; keep `tests`; update `note`), then regenerates `qa/rules/cr_obligations.json` and commits both. An obligation whose other test is still ignored stays `gap`.
- **Gap tracker:** the task marks its gap entry in `docs/RUST_ENGINE_GAPS.md` `— RESOLVED (2026-10-DD, <sha>)` in the heading and appends one "Resolution:" line (what changed, the test). It does not rewrite the entry.
- **Existing tests that encode the wrong behaviour** (named per task) are inverted or deleted in the same commit with a comment citing the rule; any other failing engine test is a regression to fix, not to edit.
- **Commit shape:** one or more focused commits per task; subject `engine: <what> (§<ids>; <GAP-ID>)`; body cites the rule text in one sentence. Trailer `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. Commit on the current branch; never push (the controller pushes).
- **Scope discipline:** no card authoring (per-card YAML) unless a task names it; no DSL vocabulary beyond what a task names; widen the substrate, never route around it (a per-card gate added to make one test pass is a defect).

## Review Focus

1. **A trigger pool whose first entry parks on a selection** — the remaining entries of the same pool (including the opponent's) must still be offered after the park resolves, never dropped. Pinned by Task 1 `turn_player_orders_first_then_non_turn_player` and Task 5 `retaliation_kind_matches_rule`; reviewer checks the resume path of each new bundle.
2. **A deletion batch whose carrier has both an `[On Deletion]` and an immediate-type prevention** — the prevention must still cancel before any pooled trigger fires. Pinned by the green `ch16_replacement_keywords` suite, which every task must keep green.
3. **An attack whose attacker and target both leave before battle** — the attack must reach end-of-attack with no battle, no winner label and no slot reuse. Pinned by Task 10 and Task 11 tests together; reviewer runs both after Task 11.
4. **A `[Main]` or trigger whose optional condition is payable only partially** (e.g. "by trashing 2 cards" with 1 in hand) — must not be offered. Pinned by Task 8's guard predicate taking the count, not a boolean.
5. **A cycle that is long but finite** (e.g. 60 chained draws) — must not be mistaken for an infinite loop. Pinned by Task 22's signature-based detection test.

---

## Phase A — Trigger-loop pooling (§15-4-3, §18-1-2, §17-1-3, §15-8-3-2, §9-1-5)

### Task 1: Attack declaration is one simultaneous pool (Alliance, Raid, ally/opponent observers)

**Gaps:** `G-ENGINE-ALLIANCE-OUTSIDE-TRIGGER-POOL`, `G-ENGINE-WHEN-ATTACKING-PARK-DROPS-ALLY-ATTACK-OBSERVERS`.
**Rules:** §15-4-3-2, §15-4-3-5-1, §15-4-3-5-2, §16-24-2 (<Alliance>), §16-23-2 (<Raid>), §16-24-3 (Alliance's suspend is an optional processing condition), §16-24-4 (DP snapshot at suspension).

**Files:**
- Modify: `code/digimon-engine/src/combat/mod.rs` — `fire_on_attack` (~L4227), `begin_attack_open` (~L502), `advance_pending_attack` (~L710: the `RaidOpen` / `AllianceOpen` arms), `try_enter_alliance` (~L1356), `try_enter_raid_switch` (~L2484), module doc (L10 state list).
- Modify: `code/digimon-engine/src/cards/keyword_effects.rs` — `keyword_to_auto_effect_inner` (~L597): add `Keyword::Alliance` and `Keyword::Raid` arms producing `Effect::when_attacking(card)` trigger bodies.
- Modify: `code/digimon-engine/src/selection.rs` / `resume.rs` only if the ally pick / switch pick needs a new `ResumeFrame` variant (reuse `SelectionKind::OwnField` / `OppField` frames that exist).
- Tests to un-ignore: `tests/rules_conformance/ch15_04_effect_states.rs::when_attacking_and_alliance_share_one_pool`, `::turn_player_orders_first_then_non_turn_player`; `tests/rules_conformance/ch16_attack_keywords.rs::alliance_kind_matches_rule`, `::raid_kind_matches_rule`.
- Regression suites: `--test combat` (esp. `alliance_interrupt`, `raid_retarget`, `on_ally_opponent_attack`, `when_attacking_scope`), `--test rules_conformance`, `--test cards_behavioral -- alliance raid when_attacking` (filter), `--test judge_quiz`, `--test rules_faq`.

**Interfaces:**
- Consumes: `Game::enqueue_triggered(timing, TriggerSource)`, `maybe_drain_effect_queue`, the drainer's `TriggerOrder` install (`effect_queue.rs::install_trigger_order_selection`), `Effect::when_attacking`, `EffectBuilder::outer_optional_guard`.
- Produces: `fire_on_attack` enqueues `OnAttack`, `WhenAttacking` (incl. the synthesized Alliance/Raid bodies), the declaration `OnSuspend`, `OnAllyAttack` (attacker filtered out) and `OnOpponentAttack` as ONE bundle and calls `maybe_drain_effect_queue` once. `AttackState::RaidOpen` and `AttackState::AllianceOpen` become pass-through states (kept in the enum for serialization/tensor compatibility; `advance_pending_attack` transitions straight through them). The Alliance body: outer optional prompt → `OwnField` ally pick (other unsuspended Digimon that can suspend) → suspend, snapshot the ally's DP, add it to the attacker and grant `<Security A. +1>` for the attack (reuse the existing `try_enter_alliance` application code as a `Game` method `apply_alliance_ally(attacker, ally)`). The Raid body: optional `OppField` pick restricted to the opponent's unsuspended Digimon with the highest DP (ties: all offered), then retarget via the existing retarget routine (`combat/mod.rs` ~L1150 `set_effective_target`-style path; keep the §11-2-6 flag from Task 11 in mind: do not reset it).

- [ ] **Step 1: Remove the four `#[ignore]` attributes; run them.** Expected: all four FAIL (first decision is `OwnField`/`OppField` or the gain resolves with no `TriggerOrder`; P1's observer never fires after the park).
- [ ] **Step 2: Enqueue the whole declaration bundle before the first drain** in `fire_on_attack`; drop the three intermediate `maybe_drain_effect_queue` + early-return pairs. Keep the attacker-slot filter for `OnAllyAttack`. The `handle_still_attacking` early return after the single drain stays (Task 10 changes what happens after it).
- [ ] **Step 3: Synthesize Alliance/Raid as `[When Attacking]` trigger bodies** in `keyword_to_auto_effect_inner`, with the optional-condition / optional-processing semantics above; make the two `AttackState` windows pass-through and delete the now-dead prompt installers (or reduce them to the shared apply helpers).
- [ ] **Step 4: Run the four tests → PASS.** Run the regression suites; fix regressions in engine code, not tests (tests in `tests/combat/alliance_interrupt.rs` / `raid_retarget.rs` that assert the window phase `GamePhase::AllianceTiming` or a bare `OwnField` prompt are updated to the pooled shape with a `§16-24-2` comment — list each in the report).
- [ ] **Step 5: Sidecar flips:** `15-4-3-2`, `16-24-2`, `16-23-2`, `15-4-3-5-1`, `15-4-3-5-2` → `proven` (only if every listed test for the id is now live). Regenerate; `--check` green. Mark both gap entries RESOLVED.
- [ ] **Step 6: Commit** — `engine: attack declaration triggers form one simultaneous pool incl. <Alliance>/<Raid> (§15-4-3-2, §16-23-2, §16-24-2; G-ENGINE-ALLIANCE-OUTSIDE-TRIGGER-POOL, G-ENGINE-WHEN-ATTACKING-PARK-DROPS-ALLY-ATTACK-OBSERVERS)`.

### Task 2: Pending processing joins the timing's simultaneous pool

**Gap:** `G-ENGINE-PENDING-PROCESSING-OUTSIDE-TRIGGER-POOL`. **Rule:** §18-1-2 (with §18-1-1, §15-4-3-5-1).

**Files:**
- Modify: `code/digimon-engine/src/scheduled_effects.rs` — `fire_scheduled_for_timing` (~L68), `drain_scheduled_for_timing`, `resume_scheduled_drain`, `ScheduledDrainTail`.
- Modify: `code/digimon-engine/src/selection.rs` — add `TriggerSource::Scheduled(ScheduledEffect)` (or a `QueuedEffect` body variant) so the drainer can run a captured body with its `captured_bindings` and `runtime`.
- Modify: `code/digimon-engine/src/effect_queue.rs` — the single-trigger/bundle run path executes the scheduled body via `run_steps` against a fresh `EffectContext` (the same code `drain_scheduled_for_timing` uses today); controller = `ScheduledEffect::controller`.
- Modify: every caller of `fire_scheduled_for_timing` (`game_phases.rs::fire_end_of_your_turn` ~L1023 and the `EndOfOpponentsTurn`, `EndOfAttack` (`combat/mod.rs::cleanup_attack`), `UntilNextUnsuspend` sites; grep `fire_scheduled_for_timing(`): materialise the eligible scheduled effects (`can_fire_scheduled` && `timing_controller_matches`) into the queue BEFORE the timing's printed triggers are drained, in the same `enter_deferred_drain` … `exit_deferred_drain_and_flush` scope.
- Tests to un-ignore: `ch18_pending_overwrite.rs::pending_processing_orders_with_simultaneous_triggers`.
- Keep green: `ch18_pending_overwrite::pending_processing_fires_at_its_timing_like_a_trigger` (§18-1-1), `--test cards_behavioral -- bt1_090 bt16_085` (the `schedule_delayed` authors), `--test combat`, Delay/turn-end suites (`--test cost_hooks`, `--test phase_flow` if present; otherwise `--test combat` + `--test rules_conformance`).

**Interfaces:**
- Produces: `Game::enqueue_scheduled_for_timing(t: EffectTiming) -> usize` (count enqueued) called by each timing site before its printed-trigger drain; `ScheduledDrainTail` retained only for the "scheduled during this drain fires at the next boundary" rule (an effect scheduled while the pool drains is NOT added to the current pool).

- [ ] **Step 1: Un-ignore; run → FAIL** (no `TriggerOrder`; fixed order `EOT_MEM` then `-3`).
- [ ] **Step 2: Implement** the source kind + the materialisation helper; switch each timing site to enqueue-before-drain.
- [ ] **Step 3: Run → PASS**; run the keep-green list.
- [ ] **Step 4: Sidecar:** `18-1-2` → proven. Regenerate; `--check`. Gap entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: pending processing pools with the timing's triggered effects (§18-1-2; G-ENGINE-PENDING-PROCESSING-OUTSIDE-TRIGGER-POOL)`.

### Task 3: Link limit base 1, link-time replacement, and the per-host link rule-check step

**Gaps:** `G-ENGINE-LINK-MAX-BASE`, `G-ENGINE-LINK-AT-LIMIT-REFUSED-NOT-REPLACED`, and the link half of `G-ENGINE-RULE-CHECK-TARGETS-INCOMPLETE` (§17-1-3-2-5/-6/-7). **Rules:** §4-9-5, §16-40-1/-2/-3, §17-1-3-2-5 (+Q6370), §17-1-3-2-6, §17-1-3-2-7, §10-1-3-1 (chooser = host's controller; W12 reading).

**Files:**
- Modify: `code/digimon-engine/src/game_actions/mod.rs` — `link_host_candidates` (~L965: base limit `1 + link_max_delta`, keep at-limit hosts), `run_state_based_rules_check` (~L1400: add the per-host link pass; the simultaneity rework is Task 4, so add the pass as a collected `RuleAction::TrimLinks { host, excess }` / `TrashUnqualifiedLinks { host, cards }` the sweep applies).
- Modify: `code/digimon-engine/src/game_actions/link.rs` — the link commit step: when `linked_cards.len() + 1 > limit`, park a resumable `CardList`-style selection for the host's controller choosing exactly `excess` existing link cards to trash, then insert the new card in the same step (one `ResumeFrame`, e.g. `ResumeFrame::LinkReplaceChoice { host, incoming, excess }`).
- Modify: `code/digimon-engine/src/option_lifecycle.rs` — `can_insert_plug_in_at` (~L113): same base and no at-limit refusal.
- Tests to un-ignore: `ch16_processing_keywords.rs::link_max_base_is_one`, `::link_plus_kind_matches_rule`, `::link_plus_adds_to_max_link_cards_and_instances_add`; `ch17_rule_checks.rs::excess_link_cards_trashed_player_chooses_which`, `::excess_link_cards_over_any_limit_prompt_before_trash`, `::link_card_failing_requirement_is_trashed`, `::linked_card_of_unlisted_category_is_trashed`.
- Existing tests encoding the base of 5: grep `tests/` and `cards_behavioral` for `link_max`, `5 link`, `LinkedMax`; invert with a `§4-9-5` comment. Keep green: `--test cards_behavioral -- link plug_in bt26 ex12` (filter), `--test rules_conformance`.

**Interfaces:**
- Produces: `Game::link_limit(host: PermanentHandle) -> usize` (= `1 + link_max_delta(host)`, used by candidates, plug-ins and the rule check); `Game::collect_link_rule_targets(&self) -> Vec<(PermanentHandle, LinkRuleTarget)>` where `LinkRuleTarget::{Excess(usize), Unqualified(Vec<CardHandle>)}`; the excess trim parks a selection for the host's controller (no tail trim, even for excess == len).

- [ ] **Step 1: Un-ignore all seven; run → FAIL** (links add past 1; at-limit refused; nothing pending after the check).
- [ ] **Step 2: Base 1 + at-limit hosts kept; link-time replacement chooser.**
- [ ] **Step 3: Per-host link rule-check pass** (excess → controller's choice; requirement/category failures → trashed, no choice).
- [ ] **Step 4: Run → PASS**; regression list.
- [ ] **Step 5: Sidecar:** `4-9-5`, `16-40-1`, `16-40-2`, `16-40-3`, `17-1-3-2-5`, `17-1-3-2-6`, `17-1-3-2-7` → proven. Regenerate; `--check`. Gap entries RESOLVED (RULE-CHECK-TARGETS-INCOMPLETE stays OPEN until Task 4 with a note).
- [ ] **Step 6: Commit** — `engine: link limit base 1, link-time replacement by the linking player, link rule-check targets (§4-9-5, §16-40, §17-1-3-2-5..7; G-ENGINE-LINK-MAX-BASE, G-ENGINE-LINK-AT-LIMIT-REFUSED-NOT-REPLACED)`.

### Task 4: Rule check covers every target and processes them simultaneously

**Gaps:** `G-ENGINE-RULE-CHECK-TARGETS-INCOMPLETE` (§17-1-3-2-3, -4), `G-ENGINE-RULE-CHECK-TARGETS-NOT-SIMULTANEOUS`. **Rules:** §17-1-3, §17-1-3-2-3, §17-1-3-2-4, §15-4-3-2, §15-4-3-5.

**Files:**
- Modify: `code/digimon-engine/src/game_actions/mod.rs::run_state_based_rules_check` — add a breeding-area pass (non-Digimon top not placed there by an effect → trash) and a face-down top check (→ trash); collect ALL targets (incl. Task 3's link targets) first; apply all trashes and ONE `delete_permanents_batch(all_zero_dp, ReplacementCause::Rule)` inside one `enter_deferred_drain` scope; `exit_deferred_drain_and_flush` once. Keep the parked-deletion resume guard (`G-ENGINE-RULES-CHECK-CONTINUES-PAST-PARKED-DELETION`).
- Modify: `code/digimon-engine/src/permanent.rs` — a `placed_by_effect: bool` marker on breeding-area permanents (set by the effect-placement paths in `game_actions/breeding.rs`; false for hatching).
- Tests to un-ignore: `ch17_rule_checks.rs::non_digimon_in_breeding_area_is_trashed`, `::face_down_card_on_field_is_trashed`, `::all_rule_check_targets_processed_simultaneously`.
- Keep green: `ch17_rule_checks` (all), `--test deletion_batching`, `--test combat`, `--test cards_behavioral -- breeding face_down rule_check` (filter), `--test rules_conformance`.

**Interfaces:**
- Consumes: Task 3's `collect_link_rule_targets`.
- Produces: `run_state_based_rules_check` with the shape: `collect → apply trashes → one deletion batch → one flush`; returns `true` when anything was processed (unchanged contract).

- [ ] **Step 1: Un-ignore the three; run → FAIL.**
- [ ] **Step 2: Implement** the two new targets and the single-scope sweep.
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `17-1-3`, `17-1-3-2-3`, `17-1-3-2-4` → proven. Regenerate; `--check`. Both gap entries RESOLVED.
- [ ] **Step 5: Commit** — `engine: rule check trashes breeding-area non-Digimon and face-down tops, processes all targets simultaneously (§17-1-3, §17-1-3-2-3, §17-1-3-2-4; G-ENGINE-RULE-CHECK-TARGETS-INCOMPLETE, G-ENGINE-RULE-CHECK-TARGETS-NOT-SIMULTANEOUS)`.

### Task 5: Deletion batch: `[On Deletion]` and deletion observers in one pool; simultaneous `<Overflow>` turn player first

**Gaps:** `G-ENGINE-ON-DELETION-DRAINED-BEFORE-DELETION-OBSERVERS`, `G-ENGINE-OVERFLOW-SIMULTANEOUS-ORDER-PER-PERMANENT`. **Rules:** §16-13-2 (with §15-4-3-2, §15-4-3-5); §4-19-5, §4-19-5-1, §4-19-5-2, §4-19-2.

**Files:**
- Modify: `code/digimon-engine/src/combat/deletion.rs` — `complete_deletion_batch_after_replacements` (~L195): enqueue stage 4a `OnDeletion` AND stage 6 `OnAnyDeletion` / `OnLeaveField` (today `drain_batch_on_any_deletion` ~L842) before the single `exit_deferred_drain_and_flush`; `resume_pending_deletion` (~L958) no longer needs to continue into stage 6. `trash_batch_survivors` (~L581) / `trash_single_for_batch` (~L628): stop applying `<Overflow>` per permanent; collect every leaving card's handle and call `apply_ace_overflow_for_sources` once after all removals, before the flush.
- Modify: `code/digimon-engine/src/game/mod.rs::apply_ace_overflow_for_sources` (~L1640) only if its signature needs the batch (it already sorts turn player first within one call).
- Tests to un-ignore: `ch16_attack_keywords.rs::retaliation_kind_matches_rule`; `ch16_new_keywords.rs::overflow_turn_player_orders_then_non_turn_player`.
- Keep green: `--test deletion_batching`, `--test ace_overflow`, `--test combat` (`deletion_cause_observer`), `--test rules_conformance` (esp. `ch16_replacement_keywords`), `--test cards_behavioral -- on_deletion retaliation overflow ace` (filter).

**Interfaces:**
- Produces: one flush per deletion batch; snapshot threading (`deleted_object_snapshot`) preserved for both stage kinds (CLAUDE.md rule 25).

- [ ] **Step 1: Un-ignore both; run → FAIL.**
- [ ] **Step 2: Implement** both changes.
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `16-13-2`, `4-19-5`, `4-19-5-1`, `4-19-5-2` → proven. Regenerate; `--check`. Both entries RESOLVED.
- [ ] **Step 5: Commit** — `engine: deletion batch pools [On Deletion] with deletion observers; simultaneous <Overflow> turn player first (§16-13-2, §4-19-5; G-ENGINE-ON-DELETION-DRAINED-BEFORE-DELETION-OBSERVERS, G-ENGINE-OVERFLOW-SIMULTANEOUS-ORDER-PER-PERMANENT)`.

### Task 6: Source-trash observers wait for the causing effect (owner rulings 1–6; #726 rework)

**Gap:** `G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS`. **Rules:** §15-8-3-2 (rule 1), §15-4-3-2/-5 (rule 2), §15-4-5-2 (rule 3), §15-5-2 (rule 4: once per condition at a time), §15-4-2-4 (rule 5: again per new event), §16-12-7.

**Files:**
- Modify: `code/digimon-engine/src/game_actions/mod.rs` — `fire_digivolution_card_trashed` (~L2096): `drain_effect_queue()` → `maybe_drain_effect_queue()`; ensure every source-trash path (De-Digivolve, Digi-Burst / cost trashes, bottom-source strips, armor purge, the batch path ~L3031) runs inside a deferred-drain scope while the trashing effect's body is still executing, so the observers resolve after it (rule 1). Note: PR #726's commit `0eb9f6587` is already an ancestor of this branch; read `git show 0eb9f6587` for what landed and what was reverted.
- Modify: `code/digimon-engine/src/effect_queue.rs` — coalescing on enqueue: one `OnDigivolutionCardTrashed` trigger per (observer, event) where one event is one effect step trashing N cards at once (rule 4: `Vec<CardHandle>` payload), and a separate trigger per distinct event (rule 5). Mirror the same rule for the other "when cards are trashed" observer timings touched by the same paths.
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs` — drop the `G-DSL-TAIL-CLOBBERS-INLINE-OBSERVER-SELECTION` park once nothing drains inline.
- Tests to un-ignore: `ch15_08_effect_categories.rs::trigger_type_waits_for_causing_effect_to_finish`; `ch16_processing_keywords.rs::de_digivolve_separate_instances_processed_separately`.
- Existing tests encoding the inline order: `cards_behavioral` `ex10_036_clause_a_after_source_trash_prompts_opp_field_delete` (re-derive from DCGO `EX10_036.cs` + the rules: the observer resolves after Magneticdramon's effect finishes; rewrite the expectation with a `§15-8-3-2` comment), the dsl Digi-Burst test the memory names (`--test dsl`), `bt26_075 decline_ascension_if_pending` tolerance. Keep green: `--test cards_behavioral -- ex10 bt26_07 st23 bt25_035 rocks mineral` (filter), `--test dsl`, `--test rules_conformance`, `--test judge_quiz`.

**Interfaces:**
- Produces: `TriggerSource::SourceTrashedFromStack { player, host, host_card, cards: Vec<CardHandle>, cause }` (plural payload) or an equivalent coalesced form; `fire_digivolution_card_trashed` never drains inline.

- [ ] **Step 1: Un-ignore both; run → FAIL.**
- [ ] **Step 2: Implement** rules 1, 4, 5 as above; keep rules 2 and 3 (already correct per the memory) intact.
- [ ] **Step 3: Run → PASS**; update the named wrong-order tests; regression list.
- [ ] **Step 4: Sidecar:** `15-8-3-2`, `16-12-7` → proven. Regenerate; `--check`. Gap entry RESOLVED (note that the 12 Glowing Dawn exams in `qa/dcgo-exams/ST23`, `BT25` need re-scripting + an oracle re-run; list them in the entry, do not run them).
- [ ] **Step 5: Commit** — `engine: source-trash observers wait for the causing effect, once per event (§15-8-3-2, §15-5-2, §15-4-2-4; G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS)`.

### Task 7: A used Option is trashed immediately and uninterrupted after its first `[Main]`

**Gaps:** `F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE` (open half), `G-ENGINE-OPTION-TRASH-NO-EVENT` (non-blocking, same site). **Rule:** §9-1-5.

**Files:**
- Modify: `code/digimon-engine/src/game_actions/options.rs` / `game_actions/mod.rs` — `dispose_option` (~L1558), `commit_option_trash_outcome` (~L1766): dispose in the step that finishes the `[Main]` body, before `rules_check_between_queued_effects` and before any trigger drain; emit `GameEvent::Trash { player, card_id, card_name }` (and the matching move event on hand/deck redirects).
- Modify: `code/digimon-engine/src/effect_queue.rs` — remove `install_option_trash_order_selection` (~L5672) and every `option_trash_order_deferred` read/write (`game/mod.rs` ~L853, `game/setup.rs` ~L264/441, `effect_queue.rs` ~L3631/5600/5767).
- Modify: `code/digimon-engine/tests/rules_conformance/support.rs` — delete `trash_used_option_first` (~L169) and `settle_after_option` (~L186) and their call sites (replace with the plain settle helper).
- Tests to un-ignore: `ch18_pending_overwrite.rs::used_option_trashed_immediately_after_first_main_uninterrupted`; then switch it to event-order assertions (the gap entry asks for this) without weakening it.
- Existing tests encoding the orderable trash: those added for `G-ENGINE-OPTION-TRASH-TURN-PLAYER-ORDER` (`5aef07fa8`; grep `Trash the used Option` / `option_trash_order`): invert with a `§9-1-5` comment. Keep green: `--test option_flow` (if present), `--test cards_behavioral -- option ex7_073 delay` (filter), `--test rules_conformance`, `--test judge_quiz`.

- [ ] **Step 1: Un-ignore; run → FAIL.**
- [ ] **Step 2: Implement**; delete the deferred-order machinery and the support workarounds.
- [ ] **Step 3: Run → PASS**; invert the named tests; regression list.
- [ ] **Step 4: Sidecar:** `9-1-5` → proven. Regenerate; `--check`. Both entries RESOLVED.
- [ ] **Step 5: Phase A close:** full `cards_behavioral` run (rule 33 invocation). Fix regressions. Record the counts in the report.
- [ ] **Step 6: Commit** — `engine: used Option trashed immediately after its first [Main], uninterrupted, with a Trash event (§9-1-5; F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE, G-ENGINE-OPTION-TRASH-NO-EVENT)`.

---

## Phase B — Optional-condition legality (§15-7-4, §15-15-7-4)

### Task 8: An unpayable optional processing condition can't be chosen

**Gap:** `G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED`. **Rules:** §15-7-4 (reversed in 4.3), §15-8-4-4-1, §15-7-2; owner ruling 2026-10-10 #6.

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/lower_triggered.rs::first_step_candidate_guard` (~L664): extend beyond `select_*` to every cost-shaped leading step — `trash_top_security` ⇒ `security.len() >= n`; `lose_memory` / `pay_memory` ⇒ payable per the memory rules; `suspend` (self or chosen) ⇒ `ActivationCostKind::is_payable`; hand / source / Tamer-card trashes ⇒ enough material (count, not boolean; Review Focus 4); `place_under_tamer` ⇒ a Tamer exists; `return_to_hand` ⇒ the material exists. Returned as a `Guard` closure/data ANDed into `outer_optional_guard`.
- Modify: `code/digimon-engine/src/effect_queue.rs::needs_pre_cost_prompt` (~L1577): also consult `ActivationCostKind::is_payable`; `queued_effect_wants_outer_optional_prompt` (~L4692) honours the extended guard.
- Modify: `code/digimon-engine/src/action/main_effect_select.rs::field_main_match` (~L74): install the same guard for `main_on_field` clauses so an unpayable `[Main]` is masked.
- Modify: production YAML: retire the hand-authored gates the gap entry names (`BT22-041` `security_count_gte: 1`, `BT25-084` `active_when: count_gte hand 2`) ONLY if their cards_behavioral tests stay green without them; otherwise leave and list in the report.
- Tests to un-ignore: `ch15_07_optional_conditions.rs::unpayable_optional_condition_cannot_be_chosen`. Keep green: `ch15_08_effect_categories::activation_type_declarable_only_when_optional_condition_payable`, `--test cards_behavioral -- bt22_041 bt25_084 bt25_049 bt25_090 st23_03 bt25_057 bt18_019` (filter), `--test rules_conformance`, `--test judge_quiz`, `--test rules_faq`.

**Interfaces:**
- Produces: `pub(crate) fn leading_cost_guard(steps: &[CompiledStep]) -> Option<CostGuard>` in `lower_triggered.rs`, where `CostGuard` is evaluated by `Game::cost_guard_holds(&self, guard: &CostGuard, source: CardHandle, controller: PlayerId) -> bool`; used by the three sites above. Task 9 consumes `Game::install_single_trigger_optional_gate` (see below) which calls it.

- [ ] **Step 1: Un-ignore; run → FAIL** (trigger half: prompt parked at 0 security; `[Main]` half: in the mask at 0 security).
- [ ] **Step 2: Implement** the per-step-kind guard and its three consumers.
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `15-7-4`, `15-8-4-4-1` → proven. Regenerate; `--check`. Entry RESOLVED. Add a line to `qa/dsl-vocab-gaps.md` under `G-DSL-OPTIONAL-PROCESSING-CONDITION-BLOCK`: the engine-side interim guard landed; the `by:` block remains the long-term vocabulary.
- [ ] **Step 5: Commit** — `engine: unpayable optional processing conditions are not offered or declarable (§15-7-4, §15-8-4-4-1; G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED)`.

### Task 9: A refired effect keeps its own optional decision

**Gap:** `G-ENGINE-REFIRED-EFFECT-SKIPS-OPTIONAL-DECISION`. **Rule:** §15-15-7-4.

**Files:**
- Modify: `code/digimon-engine/src/effect_queue.rs` — factor the drain's single-trigger pre-cost / outer-optional install (~L1577–1700) into `pub(crate) fn install_single_trigger_optional_gate(&mut self, effect: &QueuedEffect) -> OptionalGate` where `OptionalGate::{NotNeeded, Parked}`; `run_refired_effect` (~L236) calls it and, on `Parked`, resumes into `run_queued_effect` via a `ResumeFrame::RefiredEffectAccepted { effect }` after the accept (declines drop the effect). Keep `bypass_once_per_turn` and attribution stamping.
- Modify: `code/digimon-engine/src/effect_context/action/refire.rs::dispatch_refireable_effects` (~L173) and `install_refire_effect_selection` only to thread the new gate.
- Tests to un-ignore: `ch15_07_optional_conditions.rs::mandatory_invoker_does_not_force_invoked_optional_condition`. Keep green: `--test cards_behavioral -- bt15_102 bt23_060 refire` (filter), `--test rules_conformance`.

- [ ] **Step 1: Un-ignore; run → FAIL** (no prompt; security trashed, memory gained).
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `15-15-7-4` → proven. Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Phase B close:** full `cards_behavioral` run; fix regressions.
- [ ] **Step 6: Commit** — `engine: refired effects pass through the optional-condition gate (§15-15-7-4; G-ENGINE-REFIRED-EFFECT-SKIPS-OPTIONAL-DECISION)`.

---

## Phase C — Attack procedure (§11, §16-16, §16-30, §16-33)

### Task 10: The counter timing occurs even when the attacker has left

**Gap:** `G-ENGINE-ATTACK-TIMINGS-SKIPPED-WHEN-ATTACKER-LEAVES`. **Rules:** §11-1-5, §12-1-6.

**Files:**
- Modify: `code/digimon-engine/src/combat/mod.rs` — `commit_attack_declaration` (~L4168), `advance_pending_attack` (~L710: the `!handle_still_attacking` branches), `try_enter_counter` (~L1529: read `source_card` from the stored `PendingAttack` context, never from `battle_area[attacker.index]` once the attacker is gone), `try_enter_block` (~L2188: no block window when the attacker is gone, §12-1-6), then straight to `cleanup_attack` with an "unsuccessful" outcome (not `Invalid`; reuse the §11-5-1-4-1 outcome the green `unsuccessful_attack_reaches_end_of_attack_timing` test exercises). `cancelled` attacks keep their current path.
- Add to `PendingAttack`: `attacker_card: CardHandle` captured at declaration if not already present.
- Tests to un-ignore: `ch11_14_attack_procedure.rs::attack_timings_continue_when_attacker_leaves_mid_resolution` (`vortex_attack_has_block_timing` also lists this gap but needs Task 13; leave it ignored here).
- Keep green: `--test combat` (all; `counter_interrupt`, `redirect_and_cancel`, `would_attack_replacements`), `--test rules_conformance`, `--test judge_quiz`, `--test cards_behavioral -- when_attacking counter` (filter).

- [ ] **Step 1: Un-ignore; run → FAIL** (no decision for P1).
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `11-1-5` → proven only if `vortex_attack_has_block_timing` is not listed for it; otherwise leave `gap` with a note "half fixed; Task 13". Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: counter timing occurs after the attacker leaves; block timing offers no block (§11-1-5, §12-1-6; G-ENGINE-ATTACK-TIMINGS-SKIPPED-WHEN-ATTACKER-LEAVES)`.

### Task 11: A removed attack target stays the target and the attack fails

**Gap:** `G-ENGINE-REMOVED-ATTACK-TARGET-SLOT-REUSED`. **Rules:** §11-2-6, §11-5-1-4-1.

**Files:**
- Modify: `code/digimon-engine/src/combat/mod.rs` — rename `PendingAttack::effective_target_removed_by_dna` → `effective_target_removed: bool` (every reader: `combat/mod.rs` ~L1156/1185/2241/2503/2720/2933/4408, `combat/deletion.rs::live_battle_opponent_of` ~L786, `effect_context/mod.rs` ~L740/1509, `effect_context/action/combat.rs` ~L177, `game/mod.rs` ~L2683/4266/4309); set it in `shift_pending_attack_after_battle_area_remove` (~L4131) whenever the removed slot is the effective target; `resolve_pending_battle` (~L2917) treats it as "attack fails"; `try_enter_block` excludes it (§12-1-5); the Raid retarget (Task 1's body) clears it on a successful retarget to a live Digimon; audit `PendingAttack::blocker` the same way (a removed blocker → battle does not occur, attack fails).
- Tests to un-ignore: `ch11_14_attack_procedure.rs::removed_target_stays_the_target_and_attack_fails`.
- Keep green: `--test combat` (all), `--test rules_conformance` (incl. `unsuccessful_attack_reaches_end_of_attack_timing`, Task 10's test), `--test cards_behavioral -- when_attacking delete blocker` (filter), `--test judge_quiz`.

- [ ] **Step 1: Un-ignore; run → FAIL** (the shifted Digimon is battled).
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; regression list; run Task 10's test together with this one (Review Focus 3).
- [ ] **Step 4: Sidecar:** `11-2-6` → proven. Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: a removed attack target remains the target and the attack fails (§11-2-6; G-ENGINE-REMOVED-ATTACK-TARGET-SLOT-REUSED)`.

### Task 12: The attack does not end until its end-of-attack processing resolves

**Gaps:** `G-ENGINE-ATTACK-ENDS-BEFORE-END-OF-ATTACK-EFFECTS-RESOLVE`, `G-ENGINE-ENGAGE-EOT-PASS-ONLY-STEP` (non-blocking, same resume path). **Rules:** §11-6-2, §16-38-1, §16-38-4, §16-44-1.

**Files:**
- Modify: `code/digimon-engine/src/combat/mod.rs::cleanup_attack` (~L3948): split at the `EndOfAttack` drain. Add `AttackState::EndOfAttackDrain`; on a park, keep `pending_attack` and return `InProgress`; the post-selection resume (`advance_pending_attack`) finishes the drain (incl. the scheduled `EndOfAttack` batch, which Task 2 already pools), THEN `expire_end_of_attack()`, `pending_attack = None`, `check_turn_end`. Do not clear `is_attacking` before the `EndOfAttack` enqueue (the entry notes it is cleared early today; `current_attacker()` must stay valid through the drain).
- Modify: `code/digimon-engine/src/game_phases.rs` — after an end-of-turn attack's cleanup, re-check `has_end_of_turn_keywords(turn_player)` and rotate the turn when false instead of parking in `EndOfTurnAction` with PASS only.
- Tests to un-ignore: `ch11_14_attack_procedure.rs::attack_does_not_end_until_all_processing_resolved`; `ch16_attack_keywords.rs::execute_optional_eot_attack_then_pending_deletion_at_end_of_attack`.
- Existing test to update: `ch16_new_keywords::engage_optional_eot_attack` asserts the PASS-only step via `finish_end_of_turn`; update it to the no-extra-step shape with a `§16-44-1` comment. Keep green: `--test combat` (all), `--test keyword_phase_f` (if present; `execute.rs`), `--test rules_conformance`, `--test cards_behavioral -- end_of_attack progress execute engage` (filter).

- [ ] **Step 1: Un-ignore both; run → FAIL.**
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; update the named test; regression list.
- [ ] **Step 4: Sidecar:** `11-6-2`, `16-38-1`, `16-38-4` → proven. Regenerate; `--check`. Both entries RESOLVED.
- [ ] **Step 5: Commit** — `engine: the attack ends only after end-of-attack processing resolves (§11-6-2, §16-38-4; G-ENGINE-ATTACK-ENDS-BEFORE-END-OF-ATTACK-EFFECTS-RESOLVE, G-ENGINE-ENGAGE-EOT-PASS-ONLY-STEP)`.

### Task 13: `<Vortex>` attacks run the full procedure and target only legal Digimon

**Gaps:** `G-ENGINE-VORTEX-ATTACK-SKIPS-COUNTER-AND-BLOCK`, `G-ENGINE-VORTEX-ATTACKS-UNSUSPENDED-DIGIMON`. **Rules:** §11-4-1, §11-1-5, §11-2-7-1, §16-33-1 (+Q3869; W2 reading (a)).

**Files:**
- Modify: `code/digimon-engine/src/combat/mod.rs::begin_attack_open` (~L502–700): drop the `if vortex { transition_attack_state(AttackState::Battle) }` short-circuit; `AttackInitiator::Vortex` keeps only the played-this-turn exemption. Fix the module doc claim (L10–20).
- Modify: `code/digimon-engine/src/action/mask.rs` end-of-turn branch (~L596–680) and `action/decode.rs::end_of_turn_attack_vortex_flag` (~L308): Vortex-only attacks apply the normal target legality (suspended, or `CanAttackUnsuspended`, or a <Raid>-style permission).
- Existing test to invert: `tests/combat/counter_interrupt.rs::vortex_bypasses_counter` (~L376) → `vortex_attack_has_counter_timing` with a `§11-1-5 / Q3869` comment.
- Tests to un-ignore: `ch16_attack_keywords.rs::vortex_attack_has_block_timing`, `::vortex_attack_target_is_a_suspended_digimon`.
- Keep green: `--test combat` (all), `--test rules_conformance`, `--test cards_behavioral -- vortex ex7_064` (filter), `--test tensor_and_mask` (if present), `code/tests/engine` mask parity pytests if the mask bit set changes (`python -m pytest code/tests/engine -k "mask or vortex" -q`).

- [ ] **Step 1: Un-ignore both; run → FAIL.**
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; invert the named test; regression list. If Task 10's `11-1-5` stayed `gap`, flip it now.
- [ ] **Step 4: Sidecar:** `11-4-1`, `11-2-7-1`, `16-33-1` → proven (16-33-1 only if `vortex_kind_matches_rule` is not listed for it; otherwise leave `gap` until Task 21). Regenerate; `--check`. Both entries RESOLVED.
- [ ] **Step 5: Commit** — `engine: <Vortex> attacks have counter and block timings and legal targets only (§11-4-1, §11-2-7-1, §16-33-1; G-ENGINE-VORTEX-ATTACK-SKIPS-COUNTER-AND-BLOCK, G-ENGINE-VORTEX-ATTACKS-UNSUSPENDED-DIGIMON)`.

### Task 14: `<Collision>` grants `<Blocker>` declaratively

**Gap:** `G-ENGINE-COLLISION-BLOCKER-NOT-GRANTED`. **Rules:** §16-30-1, §16-30-2, §16-30-4.

**Files:**
- Modify: `code/digimon-engine/src/game/queries.rs::has_keyword` (~L52) (or the declarative keyword materialisation it reads): while the current attacker has `Keyword::Collision`, `has_keyword(h, Blocker)` is true for every opponent Digimon `h` for which `permanent_is_unaffected_by_effect(h, attacker.player, EffectSourceKind::Digimon)` is false.
- Modify: `code/digimon-engine/src/combat/mod.rs::try_enter_block` (~L2188–2260): remove the Collision candidacy special case; keep `is_optional = !attacker_has_collision`.
- Tests to un-ignore: `ch16_attack_keywords.rs::collision_kind_matches_rule`, `::collision_blocker_grant_affects_digimon_forced_block_affects_player`.
- Keep green: `collision_forces_block_whenever_possible`, `collision_timing_matches_rule`, `--test combat` (`collision_mandatory`, `block_interrupt`), `--test cards_behavioral -- collision bt8_095 blocker` (filter).

- [ ] **Step 1: Un-ignore both; run → FAIL.**
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `16-30-1`, `16-30-2`, `16-30-4` → proven. Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: <Collision> grants <Blocker> to affected opponent Digimon (§16-30-1, §16-30-4; G-ENGINE-COLLISION-BLOCKER-NOT-GRANTED)`.

### Task 15: `<Blitz>` offers its attack before the turn can end

**Gap:** `G-ENGINE-BLITZ-TURN-ENDS-BEFORE-ATTACK`. **Rules:** §16-16-1..5 (+Q602).

**Files:**
- Modify: `code/digimon-engine/src/cards/keyword_effects.rs::keyword_to_auto_effect_inner` — a `Keyword::Blitz` body (when granted by a `[When Digivolving]`-style trigger, the DSL idiom EX3-063 uses): gate "opponent's memory ≥ 1 at activation" (§16-16-3/-5); processing = an optional attack declaration as its own resumable decision (`ResumeFrame::BlitzAttackDecision { attacker }`): legal actions in that window are ONLY the <Blitz> Digimon's attacks (player or legal Digimon targets) and the decline; the attack proceeds through the normal procedure even if memory returns to ≥ 0 (§16-16-4); `check_turn_end` is deferred to the attack's cleanup (Task 12's path) or to the decline.
- Modify: `code/digimon-engine/src/game_actions/digivolve.rs` (~L1945 `check_turn_end` after the drain) — do not end the turn while a Blitz decision is parked.
- Modify: `code/digimon-engine/src/action/mask.rs` (~L198–205): retire the main-phase `digivolved_this_turn && has_keyword(Blitz)` carve-out; add the Blitz-window mask (attacks of the carrier + PASS/decline only). `action/decode.rs` decodes the window.
- Modify: `code/digimon-engine/cards/ex3/EX3-063.yaml` to the new shape only if its lowering needs a change; its `cards_behavioral` test must stay green or be updated to the offered-attack shape with a `§16-16` comment.
- Tests to un-ignore: `ch16_attack_keywords.rs::blitz_optionality_matches_rule`, `::blitz_kind_matches_rule`, `::blitz_attack_proceeds_after_memory_returns_to_zero_or_more`, `::blitz_cannot_attack_when_memory_already_zero_or_more`.
- Keep green: `--test combat`, `--test rules_conformance`, `--test cards_behavioral -- blitz ex3_063` (filter), `--test tensor_and_mask` (if present), `python -m pytest code/tests/engine -k mask -q`.

- [ ] **Step 1: Un-ignore all four; run → FAIL** (turn passes; no prompt).
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `16-16-1`, `16-16-2`, `16-16-3`, `16-16-4`, `16-16-5` → proven. Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Phase C close:** full `cards_behavioral` run; fix regressions.
- [ ] **Step 6: Commit** — `engine: <Blitz> offers its attack as a resumable decision before the turn ends (§16-16; G-ENGINE-BLITZ-TURN-ENDS-BEFORE-ATTACK)`.

---

## Phase D — Keyword implementations (§16-10, §16-21, §16-31, §16-12, §4-6-6-2, §16-33/-34/-38/-44)

Tasks 16–20 touch disjoint files and may run in parallel sessions (each on its own branch from the Phase C head, merged back by the controller); Task 21 runs after Task 15 and Task 13 on this branch.

### Task 16: `<Digisorption -N>` implemented as a before-pay-cost optional reducer

**Gap:** `G-ENGINE-KEYWORD-DIGISORPTION-UNIMPLEMENTED`. **Rules:** §16-10-1..5.

**Files:**
- Modify: `code/digimon-engine/src/enums.rs` — `Keyword::Digisorption(u8)` (~L528 enum; Display/serde arms).
- Modify: `code/digimon-engine/src/card_data.rs::parse_printed_keywords` (~L650): `<Digisorption -N>` (full-width `＜…＞` too).
- Modify: `code/digimon-dsl/src/validator.rs` `KNOWN_KEYWORD_KEYS` (~L3050) and `code/digimon-engine/src/dsl_cards/modifier_map.rs` keyword lookup (~L404): `Digisorption` with value.
- Modify: `code/digimon-engine/src/game_actions/digivolve.rs` (the from-hand cost path, ~L873–1000) and/or `player_cost_reducer.rs`: for a digivolution from the hand INTO a card with `Digisorption(n)` instances, before paying, one optional prompt per instance: pick 1 own unsuspended Digimon that can suspend (the base included, §16-10-5); on accept suspend it and reduce the cost by n (floor 0, §16-10-1); instances stack (§16-10-4). Use the existing reducer prompt loop (`G-COST-REDUCTION-OPTIONAL-REDUCER-RESIDUAL` shape) and the resumable VM.
- Regenerate the DSL schema: `cargo run -p dsl-schema-export`; run `cargo run -p dsl-lint -- code/digimon-engine/cards`.
- Tests to un-ignore: `ch16_processing_keywords.rs::digisorption_optional_suspend_reduces_cost_floors_at_zero_and_stacks`, `::digisorption_kind_matches_rule`, `::digisorption_timing_matches_rule`, `::digisorption_optionality_matches_rule`.
- Keep green: `--test digivolve_cost_choice`, `--test cost_hooks`, `--test dsl`, `--test rules_conformance`, `--test cards_behavioral -- cost_reduction p_200 bt5_092` (filter).
- Out of scope: authoring the 12 printings (`BT2-045, BT2-047, BT2-050, BT3-054, BT3-056, BT5-049, BT5-058, BT5-100, BT8-054, BT10-052, EX3-043, P-056`); list them in the gap entry's Resolution line as the follow-up.

- [ ] **Step 1: Un-ignore all four; run → FAIL** (full cost, no offer).
- [ ] **Step 2: Implement** keyword, parsing, DSL vocabulary, reducer.
- [ ] **Step 3: Run → PASS**; regression list; schema regenerated and committed.
- [ ] **Step 4: Sidecar:** `16-10-1`, `16-10-2`, `16-10-3`, `16-10-4`, `16-10-5` → proven. Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: <Digisorption -N> as an optional before-pay-cost reducer (§16-10; G-ENGINE-KEYWORD-DIGISORPTION-UNIMPLEMENTED)`.

### Task 17: `<Material Save>` is an immediate-type effect that places the full number

**Gaps:** `G-ENGINE-MATERIAL-SAVE-AFTER-DELETION`, `G-ENGINE-MATERIAL-SAVE-MAY-PLACE-FEWER`. **Rules:** §16-21-2, §16-21-3, §15-8-5-2.

**Files:**
- Modify: `code/digimon-engine/src/cards/keyword_effects.rs` `Keyword::MaterialSave(n)` (~L1562): re-time from `Effect::on_deletion` to a non-cancelling `WhenWouldBeDeleted` immediate effect (the `<Partition>` shape at ~L1439: optional, offered while the carrier stands, picks from the live stack, no `cancel_leave`); the Tamer pick is the optional decision; the card pick is then `min = max = min(n, eligible)` (eligible = the top card's DigiXros-requirement cards among the digivolution cards).
- Tests to un-ignore: `ch16_processing_keywords.rs::material_save_kind_matches_rule`, `::material_save_places_the_full_number_whenever_possible`.
- Keep green: `material_save_optionality_matches_rule` and the other green Material Save tests in the same module, `--test rules_conformance` (esp. `ch16_replacement_keywords`), `--test cards_behavioral -- material_save bt11_018 xros` (filter), `--test deletion_batching`.

- [ ] **Step 1: Un-ignore both; run → FAIL.**
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `16-21-2`, `16-21-3` → proven. Regenerate; `--check`. Both entries RESOLVED.
- [ ] **Step 5: Commit** — `engine: <Material Save> is an immediate-type effect placing the full number (§16-21-2, §16-21-3; G-ENGINE-MATERIAL-SAVE-AFTER-DELETION, G-ENGINE-MATERIAL-SAVE-MAY-PLACE-FEWER)`.

### Task 18: `<Blast DNA Digivolve>` respects the DNA requirements; DUAL Option information in card searches

**Gaps:** `G-ENGINE-BLAST-DNA-IGNORES-DNA-REQUIREMENTS`, `G-ENGINE-DUAL-OPTION-INFO-NOT-REFERENCED-IN-CARD-SEARCH`. **Rules:** §16-31-5 (with §16-31-4, §8-2-1); §4-6-6-2 (with §4-6-5-2, §4-6-1).

**Ruling (W5, controller, provisional):** an unqualified reference ("purple card") matches a DUAL if EITHER information matches, because §4-6-1 lets the card be treated as either category and the reference picks neither. `kind: option` reads `dual.option` only; `kind: digimon` reads `dual.digimon` only. Pin the unqualified reading with a new test in `ch16_new_keywords.rs` (`dual_unqualified_colour_reference_matches_either_information`, citing §4-6-1 with `cite!`), added in this task (discover-then-pin: write it, run it, fix if red).

**Files:**
- Modify: `code/digimon-engine/src/dna_digivolve.rs::blast_dna_route_for_hand_card` (~L1048) / `registered_blast_dna_route_for_hand_card` (~L1124): on the registered path, also require that (field material, hand material) satisfy one of the result card's DNA requirements (`dna_digivolve` alt paths / `dna_costs`), in requirement order, before offering the route.
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs::eval_card_fields` (~L3089) and `kind_matches_card_search` (~L4827): with `kind: option` evaluate `color_is` / `color_only` / `use_cost` / `play_cost_*` / `level` from `dual.option`; with `kind: digimon` from `dual.digimon`; with no kind, either.
- Tests to un-ignore: `ch16_processing_keywords.rs::blast_dna_cannot_ignore_dna_requirements`; `ch16_new_keywords.rs::dual_option_information_referenced_as_option_card`. New test as above.
- Keep green: `blast_dna_kind_matches_rule`, `dual_use_declares_digimon_or_option_information`, `--test blast_dna_bonus_draw`, `--test blast_dna_simultaneous_observers`, `--test bt20_045_blast_dna_source_order`, `--test cards_behavioral -- blast_dna dual bt26_032 bt26_056 st24_07` (filter), `--test dsl`.

- [ ] **Step 1: Un-ignore both; write the new test; run → FAIL on the two, record the new test's result.**
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `16-31-5`, `4-6-6-2` → proven; add `4-6-1` with the new test (status `proven`). Regenerate; `--check`. Both entries RESOLVED; record the W5 ruling in `qa/rules/mechanics-gaps-2026-10.md` W5 ("Fixed on reading (a); owner may override").
- [ ] **Step 5: Commit** — `engine: Blast DNA checks DNA requirements; DUAL searches read the referenced information (§16-31-5, §4-6-6-2; G-ENGINE-BLAST-DNA-IGNORES-DNA-REQUIREMENTS, G-ENGINE-DUAL-OPTION-INFO-NOT-REFERENCED-IN-CARD-SEARCH)`.

### Task 19: `<De-Digivolve>`: declared count, top-card level floor, top-card-trashed timing

**Gaps:** `G-DEDIGIVOLVE-NO-DECLARATION`, `G-ENGINE-DEDIGIVOLVE-LEVEL-FLOOR-READS-NEXT-CARD`, `G-ENGINE-DEDIGIVOLVE-FIRES-DIGIVOLUTION-CARD-TRASHED` (+ `G-DSL-TOP-CARD-TRASHED-TIMING`). **Rules:** §16-12-1, §16-12-3, §16-12-4, §4-8-1, §16-12-5.

**Files:**
- Modify: `code/digimon-engine/src/game_actions/helpers.rs::de_digivolve_core` (~L270–360): (a) stop when the TOP card's level is ≤ 3 (or it has no level) or the stack has no digivolution cards; drop the next-card read; (b) when ≥ 2 cards are trashable, park a non-optional count selection (1..=min(N, trashable)) for the effect's controller via the existing `count_choice` VM (`CountCappedMultiSelect` / `count_choice.rs`), then trash that many; (c) fire the new timing instead of `fire_digivolution_card_trashed`.
- Modify: `code/digimon-engine/src/enums.rs` — `EffectTiming::OnTopCardTrashed`; `code/digimon-dsl` (`when: on_top_card_trashed`, validator + schema) and `code/digimon-engine/src/dsl_cards/timing_map.rs`; fire once per <De-Digivolve> instance with all trashed cards (`TriggerSource` payload `Vec<CardHandle>`), and from `effect_context/action/sources.rs::armor_purge_top` (~L756) for <Armor Purge>.
- Modify: `tests/rules_conformance/fixtures/ch16_proc/OBS_TOP_TRASH_MEM.yaml` → authored on `on_top_card_trashed` (the gap entry asks for it); re-verify the §16-12-5 / §16-12-7 tests stay green.
- Regenerate the DSL schema; `dsl-lint`.
- Tests to un-ignore: `ch16_processing_keywords.rs::de_digivolve_player_declares_count_mandatory_at_least_one_stops_at_level_3`, `::de_digivolve_cannot_trash_level_3_or_lower`, `::de_digivolve_trashes_top_cards_not_digivolution_cards`.
- Keep green: the other `de_digivolve_*` tests, `--test cards_behavioral -- de_digivolve dedigivolve armor_purge rocks mineral bt21_094 bt8_110 ex10_010` (filter), `--test dsl`, `--test rules_conformance`.
- Audit (report only, no edits unless a test proves it): cards authored on `on_digivolution_card_trashed` that DCGO fires on `WhenTopCardTrashed` instead (the Rocks / Mineral line); list them for a follow-up.

- [ ] **Step 1: Un-ignore the three; run → FAIL.**
- [ ] **Step 2: Implement** (a), (b), (c) and the DSL timing.
- [ ] **Step 3: Run → PASS**; regression list; schema committed.
- [ ] **Step 4: Sidecar:** `16-12-1`, `16-12-3`, `16-12-4` → proven. Regenerate; `--check`. Three entries RESOLVED; `qa/dsl-vocab-gaps.md` `G-DSL-TOP-CARD-TRASHED-TIMING` → RESOLVED.
- [ ] **Step 5: Commit** — `engine: <De-Digivolve> declares its count, stops at a level 3 top, fires on_top_card_trashed (§16-12-1, §16-12-4, §4-8-1; G-DEDIGIVOLVE-NO-DECLARATION, G-ENGINE-DEDIGIVOLVE-LEVEL-FLOOR-READS-NEXT-CARD, G-ENGINE-DEDIGIVOLVE-FIRES-DIGIVOLUTION-CARD-TRASHED)`.

### Task 20: (reserved) `<Mind Link>` placement mandatory and parametric keyword references — non-blocking, batched

**Gaps:** `G-ENGINE-MIND-LINK-PLACEMENT-OPTIONAL`, `G-ENGINE-KEYWORD-REFERENCE-MATCHES-ONE-VALUE` (both non-blocking; included because each is a one-site fix with a waiting test). **Rules:** §16-28-3, §16-2.

**Files:**
- Modify: `code/digimon-engine/src/cards/keyword_effects.rs` `Keyword::MindLink` (~L2118): `is_optional = false`.
- Modify: `code/digimon-engine/src/enums.rs` — `Keyword::same_type(&self, other: &Keyword) -> bool` (discriminant equality); `code/digimon-engine/src/game/queries.rs` — `Game::has_keyword_type(handle, kind: &Keyword) -> bool` scanning face / inherited / link / granted keywords by type; `code/digimon-engine/src/dsl_cards/predicate.rs::has_keyword` (~L4297) uses it when the DSL reference carries no parameter.
- Tests to un-ignore: `ch16_processing_keywords.rs::mind_link_places_tamer_under_tamerless_digimon_mandatory`; `ch16_replacement_keywords.rs::keyword_reference_matches_every_value_of_the_keyword`.
- Keep green: `--test cards_behavioral -- mind_link has_keyword fragment decoy` (filter), `--test rules_conformance`.

- [ ] **Step 1: Un-ignore both; run → FAIL.**
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `16-28-3`, `16-2` → proven. Regenerate; `--check`. Both entries RESOLVED.
- [ ] **Step 5: Commit** — `engine: <Mind Link> placement mandatory; keyword references match by type (§16-28-3, §16-2; G-ENGINE-MIND-LINK-PLACEMENT-OPTIONAL, G-ENGINE-KEYWORD-REFERENCE-MATCHES-ONE-VALUE)`.

### Task 21: End-of-turn attack keywords declare their attack inside the drain (Engage, Execute, Vortex, Overclock)

**Gap:** `G-ENGINE-ENGAGE-ATTACK-AFTER-END-OF-TURN-TRIGGERS`. **Rules:** §16-44-2, §16-38-2, §16-33-2, §16-34-2, §11-1-4 (+Q3868/Q3869; W3 reading (a)). Depends on Tasks 12, 13, 15 (the resumable attack-declaration decision and the full Vortex procedure).

**Files:**
- Modify: `code/digimon-engine/src/cards/keyword_effects.rs` — `Keyword::Engage` (~L2002), `Keyword::Execute` (~L1853): the body offers the attack (or declines) as its own resumable decision inside the drain (reuse Task 15's `BlitzAttackDecision` frame generalised to `AttackDeclarationDecision { attacker, allow_unsuspended_targets, initiator }`), instead of granting `MayAttack`; add `Keyword::Vortex` and `Keyword::Overclock(..)` as `Effect::end_of_your_turn` trigger-type bodies (Vortex: optional attack on a legal Digimon target, same-turn exemption; Overclock: optional condition "delete 1 of your specified Digimon/Token" then the attack on a player without suspending).
- Modify: `code/digimon-engine/src/game_phases.rs` — `continue_end_turn_after_delays` / `continue_end_turn_after_effects` (~L416–470), `has_end_of_turn_keywords` (~L619), `activate_overclock` (~L819): after the declaration decision, the remaining `[End of Your Turn]` pool entries resolve, THEN the attack continues to its counter timing (`advance_pending_attack` is resumed after the drain empties); retire the post-drain `EndOfTurnAction` offers for these four keywords (keep the phase for any `MayAttack` effect grants that still use it).
- Modify: `code/digimon-engine/src/action/mask.rs` / `decode.rs` end-of-turn branch for the retired bits; `code/tests/engine` mask parity if affected.
- Tests to un-ignore: `ch16_new_keywords.rs::engage_kind_matches_rule`; `ch16_attack_keywords.rs::execute_kind_matches_rule`, `::vortex_kind_matches_rule`, `::overclock_kind_matches_rule`.
- Existing tests to update (the gap entry's "Test shape vs. the fix"): the `engage_*`, `vortex_*`, `execute_*`, `overclock_*` tests in `ch16_new_keywords` / `ch16_attack_keywords` that treat decline as PASS via `settle_declining`: accept the activation where they expect the attack offered; decline it (instead of PASS) in the optionality tests. `tests/combat/overclock.rs`, `group6_overclock.rs`, `group6_keywords.rs` and `keyword_phase_f/execute.rs` to the new offer shape with `§16-3x-2` comments.
- Keep green: `--test combat` (all), `--test rules_conformance`, `--test cards_behavioral -- engage execute vortex overclock ex7_064` (filter), `--test judge_quiz`, `--test rules_faq`.

- [ ] **Step 1: Un-ignore the four; run → FAIL.**
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; update the named tests; regression list.
- [ ] **Step 4: Sidecar:** `16-44-2`, `16-38-2`, `16-33-2`, `16-34-2` (and `16-33-1` if Task 13 left it) → proven. Regenerate; `--check`. Entry RESOLVED; W3 marked "fixed on reading (a)".
- [ ] **Step 5: Phase D close:** full `cards_behavioral` run; fix regressions.
- [ ] **Step 6: Commit** — `engine: end-of-turn attack keywords declare inside the trigger pool, other effects resolve before the counter timing (§16-33-2, §16-34-2, §16-38-2, §16-44-2, §11-1-4; G-ENGINE-ENGAGE-ATTACK-AFTER-END-OF-TURN-TRIGGERS)`.

---

## Phase E — Infinite loops (§18-3)

### Task 22: An infinite loop ends in a draw or is stopped by its owner; never silently cleared

**Gap:** `G-ENGINE-INFINITE-LOOP-SILENTLY-CLEARED`. **Rules:** §18-3-1, §18-3-2, §18-3-3, §18-3-3-1, §18-3-3-2, §18-3-3-3.

**Ruling (W10, controller, provisional):** the stoppable-loop representation is one optional prompt per repetition (the player keeps accepting, then declines), which satisfies §18-3-3-3 and is already the engine's natural shape; a bounded repeat-count action is not added. Cost if wrong: a later task adds the count action; the conformance test accepts either.

**Files:**
- Modify: `code/digimon-engine/src/effect_queue.rs` — replace the `MAX_CHAIN_DEPTH` silent `effect_queue.clear()` (~L1370–1376) and the outermost flush guards (~L1240–1300) with: (1) cycle detection by repeating `(source card, timing, board signature)` (a hash of the zones' card handles + suspension + memory) seen `LOOP_SIGNATURE_REPEATS` (= 3) times; (2) if no optional decision was offered within the detected cycle → `GameEvent::InfiniteLoop { .. }` then `GameOver { winner: None }` with `TerminalOutcomeReason::InfiniteLoopDraw` (§18-3-2); (3) otherwise the loop continues while the owner keeps accepting; the per-repetition optional prompt is the stop (§18-3-3-3); never clear unrelated queued entries; (4) reset `effect_chain_depth` whenever a player decision resolves. Keep `MAX_CHAIN_DEPTH` only as a far backstop (raise to 10_000) that also ends in the §18-3-2 draw with the event, never a silent clear.
- Modify: `code/digimon-engine/src/events.rs` (`GameEvent::InfiniteLoop`), `game/mod.rs` (`TerminalOutcomeReason::InfiniteLoopDraw`), serialization (`serialization.rs`, Tauri `engine_commands.rs` DTO if it enumerates reasons).
- Tests to un-ignore: `ch18_pending_overwrite.rs::infinite_loop_is_stopped_not_silently_cleared`, `::stoppable_loop_repeats_until_its_owner_stops_it`.
- New test (Review Focus 5): `ch18_pending_overwrite::long_finite_chain_is_not_an_infinite_loop` — a 60-deep finite chain (e.g. 60 chained "draw 1" observers on a 60-card deck) completes without a draw or event; cite §18-3-1 with `cite!`.
- Existing test to update: `effect_queue.rs` ~L5840 unit test setting `effect_chain_depth = MAX_CHAIN_DEPTH` (asserts the clear) → asserts the event + draw.
- Keep green: `--test rules_conformance`, `--test combat`, `--test cards_behavioral` (full), `--test clone_fuzz` (if present; the signature must be clone-stable).

- [ ] **Step 1: Un-ignore both; write the finite-chain test; run → FAIL on the two.**
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** `18-3-1`, `18-3-2`, `18-3-3`, `18-3-3-3` → proven; `18-3-3-1`, `18-3-3-2` stay `gap` (no test; non-blocking after this task — set `blocks_training: false` with a note "repeat-count declaration not modelled; per-repetition prompt satisfies 18-3-3-3"). Regenerate; `--check`. Entry RESOLVED; W10 marked "fixed on the per-repetition reading".
- [ ] **Step 5: Commit** — `engine: infinite loops end in a draw or stop at the owner's decision, never silently cleared (§18-3; G-ENGINE-INFINITE-LOOP-SILENTLY-CLEARED)`.

---

## Phase F — Closure

### Task 23: Ledger, digest, tracker, exam re-triage list, PR body

**Files:**
- Modify: `qa/rules/mechanics-gaps-2026-10.md` — move every fixed gap to "Retracted or resolved during the campaign" with its commit; update the Denominator tables from `coverage_report`; update W2/W3/W5/W10/W12 with "fixed on reading (x)".
- Modify: `docs/RUST_ENGINE_GAPS.md` — verify every one of the 29 (+3 non-blocking) entries reads RESOLVED; the section heading "— OPEN, logged not fixed" → "— RESOLVED 2026-10 (Plan 4)".
- Create: `qa/rules/dcgo_quirks.md` — pre-registered DCGO divergences the fixes expose (spec §6): rule-check sub-steps in sequence; Overflow per permanent; Engage/Execute/Vortex/Overclock whole-attack-inside-coroutine; `<Mind Link>` declinable pick; Material Save trash-side pick timing (DCGO agrees, list only if an exam flips). For each, the rule citation and the exam-verdict `triage: dcgo_quirk` text to apply.
- Run: `grep -rl` over `qa/qa-reports/exam-verdicts/` for cards whose keywords/timings this plan changed (Alliance, Raid, De-Digivolve, Material Save, Overflow, Blitz, Collision, Vortex, Engage, Execute, Overclock, Link, Option trash, source-trash observers); list them in `qa/rules/dcgo_quirks.md` under "Exams to re-run against the oracle" (do not run DCGO; it is local-only).
- Run: the whole verification set and record counts: `rules_conformance` (expect 0 ignored for the 29 gap ids; the only remaining ignores are `G-DSL-OPTIONAL-PROCESSING-CONDITION-BLOCK`), `combat`, `judge_quiz`, `rules_faq`, `dna_result_new_card`, full `cards_behavioral`, `python scripts/verify --tier 0`, `python -m pytest code/tests/tools -q`.
- Update the memory file `project_rules_conformance_campaign.md` (controller does this, not the subagent).

- [ ] **Step 1: Regenerate and check** (`build_obligations`, `coverage_report --check`); paste the per-chapter table into the digest.
- [ ] **Step 2: Docs** as above.
- [ ] **Step 3: Verification set**; counts into the digest's Denominator section and the PR body draft at `<workspace>/pr-body-addendum.md`.
- [ ] **Step 4: Commit** — `docs: rules-conformance Plan 4 closure — digest, tracker, DCGO quirks pre-registration, denominator`.

---

## Phase G — Owner-requested coverage (added 2026-10-16)

### Task 24: §15-15-5 "isn't affected by effects": scope matrix, choosability, lapse, granted effects

**Owner request (2026-10-16):** make sure protection is covered: Digimon-effect-only immunity (Atratusmon ST23-09: "Your opponent's Digimon effects don't affect this Digimon until their turn ends") versus full opponent-effect immunity (Magnamon (X Antibody) BT16-102: "isn't affected by your opponent's effects") against an Option such as Plasma Stake BT3-098 ("[Main] Delete 1 of your opponent's Digimon with 13000 DP or more", also via its [Security] activation) or Final Shining Burst BT13-104 ("[Main] 1 of your opponent's Digimon gets -12000 DP until the end of your opponent's turn").

**Rules:** §15-15-5, §15-15-5-1 (not affected by processing caused by effects), §15-15-5-2 (when it can be affected again, effects immediately start affecting it; example: a -3000 DP applied to a suspended Digimon with suspended-only immunity applies as soon as it unsuspends), §15-15-5-3 (still choosable), §15-15-5-4 (gained effects not considered had); §16-39 <Progress> (proven in ch16); §17-1-3-1-1 (0 DP deletion is a rule, not an effect); §15-2 effect types (Digimon / Tamer / Option effects; [Security] activation of an Option's [Main] is still that Option card's effect).

**What exists:** `Game::permanent_is_unaffected_by_effect(target, controller, EffectSourceKind)` with per-entry `EffectImmunityFilter { source_kind, controller }` (`game/queries.rs` ~L246); `EffectContext::can_affect_permanent` (`effect_context/mod.rs` ~L1657) SKIPS the application for an immune target (14 call sites); DP: `ImmuneFromDPMinus` is filtered at read time in `combat/dp.rs::effective_dp`, but `CannotBeAffected` is a skip-at-apply gate. `EffectSourceKind::Rule` is the kind used for token / card-less effects, not for rule processing (rule checks never consult the gate). Existing engine tests: `tests/effect_source_kind/immunity.rs` (4), `tests/judge_quiz/a_immunity_scope.rs` (Q1, Q2, Q17, Q18, Q28), `tests/dsl/effect_immunity_step.rs` (3). No `rules_conformance` test and no sidecar entry cites §15-15-5-*.

**Files:**
- Create: `code/digimon-engine/tests/rules_conformance/ch15_15_unaffected.rs`; fixtures `tests/rules_conformance/fixtures/ch15_15/*.yaml` (one card per file; synthetic cards shaped like the four above: `IMMUNE_DIGI_ONLY` (opponent Digimon effects don't affect this until their turn ends), `IMMUNE_ALL_OPP` (isn't affected by your opponent's effects), `IMMUNE_WHILE_SUSPENDED` (the §15-15-5-2 example), `OPT_DELETE_13K` (Plasma Stake shape incl. `[Security] Activate this card's [Main] effects`), `OPT_MINUS_12K` (Final Shining Burst shape), `TAMER_MINUS_3K`, `DIGI_MINUS_3K`, `DIGI_GRANT_SA_MINUS1` (§15-15-5-4), a token-producing card for the `Rule` source kind).
- Modify: `tests/rules_conformance/meta.rs` (`CHAPTER_SOURCES` + the fixtures directory-count guard), `fixtures/mod.rs`, `main.rs` (mod).
- Modify (only if a red test's fix is local): `effect_context/mod.rs::can_affect_permanent` callers / `modifiers.rs` — record-and-suppress instead of skip for modifiers (§15-15-5-2), keyed by the same `EffectImmunityFilter` so the modifier starts applying when the immunity entry expires or its condition lapses. A larger fix is filed as a gap and the test `#[ignore]`d with the gap id (discover-then-pin).
- Sidecar: add `15-15-5`, `15-15-5-1`, `15-15-5-2`, `15-15-5-3`, `15-15-5-4` entries (proven, or gap + `blocks_training: true` + gap id).

**Tests (discover-then-pin; each `cite!`s its rule):**
1. `digimon_only_immunity_is_affected_by_opponent_option_main` — `IMMUNE_DIGI_ONLY` (12000+ DP) vs `OPT_DELETE_13K` from hand → deleted; vs `OPT_MINUS_12K` → DP 0 → rule-check deletion (§17-1-3-1-1).
2. `digimon_only_immunity_is_affected_by_option_security_activation` — the same two Options hit from the security stack during the immune Digimon's attack → affected (the [Security] activation is still an Option effect).
3. `digimon_only_immunity_is_affected_by_opponent_tamer_and_own_effects` — opponent Tamer -3000 applies; own Digimon -3000 applies; opponent Digimon -3000 does not.
4. `full_opponent_immunity_blocks_option_tamer_digimon_and_token_effects` — `IMMUNE_ALL_OPP` vs each source kind (Option from hand, Option via [Security], Tamer, Digimon, token effect) → unaffected; own effects → affected.
5. `full_opponent_immunity_does_not_block_rule_processing` — loses a battle → deleted; DP reduced to 0 by its OWN effect → rule-check deleted (rules are not effects).
6. `immune_permanent_is_still_choosable_and_offered_as_only_candidate` (§15-15-5-3) — chosen, nothing happens; offered even when it is the only candidate.
7. `effect_applied_while_immune_starts_affecting_when_immunity_lapses` (§15-15-5-2) — the rule's own example: `IMMUNE_WHILE_SUSPENDED` suspended, opponent Digimon -3000 applied, DP unchanged; unsuspend → DP -3000 immediately (same duration as the original effect). Also the Atratusmon shape: an opponent Digimon's "-3000 until the end of your next turn" applied during the immunity → applies when the immunity ends at the end of the opponent's turn if the modifier is still live.
8. `granted_effect_not_considered_had_while_immune` (§15-15-5-4) — `<Security A. -1>` granted by an opponent Digimon → security check count unchanged while immune; counted once immunity lapses while the grant persists.
9. `immunity_granted_mid_effect_covers_later_steps` — an opponent effect that first targets, then (after the target becomes immune through a derived trigger) tries to affect it: unaffected; the reverse (own "then" clause after self-immunity) per Q18's shape.

**Regression:** `--test effect_source_kind`, `--test judge_quiz`, `--test dsl -- immunity`, `--test combat -- progress`, `--test cards_behavioral -- st23_09 bt16_102 immun`, `--test rules_conformance`; ledger checks.

- [ ] **Step 1: Write the nine tests + fixtures; register the module; run → record which are red.**
- [ ] **Step 2: For each red: fix if local (record-and-suppress for §15-15-5-2), else file `G-ENGINE-UNAFFECTED-…` gap entries in `docs/RUST_ENGINE_GAPS.md` + digest and `#[ignore]` with the id.**
- [ ] **Step 3: Sidecar entries; regenerate; `--check`.**
- [ ] **Step 4: Commit** — `tests: §15-15-5 "isn't affected by effects" conformance module (scope matrix, choosability, lapse, granted effects)` (+ `engine: …` commits for any fix).

---

## Phase G — Gaps filed during execution (owner request 2026-10-17: each of the three blocking gaps filed during Plan 4 gets its own task)

### Task 25: `<Progress>` is a read-time shield: opponent effects applied before the attack stop applying while it attacks

**Gap:** `G-ENGINE-PROGRESS-DOES-NOT-HIDE-EARLIER-OPPONENT-EFFECTS` (blocking; filed by Task 24). **Rules:** §16-39-1, §15-15-5-1, §15-15-5-2; official Q&A Q6178 (an effect already affecting a card stops affecting it as soon as the card cannot be affected). DCGO: read-time `CanNotBeAffected` re-checked on every DP/effect read (`Permanent.cs`).

**What exists (Task 24):** `EffectContext::held_for_progress_attack` holds an opponent modifier installed DURING the attack (`ModifierEntry::held_while_attacking`) and `cleanup_attack` releases it after `expire_end_of_attack`; `ModifierRegistry::get` filters by `affecting_source_kind` / `immunity_filter_covers` at read time but cannot evaluate `Game::progress_excludes` (needs `pending_attack` + the printed/inherited keywords, which live on `Game`). A modifier installed BEFORE the attack therefore keeps applying while the `<Progress>` Digimon attacks.

**Files:**
- Modify: `code/digimon-engine/src/modifiers.rs` — `ModifierRegistry` gains `progress_shield: Option<PermanentHandle>` (plus the shielded permanent's identity, so battle-area compaction cannot retarget it); `get` (and every read path that already applies the immunity filter) hides every opponent-controlled, effect-tagged entry on the shielded permanent while the shield is set. `held_while_attacking` becomes redundant: remove it and its release in `cleanup_attack`, or keep it only as the install-time record (choose one; no double mechanism).
- Modify: `code/digimon-engine/src/game/queries.rs::progress_excludes` and `code/digimon-engine/src/combat/mod.rs` — refresh the shield wherever `progress_excludes` can change: attack declaration (after the attacker is fixed), `tick_declarative_effects` (a `<Progress>` grant gained or lost mid-attack), `cleanup_attack` (→ `None`), battle-area index shifts (resolve by identity via `live_attack_attacker`). Keep the Task 24 invariant: the rule-check drain after `expire_end_of_attack` runs on every path incl. the resumed park, then `check_turn_end`.
- Modify: `code/digimon-engine/src/effect_context/action/modifiers.rs` — drop `held_for_progress_attack` if the shield supersedes it.
- Tests to un-ignore: `ch15_15_unaffected.rs::progress_attacker_not_affected_by_earlier_opponent_effect_while_attacking`.
- New tests (ch15_15_unaffected.rs, each `cite!`): `progress_lost_mid_attack_applies_held_opponent_effects_immediately` (§15-15-5-2: the attacker loses `<Progress>` mid-attack → the hidden -3000 applies at once, before the battle); `progress_shield_follows_the_attacker_through_compaction` (a lower-index Digimon on the attacker's side leaves mid-attack → the shield still hides the opponent effect on the attacker, not on the Digimon that slid into the slot).
- Keep green: the whole `ch15_15_unaffected` module, `ch16_attack_keywords` progress tests, `--test combat` (`progress_partial`), `--test effect_context`, `--test dsl -- immunity`, `--test cards_behavioral -- progress bt24_017 medusamon immun` (filter), then the FULL `cards_behavioral`.

- [ ] **Step 1: Un-ignore; write the two new tests; run → FAIL.**
- [ ] **Step 2: Implement** the registry shield and its refresh points; remove the superseded hold.
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** `16-39-1` → proven (demoted to gap by Task 24; re-add its original tests); the new tests listed under `15-15-5-2`. Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: <Progress> shields the attacker from earlier opponent effects at read time (§16-39-1, §15-15-5-1, §15-15-5-2; G-ENGINE-PROGRESS-DOES-NOT-HIDE-EARLIER-OPPONENT-EFFECTS)`.

### Task 26: a token's effect is a Digimon effect

**Gap:** `G-ENGINE-TOKEN-EFFECT-SOURCE-KIND-IS-RULE` (blocking; filed by Task 24). **Rules:** §15-2-1-1 (an effect activated by a Digimon card, Digimon, or Security Digimon is a Digimon effect), §4-21-1 (a Digimon token is a Digimon with card information). DCGO: `AutoProcessing.cs` (~L79) sets `IsDigimonEffect` for token effects.

**Ruling (controller, provisional):** the observation tensor keeps its shape; the `source_kind_bucket` semantics change (token effects move from bucket 3 "rule" to bucket 0 "Digimon") is recorded as a dated note in `docs/TENSOR_SPEC.md` without a profile version bump (token effects are rare; existing checkpoints are not invalidated for the owner's purposes). Cost if wrong: a profile bump later.

**Files:**
- Modify: the four `source_kind_for_card_kind` sites — `code/digimon-engine/src/effect_queue.rs` (~L80), `effect_context/mod.rs` (~L96), `game_actions/mod.rs` (~L195), `game_phases.rs` (~L1347): `CardKind::Token => EffectSourceKind::Digimon`. Keep `Rule` for genuinely card-less effects (`replacement.rs`, `replacement_departure_batch.rs`, `game/lifecycle.rs`). Prefer ONE shared helper if the four sites can share it without a facade-lint violation.
- Modify: `code/digimon-engine/src/observation/tensor_v2_lite.rs` (~L346) `source_kind_bucket` follows the kind (no special-casing tokens back); update `docs/TENSOR_SPEC.md` with the dated semantic note; update any tensor/parity test that pinned bucket 3 for a token (`--test tensor_and_mask`, `code/tests/engine/test_tensor*.py` if present).
- Audit (same change): the five `source_kind: rule` immunity grants authored to cover tokens — BT13-088, BT16-102, EX11-046, EX13-062, EX6-011 — re-read each printed text in `data/card_bundles/<ID>.md`; where the text says "opponent's effects" / "opponent's Digimon's effects" the `rule` arm is now redundant: remove it and re-run that card's tests. Keep a `rule` arm only where the printed text genuinely covers card-less effects (expected: none; report each decision).
- Attribution: every "by your opponent's Digimon's effect" reader (deletion cause, `was_deleted_by_opponent`, Scapegoat / Mephistomon-style riders) now counts token effects — this is the rule; add one test proving a token's `[On Deletion]` DP reduction that deletes counts as "deleted by your opponent's Digimon's effect".
- Tests to un-ignore: `ch15_15_unaffected.rs::digimon_only_immunity_blocks_a_digimon_tokens_effect`.
- Keep green: `--test effect_source_kind`, `--test tensor_and_mask`, `--test dsl -- immunity source_kind`, `--test judge_quiz`, `--test cards_behavioral -- token familiar st23 bt13_088 bt16_102 ex11_046 ex13_062 ex6_011` (filter), then the FULL `cards_behavioral` (the attribution shift is pool-wide).

- [ ] **Step 1: Un-ignore; write the attribution test; run → FAIL.**
- [ ] **Step 2: Implement** the kind change, the tensor note, the YAML audit.
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** `15-2-1-1` → proven. Regenerate; `--check`. Entry RESOLVED (list the five YAML decisions in the Resolution line).
- [ ] **Step 5: Commit** — `engine: a token's effect is a Digimon effect (§15-2-1-1, §4-21-1; G-ENGINE-TOKEN-EFFECT-SOURCE-KIND-IS-RULE)`.

### Task 27: the battle-winner qualifier resolves the winner by identity with last-known information

**Gap:** `G-ENGINE-BATTLE-WINNER-QUALIFIER-STALE-INDEX` (blocking; filed by the Task 1 review). **Rules:** §15-8-3-1, §15-8-3-8 (a reference in the triggered state is read as it was then), §15-4-4-5 (the Task 1 model: a departed referent keeps its last-known information; live while present — `Game::gate_trigger_context`). Reachable: BT25-020 Marsmon when the deleted attacker's `[On Deletion]` removes the winner (or a lower-index Digimon) before the observer activates.

**Files:**
- Modify: `code/digimon-engine/src/combat/mod.rs` — carry `PendingBattleResolution::winner_identity` (already recorded) onto the `TriggerContext` of every battle-result trigger (winner / loser observers), alongside a last-known snapshot of the winner (traits, names, level, DP) populated the way `event_snapshot` is for the event Digimon.
- Modify: `code/digimon-engine/src/effect_queue.rs` (`TriggerContext::battle_winner` → identity + snapshot) and `code/digimon-engine/src/dsl_cards/predicate.rs::event_winner_trait_has` (~L1975) and its siblings (`event_winner_*`): resolve by identity while present; read the snapshot once the winner has left the field (same rule as Task 1: live while present, last-known after departure).
- Tests (new, each `cite!`): `ch15_08_effect_categories.rs::battle_winner_qualifier_reads_last_known_information_after_the_winner_leaves` (§15-8-3-8: winner deleted by the loser's `[On Deletion]` before the observer activates → the observer still activates on the winner's last-known trait); `..._does_not_read_the_digimon_that_slid_into_the_slot` (a lower-index Digimon leaves; the qualifier still reads the real winner); a BT25-020-shaped card test in `tests/cards_behavioral/bt25/bt25_020.rs`.
- Keep green: `ch15_08_effect_categories` (all), `--test event_permanent_identity`, `--test combat`, `--test timing_dispatch`, `--test cards_behavioral -- bt25_020 wins_a_battle battle_winner ts_` (filter).

- [ ] **Step 1: Write the tests; run → FAIL on the departed-winner cases.**
- [ ] **Step 2: Implement** identity + snapshot for the winner referent.
- [ ] **Step 3: Run → PASS**; regression list.
- [ ] **Step 4: Sidecar:** `15-8-3-8` → proven if it is a gap today, else add the tests to its list; `15-8-3-1` likewise. Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: battle-winner qualifier resolves the winner by identity with last-known information (§15-8-3-1, §15-8-3-8, §15-4-4-5; G-ENGINE-BATTLE-WINNER-QUALIFIER-STALE-INDEX)`.

### Task 28: battle-result observers trigger together with the battle deletion's `[On Deletion]`

**Gap:** `G-ENGINE-BATTLE-WIN-OBSERVERS-SEQUENCED-AFTER-ON-DELETION` (blocking; filed by the Task 27 polish round). **Rules:** §14-2-1/-2/-4 (the battle is one rule process; effects triggered by it are resolved before the next action), §15-4-3-2 and §15-4-3-5-1 (simultaneous triggers pool; the turn player orders theirs first), §16-7-4 (`<Piercing>` "will trigger simultaneously with effects triggered by the battle", with an `[On Deletion]` as the example); official Q&A Q3931 (Callismon EX8-045: a battle-result trigger and the deleted Digimon's `[On Deletion]` trigger simultaneously, then the turn player's activate first). DCGO: `Destroy()` stacks the `[On Deletion]` effects, the battle stacks the "wins a battle" effects on the same list, and one `AutoProcessCheck` resolves both.

**Ours:** the battle resolution drains the whole `[On Deletion]` pool before enqueueing the battle-result observers (`combat/mod.rs` ~L4496-4573, after Task 27), so when the turn player's attacker wins, the non-turn player's `[On Deletion]` resolves before the turn player's "wins a battle" effect. Pinned wrong by `dsl::battle_deleter::on_deletion_choice_finishes_before_end_of_battle_observer_after_clone` (`tests/dsl/battle_deleter.rs` ~L381-405).

**Files:**
- Modify: `code/digimon-engine/src/combat/mod.rs` battle resolution — enqueue the battle-result observers (`TriggerSource::BattleResolved`: wins / loses / deletes-in-battle, and `<Piercing>`'s trigger per §16-7-4) BEFORE the deletion batch's single flush so they pool with the loser's `[On Deletion]` (Task 5's `TriggerFlush`) and the drainer's turn-player-first `TriggerOrder` orders them; the `EndOfBattle` timing (§14-2-5) stays after the pool resolves. The Task 27 winner snapshot is already captured at the comparison, so pooling does not change what the qualifier reads.
- Modify: `tests/dsl/battle_deleter.rs::on_deletion_choice_finishes_before_end_of_battle_observer_after_clone` — invert with a §15-4-3-5-1 / Q3931 comment: with P0 the turn player and winner, P0's observer is ordered first (or a `TriggerOrder` is offered to P0 when it has 2+); the clone-faithfulness half of the test stays.
- New conformance tests (`ch15_04_simultaneous.rs` or the chapter module that holds §15-4-3-5 today; each `cite!`): `battle_result_trigger_pools_with_the_losers_on_deletion_turn_player_first` (Q3931 shape: P0 attacker with a "when this wins a battle" effect, P1 defender with `[On Deletion]`; the pool offers P0 first; both resolve before `EndOfBattle` observers); `piercing_triggers_with_the_battle_deletion_pool` (§16-7-4: `<Piercing>` and the defender's `[On Deletion]` are one pool; the security check happens in the pool's order).
- Keep green: `--test combat`, `--test dsl -- battle_deleter`, `ch15_08_effect_categories` (Task 27's departed-winner test is re-shaped with P1 as turn player so it survives this change), `--test timing_dispatch`, `--test cards_behavioral -- bt25_020 ex8_045 piercing wins_a_battle` (filter), then the FULL `cards_behavioral`.

- [ ] **Step 1: Write the two conformance tests; run → FAIL (observer after `[On Deletion]`).**
- [ ] **Step 2: Implement** the pooled enqueue; invert the named dsl test.
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** add the tests under `15-4-3-5-1` and `16-7-4` (flip to proven if gap). Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: battle-result observers pool with the battle deletion's [On Deletion], turn player first (§14-2-4, §15-4-3-5-1, §16-7-4; G-ENGINE-BATTLE-WIN-OBSERVERS-SEQUENCED-AFTER-ON-DELETION)`.

### Task 29: W5 owner ruling — an unqualified reference to a DUAL card reads its Digimon information

**Owner ruling (2026-10-17, final):** "This would be the Digimon colour/s." An unqualified reference to a DUAL card in a card zone (hand, deck, trash, security) — one that names no card category, e.g. "trash 1 purple card from your hand" — reads the DUAL card's Digimon information only (reading (b) of worksheet item W5). A `kind: option` qualifier reads the Option information (§4-6-6-2); a `kind: digimon` qualifier or any field reference reads the Digimon information (§4-6-5-2, §4-6-3). This supersedes Task 18's provisional reading (a) (either information, OR'd at the root). **Rules:** §4-6-1, §4-6-5-2, §4-6-6-2, §4-6-3. Also records W12 as confirmed (linking player chooses the replaced link card; already implemented by Task 3).

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs` — the root DUAL decision (~L62-81: `conjunct_dual_ref(pred) == DualRef::Either` → evaluate Digimon OR Option) becomes: an unqualified (`Either`) reference evaluates with `DualRef::Digimon` only; `kind: option` keeps `DualRef::Option`; remove the root OR. Keep `DualRef` and `conjunct_dual_ref` (the qualifier logic is unchanged); update the doc comments that cite "reading (a)".
- Modify (controller-authorised re-pins, owner ruling): `tests/rules_conformance/ch16_new_keywords.rs::dual_unqualified_colour_reference_matches_either_information` → rename to `dual_unqualified_colour_reference_reads_digimon_information` and re-pin: a picker whose filter matches the DUAL's Digimon colour finds it, a picker matching only its Option colour does not, a picker matching neither does not (use the existing `HAND_PICK_*_ANY` fixtures and `DUAL_ARTS`; read `fixtures/ch16_new/DUAL_ARTS.yaml` for the two faces' colours). `::dual_unqualified_reference_holds_through_one_information` → re-pin to the Digimon information: "purple Lv.6 card" matches iff the Digimon face is purple AND Lv.6; `not: red` matches iff the Digimon face is not red. Keep every `cite!` id; fingerprints are per id and unchanged. Do NOT touch `dual_use_declares_digimon_or_option_information`, `dual_traits_belong_to_both_informations` (traits are read on both informations when the card is referenced as a Digimon card AND as an Option card; that is a qualified reference pair), or `dual_digivolution_card_on_field_reads_digimon_information`.
- Modify: `qa/rules/cr_obligations_status.yaml` `4-6-1` note (ruling (b): unqualified reference reads the Digimon information; list the renamed test), `4-6-5-2` / `4-6-6-2` notes if they mention reading (a); regenerate the JSON.
- Modify: `qa/rules/mechanics-gaps-2026-10.md` W5: "**Owner ruled (b) 2026-10-17: Digimon information.** Fixed in Task 29 (<sha>)."; W12: "**Confirmed by owner 2026-10-17** (linking player chooses; implemented by Task 3)." `docs/RUST_ENGINE_GAPS.md` `G-ENGINE-DUAL-OPTION-INFO-NOT-REFERENCED-IN-CARD-SEARCH` Resolution line: append "Owner ruling 2026-10-17: unqualified references read the Digimon information (W5 (b)); the Option information is read only under `kind: option`."
- Keep green: `--test dsl`, `--test dsl_eval_arm_coverage` (no wildcard arm), `--test rules_conformance` (whole), `--test cards_behavioral -- dual bt26_032 bt26_056 st24_07 blast_dna` (filter), `--test judge_quiz`, then the FULL `cards_behavioral` (a production DUAL searched by an unqualified colour reference may change outcome; list every card test that flips and confirm each against the card's printed text and the ruling).

- [ ] **Step 1: Re-pin the two tests; run → FAIL under reading (a).**
- [ ] **Step 2: Implement** the root change; update doc comments.
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar notes**, digest W5/W12, gap Resolution line. Regenerate; `--check`.
- [ ] **Step 5: Commit** — `engine: an unqualified reference to a DUAL card reads its Digimon information (owner ruling W5 (b); §4-6-1, §4-6-5-2, §4-6-6-2)`.

### Task 30: an effect-driven link at the link limit offers the replacement choice

**Gap:** `G-ENGINE-EFFECT-LINK-AT-LIMIT-NO-REPLACEMENT-CHOICE` (blocking; filed by Task 3). **Rules:** §4-9-5 (linking at the limit trashes the same number of existing link cards), §10-1-3-1 with W12 (the linking player, the host's controller, chooses which). Task 3 gave the declared link and the Plug-In Option link a `ResumeFrame::LinkCardTrashChoice` prompt; effect-driven links (the DSL `link_cards` step's pick loop and the relink/absorb path) still take a forced single-card replacement only, and with 2+ existing link cards they commit without a choice.

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/step/link_cards.rs` (the pick loop's attach, ~the site that calls `trash_forced_link_replacement`) and `dsl_cards/step/selections.rs` / `link_cards.rs` relink absorb path — when the host is at or over the limit with 2+ existing link cards, park the same `LinkCardTrashChoice` frame (purpose: effect link) with the step's outer continuations (`wrap_outer_conts` as Task 3 did for the Option-link purpose at `step/mod.rs` ~L407), then resume into the attach; with exactly one existing card keep the forced trim; with zero, attach directly.
- Modify: `code/digimon-engine/src/game_actions/link.rs` — a shared entry (`park_link_replacement_choice` already exists; give it the effect-link purpose and the resume site) so all three link entry points share ONE replacement path; `trash_forced_link_replacement` keeps only the one-card case (and handle `excess >= 2` with one card, noted by the Task 3 re-review, by trashing it directly).
- Tests: un-ignore `tests/rules_conformance/ch16_processing_keywords.rs::effect_link_at_limit_replaces_existing_link_card_by_choice` (the `#[ignore]`d §4-9-5 reproducer: 3 options offered, the new card excluded, the linking player chooses); add `effect_relink_at_limit_offers_the_same_choice` for the relink/absorb path (cite 4-9-5); a card test for one production effect-link carrier at the limit (pick from `code/digimon-engine/cards/` a `link_cards` user; stage a host with `<Link +1>` and two link cards).
- Keep green: `ch16_processing_keywords` link tests, `ch17_rule_checks` link tests, `--test option_flow`, `--test cards_behavioral -- link plug_in appmon` (filter), `--test rules_conformance`, then the FULL `cards_behavioral` (the ~81 fixtures Task 3 repaired must stay green).

- [ ] **Step 1: Un-ignore; write the relink and card tests; run → FAIL.**
- [ ] **Step 2: Implement** the shared replacement path for effect links.
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** `4-9-5` → proven (all its tests green and un-ignored); remove `blocks_training` / `gap_id`. Regenerate; `--check`. Entry RESOLVED; `G-ENGINE-LINK-AT-LIMIT-REFUSED-NOT-REPLACED` Resolution line updated (no longer says 4-9-5 stays gap).
- [ ] **Step 5: Commit** — `engine: effect-driven links at the limit offer the replacement choice (§4-9-5, §10-1-3-1; G-ENGINE-EFFECT-LINK-AT-LIMIT-NO-REPLACEMENT-CHOICE)`.

### Task 31: a battle winner's traits are read from its top card

**Gap:** `G-ENGINE-BATTLE-WINNER-TRAIT-READS-DIGIVOLUTION-CARDS` (blocking; filed by the Task 28 polish round). **Ruling (controller, provisional):** a Digimon's traits are those of its top card; §4-7-7 ("the bottom cards of a stack are treated as part of the information on the top card; they aren't considered to be placed on the field") governs stack membership and removal (§4-7-8), not the Digimon's characteristics. DCGO reads `permanent.TopCard` (`BT25_020.cs` ~L184 `CanTriggerWhenWinBattle`). Digivolution cards contribute only inherited effects. Cost if wrong: one re-pin.

**Files:**
- Modify: the winner-trait reader(s) (`trigger_context.rs::battle_winner_has_trait`, the winner snapshot builder in `combat/mod.rs::battle_winner_referent`, and any `event_winner_*` predicate that scans sources): read the top card's traits (plus Rule-granted traits per `all_traits`), not every card in the stack. Audit the sibling readers: `has_trait` on a permanent elsewhere in `dsl_cards/predicate.rs` (`trait_has` family) — if the general permanent trait predicate already reads the top card only, align the winner reader with it; if the GENERAL predicate also scans the stack, that is the real defect: fix it once at the shared helper and list every card test that flips (each confirmed against printed text: a Digimon with a [TS] digivolution card but a non-[TS] top card is NOT a [TS] Digimon).
- Tests: un-ignore `tests/dsl/battle_deleter.rs::turn_players_observer_reads_the_winners_top_card_before_de_digivolution`; add a conformance test in `ch15_08_effect_categories.rs` (or the chapter holding §4-7 if one exists) `battle_winner_trait_is_the_top_cards_trait` citing 4-7-7 (fingerprint from an existing 4-7-7 citer, or add the obligation) and a general `permanent_trait_is_the_top_cards_trait` if the shared predicate changed.
- Keep green: `ch15_08`, `--test dsl -- battle_deleter trait`, `--test combat`, `--test cards_behavioral -- bt25_020 trait ts_ royal_knight` (filter), whole `rules_conformance`, then the FULL `cards_behavioral`.

- [ ] **Step 1: Un-ignore; write the tests; run → FAIL.**
- [ ] **Step 2: Implement** at the shared reader.
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** list the tests under `4-7-7` (flip to proven if gap). Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: a Digimon's traits are its top card's; the battle-winner qualifier reads the top card (§4-7-7; G-ENGINE-BATTLE-WINNER-TRAIT-READS-DIGIVOLUTION-CARDS)`.

### Task 32: a battle-result trigger's condition is checked when it triggers

**Gap:** `G-ENGINE-BATTLE-RESULT-TRIGGER-CONDITION-NOT-CHECKED-AT-TRIGGER` (blocking; filed by the Task 28 polish round). **Rules:** §15-8-3-1 (a trigger-type effect triggers when its trigger condition is met), §15-8-3-8 (a reference in the triggered state is read as it was then), §15-4-4-5 (a pending effect whose trigger conditions are no longer met cannot activate). DCGO checks the condition at stack time (`CanTriggerWhenWinBattle`).

**Ours:** `enqueue_triggered` queues every `BattleResult` effect of the matching timing without evaluating its clause-level trigger qualifier; the drainer's non-firing filter is transient (`non_firing_queued_effect_indices_for`), so an entry whose condition was false at the battle stays queued while another player has firing entries, and activates once a sibling (the loser's `[On Deletion]`) makes it true. The ratified P1-staged `dsl::battle_deleter::winner_predicates_follow_ordinary_evolution_during_opponents_on_deletion` pins the wrong outcome.

**Files:**
- Modify: `code/digimon-engine/src/effect_queue.rs` — evaluate the trigger qualifier (the `active_when` / event-qualifier half, per Task 1's split: qualifier at trigger, processing conditions live) at ENQUEUE for `BattleResult` entries, using the Task 27 winner snapshot captured at the comparison; an entry whose qualifier is false at the trigger is not queued (§15-8-3-1). Check whether the same enqueue-time gap exists for OTHER pooled timings (OnDeletion observers, OnAnyDeletion, OnPlay observers): Task 1 introduced `gate_trigger_context` for qualifiers on departed referents; if non-battle timings already evaluate qualifiers at enqueue, mirror that path; if they also rely on the transient filter, fix the shared site once and say so.
- Tests: invert `winner_predicates_follow_ordinary_evolution_during_opponents_on_deletion` (the ratification is withdrawn by this task; comment cites §15-8-3-1 / §15-8-3-8 and this gap id); new conformance test `battle_result_trigger_condition_false_at_the_battle_never_activates` in `ch15_08_effect_categories.rs` (cite 15-8-3-1 and 15-4-4-5: the loser's `[On Deletion]` gives the winner the trait AFTER the battle → the "[X] Digimon wins a battle" observer does not activate); a converse test that a condition TRUE at the battle and false afterwards still activates with last-known information (already Task 27's departed-winner test — list it).
- Keep green: `ch15_08`, `ch15_04`, `--test dsl -- battle_deleter`, `--test combat`, `--test timing_dispatch`, whole `rules_conformance`, then the FULL `cards_behavioral`.

- [ ] **Step 1: Write the tests; invert the pin; run → FAIL.**
- [ ] **Step 2: Implement** qualifier evaluation at enqueue for the battle-result pool (and the shared site if the gap is general).
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** list the tests under `15-8-3-1` and `15-4-4-5`. Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: battle-result triggers check their condition when they trigger (§15-8-3-1, §15-8-3-8, §15-4-4-5; G-ENGINE-BATTLE-RESULT-TRIGGER-CONDITION-NOT-CHECKED-AT-TRIGGER)`.

### Task 33: `<Raid>` is a post-declaration switch, not a declaration-time target permission

**Gap:** `G-ENGINE-RAID-DECLARATION-TARGET-UNSUSPENDED` (blocking; filed by the Task 13 review). **Rules:** §11-2-7-1 (an attack on a Digimon is declared on a suspended Digimon, or one the attacker may attack unsuspended by effect), §16-23-1/-2 (`<Raid>`: WHEN this Digimon attacks, you may switch the target to the opponent's unsuspended Digimon with the highest DP; the attacker chooses on a tie). DCGO: `Permanent.CanAttackTargetDigimon` applies the normal legality at declaration; `Raid.cs` switches after.

**Ours:** three sites in `action/mask.rs` (Main-phase attack, end-of-turn attack, effect-granted attack) and one in `action/decode.rs` offer a `<Raid>` attacker the highest-DP UNSUSPENDED opponent Digimon as a DECLARATION target. The declared target therefore differs from the rules' (the Raid switch then has nothing to do), which changes the attack-declaration pool state (`[When Attacking]` observers read the declared target, Task 1) and the opponent's counter/block decisions. Task 1 already implements the real `<Raid>` switch as a `[When Attacking]` trigger body in the attack pool.

**Files:**
- Modify: `code/digimon-engine/src/action/mask.rs` (the three sites) and `action/decode.rs` — remove the Raid arm from declaration-time target legality; keep suspended targets and `CanAttackUnsuspended`-type grants. The Raid switch (Task 1's trigger) is then the only way a `<Raid>` attacker ends up attacking an unsuspended Digimon.
- Tests: re-pin `tests/mask_and_tensor/mask_end_of_turn_parity.rs`'s Raid pin added by Task 13 (it pins the current behaviour with a comment pointing at this gap id) to the rules reading; add a conformance test in `ch16_attack_keywords.rs` (`raid_attacker_declares_on_a_legal_target_then_may_switch`, cite 16-23-1 and 11-2-7-1): a `<Raid>` attacker with only unsuspended opponents may declare on the PLAYER (or a suspended Digimon if one exists) and is then offered the switch; it cannot declare directly on an unsuspended Digimon. Check `tests/combat/raid*.rs` and the `cards_behavioral` filter `raid` for pins of the old arm; invert each with a §11-2-7-1 / §16-23 comment and list them.
- Keep green: `--test mask_and_tensor`, `--test combat`, `ch16_attack_keywords` + whole `rules_conformance`, `--test flood_gates`, `--test cards_behavioral -- raid vortex when_attacking` (filter), then the FULL `cards_behavioral` (the mask bit set changes).

- [ ] **Step 1: Write the conformance test; run → FAIL (declaration on an unsuspended target is in the mask).**
- [ ] **Step 2: Implement** the legality change at the four sites.
- [ ] **Step 3: Run → PASS**; invert the named pins; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** list the test under `16-23-1` and `11-2-7-1` (flip to proven if gap). Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `engine: <Raid> attackers declare on a legal target and may switch afterwards (§11-2-7-1, §16-23; G-ENGINE-RAID-DECLARATION-TARGET-UNSUSPENDED)`.

### Task 34: a trigger-type effect's qualifier is checked when it triggers, for every observer timing

**Gap:** `G-ENGINE-OBSERVER-TRIGGER-QUALIFIER-NOT-CHECKED-AT-TRIGGER` (blocking; filed by Task 32; obligation `15-8-3-4` carries it). **Rules:** §15-8-3-1 (a trigger-type effect triggers when its trigger condition is met), §15-8-3-4, §15-8-3-8 (the referent is read in the triggered state), §15-4-4-5 (a pending effect whose trigger conditions are no longer met cannot activate), §15-8-3-9-3 (a PROCESSING condition is read at activation: the rulebook's own example, pinned by `processing_condition_checked_at_activation_not_trigger`). DCGO decides the trigger at stack time (`CanTrigger*` methods).

**Ours (after Task 32):** only `BattleResult` entries have their qualifier evaluated at enqueue (`discard_entries_not_triggered`). Every other observer timing queues each matching effect and relies on the drainer's transient non-firing filter, so an entry whose qualifier was false at the trigger can activate once a sibling makes it true. The general fix was impossible in Task 32 because the DSL carries the trigger qualifier and the processing condition in the SAME fields (`condition` / `active_when`): fixture `OBS_OPP_PLAY_GE4` uses `event_target_level_gte` as a processing condition while `OBS_PLAY_LE4K` uses `event_target_dp_lte` as a qualifier.

**Design (controller):** add a per-clause DSL field `trigger_if:` (name final after checking `code/digimon-dsl` naming conventions; `trigger_when:` is acceptable) lowered to its own closure on `Effect` (`trigger_qualifier`), evaluated (1) at every observer ENQUEUE against the trigger context the entry will carry (Task 1 semantics: live while the referent is present, snapshot once departed) and (2) again at activation through `gate_trigger_context` (§15-4-4-5). `condition` / `active_when` keep their meaning as PROCESSING conditions read at activation only. `discard_entries_not_triggered` is generalised to every timing and repointed at the new closure; the `BattleResult`-only `debug_assert` is removed. Migrate the production clauses whose `condition` is in fact a trigger qualifier onto `trigger_if:` — Task 32 audited the 9 `on_ally_won_battle` clauses (BT20-042, BT20-044 ×2, BT25-020, EX3-044 ×2, EX13-041, EX13-045, EX13-076); audit the other observer timings by reading each card's printed text ("When X [that is Y] …" = qualifier; "… if Y, do Z" = processing). Record every migration with the printed clause in the report.

**Files:**
- Modify: `code/digimon-dsl/src/{clause,compiled,lower*}.rs` (new field, lowering, schema), `code/tools/dsl-schema-export` output, the DSL vocab doc (`PYTHONUTF8=1` regen + `--check`), `code/digimon-engine/src/effect.rs` (new closure field), `dsl_cards/lower_triggered.rs`, `effect_queue.rs` (`enqueue_triggered` / `enqueue_triggered_scan` evaluate `trigger_qualifier` with the entry's trigger context; `discard_entries_not_triggered` generalised), `trigger_context.rs` as needed; production YAML migrations; a `dsl-lint` rule: an observer clause whose `condition` reads `event_*` fields warns "is this a trigger qualifier? use trigger_if:".
- Tests: new conformance tests in `ch15_08_effect_categories.rs`: `observer_whose_qualifier_was_false_at_the_trigger_never_activates` (cite 15-8-3-1, 15-8-3-4; an `on_opponent_play` observer with `trigger_if: event_target_dp_lte 4000`, the played Digimon gets +DP after the play but before the pool resolves → no activation) and `processing_condition_is_still_read_at_activation` (cite 15-8-3-9-3; the GE4 shape stays green with `condition:`); `ch15_04::pending_effect_whose_trigger_condition_lapsed_cannot_activate` stays green through the activation re-check. Re-pin `OBS_PLAY_LE4K` onto `trigger_if:`. Keep every Task 1 test green (`gate_trigger_context`, departed-referent snapshots).
- Keep green: `--test dsl`, `--test dsl_eval_arm_coverage`, `--test timing_dispatch`, `--test effect_context`, whole `rules_conformance`, `--test judge_quiz`, then the FULL `cards_behavioral` (every observer card is affected by the enqueue-time check).

- [ ] **Step 1: Write the two conformance tests; run → FAIL on the qualifier test.**
- [ ] **Step 2: Implement** the field, lowering, enqueue-time evaluation, activation re-check, lint; migrate the audited clauses.
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** `15-8-3-4` → proven (remove `blocks_training` / `gap_id`), tests listed under 15-8-3-1 and 15-8-3-9-3. Regenerate; `--check`. Entry RESOLVED. Note the DSL vocabulary addition in `qa/dsl-vocab-gaps.md` (closing the "trigger qualifier vs processing condition" entry if one exists).
- [ ] **Step 5: Commit** — `engine+dsl: trigger qualifiers (trigger_if:) are checked when an observer triggers and again at activation (§15-8-3-1, §15-8-3-4, §15-4-4-5; G-ENGINE-OBSERVER-TRIGGER-QUALIFIER-NOT-CHECKED-AT-TRIGGER)`.

### Task 35: a Digimon's name is its top card's — re-author the carrier-name predicates

**Gap:** `G-DSL-CARRIER-NAME-READS-DIGIVOLUTION-CARDS` (blocking; filed by the Task 31 review). **Ruling:** the same as Task 31's — a Digimon's characteristics (name, traits, level, colour) are its top card's; §4-7-7 governs stack membership; §4-8-2 says digivolution-card information is read only when digivolution cards are referenced; DCGO reads `TopCard.ContainsCardName` / `TopCard.HasGreymonName`; official Q3225 (EX1-038) treats "[Imperialdramon] in its name" as the top card's name.

**Ours:** `Permanent::contains_card_name` (`permanent.rs` ~L482-494) checks the top card and then every digivolution card; the DSL predicate `self_digivolution_contains_name` (`dsl_cards/predicate.rs` ~L654-662) exposes it, and four cards use it for "this Digimon's name / is [X]": BT12-059 Agumon (inherited "While this Digimon has [Greymon] or [Omnimon] in its name, +1000 DP"), EX1-014 ExVeemon (inherited "[Imperialdramon] in its name or [Free] in its traits"), BT20-059 ("While this Digimon is [Jesmon GX]"), BT17-102 ("If this Digimon's name is [Koromon]"). A non-matching top card with a matching digivolution card wrongly gains the buff. BT16-027 and BT20-020 read "if [Imperialdramon: Dragon Mode] is in this Digimon's digivolution cards" through the same top-card-and-sources scan (harmless today: their top card is Fighter Mode) where `self_digivolution_sources_contain_name` is the right predicate.

**Files:**
- Modify: the four YAMLs above — switch the name test to a top-card name predicate (`source_name_contains` exists; if its subject is the effect's source rather than "this Digimon" for an inherited clause, add the missing vocabulary `self_name_contains` to `code/digimon-dsl` + `predicate.rs` rather than route around it); BT16-027 / BT20-020 → `self_digivolution_sources_contain_name`. Do NOT change `contains_card_name` itself.
- Add a `dsl-lint` rule: `self_digivolution_contains_name` used in a "this Digimon's name / is [X]" clause warns (its legitimate use is "[X] in this Digimon's digivolution cards or name" shapes — check each remaining user against printed text and list them).
- Tests: a behavioural test per re-authored card staging a non-matching top card over a matching digivolution card (no buff) and the matching top card (buff); un-ignore or add a conformance test in `ch15_08_effect_categories.rs` or the §4-7 chapter if one exists (`digimon_name_is_the_top_cards_name`, cite 4-8-2 with a valid fingerprint; list under 4-8-2 and the 4-7-7 note — status per the Task 31 ruling: 4-7-7 stays gap).
- Keep green: `--test dsl`, `cards_behavioral` filter `bt12_059 ex1_014 bt20_059 bt17_102 bt16_027 bt20_020 greymon imperialdramon jesmon`, whole `rules_conformance`, `--test judge_quiz`, then the FULL `cards_behavioral`.

- [ ] **Step 1: Write the card tests; run → FAIL on the non-matching-top-card cases.**
- [ ] **Step 2: Re-author** the six cards; add the lint rule (and `self_name_contains` if needed).
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** list the test under `4-8-2` (flip to proven if gap and the test proves the text). Regenerate; `--check`. Entry RESOLVED.
- [ ] **Step 5: Commit** — `cards+dsl: a Digimon's name is its top card's; carrier-name predicates re-authored (§4-7-7, §4-8-2; G-DSL-CARRIER-NAME-READS-DIGIVOLUTION-CARDS)`.

**Phase G in Task 23's closure:** the digest headline and the "blocking gaps with no task" list are updated to reflect Tasks 25-35; W5 and W12 are recorded as owner-decided; the §4-7-7 / top-card characteristics ruling (Tasks 31, 35) is listed for owner sign-off.

### Task 36: `<Overclock>` attacks only if its cost deletion actually happened

**Gap:** `G-ENGINE-OVERCLOCK-COST-REPLACEMENT-PARKS-AFTER-POOL` (filed non-blocking by Task 21; the attack-after-prevented-cost half is BLOCKING per the Task 21 re-review — flip the entry and sidecar `16-34-3` to `blocks_training: true`). **Rules:** §16-34-3 (by deleting 1 of your Tokens / specified Digimon — the optional processing condition), §15-7-1/-2 (an optional processing condition's processing happens only when the condition is fulfilled), §15-8-5-2 (a replacement that prevents the deletion means the deletion did not happen), §11-1-4 (ordering half, stays non-blocking). DCGO `Overclock.cs` attacks only `if (isDeleted)`. Reachable with ST19-12 / ST19-08 (Overclock) against ST19-10 / ST19-13 / BT22-038 / BT22-076 (`<Armor Purge>` [Puppet]).

**Files:**
- Modify: `code/digimon-engine/src/cards/keyword_effects.rs` (the Overclock body) and `code/digimon-engine/src/game_phases.rs::run_overclock_selection_step` / the parking path: key the attack declaration on the chosen Digimon having actually left the battle area by deletion (read the deletion batch's outcome / a departed-referent check after the replacement window resolves), on BOTH the non-parking and the parking path; when the deletion is prevented, no attack and no `<Overclock>` use consumed beyond the attempted cost (§15-7-2: the condition was attempted but not fulfilled — read the text and say which).
- Candidates: §15-7-4 — a candidate that cannot be deleted (e.g. an un-deletable Digimon) is not offered as a cost choice; check `CostGuard`/`leading_cost_guard` for the Overclock cost shape.
- Tests to un-ignore: the two `#[ignore]`d tests Task 21 added in `tests/combat/overclock.rs` (they fail at their rule assertion today); add a clone-while-parked variant (clone during the Armor Purge replacement prompt, resolve both choices on clones).
- Keep green: `--test combat`, `--test keyword_phase_f`, `--test rules_conformance`, `--test cards_behavioral -- overclock st19 armor_purge puppet` (filter), `--test clone_fuzz --features dsl-yaml-loader`, the dcgo-harness tests + the Overclock sim-only exam (BT20-037), then the FULL `cards_behavioral`.

- [ ] **Step 1: Un-ignore both; run → FAIL.**
- [ ] **Step 2: Implement** on both paths.
- [ ] **Step 3: Run → PASS**; regression list; full `cards_behavioral`.
- [ ] **Step 4: Sidecar:** `16-34-3` → proven (list the tests); the ordering half stays an open non-blocking note under the gap. Regenerate; `--check`. Entry: blocking half RESOLVED.
- [ ] **Step 5: Commit** — `engine: <Overclock> attacks only when its cost deletion happened (§16-34-3, §15-7-1, §15-8-5-2; G-ENGINE-OVERCLOCK-COST-REPLACEMENT-PARKS-AFTER-POOL)`.
