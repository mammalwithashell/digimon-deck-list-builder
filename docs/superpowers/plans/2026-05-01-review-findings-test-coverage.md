# Review Findings Test Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Address the outstanding Group 1, Group 2, and Group 3 review findings by adding targeted regression coverage and cleaning stale blocker metadata without broad engine refactors.

**Architecture:** Preserve the current Rust engine test structure. Add narrowly scoped tests near the existing coverage they strengthen, using existing `DebugRunner`, DSL lowering/runtime helpers, and action-space encoders. Treat every gameplay decision as an engine action or pending selection.

**Tech Stack:** Rust, Cargo test suites under `code/digimon-engine/tests`, YAML DSL lowering through `digimon-dsl`, engine test helpers in `digimon_engine::debug_runner`.

---

## File Structure

Files to edit:

- `code/digimon-engine/tests/dsl/phase3d_event_context.rs`
- `code/digimon-engine/tests/timing_dispatch.rs`
- `code/digimon-engine/tests/cards_behavioral/bt16/bt16_082.rs`
- `code/digimon-engine/tests/selection/source_multi.rs`
- `code/digimon-engine/tests/selection/dp_budget.rs`
- `code/digimon-engine/tests/dsl/phase2g_breeding_selection.rs`
- `code/digimon-engine/tests/option_flow/replacement_integration.rs`
- `code/digimon-engine/tests/replacements/attack_cancel.rs`

Commands to run from repo root:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test phase3d_event_context -- on_move
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- option_placed
cargo test --manifest-path code/digimon-engine/Cargo.toml --test bt16_082
cargo test --manifest-path code/digimon-engine/Cargo.toml --test source_multi
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dp_budget
cargo test --manifest-path code/digimon-engine/Cargo.toml --test phase2g_breeding_selection
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacement_integration -- bt17_097
cargo test --manifest-path code/digimon-engine/Cargo.toml --test attack_cancel
cargo test --manifest-path code/digimon-engine/Cargo.toml --tests
```

---

## Task 1: Group 1 - Prove Real OnMove Dispatch Carries DSL Event Context

**Finding:** `phase3d_event_context.rs` manually enqueues `TriggerSource::MovedFromBreeding`, so DSL coverage does not prove `Game::move_from_breeding` supplies the moved card payload. Hatch also needs a negative assertion.

Implementation:

- [ ] Add a DSL test that hatches a trait-bearing level 2 into breeding, records memory, calls `move_from_breeding`, and asserts the `on_move` observer with `event_target_trait_has` fires.
- [ ] Add a hatch-only negative test using the same trait-gated observer and assert memory does not change after `hatch`.
- [ ] Keep the existing manual trigger test if it still adds value for direct event-context lowering coverage.

Add near the existing OnMove DSL tests in `code/digimon-engine/tests/dsl/phase3d_event_context.rs`:

```rust
#[test]
fn on_move_real_breeding_dispatch_supplies_moved_card_context() {
    let yaml = r#"
cards:
  DSL-MOVE-OBS:
    name: Move Observer
    type: Digimon
    color: Red
    level: 3
    play_cost: 3
    dp: 1000
    effects:
      - trigger:
          when: on_move
        condition:
          event_target_trait_has: Rock
        then:
          - gain_memory: 2
"#;

    let mut moved_card = digimon_card("BABY-ROCK", "Rock Baby", &["Rock"], 1000);
    moved_card.level = Some(2);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(moved_card)
        .digitama(0, &["BABY-ROCK"])
        .memory(10)
        .build();

    runner.place_on_field(0, "DSL-MOVE-OBS", None);

    assert!(runner.game.hatch(0), "hatch Rock baby into breeding");
    let after_hatch = runner.memory();

    assert!(
        runner.game.move_from_breeding(0),
        "move Rock baby from breeding to battle"
    );
    assert_eq!(
        runner.memory(),
        after_hatch + 2,
        "real move dispatch should expose moved card trait context"
    );
}

#[test]
fn hatch_does_not_fire_on_move_observers() {
    let yaml = r#"
cards:
  DSL-MOVE-HATCH-OBS:
    name: Hatch Observer
    type: Digimon
    color: Red
    level: 3
    play_cost: 3
    dp: 1000
    effects:
      - trigger:
          when: on_move
        condition:
          event_target_trait_has: Rock
        then:
          - gain_memory: 2
"#;

    let mut moved_card = digimon_card("BABY-HATCH-ROCK", "Hatch Rock Baby", &["Rock"], 1000);
    moved_card.level = Some(2);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(moved_card)
        .digitama(0, &["BABY-HATCH-ROCK"])
        .memory(10)
        .build();

    runner.place_on_field(0, "DSL-MOVE-HATCH-OBS", None);
    let before = runner.memory();

    assert!(runner.game.hatch(0), "hatch Rock baby into breeding");
    assert_eq!(
        runner.memory(),
        before,
        "hatch into breeding must not fire OnMove"
    );
}
```

Validation:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test phase3d_event_context -- on_move
```

---

## Task 2: Group 1 - Add OnOptionPlaced Non-Placement Negative Coverage

**Finding:** `timing_dispatch.rs` proves `OnOptionPlaced` fires for Delay options that become battle-area permanents, but not that it stays silent for standard options that resolve to trash.

Implementation:

- [ ] Add a standard option effect that resolves through normal option play and is sent to trash.
- [ ] Register an `OnOptionPlacedObserver`.
- [ ] Play the standard option from hand.
- [ ] Assert the observer does not gain memory.
- [ ] Assert no battle-area option permanent is created.
- [ ] Assert the option is in trash.

Add near the existing `DelayOptionNoop` and OnOptionPlaced tests in `code/digimon-engine/tests/timing_dispatch.rs`:

```rust
struct StandardOptionNoop;

impl CardEffect for StandardOptionNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Standard option noop")
            .option_main()
            .process(|_ctx| {})
            .build()]
    }
}

#[test]
fn on_option_placed_does_not_fire_for_standard_option_sent_to_trash() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("OBS", "Observer"))
        .add_card(option_card("OPT-STANDARD", "Standard Option"))
        .hand(0, &["OBS", "OPT-STANDARD"])
        .memory(10)
        .start();

    r.game
        .registry
        .register("OBS", Arc::new(OnOptionPlacedObserver));
    r.game
        .registry
        .register("OPT-STANDARD", Arc::new(StandardOptionNoop));

    r.play_card_from_hand(0, 0);
    r.enter_main_phase();

    let before_memory = r.memory();
    let before_battle_area = r.game.battle_area_size(0);

    let result = r.game.play_option_from_hand(0, 0);

    assert_eq!(
        result,
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    assert_eq!(
        r.memory(),
        before_memory,
        "OnOptionPlaced must not fire for standard options"
    );
    assert_eq!(
        r.game.battle_area_size(0),
        before_battle_area,
        "standard options must not create battle-area option permanents"
    );
    assert!(
        r.game
            .player(0)
            .trash
            .iter()
            .any(|card| card.card_id(&r.game.card_data) == "OPT-STANDARD"),
        "standard option should resolve to trash"
    );
}
```

Validation:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- option_placed
```

---

## Task 3: Group 1 - Clean BT16-082 Stale G-ON-MOVE Blocker Metadata

**Finding:** `bt16_082.rs` still marks shared G-ON-MOVE timing as the blocker, even though shared OnMove dispatch/DSL coverage is now handled elsewhere. The real BT16-082 card path should either be unignored where possible or clearly point at the remaining card-specific blocker.

Implementation:

- [ ] Inspect `data/cards.yaml` or the DSL fixture for `BT16-082` and confirm whether the card body is still a raw Rust/no-op placeholder.
- [ ] If a test only needs generic OnMove dispatch and now passes, remove its `#[ignore]`.
- [ ] If the BT16-082 card body still cannot pass because card-specific behavior is not implemented, keep the test ignored but update the reason to the remaining blocker.
- [ ] Update the file header comments so they no longer claim shared OnMove dispatch is unavailable.

Expected metadata direction if the card remains blocked:

```rust
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
```

Validation:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test bt16_082
```

---

## Task 4: Group 2 - Add Negative Source Selection Mask Assertions

**Finding:** `source_multi.rs` only asserts expected source actions are present. It does not prove top cards are excluded or that an already-picked source becomes illegal.

Implementation:

- [ ] In `exact_two_sources_can_be_selected_across_own_battle_area`, encode action IDs for both legal sources and illegal top cards.
- [ ] Assert top-card source indices are absent before any pick.
- [ ] After the first source pick, assert the picked source action is no longer legal.
- [ ] Keep the existing positive path and final callback assertions.

Patch the first selection assertions in `code/digimon-engine/tests/selection/source_multi.rs`:

```rust
let first_source_a = encode_source_select(first.index as u16, 0).unwrap();
let first_source_b = encode_source_select(first.index as u16, 1).unwrap();
let first_top_action = encode_source_select(first.index as u16, 2).unwrap();
let second_source_c = encode_source_select(second.index as u16, 0).unwrap();
let second_top_action = encode_source_select(second.index as u16, 1).unwrap();

assert!(sel.valid_action_ids.contains(&first_source_a));
assert!(sel.valid_action_ids.contains(&first_source_b));
assert!(sel.valid_action_ids.contains(&second_source_c));
assert!(
    !sel.valid_action_ids.contains(&first_top_action),
    "top card of first stack must not be selectable as a source"
);
assert!(
    !sel.valid_action_ids.contains(&second_top_action),
    "top card of second stack must not be selectable as a source"
);
```

Then after resolving `first_source_b`:

```rust
let sel = r
    .game
    .pending_selection
    .as_ref()
    .expect("selection should continue after first source");
assert!(
    !sel.valid_action_ids.contains(&first_source_b),
    "already-picked source must not remain legal"
);
assert!(
    !sel.valid_action_ids.contains(&first_top_action),
    "top card of first stack must remain illegal"
);
assert!(
    !sel.valid_action_ids.contains(&second_top_action),
    "top card of second stack must remain illegal"
);
```

Validation:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test source_multi
```

---

## Task 5: Group 2 - Add DP-Budget Mandatory Minimum PASS Gating Coverage

**Finding:** `dp_budget.rs` has engine-level coverage for `min_picks = 0`, and DSL coverage for `min_picks = 1`, but no engine-level assertion that PASS is hidden/rejected before a mandatory pick and available after the minimum is met.

Implementation:

- [ ] Add a new engine test with `min_picks = 1`.
- [ ] Assert `PASS` is absent before the first pick.
- [ ] Assert resolving `PASS` before the first pick returns `SelectionError::InvalidAction`.
- [ ] Pick one legal target.
- [ ] Assert `PASS` is present after the minimum is met.
- [ ] Resolve `PASS` and assert the callback receives exactly the selected target.

Add to `code/digimon-engine/tests/selection/dp_budget.rs`:

```rust
#[test]
fn dp_budget_selection_hides_pass_until_minimum_pick_is_met() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("LOW", "Low"))
        .add_card(make_test_card("MID", "Mid"))
        .start();

    let p0 = 0;
    let p1 = 1;
    let source = r.place_on_field(p0, "SRC", Some(0));
    r.force_base_dp("LOW", 3000);
    r.force_base_dp("MID", 3000);
    let low = r.place_on_field(p1, "LOW", Some(0));
    let _mid = r.place_on_field(p1, "MID", Some(0));

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);

    {
        let mut ctx = EffectContext::new(&mut r.game, p0, source);
        ctx.select_opponent_permanents_by_dp_budget(
            "delete at least one",
            6000,
            1,
            |_, _| true,
            move |_, handles| {
                *picked_slot.lock().unwrap() = handles;
            },
        );
    }

    let selection = r
        .game
        .pending_selection
        .as_ref()
        .expect("DP-budget selection should be pending");
    assert!(
        !selection.valid_action_ids.contains(&PASS),
        "PASS must be hidden before mandatory minimum is met"
    );
    assert!(!selection.is_optional);
    assert_eq!(
        r.game.resolve_selection(p0, PASS),
        Err(digimon_engine::selection::SelectionError::InvalidAction)
    );

    r.game
        .resolve_selection(p0, encode_attack(0, low.index as u16))
        .expect("pick first DP-budget target");

    let selection = r
        .game
        .pending_selection
        .as_ref()
        .expect("DP-budget selection should continue after first pick");
    assert!(
        selection.valid_action_ids.contains(&PASS),
        "PASS should be available after mandatory minimum is met"
    );
    assert!(selection.is_optional);

    r.game
        .resolve_selection(p0, PASS)
        .expect("PASS after minimum should commit selection");

    assert_eq!(picked.lock().unwrap().as_slice(), &[low]);
    assert!(r.game.pending_selection.is_none());
}
```

Validation:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dp_budget
```

---

## Task 6: Group 2 - Verify Breeding DSL bind_as Inserts the Selected Binding

**Finding:** `phase2g_breeding_selection.rs` names a binding but the tail only gains memory, so the test would pass even if `SelectOwnBreedingPermanent` resumed without inserting `BreedingPermanentRef`.

Implementation:

- [ ] Extend the test imports to include `Arc`, `Mutex`, `EngineRawRustRegistry`, `run_steps_with_runtime`, and `StepRuntime`.
- [ ] Add a raw Rust assertion step after the breeding selection that consumes the binding.
- [ ] Register the raw Rust assertion in a local runtime and capture the binding into test state.
- [ ] Assert the captured binding equals the selected breeding permanent handle.

Expected import additions in `code/digimon-engine/tests/dsl/phase2g_breeding_selection.rs`:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::step::{
    run_steps, run_steps_with_runtime, RunOutcome, StepRuntime,
};
```

Add or update a binding-focused test:

```rust
#[test]
fn select_own_breeding_permanent_binds_selected_ref() {
    let mut game = DebugRunner::builder()
        .add_card(make_test_card("EGG", "Egg"))
        .digitama(0, &["EGG"])
        .memory(0)
        .start()
        .game;

    assert!(game.hatch(0));
    let breeding_ref = game.player(0).breeding_area.expect("breeding permanent");

    let steps = vec![
        CompiledStep::SelectOwnBreedingPermanent {
            label: "choose breeding".to_string(),
            bind_as: "breeding_target".to_string(),
        },
        CompiledStep::RawRust {
            fn_name: "assert_breeding_binding".to_string(),
            consumes: vec!["breeding_target".to_string()],
            binds: vec![],
        },
    ];

    let seen = Arc::new(Mutex::new(None));
    let seen_slot = Arc::clone(&seen);
    let mut raw = EngineRawRustRegistry::new();
    raw.register_step("assert_breeding_binding", move |_ctx, bindings| {
        *seen_slot.lock().unwrap() =
            bindings.get_breeding_permanent_ref("breeding_target");
    });
    let runtime = StepRuntime::new(Arc::new(raw));

    let mut bindings = Bindings::default();
    let outcome = {
        let mut ctx = EffectContext::new(&mut game, 0, breeding_ref);
        run_steps_with_runtime(&mut ctx, &steps, 0, &mut bindings, &runtime)
    };

    assert_eq!(outcome, RunOutcome::Pending);
    game.resolve_selection(0, encode_breeding_select(0)).unwrap();

    assert_eq!(*seen.lock().unwrap(), Some(breeding_ref));
}
```

If the exact binding getter name differs, inspect `code/digimon-engine/src/dsl_cards/bindings.rs` and use the existing typed accessor rather than adding a new one unless no accessor exists.

Validation:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test phase2g_breeding_selection
```

---

## Task 7: Group 3 - Cover Paid BT17-097 Delay Decline Path

**Finding:** `replacement_integration.rs` covers accepting the paid BT17-097 hand choice, but not paying the Delay cost and then declining the hand-choice prompt with `PASS`.

Implementation:

- [ ] Add `PASS` to the action-space imports if absent.
- [ ] Create a test with BT17-097 in battle area as a Delay option, a Free target in battle, and an Imperialdramon card in hand.
- [ ] Trigger opponent-effect deletion of the Free target.
- [ ] Resolve the hand-choice pending selection with `PASS`.
- [ ] Assert the original Free target is deleted.
- [ ] Assert BT17-097 is in trash because the Delay cost was paid.
- [ ] Assert the hand card remains in hand.

Add near the existing BT17-097 replacement tests in `code/digimon-engine/tests/option_flow/replacement_integration.rs`:

```rust
#[test]
fn bt17_097_paid_delay_decline_commits_original_deletion() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT17-097")
        .expect("BT17-097 DSL card should load")
        .add_card(trait_digimon_card("FREE-TARGET", "Free Target", &["Free"]))
        .add_card(trait_digimon_card(
            "IMPERIAL-HAND",
            "Imperial Hand",
            &["Imperialdramon"],
        ))
        .hand(0, &["IMPERIAL-HAND"])
        .memory(0)
        .start();

    let target = r.place_on_field(0, "FREE-TARGET", Some(0));
    place_delay_option(&mut r, 0, "BT17-097");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_some(),
        "paid BT17-097 replacement should ask for a hand choice"
    );
    r.game
        .resolve_selection(0, PASS)
        .expect("decline paid Delay hand choice");

    assert!(
        find_battle_permanent(&r, 0, "FREE-TARGET").is_none(),
        "declining the hand choice should allow the original deletion"
    );
    assert!(
        trash_contains(&r, 0, "BT17-097"),
        "BT17-097 should be trashed after paying its Delay cost"
    );
    assert!(
        r.game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&r.game.card_data) == "IMPERIAL-HAND"),
        "declined hand card should remain in hand"
    );
}
```

Validation:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacement_integration -- bt17_097
```

---

## Task 8: Group 3 - Prove Source-Cost Attack Cancel Runs Normal EndOfAttack Cleanup

**Finding:** `attack_cancel.rs` verifies `pending_attack` clears after EX10-003 pays three sources, but does not prove the engine went through normal attack cleanup and fired EndOfAttack observers.

Implementation:

- [ ] Add or reuse an `EndOfAttack` witness effect in `attack_cancel.rs`.
- [ ] Register the witness in the EX10-003 source-cost cancel scenario.
- [ ] Place the witness on board before the attack.
- [ ] Assert the witness count is `0` before the third source is selected.
- [ ] Assert the witness count is exactly `1` after the third source selection resolves the cancellation.

Add the witness helper in `code/digimon-engine/tests/replacements/attack_cancel.rs`:

```rust
struct EndOfAttackWitness(Arc<Mutex<u32>>);

impl CardEffect for EndOfAttackWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let fired = Arc::clone(&self.0);
        vec![Effect::end_of_attack(card)
            .name("EndOfAttack witness")
            .process(move |_ctx| {
                *fired.lock().unwrap() += 1;
            })
            .build()]
    }
}
```

Then amend `ex10_003_pay_cost_can_end_pending_attack`:

```rust
let end_of_attack_count = Arc::new(Mutex::new(0));
r.game.registry.register(
    "EOA-WITNESS",
    Arc::new(EndOfAttackWitness(Arc::clone(&end_of_attack_count))),
);
r.place_on_field(0, "EOA-WITNESS", Some(0));
```

After the second source selection:

```rust
assert_eq!(*end_of_attack_count.lock().unwrap(), 0);
```

After the third source selection resolves the cancel:

```rust
assert_eq!(
    *end_of_attack_count.lock().unwrap(),
    1,
    "source-cost attack cancel should run normal EndOfAttack cleanup exactly once"
);
```

Validation:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test attack_cancel
```

---

## Task 9: Final Verification

Run targeted verification first, then the broader Rust engine suite:

- [ ] `cargo test --manifest-path code/digimon-engine/Cargo.toml --test phase3d_event_context -- on_move`
- [ ] `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- option_placed`
- [ ] `cargo test --manifest-path code/digimon-engine/Cargo.toml --test bt16_082`
- [ ] `cargo test --manifest-path code/digimon-engine/Cargo.toml --test source_multi`
- [ ] `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dp_budget`
- [ ] `cargo test --manifest-path code/digimon-engine/Cargo.toml --test phase2g_breeding_selection`
- [ ] `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacement_integration -- bt17_097`
- [ ] `cargo test --manifest-path code/digimon-engine/Cargo.toml --test attack_cancel`
- [ ] `cargo test --manifest-path code/digimon-engine/Cargo.toml --tests`

If any test fails because an engine behavior is actually missing, stop and apply `superpowers:systematic-debugging` before changing implementation code.
