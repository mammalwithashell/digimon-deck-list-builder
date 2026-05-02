# Option Delay Link State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining Group 5 Rust engine and DSL gaps for Option field state, start/event/replacement Delay, Plug-In Link, Training scope, option-placement observers, and scheduled option effects.

**Architecture:** Build on the existing Phase 8 substrate instead of replacing it: `Permanent::option_state` already models `Standard`, `Delayed`, `Linked`, and `Training`; `Game::dispose_option` already routes Standard/Delay/Link/Training; `effect_queue` already scans linked and Training sources. This plan adds the missing timing/state distinctions and DSL vocabulary while preserving every player-visible choice through pending selections and action masks.

**Tech Stack:** Rust engine (`code/digimon-engine`), `digimon-dsl`, cargo integration tests, Rust PyO3/RL contract docs.

---

## Scope Note

Group 5 depends on Group 1 event context and dispatch, Group 2 pending-selection/action-mask primitives, and Group 4 zone movement helpers. Do not implement the Delay replacement-window slice in parallel with `docs/superpowers/plans/2026-04-30-gap-group-3-task-5-delay-replacement.md`; that child plan owns the replacement framework work for `BT17-097` and should be executed or merged first. If this plan changes `ACTION_SPACE_SIZE`, tensor shape, selection encodings, or exported runner constants, update `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md`, PyO3 constants, frontend constants, and RL wrappers in the same task.

Current substrate already present:

- `code/digimon-engine/src/permanent.rs`: `OptionState::{Standard, Delayed, Linked, Training}`.
- `code/digimon-engine/src/game_actions.rs`: `OptionSubtype`, `play_option_from_hand`, `dispose_option`, `attach_linked_card`, and `compute_delay_trash_turn`.
- `code/digimon-engine/src/game_phases.rs`: end-of-turn `resolve_delayed_options`.
- `code/digimon-engine/src/effect_queue.rs`: linked-card and Training scans.
- `code/digimon-engine/tests/option_flow/`: existing `delay_flow.rs`, `link_flow.rs`, `training_flow.rs`, `behavioral_end_to_end.rs`.

## First Fixtures

- `LM-027` Red Scramble: start-of-your-turn Delay and security add-self-to-hand.
- `BT22-098` Unique Emblem: Fable Waltz: event-gated Delay when Arisa Kinosaki suspends.
- `BT17-097` Return to the Primogenitor: Delay-as-replacement prevention, owned by the Group 3 replacement child plan.
- `ST22-08` Offensive Plug-In V: Plug-In/Link declaration, free link step, linked scope, and color-ignore mask.
- `BT13-110` Royal Knights of the Purge: option placement plus Royal Knight trait context for King Drasil observers.

## File Structure

- Modify: `code/digimon-engine/src/enums.rs` for `DelayTrigger::StartOfYourNextTurn`; first inspect the enum and skip the enum edit if another branch already introduced that exact variant.
- Modify: `code/digimon-engine/src/game_actions.rs` for delay scheduling, option-placement dispatch extensions, and Link attach helpers.
- Modify: `code/digimon-engine/src/game_phases.rs` for start-of-turn delay drains.
- Modify: `code/digimon-engine/src/effect_context/mod.rs` for inherited-security option placement and optional helper APIs.
- Modify: `code/digimon-engine/src/scheduled_effects.rs` for transient scheduled option effect integration if the existing queue needs option-source replay fixes.
- Modify: `code/digimon-engine/src/dsl_cards/lower_delay.rs` and `code/digimon-engine/src/dsl_cards/timing_map.rs` for Delay trigger lowering.
- Modify: `code/digimon-engine/src/dsl_cards/lower_triggered.rs`, `code/digimon-engine/src/dsl_cards/mod.rs`, and `code/digimon-engine/src/dsl_cards/step/*.rs` for Link/Delay/placement DSL lowering.
- Modify: `code/digimon-engine/../digimon-dsl/src/compiled.rs`, `code/digimon-engine/../digimon-dsl/src/clause.rs`, `code/digimon-engine/../digimon-dsl/src/compiler/clause.rs`, and `code/digimon-engine/../digimon-dsl/src/step.rs` for new DSL vocabulary.
- Test: `code/digimon-engine/tests/option_flow/start_delay_flow.rs`.
- Test: `code/digimon-engine/tests/option_flow/event_gated_delay.rs`.
- Test: `code/digimon-engine/tests/option_flow/inherited_security_option.rs`.
- Test: `code/digimon-engine/tests/option_flow/option_placed_observers.rs`.
- Test: `code/digimon-engine/tests/option_flow/link_flow.rs`.
- Test: `code/digimon-engine/tests/option_flow/training_flow.rs`.
- Test: `code/digimon-engine/tests/dsl/delay.rs`.
- Test: `code/digimon-engine/tests/dsl/link.rs`.
- Trackers: `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`.

## Task 1: Start-of-Turn Delay for Scrambles

**Files:**
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/game_phases.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_delay.rs`
- Test: `code/digimon-engine/tests/option_flow/start_delay_flow.rs`
- Test: `code/digimon-engine/tests/dsl/delay.rs`

- [ ] **Step 1: Write the failing engine test**

Create `code/digimon-engine/tests/option_flow/start_delay_flow.rs`:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;

struct StartDelayWitness(Arc<Mutex<u32>>);

impl CardEffect for StartDelayWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::on_play(card)
            .name("Start of your next turn Delay")
            .delay(DelayTrigger::StartOfYourNextTurn)
            .process(move |_ctx| {
                *seen.lock().unwrap() += 1;
            })
            .build()]
    }
}

fn option_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = 0;
    cd.colors = vec![CardColor::Red];
    cd
}

fn digimon_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![CardColor::Red];
    cd
}

#[test]
fn start_of_your_next_turn_delay_fires_at_turn_start_not_end() {
    let witness = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(option_card("LM-027"))
        .add_card(digimon_card("RED-MATCH"))
        .add_card(digimon_card("FILLER"))
        .hand(0, &["LM-027"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(0)
        .start();
    r.register_effect("LM-027", Arc::new(StartDelayWitness(witness.clone())));
    r.place_on_field(0, "RED-MATCH", Some(0));
    r.game.enter_main_phase();

    let start_turn = r.game.turn_count;
    assert_eq!(r.game.play_option_from_hand(0, 0), digimon_engine::selection::OptionPlayResult::Trashed);
    assert_eq!(*witness.lock().unwrap(), 0);

    r.end_turn();
    assert_eq!(*witness.lock().unwrap(), 0, "does not fire at end of placement turn");
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(*witness.lock().unwrap(), 0, "does not fire during opponent turn");

    assert_eq!(r.game.turn_count, start_turn + 2);
    assert_eq!(r.game.turn_player(), 0);
    assert_eq!(*witness.lock().unwrap(), 1, "fires from begin_turn before draw/main");
    assert!(!r.game.player(0).battle_area.iter().any(|p| matches!(p.option_state, OptionState::Delayed { .. })));
    assert_eq!(r.trash_size(0), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- start_of_your_next_turn_delay_fires_at_turn_start_not_end
```

Expected: FAIL because `DelayTrigger::StartOfYourNextTurn` is missing or because the Delay fires only through the end-of-turn drain.

- [ ] **Step 3: Add the minimal engine implementation**

In `code/digimon-engine/src/enums.rs`, extend `DelayTrigger`:

```rust
pub enum DelayTrigger {
    EndOfThisTurn,
    EndOfYourNextTurn,
    StartOfYourNextTurn,
}
```

In `code/digimon-engine/src/game_actions.rs`, update `compute_delay_trash_turn`:

```rust
DelayTrigger::StartOfYourNextTurn => {
    if self.turn_player() == owner {
        self.turn_count + self.rules.player_count as u16
    } else {
        self.turn_count + ((owner + self.rules.player_count as u8 - self.turn_player()) % self.rules.player_count as u8) as u16
    }
}
```

If the existing `EndOfYourNextTurn` arm already computes the next owner turn cleanly, call that same helper from both arms rather than duplicating arithmetic.

In `code/digimon-engine/src/game_phases.rs`, add a start-of-turn drain immediately after `StartOfYourTurn` observers and before `new_turn()`:

```rust
self.resolve_start_delayed_options(self.turn_count);
```

Add a helper beside `resolve_delayed_options` that selects only `OptionState::Delayed` permanents whose trigger is `StartOfYourNextTurn`. If `OptionState::Delayed` still stores only `trash_on_turn`, add `trigger: DelayTrigger` to the variant and update all construction/matches:

```rust
OptionState::Delayed {
    owner,
    trash_on_turn,
    trigger,
}
```

- [ ] **Step 4: Add DSL lowering coverage**

Append to `code/digimon-engine/tests/dsl/delay.rs`:

```rust
#[test]
fn delay_start_of_your_turn_maps_to_start_of_your_next_turn() {
    let dsl = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::StartOfYourTurn,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects[0].delay_trigger, Some(DelayTrigger::StartOfYourNextTurn));
}
```

Then update `code/digimon-engine/src/dsl_cards/lower_delay.rs`:

```rust
let delay_trigger = match trigger {
    CompiledTiming::EndOfYourTurn => DelayTrigger::EndOfThisTurn,
    CompiledTiming::StartOfYourTurn => DelayTrigger::StartOfYourNextTurn,
    _ => DelayTrigger::EndOfYourNextTurn,
};
```

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- start_of_your_next_turn_delay_fires_at_turn_start_not_end
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay_start_of_your_turn_maps_to_start_of_your_next_turn
```

Expected: PASS.

- [ ] **Step 6: Update trackers and commit**

Edit `qa/archetype-qa/engine-gaps.md` entry `G-DELAY-START-OF-TURN` to say it is resolved, name the two passing test commands, and keep any card-specific blockers that are not start-delay timing. If `docs/RUST_ENGINE_GAPS.md` has the same row open, update it with the same evidence.

```bash
git add code/digimon-engine/src/enums.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/game_phases.rs code/digimon-engine/src/dsl_cards/lower_delay.rs code/digimon-engine/tests/option_flow/start_delay_flow.rs code/digimon-engine/tests/dsl/delay.rs docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md
git commit -m "feat: support start-of-turn delay options"
```

## Task 2: Event-Gated Delay for Unique Emblem

**Files:**
- Modify: `code/digimon-engine/src/permanent.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_delay.rs`
- Modify: `code/digimon-engine/../digimon-dsl/src/clause.rs`
- Test: `code/digimon-engine/tests/option_flow/event_gated_delay.rs`
- Test: `code/digimon-engine/tests/dsl/delay.rs`

- [ ] **Step 1: Write the failing event-gated Delay test**

Create `code/digimon-engine/tests/option_flow/event_gated_delay.rs`:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};

struct FableWaltzDelay(Arc<Mutex<u32>>);

impl CardEffect for FableWaltzDelay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::on_play(card)
            .name("Arisa suspend gated Delay")
            .delay(DelayTrigger::OnEvent(EffectTiming::OnSuspend))
            .condition(|ctx| ctx.event_card_name_contains("Arisa Kinosaki"))
            .process(move |_ctx| {
                *seen.lock().unwrap() += 1;
            })
            .build()]
    }
}

fn option_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = 0;
    cd.colors = vec![CardColor::Yellow];
    cd
}

fn tamer_card(card_id: &str, name: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, name);
    cd.card_kind = CardKind::Tamer;
    cd.level = None;
    cd.dp = None;
    cd.colors = vec![CardColor::Yellow];
    cd
}

#[test]
fn event_gated_delay_only_fires_after_placement_turn_and_matching_event() {
    let witness = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(option_card("BT22-098"))
        .add_card(tamer_card("ARISA", "Arisa Kinosaki"))
        .add_card(tamer_card("OTHER", "Other Tamer"))
        .hand(0, &["BT22-098"])
        .memory(0)
        .start();
    r.register_effect("BT22-098", Arc::new(FableWaltzDelay(witness.clone())));
    let arisa = r.place_on_field(0, "ARISA", Some(0));
    let other = r.place_on_field(0, "OTHER", Some(0));
    r.game.enter_main_phase();

    assert_eq!(r.game.play_option_from_hand(0, 0), digimon_engine::selection::OptionPlayResult::Trashed);
    r.game.suspend_permanent(other);
    assert_eq!(*witness.lock().unwrap(), 0, "wrong Tamer event does not fire");
    r.game.suspend_permanent(arisa);
    assert_eq!(*witness.lock().unwrap(), 0, "placement-turn event is gated");

    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.unsuspend_permanent(arisa);
    r.game.suspend_permanent(arisa);

    assert_eq!(*witness.lock().unwrap(), 1, "matching event after placement turn fires once");
    assert_eq!(r.trash_size(0), 1, "Delay trashes itself as activation cost");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- event_gated_delay_only_fires_after_placement_turn_and_matching_event
```

Expected: FAIL because there is no event-gated `DelayTrigger` variant and no event-dispatch path for delayed option permanents.

- [ ] **Step 3: Implement the event-gated Delay state**

Add an event trigger representation. If `EffectTiming` is too broad for Delay, create a narrower enum:

```rust
pub enum DelayTrigger {
    EndOfThisTurn,
    EndOfYourNextTurn,
    StartOfYourNextTurn,
    OnEvent(EffectTiming),
}
```

Store the placement turn on `OptionState::Delayed`:

```rust
Delayed {
    owner: PlayerId,
    trash_on_turn: u16,
    trigger: DelayTrigger,
    placed_on_turn: u16,
}
```

During `dispose_option`, populate `placed_on_turn: self.turn_count` and `trigger`. In the global event enqueue path, scan delayed option permanents whose `trigger == DelayTrigger::OnEvent(timing)`, whose `placed_on_turn < self.turn_count`, and whose condition passes against the event context. Fire the `DelayEffect`, then trash the option through the same replacement-aware path used by `resolve_delayed_options`.

- [ ] **Step 4: Add DSL active_when/event lowering**

Extend the Delay declarative body so this YAML can compile:

```yaml
- kind: delay
  trigger: on_suspend
  active_when:
    event_card_name_contains: "Arisa Kinosaki"
  process:
    - effect_initiated_digivolve:
        target: { trait: Puppet }
        into: { trait_all: [Puppet, LIBERATOR], zone: hand }
        cost_delta: -3
```

In `lower_delay.rs`, map event timings to `DelayTrigger::OnEvent(...)` and use `_active_when` instead of ignoring it. The process closure should evaluate `active_when` at event fire time, not at placement time.

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- event_gated_delay_only_fires_after_placement_turn_and_matching_event
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay_event_trigger_lowers_to_on_event_delay
```

Expected: PASS. Add the DSL test `delay_event_trigger_lowers_to_on_event_delay` in this task before running the command.

- [ ] **Step 6: Update trackers and commit**

Update `qa/archetype-qa/engine-gaps.md` and `qa/dsl-vocab-gaps.md` for the `BT22-098` event-gated Delay entry. Keep follow-up process blockers open if effect-initiated digivolution or trait filters are still not natively expressible.

```bash
git add code/digimon-engine/src/permanent.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/effect_queue.rs code/digimon-engine/src/dsl_cards/lower_delay.rs code/digimon-engine/tests/option_flow/event_gated_delay.rs code/digimon-engine/tests/dsl/delay.rs qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md
git commit -m "feat: support event-gated delay options"
```

## Task 3: Inherited Security Option Placement

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs`
- Modify: `code/digimon-engine/../digimon-dsl/src/step.rs`
- Test: `code/digimon-engine/tests/option_flow/inherited_security_option.rs`
- Test: `code/digimon-engine/tests/dsl/delay.rs`

- [ ] **Step 1: Write the failing placement test**

Create `code/digimon-engine/tests/option_flow/inherited_security_option.rs`:

```rust
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::OptionState;

fn option_source(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.colors = vec![CardColor::Red];
    cd
}

fn digimon_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![CardColor::Red];
    cd
}

#[test]
fn inherited_security_places_source_option_as_delay_permanent() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("HOST"))
        .add_card(option_source("P-035"))
        .memory(0)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.push_source(host, "P-035");

    r.run_inherited_security_effect(host, "P-035", |ctx| {
        ctx.place_self_as_delay_option_permanent();
    });

    assert_eq!(r.game.player(0).battle_area[host.index as usize].card_sources.len(), 1);
    let placed = r.game.player(0).battle_area.last().expect("placed option permanent");
    assert!(matches!(placed.option_state, OptionState::Delayed { owner: 0, .. }));
    assert_eq!(placed.top_card().card_id(&r.game.card_data), "P-035");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- inherited_security_places_source_option_as_delay_permanent
```

Expected: FAIL because `run_inherited_security_effect`, `push_source`, or `place_self_as_delay_option_permanent` is missing. Add missing test helpers under the existing `DebugRunner` pattern; the production API gap is `EffectContext::place_self_as_delay_option_permanent`.

- [ ] **Step 3: Implement `EffectContext` placement**

In `code/digimon-engine/src/effect_context/mod.rs`, add:

```rust
pub fn place_self_as_delay_option_permanent(&mut self) {
    let Some(source_perm) = self.source_permanent else {
        return;
    };
    let Some(source_card) = self.remove_source_card_from_permanent(source_perm, self.source_card) else {
        return;
    };
    let owner = source_card.owner;
    let mut permanent = crate::permanent::Permanent::new(source_card, self.game.turn_count);
    permanent.option_state = crate::permanent::OptionState::Delayed {
        owner,
        trash_on_turn: self.game.compute_delay_trash_turn(owner, crate::enums::DelayTrigger::EndOfYourNextTurn),
        trigger: crate::enums::DelayTrigger::EndOfYourNextTurn,
        placed_on_turn: self.game.turn_count,
    };
    self.game.player_mut(owner).battle_area.push(permanent);
    let handle = crate::permanent::PermanentHandle {
        player: owner,
        index: (self.game.player(owner).battle_area.len() - 1) as u8,
    };
    self.game.enqueue_triggered(
        crate::enums::EffectTiming::OnOptionPlaced,
        crate::selection::TriggerSource::OptionPlaced {
            player: owner,
            permanent: handle,
            card: self.source_card,
        },
    );
    self.game.drain_effect_queue();
}
```

If no helper exists to remove a specific source card from a permanent, add a private helper in the same module that finds `source_card` in `card_sources`, removes it, and leaves the top-card stack valid.

- [ ] **Step 4: Add the DSL step**

In `digimon-dsl/src/step.rs`, add:

```rust
PlaceSelfAsDelayOption {},
```

Parse this YAML shape:

```yaml
- place_self_as_delay_option: {}
```

Lower it in `code/digimon-engine/src/dsl_cards/step/mod.rs`:

```rust
CompiledStep::PlaceSelfAsDelayOption => {
    ctx.place_self_as_delay_option_permanent();
    RunOutcome::Complete
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- inherited_security_places_source_option_as_delay_permanent
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- place_self_as_delay_option
```

Expected: PASS for the new engine test and the exact DSL parser/lowering test introduced in this task.

- [ ] **Step 6: Update trackers and commit**

Update `qa/dsl-vocab-gaps.md` entry `G-PLACE-SELF-AS-OPTION-PERMANENT` and mirror any engine entry in `qa/archetype-qa/engine-gaps.md`.

```bash
git add code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/dsl_cards/step code/digimon-engine/tests/option_flow/inherited_security_option.rs code/digimon-engine/tests/dsl/delay.rs code/digimon-engine/../digimon-dsl/src/step.rs qa/dsl-vocab-gaps.md qa/archetype-qa/engine-gaps.md
git commit -m "feat: place inherited security options on field"
```

## Task 4: Option Placement Observers for Link, Training, Security, and Breeding

**Files:**
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Test: `code/digimon-engine/tests/option_flow/option_placed_observers.rs`

- [ ] **Step 1: Write the failing observer matrix test**

Create `code/digimon-engine/tests/option_flow/option_placed_observers.rs`:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

struct OnOptionPlacedWitness(Arc<Mutex<u32>>);

impl CardEffect for OnOptionPlacedWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::on_play(card)
            .name("Royal Knight option placed witness")
            .timing(EffectTiming::OnOptionPlaced)
            .condition(|ctx| ctx.event_card_trait_has("Royal Knight"))
            .process(move |ctx| {
                ctx.gain_memory(ctx.player, 1);
                *seen.lock().unwrap() += 1;
            })
            .build()]
    }
}

fn option_card(card_id: &str, royal_knight: bool) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = 0;
    cd.colors = vec![CardColor::White];
    if royal_knight {
        cd.traits = vec!["Royal Knight".to_string()];
    }
    cd
}

fn digimon_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![CardColor::White];
    cd
}

#[test]
fn on_option_placed_fires_for_training_link_and_security_placement_with_event_card() {
    let witness = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(option_card("BT13-110", true))
        .add_card(option_card("ST22-08", false))
        .add_card(digimon_card("KING-DRASIL"))
        .add_card(digimon_card("HOST"))
        .hand(0, &["BT13-110", "ST22-08"])
        .memory(0)
        .start();
    r.register_effect("KING-DRASIL", Arc::new(OnOptionPlacedWitness(witness.clone())));
    r.place_in_breeding(0, "KING-DRASIL", Some(0));
    r.place_on_field(0, "HOST", Some(0));
    r.game.enter_main_phase();

    assert_eq!(r.game.play_option_from_hand(0, 0), digimon_engine::selection::OptionPlayResult::Trashed);
    assert_eq!(*witness.lock().unwrap(), 1, "Royal Knight Delay placement fires");

    assert_eq!(r.game.play_option_from_hand(0, 0), digimon_engine::selection::OptionPlayResult::Pending);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, action);
    assert_eq!(*witness.lock().unwrap(), 1, "non-Royal-Knight Link placement does not match trait");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- on_option_placed_fires_for_training_link_and_security_placement_with_event_card
```

Expected: FAIL because breeding-area observer fan-out, Link/Training placement dispatch, or helper fixtures are missing.

- [ ] **Step 3: Dispatch `OnOptionPlaced` from every placement path**

In `dispose_option`, after creating a `Training` permanent, dispatch:

```rust
self.enqueue_triggered(
    EffectTiming::OnOptionPlaced,
    TriggerSource::OptionPlaced { player: owner, permanent: handle, card },
);
self.drain_effect_queue();
```

In `attach_linked_card`, dispatch after `host.linked_cards.push(card)` with a source that identifies the host and linked card. If `TriggerSource::OptionPlaced` cannot represent a linked card with no standalone permanent, extend it:

```rust
OptionPlaced {
    player: PlayerId,
    permanent: Option<PermanentHandle>,
    linked_host: Option<PermanentHandle>,
    card: CardHandle,
}
```

Update existing Delay dispatch call sites to use the new shape.

- [ ] **Step 4: Include breeding-area observers**

In `effect_queue::enqueue_from_permanent` or the caller fan-out for `OnOptionPlaced`, include the owner's breeding permanent when the effect scope/condition allows `[Breeding]`. Do not globally scan breeding for unrelated timings unless Group 1 already introduced the general helper.

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- on_option_placed_fires_for_training_link_and_security_placement_with_event_card
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed
```

Expected: PASS.

- [ ] **Step 6: Update trackers and commit**

Update `qa/archetype-qa/engine-gaps.md` entry `G-OPTION-PLACED-TIMING` to remove Link, Training, security placement, and breeding fan-out only when covered by passing tests. Keep once-per-turn Royal Knights loop open if `max_per_turn` in breeding is not proven.

```bash
git add code/digimon-engine/src/game_actions.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/effect_queue.rs code/digimon-engine/tests/option_flow/option_placed_observers.rs qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md
git commit -m "feat: dispatch option placement observers consistently"
```

## Task 5: DSL Link Requirement, Free Link Step, and Linked Scope

**Files:**
- Modify: `code/digimon-engine/../digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-engine/../digimon-dsl/src/clause.rs`
- Modify: `code/digimon-engine/../digimon-dsl/src/compiler/clause.rs`
- Modify: `code/digimon-engine/../digimon-dsl/src/step.rs`
- Modify: `code/digimon-engine/src/dsl_cards/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_triggered.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/mod.rs`
- Test: `code/digimon-engine/tests/dsl/link.rs`
- Test: `code/digimon-engine/tests/option_flow/link_flow.rs`

- [ ] **Step 1: Write the failing DSL tests**

Create `code/digimon-engine/tests/dsl/link.rs`:

```rust
use digimon_dsl::parse_card_yaml;
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::enums::EffectTiming;

#[test]
fn link_requirement_clause_lowers_to_link_option_effect() {
    let yaml = r#"
card: ST22-08
name: Offensive Plug-In V
kind: option
effects:
  - kind: link_requirement
    scope: inherited
    cost: 2
    filter: { level_gte: 3 }
"#;
    let card = parse_card_yaml(yaml).expect("parse link requirement");
    let dsl = DslCardEffect::new(std::sync::Arc::new(card));
    let effects = dsl.effects(CardHandle(0));
    assert!(effects.iter().any(|e| e.timing == EffectTiming::OptionMain && e.link_requirement.is_some()));
}

#[test]
fn linked_scope_lowers_to_linked_effect_flag() {
    let yaml = r#"
card: ST22-08
name: Offensive Plug-In V
kind: option
effects:
  - scope: linked
    when: end_of_your_turn
    optional: true
    process:
      - gain_memory: 1
"#;
    let card = parse_card_yaml(yaml).expect("parse linked scope");
    let dsl = DslCardEffect::new(std::sync::Arc::new(card));
    let effects = dsl.effects(CardHandle(0));
    assert!(effects.iter().any(|e| e.linked && e.timing == EffectTiming::EndOfYourTurn));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- link_requirement_clause_lowers_to_link_option_effect linked_scope_lowers_to_linked_effect_flag
```

Expected: FAIL because `link_requirement` and `scope: linked` are not valid DSL vocabulary.

- [ ] **Step 3: Add DSL vocabulary and lowering**

Add `CompiledScope::Linked`:

```rust
pub enum CompiledScope {
    FaceUp,
    Inherited,
    Linked,
}
```

Add a declarative link body:

```rust
LinkRequirement {
    scope: CompiledScope,
    cost: u16,
    filter: CompiledPredicate,
}
```

Lower it to:

```rust
Effect::on_play(card)
    .link(cost, move |ctx, host| eval_predicate(&filter, ctx, PredicateSubject::Permanent(host)))
    .build()
```

When `scope == CompiledScope::Linked`, lower triggered clauses with:

```rust
builder = builder.linked();
```

- [ ] **Step 4: Add the free link process step and action-mask test**

Append to `code/digimon-engine/tests/option_flow/link_flow.rs`:

```rust
#[test]
fn dsl_free_link_step_surfaces_host_selection_mask() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("ST22-08", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["ST22-08"])
        .memory(0)
        .start();
    r.register_dsl_yaml("ST22-08", r#"
card: ST22-08
name: Offensive Plug-In V
kind: option
effects:
  - scope: face_up
    when: main
    optional: true
    process:
      - link_to_own_digimon:
          optional: true
          free: true
          filter: { kind: digimon }
"#);
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let mask = r.game.get_action_mask(0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    assert_eq!(mask[action as usize], 1.0);
    r.game.resolve_selection(0, action);
    assert_eq!(r.game.player(0).battle_area[host.index as usize].linked_cards.len(), 1);
}
```

Add `StepSpec::LinkToOwnDigimon { optional, free, filter }` and lower to the same host-selection installer used by `dispose_option` Link flow. Do not attach automatically when exactly one host exists; the selection must remain visible.

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- link_requirement_clause_lowers_to_link_option_effect linked_scope_lowers_to_linked_effect_flag
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- dsl_free_link_step_surfaces_host_selection_mask
```

Expected: PASS.

- [ ] **Step 6: Update trackers and commit**

Update `qa/dsl-vocab-gaps.md` entries `G-DSL-LINK-VERB` and `G-DSL-LINKED-SCOPE`. Keep `G-BINDING-DP-FORMULA` open for ST22-08's DP comparison unless that separate formula work has landed.

```bash
git add code/digimon-engine/../digimon-dsl/src/compiled.rs code/digimon-engine/../digimon-dsl/src/clause.rs code/digimon-engine/../digimon-dsl/src/compiler/clause.rs code/digimon-engine/../digimon-dsl/src/step.rs code/digimon-engine/src/dsl_cards code/digimon-engine/tests/dsl/link.rs code/digimon-engine/tests/option_flow/link_flow.rs qa/dsl-vocab-gaps.md
git commit -m "feat: add dsl link option vocabulary"
```

## Task 6: Training Scope Refinement

**Files:**
- Modify: `code/digimon-engine/src/effect_queue.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Test: `code/digimon-engine/tests/option_flow/training_flow.rs`

- [ ] **Step 1: Write the failing scope test**

Append to `code/digimon-engine/tests/option_flow/training_flow.rs`:

```rust
#[test]
fn training_sideways_effect_applies_only_to_its_intended_trained_permanent() {
    let witness = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(option_card("TRAIN-SCOPE", 0, CardColor::Red))
        .add_card(digimon_card("TRAINED", CardColor::Red))
        .add_card(digimon_card("UNTRAINED", CardColor::Red))
        .hand(0, &["TRAIN-SCOPE"])
        .memory(0)
        .start();
    r.register_effect("TRAIN-SCOPE", Arc::new(TrainingWithInheritedHatch(witness.clone())));
    let trained = r.place_on_field(0, "TRAINED", Some(0));
    let untrained = r.place_on_field(0, "UNTRAINED", Some(0));
    advance_to_main(&mut r);

    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Trashed);
    r.game.bind_training_to_permanent("TRAIN-SCOPE", trained);
    r.game.enqueue_triggered(EffectTiming::OnHatch, digimon_engine::selection::TriggerSource::Permanent(untrained));
    r.game.drain_effect_queue();
    assert_eq!(*witness.lock().unwrap(), 0, "untrained permanent does not receive Training effect");
    r.game.enqueue_triggered(EffectTiming::OnHatch, digimon_engine::selection::TriggerSource::Permanent(trained));
    r.game.drain_effect_queue();
    assert_eq!(*witness.lock().unwrap(), 1, "trained permanent receives Training effect");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- training_sideways_effect_applies_only_to_its_intended_trained_permanent
```

Expected: FAIL because current `effect_queue` comments state Training scans may apply to all owner permanents.

- [ ] **Step 3: Store Training attachment scope**

Extend `OptionState::Training` from:

```rust
Training { owner: PlayerId }
```

to:

```rust
Training {
    owner: PlayerId,
    trained: Option<PermanentHandle>,
}
```

Update constructors to use `trained: None`, then set it when a card effect explicitly binds the Training to a permanent. If printed Training cards define a specific carrier at placement, set the carrier in `dispose_option`; if the carrier is chosen later, expose it through a pending selection.

- [ ] **Step 4: Gate the Training scan**

In `effect_queue.rs`, when scanning `OptionState::Training`, include an effect only when:

```rust
trained.is_none() || trained == Some(source_permanent_handle)
```

Do not use owner-wide fan-out for printed cards whose text implies a specific trained permanent.

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- training_sideways_effect_applies_only_to_its_intended_trained_permanent
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- training_parks_alongside_breeding training_trashes_on_breeding_promotion
```

Expected: the new scope test PASS and the existing Training flow tests PASS.

- [ ] **Step 6: Update trackers and commit**

If no tracker entry exists for Training over-broad fan-out, add one to `docs/RUST_ENGINE_GAPS.md` before marking it resolved with test evidence.

```bash
git add code/digimon-engine/src/permanent.rs code/digimon-engine/src/effect_queue.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/tests/option_flow/training_flow.rs docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md
git commit -m "fix: scope training option sideways effects"
```

## Task 7: Scheduled End-of-Turn Option Effects

**Files:**
- Modify: `code/digimon-engine/src/scheduled_effects.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/game_phases.rs`
- Test: `code/digimon-engine/tests/option_flow/standard_flow.rs`

- [ ] **Step 1: Write the failing scheduled option test**

Append to `code/digimon-engine/tests/option_flow/standard_flow.rs`:

```rust
#[test]
fn transient_option_scheduled_end_of_turn_effect_replays_with_option_source() {
    let witness = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(option_card("SCHEDULED-OPT", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .hand(0, &["SCHEDULED-OPT"])
        .memory(0)
        .start();
    r.register_effect("SCHEDULED-OPT", Arc::new(StandardSchedulesEndOfTurn(witness.clone())));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Trashed);
    assert_eq!(*witness.lock().unwrap(), 0);
    r.end_turn();
    assert_eq!(*witness.lock().unwrap(), 1);
}
```

Add the local helper:

```rust
struct StandardSchedulesEndOfTurn(Arc<Mutex<u32>>);

impl CardEffect for StandardSchedulesEndOfTurn {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::on_play(card)
            .name("Schedule end turn")
            .option_main()
            .process(move |ctx| {
                ctx.schedule_delayed_steps(
                    EffectTiming::EndOfYourTurn,
                    vec![CompiledStep::RawRust { fn_name: "increment_witness".to_string() }],
                );
                ctx.set_test_witness(seen.clone());
            })
            .build()]
    }
}
```

If test-only raw-rust witness plumbing does not exist, use a scheduled `gain_memory: 1` body and assert memory instead of a witness counter.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- transient_option_scheduled_end_of_turn_effect_replays_with_option_source
```

Expected: FAIL if scheduled effects do not drain at end turn, lose `EffectSourceKind::Option`, or cannot replay after the transient option is trashed.

- [ ] **Step 3: Implement scheduled option replay**

Ensure `EffectContext::schedule_delayed` captures:

```rust
source_card,
source_permanent,
source_kind: EffectSourceKind::Option,
controller,
captured_bindings,
scheduled_at_turn,
runtime,
```

Ensure `Game::end_turn` calls:

```rust
crate::scheduled_effects::fire_scheduled_for_timing(self, EffectTiming::EndOfYourTurn);
crate::scheduled_effects::fire_scheduled_for_timing(self, EffectTiming::EndOfYourNextTurn);
```

Preserve the existing guard that `EndOfYourNextTurn` requires `current_turn > scheduled_at_turn`.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- transient_option_scheduled_end_of_turn_effect_replays_with_option_source
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- standard_option_trashes_after_resolve standard_option_fires_on_use_option_globally
```

Expected: the new test PASS and existing Standard option tests PASS.

- [ ] **Step 5: Update trackers and commit**

Update `docs/RUST_ENGINE_GAPS.md` entry "Scheduled end-of-turn effect queue (for transient Options)" with the passing test command.

```bash
git add code/digimon-engine/src/scheduled_effects.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/game_phases.rs code/digimon-engine/tests/option_flow/standard_flow.rs docs/RUST_ENGINE_GAPS.md
git commit -m "feat: replay scheduled option effects"
```

## Task 8: Replacement Delay Handoff and Contract Review

**Files:**
- Read: `docs/superpowers/plans/2026-04-30-gap-group-3-task-5-delay-replacement.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Review/modify: `docs/ACTION_SPEC.md` if an action id, action decoder shape, or mask contract changed.
- Review/modify: `docs/TENSOR_SPEC.md` if tensor shape or encoded Option/linked fields changed.

- [ ] **Step 1: Verify replacement-window ownership**

Run:

```bash
Get-Content docs/superpowers/plans/2026-04-30-gap-group-3-task-5-delay-replacement.md
```

Expected: The plan names `BT17-097` Return to the Primogenitor and owns Delay-as-replacement prevention. If it is incomplete, execute or update that child plan before closing Group 5 replacement rows.

- [ ] **Step 2: Run the Group 5 regression set**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed
```

Expected: PASS. Any ignored test mentioning `G-DELAY-START-OF-TURN`, `G-DSL-LINK-VERB`, `G-DSL-LINKED-SCOPE`, `G-PLACE-SELF-AS-OPTION-PERMANENT`, or scheduled option effects must be unignored or moved to a still-open tracker entry.

- [ ] **Step 3: Review action/tensor/PyO3 contracts**

If any task added or changed selection action ids, update:

```text
docs/ACTION_SPEC.md
code/digimon-engine/src/action/space.rs
code/digimon-engine/src/action/mask.rs
code/digimon-engine-py/src/lib.rs
code/digimon_gym/digimon_gym.py
code/frontend/src/**/action constants
```

If no ids changed, add this tracker note instead:

```text
Group 5 did not change ACTION_SPACE_SIZE or TENSOR_SIZE. New Link/Delay choices reuse existing pending-selection masks.
```

- [ ] **Step 4: Final tracker closure**

Update trackers precisely:

```text
docs/RUST_ENGINE_GAPS.md
qa/archetype-qa/engine-gaps.md
qa/dsl-vocab-gaps.md
```

For each closed gap, include the exact passing command. For each still-open card-level blocker, leave the gap open and name the remaining primitive, such as `G-BINDING-DP-FORMULA`, `G-COLOR-MATCH-AGAINST-BOARD`, or breeding permanent selection.

- [ ] **Step 5: Commit**

```bash
git add docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md docs/ACTION_SPEC.md docs/TENSOR_SPEC.md
git commit -m "docs: close option delay link trackers"
```

## Self-Review Checklist

- [ ] Scope coverage: The plan covers start-of-turn Delay (`LM-027`), event-gated Delay (`BT22-098`), replacement Delay handoff (`BT17-097`), Plug-In/Link (`ST22-08`), Training scope, option-placement observers (`BT13-110`), inherited-security placement, and scheduled option effects.
- [ ] No hidden choices: Link host choice and any Training carrier choice are surfaced through `PendingSelection`; no single-target auto-selection is allowed.
- [ ] Action masks: New player-visible Link/Training choices include mask assertions; if a new action id is introduced, action/tensor/PyO3/frontend docs must be updated in the same task.
- [ ] Tracker discipline: No gap is marked closed without a passing test command and exact tracker edit.
- [ ] Type consistency: `DelayTrigger`, `OptionState::Delayed`, `OptionState::Training`, `TriggerSource::OptionPlaced`, and DSL `CompiledScope::Linked` names are used consistently across tasks.
- [ ] Placeholder scan: There is no task that says to add tests or implementation without a concrete file, command, and expected result.
