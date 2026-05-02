# Modifiers, Auras, and Keywords Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Group 6 reusable Rust engine and YAML DSL capabilities for player/permanent modifiers, query-time auras, printed/granted combat keywords, dynamic DP/Security Attack formulas, and DigiXros-scoped aliases without hidden choices or action-mask drift.

**Architecture:** Keep permanent, player, source-kind, and query-time aura semantics distinct. Engine behavior lands first with failing Rust tests; DSL schema/lowering lands only after the engine surface exists, and every player-visible legality change is proven in both masks and decoder/execution validation. This group should not change `ACTION_SPACE_SIZE` or `TENSOR_SIZE`; if an implementation discovers that a new action bit or tensor slot is required, pause and update `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md`, PyO3 bindings, RL wrappers, and frontend constants in the same slice.

**Tech Stack:** Rust `digimon-engine`, Rust `digimon-dsl`, PyO3/RL action-mask contracts, YAML card DSL, `cargo test`, docs trackers under `docs/` and `qa/`.

---

## Scope And Parallelism

Group 6 depends on earlier groups for event context, pending selections, cost/replacement plumbing, and option flow. Do not execute these slices in parallel when they touch the same surfaces:

- Tasks 1 and 2 both touch `code/digimon-engine/src/enums.rs`, `code/digimon-engine/src/modifiers.rs`, `code/digimon-engine/src/action/mask.rs`, and `code/digimon-engine/src/action/decode.rs`.
- Tasks 3 and 6 both touch `code/digimon-dsl/src/clause.rs`, `code/digimon-dsl/src/compiled.rs`, `code/digimon-dsl/src/compile.rs`, `code/digimon-engine/src/dsl_cards/lower_aura.rs`, `code/digimon-engine/src/effect.rs`, and `code/digimon-engine/src/game.rs`.
- Tasks 4 and 5 both touch combat keyword behavior in `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/game_actions.rs`, and `code/digimon-engine/src/action/mask.rs`.
- Task 7 touches DigiXros data and matching only; it can run after Tasks 1-6 or in a separate worktree if no one else is editing `card_data.rs`, `dna_digivolve.rs`, or `deck_tools.rs`.

Never close a gap in `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, or `qa/dsl-vocab-gaps.md` until the slice's exact passing command has been run and recorded. Do not add card-effect stubs, no-op raw Rust bridges, hidden auto-selection, UI-only rule handling, or broad `CannotBeAffected` substitutions for narrower printed protection.

## File Structure

- Modify: `code/digimon-engine/src/enums.rs` for `ModifierType` and `Keyword` variants when a slice needs new typed vocabulary.
- Modify: `code/digimon-engine/src/modifiers.rs` for player-scoped and permanent-scoped modifier registry storage/query helpers.
- Modify: `code/digimon-engine/src/action/mask.rs` for option color legality, attack legality, blocker/collision windows, and any mask-visible keyword behavior.
- Modify: `code/digimon-engine/src/action/decode.rs` for matching execution validation when the mask suppresses or exposes actions.
- Modify: `code/digimon-engine/src/game.rs` for unified keyword queries, source-kind immunity gates, security attack totals, dynamic aura contribution, and card-data lookup helpers.
- Modify: `code/digimon-engine/src/game_actions.rs` for zone movement and option-use execution guardrails.
- Modify: `code/digimon-engine/src/combat.rs` for Collision, Piercing, Reboot, Retaliation, Overclock, redirect immunity, and attack-state behavior.
- Modify: `code/digimon-engine/src/effect.rs` and `code/digimon-engine/src/effect_context.rs` for builder/context helpers needed by reusable modifiers and dynamic aura formulas.
- Modify: `code/digimon-engine/src/card_data.rs`, `code/digimon-engine/src/card_registry.rs`, `code/digimon-engine/src/cards.rs`, and `code/digimon-engine/src/dna_digivolve.rs` for printed keyword and DigiXros alias parsing/matching.
- Modify: `code/digimon-dsl/src/clause.rs`, `code/digimon-dsl/src/compiled.rs`, `code/digimon-dsl/src/compile.rs`, and `code/digimon-dsl/src/formula.rs` for DSL syntax and compiled forms.
- Modify: `code/digimon-engine/src/dsl_cards/lower_aura.rs`, `code/digimon-engine/src/dsl_cards/lower_flood_gate.rs`, `code/digimon-engine/src/dsl_cards/lower_grant_keyword.rs`, `code/digimon-engine/src/dsl_cards/modifier_map.rs`, and `code/digimon-engine/src/dsl_cards/formula_eval.rs` for runtime lowering.
- Test: `code/digimon-engine/tests/flood_gates/group6_option_color.rs`.
- Test: `code/digimon-engine/tests/replacements/source_scoped_immunity.rs`.
- Test: `code/digimon-engine/tests/dsl/group6_auras.rs` plus `code/digimon-engine/tests/dsl/main.rs`.
- Test: `code/digimon-engine/tests/combat/group6_keywords.rs` plus `code/digimon-engine/tests/combat/main.rs`.
- Test: `code/digimon-engine/tests/combat/group6_overclock.rs` plus `code/digimon-engine/tests/combat/main.rs`.
- Test: `code/digimon-engine/tests/dsl/group6_dynamic_formulas.rs` plus `code/digimon-engine/tests/dsl/main.rs`.
- Test: `code/digimon-engine/tests/keyword_parsing.rs` and `code/digimon-engine/tests/dna_digivolve_user_action.rs` for DigiXros aliases.
- Modify: `docs/RUST_ENGINE_API.md`, `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/dsl-vocab-gaps.md` for each closed slice.

---

### Task 1: IgnoreColorRequirement In Option Masks And Decode

**Files:**
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/modifiers.rs`
- Modify: `code/digimon-engine/src/action/mask.rs`
- Modify: `code/digimon-engine/src/action/decode.rs`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Test: `code/digimon-engine/tests/flood_gates/group6_option_color.rs`
- Modify: `code/digimon-engine/tests/flood_gates/main.rs`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Write the failing mask and decode tests**

Create `code/digimon-engine/tests/flood_gates/group6_option_color.rs`:

```rust
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;

#[test]
fn player_modifier_bypasses_option_color_requirement_in_mask() {
    let mut r = DebugRunner::builder()
        .with_player_1_hand(vec!["P-206"])
        .with_player_1_battle_area(vec![])
        .build();
    r.game.current_player = 0;
    r.game.memory = 10;

    let before = r.game.get_action_mask(0);
    assert_eq!(before[PLAY_HAND_START as usize], 0.0, "P-206 has no matching board color yet");

    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::IgnoreColorRequirement,
            0,
            Expiry::EndOfTurn,
            None,
            0,
        ),
    );

    let after = r.game.get_action_mask(0);
    assert_eq!(after[PLAY_HAND_START as usize], 1.0, "player-scoped IgnoreColorRequirement opens the option-use bit");
}

#[test]
fn decode_rejects_option_without_color_or_bypass_and_accepts_with_bypass() {
    let mut r = DebugRunner::builder()
        .with_player_1_hand(vec!["P-206"])
        .with_player_1_battle_area(vec![])
        .build();
    r.game.current_player = 0;
    r.game.memory = 10;

    r.game.decode_action(PLAY_HAND_START, 0);
    assert_eq!(r.game.player(0).hand.len(), 1, "decoder must enforce the same color rule as the mask");

    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::IgnoreColorRequirement,
            0,
            Expiry::EndOfTurn,
            None,
            0,
        ),
    );
    r.game.decode_action(PLAY_HAND_START, 0);
    assert_eq!(r.game.player(0).hand.len(), 0, "bypass should make the option executable");
}
```

Add `mod group6_option_color;` to `code/digimon-engine/tests/flood_gates/main.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test flood_gates -- group6_option_color --nocapture
```

Expected: FAIL because `ModifierType::IgnoreColorRequirement` is missing or not consumed by `option_use_requirement_or_color_available`, and decode may still allow a masked-out play path.

- [ ] **Step 3: Implement the minimal engine behavior**

In `code/digimon-engine/src/enums.rs`, add the variant if absent:

```rust
IgnoreColorRequirement,
```

In `code/digimon-engine/src/action/mask.rs`, route the option-color helper through the player modifier before checking board colors:

```rust
fn option_use_requirement_or_color_available(
    card: &CardSource,
    game: &Game,
    player_id: PlayerId,
) -> bool {
    if game
        .modifiers
        .player_has(player_id, ModifierType::IgnoreColorRequirement)
    {
        return true;
    }
    if option_use_requirement_satisfied(card, game, player_id) {
        return true;
    }
    option_color_available(card, game, player_id)
}
```

In `code/digimon-engine/src/action/decode.rs`, call the same legality helper before executing a hand Option play. If the helper is private today, make it `pub(crate)` in `mask.rs` and import it instead of duplicating color logic:

```rust
use crate::action::mask::option_use_requirement_or_color_available;

if is_option_use && !option_use_requirement_or_color_available(card, self, player_id) {
    return;
}
```

In `code/digimon-engine/src/effect_context.rs`, expose a reusable helper for cards and DSL lowering:

```rust
pub fn ignore_option_color_requirement(&mut self, target_player: PlayerId, expiry: Expiry) {
    self.game.modifiers.add_player_modifier(
        target_player,
        PlayerModifierEntry::simple(
            ModifierType::IgnoreColorRequirement,
            0,
            expiry,
            self.source_permanent(),
            self.controller,
        ),
    );
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test flood_gates -- group6_option_color --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run the contract smoke tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- action_space_size_constant_matches_masks tensor_size_constant_matches_observation --nocapture
```

Expected: PASS and no `ACTION_SPACE_SIZE` or `TENSOR_SIZE` change.

- [ ] **Step 6: Update trackers**

In `docs/RUST_ENGINE_GAPS.md`, update the player-scoped modifier entry to say `IgnoreColorRequirement` is implemented for option masks and decode, with the passing `flood_gates` command. In `qa/archetype-qa/engine-gaps.md`, add a Rust note beside the legacy "Ignore Color Requirement" entry. In `qa/dsl-vocab-gaps.md`, keep any DSL-specific blocker open unless a DSL syntax path also landed in this task.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/enums.rs code/digimon-engine/src/modifiers.rs code/digimon-engine/src/action/mask.rs code/digimon-engine/src/action/decode.rs code/digimon-engine/src/effect_context.rs code/digimon-engine/tests/flood_gates/main.rs code/digimon-engine/tests/flood_gates/group6_option_color.rs docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md
git commit -m "feat: honor ignore color requirement in rust option masks"
```

---

### Task 2: Source-Scoped Return And De-Digivolve Immunity

**Files:**
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/modifiers.rs`
- Modify: `code/digimon-engine/src/replacement.rs`
- Modify: `code/digimon-engine/src/game.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/effect_context.rs`
- Test: `code/digimon-engine/tests/replacements/source_scoped_immunity.rs`
- Modify: `code/digimon-engine/tests/replacements/main.rs`
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`

- [ ] **Step 1: Write the failing source-scoped immunity tests**

Create `code/digimon-engine/tests/replacements/source_scoped_immunity.rs`:

```rust
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

fn protected_handle() -> PermanentHandle {
    PermanentHandle { player: 0, index: 0 }
}

#[test]
fn opponent_effect_cannot_return_protected_digimon_to_hand() {
    let mut r = DebugRunner::builder()
        .with_player_1_battle_area(vec!["EX8-070"])
        .build();
    let target = protected_handle();
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(ModifierType::CannotBeReturnedToHand, Expiry::Permanent, 0),
    );

    let moved = r.game.return_to_hand_from_effect(target, 1);
    assert!(!moved, "opponent effect should be blocked");
    assert_eq!(r.game.player(0).battle_area.len(), 1);
    assert!(r.game.player(0).hand.is_empty());
}

#[test]
fn own_effect_can_return_protected_digimon_to_hand() {
    let mut r = DebugRunner::builder()
        .with_player_1_battle_area(vec!["EX8-070"])
        .build();
    let target = protected_handle();
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(ModifierType::CannotBeReturnedToHand, Expiry::Permanent, 0),
    );

    let moved = r.game.return_to_hand_from_effect(target, 0);
    assert!(moved, "own effect should not be blocked by opponent-only immunity");
    assert!(r.game.player(0).battle_area.is_empty());
    assert_eq!(r.game.player(0).hand.len(), 1);
}

#[test]
fn opponent_effect_cannot_return_to_deck_or_de_digivolve_protected_digimon() {
    let mut r = DebugRunner::builder()
        .with_player_1_battle_area_stack(vec![vec!["BT18-064", "BT17-064"]])
        .build();
    let target = protected_handle();
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(ModifierType::CannotBeReturnedToDeck, Expiry::Permanent, 0),
    );
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(ModifierType::CannotBeDeDigivolved, Expiry::Permanent, 0),
    );

    assert!(!r.game.return_to_deck_from_effect(target, 1));
    assert!(!r.game.de_digivolve_from_effect(target, 1, 1));
    assert_eq!(r.game.player(0).battle_area.len(), 1);
    assert_eq!(r.game.player(0).battle_area[0].card_sources.len(), 2);
}
```

Add `mod source_scoped_immunity;` to `code/digimon-engine/tests/replacements/main.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- source_scoped_immunity --nocapture
```

Expected: FAIL because `CannotBeReturnedToDeck`, `CannotBeDeDigivolved`, and/or `*_from_effect` source-player consumers are absent.

- [ ] **Step 3: Implement typed modifiers and cause filtering**

In `code/digimon-engine/src/enums.rs`, ensure all three variants exist:

```rust
CannotBeReturnedToHand,
CannotBeReturnedToDeck,
CannotBeDeDigivolved,
```

In `code/digimon-engine/src/modifiers.rs`, keep the default opponent-only cause filter:

```rust
ModifierType::CannotBeReturnedToDeck
| ModifierType::CannotBeReturnedToHand
| ModifierType::CannotBeTrashedByEffect
| ModifierType::CannotBeDeDigivolved => Some(ReplacementCause::OpponentEffect),
```

Add a query helper that checks the target and source controller:

```rust
pub fn blocks_opponent_effect(
    &self,
    target: PermanentHandle,
    modifier: ModifierType,
    effect_player: PlayerId,
) -> bool {
    self.get(target, modifier).into_iter().any(|entry| {
        let opponent_only = entry
            .cause_filter
            .map(|cause| cause == ReplacementCause::OpponentEffect)
            .unwrap_or(false);
        opponent_only && effect_player != target.player
    })
}
```

In `code/digimon-engine/src/game.rs` or `game_actions.rs`, add source-player wrappers and use them from `EffectContext`:

```rust
pub fn return_to_hand_from_effect(&mut self, target: PermanentHandle, effect_player: PlayerId) -> bool {
    if self.modifiers.blocks_opponent_effect(target, ModifierType::CannotBeReturnedToHand, effect_player) {
        return false;
    }
    self.return_to_hand(target)
}

pub fn return_to_deck_from_effect(&mut self, target: PermanentHandle, effect_player: PlayerId) -> bool {
    if self.modifiers.blocks_opponent_effect(target, ModifierType::CannotBeReturnedToDeck, effect_player) {
        return false;
    }
    self.return_to_deck_bottom(target)
}

pub fn de_digivolve_from_effect(&mut self, target: PermanentHandle, effect_player: PlayerId, amount: u8) -> bool {
    if self.modifiers.blocks_opponent_effect(target, ModifierType::CannotBeDeDigivolved, effect_player) {
        return false;
    }
    self.de_digivolve(target, amount)
}
```

In `code/digimon-engine/src/effect_context.rs`, add narrow builder sugar:

```rust
pub fn grant_zone_return_immunity_to_opponent_effects(
    &mut self,
    target: PermanentHandle,
    expiry: Expiry,
) {
    for modifier in [
        ModifierType::CannotBeReturnedToHand,
        ModifierType::CannotBeReturnedToDeck,
        ModifierType::CannotBeDeDigivolved,
    ] {
        self.game.modifiers.add(
            target,
            ModifierEntry::passive_replacement(modifier, expiry, self.controller),
        );
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- source_scoped_immunity --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run related regression tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_source_kind --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- progress_mutation_gates --nocapture
```

Expected: PASS. Progress and broad `CannotBeAffected` behavior must remain unchanged.

- [ ] **Step 6: Update trackers**

In `docs/RUST_ENGINE_API.md`, document `return_to_hand_from_effect`, `return_to_deck_from_effect`, `de_digivolve_from_effect`, and `grant_zone_return_immunity_to_opponent_effects`. In `docs/RUST_ENGINE_GAPS.md`, mark "Source-scoped return-immunity modifiers" resolved only for the three covered consumers and list the passing replacements command. In `qa/archetype-qa/engine-gaps.md`, add the Rust closure note for EX8-070, P-215, and BT18-064.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/enums.rs code/digimon-engine/src/modifiers.rs code/digimon-engine/src/replacement.rs code/digimon-engine/src/game.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/src/effect_context.rs code/digimon-engine/tests/replacements/main.rs code/digimon-engine/tests/replacements/source_scoped_immunity.rs docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md
git commit -m "feat: add source scoped return immunity"
```

---

### Task 3: Declarative Auras To Player And Permanent Modifiers

**Files:**
- Modify: `code/digimon-dsl/src/clause.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_aura.rs`
- Modify: `code/digimon-engine/src/dsl_cards/modifier_map.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `code/digimon-engine/src/game.rs`
- Test: `code/digimon-engine/tests/dsl/group6_auras.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Write failing DSL aura tests**

Create `code/digimon-engine/tests/dsl/group6_auras.rs`:

```rust
use digimon_dsl::compile::compile_card_spec;
use digimon_dsl::loader::load_card_spec_from_str;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;
use digimon_engine::permanent::PermanentHandle;

#[test]
fn aura_other_predicate_excludes_source_permanent() {
    let yaml = r#"
card_id: TEST-AURA-OTHER
name: Aura Source
card_type: Digimon
colors: [Blue]
level: 3
play_cost: 3
dp: 1000
effects:
  - kind: aura
    target: { owner: you, trait: Gaossmon, other: true }
    dp_modifier: 3000
"#;
    let spec = load_card_spec_from_str(yaml).expect("parse");
    let compiled = compile_card_spec(&spec).expect("compile");
    assert_eq!(compiled.declaratives.len(), 1);

    let mut r = DebugRunner::builder()
        .with_dsl_card(compiled)
        .with_player_1_battle_area(vec!["TEST-AURA-OTHER", "BT5-008"])
        .build();
    r.game.tick_declarative_effects();

    let source = PermanentHandle { player: 0, index: 0 };
    let ally = PermanentHandle { player: 0, index: 1 };
    assert_eq!(r.game.modifiers.sum(source, ModifierType::ChangeDp), 0);
    assert_eq!(r.game.modifiers.sum(ally, ModifierType::ChangeDp), 3000);
}

#[test]
fn aura_can_install_player_scoped_modifier_from_static_field_effect() {
    let yaml = r#"
card_id: TEST-PLAYER-AURA
name: Player Aura Source
card_type: Digimon
colors: [Black]
level: 3
play_cost: 3
dp: 1000
effects:
  - kind: aura
    target_player: opponent
    modifier: CannotReduceDigivolveCost
"#;
    let spec = load_card_spec_from_str(yaml).expect("parse");
    let compiled = compile_card_spec(&spec).expect("compile");

    let mut r = DebugRunner::builder()
        .with_dsl_card(compiled)
        .with_player_1_battle_area(vec!["TEST-PLAYER-AURA"])
        .build();
    r.game.tick_declarative_effects();

    assert!(r.game.modifiers.player_has(1, ModifierType::CannotReduceDigivolveCost));
    assert!(!r.game.modifiers.player_has(0, ModifierType::CannotReduceDigivolveCost));
}
```

Add `mod group6_auras;` to `code/digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras --nocapture
```

Expected: FAIL because `AuraBody` lacks `target_player`, `lower_aura` ignores `modifier`, and `PredicateSpec.other` is not enforced at runtime.

- [ ] **Step 3: Add DSL schema and compile fields**

In `code/digimon-dsl/src/clause.rs`, extend `AuraBody`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub target_player: Option<crate::common::PlayerRef>,
```

In `code/digimon-dsl/src/compiled.rs`, extend `CompiledDeclarativeClause::Aura`:

```rust
target_player: Option<CompiledPlayerRef>,
```

In `code/digimon-dsl/src/compile.rs`, compile it:

```rust
target_player: a.target_player.map(compile_player_ref),
```

- [ ] **Step 4: Implement runtime lowering**

In `code/digimon-engine/src/dsl_cards/predicate.rs`, enforce `other: true` in permanent predicate evaluation:

```rust
if pred.other == Some(true) {
    if let Some(source) = rctx.source_permanent {
        if handle == source {
            return false;
        }
    }
}
```

In `code/digimon-engine/src/dsl_cards/lower_aura.rs`, lower `modifier` to a permanent modifier when `target` is present, and to a player modifier when `target_player` is present:

```rust
let modifier = modifier.and_then(|name| lookup_modifier_type(&name));
let target_player = target_player;

builder = builder.process(move |ctx| {
    if let (Some(player_ref), Some(modifier)) = (target_player, modifier) {
        for pid in resolve_player_ref(ctx, player_ref) {
            ctx.add_player_modifier(pid, modifier, 0, Expiry::Permanent);
        }
        return;
    }
    // Existing permanent scan remains here, now applying dp_modifier,
    // grant_keyword, and modifier to each matched PermanentHandle.
});
```

Use the existing player-ref resolver used by `lower_flood_gate.rs`; do not create a second resolver with different semantics.

- [ ] **Step 5: Run test to verify it passes**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run DSL schema and declarative regressions**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- parse_declarative standalone_declaratives_exit phase2c_modifiers group6_auras --nocapture
```

Expected: PASS.

- [ ] **Step 7: Update trackers**

In `qa/dsl-vocab-gaps.md`, close `G-OTHER-PREDICATE-UNEVALUATED` and update `G-PLAYER-FLOOD-GATE-DSL` to say the remaining static dispatcher blocker is closed by `tick_declarative_effects` for aura/flood-gate installation if this task proves it. In `docs/RUST_ENGINE_GAPS.md`, update "Declarative aura to player-scoped modifiers" and "Named-target declarative auras" with the passing DSL command.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-dsl/src/clause.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/lower_aura.rs code/digimon-engine/src/dsl_cards/modifier_map.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/src/game.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/group6_auras.rs docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
git commit -m "feat: lower declarative auras to scoped modifiers"
```

---

### Task 4: Collision, Piercing, Reboot, And Retaliation End-To-End

**Files:**
- Modify: `code/digimon-engine/src/card_data.rs`
- Modify: `code/digimon-engine/src/game.rs`
- Modify: `code/digimon-engine/src/combat.rs`
- Modify: `code/digimon-engine/src/action/mask.rs`
- Modify: `code/digimon-engine/src/action/decode.rs`
- Modify: `code/digimon-engine/src/cards.rs`
- Test: `code/digimon-engine/tests/combat/group6_keywords.rs`
- Modify: `code/digimon-engine/tests/combat/main.rs`
- Modify: `code/digimon-engine/tests/keyword_parsing.rs`
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`

- [ ] **Step 1: Write failing keyword end-to-end tests**

Create `code/digimon-engine/tests/combat/group6_keywords.rs`:

```rust
use digimon_engine::action::space::{encode_attack, SEL_REPLACEMENT_PASS};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;
use digimon_engine::permanent::PermanentHandle;

#[test]
fn collision_removes_block_pass_when_blocker_exists() {
    let mut r = DebugRunner::builder()
        .with_player_1_battle_area(vec!["EX10-034"])
        .with_player_2_battle_area(vec!["BT5-008"])
        .build();
    let attacker = PermanentHandle { player: 0, index: 0 };
    assert!(r.game.has_keyword(attacker, Keyword::Collision));

    r.game.decode_action(encode_attack(0, 30), 0);
    let mask = r.game.get_action_mask(1);
    assert_eq!(mask[SEL_REPLACEMENT_PASS as usize], 0.0, "Collision makes block mandatory when a legal blocker exists");
}

#[test]
fn piercing_after_digimon_battle_checks_security_once() {
    let mut r = DebugRunner::builder()
        .with_player_1_battle_area(vec!["BT1-084"])
        .with_player_2_battle_area(vec!["BT5-008"])
        .with_player_2_security(vec!["BT1-010", "BT1-011"])
        .build();
    r.game.decode_action(encode_attack(0, 0), 0);
    r.game.drive_combat_to_completion();
    assert_eq!(r.game.player(1).security.len(), 1);
}

#[test]
fn reboot_unsuspends_during_opponents_unsuspend_phase_once() {
    let mut r = DebugRunner::builder()
        .with_player_1_battle_area(vec!["ST5-11"])
        .build();
    let h = PermanentHandle { player: 0, index: 0 };
    r.game.suspend(h, None);
    r.game.current_player = 1;

    r.game.start_unsuspend_phase();
    assert!(!r.game.player(0).battle_area[0].is_suspended);
}

#[test]
fn retaliation_deletes_battle_opponent_when_deleted_in_battle() {
    let mut r = DebugRunner::builder()
        .with_player_1_battle_area(vec!["BT2-074"])
        .with_player_2_battle_area(vec!["BT5-008"])
        .build();
    r.game.decode_action(encode_attack(0, 0), 0);
    r.game.drive_combat_to_completion();

    assert!(r.game.player(0).battle_area.is_empty());
    assert!(r.game.player(1).battle_area.is_empty(), "Retaliation deletes the battled opponent Digimon");
}
```

Add `mod group6_keywords;` to `code/digimon-engine/tests/combat/main.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_keywords --nocapture
```

Expected: FAIL on whichever keyword consumer is still incomplete. If card IDs in the test do not currently carry the printed keyword in `data/cards.json`, replace only the fixture card IDs with local known IDs; keep the four behavior assertions unchanged.

- [ ] **Step 3: Ensure printed keyword parsing covers all four keywords**

In `code/digimon-engine/src/card_data.rs`, keep these simple keyword prefixes in `parse_printed_keywords`:

```rust
("Collision", Keyword::Collision),
("Piercing", Keyword::Piercing),
("Reboot", Keyword::Reboot),
("Retaliation", Keyword::Retaliation),
```

In `code/digimon-engine/tests/keyword_parsing.rs`, add:

```rust
#[test]
fn parses_group6_core_combat_keywords() {
    let keywords = parse_printed_keywords(
        "＜Collision＞ ＜Piercing＞ ＜Reboot＞ ＜Retaliation＞",
        "",
        "",
    );
    assert!(keywords.contains(&Keyword::Collision));
    assert!(keywords.contains(&Keyword::Piercing));
    assert!(keywords.contains(&Keyword::Reboot));
    assert!(keywords.contains(&Keyword::Retaliation));
}
```

- [ ] **Step 4: Implement missing combat consumers**

Use the canonical `Game::has_keyword(handle, keyword)` helper, not `modifiers.has_keyword`, in every consumer:

```rust
let has_collision = game.has_keyword(attacker, Keyword::Collision);
let has_piercing = game.has_keyword(attacker, Keyword::Piercing);
let has_reboot = game.has_keyword(handle, Keyword::Reboot);
let has_retaliation = game.has_keyword(deleted, Keyword::Retaliation);
```

Collision must remove the block-decline/PASS bit only when at least one legal blocker exists. Piercing must fire only after Digimon-vs-Digimon battle where the opposing Digimon is deleted and the attacker survives. Reboot must unsuspend during the opponent's unsuspend phase and only once. Retaliation must trigger on battle deletion and delete the battled opponent Digimon even if the Retaliation carrier also leaves.

- [ ] **Step 5: Add mask and decoder parity checks**

In the Collision test file, add one direct decoder validation:

```rust
#[test]
fn collision_decode_rejects_block_decline_when_mandatory() {
    let mut r = DebugRunner::builder()
        .with_player_1_battle_area(vec!["EX10-034"])
        .with_player_2_battle_area(vec!["BT5-008"])
        .build();
    r.game.decode_action(encode_attack(0, 30), 0);
    r.game.decode_action(SEL_REPLACEMENT_PASS, 1);
    assert!(r.game.pending_attack.is_some(), "declining a mandatory block should not advance combat");
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_keywords collision_mandatory piercing_security reboot_unsuspend --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parses_group6_core_combat_keywords --nocapture
```

Expected: PASS.

- [ ] **Step 7: Update trackers**

In `docs/RUST_ENGINE_GAPS.md`, update the native printed keyword parsing row and the Collision grant/enforcement note with the passing commands. In `docs/RUST_ENGINE_API.md`, document that mask-affecting keywords must be consumed through `Game::has_keyword` and validated by both mask and decode tests.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/card_data.rs code/digimon-engine/src/game.rs code/digimon-engine/src/combat.rs code/digimon-engine/src/action/mask.rs code/digimon-engine/src/action/decode.rs code/digimon-engine/src/cards.rs code/digimon-engine/tests/combat/main.rs code/digimon-engine/tests/combat/group6_keywords.rs code/digimon-engine/tests/keyword_parsing.rs docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md
git commit -m "feat: enforce core combat keywords end to end"
```

---

### Task 5: Overclock Predicate Parameterization

**Files:**
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/combat.rs`
- Modify: `code/digimon-engine/src/action/mask.rs`
- Modify: `code/digimon-engine/src/action/decode.rs`
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-dsl/src/clause.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_grant_keyword.rs`
- Test: `code/digimon-engine/tests/combat/group6_overclock.rs`
- Modify: `code/digimon-engine/tests/combat/main.rs`
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `qa/archetype-qa/engine-gaps.md`

- [ ] **Step 1: Write failing Overclock selection tests**

Create `code/digimon-engine/tests/combat/group6_overclock.rs`:

```rust
use digimon_engine::action::space::{encode_field_effect, encode_select_target, SEL_REPLACEMENT_PASS};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn overclock_only_exposes_token_or_matching_other_digimon_as_cost() {
    let mut r = DebugRunner::builder()
        .with_player_1_battle_area(vec!["PUPPET-OVERCLOCK", "PUPPET-TOKEN", "BT5-008"])
        .build();
    r.game.current_player = 0;
    r.game.start_end_of_turn_attack_window();

    let effect_bit = encode_field_effect(0, 0);
    let mask = r.game.get_action_mask(0);
    assert_eq!(mask[effect_bit as usize], 1.0, "Overclock activation should be visible");

    r.game.decode_action(effect_bit, 0);
    let cost_mask = r.game.get_action_mask(0);
    assert_eq!(cost_mask[encode_select_target(1) as usize], 1.0, "matching token is legal cost");
    assert_eq!(cost_mask[encode_select_target(2) as usize], 0.0, "unmatched non-token is not legal cost");
    assert_eq!(cost_mask[SEL_REPLACEMENT_PASS as usize], 1.0, "Overclock is optional");
}

#[test]
fn overclock_decline_does_not_delete_or_attack() {
    let mut r = DebugRunner::builder()
        .with_player_1_battle_area(vec!["PUPPET-OVERCLOCK", "PUPPET-TOKEN"])
        .build();
    r.game.current_player = 0;
    r.game.start_end_of_turn_attack_window();
    r.game.decode_action(encode_field_effect(0, 0), 0);
    r.game.decode_action(SEL_REPLACEMENT_PASS, 0);

    assert_eq!(r.game.player(0).battle_area.len(), 2);
    assert!(r.game.pending_attack.is_none());
}
```

Add `mod group6_overclock;` to `code/digimon-engine/tests/combat/main.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_overclock --nocapture
```

Expected: FAIL because Overclock cost candidates are not parameterized by the printed predicate or because the pending selection does not expose PASS/decline correctly.

- [ ] **Step 3: Add typed Overclock parameters**

In `code/digimon-engine/src/enums.rs`, replace bare `Overclock` only if necessary with a parameter-holding effect record. Do not change the `Keyword::Overclock` variant if printed keyword parsing depends on it; instead store parameters in `Effect` or declarative lowering:

```rust
pub struct OverclockSpec {
    pub cost_filter: Arc<dyn Fn(&EffectReadContext, PermanentHandle) -> bool + Send + Sync>,
}
```

In `code/digimon-engine/src/effect.rs`, add:

```rust
pub overclock_cost_filter: Option<OverclockCostFilterFn>,
```

and a builder:

```rust
pub fn overclock_with_cost_filter(mut self, filter: OverclockCostFilterFn) -> Self {
    self.granted_keyword = Some(Keyword::Overclock);
    self.overclock_cost_filter = Some(filter);
    self
}
```

- [ ] **Step 4: Wire pending selection, mask, and decode**

In `code/digimon-engine/src/combat.rs`, when the end-of-turn Overclock activation is chosen, install a pending cost selection owned by the Overclock controller:

```rust
PendingSelection::OverclockCost {
    source,
    selecting_player,
    min: 1,
    max: 1,
    optional: true,
    candidates,
}
```

In `code/digimon-engine/src/action/mask.rs`, emit only candidate target bits plus `SEL_REPLACEMENT_PASS` for optional decline. In `code/digimon-engine/src/action/decode.rs`, reject target bits outside the stored candidate set before deleting the cost permanent or starting the attack.

- [ ] **Step 5: Add DSL syntax for predicate parameterization**

In `code/digimon-dsl/src/clause.rs`, extend `GrantKeywordBody`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub overclock_cost_filter: Option<PredicateSpec>,
```

Compile it through `CompiledDeclarativeClause::GrantKeyword` and lower it in `lower_grant_keyword.rs`:

```rust
if keyword == "Overclock" {
    return Some(
        Effect::declarative(card)
            .overclock_with_cost_filter(Box::new(move |rctx, handle| {
                eval_predicate(&filter, rctx, PredicateSubject::Permanent(handle))
            }))
            .build(),
    );
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_overclock overclock --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- parse_declarative --nocapture
```

Expected: PASS.

- [ ] **Step 7: Update trackers**

In `qa/archetype-qa/engine-gaps.md`, close `G-OVERCLOCK-TRAIT-FILTER` with the passing `group6_overclock` command. In `docs/RUST_ENGINE_API.md`, document the pending selection shape and the action-mask/decoder requirement for Overclock cost candidates.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/enums.rs code/digimon-engine/src/combat.rs code/digimon-engine/src/action/mask.rs code/digimon-engine/src/action/decode.rs code/digimon-engine/src/selection.rs code/digimon-dsl/src/clause.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/lower_grant_keyword.rs code/digimon-engine/tests/combat/main.rs code/digimon-engine/tests/combat/group6_overclock.rs docs/RUST_ENGINE_API.md qa/archetype-qa/engine-gaps.md
git commit -m "feat: parameterize overclock cost selection"
```

---

### Task 6: Dynamic DP And Security Attack Formula Auras

**Files:**
- Modify: `code/digimon-engine/src/effect.rs`
- Modify: `code/digimon-engine/src/game.rs`
- Modify: `code/digimon-engine/src/combat.rs`
- Modify: `code/digimon-engine/src/tensor.rs`
- Modify: `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_aura.rs`
- Modify: `code/digimon-dsl/src/clause.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Test: `code/digimon-engine/tests/dsl/group6_dynamic_formulas.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Write failing dynamic formula tests**

Create `code/digimon-engine/tests/dsl/group6_dynamic_formulas.rs`:

```rust
use digimon_dsl::compile::compile_card_spec;
use digimon_dsl::loader::load_card_spec_from_str;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::permanent::PermanentHandle;

#[test]
fn self_aura_dp_formula_recomputes_after_stack_depth_changes() {
    let yaml = r#"
card_id: TEST-DP-FORMULA
name: Formula Body
card_type: Digimon
colors: [Black]
level: 4
play_cost: 4
dp: 4000
effects:
  - kind: aura
    target: {}
    dp_modifier_fn: { base: 0, per: material_count, delta: 1000 }
"#;
    let spec = load_card_spec_from_str(yaml).expect("parse");
    let compiled = compile_card_spec(&spec).expect("compile");
    let mut r = DebugRunner::builder()
        .with_dsl_card(compiled)
        .with_player_1_battle_area_stack(vec![vec!["TEST-DP-FORMULA", "BT5-008", "BT5-009"]])
        .build();
    let h = PermanentHandle { player: 0, index: 0 };

    assert_eq!(r.game.effective_dp(h), Some(6000));
    r.game.de_digivolve(h, 1);
    assert_eq!(r.game.effective_dp(h), Some(5000), "formula must be live, not snapshotted");
}

#[test]
fn security_attack_formula_recomputes_at_attack_resolution() {
    let yaml = r#"
card_id: TEST-SA-FORMULA
name: Security Formula Body
card_type: Digimon
colors: [Red]
level: 4
play_cost: 4
dp: 4000
effects:
  - kind: aura
    target: {}
    security_attack_fn: { base: 1, per: material_count, delta: 1 }
"#;
    let spec = load_card_spec_from_str(yaml).expect("parse");
    let compiled = compile_card_spec(&spec).expect("compile");
    let mut r = DebugRunner::builder()
        .with_dsl_card(compiled)
        .with_player_1_battle_area_stack(vec![vec!["TEST-SA-FORMULA", "BT5-008"]])
        .with_player_2_security(vec!["BT1-010", "BT1-011", "BT1-012"])
        .build();

    r.game.decode_action(digimon_engine::action::space::encode_attack(0, 30), 0);
    r.game.drive_combat_to_completion();
    assert_eq!(r.game.player(1).security.len(), 1, "base 1 plus one material performs two checks");
}
```

Add `mod group6_dynamic_formulas;` to `code/digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_dynamic_formulas --nocapture
```

Expected: FAIL because `dp_modifier_fn` and `security_attack_fn` are unknown DSL fields or because formulas are snapshotted instead of evaluated at query time.

- [ ] **Step 3: Add DSL and effect storage**

In `code/digimon-dsl/src/clause.rs`, extend `AuraBody`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub dp_modifier_fn: Option<crate::formula::FormulaSpec>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub security_attack_fn: Option<crate::formula::FormulaSpec>,
```

In `code/digimon-dsl/src/compiled.rs`, add compiled formula fields to `CompiledDeclarativeClause::Aura`:

```rust
dp_modifier_fn: Option<CompiledFormula>,
security_attack_fn: Option<CompiledFormula>,
```

In `code/digimon-dsl/src/compile.rs`, compile them:

```rust
dp_modifier_fn: a.dp_modifier_fn.as_ref().map(compile_formula),
security_attack_fn: a.security_attack_fn.as_ref().map(compile_formula),
```

In `code/digimon-engine/src/effect.rs`, add:

```rust
pub dp_modifier_formula: Option<CompiledFormula>,
pub security_attack_formula: Option<CompiledFormula>,
```

and builder methods:

```rust
pub fn dp_modifier_formula(mut self, formula: CompiledFormula) -> Self {
    self.dp_modifier_formula = Some(formula);
    self
}

pub fn security_attack_formula(mut self, formula: CompiledFormula) -> Self {
    self.security_attack_formula = Some(formula);
    self
}
```

- [ ] **Step 4: Evaluate formulas at query time**

In `code/digimon-engine/src/game.rs`, update `source_dp_contribution`:

```rust
if let Some(formula) = &effect.dp_modifier_formula {
    total += evaluate_formula_for_source(self, perm, source_index, formula);
}
```

In the same file, replace fixed security attack summing with a helper that includes printed keywords, `ModifierType::SecurityAttackChange`, and formula-backed aura effects:

```rust
pub fn effective_security_checks(&self, attacker: PermanentHandle) -> u8 {
    let mut checks = 1
        + self.security_attack_keyword_bonus(attacker)
        + self.modifiers.sum(attacker, ModifierType::SecurityAttackChange);
    checks += self.security_attack_formula_bonus(attacker);
    checks.max(0) as u8
}
```

Use `effective_security_checks` from `code/digimon-engine/src/combat.rs` for direct attacks and Piercing checks.

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_dynamic_formulas phase2f2_formula_eval phase2f2_modifier_formula --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- tensor_shape_and_values --nocapture
```

Expected: PASS. Tensor shape must not change; only DP-derived values may change according to the new formula behavior.

- [ ] **Step 6: Update trackers**

In `qa/dsl-vocab-gaps.md`, close `G-AURA-DP-FORMULA` and add a sibling closure note for dynamic Security Attack formula auras if one exists. In `docs/RUST_ENGINE_GAPS.md`, update "Dynamic DP scaling modifier" and "Dynamic security attack modifiers" with the passing commands. In `docs/RUST_ENGINE_API.md`, document that formula auras are continuously recomputed by `source_dp_contribution` and security-check resolution.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/effect.rs code/digimon-engine/src/game.rs code/digimon-engine/src/combat.rs code/digimon-engine/src/tensor.rs code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-engine/src/dsl_cards/lower_aura.rs code/digimon-dsl/src/clause.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/group6_dynamic_formulas.rs docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md
git commit -m "feat: support dynamic formula auras"
```

---

### Task 7: DigiXros-Scoped Name Aliases

**Files:**
- Modify: `code/digimon-engine/src/card_data.rs`
- Modify: `code/digimon-engine/src/dna_digivolve.rs`
- Modify: `code/digimon-engine/src/deck_tools.rs`
- Modify: `code/digimon-engine/src/effect.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_aura.rs`
- Test: `code/digimon-engine/tests/keyword_parsing.rs`
- Modify: `code/digimon-engine/tests/dna_digivolve_user_action.rs`
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`

- [ ] **Step 1: Write failing alias tests**

In `code/digimon-engine/tests/keyword_parsing.rs`, add:

```rust
#[test]
fn parses_digixros_scoped_alias_without_global_name_alias() {
    let data = CardData::from_json_fixture(r#"{
        "card_id": "BT21-021",
        "name": "OmniShoutmon",
        "effect_text": "This card is also treated as [Shoutmon] for DigiXros."
    }"#);
    assert_eq!(data.digixros_aliases, vec!["Shoutmon"]);
    assert!(!data.name_aliases.contains(&"Shoutmon".to_string()));
}
```

In `code/digimon-engine/tests/dna_digivolve_user_action.rs`, add:

```rust
#[test]
fn digixros_matching_accepts_scoped_alias_but_generic_name_checks_do_not() {
    let mut r = DebugRunner::builder()
        .with_player_1_hand(vec!["BT21-021", "BT10-009"])
        .build();

    assert!(r.game.card_can_satisfy_digixros_name("BT21-021", "Shoutmon"));
    assert!(!r.game.card_matches_generic_name("BT21-021", "Shoutmon"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parses_digixros_scoped_alias_without_global_name_alias --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- digixros_matching_accepts_scoped_alias_but_generic_name_checks_do_not --nocapture
```

Expected: FAIL because `CardData::digixros_aliases` and alias-aware DigiXros matching are missing.

- [ ] **Step 3: Add card-data alias storage and parsing**

In `code/digimon-engine/src/card_data.rs`, add:

```rust
pub digixros_aliases: Vec<String>,
```

Parse text with a narrowly scoped extractor:

```rust
fn parse_digixros_aliases(effect_text: &str) -> Vec<String> {
    let marker = "also treated as [";
    let mut aliases = Vec::new();
    for tail in effect_text.split(marker).skip(1) {
        let Some((name, after_name)) = tail.split_once(']') else {
            continue;
        };
        if after_name.contains("for DigiXros") {
            aliases.push(name.trim().to_string());
        }
    }
    aliases
}
```

Keep this parser scoped to text containing `for DigiXros`; do not append the alias to any generic `name_aliases` or predicate name lookup field.

- [ ] **Step 4: Use aliases only in DigiXros matching**

In `code/digimon-engine/src/dna_digivolve.rs`, route DigiXros material name checks through:

```rust
pub fn card_can_satisfy_digixros_name(&self, card_id: &str, required_name: &str) -> bool {
    let Some(data) = self.card_data_by_id(card_id) else {
        return false;
    };
    data.name == required_name || data.digixros_aliases.iter().any(|alias| alias == required_name)
}
```

Leave generic name predicates on `CardData.name` plus existing generic aliases only.

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parses_digixros_scoped_alias_without_global_name_alias --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- digixros_matching_accepts_scoped_alias_but_generic_name_checks_do_not --nocapture
```

Expected: PASS.

- [ ] **Step 6: Update trackers**

In `docs/RUST_ENGINE_GAPS.md`, close "DigiXros name alias" with the two passing commands. In `docs/RUST_ENGINE_API.md`, document that DigiXros aliases are intentionally not generic name aliases.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/card_data.rs code/digimon-engine/src/dna_digivolve.rs code/digimon-engine/src/deck_tools.rs code/digimon-engine/src/effect.rs code/digimon-engine/src/dsl_cards/lower_aura.rs code/digimon-engine/tests/keyword_parsing.rs code/digimon-engine/tests/dna_digivolve_user_action.rs docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md
git commit -m "feat: add digixros scoped name aliases"
```

---

### Task 8: Contract Review And Group Closure

**Files:**
- Modify: `docs/RUST_ENGINE_API.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify only if constants changed: `docs/ACTION_SPEC.md`
- Modify only if constants changed: `docs/TENSOR_SPEC.md`
- Modify only if constants changed: `code/digimon-engine-py/src/lib.rs`
- Modify only if constants changed: `code/digimon_gym/digimon_gym.py`
- Modify only if constants changed: frontend action/tensor constants under `code/frontend/src/`

- [ ] **Step 1: Run full targeted group suite**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test flood_gates -- group6_option_color --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- source_scoped_immunity --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras group6_dynamic_formulas --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_keywords group6_overclock --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parses_group6_core_combat_keywords parses_digixros_scoped_alias_without_global_name_alias --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- digixros_matching_accepts_scoped_alias_but_generic_name_checks_do_not --nocapture
```

Expected: all commands PASS.

- [ ] **Step 2: Run broad regression gates**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl --nocapture
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v
python -m pytest code/tests/rl -v
```

Expected: all commands PASS. If Python environment dependencies are not installed, record the exact import or dependency error in the tracker edit and do not claim RL/PyO3 contract verification passed.

- [ ] **Step 3: Review action and tensor contracts**

Inspect `code/digimon-engine/src/action/space.rs`, `code/digimon-engine/src/tensor.rs`, `docs/ACTION_SPEC.md`, and `docs/TENSOR_SPEC.md`.

Expected:

```text
ACTION_SPACE_SIZE remains 2168.
TENSOR_SIZE remains 1375.
No new action bit is required for Group 6; mask-visible choices reuse existing play, attack, block, replacement/pass, and selection ranges.
No new tensor slot is required for Group 6; dynamic DP changes affect existing DP-derived values only.
```

If any constant changed, update the Rust constants, PyO3 exports, Python env spaces, frontend constants, and both specs in the same commit before closing the group.

- [ ] **Step 4: Search for unfinished markers and raw-rust escapes introduced by this group**

Run:

```bash
$unfinished = @('TO' + 'DO', 'T' + 'BD', 'raw_rust', 'no-op', 'stub', 'place' + 'holder', 'approx') -join '|'
Select-String -Path 'code/digimon-engine/src/**/*.rs','code/digimon-dsl/src/**/*.rs','code/digimon-engine/cards/**/*.yaml' -Pattern $unfinished
```

Expected: no new unfinished marker or raw-Rust escape attributable to Group 6. Existing unrelated hits must be listed in the final implementation notes and left untouched.

- [ ] **Step 5: Final tracker edits**

Update:

```text
docs/RUST_ENGINE_GAPS.md
qa/archetype-qa/engine-gaps.md
qa/dsl-vocab-gaps.md
```

Each closed entry must include:

```text
Status: RESOLVED by Group 6.
Passing command(s): <exact command from this plan>.
Remaining related blocker: <specific blocker>, or "none for this primitive".
```

- [ ] **Step 6: Self-review**

Verify these statements are true before committing:

```text
Every mask-visible keyword or modifier has both mask and decode/execution tests.
Every new DSL field has parse, compile, lower, and behavior coverage.
Continuous auras are recomputed at query time and cannot leave stale materialized modifiers behind.
Source-scoped immunity blocks only opponent effects and does not block own effects, battle, costs, or rule cleanup.
DigiXros aliases are scoped to DigiXros material matching only.
No action-space or tensor constants changed without synchronized docs/PyO3/RL/frontend updates.
No tracker entry was closed without a passing command.
```

- [ ] **Step 7: Commit**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md docs/ACTION_SPEC.md docs/TENSOR_SPEC.md code/digimon-engine-py/src/lib.rs code/digimon_gym/digimon_gym.py code/frontend/src
git commit -m "docs: close modifiers auras keywords gap group"
```

If action/tensor constants did not change and frontend/PyO3/RL files were not edited, stage only the docs/tracker files:

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md
git commit -m "docs: close modifiers auras keywords gap group"
```

---

## Execution Notes

Start with Task 1 because it is a small mask/decode slice and validates the player-scoped modifier lane. Task 2 should follow before broader aura work because source-scoped immunity sharpens the registry semantics. Task 3 then expands declarative delivery. Tasks 4 and 5 should be handled after the aura delivery work is stable because they depend on correct keyword lookup and mask/execution parity. Task 6 should land after Task 3 because it extends the same aura pipeline. Task 7 is intentionally scoped to DigiXros matching and must not leak into generic name predicates. Task 8 is the closure gate, not a substitute for per-task tracker updates.

## Self-Review

- Spec coverage: Covers parent roadmap slices for IgnoreColorRequirement, source-scoped return/de-digivolve immunity, declarative aura to player-scoped modifiers, Collision/Piercing/Reboot/Retaliation, Overclock predicate parameterization, dynamic DP/Security Attack formula-backed auras, and DigiXros scoped alias handling.
- Unfinished-marker scan: The plan contains no literal forbidden unfinished-work tokens and no raw-Rust escape as a final state.
- Type/name consistency: Reuses existing `ModifierType`, `PlayerModifierEntry`, `ModifierEntry`, `Keyword`, `CompiledFormula`, `AuraBody`, `CompiledDeclarativeClause::Aura`, `Game::has_keyword`, and `source_dp_contribution` names. New proposed helper names are introduced before use in later steps.
- No-approximations compliance: Every player-visible choice has mask and decode/execution validation; optional Overclock decline remains explicit; source-scoped immunity is narrow and does not substitute broad `CannotBeAffected`; DigiXros aliases are scoped to DigiXros only.
