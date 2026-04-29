# Gap Group 1: Event Context Follow-Ups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the missing event payloads and dispatch sites that unblock OnMove, OnDigivolve trait filters, OnEnterFieldAnyone trait filters, option placement, and source-trash observers.

**Architecture:** Extend the existing Rust event-dispatch layer by adding narrow trigger payloads to `TriggerSource` / `TriggerContext` and firing them at the exact state-machine sites where the source event commits. Keep action and tensor contracts unchanged: these slices add observer fan-out and read-only event context, not new player choices. Each timing lands with a failing Rust test before engine changes and a tracker update after targeted tests pass.

**Tech Stack:** Rust `digimon-engine`, `digimon-dsl` timing lowering, Cargo integration tests, DebugRunner fixtures, markdown gap trackers.

---

## Scope Check

This child plan covers only event payloads, event-log emission, and dispatch sites for these timings, in this exact order:

1. `OnMove` dispatch from breeding-to-battle movement.
2. `GameEvent::Digivolve` emission with newly-digivolved permanent in context.
3. `OnEnterFieldAnyone` context with entering permanent/card.
4. `OnOptionPlaced` timing and dispatch after option placement.
5. `OnDigivolutionCardTrashed` with host permanent and trashed source card in context.
6. `OnAllyAttack` / `OnOpponentAttack` declared-attack observer dispatch.

This child plan does not add new pending-selection kinds, resize `ACTION_SPACE_SIZE`, change `TENSOR_SIZE`, implement source-selection costs, implement replacement prevention, or author production card YAML. If a slice exposes a missing player-visible choice while testing, record it in the relevant tracker and leave it for the selection/cost child plans.

Before starting any implementation task, read:

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

## File Structure

Implementation files:

- Modify: `code/digimon-engine/src/enums.rs`
  - Add missing `EffectTiming` variants and effect-builder constructors only where the timing does not already exist.
  - Expected additions in this plan: `OnMove` and `OnOptionPlaced`.
- Modify: `code/digimon-engine/src/effect.rs`
  - Add `Effect::on_move(card)` and `Effect::on_option_placed(card)` builder helpers if absent.
- Modify: `code/digimon-engine/src/selection.rs`
  - Extend `TriggerSource` with event-specific variants that carry stable handles: moving permanent, digivolved permanent, entering permanent/card, placed option, source-trash host/card, attack attacker/target.
- Modify: `code/digimon-engine/src/trigger_context.rs`
  - Extend `TriggerContext` with explicit read-only payload fields.
  - Required fields by the end of this plan: `event_permanent`, `event_card`, `event_source_card`, `event_host_permanent`, `attack_attacker`, `attack_target`, and `source_player`.
- Modify: `code/digimon-engine/src/effect_context.rs` and related submodules if accessors are split out.
  - Add script-facing accessors used by conditions and DSL lowering:
    `event_permanent()`, `event_card()`, `event_source_card()`, `event_host_permanent()`, `attack_attacker()`, and `attack_target()`.
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
  - Use the new trigger context for `event_card_trait_has`, `event_target_kind`, `event_target`, and related event predicates.
- Modify: `code/digimon-engine/src/dsl_cards/timing_map.rs`
  - Map DSL timings to the new engine timings for `on_move` and `on_option_placed`.
- Modify: `code/digimon-dsl/src/compiled.rs` and parser/lowering files only if the token is not already represented in `CompiledTiming`.
- Modify: `code/digimon-engine/src/events.rs`
  - Emit `GameEvent::Digivolve` from digivolve commit paths and keep event-log payload stable.
- Modify: `code/digimon-engine/src/game_actions.rs`
  - Fire `OnMove`, `OnDigivolve`, `OnEnterFieldAnyone`, `OnOptionPlaced`, and source-trash events at commit sites.
- Modify: `code/digimon-engine/src/game_phases.rs`
  - Fire option placement observers if placement is handled during option disposition from phase code.
- Modify: `code/digimon-engine/src/combat.rs`
  - Fire attack observer timings at declared-attack time and preserve existing `OnAttack` / `WhenAttacking` order.

Tests:

- Test: `code/digimon-engine/tests/timing_dispatch.rs`
  - Add one failing dispatch test per event when a small hand-written fixture is clearest.
- Test: `code/digimon-engine/tests/dsl/phase3d_event_context.rs`
  - Add DSL predicate/binding tests for event context fields, especially trait filters.
- Test when cleaner: relevant real-card behavioral tests under `code/digimon-engine/tests/cards_behavioral/<set>/<card>.rs`
  - Use this lane when a named tracker fixture already exposes the exact event semantics.

Trackers:

- Docs: `docs/RUST_ENGINE_GAPS.md`
- Docs: `qa/archetype-qa/engine-gaps.md`
- Docs: `qa/dsl-vocab-gaps.md`

## Task 1: `OnMove` Dispatch From Breeding-To-Battle Movement

**Files:**
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/effect.rs`
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/trigger_context.rs`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/dsl_cards/timing_map.rs`
- Modify if needed: `code/digimon-dsl/src/compiled.rs`
- Test: `code/digimon-engine/tests/timing_dispatch.rs`
- Test: `code/digimon-engine/tests/dsl/phase3d_event_context.rs`

- [ ] **Step 1: Write the failing engine dispatch test**

Add this test to `code/digimon-engine/tests/timing_dispatch.rs`. Adapt only helper names that already differ in the file; keep the semantics identical.

```rust
struct OnMoveObserver;
impl CardEffect for OnMoveObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_move(card)
            .name("OnMove observer gains memory")
            .condition(|ctx| ctx.event_permanent().is_some())
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

#[test]
fn on_move_fires_after_breeding_permanent_moves_to_battle() {
    let filler: Vec<&str> = vec!["FILLER"; 5];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS", "Move Observer", 3))
        .add_card(plain_digimon("BABY", "Baby", 0))
        .add_card(plain_digimon("FILLER", "Filler", 1))
        .hand(0, &["OBS"])
        .digitama(0, &["BABY"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(OnMoveObserver));

    assert_eq!(r.play(0, 0), Some(0));
    assert!(r.game.hatch(0), "hatch BABY into breeding");

    let before = r.memory();
    assert!(r.game.move_from_breeding(0), "breeding permanent should move");

    assert_eq!(r.memory(), before + 1);
}
```

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_move_fires_after_breeding_permanent_moves_to_battle
```

Expected before implementation: FAIL because `Effect::on_move`, `EffectTiming::OnMove`, or the dispatch site is missing.

- [ ] **Step 2: Write the failing DSL event-context test**

Add a DSL test to `code/digimon-engine/tests/dsl/phase3d_event_context.rs` proving `[When Moving]` can filter the moved permanent by trait:

```rust
#[test]
fn on_move_event_target_trait_predicate_matches_moved_permanent() {
    let yaml = r#"
card: DSL-MOVE-OBS
name: Move Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_move
    condition: { event_card_trait_has: Rock }
    process:
      - gain_memory: 2
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("BABY-ROCK", "Rock Baby", &["Rock"], 1000))
        .build();
    let observer = runner.place_on_field(0, "DSL-MOVE-OBS", None);
    let moved = runner.place_on_field(0, "BABY-ROCK", None);
    let event_card = runner
        .game
        .players[moved.player as usize]
        .battle_area[moved.index as usize]
        .top_card()
        .handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnMove,
        TriggerSource::MovedFromBreeding {
            player: 0,
            permanent: moved,
            card: event_card,
        },
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 2);
    assert_eq!(observer.player, 0);
}
```

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_move_event_target_trait_predicate_matches_moved_permanent
```

Expected before implementation: FAIL because `when: on_move` does not lower to a firing engine timing and the trigger payload cannot expose the moved card.

- [ ] **Step 3: Implement the narrow timing and dispatch**

Make these code changes:

```text
1. Add `EffectTiming::OnMove`.
2. Add `Effect::on_move(card)` that sets `timing = EffectTiming::OnMove`.
3. Add `TriggerSource::MovedFromBreeding { player, permanent, card }`.
4. Populate `TriggerContext.event_permanent`, `TriggerContext.event_card`, and `TriggerContext.source_player` from that trigger source.
5. In `Game::move_from_breeding`, after the permanent is successfully moved into the battle area and has a stable battle-area handle, call `enqueue_triggered(EffectTiming::OnMove, TriggerSource::MovedFromBreeding { ... })` and drain using the local convention for other immediate observers.
6. Map DSL `when: on_move` to `EffectTiming::OnMove`.
```

Do not fire `OnMove` for hatching from digitama to breeding. `OnHatch` remains a separate timing.

- [ ] **Step 4: Verify the slice**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_move_fires_after_breeding_permanent_moves_to_battle
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_move_event_target_trait_predicate_matches_moved_permanent
```

Expected after implementation: PASS.

## Task 2: `GameEvent::Digivolve` Emission With Newly-Digivolved Permanent in Context

**Files:**
- Modify: `code/digimon-engine/src/events.rs`
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/trigger_context.rs`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Test: `code/digimon-engine/tests/timing_dispatch.rs`
- Test: `code/digimon-engine/tests/dsl/phase3d_event_context.rs`

- [ ] **Step 1: Write the failing event-log test**

Add this test to `code/digimon-engine/tests/timing_dispatch.rs`:

```rust
fn zero_cost_evo_card(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = plain_digimon(card_id, name, 0);
    card.level = Some(4);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 0,
    }];
    card
}

#[test]
fn game_event_digivolve_is_emitted_with_new_top_card_and_field_index() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 3))
        .add_card(zero_cost_evo_card("EVO", "Evolution", &[]))
        .hand(0, &["EVO"])
        .memory(10)
        .start();
    let base = r.place_on_field(0, "BASE", None);

    let checkpoint = r.event_checkpoint();
    assert!(r.game.digivolve_from_hand(
        0,
        0,
        base.index as usize,
        PlaySource::ByDigivolve
    ));

    let events = r.events_since(checkpoint);
    assert!(
        events.iter().any(|event| matches!(
            event,
            GameEvent::Digivolve {
                player: 0,
                top_card_id,
                field_index,
                from_stack_top,
                ..
            } if top_card_id == "EVO"
                && *field_index == base.index
                && from_stack_top == "BASE"
        )),
        "digivolve should emit a GameEvent::Digivolve containing new top card and previous stack top"
    );
}
```

Also add `use digimon_engine::events::GameEvent;` if `timing_dispatch.rs` does not already import it. The existing file already imports `EvoCost`, `CardColor`, and `PlaySource`; keep using those types rather than hard-coded color integers.

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- game_event_digivolve_is_emitted_with_new_top_card_and_field_index
```

Expected before implementation: FAIL because `GameEvent::Digivolve` is defined but not emitted.

- [ ] **Step 2: Write the failing DSL trait-filter test for `OnDigivolve`**

Add this test to `code/digimon-engine/tests/dsl/phase3d_event_context.rs`:

```rust
#[test]
fn on_digivolve_event_card_trait_predicate_matches_new_top_card() {
    let yaml = r#"
card: DSL-DIGI-OBS
name: Digivolve Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolve
    condition: { event_card_trait_has: Mineral }
    process:
      - gain_memory: 3
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("BASE", "Base", &[], 1000))
        .add_card({
            let mut card = digimon_card("EVO-MINERAL", "Mineral Evo", &["Mineral"], 3000);
            card.level = Some(4);
            card.evo_costs = vec![EvoCost {
                card_color: CardColor::Red as u8,
                level: 3,
                memory_cost: 0,
            }];
            card
        })
        .hand(0, &["EVO-MINERAL"])
        .build();
    runner.place_on_field(0, "DSL-DIGI-OBS", None);
    let target = runner.place_on_field(0, "BASE", None);

    assert!(runner.game.digivolve_from_hand(
        0,
        0,
        target.index as usize,
        PlaySource::ByDigivolve
    ));

    assert_eq!(runner.memory(), 3);
}
```

Extend the imports at the top of `phase3d_event_context.rs` to include `EvoCost` and `PlaySource`:

```rust
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlaySource};
```

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card
```

Expected before implementation: FAIL because `OnDigivolve` observers do not receive the newly-digivolved permanent/card as event context.

- [ ] **Step 3: Implement digivolve event emission and payload threading**

Make these code changes:

```text
1. In `Game::digivolve_from_hand`, before mutating the stack, record the previous top card id.
2. After the new card is on top and the permanent handle remains stable, emit `GameEvent::Digivolve { player, top_card_id, field_index, from_stack_top }`.
3. Replace broad `TriggerSource::PlayerBattleArea(player)` for `EffectTiming::OnDigivolve` with a payload that carries `permanent` and new `card`.
4. Populate `TriggerContext.event_permanent`, `TriggerContext.event_card`, and `TriggerContext.source_player`.
5. Update `event_card_trait_has`, `event_target_kind`, and `event_target` predicate/binding code to read the payload instead of trying to infer the target from the observer permanent.
```

This slice is proven only for `Game::digivolve_from_hand`. Do not mark `effect_initiated_digivolve`, `dna_digivolve_inner`, or breeding-area digivolve as complete in trackers unless the implementation worker adds separate red/green tests for those paths in this same task. Do not fire `WhenDigivolving` for breeding-area digivolve unless a separate existing test proves that behavior has already changed; current comments state breeding digivolve does not fire that timing.

- [ ] **Step 4: Verify the slice**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- game_event_digivolve_is_emitted_with_new_top_card_and_field_index
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card
```

Expected after implementation: PASS.

## Task 3: `OnEnterFieldAnyone` Context With Entering Permanent/Card

**Files:**
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/trigger_context.rs`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Test: `code/digimon-engine/tests/dsl/phase3d_event_context.rs`

- [ ] **Step 1: Write the failing DSL trait-filter test**

Add this test to `code/digimon-engine/tests/dsl/phase3d_event_context.rs`:

```rust
#[test]
fn on_enter_field_anyone_event_card_trait_predicate_matches_entering_card() {
    let yaml = r#"
card: DSL-ENTER-OBS
name: Enter Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_enter_field_anyone
    condition: { event_card_trait_has: Royal Knight }
    process:
      - gain_memory: 4
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card({
            let mut card = digimon_card("RK-ENTER", "Royal Knight", &["Royal Knight"], 3000);
            card.play_cost = 0;
            card
        })
        .hand(1, &["RK-ENTER"])
        .memory(10)
        .build();
    runner.place_on_field(0, "DSL-ENTER-OBS", None);

    let before = runner.memory();
    assert_eq!(runner.play(1, 0), Some(0));

    assert_eq!(
        runner.memory(),
        before + 4,
        "observer should see the entering card traits through event context"
    );
}
```

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_enter_field_anyone_event_card_trait_predicate_matches_entering_card
```

Expected before implementation: FAIL because the observer fires without entering-card context or the predicate evaluates against the observer's own card.

- [ ] **Step 2: Implement enter-field payload threading**

Make these code changes:

```text
1. Add `TriggerSource::EnteredField { player, permanent, card }` or extend the existing enter-field trigger source with those fields.
2. In the normal `Game::play_from_hand` / `DebugRunner::play` battle-area Digimon path that already fires `OnEnterFieldAnyone`, pass the entering permanent handle and top card handle in the trigger source.
3. Populate `TriggerContext.event_permanent`, `TriggerContext.event_card`, and `TriggerContext.source_player`.
4. Update DSL event predicates and bindings to use those fields.
5. Preserve current ordering: the card's own `OnPlay` resolves before global `OnEnterFieldAnyone`, matching existing comments in `game_actions.rs`.
```

This slice is proven only for the normal hand-play path. Do not claim coverage for effect-created permanents, token play, option placement, or breeding-area fan-out unless separate red/green tests are added. If a Royal Knights breeding observer still cannot see this event, keep that as `G-BREEDING-TRIGGER-DISPATCH`.

- [ ] **Step 3: Verify the slice**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_enter_field_anyone_event_card_trait_predicate_matches_entering_card
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_enter_field_anyone
```

Expected after implementation: PASS for the new DSL test and existing enter-field dispatch tests.

## Task 4: `OnOptionPlaced` Timing and Dispatch After Option Placement

**Files:**
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/effect.rs`
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/trigger_context.rs`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify if placement is phase-owned: `code/digimon-engine/src/game_phases.rs`
- Modify: `code/digimon-engine/src/dsl_cards/timing_map.rs`
- Test: `code/digimon-engine/tests/timing_dispatch.rs`
- Test: `code/digimon-engine/tests/dsl/phase3d_event_context.rs`

- [ ] **Step 1: Write the failing timing constructor/dispatch test**

Add this test to `code/digimon-engine/tests/timing_dispatch.rs`:

```rust
fn option_card(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: 0,
        colors: vec![CardColor::Red],
        traits: traits.iter().map(|t| t.to_string()).collect(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

struct DelayOptionNoop;
impl CardEffect for DelayOptionNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Delay noop")
            .delay(DelayTrigger::EndOfYourNextTurn)
            .process(|_ctx| {})
            .build()]
    }
}

struct OnOptionPlacedObserver;
impl CardEffect for OnOptionPlacedObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_option_placed(card)
            .name("Option placed observer")
            .condition(|ctx| ctx.event_card().is_some())
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

#[test]
fn on_option_placed_fires_after_delay_option_enters_battle_area() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS", "Option Observer", 3))
        .add_card(option_card("OPT-DELAY", "Delay Option", &["Royal Knight"]))
        .hand(0, &["OBS", "OPT-DELAY"])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(OnOptionPlacedObserver));
    r.register_effect("OPT-DELAY", Arc::new(DelayOptionNoop));

    assert_eq!(r.play(0, 0), Some(0));
    let before = r.memory();
    let battle_before = r.battle_area_size(0);
    let _ = r.game.play_option_from_hand(0, 0);

    assert_eq!(
        r.battle_area_size(0),
        battle_before + 1,
        "Delay option should be placed as a battle-area option permanent"
    );

    assert_eq!(r.memory(), before + 1);
}
```

Extend the imports in `timing_dispatch.rs` with `DelayTrigger` if it is not already imported. The test drives the current API, `r.game.play_option_from_hand(0, 0)`, and proves the card parked on the battle area before asserting the observer fired.

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed_fires_after_delay_option_enters_battle_area
```

Expected before implementation: FAIL because `EffectTiming::OnOptionPlaced`, builder support, timing lowerer, or dispatch after placement is missing.

- [ ] **Step 2: Write the failing DSL trait-filter test**

Add this test to `code/digimon-engine/tests/dsl/phase3d_event_context.rs`:

```rust
#[test]
fn on_option_placed_event_card_trait_predicate_matches_placed_option() {
    let yaml = r#"
card: DSL-OPT-OBS
name: Option Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_option_placed
    condition: { event_card_trait_has: Royal Knight }
    process:
      - gain_memory: 2
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(option_card("RK-OPTION", "Royal Knight Option", &["Royal Knight"]))
        .hand(0, &["RK-OPTION"])
        .build();
    runner.register_effect("RK-OPTION", Arc::new(DelayOptionNoop));
    runner.place_on_field(0, "DSL-OPT-OBS", None);

    let before = runner.memory();
    let battle_before = runner.battle_area_size(0);
    let _ = runner.game.play_option_from_hand(0, 0);

    assert_eq!(
        runner.battle_area_size(0),
        battle_before + 1,
        "Delay option should be placed before OnOptionPlaced observers resolve"
    );

    assert_eq!(runner.memory(), before + 2);
}
```

Add the same `option_card` helper and `DelayOptionNoop` fixture shown in Step 1 to `phase3d_event_context.rs`, or place equivalent local helpers above this test. Also add these imports if absent:

```rust
use std::sync::Arc;
use digimon_engine::card_source::CardHandle;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::DelayTrigger;
```

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_option_placed_event_card_trait_predicate_matches_placed_option
```

Expected before implementation: FAIL because `when: on_option_placed` does not map to a dispatchable engine timing with option-card payload.

- [ ] **Step 3: Implement option placement timing**

Make these code changes:

```text
1. Add `EffectTiming::OnOptionPlaced` and `Effect::on_option_placed(card)`.
2. Map `CompiledTiming::OnOptionPlaced` to `EffectTiming::OnOptionPlaced`.
3. Add `TriggerSource::OptionPlaced { player, permanent, card }`.
4. After option placement commits and the placed option card/permanent handle is stable, fire `enqueue_triggered(EffectTiming::OnOptionPlaced, TriggerSource::OptionPlaced { ... })`.
5. Populate `TriggerContext.event_card`, `TriggerContext.event_permanent`, and `TriggerContext.source_player`.
6. Dispatch after placement, not before, so predicates can inspect option state and the option can be referenced by handle when a permanent exists.
```

This slice is proven only for Delay-style option placement through `Game::play_option_from_hand`. Do not route transient options that immediately resolve to trash through this timing unless a separate red/green test proves the engine explicitly represents them as placed battle-area option permanents.

- [ ] **Step 4: Verify the slice**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed_fires_after_delay_option_enters_battle_area
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_option_placed_event_card_trait_predicate_matches_placed_option
```

Expected after implementation: PASS.

## Task 5: `OnDigivolutionCardTrashed` With Host Permanent and Trashed Source Card in Context

**Files:**
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/trigger_context.rs`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Test: `code/digimon-engine/tests/timing_dispatch.rs`
- Test: `code/digimon-engine/tests/dsl/phase3d_event_context.rs`
- Test when cleaner: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_032.rs` or `code/digimon-engine/tests/cards_behavioral/p/p_167.rs`

- [ ] **Step 1: Write the failing host/source context test**

Add this test to `code/digimon-engine/tests/timing_dispatch.rs`:

```rust
use digimon_engine::card_source::CardSource;
use digimon_engine::permanent::{Permanent, PermanentHandle};

struct SourceTrashObserver;
impl CardEffect for SourceTrashObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_digivolution_card_trashed(card)
            .name("Source trash observer")
            .condition(|ctx| ctx.event_host_permanent().is_some() && ctx.event_source_card().is_some())
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

#[test]
fn on_digivolution_card_trashed_context_carries_host_and_trashed_source() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS", "Source Trash Observer", 3))
        .add_card(plain_digimon("TOP", "Top", 4))
        .add_card(plain_digimon("UNDER", "Under", 3))
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(SourceTrashObserver));

    r.place_on_field(0, "OBS", None);
    let host = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let under_idx = g.card_data.iter().position(|c| c.card_id == "UNDER").unwrap();
        let top_idx = g.card_data.iter().position(|c| c.card_id == "TOP").unwrap();
        let under = CardSource::new(under_idx, 0, g.next_card_index());
        let top = CardSource::new(top_idx, 0, g.next_card_index());
        let mut permanent = Permanent::new(under, turn);
        permanent.card_sources.push(top);
        g.players[0].battle_area.push(permanent);
        PermanentHandle {
            player: 0,
            index: (g.players[0].battle_area.len() - 1) as u8,
        }
    };

    let before = r.memory();
    assert!(
        r.game_mut().return_to_hand(host).is_some(),
        "return_to_hand should move TOP to hand and trash UNDER"
    );

    assert_eq!(r.memory(), before + 1);
}
```

This uses the existing `Game::return_to_hand` source-disposition path because it already trashes below-top sources and claims to fire `OnDigivolutionCardTrashed`. Do not use non-existent DebugRunner source-stack helpers.

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed_context_carries_host_and_trashed_source
```

Expected before implementation: FAIL because source-trash trigger context does not expose both host and trashed source card, or the source-trash helper does not fire the timing.

- [ ] **Step 2: Write the failing DSL trait-filter test**

Add this test to `code/digimon-engine/tests/dsl/phase3d_event_context.rs`:

```rust
#[test]
fn on_digivolution_card_trashed_event_card_trait_predicate_matches_trashed_source() {
    let yaml = r#"
card: DSL-SOURCE-TRASH-OBS
name: Source Trash Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolution_card_trashed
    condition: { event_card_trait_has: Mineral }
    process:
      - gain_memory: 3
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("TOP", "Top", &[], 4000))
        .add_card(digimon_card("UNDER-MINERAL", "Under Mineral", &["Mineral"], 1000))
        .build();
    runner.place_on_field(0, "DSL-SOURCE-TRASH-OBS", None);
    let host = {
        let g = runner.game_mut();
        let turn = g.turn_count;
        let under_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER-MINERAL")
            .unwrap();
        let top_idx = g.card_data.iter().position(|c| c.card_id == "TOP").unwrap();
        let under = CardSource::new(under_idx, 0, g.next_card_index());
        let top = CardSource::new(top_idx, 0, g.next_card_index());
        let mut permanent = Permanent::new(under, turn);
        permanent.card_sources.push(top);
        g.players[0].battle_area.push(permanent);
        PermanentHandle {
            player: 0,
            index: (g.players[0].battle_area.len() - 1) as u8,
        }
    };

    assert!(runner.game_mut().return_to_hand(host).is_some());

    assert_eq!(runner.memory(), 3);
}
```

Add these imports to `phase3d_event_context.rs` for the manual source-stack fixture:

```rust
use digimon_engine::card_source::CardSource;
use digimon_engine::permanent::{Permanent, PermanentHandle};
```

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolution_card_trashed_event_card_trait_predicate_matches_trashed_source
```

Expected before implementation: FAIL because `event_card_trait_has` cannot see the trashed source card.

- [ ] **Step 3: Implement source-trash payload threading**

Make these code changes:

```text
1. Add or extend `TriggerSource::SourceTrashedFromStack { host, trashed_card, cause }`.
2. Add a minimal `TrashCause` enum only if the current engine has no source-trash cause discriminator; start with `Effect` for this slice.
3. Populate `TriggerContext.event_host_permanent`, `TriggerContext.event_source_card`, `TriggerContext.event_card`, and `TriggerContext.source_player`.
4. Route the `Game::return_to_hand` below-top source-disposition path through this trigger source.
5. Ensure inherited effects on the trashed source can still inspect their former host before the host stack is mutated beyond recognition.
6. Update DSL event-card predicates so `event_card` resolves to the trashed source card for this timing.
```

This slice is proven only for below-top sources trashed by `Game::return_to_hand`. Do not mark `return_to_deck`, `de_digivolve`, `trash_card_source`, `trash_top_source`, Armor Purge, Fragment, Digi-Burst, or cross-permanent source selection complete unless the implementation worker adds separate red/green tests for those paths. Do not add cross-permanent multi-source selection here.

- [ ] **Step 4: Verify the slice**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed_context_carries_host_and_trashed_source
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolution_card_trashed_event_card_trait_predicate_matches_trashed_source
```

Expected after implementation: PASS.

## Task 6: `OnAllyAttack` / `OnOpponentAttack` Declared-Attack Observer Dispatch

**Files:**
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/trigger_context.rs`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Modify: `code/digimon-engine/src/combat.rs`
- Test: `code/digimon-engine/tests/timing_dispatch.rs`
- Test when cleaner: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_003.rs`

- [ ] **Step 1: Write the failing declared-attack dispatch test**

Add this test to `code/digimon-engine/tests/timing_dispatch.rs`:

```rust
struct AllyAttackObserver;
impl CardEffect for AllyAttackObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_ally_attack(card)
            .name("Ally attack observer")
            .condition(|ctx| ctx.attack_attacker().is_some() && ctx.attack_target().is_some())
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

struct OpponentAttackObserver;
impl CardEffect for OpponentAttackObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_opponent_attack(card)
            .name("Opponent attack observer")
            .condition(|ctx| ctx.attack_attacker().is_some() && ctx.attack_target().is_some())
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

#[test]
fn declared_attack_fires_ally_and_opponent_observers_with_attack_context() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", "Attacker", 3))
        .add_card(plain_digimon("ALLY-OBS", "Ally Observer", 3))
        .add_card(plain_digimon("OPP-OBS", "Opponent Observer", 3))
        .memory(10)
        .start();
    r.register_effect("ALLY-OBS", Arc::new(AllyAttackObserver));
    r.register_effect("OPP-OBS", Arc::new(OpponentAttackObserver));

    let attacker = r.place_on_field(0, "ATTACKER", None);
    r.place_on_field(0, "ALLY-OBS", None);
    r.place_on_field(1, "OPP-OBS", None);

    let before = r.memory();
    r.attack_player(attacker, 1, false);
    r.auto_resolve();

    assert_eq!(
        r.memory(),
        before + 3,
        "ally observer should gain 1 and opponent observer should gain 2 at attack declaration"
    );
}
```

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- declared_attack_fires_ally_and_opponent_observers_with_attack_context
```

Expected before implementation: FAIL if attack observer dispatch or attack payload accessors are missing. If dispatch already exists, the first failure should be missing context accessors or target payload.

- [ ] **Step 2: Write the failing exclusion/order regression**

Add this test to the same file:

```rust
#[test]
fn on_ally_attack_does_not_fire_on_the_attacker_itself() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER-OBS", "Attacker Observer", 3))
        .memory(10)
        .start();
    r.register_effect("ATTACKER-OBS", Arc::new(AllyAttackObserver));

    let attacker = r.place_on_field(0, "ATTACKER-OBS", None);
    let before = r.memory();

    r.attack_player(attacker, 1, false);
    r.auto_resolve();

    assert_eq!(
        r.memory(),
        before,
        "OnAllyAttack observers exclude the attacking permanent itself; use OnAttack for self"
    );
}
```

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_ally_attack_does_not_fire_on_the_attacker_itself
```

Expected before implementation: FAIL if the fan-out includes the attacker or if no dispatch exists.

- [ ] **Step 3: Implement declared-attack payload threading**

Make these code changes:

```text
1. Add or extend `TriggerSource::DeclaredAttack { attacker, target }`.
2. Populate `TriggerContext.attack_attacker`, `TriggerContext.attack_target`, and `TriggerContext.source_player`.
3. Fire `OnAllyAttack` after the attack is declared and before Alliance/Counter/Block windows, scanning the attacker's controller battle area and excluding the attacking permanent itself.
4. Fire `OnOpponentAttack` in the same declared-attack window for the defending player's battle area.
5. Preserve the existing `OnAttack` self timing and `WhenAttacking` observer timing order unless current tests specify a different order.
6. Ensure direct security attacks and Digimon-target attacks both set an attack target variant, not an absent target.
```

This slice only dispatches observers. Attack cancellation and triggered-body source-trash costs belong to the cost/replacement child plan.

- [ ] **Step 4: Verify the slice**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- declared_attack_fires_ally_and_opponent_observers_with_attack_context
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_ally_attack_does_not_fire_on_the_attacker_itself
```

Expected after implementation: PASS.

## Task 7: Tracker Updates After Event Tests Pass

**Files:**
- Docs: `docs/RUST_ENGINE_GAPS.md`
- Docs: `qa/archetype-qa/engine-gaps.md`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Update `docs/RUST_ENGINE_GAPS.md`**

After the targeted tests for a slice pass, update only the matching gap text:

```text
G-ON-MOVE / [When Moving]:
  Cite the `on_move_fires_after_breeding_permanent_moves_to_battle` test and state that breeding-to-battle movement now carries moved permanent/card context.

G-GAME-EVENT-DIGIVOLVE:
  Cite the `game_event_digivolve_is_emitted_with_new_top_card_and_field_index` test and state that the normal `Game::digivolve_from_hand` path emits the event. Keep effect-initiated digivolve and DNA digivolve listed as follow-up paths unless separately tested.

G-ON-DIGIVOLVE-TRAIT-FILTER:
  Cite the DSL trait-filter test and state that `event_card` is the new top card for OnDigivolve.

G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER:
  Cite the DSL trait-filter test and state that `event_card` is the entering card for normal hand-played Digimon. Keep effect-created permanents, token play, and breeding fan-out listed as follow-up paths unless separately tested.

G-OPTION-PLACED-TIMING:
  Cite the Delay option placement timing and DSL tests. Keep transient option resolution and other option-state paths open unless separately tested.

OnDigivolutionCardTrashed observer timing:
  Cite the `Game::return_to_hand` host/source context tests and keep `return_to_deck`, `de_digivolve`, direct source-trash helpers, keyword costs, and cross-permanent source selection open unless separately tested.

OnAllyAttack / OnOpponentAttack:
  Cite the declared-attack dispatch and attacker-exclusion tests.
```

Do not mark a gap resolved if the event only works through direct `enqueue_triggered` and not through the real state-machine dispatch site.

- [ ] **Step 2: Update `qa/archetype-qa/engine-gaps.md`**

Update these entries with the same evidence:

```text
G-ON-MOVE
G-ON-DIGIVOLVE-TRAIT-FILTER
G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER
G-GAME-EVENT-DIGIVOLVE
G-OPTION-PLACED-TIMING
OnDigivolutionCardTrashed observer timing
OnAllyAttack / OnOpponentAttack observer timings
```

For Rocks and Royal Knights notes, narrow only the event-context blocker. Keep selection, replacement, breeding-source fan-out, option state, and triggered-body cost blockers open unless separate tests in this implementation prove those behaviors.

- [ ] **Step 3: Update `qa/dsl-vocab-gaps.md`**

Update these DSL entries only after both schema/lowering and runtime dispatch tests pass:

```text
Rocks / Ukkomon `[When Moving]` timing token
Royal Knights `on_option_placed` timing lowerer
Rocks source-trash observer companion note
Event-card trait/name/owner predicate notes for OnDigivolve and OnEnterFieldAnyone
Attack observer predicate notes if this file has an existing OnAllyAttack or OnOpponentAttack entry
```

For each entry, include the exact test command that proves the DSL lowerer and runtime payload both work.

- [ ] **Step 4: Run tracker diff check**

Run:

```bash
git diff --check -- docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md
```

Expected: no output.

## Task 8: Final Verification and Implementation Commit

**Files:**
- All source, test, and tracker files touched by the implementation worker.

- [ ] **Step 1: Run the event-context targeted test suite**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_move
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- game_event_digivolve
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- declared_attack
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context
```

Expected after implementation: all commands PASS.

- [ ] **Step 2: Run action/tensor contract review commands**

Run:

```bash
git diff -- docs/ACTION_SPEC.md docs/TENSOR_SPEC.md
Select-String -Path 'code/digimon-engine/src/action/space.rs','code/digimon-engine/src/tensor.rs' -Pattern 'ACTION_SPACE_SIZE','TENSOR_SIZE'
```

Expected: no diff to `docs/ACTION_SPEC.md` or `docs/TENSOR_SPEC.md`; constants remain `ACTION_SPACE_SIZE = 2168` and `TENSOR_SIZE = 1375`.

- [ ] **Step 3: Run whitespace diff check for the implementation**

Run with the actual files changed:

```bash
git diff --check -- code/digimon-engine/src/enums.rs code/digimon-engine/src/effect.rs code/digimon-engine/src/selection.rs code/digimon-engine/src/trigger_context.rs code/digimon-engine/src/effect_context.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/src/dsl_cards/timing_map.rs code/digimon-engine/src/events.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/game_phases.rs code/digimon-engine/src/combat.rs code/digimon-engine/tests/timing_dispatch.rs code/digimon-engine/tests/dsl/phase3d_event_context.rs docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md
```

Expected: no output.

- [ ] **Step 4: Commit the implementation slice**

Run:

```bash
git status --short
git add code/digimon-engine/src/enums.rs code/digimon-engine/src/effect.rs code/digimon-engine/src/selection.rs code/digimon-engine/src/trigger_context.rs code/digimon-engine/src/effect_context.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/src/dsl_cards/timing_map.rs code/digimon-engine/src/events.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/game_phases.rs code/digimon-engine/src/combat.rs code/digimon-engine/tests/timing_dispatch.rs code/digimon-engine/tests/dsl/phase3d_event_context.rs docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md
git commit -m "feat: wire event context followups"
```

Expected: commit succeeds with only files from this implementation plan staged. If one of the listed optional files was not changed, omit it from `git add`.
