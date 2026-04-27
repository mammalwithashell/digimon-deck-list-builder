# Card Scripting DSL — Phase 3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every remaining gap between Phase 2's feature-complete step vocabulary and the boss-tier 15 worked examples in the design spec — full replacement / partition / delay integration with the Phase 2 step vocabulary, formula primitives beyond literals, scheduled-body re-entrancy, and the DNA-digivolve trigger wiring needed for cards like BT18-015 Kimeramon.

**Architecture:** Phase 3 splits into **five independent sub-phases (3a–3e)**, each shippable on its own. Sub-phase ordering is by dependency: 3a's IR widenings unblock 3b's replacement bodies and 3e's `OnDnaDigivolve` wiring; 3c (partition + delay) and 3d (formula / event predicates / scheduled generation counter) stand alone and can land in parallel with 3b.

This plan details **3a in full**. Sub-phases 3b–3e carry scoped task outlines — each gets expanded into its own dated detailed plan file (`2026-04-XX-card-scripting-dsl-phase-3b.md` etc.) when the prior sub-phase lands, mirroring the pattern that produced separate plans for 2a/2b/2c/2d/2e and a single rolled-up plan for 2f.

**Tech Stack:** Rust 2021, `digimon-engine`, `digimon-dsl` leaf crate.

---

## Sub-phase decomposition

| Sub-phase | Scope | Depends on |
|---|---|---|
| **3a** | IR widenings: `BindingValue::HandIndex` / `TrashIndex` carry `PlayerId`; `EffectInitiatedDigivolve.cost` and `EffectInitiatedDnaDigivolve.cost` widen from `i32` to `CompiledCostDelta` so `Reduce(n)` is expressible. | — |
| **3b** | Replacement clause body-step integration: `replacement: { trigger, process: [...] }` runs Phase 2 step vocabulary inside the process body, including `select_*`, `delete_permanent`, `add_modifier`, control flow. Existing standalone declarative lowering (#345) currently terminates without invoking `run_steps`. | 3a |
| **3c** | Partition + Delay clause body-step integration. Same pattern as 3b but for `partition` and `delay` declaratives. Includes `Delay { trigger, until_timing }` lowering for printed-text "[Delay] At end of your next turn, …". | — (parallelizable with 3b) |
| **3d** | Formula primitives + event predicates + ScheduledEffect generation counter: (i) `raw_rust` formula registry dispatch; (ii) `CardCountInZone` zone payload; (iii) opponent / universal `Aggregate` scope (currently scoped to `ctx.player`); (iv) broader `event_target_*` predicates (event source player, timing context); (v) generation counter on `ScheduledEffect` so `EndOfYourNextTurn` / `EndOfOpponentsNextTurn` / `UntilNextUnsuspend` fire on the *next* matching boundary, not the immediate one. | — (parallelizable) |
| **3e** | Re-entrancy + DNA digivolve trigger wiring: (i) multi-parking drains in `ScheduledEffect` so a scheduled body that itself parks a selection resumes correctly (today's Phase 2f4 `debug_assert!(game.dsl_outer_tail.is_none())` becomes a real loop); (ii) `OnDnaDigivolve` trigger wiring fired from both `effect_initiated_dna_digivolve` (effect-initiated) and the canonical user-action DNA digivolve flow. | 3a |

Each sub-phase ships independently. Within a sub-phase, tasks are sequential TDD-shaped (failing test → implementation → green run → commit).

**Phase 3 exit criteria:** every variant of `CompiledClause` and `CompiledStep` in the IR runs end-to-end through `DslCardEffect`. The 15 worked examples in design-spec §10 all compile and pass behavioural tests. Hand-written-`CardEffect` footprint shrinks to ~1,000 cards (the tail covered by Phase 4 raw_rust).

---

# Sub-phase 3a — IR widenings

**Goal:** Two backward-compatible-where-possible IR shape changes that unblock 3b and 3e: `BindingValue::HandIndex` / `TrashIndex` carry `PlayerId`, and `EffectInitiated{,Dna}Digivolve.cost` widens from `i32` to `CompiledCostDelta`.

**Why now:** Replacement bodies (3b) routinely move cards between *opponent* zones (e.g. "instead of being deleted, place this in your opponent's trash"). Today's `BindingValue::HandIndex(u16)` / `TrashIndex(u16)` shape carries no controller — every consumer assumes `ctx.player`. That assumption breaks for cross-controller placement. Widening upfront avoids retrofitting every consumer in 3b. Likewise, `EffectInitiatedDigivolve` cards that read "Digivolve cost reduced by N" need `CostDelta::Reduce(n)`; today's bare `i32` cost field admits only `Free (0)` and `Fixed(n)`.

**Architecture:** Pure mechanical widening + adapter-call updates. No new step variants, no new engine primitives. Every consumer of the affected types either gets the player threaded through or asserts `ctx.player` for now (with a `TODO(3b-cross-controller)` comment if the consumer is provably called only from same-controller paths today).

## Files

- Modify: `code/digimon-engine/src/dsl_cards/bindings.rs` — widen `BindingValue::HandIndex` / `TrashIndex` variants from `u16` to `(PlayerId, u16)`.
- Modify: `code/digimon-engine/src/dsl_cards/binding_ref.rs` — update `ResolvedBinding` and the helpers that pull integers out (`get_hand_index_ref` / `get_trash_index_ref`).
- Modify: `code/digimon-engine/src/dsl_cards/step/selections.rs` — every `bindings.insert_hand_index(...)` / `insert_trash_index(...)` call site threads the selecting player.
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs` — consumers of `HandIndex` / `TrashIndex` use the new `(PlayerId, u16)` tuple (verify each is operating on the right player; use the new `PlayerId` arm rather than reading `ctx.player`).
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` — `play_from_hand` / `play_from_trash` paths take their player from the binding.
- Modify: `code/digimon-dsl/src/compiled.rs` — change `EffectInitiatedDigivolve.cost: i32` and `EffectInitiatedDnaDigivolve.cost: i32` to `cost: CompiledCostDelta`.
- Modify: `code/digimon-dsl/src/compile.rs` — adapter that produces `CompiledCostDelta` from the parsed `CardSpec`.
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` — `effect_initiated_digivolve` / `effect_initiated_dna_digivolve` step handlers convert `CompiledCostDelta` → `crate::enums::CostDelta` via the existing `lower_cost_delta` helper.
- Create: `code/digimon-engine/tests/dsl/phase3a_binding_player_id.rs` — fixture exercising opponent-trash placement via a binding.
- Create: `code/digimon-engine/tests/dsl/phase3a_cost_delta.rs` — fixture exercising `cost: { reduce: 2 }` on `effect_initiated_digivolve`.
- Modify: `code/digimon-engine/tests/dsl/main.rs` — register both new test modules.

## Task 3a.1: Widen `BindingValue::HandIndex` / `TrashIndex` to carry `PlayerId`

- [ ] **Step 1: Add the failing test**

Create `code/digimon-engine/tests/dsl/phase3a_binding_player_id.rs`:

```rust
//! Phase 3a — binding values for Hand/Trash indices carry PlayerId.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn binding_value_hand_index_records_player_id() {
    use digimon_engine::dsl_cards::bindings::{Bindings, BindingValue};

    let mut b = Bindings::new();
    b.insert_hand_index("h", /* player */ 1, /* index */ 3);

    match b.get("h") {
        Some(BindingValue::HandIndex(player, idx)) => {
            assert_eq!(player, 1);
            assert_eq!(idx, 3);
        }
        other => panic!("expected HandIndex(1, 3), got {:?}", other),
    }
}

#[test]
fn binding_value_trash_index_records_player_id() {
    use digimon_engine::dsl_cards::bindings::{Bindings, BindingValue};

    let mut b = Bindings::new();
    b.insert_trash_index("t", /* player */ 0, /* index */ 7);

    match b.get("t") {
        Some(BindingValue::TrashIndex(player, idx)) => {
            assert_eq!(player, 0);
            assert_eq!(idx, 7);
        }
        other => panic!("expected TrashIndex(0, 7), got {:?}", other),
    }
}
```

Append `mod phase3a_binding_player_id;` to `code/digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run the test — expect failure**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3a_binding_player_id
```

Expected: compile error — `insert_hand_index` takes `(name, u16)`, not `(name, PlayerId, u16)`.

- [ ] **Step 3: Widen the variants**

In `code/digimon-engine/src/dsl_cards/bindings.rs`:

```rust
pub enum BindingValue {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(PlayerId, u16),
    TrashIndex(PlayerId, u16),
    Literal(i64),
    PermanentList(Vec<PermanentHandle>),
    CardList(Vec<CardHandle>),
}
```

Update the helper inserts:

```rust
pub fn insert_hand_index(&mut self, name: &str, player: PlayerId, i: u16) {
    self.insert(name, BindingValue::HandIndex(player, i));
}

pub fn insert_trash_index(&mut self, name: &str, player: PlayerId, i: u16) {
    self.insert(name, BindingValue::TrashIndex(player, i));
}
```

And the readers (`get_hand_index` / `get_trash_index`) — keep them returning `Option<u16>` but now also expose `get_hand_index_with_player` / `get_trash_index_with_player`:

```rust
pub fn get_hand_index(&self, name: &str) -> Option<u16> {
    match self.get(name)? {
        BindingValue::HandIndex(_, i) => Some(i),
        _ => None,
    }
}

pub fn get_hand_index_with_player(&self, name: &str) -> Option<(PlayerId, u16)> {
    match self.get(name)? {
        BindingValue::HandIndex(p, i) => Some((p, i)),
        _ => None,
    }
}

// Mirror for trash.
```

- [ ] **Step 4: Update every call site**

Every `insert_hand_index(name, idx)` becomes `insert_hand_index(name, player, idx)`. Audit:

```
cargo build --manifest-path code/digimon-engine/Cargo.toml 2>&1 | grep "error\[" | head -20
```

Expected sites (non-exhaustive — compiler will list all):

- `code/digimon-engine/src/dsl_cards/step/selections.rs` — `SelectHand` / `SelectTrash` callbacks. Player is `ctx.player` for own-player selections; for `AsSelectingPlayer` overrides, use `ctx.selecting_player()`.
- `code/digimon-engine/src/dsl_cards/binding_ref.rs` — `resolve_binding_ref` and `ResolvedBinding::HandIndex` / `TrashIndex` arms.
- `code/digimon-engine/src/dsl_cards/step/zone_moves.rs` — `add_to_hand_from_trash`, `move_to_trash`, etc. consumers.
- `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` — `play_from_hand_*` / `play_from_trash_*` arms.

For every consumer that previously read `ctx.player` to derive the zone owner, switch to reading the player from the resolved binding. If a path only ever fires with `ctx.player`-owned indices (provably from the test corpus), a `debug_assert_eq!(player, ctx.player, "TODO(3b-cross-controller): cross-controller binding")` is acceptable.

- [ ] **Step 5: Run the tests — expect pass**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3a_binding_player_id
```

Expected: PASS.

- [ ] **Step 6: Run the full DSL test suite**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
```

Expected: every existing DSL test still passes. Failures here mean a call site was missed in Step 4.

- [ ] **Step 7: Run the full engine suite**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/dsl_cards/bindings.rs \
        code/digimon-engine/src/dsl_cards/binding_ref.rs \
        code/digimon-engine/src/dsl_cards/step/ \
        code/digimon-engine/tests/dsl/phase3a_binding_player_id.rs \
        code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl(3a): widen BindingValue Hand/Trash indices to carry PlayerId"
```

## Task 3a.2: End-to-end opponent-trash placement fixture

Verifies the widening unblocks the actual use case: a binding can target an opponent's zone.

- [ ] **Step 1: Add the failing test**

Append to `code/digimon-engine/tests/dsl/phase3a_binding_player_id.rs`:

```rust
#[test]
fn select_opponent_trash_then_move_routes_through_player_id_binding() {
    use digimon_dsl::compile::compile;
    use digimon_dsl::CardSpec;
    use std::sync::Arc;

    let yaml = r#"
card: DSL-3A-001
name: OppTrashMover
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_trash:
          of: opponent
          bind_as: pick
          filter: {}
          prompt: "Pick from opponent trash"
      - place_on_security: { player: opponent, card: pick }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = compile(&spec).expect("compiles");

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("DSL-3A-001", "OppTrashMover"))
        .add_card(make_test_card("VICTIM", "Victim"))
        .hand(0, &["DSL-3A-001"])
        .memory(5)
        .build();

    // Seed opponent trash with the victim card.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "VICTIM")
        .unwrap();
    let card_index = runner.game.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_idx, /* opponent */ 1, card_index);
    runner.game.players[1].trash.push(card);

    runner.register_effect(
        "DSL-3A-001",
        Arc::new(digimon_engine::dsl_cards::DslCardEffect::new(Arc::new(compiled))),
    );

    runner.play(0, 0);
    runner.auto_resolve();

    assert_eq!(runner.game.players[1].trash.len(), 0, "opponent trash drained");
    assert_eq!(runner.game.players[1].security.len(), 1, "victim moved to opp security");
}
```

- [ ] **Step 2: Run, fail, implement until green**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl select_opponent_trash_then_move_routes_through_player_id_binding
```

Expected to fail until Task 3a.1's call-site updates correctly thread the opponent player ID through `select_trash` → binding insert → `place_on_security` consumer. Drive each compile error into a fix.

- [ ] **Step 3: Commit when green**

```bash
git commit -am "dsl(3a): opponent-trash → opponent-security routing parity test"
```

## Task 3a.3: Widen `EffectInitiatedDigivolve.cost` to `CompiledCostDelta`

- [ ] **Step 1: Add the failing parse + behavioural test**

Create `code/digimon-engine/tests/dsl/phase3a_cost_delta.rs`:

```rust
//! Phase 3a — `cost: { reduce: 2 }` lowering on effect_initiated_digivolve.

use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{CompiledCostDelta, CompiledStep};
use digimon_dsl::CardSpec;

#[test]
fn effect_initiated_digivolve_compiles_reduce_cost() {
    let yaml = r#"
card: DSL-3A-002
name: ReducedDigivolve
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: base
          filter: { kind: digimon }
          prompt: "Base"
      - select_hand:
          of: you
          bind_as: evo
          filter: { kind: digimon }
          prompt: "Evo"
      - effect_initiated_digivolve:
          target: base
          from_hand: evo
          cost: { reduce: 2 }
          ignore_requirements: false
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = compile(&spec).expect("compiles");

    let triggered = match compiled.effects.first() {
        Some(digimon_dsl::compiled::CompiledClause::Triggered(t)) => t,
        _ => panic!("expected triggered clause"),
    };

    let last_step = triggered.process.last().expect("process has steps");
    match last_step {
        CompiledStep::EffectInitiatedDigivolve { cost, .. } => {
            assert_eq!(*cost, CompiledCostDelta::Reduce(2), "cost is Reduce(2)");
        }
        other => panic!("expected EffectInitiatedDigivolve, got {:?}", other),
    }
}
```

Append `mod phase3a_cost_delta;` to `code/digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run the test — expect failure**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3a_cost_delta
```

Expected: compile error on `cost: CompiledCostDelta` because the field is still `cost: i32`.

- [ ] **Step 3: Widen the IR**

In `code/digimon-dsl/src/compiled.rs`:

```rust
CompiledStep::EffectInitiatedDigivolve {
    target: CompiledBindingRef,
    from_hand: CompiledBindingRef,
    cost: CompiledCostDelta,
    ignore_requirements: bool,
},
CompiledStep::EffectInitiatedDnaDigivolve {
    target_a: CompiledBindingRef,
    target_b: CompiledBindingRef,
    from_hand: CompiledBindingRef,
    cost: CompiledCostDelta,
    ignore_requirements: bool,
},
```

`CompiledCostDelta` already exists (Phase 2f1 introduced it for `play_*` steps). If not exposed publicly, re-export from `compiled.rs`:

```rust
pub use crate::step::CompiledCostDelta; // or move the def to compiled.rs
```

- [ ] **Step 4: Update the compile pass**

In `code/digimon-dsl/src/compile.rs`, the arm that lowers the YAML `effect_initiated_digivolve` step. Today it parses `cost: i32` and writes `cost: i32`. Widen to accept the same `CostDeltaSpec` enum 2f1 introduced (`Free`, `Printed`, `Literal(n)`, `Reduce(n)`, `Fixed(n)`) and lower to `CompiledCostDelta`.

- [ ] **Step 5: Update the engine step handler**

In `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`, the `effect_initiated_digivolve` handler:

```rust
CompiledStep::EffectInitiatedDigivolve { target, from_hand, cost, ignore_requirements } => {
    let cost_delta = lower_cost_delta(cost);  // existing helper
    // …rest of handler, passing cost_delta to ctx.effect_initiated_digivolve
}
```

`EffectContext::effect_initiated_digivolve` already accepts `crate::enums::CostDelta` (Phase 2f1 work). No new engine primitive needed.

- [ ] **Step 6: Run, fail, implement**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase3a_cost_delta
```

Drive to PASS.

- [ ] **Step 7: Add a behavioural test**

Append to the same fixture file:

```rust
#[test]
fn effect_initiated_digivolve_with_reduce_cost_pays_reduced_amount() {
    use digimon_engine::card_data::{CardData, EvoCost};
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::enums::{CardColor, CardKind};
    use std::sync::Arc;

    let yaml = r#"
card: DSL-3A-002-CARD
name: ReducedDigivolve
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: base
          filter: { kind: digimon }
          prompt: "Base"
      - select_hand:
          of: you
          bind_as: evo
          filter: { kind: digimon }
          prompt: "Evo"
      - effect_initiated_digivolve:
          target: base
          from_hand: evo
          cost: { reduce: 2 }
          ignore_requirements: false
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles");

    // BASE: an Lv.3 Digimon already on the field — eligible to be digivolved.
    let mut base_card = make_test_card("BASE", "Base");
    base_card.level = Some(3);
    base_card.dp = Some(2000);

    // EVO: an Lv.4 Digimon with printed digivolve cost 4 from level 3 / red.
    let mut evo_card = make_test_card("EVO", "Evo");
    evo_card.level = Some(4);
    evo_card.dp = Some(4000);
    evo_card.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 4,
    }];

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("DSL-3A-002-CARD", "ReducedDigivolve"))
        .add_card(base_card)
        .add_card(evo_card)
        .hand(0, &["DSL-3A-002-CARD", "EVO"])
        .memory(3) // exactly enough for printed 4 - reduce(2) = 2 cost
        .build();

    // Place BASE on the field first so the select_own_permanent step has a target.
    runner.place_on_field(0, "BASE", Some(0));

    runner.register_effect(
        "DSL-3A-002-CARD",
        Arc::new(digimon_engine::dsl_cards::DslCardEffect::new(Arc::new(compiled))),
    );

    // Memory before play: 3. Play DSL-3A-002-CARD costs 5 memory by itself —
    // pre-fund accordingly. Adjust .memory(...) if the test card's printed cost
    // changes from the YAML spec above.
    let mem_before = runner.memory();
    runner.play(0, 0); // plays DSL-3A-002-CARD; OnPlay installs selections
    runner.auto_resolve(); // pick BASE, then pick EVO; effect_initiated_digivolve fires

    // Net memory delta: -5 (play cost) + 0 (gain_memory absent here)
    //                   - (4 - 2) (digivolve reduced cost) = -7 from baseline 3.
    // Expected final memory: 3 - 5 - 2 = -4 (player loses memory; turn ends).
    assert_eq!(
        runner.memory(),
        mem_before - 5 - 2,
        "play cost + reduced digivolve cost both deducted"
    );

    // BASE is now stacked under EVO — battle area still has 1 entry, top is EVO.
    assert_eq!(runner.battle_area_size(0), 1);
    let perm = runner.perm_handle(0, 0);
    let perm_ref = &runner.game.players[0].battle_area[perm.index as usize];
    assert_eq!(perm_ref.card_sources.len(), 2, "EVO stacked on BASE");
}
```

Run, drive to PASS. If the engine `EffectContext::effect_initiated_digivolve` API differs from the assumptions above (cost-delta semantics, stacking expectations), follow the existing `phase2f1_effect_initiated_digivolve_full.rs` fixture as the canonical example and mirror its assertion shape.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-dsl/src/ \
        code/digimon-engine/src/dsl_cards/step/play_digivolve.rs \
        code/digimon-engine/tests/dsl/phase3a_cost_delta.rs \
        code/digimon-engine/tests/dsl/main.rs
git commit -m "dsl(3a): widen EffectInitiatedDigivolve.cost to CompiledCostDelta"
```

## Task 3a.4: Spec note + 3a closeout

- [ ] **Step 1: Append a 3a entry to the design spec**

In `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`, after the 2f4 sub-phase entry, add:

```markdown
- **3a** (landed YYYY-MM-DD) — IR widenings: `BindingValue::HandIndex` /
  `TrashIndex` carry `PlayerId` (cross-controller binding now expressible);
  `EffectInitiated{,Dna}Digivolve.cost` widens from `i32` to
  `CompiledCostDelta` so `Reduce(n)` lowers (was Phase 2f1's tracked
  follow-up). End-to-end fixtures at
  `code/digimon-engine/tests/dsl/phase3a_*.rs`.
```

- [ ] **Step 2: Run the full engine suite**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-04-21-card-scripting-dsl.md
git commit -m "docs(dsl): note Phase 3a landing"
```

---

# Sub-phase 3b — Replacement clause body integration

**Goal:** A `replacement` declarative clause's `process: [...]` body runs Phase 2 step vocabulary — `select_*`, `delete_permanent`, `add_modifier`, `add_to_hand_from_trash`, control flow — instead of terminating after the standalone declarative lowering.

**Why:** Today's lowering at `code/digimon-engine/src/dsl_cards/lower_replacement.rs` (#345) only registers the replacement subscription. When the replacement fires, the body is a no-op. Cards like Evade ("Once per turn, when this Digimon would be deleted, you may unsuspend it instead"), Partition-tier replacements, and EX5-015's printed-text replacement need the body to run real engine mutations.

**Architecture:** The replacement-fire path already invokes a handler at the right engine moment — the handler currently drops the body. Phase 3b threads `Bindings` into the replacement handler so it can call `run_steps`. Re-uses Phase 2c/2d step vocabulary verbatim.

**Depends on:** 3a (cross-controller bindings — replacement bodies often touch opponent zones).

## Task outline (expand into a dated detailed plan when 3a lands)

1. Add a `body: Vec<CompiledStep>` field on `CompiledReplacementClause` (already there per 2e? — verify and extend if missing).
2. Thread `Bindings` through the replacement-fire callback.
3. Invoke `run_steps(ctx, &mut bindings, body)` inside the callback. Park-resume already handled by existing `run_steps` continuation dispatcher.
4. Behavioural test fixtures: Evade replacement, "would be deleted → place under another Digimon" replacement, "would lose security → trash a hand card instead" replacement.
5. Optional-replacement (`PASS` to decline) parity — verify existing `Replacement` selection plumbing already handles the decline flow.
6. Spec note + 3b closeout.

**Detailed plan path (when expanded):** `docs/superpowers/plans/2026-04-XX-card-scripting-dsl-phase-3b.md`.

---

# Sub-phase 3c — Partition + Delay clause body integration

**Goal:** Same as 3b but for `partition` and `delay` declaratives.

**Why:** Partition cards (Royal Knights of the Purge, Galacticmon "place 4 Vemmon") and Delay cards (Yellow Scramble, Crescent Leaf, Comet Hammer, In-Between Theater) compose multi-step bodies that today's standalone lowerings drop on the floor. The body-integration story is identical to 3b.

**Architecture:** Same pattern as 3b. `Partition` and `Delay` declaratives gain `body: Vec<CompiledStep>` (or extend existing `process: [...]` field if present). `run_steps` invocation at the right fire-site.

**Depends on:** — (parallelizable with 3b; doesn't need 3a unless a specific partition fixture targets opponent zones).

## Task outline

1. Audit `CompiledPartitionClause` and `CompiledDelayClause` shapes — confirm `body` / `process` fields exist or extend if missing.
2. `Partition` fire-site (post-Phase 2f registration in `lower_partition.rs`): currently iterates `sources` to gate "where can this play come from" — Phase 3c hooks the body to run on the *resolved* partition outcome.
3. `Delay` fire-site: at `until_timing` boundary (engine subsystem from 2f4 already drains scheduled effects — the delay declarative is functionally a `schedule_delayed` aliasing pattern; verify whether 3c can reuse the 2f4 plumbing or needs its own queue).
4. Behavioural fixtures: BT13-110 Royal Knights of the Purge (Delay digi-source iteration), BT22-099 Kuremi Detective Agency (Delay +2 memory), Galacticmon Vemmon partition placement.
5. Spec note + 3c closeout.

**Detailed plan path (when expanded):** `docs/superpowers/plans/2026-04-XX-card-scripting-dsl-phase-3c.md`.

---

# Sub-phase 3d — Formula primitives + event predicates + scheduled generation counter

**Goal:** Five independent items, bundled because each is small but together they unblock a meaningful tail of cards. Sub-tasks 3d1–3d5 are each shippable independently within the sub-phase.

**Why:** Formula gaps surface in cards like Susanoomon ("+1000 DP per material" — covered by 2f2), but `raw_rust` formula registry, `CardCountInZone`, opponent-Aggregate scope are still placeholders. Event predicates are needed for cards reacting to specific event sources. Generation counter on `ScheduledEffect` is needed for any "next turn" wording that today fires immediately.

## Sub-tasks

### 3d1: `raw_rust` formula registry dispatch

Pre-2f2 pattern: the IR has `CompiledFormula::RawRust(String)` for cards whose math doesn't fit the structured formula vocabulary. Today's `formula_eval::evaluate` returns `0` for `RawRust` and emits a TODO. 3d1 wires a `FormulaExtensionRegistry` (mirror of `CardEffectExtensionRegistry`) that maps the string ID to a `fn(&EffectContext, PermanentHandle) -> i32`.

### 3d2: `CardCountInZone` zone payload

Today's `CompiledPerSelector::CardCountInZone` has no zone field — runtime returns `0`. 3d2 widens to `CardCountInZone { zone: CompiledZone, of: CompiledPlayerRef }` and wires the count.

### 3d3: Opponent / universal `Aggregate` scope

`CompiledAggregateSelector::{LowestDp, HighestDp, LowestLevel, HighestLevel}` currently scopes to `ctx.player`. 3d3 widens to `Aggregate(CompiledAggregateSelector, AggregateScope)` where `AggregateScope = You | Opponent | Both`.

### 3d4: Broader `event_target_*` predicates

Currently `event_target_kind`, `event_target_trait_has`, `event_card_trait_has`. 3d4 adds `event_source_player`, `event_target_owner`, `event_timing_is`, `event_was_security_skill`. Predicate evaluator extends.

### 3d5: `ScheduledEffect` generation counter

Add `scheduled_at_turn: u16` field on `ScheduledEffect`. The drainer compares against `game.turn_count` and skips entries scheduled this turn for "next turn" timings (`EndOfYourNextTurn`, `EndOfOpponentsNextTurn`, `UntilNextUnsuspend`). New `CompiledTiming` variants land in the same task.

## Task outline

1. 3d1: extension registry + lowering arm + behavioural test.
2. 3d2: IR widening + lowering + behavioural test.
3. 3d3: IR widening + lowering + behavioural test.
4. 3d4: predicate enum extensions + evaluator + behavioural test per predicate.
5. 3d5: `ScheduledEffect` field + drainer logic + behavioural fixture per new timing.
6. Spec note + 3d closeout.

**Detailed plan path (when expanded):** `docs/superpowers/plans/2026-04-XX-card-scripting-dsl-phase-3d.md`.

---

# Sub-phase 3e — Re-entrancy + DNA digivolve trigger wiring

**Goal:** Two engine-side completeness items that don't fit cleanly into 3a–3d.

## 3e1: Multi-parking drains in `ScheduledEffect`

**Why:** Phase 2f4's `fire_scheduled_for_timing` includes `debug_assert!(game.dsl_outer_tail.is_none())` — a hard panic if a scheduled body parks a selection. Most scheduled bodies are synchronous (`gain_memory`, `draw`, `add_modifier`), but cards like "[Delay] Trash up to 1 of your opponent's Digimon" need the parked selection to resume.

**Architecture:** `fire_scheduled_for_timing` becomes a state machine that can park, resume, and continue draining the queue. Reuses the existing `dsl_outer_tail` continuation primitive 2d introduced.

## 3e2: `OnDnaDigivolve` trigger wiring

**Why:** 2f1 introduced `effect_initiated_dna_digivolve` but didn't fire `OnDnaDigivolve` — a tracked TODO. 3e2 wires the trigger from both fire-sites (effect-initiated and the canonical user-action DNA digivolve flow), unblocking BT18-015 Kimeramon and DNA Omnimon's BT17-007 inherited DNA digivolve.

**Depends on:** 3a (cross-controller bindings if a DNA-digivolve trigger body needs to touch opponent state).

## Task outline

1. 3e1: re-entrancy refactor of `fire_scheduled_for_timing` + behavioural test (parking inside scheduled body resumes correctly).
2. 3e2: identify both fire-sites in `combat.rs` / `game_actions.rs`; emit `OnDnaDigivolve` trigger; wire enqueue-triggered path; behavioural test that an inherited `[OnDnaDigivolve]` clause fires.
3. Spec note + 3e closeout.

**Detailed plan path (when expanded):** `docs/superpowers/plans/2026-04-XX-card-scripting-dsl-phase-3e.md`.

---

## Phase 3 acceptance criteria

After 3a–3e land:

1. Every variant of `CompiledClause` (Triggered, Aura, CostReduction, GrantKeyword, Replacement, Partition, Delay, AceOverflow) runs its body end-to-end.
2. Every variant of `CompiledStep` runs end-to-end.
3. The 15 worked examples in design-spec §10 compile and pass behavioural tests.
4. `cargo test --manifest-path code/digimon-engine/Cargo.toml` stays green throughout.
5. Hand-written-`CardEffect` footprint in `code/digimon-engine/src/cards/` is on track to ~1,000 cards (75% retired, mirroring §7.4 exit criteria).
6. The §7.4 Phase 4 prerequisites are unblocked: only `raw_rust` long-tail cards remain.

## Out of scope

- The `raw_rust` escape hatch itself (Phase 4).
- Card-by-card DSL authoring against the new vocabulary (`/batch-implement-cards-rust-dsl` skill work).
- The example card pool research task from `docs/RUST_DSL_TEST_API.md` §4 (separate dispatch).
- Runner helpers in `docs/superpowers/plans/2026-04-25-runner-helpers-for-dsl-tests.md` (separate plan, parallel track).
