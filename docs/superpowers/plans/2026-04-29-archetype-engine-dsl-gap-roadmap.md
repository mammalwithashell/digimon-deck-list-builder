# Archetype Engine and DSL Gap Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the archetype engine/DSL gap roadmap into a sequence of implementable, testable child plans that close reusable Rust engine and DSL capabilities in dependency order.

**Architecture:** This is a parent execution plan for a multi-subsystem roadmap. Each task creates or executes one focused child implementation plan with a narrow behavioral slice, TDD tests, tracker updates, and an archetype unlock check. Shared state-machine surfaces are serialized; independent DSL predicate, token, and keyword slices can run in parallel after their dependencies land.

**Tech Stack:** Rust (`code/digimon-engine`, `code/digimon-dsl`), YAML card DSL, Cargo integration tests, PyO3 boundary review, markdown gap trackers.

---

## Scope Check

The source spec `docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md` intentionally covers many independent subsystems. Do not implement it as one giant patch. Execute this parent plan by creating and completing child plans, each of which must produce working, testable software on its own.

The child plans should be created in this order:

1. `2026-04-29-gap-group-1-inherited-dispatch-opt.md`
2. `2026-04-29-gap-group-1-event-context-followups.md`
3. `2026-04-29-gap-group-2-selection-primitives.md`
4. `2026-04-29-gap-group-3-cost-replacement.md`
5. `2026-04-29-gap-group-4-zone-movement.md`
6. `2026-04-29-gap-group-5-option-delay-link.md`
7. `2026-04-29-gap-group-6-modifiers-keywords.md`
8. `2026-04-29-gap-group-7-dsl-predicates-formulas.md`
9. `2026-04-29-gap-group-8-token-card-data.md`
10. `2026-04-29-gap-group-9-archetype-unlocks.md`

Only Task 1 is the first executable engineering slice. Tasks 2-10 should not be started until their child plan exists and is reviewed.

## File Structure

Parent plan artifact:

- Create: `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`

Child implementation plan artifacts:

- Create: `docs/superpowers/plans/2026-04-29-gap-group-1-inherited-dispatch-opt.md`
- Create: `docs/superpowers/plans/2026-04-29-gap-group-1-event-context-followups.md`
- Create: `docs/superpowers/plans/2026-04-29-gap-group-2-selection-primitives.md`
- Create: `docs/superpowers/plans/2026-04-29-gap-group-3-cost-replacement.md`
- Create: `docs/superpowers/plans/2026-04-29-gap-group-4-zone-movement.md`
- Create: `docs/superpowers/plans/2026-04-29-gap-group-5-option-delay-link.md`
- Create: `docs/superpowers/plans/2026-04-29-gap-group-6-modifiers-keywords.md`
- Create: `docs/superpowers/plans/2026-04-29-gap-group-7-dsl-predicates-formulas.md`
- Create: `docs/superpowers/plans/2026-04-29-gap-group-8-token-card-data.md`
- Create: `docs/superpowers/plans/2026-04-29-gap-group-9-archetype-unlocks.md`

Likely engine files by group:

- Event dispatch: `code/digimon-engine/src/effect_queue.rs`, `code/digimon-engine/src/effect.rs`, `code/digimon-engine/src/events.rs`, `code/digimon-engine/src/trigger_context.rs`, `code/digimon-engine/src/game_actions.rs`, `code/digimon-engine/src/game_phases.rs`, `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/permanent.rs`, `code/digimon-engine/src/enums.rs`
- Selection/action masks: `code/digimon-engine/src/selection.rs`, `code/digimon-engine/src/action/space.rs`, `code/digimon-engine/src/action/mask.rs`, `code/digimon-engine/src/action/decode.rs`, `code/digimon-engine/src/action/explain.rs`, `code/digimon-engine/src/debug_runner.rs`
- Cost/replacement: `code/digimon-engine/src/effect.rs`, `code/digimon-engine/src/effect_queue.rs`, `code/digimon-engine/src/effect_context.rs`, `code/digimon-engine/src/replacement.rs`, `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/modifiers.rs`
- Zone movement: `code/digimon-engine/src/effect_context.rs`, `code/digimon-engine/src/game_actions.rs`, `code/digimon-engine/src/game.rs`, `code/digimon-engine/src/player.rs`, `code/digimon-engine/src/permanent.rs`, `code/digimon-engine/src/card_source.rs`
- Option/Delay/Link: `code/digimon-engine/src/enums.rs`, `code/digimon-engine/src/game_actions.rs`, `code/digimon-engine/src/game_phases.rs`, `code/digimon-engine/src/effect_context.rs`, `code/digimon-engine/src/scheduled_effects.rs`, `code/digimon-engine/src/dsl_cards/lower_delay.rs`
- Modifiers/keywords: `code/digimon-engine/src/modifiers.rs`, `code/digimon-engine/src/enums.rs`, `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/action/mask.rs`, `code/digimon-engine/src/tensor.rs`, `code/digimon-engine/src/card_data.rs`
- DSL predicates/formulas: `code/digimon-dsl/src/predicate.rs`, `code/digimon-dsl/src/formula.rs`, `code/digimon-dsl/src/clause.rs`, `code/digimon-dsl/src/step.rs`, `code/digimon-engine/src/dsl_cards/predicate.rs`, `code/digimon-engine/src/dsl_cards/formula_eval.rs`, `code/digimon-engine/src/dsl_cards/lower_triggered.rs`, `code/digimon-engine/src/dsl_cards/lower_replacement.rs`, `code/digimon-engine/src/dsl_cards/lower_aura.rs`
- Token/card data: `code/digimon-engine/src/token_registry.rs`, `code/digimon-engine/src/card_data.rs`, `code/digimon-engine/src/cards.rs`, `code/digimon-engine/src/card_registry.rs`

Likely test files by group:

- Event dispatch: `code/digimon-engine/tests/timing_dispatch.rs`, `code/digimon-engine/tests/effects/queue_drainer.rs`, `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs`, `code/digimon-engine/tests/cards_behavioral/bt13/`
- Selection/action masks: `code/digimon-engine/tests/selection/*.rs`, `code/digimon-engine/tests/mask_and_tensor/*.rs`, `code/digimon-engine/tests/cards_behavioral/ex10/`
- Cost/replacement: `code/digimon-engine/tests/cost_hooks/*.rs`, `code/digimon-engine/tests/replacements/*.rs`, `code/digimon-engine/tests/combat/redirect_and_cancel.rs`
- Zone movement: `code/digimon-engine/tests/effect_context/*.rs`, `code/digimon-engine/tests/zone_manipulation.rs`
- Option/Delay/Link: `code/digimon-engine/tests/option_flow/*.rs`, `code/digimon-engine/tests/dsl/delay.rs`
- Modifiers/keywords: `code/digimon-engine/tests/combat/*.rs`, `code/digimon-engine/tests/flood_gates/*.rs`, `code/digimon-engine/tests/keyword_phase_*/`
- DSL predicates/formulas: `code/digimon-engine/tests/dsl/parse_predicates.rs`, `code/digimon-engine/tests/dsl/parse_formulas.rs`, `code/digimon-engine/tests/dsl/phase3d_event_context.rs`, `code/digimon-engine/tests/dsl/phase3d_formula_zone_count.rs`
- Token/card data: `code/digimon-engine/tests/cards_behavioral/tokens.rs`, `code/digimon-engine/tests/keyword_parsing.rs`

Tracker files touched by almost every child plan:

- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`
- Relevant `qa/archetype-qa/*.md`
- Relevant `qa/archetype-qa/dsl/*.md`

## Global Rules for Every Child Plan

- [ ] **Step 1: Read the governing docs**

Read these before making the child plan:

```text
docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md
docs/RUST_ENGINE_API.md
docs/RUST_DSL_TEST_API.md
docs/ACTION_SPEC.md
docs/TENSOR_SPEC.md
docs/RUST_ENGINE_GAPS.md
qa/archetype-qa/engine-gaps.md
qa/dsl-vocab-gaps.md
```

- [ ] **Step 2: Prove the current gap with a failing test first**

Use the smallest real-card fixture named in the relevant tracker. Prefer an existing ignored test when one already exists. If creating a new test, place it in the group-specific test lane listed in the File Structure section.

- [ ] **Step 3: Keep action/mask contracts explicit**

If the child plan adds a player-visible choice, include an action-mask test and an action-decoder test. If it changes `ACTION_SPACE_SIZE`, also update:

```text
docs/ACTION_SPEC.md
docs/TENSOR_SPEC.md if tensor shape or semantics change
code/digimon-engine-py/src/lib.rs if exposed constants change
code/digimon_gym/digimon_gym.py if Python env constants or masks change
code/frontend/src if desktop/frontend constants exist for the changed range
```

- [ ] **Step 4: Close trackers only with evidence**

Move a gap to resolved only after tests pass and the implementation removes the blocker. If a gap narrows, update the wording rather than marking it resolved.

- [ ] **Step 5: Commit each child plan and each implementation slice separately**

Use focused commits:

```bash
git add docs/superpowers/plans/<child-plan>.md
git commit -m "docs: plan <gap group>"
```

For implementation slices, use:

```bash
git add <tests> <src> <docs>
git commit -m "feat: <capability>"
```

## Task 1: Create and Execute Child Plan for Inherited Dispatch + OPT Enforcement

**Files:**
- Create: `docs/superpowers/plans/2026-04-29-gap-group-1-inherited-dispatch-opt.md`
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Modify: `code/digimon-engine/src/effect.rs`
- Modify: `code/digimon-engine/src/trigger_context.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs`
- Test: `code/digimon-engine/tests/effects/queue_drainer.rs`
- Docs: `qa/archetype-qa/engine-gaps.md`

- [ ] **Step 1: Write the child plan**

Create `docs/superpowers/plans/2026-04-29-gap-group-1-inherited-dispatch-opt.md` with these exact sections:

```markdown
# Gap Group 1: Inherited Dispatch + OPT Enforcement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make triggered inherited effects in digivolution stacks dispatch correctly and enforce once-per-turn limits on triggered effects.

**Architecture:** Extend `effect_queue::enqueue_from_permanent` so it scans inherited source cards below the top card when an effect is marked inherited. Reuse the existing trigger context and activation-count path so inherited effects receive the carrier permanent and obey `once_per_turn` / `max_per_turn` rules.

**Tech Stack:** Rust digimon-engine, Cargo integration tests, DebugRunner/card behavioral fixtures.

---
```

- [ ] **Step 2: Add the failing inherited-dispatch test to the child plan**

The child plan must instruct the implementer to add a test like this to `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs` or revive an existing ignored equivalent:

```rust
#[test]
fn bt21_008_inherited_security_removed_gain_memory_fires_from_source() {
    let mut runner = DebugRunner::new();
    runner.load_card_data();
    runner.setup_basic_game();

    let carrier = runner
        .play_digimon_with_sources(
            0,
            "BT21-017",
            vec!["BT21-008"],
        )
        .expect("carrier with BT21-008 source");

    runner.set_memory(0, 0);
    runner.trash_top_security_by_effect(1);
    runner.drain_effects();

    assert_eq!(
        runner.memory(),
        1,
        "BT21-008 inherited observer should gain memory when opponent security is removed"
    );
    assert!(
        runner.permanent(0, carrier)
            .expect("carrier remains")
            .has_source_card("BT21-008"),
        "inherited effect fires from source without removing it"
    );
}
```

If helper names differ, the implementer must use the existing DebugRunner helper with the same setup semantics, not skip the test.

- [ ] **Step 3: Add the failing OPT test to the child plan**

The child plan must instruct the implementer to add a queue-level or behavioral test asserting that the same inherited effect does not fire twice in the same turn:

```rust
#[test]
fn inherited_once_per_turn_security_removed_observer_fires_once() {
    let mut runner = DebugRunner::new();
    runner.load_card_data();
    runner.setup_basic_game();

    runner
        .play_digimon_with_sources(0, "BT21-017", vec!["BT21-008"])
        .expect("carrier with inherited source");

    runner.set_memory(0, 0);
    runner.trash_top_security_by_effect(1);
    runner.drain_effects();
    runner.trash_top_security_by_effect(1);
    runner.drain_effects();

    assert_eq!(
        runner.memory(),
        1,
        "once-per-turn inherited observer must not fire twice in one turn"
    );
}
```

- [ ] **Step 4: Add implementation instructions to the child plan**

The child plan must instruct the implementer to make these code changes:

```text
1. In `effect_queue::enqueue_from_permanent`, keep the existing top-card scan.
2. After top-card/linked/training scans, iterate source cards below the top card.
3. For each source card, fetch effects with `effects_for_card`.
4. Enqueue only effects whose `inherited` flag is true and whose timing matches the event.
5. Build trigger context so `source_permanent` is the carrier permanent and `source_card` is the inherited card source.
6. Route the effect through the same activation-count / once-per-turn checks as top-card triggered effects.
7. Add regression coverage so top-card non-inherited effects from sources do not fire while buried.
```

- [ ] **Step 5: Run the child plan tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_008
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effects -- queue
```

Expected: tests fail before implementation, pass after implementation.

- [ ] **Step 6: Update trackers**

The child plan must update `qa/archetype-qa/engine-gaps.md` entry `G-INHERITED-DISPATCH` and any linked Medusamon/Rocks notes. It may only mark the gap resolved after the tests above pass.

- [ ] **Step 7: Commit child plan and implementation separately**

Run after child plan creation:

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-1-inherited-dispatch-opt.md
git commit -m "docs: plan inherited dispatch opt"
```

Run after implementation:

```bash
git add code/digimon-engine/src/effect_queue.rs code/digimon-engine/src/effect.rs code/digimon-engine/src/trigger_context.rs code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs code/digimon-engine/tests/effects/queue_drainer.rs qa/archetype-qa/engine-gaps.md
git commit -m "feat: dispatch inherited triggered effects"
```

## Task 2: Create Child Plan for Event Context Follow-Ups

**Files:**
- Create: `docs/superpowers/plans/2026-04-29-gap-group-1-event-context-followups.md`
- Modify: `code/digimon-engine/src/events.rs`
- Modify: `code/digimon-engine/src/trigger_context.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/game_phases.rs`
- Modify: `code/digimon-engine/src/combat.rs`
- Test: `code/digimon-engine/tests/timing_dispatch.rs`
- Test: `code/digimon-engine/tests/dsl/phase3d_event_context.rs`

- [ ] **Step 1: Create the child plan file**

Use the required writing-plans header. The goal sentence must be:

```markdown
**Goal:** Add the missing event payloads and dispatch sites that unblock OnMove, OnDigivolve trait filters, OnEnterFieldAnyone trait filters, option placement, and source-trash observers.
```

- [ ] **Step 2: Define the implementation slices inside the child plan**

Include these slices in this order:

```text
1. `OnMove` dispatch from breeding-to-battle movement.
2. `GameEvent::Digivolve` emission with newly-digivolved permanent in context.
3. `OnEnterFieldAnyone` context with entering permanent/card.
4. `OnOptionPlaced` timing and dispatch after option placement.
5. `OnDigivolutionCardTrashed` with host permanent and trashed source card in context.
6. `OnAllyAttack` / `OnOpponentAttack` declared-attack observer dispatch.
```

- [ ] **Step 3: Require one failing test per event**

The child plan must state that each slice adds one failing test before implementation in either `timing_dispatch.rs`, `phase3d_event_context.rs`, or the relevant real-card behavioral test.

- [ ] **Step 4: Define tracker updates**

The child plan must update all matching entries in:

```text
docs/RUST_ENGINE_GAPS.md
qa/archetype-qa/engine-gaps.md
qa/dsl-vocab-gaps.md
```

- [ ] **Step 5: Commit the child plan**

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-1-event-context-followups.md
git commit -m "docs: plan event context followups"
```

## Task 3: Create Child Plan for Selection and Action-Mask Primitives

**Files:**
- Create: `docs/superpowers/plans/2026-04-29-gap-group-2-selection-primitives.md`
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/action/space.rs`
- Modify: `code/digimon-engine/src/action/mask.rs`
- Modify: `code/digimon-engine/src/action/decode.rs`
- Modify: `code/digimon-engine/src/action/explain.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/selections.rs`
- Test: `code/digimon-engine/tests/selection/*.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/*.rs`
- Test: `code/digimon-engine/tests/dsl/phase2e_select_material.rs`

- [x] **Step 1: Create the child plan file**

The child plan must start with a scope note:

```markdown
This plan changes shared action/selection internals. Do not run it in parallel with breeding permanent handle work, action-space resizing, or replacement nested-selection work.
```

- [x] **Step 2: Require an action-space decision record**

The child plan must include a step to document whether new selections reuse existing action ranges or require `ACTION_SPACE_SIZE` changes. If action size changes, the plan must update `docs/ACTION_SPEC.md`, PyO3 constants, and RL env constants in the same implementation slice.

- [x] **Step 3: Define selection slices**

Include these slices:

```text
1. Stable source-card reference across own permanents.
2. Exact-N and up-to-N source selection with PASS terminator.
3. DP-budget multi-select for opponent permanents.
4. Effect-choice branch selector.
5. Breeding permanent selection if not separated into Task 10.
6. Ordered permutation / place remainder regression checks.
7. Empty inner selection continues outer tail.
```

- [x] **Step 4: Require test commands**

Each slice must run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- <slice_test_name>
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- <mask_test_name>
```

- [x] **Step 5: Commit the child plan**

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-2-selection-primitives.md
git commit -m "docs: plan selection primitives"
```

## Task 4: Create Child Plan for Cost and Replacement Framework

**Files:**
- Create: `docs/superpowers/plans/2026-04-29-gap-group-3-cost-replacement.md`
- Modify: `code/digimon-engine/src/effect.rs`
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Modify: `code/digimon-engine/src/replacement.rs`
- Modify: `code/digimon-engine/src/combat.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
- Test: `code/digimon-engine/tests/cost_hooks/*.rs`
- Test: `code/digimon-engine/tests/replacements/*.rs`
- Test: `code/digimon-engine/tests/option_flow/replacement_integration.rs`

- [ ] **Step 1: Create the child plan file**

Use this architecture statement:

```markdown
**Architecture:** Generalize effect activation costs so queued triggered effects can decline before process execution, then thread replacement cause/controller context through replacement predicates before exposing prevention choices.
```

- [ ] **Step 2: Define slices**

Include these slices:

```text
1. `.pay_cost()` for non-BeforePayCost triggered effects.
2. Optional cost decline path through pending selection.
3. Replacement context cause/controller predicate.
4. Partition source enforcement and selection.
5. Delay-as-replacement prevention.
6. Attack cancellation return path.
```

- [ ] **Step 3: Require regression fixtures**

The child plan must name these first fixtures:

```text
EX10-003 Tumblemon for attack cancellation.
BT16-025 Paildramon for Partition source enforcement.
BT17-097 Return to the Primogenitor for Delay-as-replacement.
EX9-032 / EX7-027 / BT22-036 for replacement cause gate.
```

- [ ] **Step 4: Commit the child plan**

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-3-cost-replacement.md
git commit -m "docs: plan cost replacement framework"
```

## Task 5: Create Child Plan for Zone Movement and Stack Operations

**Files:**
- Create: `docs/superpowers/plans/2026-04-29-gap-group-4-zone-movement.md`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/game.rs`
- Modify: `code/digimon-engine/src/player.rs`
- Modify: `code/digimon-engine/src/permanent.rs`
- Modify: `code/digimon-engine/src/card_source.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`
- Test: `code/digimon-engine/tests/effect_context/*.rs`
- Test: `code/digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 1: Create the child plan file**

The child plan must state that every zone helper accepts or derives movement cause, source player, and event emission behavior.

- [ ] **Step 2: Define slices**

Include these slices:

```text
1. Add pending security option to hand.
2. Effect-initiated digivolve from trash.
3. Effect-initiated digivolve from security.
4. Return to deck top/bottom helpers with source-scoped immunity checks.
5. Bottom-source placement and stack extraction helpers.
6. Effect-initiated move from breeding.
7. Security stack search/place/trash helpers.
```

- [ ] **Step 3: Commit the child plan**

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-4-zone-movement.md
git commit -m "docs: plan zone movement stack operations"
```

## Task 6: Create Child Plan for Option, Delay, Plug-In, Link, and Training State

**Files:**
- Create: `docs/superpowers/plans/2026-04-29-gap-group-5-option-delay-link.md`
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/game_phases.rs`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Modify: `code/digimon-engine/src/scheduled_effects.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_delay.rs`
- Modify: `code/digimon-engine/src/dsl_cards/timing_map.rs`
- Test: `code/digimon-engine/tests/option_flow/*.rs`
- Test: `code/digimon-engine/tests/dsl/delay.rs`

- [ ] **Step 1: Create the child plan file**

The child plan must separate Delay work into three slices: start-of-turn Delay, event-gated Delay, and replacement-window Delay.

- [ ] **Step 2: Define first fixtures**

Use these fixtures:

```text
LM-027 Red Scramble for StartOfYourNextTurn.
BT22-098 Unique Emblem: Fable Waltz for event-gated Delay.
BT17-097 Return to the Primogenitor for replacement Delay.
ST22-08 Offensive Plug-In V for Plug-In/Link.
BT13-110 Royal Knights of the Purge for option placement observers.
```

- [ ] **Step 3: Commit the child plan**

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-5-option-delay-link.md
git commit -m "docs: plan option delay link state"
```

## Task 7: Create Child Plan for Modifiers, Auras, and Keywords

**Files:**
- Create: `docs/superpowers/plans/2026-04-29-gap-group-6-modifiers-keywords.md`
- Modify: `code/digimon-engine/src/modifiers.rs`
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/combat.rs`
- Modify: `code/digimon-engine/src/action/mask.rs`
- Modify: `code/digimon-engine/src/tensor.rs`
- Modify: `code/digimon-engine/src/card_data.rs`
- Test: `code/digimon-engine/tests/combat/*.rs`
- Test: `code/digimon-engine/tests/flood_gates/*.rs`
- Test: `code/digimon-engine/tests/keyword_phase_*/`

- [ ] **Step 1: Create the child plan file**

The child plan must state that mask-affecting keywords require both mask and execution validation tests.

- [ ] **Step 2: Define slices**

Include these slices:

```text
1. IgnoreColorRequirement enforcement in option masks.
2. Source-scoped return/de-digivolve immunity modifiers.
3. Declarative aura to player-scoped modifier delivery.
4. Collision/Piercing/Reboot/Retaliation enforcement.
5. Overclock predicate parameterization.
6. Dynamic DP and Security A. formula-backed auras.
7. DigiXros scoped alias handling if not covered by card-data plan.
```

- [ ] **Step 3: Commit the child plan**

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-6-modifiers-keywords.md
git commit -m "docs: plan modifiers auras keywords"
```

## Task 8: Create Child Plan for DSL Predicate, Formula, and Lowering Coverage

**Files:**
- Create: `docs/superpowers/plans/2026-04-29-gap-group-7-dsl-predicates-formulas.md`
- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/formula.rs`
- Modify: `code/digimon-dsl/src/clause.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_triggered.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_aura.rs`
- Test: `code/digimon-engine/tests/dsl/parse_predicates.rs`
- Test: `code/digimon-engine/tests/dsl/parse_formulas.rs`
- Test: `code/digimon-engine/tests/dsl/phase3d_event_context.rs`
- Test: `code/digimon-engine/tests/dsl/phase3d_formula_zone_count.rs`

- [ ] **Step 1: Create the child plan file**

The child plan must divide pure DSL changes from hybrid changes that depend on engine primitives.

- [ ] **Step 2: Define pure DSL first batch**

Include these pure or mostly pure slices:

```text
1. `dp_lte` / `dp_gte` permanent predicate evaluation.
2. `not_in_binding` for for_each filters.
3. board-color cross-reference predicate.
4. play-cost-lte predicate.
5. formula filters for counted battle-area cards.
6. binding-DP formula support.
```

- [ ] **Step 3: Define hybrid batch**

Include these hybrid slices after engine event context exists:

```text
1. event-card trait/name/owner predicates.
2. replacement cause predicates.
3. self-digivolution-stack name predicate subject threading.
4. Delay event trigger lowerings.
5. `dna_costs` authoring and production data population.
6. dynamic aura formula fields.
```

- [ ] **Step 4: Commit the child plan**

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-7-dsl-predicates-formulas.md
git commit -m "docs: plan dsl predicates formulas"
```

## Task 9: Create Child Plan for Token and Card-Data Completion

**Files:**
- Create: `docs/superpowers/plans/2026-04-29-gap-group-8-token-card-data.md`
- Modify: `code/digimon-engine/src/token_registry.rs`
- Modify: `code/digimon-engine/src/card_data.rs`
- Modify: `code/digimon-engine/src/cards.rs`
- Modify: `code/digimon-engine/src/card_registry.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/tokens.rs`
- Test: `code/digimon-engine/tests/keyword_parsing.rs`
- Test: `code/digimon-engine/tests/dna_digivolve_user_action.rs`

- [ ] **Step 1: Create the child plan file**

The child plan must separate card-data schema changes from generated/loaded data population.

- [ ] **Step 2: Define slices**

Include these slices:

```text
1. Familiar Token On Deletion.
2. Token definitions and `CardKind::Token` invariants.
3. `CardData.dna_costs` YAML/data population.
4. DigiXros scoped aliases.
5. Ace Overflow metadata and leave-zone penalty.
6. Reveal-zone overlays.
```

- [ ] **Step 3: Commit the child plan**

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-8-token-card-data.md
git commit -m "docs: plan token card data completion"
```

## Task 10: Create Child Plan for Archetype Unlock Passes

**Files:**
- Create: `docs/superpowers/plans/2026-04-29-gap-group-9-archetype-unlocks.md`
- Modify: `qa/archetype-qa/INDEX.md`
- Modify: `qa/archetype-qa/medusa.md`
- Modify: `qa/archetype-qa/rocks.md`
- Modify: `qa/archetype-qa/royal-knights.md`
- Modify: `qa/archetype-qa/Puppets.md`
- Modify: `qa/archetype-qa/bg-imperial.md`
- Modify: `qa/archetype-qa/chaos_control.md`
- Modify: `qa/archetype-qa/DNA_Omnimon.md`
- Modify: `qa/archetype-qa/Dark_Masters.md`
- Modify: `qa/archetype-qa/dsl/*.md`

- [ ] **Step 1: Create the child plan file**

The child plan must use the archetype order from the roadmap spec:

```text
1. Medusamon
2. Rocks
3. Royal Knights
4. Puppets
5. BG Imperial
6. Chaos Control / DNA Omnimon
7. Dark Masters and remaining audits
```

- [ ] **Step 2: Define readiness command expectations**

For each archetype checkpoint, the child plan must instruct the worker to run or emulate the `assess-rust-engine-archetype` workflow and update the relevant QA docs with:

```text
- gaps closed by the current capability group
- gaps still blocking
- cards now ready for DSL implementation
- cards that still need hand-written Rust or raw-rust retirement
```

- [ ] **Step 3: Commit the child plan**

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-9-archetype-unlocks.md
git commit -m "docs: plan archetype unlock passes"
```

## Final Verification for the Parent Plan

- [ ] **Step 1: Check markdown for obvious placeholders**

Run:

```bash
$patterns = @(
  [string]::new([char[]](84,66,68)),
  [string]::new([char[]](84,79,68,79)),
  ('implement' + ' later'),
  ('fill in ' + 'details')
)
Select-String -Path 'docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md' -Pattern $patterns
```

Expected: no output.

- [ ] **Step 2: Check git diff whitespace**

Run:

```bash
git diff --check -- docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md
```

Expected: no output.

- [ ] **Step 3: Commit the parent plan**

Run:

```bash
git add docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md
git commit -m "docs: plan archetype gap roadmap execution"
```

Expected: commit succeeds with only the parent plan file staged.
