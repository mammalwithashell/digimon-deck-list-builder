# Zone Movement and Stack Operations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close group 4 by proving and completing effect-driven movement between hand, trash, security, breeding, deck, reveal, and digivolution stacks without hidden auto-selections.

**Architecture:** Keep movement state mutations in the Rust engine and expose scriptable wrappers through `EffectContext`. Reuse existing pending-selection and action ranges wherever possible, and update DSL lowering only after the engine primitive has behavioral coverage.

**Tech Stack:** Rust `digimon-engine`, Rust `digimon-dsl`, Cargo integration tests, Markdown trackers.

---

## Scope Note

This plan changes shared zone mutation, stack mutation, security, and DSL step lowering surfaces. Do not run it in parallel with group 5 Option/Delay/Link state, group 6 modifier/immunity work, action-space resizing, tensor layout changes, or selection state-machine refactors.

Group 4 depends on group 2 selection primitives and group 3 replacement plumbing. If a task discovers a new player-visible choice, route it through `PendingSelection` and action masks first. Do not add auto-targeting, no-op card effects, or raw-Rust escape hatches as the final state.

Current branch note: several group 4 surfaces already exist (`add_pending_security_to_hand`, `play_pending_security`, `return_to_hand`, `return_to_deck`, `place_on_security`, `place_as_bottom_source`, `play_from_security`, `play_from_materials`, `effect_initiated_digivolve`). These tasks are written evidence-first: add the missing regression or contract test, run it, and only implement code if the test fails.

## File Structure

Likely engine files:

- Modify: `code/digimon-engine/src/effect_context/mod.rs` - core non-selection movement helpers and public script API.
- Modify: `code/digimon-engine/src/effect_context/selections.rs` - security-check pending helpers and any selection-backed movement helper.
- Modify: `code/digimon-engine/src/game.rs` - game-level pending state, security resolution, owner lookup, and event dispatch integration.
- Modify: `code/digimon-engine/src/game_actions.rs` - zone mutation implementation, source disposition, play/digivolve helpers, and security stack helpers.
- Modify: `code/digimon-engine/src/player.rs` - zone vectors and helper accessors if shared movement code needs owner-safe operations.
- Modify: `code/digimon-engine/src/permanent.rs` - stack push/pop/reorder helpers if direct vector mutation is repeated.
- Modify: `code/digimon-engine/src/card_source.rs` - `CardSourceRef` or handle utilities if a movement source variant is missing.
- Modify: `code/digimon-engine/src/enums.rs` - only if a new `CardSourceRef`, `StackPosition`, or movement cause enum is required.
- Modify: `code/digimon-engine/src/selection.rs` - only if a new player-visible selection kind is genuinely needed.
- Modify: `code/digimon-engine/src/action/space.rs` and `code/digimon-engine/src/action/decoder.rs` - only if existing selection/action ranges cannot represent a new choice.

Likely DSL files:

- Modify: `code/digimon-dsl/src/step.rs` - YAML step args and deserialization names.
- Modify: `code/digimon-dsl/src/compile.rs` - `StepSpec` to `CompiledStep` mapping.
- Modify: `code/digimon-dsl/src/compiled.rs` - compiled step variants if absent.
- Modify: `code/digimon-dsl/src/validator.rs` - DSL validation for new movement step fields.
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs` - hand/trash/reveal/deck/security movement lowering.
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` - play/digivolve from hand, trash, security, and materials.
- Modify: `code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs` - return, de-digivolve, and permanent-target stack changes.
- Modify: `code/digimon-engine/src/dsl_cards/step/draw.rs` - security trash/recover/draw-adjacent helpers.
- Modify: `code/digimon-engine/src/dsl_cards/binding_ref.rs` and `code/digimon-engine/src/dsl_cards/bindings.rs` - only if a new card-source binding form is required.

Likely tests:

- Modify: `code/digimon-engine/tests/zone_manipulation.rs`
- Modify: `code/digimon-engine/tests/effect_context/main.rs`
- Create: `code/digimon-engine/tests/effect_context/pending_security_to_hand.rs`
- Create: `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`
- Create: `code/digimon-engine/tests/effect_context/source_stack_operations.rs`
- Create: `code/digimon-engine/tests/effect_context/breeding_zone_movement.rs`
- Create: `code/digimon-engine/tests/effect_context/security_stack_operations.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`
- Create: `code/digimon-engine/tests/dsl/group4_zone_movement.rs`
- Modify: `code/digimon-engine/tests/mask_and_tensor/main.rs`
- Create: `code/digimon-engine/tests/mask_and_tensor/group4_selection_masks.rs`

Tracker and contract docs:

- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `docs/ACTION_SPEC.md` only if action ranges change.
- Modify: `docs/TENSOR_SPEC.md` only if tensor shape or selection encoding changes.
- Modify: `code/digimon-engine-py/src/lib.rs`, `code/digimon_gym/digimon_gym.py`, and frontend constants only if `ACTION_SPACE_SIZE` or exposed runner constants change.

## Task 1: Baseline Current Group 4 Coverage

**Files:**
- Modify: `docs/superpowers/plans/2026-04-29-gap-group-4-zone-movement.md`
- Inspect: `code/digimon-engine/tests/zone_manipulation.rs`
- Inspect: `code/digimon-engine/tests/effect_context/main.rs`
- Inspect: `code/digimon-engine/tests/dsl/main.rs`
- Inspect: `docs/RUST_ENGINE_GAPS.md`
- Inspect: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Run existing group-adjacent tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test zone_manipulation
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- play_from_hand play_from_trash play_from_security play_from_materials place_under trash_top_source trash_card_source
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- breeding_permanent source_multi
```

Expected: PASS, or FAIL only in a slice named by the failure. Record exact failing test names in this plan before changing implementation.

- [ ] **Step 2: Confirm action and tensor contract baseline**

Run:

```bash
Select-String -Path docs/ACTION_SPEC.md -Pattern "ACTION_SPACE_SIZE|2168|Selection Primitive Reuse|Mask size"
Select-String -Path docs/TENSOR_SPEC.md -Pattern "TENSOR_SIZE|1375|SelectSecurity|SelectSource|SelectBreeding"
```

Expected: output shows `2168`, `1375`, and existing selection reuse language. Group 4 should keep these unchanged unless a later task proves an unavoidable new action range.

- [ ] **Step 3: Update this plan's current-state note if needed**

If Step 1 shows a primitive already covered by passing tests, add a short line under the relevant task:

```markdown
Current evidence: `cargo test ... -- <test-name>` passes before implementation. Keep task focused on DSL/tracker closure.
```

- [ ] **Step 4: Commit baseline note if changed**

```bash
git add docs/superpowers/plans/2026-04-29-gap-group-4-zone-movement.md
git commit -m "docs: record group 4 zone movement baseline"
```

## Task 2: Pending Security Option to Hand

**Files:**
- Create: `code/digimon-engine/tests/effect_context/pending_security_to_hand.rs`
- Modify: `code/digimon-engine/tests/effect_context/main.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- Modify: `code/digimon-engine/tests/dsl/group4_zone_movement.rs`

- [ ] **Step 1: Write the failing engine test**

Add `mod pending_security_to_hand;` to `code/digimon-engine/tests/effect_context/main.rs`.

Create `code/digimon-engine/tests/effect_context/pending_security_to_hand.rs`:

```rust
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind};

fn option_card(card_id: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: vec![],
        evo_costs: vec![],
        dna_costs: vec![],
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: "[Security] Add this card to its owner's hand.".to_string(),
        keywords: vec![],
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn add_pending_security_to_hand_consumes_security_card_without_trashing_it() {
    let mut runner = DebugRunner::builder()
        .add_card(option_card("SEC-HAND"))
        .security(0, &["SEC-HAND"])
        .start();

    runner.game_mut().begin_security_check_for_test(0);

    {
        let card = runner.game().pending_security.as_ref().unwrap().card.handle();
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            runner.game_mut(),
            card,
            None,
            0,
        );
        assert!(ctx.add_pending_security_to_hand());
    }

    runner.game_mut().finish_security_check_for_test();

    assert_eq!(runner.hand_size(0), 1);
    assert_eq!(runner.trash_size(0), 0);
    assert!(runner.game().pending_security.is_none());
}

#[test]
fn add_pending_security_to_hand_noops_after_security_card_was_played() {
    let mut runner = DebugRunner::builder()
        .add_card(option_card("SEC-PLAYED"))
        .security(0, &["SEC-PLAYED"])
        .start();

    runner.game_mut().begin_security_check_for_test(0);
    runner.game_mut().pending_security.as_mut().unwrap().played = true;

    let card = runner.game().pending_security.as_ref().unwrap().card.handle();
    let mut ctx =
        digimon_engine::effect_context::EffectContext::new(runner.game_mut(), card, None, 0);

    assert!(!ctx.add_pending_security_to_hand());
    assert!(runner.game().pending_security.is_some());
    assert_eq!(runner.hand_size(0), 0);
}
```

If `begin_security_check_for_test` and `finish_security_check_for_test` do not exist, implement those as `#[cfg(test)]` helpers on `Game` rather than reaching into private security-resolution internals from tests.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- pending_security_to_hand --nocapture
```

Expected before implementation: FAIL with missing helper methods or incorrect pending-security movement behavior.

- [ ] **Step 3: Write minimal implementation**

Implement or verify this public `EffectContext` method in `code/digimon-engine/src/effect_context/mod.rs`:

```rust
pub fn add_pending_security_to_hand(&mut self) -> bool {
    let Some(pending) = self.game.pending_security.take() else {
        return false;
    };

    if pending.played {
        self.game.pending_security = Some(pending);
        return false;
    }

    let defender = pending.defender;
    self.game.player_mut(defender).hand.push(pending.card);
    true
}
```

In the DSL, ensure `add_this_option_to_hand: {}` lowers to `CompiledStep::AddThisOptionToHand` and `zone_moves.rs` calls:

```rust
CompiledStep::AddThisOptionToHand => {
    ctx.add_pending_security_to_hand();
    true
}
```

- [ ] **Step 4: Add DSL lowering regression**

Append to `code/digimon-engine/tests/dsl/group4_zone_movement.rs` and register the module from `code/digimon-engine/tests/dsl/main.rs`:

```rust
use digimon_dsl::compile::compile_card;
use digimon_dsl::step::StepSpec;

#[test]
fn add_this_option_to_hand_parses_and_compiles() {
    let yaml = r#"
card: DSL-G4-SEC-HAND
name: Security Hand
kind: option
color: [red]
cost: 3
effects:
  - when: on_security
    process:
      - add_this_option_to_hand: {}
"#;
    let spec: digimon_dsl::card::CardSpec = serde_yml::from_str(yaml).unwrap();
    assert!(matches!(
        spec.effects[0].process[0],
        StepSpec::AddThisOptionToHand(_)
    ));
    let compiled = compile_card(&spec).unwrap();
    assert_eq!(compiled.effects.len(), 1);
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- pending_security_to_hand --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- add_this_option_to_hand_parses_and_compiles --nocapture
```

Expected: PASS.

- [ ] **Step 6: Update trackers**

In `docs/RUST_ENGINE_GAPS.md`, add or update a group 4 note under the security movement entry:

```markdown
- Updated YYYY-MM-DD (Group 4): Pending security cards can be moved to hand by `EffectContext::add_pending_security_to_hand`; the helper consumes `Game::pending_security` so the card is not also trashed after the security check. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- pending_security_to_hand` and DSL parse coverage in `group4_zone_movement::add_this_option_to_hand_parses_and_compiles`.
```

In `qa/dsl-vocab-gaps.md`, remove any claim that `add_this_option_to_hand` lacks lowering. If no entry exists, do not add noise.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/tests/effect_context/main.rs code/digimon-engine/tests/effect_context/pending_security_to_hand.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-dsl/src/step.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/step/zone_moves.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/group4_zone_movement.rs docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
git commit -m "feat: support pending security card to hand"
```

## Task 3: Effect-Initiated Digivolve From Trash, Security, and Materials

**Files:**
- Create: `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`
- Modify: `code/digimon-engine/tests/effect_context/main.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/tests/dsl/group4_zone_movement.rs`

- [ ] **Step 1: Write failing engine tests**

Add `mod effect_digivolve_from_zones;` to `code/digimon-engine/tests/effect_context/main.rs`.

Create `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`:

```rust
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, CostDelta, PlaySource};
use digimon_engine::permanent::{Permanent, PermanentHandle};

fn digimon(card_id: &str, level: u8, color: CardColor, evo_from: Option<(CardColor, u8, u8)>) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(level),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![color],
        traits: vec![],
        evo_costs: evo_from
            .map(|(card_color, from_level, memory_cost)| EvoCost {
                card_color: card_color as u8,
                level: from_level,
                memory_cost,
            })
            .into_iter()
            .collect(),
        dna_costs: vec![],
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: vec![],
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

fn seed_base(runner: &mut DebugRunner) -> PermanentHandle {
    let game = runner.game_mut();
    let data_idx = game.card_data.iter().position(|c| c.card_id == "BASE3").unwrap();
    let card = digimon_engine::card_source::CardSource::new(data_idx, 0, game.next_card_index());
    game.player_mut(0).battle_area.push(Permanent::new(card, game.turn_count));
    PermanentHandle { player: 0, index: 0 }
}

#[test]
fn effect_digivolve_from_trash_moves_card_to_top_of_stack() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("BASE3", 3, CardColor::Red, None))
        .add_card(digimon("EVO4", 4, CardColor::Red, Some((CardColor::Red, 3, 2))))
        .trash(0, &["EVO4"])
        .memory(0)
        .start();
    let target = seed_base(&mut runner);

    let ok = runner.game_mut().effect_initiated_digivolve_from_source(
        0,
        CardSourceRef::Trash(0, 0),
        target,
        CostDelta::Free,
        false,
        PlaySource::ByEffect,
    );

    assert!(ok);
    assert_eq!(runner.trash_size(0), 0);
    assert_eq!(runner.game().player(0).battle_area[0].card_sources.len(), 2);
    assert_eq!(
        runner.game().player(0).battle_area[0].top_card().card_id(&runner.game().card_data),
        "EVO4"
    );
}

#[test]
fn effect_digivolve_from_security_removes_exact_security_card() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("BASE3", 3, CardColor::Red, None))
        .add_card(digimon("EVO4", 4, CardColor::Red, Some((CardColor::Red, 3, 2))))
        .security(0, &["EVO4"])
        .memory(0)
        .start();
    let target = seed_base(&mut runner);

    let ok = runner.game_mut().effect_initiated_digivolve_from_source(
        0,
        CardSourceRef::Security(0, 0),
        target,
        CostDelta::Free,
        false,
        PlaySource::ByEffect,
    );

    assert!(ok);
    assert_eq!(runner.security_count(0), 0);
    assert_eq!(
        runner.game().player(0).battle_area[0].top_card().card_id(&runner.game().card_data),
        "EVO4"
    );
}
```

If `CardSourceRef::Security` or `effect_initiated_digivolve_from_source` does not exist, this is the intended failing surface.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_digivolve_from_zones --nocapture
```

Expected before implementation: FAIL with missing `CardSourceRef::Security` or missing source-general digivolve helper, or with behavior that only accepts hand indices.

- [ ] **Step 3: Write minimal implementation**

Add a source-general helper in `Game` and expose it through `EffectContext`:

```rust
pub fn effect_initiated_digivolve_from_source(
    &mut self,
    player: PlayerId,
    source: CardSourceRef,
    target: PermanentHandle,
    cost: CostDelta,
    ignore_color: bool,
    play_source: PlaySource,
) -> bool {
    let Some(card) = self.take_card_source(source) else {
        return false;
    };
    if !self.can_digivolve_card_onto(player, &card, target, ignore_color) {
        self.restore_card_source(source, card);
        return false;
    }
    self.pay_digivolve_cost(player, &card, target, cost);
    self.stack_card_on_permanent(target, card, play_source)
}
```

Implement `take_card_source` / `restore_card_source` in `game_actions.rs` as private helpers that support `Hand`, `Trash`, `Security`, `Material`, `Reveal`, and `DeckTop` without losing owner identity. Do not duplicate vector-removal logic in every public method.

- [ ] **Step 4: Add DSL tests for zone-sensitive digivolve**

Append to `code/digimon-engine/tests/dsl/group4_zone_movement.rs`:

```rust
use digimon_dsl::compile::compile_card;
use digimon_dsl::compiled::CompiledStep;

#[test]
fn effect_initiated_digivolve_can_name_trash_source_binding() {
    let yaml = r#"
card: DSL-G4-DIGI-TRASH
name: Digivolve From Trash
kind: option
color: [red]
cost: 3
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          bind_as: evo
          filter: { level: 4 }
          prompt: "Choose a level 4 in trash"
      - select_own_permanent:
          bind_as: base
          filter: { level: 3 }
          prompt: "Choose a level 3"
      - effect_initiated_digivolve:
          target: base
          card: { binding: evo, zone: trash }
          cost: free
"#;
    let spec: digimon_dsl::card::CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = compile_card(&spec).unwrap();
    assert!(compiled.effects[0]
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::EffectInitiatedDigivolve { .. })));
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_digivolve_from_zones --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- effect_initiated_digivolve_can_name_trash_source_binding --nocapture
```

Expected: PASS.

- [ ] **Step 6: Contract review**

If the helper reuses existing selection actions (`SelectTrash`, `SelectSecurity`, `SelectSource`, `SelectMaterial`), do not change `ACTION_SPACE_SIZE`. Add a short note to `docs/RUST_ENGINE_GAPS.md`:

```markdown
- Group 4 contract review: effect-initiated digivolve from non-hand zones reuses existing pending-selection action IDs and keeps `ACTION_SPACE_SIZE = 2168`; tensor layout remains `TENSOR_SIZE = 1375`.
```

If a new action range was required, update `docs/ACTION_SPEC.md`, Rust constants, PyO3 constants, RL env constants, and frontend constants in this same task before committing.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/tests/effect_context/main.rs code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/dsl_cards/step/play_digivolve.rs code/digimon-dsl/src/step.rs code/digimon-dsl/src/compile.rs code/digimon-engine/tests/dsl/group4_zone_movement.rs docs/RUST_ENGINE_GAPS.md docs/ACTION_SPEC.md docs/TENSOR_SPEC.md
git commit -m "feat: support effect digivolve from non-hand zones"
```

## Task 4: Return to Hand and Deck With Faithful Source Disposition

**Files:**
- Modify: `code/digimon-engine/tests/zone_manipulation.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `docs/RUST_ENGINE_GAPS.md`

- [ ] **Step 1: Write failing source-disposition tests**

Append to `code/digimon-engine/tests/zone_manipulation.rs`:

```rust
#[test]
fn return_to_deck_can_return_full_stack_when_include_sources_true() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 3))
        .add_card(plain_digimon("TOP", "Top", 5))
        .deck(0, &[])
        .start();

    let handle = {
        let g = r.game_mut();
        let base_idx = g.card_data.iter().position(|c| c.card_id == "BASE").unwrap();
        let top_idx = g.card_data.iter().position(|c| c.card_id == "TOP").unwrap();
        let base = digimon_engine::card_source::CardSource::new(base_idx, 0, g.next_card_index());
        let top = digimon_engine::card_source::CardSource::new(top_idx, 0, g.next_card_index());
        let mut perm = digimon_engine::permanent::Permanent::new(base, g.turn_count);
        perm.card_sources.push(top);
        g.player_mut(0).battle_area.push(perm);
        PermanentHandle { player: 0, index: 0 }
    };

    assert!(r.game_mut().return_stack_to_deck(handle, StackPosition::Top));
    assert_eq!(r.battle_area_size(0), 0);
    assert_eq!(r.trash_size(0), 0);
    assert_eq!(r.deck_size(0), 2);
    let top_two: Vec<String> = r.game().player(0).deck.iter()
        .rev()
        .take(2)
        .map(|c| c.card_id(&r.game().card_data).to_string())
        .collect();
    assert_eq!(top_two, vec!["TOP".to_string(), "BASE".to_string()]);
}

#[test]
fn return_to_hand_still_trashes_sources_by_default() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 3))
        .add_card(plain_digimon("TOP", "Top", 5))
        .start();

    let handle = seed_two_card_stack(&mut r, "BASE", "TOP");

    assert!(r.game_mut().return_to_hand(handle).is_some());
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.trash_size(0), 1);
}
```

Add `seed_two_card_stack` near the existing helper functions in the same file:

```rust
fn seed_two_card_stack(r: &mut DebugRunner, bottom_id: &str, top_id: &str) -> PermanentHandle {
    let g = r.game_mut();
    let bottom_idx = g.card_data.iter().position(|c| c.card_id == bottom_id).unwrap();
    let top_idx = g.card_data.iter().position(|c| c.card_id == top_id).unwrap();
    let bottom = digimon_engine::card_source::CardSource::new(bottom_idx, 0, g.next_card_index());
    let top = digimon_engine::card_source::CardSource::new(top_idx, 0, g.next_card_index());
    let mut perm = digimon_engine::permanent::Permanent::new(bottom, g.turn_count);
    perm.card_sources.push(top);
    g.player_mut(0).battle_area.push(perm);
    PermanentHandle { player: 0, index: 0 }
}
```

- [ ] **Step 2: Run tests to verify they fail or prove already done**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test zone_manipulation -- return_to_deck_can_return_full_stack_when_include_sources_true return_to_hand_still_trashes_sources_by_default --nocapture
```

Expected before implementation: either FAIL because `return_stack_to_deck` is missing, or PASS if the branch already implemented full-stack return. If PASS, skip implementation and update trackers with evidence.

- [ ] **Step 3: Write minimal implementation**

In `game_actions.rs`, implement a separate full-stack return helper rather than changing the existing default:

```rust
pub fn return_stack_to_deck(
    &mut self,
    handle: PermanentHandle,
    position: StackPosition,
) -> bool {
    let Some(mut perm) = self.remove_permanent(handle) else {
        return false;
    };
    let player = handle.player;
    match position {
        StackPosition::Top => {
            for card in perm.card_sources.drain(..) {
                self.player_mut(player).deck.push(card);
            }
        }
        StackPosition::Bottom => {
            for card in perm.card_sources.drain(..).rev() {
                self.player_mut(player).deck.insert(0, card);
            }
        }
        StackPosition::Random => {
            for card in perm.card_sources.drain(..) {
                self.insert_random_into_deck(player, card);
            }
        }
    }
    true
}
```

In `permanent_mutations.rs`, honor `include_sources`:

```rust
CompiledStep::ReturnToDeck { target, position, include_sources } => {
    if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings) {
        if *include_sources {
            let _ = ctx.return_stack_to_deck(h, super::map_stack_position(*position));
        } else {
            let _ = ctx.return_to_deck(h, super::map_stack_position(*position));
        }
    }
    true
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test zone_manipulation -- return_to_deck_can_return_full_stack_when_include_sources_true return_to_hand_still_trashes_sources_by_default --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group4_zone_movement --nocapture
```

Expected: PASS.

- [ ] **Step 5: Update trackers**

In `docs/RUST_ENGINE_GAPS.md`, update the return-to-hand/deck row:

```markdown
- Updated YYYY-MM-DD (Group 4): `return_to_hand` preserves the top card and trashes lower sources by default; `return_stack_to_deck(..., include_sources=true)` returns the whole stack in order. Covered by `zone_manipulation::return_to_deck_can_return_full_stack_when_include_sources_true` and `zone_manipulation::return_to_hand_still_trashes_sources_by_default`.
```

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/tests/zone_manipulation.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs code/digimon-dsl/src/step.rs code/digimon-dsl/src/compile.rs docs/RUST_ENGINE_GAPS.md
git commit -m "feat: support full-stack return to deck"
```

## Task 5: Bottom Source Placement and Source Extraction

**Files:**
- Create: `code/digimon-engine/tests/effect_context/source_stack_operations.rs`
- Modify: `code/digimon-engine/tests/effect_context/main.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/permanent.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compile.rs`

- [ ] **Step 1: Write failing stack-operation tests**

Add `mod source_stack_operations;` to `code/digimon-engine/tests/effect_context/main.rs`.

Create `code/digimon-engine/tests/effect_context/source_stack_operations.rs`:

```rust
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::CardSourceRef;

#[test]
fn place_as_bottom_source_from_trash_preserves_top_card() {
    let mut runner = DebugRunner::builder()
        .add_card(super::plain_digimon("BASE", "Base", 3))
        .add_card(super::plain_digimon("FUEL", "Fuel", 3))
        .trash(0, &["FUEL"])
        .start();

    let target = super::seed_single_card_permanent_with_id(&mut runner, "BASE");

    assert!(runner.game_mut().place_as_bottom_source(CardSourceRef::Trash(0, 0), target));
    assert_eq!(runner.trash_size(0), 0);
    let ids: Vec<String> = runner.game().player(0).battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game().card_data).to_string())
        .collect();
    assert_eq!(ids, vec!["FUEL".to_string(), "BASE".to_string()]);
}

#[test]
fn extract_source_to_trash_removes_exact_source_index() {
    let mut runner = DebugRunner::builder()
        .add_card(super::plain_digimon("BOTTOM", "Bottom", 2))
        .add_card(super::plain_digimon("MID", "Mid", 3))
        .add_card(super::plain_digimon("TOP", "Top", 4))
        .start();

    let target = super::seed_three_card_stack(&mut runner, "BOTTOM", "MID", "TOP");
    assert!(runner.game_mut().trash_source_by_index(target, 1));

    let ids: Vec<String> = runner.game().player(0).battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game().card_data).to_string())
        .collect();
    assert_eq!(ids, vec!["BOTTOM".to_string(), "TOP".to_string()]);
    assert_eq!(runner.trash_size(0), 1);
}
```

If shared helpers are not available from another test module, define local `plain_digimon`, `seed_single_card_permanent_with_id`, and `seed_three_card_stack` in this file. Keep the test self-contained.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- source_stack_operations --nocapture
```

Expected before implementation: FAIL with missing `trash_source_by_index` or incorrect stack mutation.

- [ ] **Step 3: Write minimal implementation**

Add a permanent helper:

```rust
impl Permanent {
    pub fn remove_source_at(&mut self, source_index: usize) -> Option<CardSource> {
        if source_index >= self.card_sources.len().saturating_sub(1) {
            return None;
        }
        Some(self.card_sources.remove(source_index))
    }
}
```

Add game/effect-context wrappers:

```rust
pub fn trash_source_by_index(&mut self, target: PermanentHandle, source_index: usize) -> bool {
    let Some(card) = self
        .player_mut(target.player)
        .battle_area
        .get_mut(target.index as usize)
        .and_then(|perm| perm.remove_source_at(source_index))
    else {
        return false;
    };
    self.player_mut(target.player).trash.push(card);
    self.dispatch_on_digivolution_card_trashed(target, source_index);
    true
}
```

Use existing event-context machinery when dispatching `OnDigivolutionCardTrashed`; do not rescan trash to infer which source moved.

- [ ] **Step 4: Add DSL lowering test**

Append to `code/digimon-engine/tests/dsl/group4_zone_movement.rs`:

```rust
#[test]
fn trash_top_source_and_place_as_bottom_source_compile() {
    let yaml = r#"
card: DSL-G4-SOURCE
name: Source Ops
kind: digimon
level: 4
color: [red]
cost: 4
dp: 5000
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: target
          filter: { level: 4 }
          prompt: "Choose target"
      - trash_top_source:
          target: target
      - select_trash:
          of: you
          bind_as: fuel
          filter: { kind: digimon }
          prompt: "Choose source"
      - place_as_bottom_source:
          source: { binding: fuel, zone: trash }
          target: target
"#;
    let spec: digimon_dsl::card::CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = digimon_dsl::compile::compile_card(&spec).unwrap();
    assert_eq!(compiled.effects[0].process.len(), 4);
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- source_stack_operations --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- trash_top_source_and_place_as_bottom_source_compile --nocapture
```

Expected: PASS.

- [ ] **Step 6: Update trackers**

Update `docs/RUST_ENGINE_GAPS.md` stack-position entry:

```markdown
- Updated YYYY-MM-DD (Group 4): source insertion and extraction are covered by `place_as_bottom_source` and exact-index source trashing; source-trash dispatch carries the exact moved source. Tests: `effect_context::source_stack_operations`.
```

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/tests/effect_context/main.rs code/digimon-engine/tests/effect_context/source_stack_operations.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/permanent.rs code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs code/digimon-engine/src/dsl_cards/step/zone_moves.rs code/digimon-dsl/src/step.rs code/digimon-dsl/src/compile.rs code/digimon-engine/tests/dsl/group4_zone_movement.rs docs/RUST_ENGINE_GAPS.md
git commit -m "feat: complete source stack movement helpers"
```

## Task 6: Effect-Initiated Movement From and To Breeding

**Files:**
- Create: `code/digimon-engine/tests/effect_context/breeding_zone_movement.rs`
- Modify: `code/digimon-engine/tests/effect_context/main.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/player.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs`
- Modify: `code/digimon-engine/tests/mask_and_tensor/group4_selection_masks.rs`

- [ ] **Step 1: Write failing breeding movement tests**

Add `mod breeding_zone_movement;` to `code/digimon-engine/tests/effect_context/main.rs`.

Create `code/digimon-engine/tests/effect_context/breeding_zone_movement.rs`:

```rust
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::permanent::PermanentHandle;

#[test]
fn effect_move_from_breeding_moves_real_breeding_permanent_to_battle() {
    let mut runner = DebugRunner::builder()
        .add_card(super::plain_digimon("EGG3", "Egg3", 3))
        .digitama(0, &["EGG3"])
        .start();

    assert!(runner.game_mut().hatch(0));
    assert_eq!(runner.battle_area_size(0), 0);

    let moved = runner.game_mut().move_from_breeding_by_effect(0);

    assert_eq!(moved, Some(PermanentHandle { player: 0, index: 0 }));
    assert!(runner.game().player(0).breeding_area.is_none());
    assert_eq!(runner.battle_area_size(0), 1);
}

#[test]
fn play_to_empty_breeding_slot_does_not_use_battle_area_capacity() {
    let mut runner = DebugRunner::builder()
        .add_card(super::plain_digimon("BABY", "Baby", 2))
        .hand(0, &["BABY"])
        .start();

    assert!(runner.game_mut().play_to_breeding_from_hand(0, 0));
    assert!(runner.game().player(0).breeding_area.is_some());
    assert_eq!(runner.hand_size(0), 0);
    assert_eq!(runner.battle_area_size(0), 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- breeding_zone_movement --nocapture
```

Expected before implementation: FAIL with missing effect movement helpers or an implementation that uses fake battle-area handles incorrectly.

- [ ] **Step 3: Write minimal implementation**

In `game_actions.rs`:

```rust
pub fn move_from_breeding_by_effect(&mut self, player: PlayerId) -> Option<PermanentHandle> {
    let permanent = self.player_mut(player).breeding_area.take()?;
    let index = self.player(player).battle_area.len();
    if index >= self.rules.field_slots as usize {
        self.player_mut(player).breeding_area = Some(permanent);
        return None;
    }
    self.player_mut(player).battle_area.push(permanent);
    let handle = PermanentHandle { player, index: index as u8 };
    self.enqueue_move_observers(handle);
    Some(handle)
}

pub fn play_to_breeding_from_hand(&mut self, player: PlayerId, hand_index: usize) -> bool {
    if self.player(player).breeding_area.is_some() {
        return false;
    }
    let Some(card) = self.take_card_source(CardSourceRef::Hand(player, hand_index)) else {
        return false;
    };
    self.player_mut(player).breeding_area = Some(Permanent::new(card, self.turn_count));
    true
}
```

Use `PermanentHandle { index: BREEDING_TARGET as u8 }` only for selection references and effect source identity. Do not store breeding permanents in the battle-area vector under a virtual slot.

- [ ] **Step 4: Add mask/action regression if any new prompt is added**

If this task adds or changes player-visible breeding selection, create `code/digimon-engine/tests/mask_and_tensor/group4_selection_masks.rs`:

```rust
use digimon_engine::action::space::PASS;
use digimon_engine::selection::SelectionKind;

#[test]
fn breeding_selection_mask_only_exposes_breeding_action_ids() {
    let mut runner = digimon_engine::debug_runner::DebugRunner::builder()
        .dsl_card("BT20-083")
        .start();
    runner.install_test_breeding_selection(0, "Choose breeding");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::BreedingPermanent));
    let mask = runner.game().get_action_mask(0);
    assert_eq!(mask[14], 1.0);
    assert_eq!(mask[15], 0.0);
    assert_eq!(mask[PASS as usize], 0.0);
}
```

Expected: PASS without changing `ACTION_SPACE_SIZE`.

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- breeding_zone_movement --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- breeding_selection_mask_only_exposes_breeding_action_ids --nocapture
```

Expected: PASS, or skip the second command if no selection/mask code changed.

- [ ] **Step 6: Update trackers**

Update `qa/dsl-vocab-gaps.md` entry `G-BREEDING-PERMANENT-SELECTION` only if the task actually closes the DSL lowering. Otherwise leave it open and add an engine-only note.

Add to `docs/RUST_ENGINE_GAPS.md`:

```markdown
- Updated YYYY-MM-DD (Group 4): effect movement from breeding and play-to-empty-breeding are real breeding-zone operations, not fake battle-area placements. Tests: `effect_context::breeding_zone_movement`.
```

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/tests/effect_context/main.rs code/digimon-engine/tests/effect_context/breeding_zone_movement.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/player.rs code/digimon-engine/src/dsl_cards/step/play_digivolve.rs code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs code/digimon-engine/tests/mask_and_tensor/main.rs code/digimon-engine/tests/mask_and_tensor/group4_selection_masks.rs docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
git commit -m "feat: support effect movement through breeding"
```

## Task 7: Security Stack Search, Place, Trash, Recover, and Shuffle Helpers

**Files:**
- Create: `code/digimon-engine/tests/effect_context/security_stack_operations.rs`
- Modify: `code/digimon-engine/tests/effect_context/main.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/effect_context/selections.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/action/decoder.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/draw.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compile.rs`

- [ ] **Step 1: Write failing security stack tests**

Add `mod security_stack_operations;` to `code/digimon-engine/tests/effect_context/main.rs`.

Create `code/digimon-engine/tests/effect_context/security_stack_operations.rs`:

```rust
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardSourceRef, StackPosition};

#[test]
fn select_security_then_add_to_hand_moves_exact_card() {
    let mut runner = DebugRunner::builder()
        .add_card(super::plain_digimon("TOP", "Top", 3))
        .add_card(super::plain_digimon("BOTTOM", "Bottom", 3))
        .security(0, &["BOTTOM", "TOP"])
        .start();

    runner.install_test_security_selection(0, 0, "Choose security");
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(view.valid_action_ids.len(), 2);

    runner.execute_action(view.valid_action_ids[0]);
    runner.auto_resolve();

    assert_eq!(runner.security_count(0), 1);
    assert_eq!(runner.hand_size(0), 1);
}

#[test]
fn trash_top_security_fires_loss_observers_once() {
    let mut runner = DebugRunner::builder()
        .add_card(super::plain_digimon("SHIELD", "Shield", 3))
        .security(0, &["SHIELD"])
        .start();

    let checkpoint = runner.event_checkpoint();
    assert!(runner.game_mut().trash_top_security(0));

    assert_eq!(runner.security_count(0), 0);
    assert_eq!(runner.trash_size(0), 1);
    let events = runner.events_since(checkpoint);
    assert_eq!(
        events.iter().filter(|event| event.kind_name() == "OnLoseSecurity").count(),
        1
    );
}

#[test]
fn place_on_security_bottom_and_recover_from_trash_preserve_owner() {
    let mut runner = DebugRunner::builder()
        .add_card(super::plain_digimon("RECOVER", "Recover", 3))
        .trash(0, &["RECOVER"])
        .start();

    assert!(runner.game_mut().place_on_security(
        0,
        CardSourceRef::Trash(0, 0),
        StackPosition::Bottom,
        false,
    ));
    assert_eq!(runner.trash_size(0), 0);
    assert_eq!(runner.security_count(0), 1);
}
```

If `install_test_security_selection`, `event_checkpoint`, `events_since`, or `kind_name` do not exist, implement test helpers in `DebugRunner` rather than weakening the assertions.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- security_stack_operations --nocapture
```

Expected before implementation: FAIL with missing selection/test helpers or incomplete security movement/event behavior.

- [ ] **Step 3: Write minimal implementation**

Use existing `SelectSecurity` action indices (`40-49` own security, `50-59` opponent security) from `docs/ACTION_SPEC.md`. Add no new action range unless a test proves the range cannot represent the prompt.

Add selection helper if missing:

```rust
pub fn select_security<F, C>(
    &mut self,
    of_player: PlayerId,
    prompt: &str,
    optional: bool,
    filter: F,
    callback: C,
)
where
    F: Fn(&Game, &CardSource) -> bool + Send + Sync + 'static,
    C: FnOnce(&mut EffectContext<'_>, CardHandle) + Send + Sync + 'static,
{
    self.install_security_selection(of_player, self.player, prompt, optional, filter, callback);
}
```

Ensure movement methods:

```rust
pub fn trash_top_security(&mut self, player: PlayerId) -> bool;
pub fn place_on_security(&mut self, player: PlayerId, source: CardSourceRef, position: StackPosition, face_up: bool) -> bool;
pub fn add_security_to_hand(&mut self, player: PlayerId, handle: CardHandle) -> bool;
pub fn shuffle_security(&mut self, player: PlayerId);
```

Security trash must fire loss/removal observers once. Security placement must not fire loss observers.

- [ ] **Step 4: Add DSL lowering coverage**

Append to `code/digimon-engine/tests/dsl/group4_zone_movement.rs`:

```rust
#[test]
fn security_stack_steps_parse_and_compile() {
    let yaml = r#"
card: DSL-G4-SECURITY
name: Security Ops
kind: option
color: [yellow]
cost: 3
effects:
  - when: on_play
    process:
      - select_security:
          of: you
          bind_as: picked
          filter: { kind: digimon }
          prompt: "Choose security"
      - add_to_hand_from_security:
          of: you
          card: picked
      - trash_top_security:
          of: opponent
      - place_on_security:
          of: you
          source: { binding: picked, zone: security }
          position: bottom
"#;
    let spec: digimon_dsl::card::CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = digimon_dsl::compile::compile_card(&spec).unwrap();
    assert_eq!(compiled.effects[0].process.len(), 4);
}
```

If `add_to_hand_from_security` is not accepted by the DSL, add `StepSpec::AddToHandFromSecurity` with args `{ of, card }`, compile it to `CompiledStep::AddToHandFromSecurity`, and lower it in `zone_moves.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- security_stack_operations --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- security_stack_steps_parse_and_compile --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- security --nocapture
```

Expected: PASS.

- [ ] **Step 6: Update trackers**

Update the security stack operation entry in `docs/RUST_ENGINE_GAPS.md`:

```markdown
- Updated YYYY-MM-DD (Group 4): security stack search, exact-card move to hand, top trash, bottom placement/recover, and shuffle helpers are scriptable through `EffectContext` and DSL lowering. SelectSecurity reuses existing action indices, so `ACTION_SPACE_SIZE` remains 2168. Tests: `effect_context::security_stack_operations`, `dsl::security_stack_steps_parse_and_compile`.
```

Update `qa/archetype-qa/engine-gaps.md` for any archetype rows blocked only by security stack movement.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/tests/effect_context/main.rs code/digimon-engine/tests/effect_context/security_stack_operations.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/effect_context/selections.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/selection.rs code/digimon-engine/src/action/decoder.rs code/digimon-engine/src/dsl_cards/step/draw.rs code/digimon-engine/src/dsl_cards/step/zone_moves.rs code/digimon-dsl/src/step.rs code/digimon-dsl/src/compile.rs code/digimon-engine/tests/dsl/group4_zone_movement.rs docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md
git commit -m "feat: complete security stack movement helpers"
```

## Task 8: Group 4 DSL and Real-Card Smoke Coverage

**Files:**
- Modify: `code/digimon-engine/tests/dsl/group4_zone_movement.rs`
- Modify: `code/digimon-engine/tests/cards_behavioral/main.rs`
- Modify: card-specific tests under `code/digimon-engine/tests/cards_behavioral/`
- Modify: production YAML under `code/digimon-engine/cards/` only for cards that become unblocked by this group
- Modify: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Write real-card smoke tests for unblocked patterns**

Add one real-card test per newly closed pattern. Use cards already called out by the roadmap and DSL test API:

```rust
// code/digimon-engine/tests/cards_behavioral/ex6/ex6_072.rs
//! EX6-072 Mega Digimon Assembly! - Option.
//!
//! # Card text
//! Use printed text from data/cards.json.
//!
//! # Pattern
//! - C5 Mega Digimon Assembly: security trash-to-hand selection.

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex6_072_security_can_add_itself_to_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX6-072")
        .security(0, &["EX6-072"])
        .start();

    runner.attack_player(1, 0, false);
    runner.auto_resolve();

    assert_eq!(runner.hand_size(0), 1);
    assert_eq!(runner.trash_size(0), 0);
}
```

Use the same pattern for:

- `EX11-005` Yaamon or `BT24-070` Growlmon for stack shift / digivolve from trash.
- `BT13-075` Alphamon for trash-to-bottom-source.
- `BT20-083` Omekamon for breeding King Drasil bottom-source placement.

- [ ] **Step 2: Run tests to verify they fail**

Run each new card test individually:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex6_072_security_can_add_itself_to_hand --nocapture
```

Expected before YAML/card updates: FAIL because the card is not authored, is ignored behind a gap, or uses missing DSL vocabulary.

- [ ] **Step 3: Write minimal YAML/card updates**

For each card that is now unblocked, update its production YAML under `code/digimon-engine/cards/<set>/<CARD-ID>.yaml`. Example shape for security-to-hand:

```yaml
card: EX6-072
name: Mega Digimon Assembly!
kind: option
color: [white]
cost: 6
effects:
  - when: on_security
    process:
      - add_this_option_to_hand: {}
```

Do not add raw-rust callbacks. If a card still needs group 5 or group 7 vocabulary, leave the real-card test ignored with a precise tracker gap:

```rust
#[ignore = "pending: G-OPTION-DELAY-LINK-STATE from qa/dsl-vocab-gaps.md"]
```

- [ ] **Step 4: Run tests to verify they pass or are precisely ignored**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex6_072 bt13_075 bt20_083 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group4_zone_movement --nocapture
```

Expected: PASS for group 4-only cards. Any `#[ignore]` must name an open non-group-4 gap.

- [ ] **Step 5: Update DSL tracker**

In `qa/dsl-vocab-gaps.md`, close only entries whose YAML vocabulary and lowering now pass tests. For each closed entry, append:

```markdown
- Updated YYYY-MM-DD (Group 4): closed by `<step_name>` lowering and behavioral coverage in `<test_name>`. Remove ignores that cited this gap.
```

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/tests/dsl/group4_zone_movement.rs code/digimon-engine/tests/cards_behavioral/main.rs code/digimon-engine/tests/cards_behavioral code/digimon-engine/cards qa/dsl-vocab-gaps.md
git commit -m "test: add group 4 real card movement coverage"
```

## Task 9: Final Contract Review and Tracker Closure

**Files:**
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `docs/ACTION_SPEC.md` if action ranges changed.
- Modify: `docs/TENSOR_SPEC.md` if tensor shape changed.
- Modify: `docs/RUST_ENGINE_API.md` if new public `EffectContext` methods were added.
- Modify: `docs/RUST_DSL_TEST_API.md` if new DSL test idioms were added.

- [ ] **Step 1: Run group 4 verification commands**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test zone_manipulation
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- pending_security_to_hand effect_digivolve_from_zones source_stack_operations breeding_zone_movement security_stack_operations
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group4_zone_movement
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- breeding_permanent source_multi
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- security source breeding
```

Expected: PASS. If a command is too broad because test filtering matches unrelated tests, rerun with exact failing test names and record the exact pass command in the tracker.

- [ ] **Step 2: Run full engine check**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: PASS.

- [ ] **Step 3: Review action/tensor/PyO3/frontend contracts**

If no action or tensor constants changed, add this note to `docs/RUST_ENGINE_GAPS.md` group 4 closure:

```markdown
- Contract review: Group 4 reuses existing action ranges (`SelectHand`, `SelectTrash`, `SelectSecurity`, `SelectSource`, `SelectBreedingPermanent`, `PASS`) and keeps `ACTION_SPACE_SIZE = 2168`; no PyO3, RL env, frontend constant, or tensor layout updates were required. `TENSOR_SIZE` remains 1375.
```

If constants changed, update all of these in one commit:

- `docs/ACTION_SPEC.md`
- `docs/TENSOR_SPEC.md`
- `code/digimon-engine/src/action/space.rs`
- `code/digimon-engine/src/tensor_profiles/standard/v1.rs`
- `code/digimon-engine-py/src/lib.rs`
- `code/digimon_gym/digimon_gym.py`
- frontend action/tensor constants under `code/frontend/src/`

- [ ] **Step 4: Close or split remaining group 4 gaps**

For each group 4 tracker row, make one of these exact edits:

```markdown
- Closed YYYY-MM-DD by Group 4. Evidence: `<exact cargo test command>`.
```

or:

```markdown
- Still open after Group 4 because `<specific missing behavior>`. Split to Group `<N>` because it depends on `<dependency>`. Evidence: `<test name>` is ignored with `#[ignore = "pending: <gap-id>"]`.
```

Do this in all three trackers:

- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`

- [ ] **Step 5: Update public docs for new helpers**

If new public helpers were added, add concise entries to `docs/RUST_ENGINE_API.md`. Use this format:

```markdown
### `ctx.add_pending_security_to_hand() -> bool`

Moves the currently resolving security card into its defender's hand and consumes `Game::pending_security`, preventing the default security-check trash step. Returns `false` when no security check is active or the pending security card was already played.
```

If new DSL movement idioms were added, add them to `docs/RUST_DSL_TEST_API.md` selection or zone-move sections with one small YAML example and one expected assertion pattern.

- [ ] **Step 6: Self-review**

Before committing, search this plan and changed trackers manually for these red flags:

```text
TBD
TODO
implement later
stub
auto-select
raw-rust workaround
no-op placeholder
```

Expected: no new unresolved placeholder language except quoted anti-pattern text in this plan.

Then check:

- Every group 4 roadmap slice has a passing test command or a split-open tracker entry.
- Every new player-visible choice has action-mask coverage.
- Every moved card preserves owner identity.
- Every stack mutation has stable source disposition and does not infer moved sources by scanning trash after the fact.
- Security movement distinguishes loss/trash observers from placement/recovery.
- Breeding movement uses the real breeding slot, not a fake battle-area vector entry.

- [ ] **Step 7: Commit closure**

```bash
git add docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md docs/ACTION_SPEC.md docs/TENSOR_SPEC.md docs/RUST_ENGINE_API.md docs/RUST_DSL_TEST_API.md
git commit -m "docs: close group 4 zone movement gaps"
```

## Execution Handoff

Recommended execution order:

1. Task 1 baseline.
2. Tasks 2, 4, 5, and 7 can run independently if workers do not touch the same files at the same time. Assign disjoint write sets.
3. Task 3 should not run in parallel with Task 5 if both change shared `CardSourceRef` or `take_card_source` helpers.
4. Task 6 should not run in parallel with any selection/action mask work.
5. Task 8 waits for the engine/DSL primitives it exercises.
6. Task 9 runs last.

Use `superpowers:subagent-driven-development` for execution if multiple workers are used. Tell every worker they are not alone in the codebase and must not revert edits made by others.
