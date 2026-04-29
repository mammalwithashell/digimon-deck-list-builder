# Gap Group 1: Inherited Dispatch + OPT Enforcement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make triggered inherited effects in digivolution stacks dispatch correctly and enforce once-per-turn limits on triggered effects.

**Architecture:** Extend `effect_queue::enqueue_from_permanent` so it scans inherited source cards below the top card when an effect is marked inherited. Reuse the existing trigger context and activation-count path so inherited effects receive the carrier permanent and obey `once_per_turn` / `max_per_turn` rules.

**Tech Stack:** Rust digimon-engine, Cargo integration tests, DebugRunner/card behavioral fixtures.

---

## Scope Check

This child plan covers only:

- `G-INHERITED-DISPATCH`: triggered inherited effects from cards below a permanent's top card must dispatch when their timing matches.
- `G-OPT-TRIGGERED`: triggered effects, including triggered inherited effects, must obey `once_per_turn` / `max_per_turn` through the same activation-count path used by activated effects.

This child plan explicitly defers these related but separate roadmap items to later child plans:

- Broad event context followups: `OnMove`, `OnDigivolve` trait filters, `OnEnterFieldAnyone` trait filters, `OnOptionPlaced`, attack observer payloads, and richer security event payloads.
- Source-trash dispatch: `OnDigivolutionCardTrashed`, source-card trash cause payloads, and host/source predicates.
- Breeding dispatch: breeding-area trigger fan-out and breeding-area source permanent addressing.
- Selection and action-space work: source-card selection, optional trigger branch choices, action-mask resizing, and new pending-selection variants.

Do not edit DSL lowerers, card YAML, action/tensor contracts, breeding-area dispatch, or QA trackers until the implementation tests in this plan pass.

## File Structure

Implementation files for the later code slice:

- Modify: `code/digimon-engine/src/effect_queue.rs`
  - Extend `enqueue_from_permanent` to scan inherited source cards below the top card after the existing top-card, linked-card, and Training scans.
  - Ensure triggered queue drain checks `max_per_turn` before process execution and records activation after successful execution.
- Modify if needed: `code/digimon-engine/src/effect.rs`
  - Only adjust effect metadata or helper accessors if existing fields do not expose `inherited`, `max_per_turn`, effect slot, or timing matching cleanly.
- Modify if needed: `code/digimon-engine/src/trigger_context.rs`
  - Only adjust trigger/source attribution helpers if the existing context cannot carry carrier permanent plus inherited source-card attribution.

Regression tests:

- Test: `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs`
  - Add the real-card inherited-dispatch regression for BT21-008 under a carrier.
  - Add the same-card once-per-turn regression for two security-removal events in one turn.
- Test: `code/digimon-engine/tests/effects/queue_drainer.rs`
  - Add a small queue-level regression that a buried non-inherited top-card effect does not dispatch from source position, if a card-behavioral fixture would be too noisy.

Tracker docs for the implementation slice, after tests pass:

- Docs: `qa/archetype-qa/engine-gaps.md`
  - Update `G-INHERITED-DISPATCH`.
  - Update `G-OPT-TRIGGERED` if the implementation closes it for triggered queue dispatch.
  - Update linked Medusamon notes and Rocks notes only where this slice genuinely changes the blocker wording.
- Docs: `docs/RUST_ENGINE_GAPS.md`
  - Narrow or close the inherited triggered-effect dispatch entry if the implementation proves the full stack walk.
  - Leave source-trash, breeding, and selection blockers open.
- Docs: `qa/dsl-vocab-gaps.md`
  - No expected change for this slice unless implementation proves a DSL gap note can be narrowed without changing DSL vocabulary.

## Task 1: Add Failing Inherited Dispatch Test

**Files:**
- Test: `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs`

- [ ] **Step 1: Inspect the existing BT21-008 fixture**

Read the file and identify the current DebugRunner setup helpers:

```bash
rg -n "BT21-008|bt21_008|play_digimon_with_sources|trash_top_security|drain_effects" code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs code/digimon-engine/tests -g "*.rs"
```

Expected: existing helper names or nearby patterns for creating a carrier with sources, removing security by effect, and draining the queue.

- [ ] **Step 2: Add the failing inherited BT21-008 security-removed test**

Add this test to `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs`. If helper names differ, adapt to existing DebugRunner helpers with identical semantics; do not skip the test.

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

- [ ] **Step 3: Run the focused card test and confirm failure**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_008
```

Expected before implementation: the new inherited-dispatch test fails because the source-card inherited trigger is not enqueued from below the carrier top card.

## Task 2: Add Failing Triggered OPT Test

**Files:**
- Test: `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs`
- Test if better isolated: `code/digimon-engine/tests/effects/queue_drainer.rs`

- [ ] **Step 1: Add same-turn double-event OPT coverage**

Prefer the real-card BT21-008 fixture. Add this test to `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs`. If the existing fixture cannot create two security-removal events with current helpers, place an equivalent queue-level test in `code/digimon-engine/tests/effects/queue_drainer.rs` using a test effect with `max_per_turn = 1`.

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

If helper names differ, adapt to existing DebugRunner helpers with identical semantics; do not skip the test.

- [ ] **Step 2: Run the focused tests and confirm failure**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_008
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effects -- queue
```

Expected before implementation: at least one new test fails. The failure should show either missing inherited dispatch or triggered queue drain ignoring `max_per_turn`.

## Task 3: Add Buried Non-Inherited Regression

**Files:**
- Test: `code/digimon-engine/tests/effects/queue_drainer.rs`
- Test if fixture already exists: `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs`

- [ ] **Step 1: Add a buried non-inherited effect test**

Add a regression proving that scanning source cards does not make ordinary top-card effects fire from source position. Use the existing queue-drainer fixture style if it already has synthetic cards/effects; otherwise use a real card with a non-inherited triggered effect as the buried source.

The test must assert these semantics:

```rust
#[test]
fn buried_non_inherited_triggered_effect_does_not_fire_from_source_position() {
    let mut runner = DebugRunner::new();
    runner.load_card_data();
    runner.setup_basic_game();

    runner
        .play_digimon_with_sources(0, "BT21-017", vec!["NON_INHERITED_TRIGGER_CARD"])
        .expect("carrier with buried non-inherited source");

    runner.set_memory(0, 0);
    runner.fire_matching_event_for_non_inherited_trigger();
    runner.drain_effects();

    assert_eq!(
        runner.memory(),
        0,
        "buried non-inherited top-card effects must not dispatch from source position"
    );
}
```

Replace `NON_INHERITED_TRIGGER_CARD` and `fire_matching_event_for_non_inherited_trigger` with existing fixture names that have identical semantics. If no real card fixture is appropriate, use queue-drainer test helpers to register a synthetic non-inherited triggered effect and a synthetic event. Do not mark this as ignored.

- [ ] **Step 2: Run the queue test and confirm failure or red/green coverage**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effects -- queue
```

Expected before implementation: if the source scan is not yet implemented, this test may already pass. Keep it as a guard so the implementation cannot over-scan all source effects.

## Task 4: Implement Inherited Source Dispatch

**Files:**
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Modify if needed: `code/digimon-engine/src/effect.rs`
- Modify if needed: `code/digimon-engine/src/trigger_context.rs`

- [ ] **Step 1: Preserve existing top-card dispatch**

In `effect_queue::enqueue_from_permanent`, keep the current top-card scan behavior intact. Do not replace the top-card branch with a full-stack loop unless the top-card branch still preserves existing source-card, source-permanent, linked-card, and Training behavior.

- [ ] **Step 2: Add inherited source-card scan after existing scans**

After the top-card scan and after the existing linked-card / Training scans, iterate source cards below the top card:

```rust
let stack_len = perm.card_sources.len();
if stack_len > 1 {
    for source_index in 0..(stack_len - 1) {
        let source = &perm.card_sources[source_index];
        // Fetch effects through the same registry path used for top-card effects.
        // Enqueue only effects with effect.inherited == true and matching timing.
    }
}
```

Keep this scan after the existing top-card / linked / Training scans so existing trigger ordering does not move unexpectedly.

- [ ] **Step 3: Fetch effects through the existing registry path**

Use the same card effect registry path currently used by `enqueue_from_permanent`, such as `effects_for_card(card_id, source_card_handle)` or the local equivalent. Do not instantiate card effects through a new side path.

For each source-card effect:

```rust
if !effect.inherited {
    continue;
}
if !timing_flag_matches(&effect, timing) {
    continue;
}
```

Use the repository's existing timing helper name and signature if it differs from the sketch above.

- [ ] **Step 4: Set carrier and source-card attribution correctly**

When queuing an inherited source-card effect, set trigger context and queued attribution so:

- `source_permanent` is the carrier permanent, not a synthetic source permanent.
- `source_card` / card source attribution is the inherited source card below the top card.
- The queued effect's controller is the carrier permanent's controller, matching normal inherited effect ownership.
- Any existing event payload in the trigger context is preserved unchanged.

Follow the same attribution pattern as the existing linked-card branch where possible.

- [ ] **Step 5: Avoid hidden event broadening**

Do not add new event timings, event payload fields, source-trash fan-out, or breeding-area scan paths in this slice. This implementation should only make existing dispatch calls see inherited source-card effects when those calls already fan out over a battle-area carrier permanent.

## Task 5: Enforce `once_per_turn` / `max_per_turn` for Queued Triggered Effects

**Files:**
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Modify if needed: `code/digimon-engine/src/effect.rs`
- Modify if needed: `code/digimon-engine/src/trigger_context.rs`

- [ ] **Step 1: Locate the current activation-count helpers**

Find the activated-effect path that already checks and records activations:

```bash
rg -n "max_per_turn|activation_count|record_activation|once_per_turn" code/digimon-engine/src
```

Expected: existing `Permanent::activation_count` and `Permanent::record_activation` usage, likely in activated Main-effect logic.

- [ ] **Step 2: Add the same guard to queued triggered execution**

Before invoking a queued triggered effect's process closure in the queue drain path, check the activation count for the queued source permanent/source card/effect slot:

```rust
if let Some(max) = effect.max_per_turn {
    if source_permanent.activation_count(source_card, effect_slot) >= max {
        return;
    }
}
```

Adapt `effect.max_per_turn`, `source_card`, `effect_slot`, and return behavior to the existing types. The key requirement is that triggered effects route through the same activation-count/once-per-turn checks as top-card activated effects.

- [ ] **Step 3: Record activation only after successful execution**

After the queued effect successfully invokes its process closure, call the existing activation recording helper:

```rust
source_permanent.record_activation(source_card, effect_slot);
```

Use the existing helper signature and mutation access pattern. If the queue path can create pending selections before final resolution, record at the same semantic point the current engine treats an activation as consumed for activated effects.

- [ ] **Step 4: Preserve no-op behavior for unlimited effects**

Effects with no `max_per_turn` limit must behave exactly as before. Add no new global counter for effects that are not limited.

## Task 6: Run Verification

**Files:**
- Test: `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs`
- Test: `code/digimon-engine/tests/effects/queue_drainer.rs`
- Verify: `code/digimon-engine/src/effect_queue.rs`
- Verify if changed: `code/digimon-engine/src/effect.rs`
- Verify if changed: `code/digimon-engine/src/trigger_context.rs`

- [ ] **Step 1: Run BT21-008 behavioral tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_008
```

Expected after implementation: all non-ignored `bt21_008` tests pass, including inherited dispatch and inherited OPT coverage.

- [ ] **Step 2: Run queue-drainer tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effects -- queue
```

Expected after implementation: queue tests pass, including the buried non-inherited regression and any queue-level OPT coverage.

- [ ] **Step 3: Run whitespace diff check for changed files**

Run with the actual implementation/docs files touched:

```bash
git diff --check -- code/digimon-engine/src/effect_queue.rs code/digimon-engine/src/effect.rs code/digimon-engine/src/trigger_context.rs code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs code/digimon-engine/tests/effects/queue_drainer.rs qa/archetype-qa/engine-gaps.md docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
```

Expected: no output.

## Task 7: Update Trackers After Tests Pass

**Files:**
- Docs: `qa/archetype-qa/engine-gaps.md`
- Docs if affected: `docs/RUST_ENGINE_GAPS.md`
- Docs if affected: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Update `G-INHERITED-DISPATCH`**

In `qa/archetype-qa/engine-gaps.md`, update `G-INHERITED-DISPATCH` only after both targeted cargo commands pass. Mark it resolved if inherited stack dispatch is complete for battle-area carrier permanents. If breeding or source-trash inherited effects remain out of scope, state that they are covered by separate gap entries and not by this resolved item.

- [ ] **Step 2: Update `G-OPT-TRIGGERED`**

If the triggered queue path now enforces `max_per_turn` for all queued triggered effects, mark `G-OPT-TRIGGERED` resolved or narrow it to any remaining pending-selection edge case discovered during implementation. Cite the BT21-008 same-turn double-event regression or queue-drainer test as evidence.

- [ ] **Step 3: Update linked Medusamon and Rocks notes**

For Medusamon notes, mention that BT21-008-style inherited security-removed observers now dispatch from a source under a battle-area carrier and obey once-per-turn.

For Rocks notes, do not claim source-trash dispatch is fixed. Narrow only the inherited-dispatch prerequisite if relevant, and keep `OnDigivolutionCardTrashed`, source selection, and triggered-body cost ordering open.

- [ ] **Step 4: Update `docs/RUST_ENGINE_GAPS.md` only if the implementation evidence maps cleanly**

Close or narrow the "Inherited triggered-effect dispatch: `enqueue_from_permanent` must walk digivolution stack" entry if the new tests prove that behavior. Keep broader event-context, source-trash, breeding, and selection gaps open.

## Task 8: Commit Implementation Slice

**Files:**
- Source/tests/docs actually changed by the implementation worker.

- [ ] **Step 1: Review staged scope**

Run:

```bash
git status --short
git diff --stat
```

Expected: only files from this implementation slice are modified.

- [ ] **Step 2: Commit implementation**

After tests pass and trackers are updated, commit the implementation slice:

```bash
git add code/digimon-engine/src/effect_queue.rs code/digimon-engine/src/effect.rs code/digimon-engine/src/trigger_context.rs code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs code/digimon-engine/tests/effects/queue_drainer.rs qa/archetype-qa/engine-gaps.md docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
git commit -m "feat: dispatch inherited triggered effects"
```

If one of the optional files was not changed, omit it from `git add`. Do not include unrelated files.

## Plan-File Verification and Commit

- [ ] **Step 1: Run placeholder scan for this plan**

Run:

```bash
$patterns = @(
  [string]::new([char[]](84,66,68)),
  [string]::new([char[]](84,79,68,79)),
  ('implement' + ' later'),
  ('fill in ' + 'details')
)
Select-String -Path 'docs/superpowers/plans/2026-04-29-gap-group-1-inherited-dispatch-opt.md' -Pattern $patterns
```

Expected: no output.

- [ ] **Step 2: Run whitespace diff check for this plan**

Run:

```bash
git diff --check -- docs/superpowers/plans/2026-04-29-gap-group-1-inherited-dispatch-opt.md
```

Expected: no output.

- [ ] **Step 3: Commit only this child plan file**

Run:

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-1-inherited-dispatch-opt.md
git commit -m "docs: plan inherited dispatch opt"
```

Expected: commit succeeds with only `docs/superpowers/plans/2026-04-29-gap-group-1-inherited-dispatch-opt.md` staged.
