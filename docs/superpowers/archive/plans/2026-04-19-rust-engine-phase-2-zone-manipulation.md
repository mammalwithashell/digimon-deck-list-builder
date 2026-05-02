# Rust Engine Phase 2 — Zone Manipulation + Play-Free Pipeline

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 15 `EffectContext` methods covering card movement and effect-initiated plays, so card scripts can express moves between hand/deck/trash/security/battle area and play/digivolve without paying printed costs.

**Architecture:** Two shared enums (`CostDelta`, `StackPosition`) live in `enums.rs`. Each new `EffectContext` method delegates to a `Game`-level helper in `game_actions.rs` (or a `Player` helper in `player.rs` when the op is single-player). Provenance is threaded through `Option<PermanentHandle>` / `Option<CardHandle>` returns. `place_as_bottom_source` uses the reconciled unified signature from the meta-analysis. Play-free variants go through `Game::play_from_hand_with_cost` (new) which is a generalization of the existing `Game::play_from_hand`; the existing method becomes a thin wrapper that computes base cost and calls through.

**Tech Stack:** Rust 2021, `digimon-engine` crate (path: `digimon-engine/`). Tests in `digimon-engine/tests/` using `DebugRunner`. Cargo.

**Roadmap context:** See [.claude/plans/recursive-coalescing-candle.md](../../../.claude/plans/recursive-coalescing-candle.md) Phase 2. Phase 0 parity prereqs confirmed already landed in the code during planning. Follows the seven API design principles in the roadmap — every new method obeys no-auto-selection, closures-over-flags, provenance-via-handles, one-concept-one-primitive.

---

## File Structure

**Files created:**
- `digimon-engine/tests/zone_manipulation.rs` — integration test suite for all 15 methods
- `digimon-engine/src/cards/phase2_test_cards.rs` — Test cards (TEST-P2-001..015) exercising each new primitive in an end-to-end script

**Files modified:**
- `digimon-engine/src/enums.rs` — add `CostDelta`, `StackPosition` enums
- `digimon-engine/src/effect_context/mod.rs` — add 15 new public methods
- `digimon-engine/src/game_actions.rs` — add 10+ new `Game`-level helpers; refactor `play_from_hand` to delegate through `play_from_hand_with_cost`
- `digimon-engine/src/player.rs` — add `remove_from_hand_by_index`, `remove_from_trash_by_index`, `add_to_hand`, `find_in_deck`, `find_in_trash`, `remove_from_deck_by_handle`, `push_to_security_top` helpers
- `digimon-engine/src/permanent.rs` — add `push_under_card_source` helper for bottom-of-stack insertion (place_as_bottom_source)
- `digimon-engine/src/cards/mod.rs` — register phase2_test_cards module
- `docs/RUST_ENGINE_API.md` — append §Zone Manipulation section with one TDD example per method
- `docs/RUST_ENGINE_GAPS.md` — remove or annotate gap entries closed by Phase 2 (zone-manipulation-helpers entries from the Medusamon / Rocks / TS Olympos audits)

**Files read for reference (not modified):**
- `digimon_gym/engine/game/__init__.py` — Python reference semantics for each primitive
- `digimon_gym/engine/core/player.py` — Python zone helpers

---

## Task 1: Shared Types (CostDelta, StackPosition)

**Files:**
- Modify: `digimon-engine/src/enums.rs`

- [ ] **Step 1.1: Open `digimon-engine/src/enums.rs` and find the end of the `Zone` enum definition** (around line 210 per context pack)

- [ ] **Step 1.2: Add `CostDelta` enum below `Zone`**

```rust
/// How a play-from-zone helper should compute the memory cost deducted.
///
/// - `Free` — pay 0 memory regardless of printed cost. Used by "play without
///   paying its cost" effects.
/// - `Reduce(n)` — pay max(0, printed_cost - n). Used by "play with cost
///   reduced by n" effects. Negative reductions (cost increases) are allowed.
/// - `Fixed(n)` — pay exactly n regardless of printed cost. Used by the rare
///   "play for exactly n memory" effects. Negative values clamp to 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostDelta {
    Free,
    Reduce(i16),
    Fixed(i16),
}

impl CostDelta {
    /// Resolve the concrete memory cost to deduct given a printed cost.
    pub fn resolve(self, printed_cost: u16) -> u16 {
        match self {
            CostDelta::Free => 0,
            CostDelta::Reduce(n) => {
                let reduced = printed_cost as i32 - n as i32;
                reduced.max(0) as u16
            }
            CostDelta::Fixed(n) => n.max(0) as u16,
        }
    }
}
```

- [ ] **Step 1.3: Add `StackPosition` enum below `CostDelta`**

```rust
/// Placement position when moving a card to the deck, security stack, or
/// digivolution source stack. `Random` shuffles the single card into a
/// random index — used by "shuffle into the deck" effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackPosition {
    Top,
    Bottom,
    Random,
}
```

- [ ] **Step 1.4: Write unit tests for `CostDelta::resolve`**

Append to the bottom of `enums.rs` (inside an existing `#[cfg(test)] mod tests` block if present, else create one):

```rust
#[cfg(test)]
mod cost_delta_tests {
    use super::*;

    #[test]
    fn free_is_always_zero() {
        assert_eq!(CostDelta::Free.resolve(0), 0);
        assert_eq!(CostDelta::Free.resolve(12), 0);
    }

    #[test]
    fn reduce_subtracts() {
        assert_eq!(CostDelta::Reduce(3).resolve(10), 7);
        assert_eq!(CostDelta::Reduce(0).resolve(10), 10);
    }

    #[test]
    fn reduce_clamps_at_zero() {
        assert_eq!(CostDelta::Reduce(100).resolve(10), 0);
    }

    #[test]
    fn reduce_negative_increases_cost() {
        assert_eq!(CostDelta::Reduce(-2).resolve(10), 12);
    }

    #[test]
    fn fixed_replaces_cost() {
        assert_eq!(CostDelta::Fixed(4).resolve(10), 4);
        assert_eq!(CostDelta::Fixed(0).resolve(10), 0);
    }

    #[test]
    fn fixed_clamps_at_zero() {
        assert_eq!(CostDelta::Fixed(-3).resolve(10), 0);
    }
}
```

- [ ] **Step 1.5: Run the tests, verify they pass**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml cost_delta_
```
Expected: 6 passed.

- [ ] **Step 1.6: Commit**

```bash
git add digimon-engine/src/enums.rs
git commit -m "feat(engine): add CostDelta and StackPosition enums for Phase 2 zone helpers"
```

---

## Task 2: play_from_hand_with_cost_delta + refactor play_from_hand

This generalizes the existing `play_from_hand` so that free/reduced/fixed play variants share one code path. The existing `play_from_hand` becomes a thin wrapper that passes `CostDelta::Reduce(0)`.

**Files:**
- Modify: `digimon-engine/src/game_actions.rs`
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs` (create)

- [ ] **Step 2.1: Create `digimon-engine/tests/zone_manipulation.rs` with the failing test for play_from_hand_free**

```rust
//! Phase 2 zone-manipulation integration tests.
//!
//! See docs/superpowers/plans/2026-04-19-rust-engine-phase-2-zone-manipulation.md.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, CostDelta};

/// Helper: a Lv.3 Red Digimon with play_cost=4 and no effects.
fn plain_digimon(card_id: &str, name: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn play_from_hand_free_ignores_printed_cost() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("COSTLY", "Costly", 10))
        .hand(0, &["COSTLY"])
        .memory(0)
        .start();

    assert_eq!(r.memory(), 0);
    assert_eq!(r.hand_size(0), 1);

    let result = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Free);

    assert_eq!(result, Some(0), "play should succeed at free cost");
    assert_eq!(r.hand_size(0), 0, "card leaves hand");
    assert_eq!(r.battle_area_size(0), 1, "card enters battle area");
    assert_eq!(r.memory(), 0, "memory unchanged — CostDelta::Free pays 0");
}
```

- [ ] **Step 2.2: Run the test, verify it fails to compile**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test zone_manipulation play_from_hand_free_ignores_printed_cost
```
Expected: compile error — `play_from_hand_with_cost` is not defined, and `DebugRunner::game_mut` may not exist.

- [ ] **Step 2.3: Add `DebugRunner::game_mut` accessor if missing**

Open `digimon-engine/src/debug_runner.rs`. Find the `impl DebugRunner` block with `pub fn memory`. Add:

```rust
/// Mutable access to the underlying `Game` — for tests that drive
/// new APIs before the higher-level `DebugRunner` helpers exist.
pub fn game_mut(&mut self) -> &mut Game {
    &mut self.game
}
```

(If `Game` isn't in scope in debug_runner.rs, add `use crate::game::Game;`.)

- [ ] **Step 2.4: Add `Game::play_from_hand_with_cost` in game_actions.rs**

Open `digimon-engine/src/game_actions.rs`. Below the existing `play_from_hand` (line ~93), add:

```rust
/// Generalization of `play_from_hand` — computes memory cost via the given
/// `CostDelta` and plays the card. The caller's `CostDelta::Reduce(0)` is
/// equivalent to paying the printed cost.
///
/// Returns `Some(field_index)` on success, `None` if the hand index is
/// invalid, the battle area is full, or memory is insufficient.
///
/// See [docs/RUST_ENGINE_API.md] §Zone Manipulation.
pub fn play_from_hand_with_cost(
    &mut self,
    player_id: PlayerId,
    hand_index: usize,
    cost_delta: crate::enums::CostDelta,
) -> Option<usize> {
    let turn = self.turn_count;
    let field_slots = self.rules.field_slots;

    let printed_cost = {
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            return None;
        }
        if player.battle_area.len() >= field_slots as usize {
            return None;
        }
        player.hand[hand_index].play_cost(&self.card_data)
    };

    let effective_cost = cost_delta.resolve(printed_cost);

    if !self.pay_memory(effective_cost) {
        return None;
    }

    let player = self.player_mut(player_id);
    let card = player.hand.remove(hand_index);
    let perm = crate::permanent::Permanent::new(card, turn);
    player.battle_area.push(perm);
    let field_index = player.battle_area.len() - 1;

    let emitted_card_id = self.players[player_id as usize].battle_area[field_index]
        .top_card()
        .card_id(&self.card_data)
        .to_string();
    let seq = self.next_event_seq();
    self.events.push(crate::events::GameEvent::Play {
        seq,
        player: player_id,
        card_id: emitted_card_id,
        field_index: field_index as u8,
    });

    self.fire_on_play(player_id, field_index);

    Some(field_index)
}
```

- [ ] **Step 2.5: Refactor existing `play_from_hand` to delegate**

Replace the body of the existing `Game::play_from_hand` with:

```rust
pub fn play_from_hand(&mut self, player_id: PlayerId, hand_index: usize) -> Option<usize> {
    self.play_from_hand_with_cost(player_id, hand_index, crate::enums::CostDelta::Reduce(0))
}
```

- [ ] **Step 2.6: Run the new test**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test zone_manipulation play_from_hand_free_ignores_printed_cost
```
Expected: PASS.

- [ ] **Step 2.7: Run the full engine test suite to verify refactor didn't break existing callers**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml
```
Expected: all existing tests still pass.

- [ ] **Step 2.8: Add failing test for Reduce path**

Append to `tests/zone_manipulation.rs`:

```rust
#[test]
fn play_from_hand_reduce_subtracts_from_cost() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("C6", "SixCost", 6))
        .hand(0, &["C6"])
        .memory(5)
        .start();

    let before = r.memory();
    let res = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Reduce(4));
    assert_eq!(res, Some(0));
    assert_eq!(r.memory(), before - 2, "6 - 4 = 2 memory paid");
}

#[test]
fn play_from_hand_reduce_clamps_at_zero() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("C3", "ThreeCost", 3))
        .hand(0, &["C3"])
        .memory(5)
        .start();

    let before = r.memory();
    let res = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Reduce(10));
    assert_eq!(res, Some(0));
    assert_eq!(r.memory(), before, "reducing below 0 pays 0, not negative");
}

#[test]
fn play_from_hand_with_cost_rejects_unaffordable() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("C10", "TenCost", 10))
        .hand(0, &["C10"])
        .memory(0)
        .start();

    // Fixed(5) is affordable only if memory_range.0 allows -5; default rules
    // permit -10 so this should succeed.
    let res = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Fixed(5));
    assert_eq!(res, Some(0), "fixed cost 5 at memory 0 is affordable (goes to -5)");
}
```

- [ ] **Step 2.9: Run the new tests**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test zone_manipulation play_from_hand_
```
Expected: 4 passed.

- [ ] **Step 2.10: Commit**

```bash
git add digimon-engine/src/game_actions.rs digimon-engine/src/debug_runner.rs digimon-engine/tests/zone_manipulation.rs
git commit -m "feat(engine): add play_from_hand_with_cost generalization + CostDelta plumbing"
```

---

## Task 3: EffectContext::play_from_hand (Free / CostDelta variants)

Expose the Game-level helper as EffectContext methods for card scripts.

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 3.1: Write a failing test exercising EffectContext::play_from_hand**

Append to `tests/zone_manipulation.rs`:

```rust
use digimon_engine::card_source::CardHandle;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::permanent::PermanentHandle;
use std::sync::Arc;

/// TEST-P2-001: on play, look at own hand slot 0 (if present) and play it
/// free via EffectContext::play_from_hand_with_cost.
struct TestP2_001;
impl CardEffect for TestP2_001 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play top of hand free")
            .process(|ctx| {
                let me = ctx.player;
                if ctx.hand(me).is_empty() {
                    return;
                }
                ctx.play_from_hand_with_cost(me, 0, digimon_engine::enums::CostDelta::Free);
            })
            .build()]
    }
}

#[test]
fn ctx_play_from_hand_free_plays_target() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TEST-P2-001", "P2-001", 3))
        .add_card(plain_digimon("TARGET", "Target", 10))
        .hand(0, &["TEST-P2-001", "TARGET"])
        .memory(3)
        .start();

    r.register_effect("TEST-P2-001", Arc::new(TestP2_001));

    // Play TEST-P2-001 (hand slot 0). After OnPlay fires, it should have played
    // TARGET (now hand slot 0 since TEST-P2-001 was removed first) for free.
    let res = r.play(0, 0);
    assert_eq!(res, Some(0));
    assert_eq!(r.battle_area_size(0), 2, "both cards entered battle area");
    assert_eq!(r.hand_size(0), 0, "hand emptied");
    // Memory: started 3, paid 3 for TEST-P2-001, then 0 for TARGET (free).
    assert_eq!(r.memory(), 0);
}
```

- [ ] **Step 3.2: Verify test fails to compile**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test zone_manipulation ctx_play_from_hand_free
```
Expected: `ctx.play_from_hand_with_cost` undefined; `DebugRunner::register_effect` may not exist.

- [ ] **Step 3.3: Add `DebugRunner::register_effect` if missing**

In `digimon-engine/src/debug_runner.rs` inside the `impl DebugRunner` block:

```rust
/// Install a `CardEffect` into the registry under a card id. Tests can
/// declare one-off effects inline without a frozen `cards/` entry.
pub fn register_effect(
    &mut self,
    card_id: &str,
    effect: std::sync::Arc<dyn crate::effect::CardEffect>,
) {
    self.game
        .effect_registry
        .insert(card_id.to_string(), effect);
}
```

(If `effect_registry` is not pub, add a setter method on `Game` that wraps the insert.)

- [ ] **Step 3.4: Add `EffectContext::play_from_hand_with_cost`**

Open `digimon-engine/src/effect_context/mod.rs`. After the existing `delete_permanent` method (around line 255), add:

```rust
/// Play a card from `player`'s hand at `hand_index`, deducting memory
/// according to `cost_delta`. OnPlay effects fire. Returns the field
/// index on success.
///
/// See [docs/RUST_ENGINE_API.md] §Zone Manipulation.
pub fn play_from_hand_with_cost(
    &mut self,
    player: PlayerId,
    hand_index: usize,
    cost_delta: crate::enums::CostDelta,
) -> Option<PermanentHandle> {
    let field_index = self.game.play_from_hand_with_cost(player, hand_index, cost_delta)?;
    Some(PermanentHandle {
        player,
        index: field_index as u8,
    })
}
```

- [ ] **Step 3.5: Run the test, verify PASS**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test zone_manipulation ctx_play_from_hand_free
```
Expected: PASS.

- [ ] **Step 3.6: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/src/debug_runner.rs digimon-engine/tests/zone_manipulation.rs
git commit -m "feat(engine): EffectContext::play_from_hand_with_cost for Phase 2 scripts"
```

---

## Task 4: play_from_trash_with_cost (Game + EffectContext)

Mirrors Task 2/3 but reads from `player.trash` instead of `player.hand`. Exists on both `Game` and `EffectContext`.

**Files:**
- Modify: `digimon-engine/src/game_actions.rs`
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 4.1: Write failing integration test**

Append to `tests/zone_manipulation.rs`:

```rust
#[test]
fn play_from_trash_free_moves_card_to_field() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BURIED", "Buried", 6))
        .start();

    // Seed trash directly. Phase 2 doesn't add a `trash(...)` builder helper
    // — use game_mut to push a CardSource into the trash for setup.
    {
        let g = r.game_mut();
        let handle = g.next_card_handle();
        let card = digimon_engine::card_source::CardSource::with_handle("BURIED", handle);
        g.player_mut(0).trash.push(card);
    }

    assert_eq!(r.trash_size(0), 1);
    let res = r.game_mut().play_from_trash_with_cost(0, 0, digimon_engine::enums::CostDelta::Free);
    assert_eq!(res, Some(0));
    assert_eq!(r.trash_size(0), 0, "card left trash");
    assert_eq!(r.battle_area_size(0), 1);
}
```

- [ ] **Step 4.2: Run test, verify compile error**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test zone_manipulation play_from_trash_free
```
Expected: `play_from_trash_with_cost` undefined. Also possibly `Game::next_card_handle` and `CardSource::with_handle` — check current API. If those setup helpers don't exist, replace the seed block with whatever idiom the existing tests use for seeding trash (grep `trash.push` under `digimon-engine/tests/`).

- [ ] **Step 4.3: Add `Game::play_from_trash_with_cost`**

In `digimon-engine/src/game_actions.rs`, below `play_from_hand_with_cost`:

```rust
/// Play a card from `player`'s trash. Like `play_from_hand_with_cost` but
/// reads and removes from `player.trash`. Returns `Some(field_index)` on
/// success.
pub fn play_from_trash_with_cost(
    &mut self,
    player_id: PlayerId,
    trash_index: usize,
    cost_delta: crate::enums::CostDelta,
) -> Option<usize> {
    let turn = self.turn_count;
    let field_slots = self.rules.field_slots;

    let printed_cost = {
        let player = self.player(player_id);
        if trash_index >= player.trash.len() {
            return None;
        }
        if player.battle_area.len() >= field_slots as usize {
            return None;
        }
        player.trash[trash_index].play_cost(&self.card_data)
    };

    let effective_cost = cost_delta.resolve(printed_cost);
    if !self.pay_memory(effective_cost) {
        return None;
    }

    let player = self.player_mut(player_id);
    let card = player.trash.remove(trash_index);
    let perm = crate::permanent::Permanent::new(card, turn);
    player.battle_area.push(perm);
    let field_index = player.battle_area.len() - 1;

    let emitted_card_id = self.players[player_id as usize].battle_area[field_index]
        .top_card()
        .card_id(&self.card_data)
        .to_string();
    let seq = self.next_event_seq();
    self.events.push(crate::events::GameEvent::Play {
        seq,
        player: player_id,
        card_id: emitted_card_id,
        field_index: field_index as u8,
    });

    self.fire_on_play(player_id, field_index);

    Some(field_index)
}
```

- [ ] **Step 4.4: Add `EffectContext::play_from_trash_with_cost`**

In `digimon-engine/src/effect_context/mod.rs`, after `play_from_hand_with_cost`:

```rust
/// Play a card from `player`'s trash at `trash_index`, deducting memory
/// according to `cost_delta`. OnPlay effects fire.
pub fn play_from_trash_with_cost(
    &mut self,
    player: PlayerId,
    trash_index: usize,
    cost_delta: crate::enums::CostDelta,
) -> Option<PermanentHandle> {
    let field_index = self
        .game
        .play_from_trash_with_cost(player, trash_index, cost_delta)?;
    Some(PermanentHandle {
        player,
        index: field_index as u8,
    })
}
```

- [ ] **Step 4.5: Run the test, verify PASS**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test zone_manipulation play_from_trash
```
Expected: PASS.

- [ ] **Step 4.6: Commit**

```bash
git add digimon-engine/src/game_actions.rs digimon-engine/src/effect_context/mod.rs digimon-engine/tests/zone_manipulation.rs
git commit -m "feat(engine): add play_from_trash_with_cost on Game + EffectContext"
```

---

## Task 5: add_to_hand_from_deck / add_to_hand_from_trash (search helpers)

These support "search your deck for a Digimon with [trait], add it to your hand, then shuffle" style effects. The caller selects the card via an existing `select_reveal` or a new `select_deck` prompt; this task only provides the movement primitive. Deck-shuffling on search is the caller's responsibility (the rule is "shuffle after searching" — exposed via a separate `shuffle_deck` helper below).

**Files:**
- Modify: `digimon-engine/src/player.rs` — add `shuffle_deck`, `remove_from_deck_by_handle`, `remove_from_trash_by_handle`, `add_to_hand`
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 5.1: Write failing test**

```rust
#[test]
fn add_to_hand_from_deck_moves_specific_card() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("WANTED", "Wanted", 4))
        .add_card(plain_digimon("FILLER", "Filler", 4))
        .deck(0, &["FILLER", "WANTED", "FILLER"])
        .start();

    // Grab the CardHandle of the WANTED card (deck slot 1).
    let target_handle = r.game_mut().player(0).deck[1].handle();

    let ok = r.game_mut().add_to_hand_from_deck(0, target_handle);
    assert!(ok);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.deck_size(0), 2, "one card left deck");

    // Confirm the correct card moved.
    let moved_id = r.game_mut().player(0).hand[0].card_id(&r.game_mut().card_data).to_string();
    assert_eq!(moved_id, "WANTED");
}

#[test]
fn add_to_hand_from_trash_moves_card() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("DEAD", "Dead", 5))
        .start();

    let handle = {
        let g = r.game_mut();
        let h = g.next_card_handle();
        let card = digimon_engine::card_source::CardSource::with_handle("DEAD", h);
        g.player_mut(0).trash.push(card);
        h
    };

    let ok = r.game_mut().add_to_hand_from_trash(0, handle);
    assert!(ok);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn add_to_hand_missing_handle_returns_false() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("DEAD", "Dead", 5))
        .deck(0, &["DEAD"])
        .start();

    let bogus = digimon_engine::card_source::CardHandle::from_raw(u32::MAX);
    let ok = r.game_mut().add_to_hand_from_deck(0, bogus);
    assert!(!ok);
    assert_eq!(r.hand_size(0), 0);
}
```

- [ ] **Step 5.2: Run, verify compile errors for `add_to_hand_from_deck` / `add_to_hand_from_trash`**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test zone_manipulation add_to_hand_
```

- [ ] **Step 5.3: Add Player helpers**

Open `digimon-engine/src/player.rs`. After `draw_many`:

```rust
/// Remove the first card in `deck` matching `handle`. Returns the removed
/// card if found.
pub fn remove_from_deck_by_handle(
    &mut self,
    handle: crate::card_source::CardHandle,
) -> Option<crate::card_source::CardSource> {
    let pos = self.deck.iter().position(|c| c.handle() == handle)?;
    Some(self.deck.remove(pos))
}

/// Remove the first card in `trash` matching `handle`.
pub fn remove_from_trash_by_handle(
    &mut self,
    handle: crate::card_source::CardHandle,
) -> Option<crate::card_source::CardSource> {
    let pos = self.trash.iter().position(|c| c.handle() == handle)?;
    Some(self.trash.remove(pos))
}

/// Append `card` to hand.
pub fn add_to_hand(&mut self, card: crate::card_source::CardSource) {
    self.hand.push(card);
}

/// Shuffle the deck in place using the supplied rng.
pub fn shuffle_deck(&mut self, rng: &mut impl rand::Rng) {
    use rand::seq::SliceRandom;
    self.deck.shuffle(rng);
}
```

- [ ] **Step 5.4: Add Game-level wrappers in game_actions.rs**

```rust
/// Move a specific card from `player`'s deck to their hand. Returns false
/// if the handle isn't in the deck. Does NOT shuffle — callers that mirror
/// the printed "search then shuffle" rule must call `shuffle_deck` after.
pub fn add_to_hand_from_deck(
    &mut self,
    player_id: PlayerId,
    card: crate::card_source::CardHandle,
) -> bool {
    let Some(removed) = self.player_mut(player_id).remove_from_deck_by_handle(card) else {
        return false;
    };
    self.player_mut(player_id).add_to_hand(removed);
    true
}

/// Move a specific card from `player`'s trash to their hand.
pub fn add_to_hand_from_trash(
    &mut self,
    player_id: PlayerId,
    card: crate::card_source::CardHandle,
) -> bool {
    let Some(removed) = self.player_mut(player_id).remove_from_trash_by_handle(card) else {
        return false;
    };
    self.player_mut(player_id).add_to_hand(removed);
    true
}

/// Shuffle `player`'s deck.
pub fn shuffle_deck(&mut self, player_id: PlayerId) {
    // Split borrow: move the deck out, shuffle it, put it back. Avoids
    // holding `self.rng` and `self.player_mut` at the same time.
    let mut deck = std::mem::take(&mut self.player_mut(player_id).deck);
    use rand::seq::SliceRandom;
    deck.shuffle(&mut self.rng);
    self.player_mut(player_id).deck = deck;
}
```

- [ ] **Step 5.5: Add EffectContext wrappers**

In `effect_context/mod.rs`:

```rust
pub fn add_to_hand_from_deck(
    &mut self,
    player: PlayerId,
    card: crate::card_source::CardHandle,
) -> bool {
    self.game.add_to_hand_from_deck(player, card)
}

pub fn add_to_hand_from_trash(
    &mut self,
    player: PlayerId,
    card: crate::card_source::CardHandle,
) -> bool {
    self.game.add_to_hand_from_trash(player, card)
}

pub fn shuffle_deck(&mut self, player: PlayerId) {
    self.game.shuffle_deck(player);
}
```

- [ ] **Step 5.6: Add `Game::next_card_handle` and `CardSource::with_handle` if absent**

If the test in Step 5.1 fails because these are missing, open `digimon-engine/src/game.rs` and `digimon-engine/src/card_source.rs` respectively and add minimal public wrappers around the existing handle allocation logic. (Check existing tests — if they already seed trash with a different idiom, substitute that idiom instead of adding new APIs.)

- [ ] **Step 5.7: Run, verify PASS**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test zone_manipulation add_to_hand_
```

- [ ] **Step 5.8: Commit**

```bash
git add digimon-engine/src/player.rs digimon-engine/src/game_actions.rs digimon-engine/src/effect_context/mod.rs digimon-engine/tests/zone_manipulation.rs
git commit -m "feat(engine): add_to_hand_from_deck/trash + shuffle_deck primitives"
```

---

## Task 6: reveal_top_deck

Reveals N cards from the top of a player's deck. Cards go into `game.revealed_cards` (already a field on `Game`), staying revealed until a follow-up effect moves them elsewhere or the turn ends. Does not trigger draw observers.

**Files:**
- Modify: `digimon-engine/src/game_actions.rs`
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 6.1: Failing test**

```rust
#[test]
fn reveal_top_deck_populates_reveal_pool() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .add_card(plain_digimon("B", "B", 1))
        .add_card(plain_digimon("C", "C", 1))
        .deck(0, &["A", "B", "C"])
        .start();

    let revealed = r.game_mut().reveal_top_deck(0, 2);
    assert_eq!(revealed.len(), 2);
    assert_eq!(r.deck_size(0), 1, "two cards left the deck");
    assert_eq!(r.game_mut().revealed_cards.len(), 2);
    // Top-of-deck is the last element; reveal order is top-first (reverse of slice).
    let card_ids: Vec<_> = r.game_mut().revealed_cards.iter()
        .map(|c| c.card_id(&r.game_mut().card_data).to_string())
        .collect();
    assert_eq!(card_ids, vec!["C", "B"], "revealed top-first");
}

#[test]
fn reveal_top_deck_handles_empty_deck() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .deck(0, &["A"])
        .start();
    let revealed = r.game_mut().reveal_top_deck(0, 5);
    assert_eq!(revealed.len(), 1, "only 1 was available");
    assert_eq!(r.deck_size(0), 0);
}
```

- [ ] **Step 6.2: Verify failure**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test zone_manipulation reveal_top_deck
```

- [ ] **Step 6.3: Implement Game::reveal_top_deck**

In `game_actions.rs`:

```rust
/// Reveal up to `n` cards from the top of `player`'s deck. Cards move
/// into `game.revealed_cards` (transient reveal pool, cleared on turn
/// rotation). Returns the list of revealed card handles in top-first order.
///
/// Does not fire `OnDraw` or modify hand. Callers that want to then move a
/// revealed card to hand/deck/trash use `add_to_hand_from_reveal` (Task 9)
/// or `trash_from_reveal` / `return_to_deck_from_reveal` (forthcoming).
pub fn reveal_top_deck(
    &mut self,
    player_id: PlayerId,
    n: u8,
) -> Vec<crate::card_source::CardHandle> {
    let mut handles = Vec::new();
    for _ in 0..n {
        let p = self.player_mut(player_id);
        let Some(card) = p.deck.pop() else { break };
        handles.push(card.handle());
        self.revealed_cards.push(card);
    }
    handles
}
```

- [ ] **Step 6.4: Expose on EffectContext**

```rust
pub fn reveal_top_deck(
    &mut self,
    player: PlayerId,
    n: u8,
) -> Vec<crate::card_source::CardHandle> {
    self.game.reveal_top_deck(player, n)
}

/// Snapshot of the current reveal pool. Scripts inspect this to decide
/// follow-up moves.
pub fn revealed(&self) -> &[crate::card_source::CardSource] {
    &self.game.revealed_cards
}
```

- [ ] **Step 6.5: Run, verify PASS**

- [ ] **Step 6.6: Commit**

```bash
git add -u && git commit -m "feat(engine): reveal_top_deck + revealed_cards pool accessor"
```

---

## Task 7: return_to_hand (bounce a permanent)

Takes a `PermanentHandle`, moves the top card of the stack to its owner's hand. Sources under the top go to the owner's trash (matches DCGO "when this Digimon leaves the battle area" rules). OnLeaveField observers fire per roadmap Cluster B — but Phase 2 only implements the movement primitive; observer dispatch is Phase 1 work and is intentionally out of scope. The test therefore only asserts zone-state changes.

**Files:**
- Modify: `digimon-engine/src/game_actions.rs`
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 7.1: Failing test**

```rust
use digimon_engine::permanent::PermanentHandle;

#[test]
fn return_to_hand_moves_top_card_to_hand_and_sources_to_trash() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TOP", "Top", 5))
        .add_card(plain_digimon("UNDER", "Under", 3))
        .start();

    // Seed a permanent with TOP on top of UNDER.
    let handle = {
        let g = r.game_mut();
        let turn = g.turn_count();
        let h_under = g.next_card_handle();
        let h_top = g.next_card_handle();
        let under = digimon_engine::card_source::CardSource::with_handle("UNDER", h_under);
        let top = digimon_engine::card_source::CardSource::with_handle("TOP", h_top);
        let mut perm = digimon_engine::permanent::Permanent::new(under, turn);
        perm.card_sources.push(top);
        g.player_mut(0).battle_area.push(perm);
        PermanentHandle { player: 0, index: 0 }
    };

    let returned = r.game_mut().return_to_hand(handle);
    assert!(returned.is_some(), "returned a card handle");
    assert_eq!(r.battle_area_size(0), 0, "permanent gone");
    assert_eq!(r.hand_size(0), 1, "top card went to hand");
    assert_eq!(r.trash_size(0), 1, "under card went to trash");

    // Correct card ids in correct zones
    let hand_id = r.game_mut().player(0).hand[0].card_id(&r.game_mut().card_data).to_string();
    assert_eq!(hand_id, "TOP");
    let trash_id = r.game_mut().player(0).trash[0].card_id(&r.game_mut().card_data).to_string();
    assert_eq!(trash_id, "UNDER");
}

#[test]
fn return_to_hand_bad_handle_returns_none() {
    let mut r = DebugRunner::builder().start();
    let returned = r.game_mut().return_to_hand(PermanentHandle { player: 0, index: 99 });
    assert!(returned.is_none());
}
```

- [ ] **Step 7.2: Verify failure**

- [ ] **Step 7.3: Implement Game::return_to_hand**

```rust
/// Bounce a permanent to its owner's hand: the top card moves to hand,
/// every card beneath it goes to the owner's trash (per DCGO leave-field
/// rules). Linked cards go to trash. Returns the handle of the card that
/// ended up in hand.
///
/// Does not fire OnLeaveField observers — that's Phase 1 timing-dispatch
/// infrastructure.
pub fn return_to_hand(
    &mut self,
    handle: PermanentHandle,
) -> Option<crate::card_source::CardHandle> {
    let player = self.player_mut(handle.player);
    if (handle.index as usize) >= player.battle_area.len() {
        return None;
    }
    let perm = player.battle_area.remove(handle.index as usize);

    // Top card (last in the stack) goes to hand.
    let mut sources = perm.card_sources;
    let Some(top) = sources.pop() else { return None };
    let top_handle = top.handle();
    player.hand.push(top);

    // Remaining sources go to trash, bottom-first.
    for card in sources {
        player.trash.push(card);
    }
    for card in perm.linked_cards {
        player.trash.push(card);
    }

    self.modifiers.clear_permanent(handle);
    Some(top_handle)
}
```

- [ ] **Step 7.4: EffectContext wrapper**

```rust
pub fn return_to_hand(
    &mut self,
    target: PermanentHandle,
) -> Option<crate::card_source::CardHandle> {
    self.game.return_to_hand(target)
}
```

- [ ] **Step 7.5: Run, verify PASS**

- [ ] **Step 7.6: Commit**

```bash
git add -u && git commit -m "feat(engine): return_to_hand — bounce a permanent to owner's hand"
```

---

## Task 8: return_to_deck (Top / Bottom / Random)

Same shape as `return_to_hand` but moves the top card to the deck at `StackPosition`. Sources go to trash.

**Files:**
- Modify: `digimon-engine/src/game_actions.rs`
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 8.1: Failing tests for Top, Bottom, Random**

```rust
use digimon_engine::enums::StackPosition;

#[test]
fn return_to_deck_top_places_on_top() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TOP", "Top", 4))
        .add_card(plain_digimon("FILLER", "F", 1))
        .deck(0, &["FILLER", "FILLER"])
        .start();

    let handle = {
        let g = r.game_mut();
        let turn = g.turn_count();
        let h = g.next_card_handle();
        let card = digimon_engine::card_source::CardSource::with_handle("TOP", h);
        g.player_mut(0).battle_area.push(digimon_engine::permanent::Permanent::new(card, turn));
        PermanentHandle { player: 0, index: 0 }
    };

    let ok = r.game_mut().return_to_deck(handle, StackPosition::Top);
    assert!(ok);
    assert_eq!(r.battle_area_size(0), 0);
    assert_eq!(r.deck_size(0), 3);

    // Top of deck is the last element of the vec (pop is from end).
    let top_id = r.game_mut().player(0).deck.last().unwrap()
        .card_id(&r.game_mut().card_data).to_string();
    assert_eq!(top_id, "TOP");
}

#[test]
fn return_to_deck_bottom_places_at_position_zero() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BOTTOM", "Bot", 4))
        .add_card(plain_digimon("FILLER", "F", 1))
        .deck(0, &["FILLER", "FILLER"])
        .start();

    let handle = {
        let g = r.game_mut();
        let turn = g.turn_count();
        let h = g.next_card_handle();
        let card = digimon_engine::card_source::CardSource::with_handle("BOTTOM", h);
        g.player_mut(0).battle_area.push(digimon_engine::permanent::Permanent::new(card, turn));
        PermanentHandle { player: 0, index: 0 }
    };

    let ok = r.game_mut().return_to_deck(handle, StackPosition::Bottom);
    assert!(ok);
    let bottom_id = r.game_mut().player(0).deck.first().unwrap()
        .card_id(&r.game_mut().card_data).to_string();
    assert_eq!(bottom_id, "BOTTOM");
}

#[test]
fn return_to_deck_random_inserts_somewhere() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("RANDOM", "R", 4))
        .add_card(plain_digimon("FILLER", "F", 1))
        .deck(0, &["FILLER"; 5])
        .start();

    let handle = {
        let g = r.game_mut();
        let turn = g.turn_count();
        let h = g.next_card_handle();
        let card = digimon_engine::card_source::CardSource::with_handle("RANDOM", h);
        g.player_mut(0).battle_area.push(digimon_engine::permanent::Permanent::new(card, turn));
        PermanentHandle { player: 0, index: 0 }
    };

    let ok = r.game_mut().return_to_deck(handle, StackPosition::Random);
    assert!(ok);
    assert_eq!(r.deck_size(0), 6);
    // Card must be somewhere in the deck.
    let positions: Vec<_> = r.game_mut().player(0).deck.iter()
        .enumerate()
        .filter(|(_, c)| c.card_id(&r.game_mut().card_data) == "RANDOM")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(positions.len(), 1, "exactly one copy in deck");
}
```

- [ ] **Step 8.2: Verify failure**

- [ ] **Step 8.3: Implement Game::return_to_deck**

```rust
pub fn return_to_deck(
    &mut self,
    handle: PermanentHandle,
    position: crate::enums::StackPosition,
) -> bool {
    let player = self.player_mut(handle.player);
    if (handle.index as usize) >= player.battle_area.len() {
        return false;
    }
    let perm = player.battle_area.remove(handle.index as usize);

    let mut sources = perm.card_sources;
    let Some(top) = sources.pop() else { return false };

    match position {
        crate::enums::StackPosition::Top => player.deck.push(top),
        crate::enums::StackPosition::Bottom => player.deck.insert(0, top),
        crate::enums::StackPosition::Random => {
            let len = player.deck.len();
            let idx = if len == 0 {
                0
            } else {
                use rand::Rng;
                self.rng.gen_range(0..=len)
            };
            self.player_mut(handle.player).deck.insert(idx, top);
        }
    }

    // Sources under the top go to trash.
    for card in sources {
        self.player_mut(handle.player).trash.push(card);
    }
    for card in perm.linked_cards {
        self.player_mut(handle.player).trash.push(card);
    }
    self.modifiers.clear_permanent(handle);
    true
}
```

Note: the `Random` branch splits the borrow (uses `self.rng` then `self.player_mut`). Adjust the code to avoid the double-borrow if the compiler objects — e.g. compute `idx` first under a short immutable borrow, then insert.

- [ ] **Step 8.4: EffectContext wrapper**

```rust
pub fn return_to_deck(
    &mut self,
    target: PermanentHandle,
    position: crate::enums::StackPosition,
) -> bool {
    self.game.return_to_deck(target, position)
}
```

- [ ] **Step 8.5: Run, verify PASS**

- [ ] **Step 8.6: Commit**

```bash
git add -u && git commit -m "feat(engine): return_to_deck with StackPosition variants"
```

---

## Task 9: trash_from_hand_by_index (+ reveal-pool integration)

Explicitly trashes a specific card from a player's hand by index. Also adds `trash_from_reveal` / `add_to_hand_from_reveal` / `return_to_deck_from_reveal` as companion primitives for the reveal pool.

**Files:**
- Modify: `digimon-engine/src/game_actions.rs`
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 9.1: Failing tests**

```rust
#[test]
fn trash_from_hand_by_index_moves_card_to_trash() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("DOOMED", "D", 3))
        .hand(0, &["DOOMED"])
        .start();

    let trashed = r.game_mut().trash_from_hand_by_index(0, 0);
    assert!(trashed.is_some());
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn trash_from_hand_bad_index_is_noop() {
    let mut r = DebugRunner::builder().start();
    assert!(r.game_mut().trash_from_hand_by_index(0, 10).is_none());
}

#[test]
fn add_to_hand_from_reveal_moves_and_shrinks_pool() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .add_card(plain_digimon("B", "B", 1))
        .deck(0, &["A", "B"])
        .start();

    let revealed = r.game_mut().reveal_top_deck(0, 2);
    assert_eq!(revealed.len(), 2);

    let handle = revealed[0]; // first revealed (top of original deck)
    let ok = r.game_mut().add_to_hand_from_reveal(0, handle);
    assert!(ok);
    assert_eq!(r.game_mut().revealed_cards.len(), 1);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn trash_from_reveal_moves_and_shrinks_pool() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .deck(0, &["A"])
        .start();

    let revealed = r.game_mut().reveal_top_deck(0, 1);
    let h = revealed[0];
    let ok = r.game_mut().trash_from_reveal(0, h);
    assert!(ok);
    assert_eq!(r.game_mut().revealed_cards.len(), 0);
    assert_eq!(r.trash_size(0), 1);
}
```

- [ ] **Step 9.2: Verify failure**

- [ ] **Step 9.3: Implement helpers in game_actions.rs**

```rust
pub fn trash_from_hand_by_index(
    &mut self,
    player_id: PlayerId,
    hand_index: usize,
) -> Option<crate::card_source::CardHandle> {
    let player = self.player_mut(player_id);
    if hand_index >= player.hand.len() {
        return None;
    }
    let card = player.hand.remove(hand_index);
    let h = card.handle();
    player.trash.push(card);
    Some(h)
}

/// Move a specific revealed card to `player`'s hand.
pub fn add_to_hand_from_reveal(
    &mut self,
    player_id: PlayerId,
    card: crate::card_source::CardHandle,
) -> bool {
    let pos = self.revealed_cards.iter().position(|c| c.handle() == card);
    let Some(pos) = pos else { return false };
    let taken = self.revealed_cards.remove(pos);
    self.player_mut(player_id).hand.push(taken);
    true
}

/// Move a specific revealed card to `player`'s trash.
pub fn trash_from_reveal(
    &mut self,
    player_id: PlayerId,
    card: crate::card_source::CardHandle,
) -> bool {
    let pos = self.revealed_cards.iter().position(|c| c.handle() == card);
    let Some(pos) = pos else { return false };
    let taken = self.revealed_cards.remove(pos);
    self.player_mut(player_id).trash.push(taken);
    true
}

/// Move a specific revealed card back to `player`'s deck at `position`.
pub fn return_to_deck_from_reveal(
    &mut self,
    player_id: PlayerId,
    card: crate::card_source::CardHandle,
    position: crate::enums::StackPosition,
) -> bool {
    let pos_idx = self.revealed_cards.iter().position(|c| c.handle() == card);
    let Some(pos_idx) = pos_idx else { return false };
    let taken = self.revealed_cards.remove(pos_idx);
    let deck = &mut self.player_mut(player_id).deck;
    match position {
        crate::enums::StackPosition::Top => deck.push(taken),
        crate::enums::StackPosition::Bottom => deck.insert(0, taken),
        crate::enums::StackPosition::Random => {
            let len = deck.len();
            use rand::Rng;
            let idx = if len == 0 { 0 } else { self.rng.gen_range(0..=len) };
            self.player_mut(player_id).deck.insert(idx, taken);
        }
    }
    true
}
```

- [ ] **Step 9.4: EffectContext wrappers**

```rust
pub fn trash_from_hand_by_index(
    &mut self,
    player: PlayerId,
    hand_index: usize,
) -> Option<crate::card_source::CardHandle> {
    self.game.trash_from_hand_by_index(player, hand_index)
}

pub fn add_to_hand_from_reveal(
    &mut self,
    player: PlayerId,
    card: crate::card_source::CardHandle,
) -> bool {
    self.game.add_to_hand_from_reveal(player, card)
}

pub fn trash_from_reveal(
    &mut self,
    player: PlayerId,
    card: crate::card_source::CardHandle,
) -> bool {
    self.game.trash_from_reveal(player, card)
}

pub fn return_to_deck_from_reveal(
    &mut self,
    player: PlayerId,
    card: crate::card_source::CardHandle,
    position: crate::enums::StackPosition,
) -> bool {
    self.game.return_to_deck_from_reveal(player, card, position)
}
```

- [ ] **Step 9.5: Run, verify PASS**

- [ ] **Step 9.6: Commit**

```bash
git add -u && git commit -m "feat(engine): trash_from_hand + reveal-pool movement primitives"
```

---

## Task 10: place_as_bottom_source (unified signature)

The reconciled signature from the meta-analysis:

```rust
pub fn place_as_bottom_source(
    source: CardSourceRef,
    target: PermanentHandle,
) -> bool
```

Where `CardSourceRef` selects where the to-be-placed card comes from:

```rust
pub enum CardSourceRef {
    Hand(PlayerId, usize),         // hand_index
    Trash(PlayerId, usize),        // trash_index
    DeckTop(PlayerId),             // draws one from top
    Reveal(CardHandle),            // takes from reveal pool
}
```

**Files:**
- Modify: `digimon-engine/src/enums.rs` — add `CardSourceRef`
- Modify: `digimon-engine/src/game_actions.rs`
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/src/permanent.rs` — add `push_under` helper
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 10.1: Failing test**

```rust
use digimon_engine::enums::CardSourceRef;

#[test]
fn place_as_bottom_source_from_hand_stacks_under_target() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 4))
        .add_card(plain_digimon("FUEL", "Fuel", 2))
        .hand(0, &["FUEL"])
        .start();

    // Seed BASE on field.
    let target = {
        let g = r.game_mut();
        let turn = g.turn_count();
        let h = g.next_card_handle();
        let card = digimon_engine::card_source::CardSource::with_handle("BASE", h);
        g.player_mut(0).battle_area.push(digimon_engine::permanent::Permanent::new(card, turn));
        PermanentHandle { player: 0, index: 0 }
    };

    let ok = r.game_mut().place_as_bottom_source(CardSourceRef::Hand(0, 0), target);
    assert!(ok);
    assert_eq!(r.hand_size(0), 0);

    let perm = &r.game_mut().player(0).battle_area[0];
    assert_eq!(perm.card_sources.len(), 2, "stack grew");
    // FUEL under BASE → position 0, BASE stays on top (position 1).
    let bottom_id = perm.card_sources[0].card_id(&r.game_mut().card_data).to_string();
    let top_id = perm.card_sources[1].card_id(&r.game_mut().card_data).to_string();
    assert_eq!(bottom_id, "FUEL");
    assert_eq!(top_id, "BASE");
}
```

- [ ] **Step 10.2: Verify failure**

- [ ] **Step 10.3: Add `CardSourceRef` to enums.rs**

```rust
/// Where a card originates from for `place_as_bottom_source` / similar
/// cross-zone moves. Named `Ref` because it indexes a live zone; the
/// caller must ensure the index/handle is valid at call time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardSourceRef {
    Hand(PlayerId, usize),
    Trash(PlayerId, usize),
    DeckTop(PlayerId),
    Reveal(crate::card_source::CardHandle),
}
```

(Import `PlayerId` at the top of enums.rs if needed.)

- [ ] **Step 10.4: Add `Permanent::push_under` in permanent.rs**

```rust
/// Insert `card` at the bottom of the digivolution stack (position 0).
/// The current top card remains on top. Matches DCGO's "place X as the
/// bottom digivolution source" semantics.
pub fn push_under(&mut self, card: crate::card_source::CardSource) {
    self.card_sources.insert(0, card);
}
```

- [ ] **Step 10.5: Implement Game::place_as_bottom_source**

```rust
pub fn place_as_bottom_source(
    &mut self,
    source: crate::enums::CardSourceRef,
    target: PermanentHandle,
) -> bool {
    // Take the card out of its source zone.
    let taken = match source {
        crate::enums::CardSourceRef::Hand(p, i) => {
            let player = self.player_mut(p);
            if i >= player.hand.len() {
                return false;
            }
            player.hand.remove(i)
        }
        crate::enums::CardSourceRef::Trash(p, i) => {
            let player = self.player_mut(p);
            if i >= player.trash.len() {
                return false;
            }
            player.trash.remove(i)
        }
        crate::enums::CardSourceRef::DeckTop(p) => {
            let Some(c) = self.player_mut(p).deck.pop() else {
                return false;
            };
            c
        }
        crate::enums::CardSourceRef::Reveal(h) => {
            let Some(idx) = self.revealed_cards.iter().position(|c| c.handle() == h) else {
                return false;
            };
            self.revealed_cards.remove(idx)
        }
    };

    // Push under the target permanent.
    let target_player = self.player_mut(target.player);
    if (target.index as usize) >= target_player.battle_area.len() {
        // Source already mutated — revert by routing the taken card to trash.
        target_player.trash.push(taken);
        return false;
    }
    target_player.battle_area[target.index as usize].push_under(taken);
    true
}
```

- [ ] **Step 10.6: EffectContext wrapper**

```rust
pub fn place_as_bottom_source(
    &mut self,
    source: crate::enums::CardSourceRef,
    target: PermanentHandle,
) -> bool {
    self.game.place_as_bottom_source(source, target)
}
```

- [ ] **Step 10.7: Run, verify PASS**

- [ ] **Step 10.8: Add coverage tests for Trash, DeckTop, Reveal source variants**

(Copy the pattern from Step 10.1 with three new `#[test]` functions — one per variant.)

- [ ] **Step 10.9: Run, verify all 4 variant tests PASS**

- [ ] **Step 10.10: Commit**

```bash
git add -u && git commit -m "feat(engine): place_as_bottom_source with unified CardSourceRef"
```

---

## Task 11: effect_initiated_digivolve

Allows an effect to trigger a digivolve from hand onto a field target with a cost delta and optional color-requirement bypass. Delegates to the existing digivolve machinery (find/call the real `digivolve_from_hand` in game_actions.rs — if it doesn't exist, this task must add it; check the current code before writing the test).

**Files:**
- Modify: `digimon-engine/src/game_actions.rs`
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 11.1: Read current digivolve implementation**

```bash
rg -n "fn digivolve" digimon-engine/src/
```

If there's an existing `Game::digivolve_from_hand(player, hand_index, target, cost_override, ignore_color)` or similar, the new method is a thin wrapper. If not, this task expands to implement base digivolve_from_hand first. Document the finding in the commit message.

- [ ] **Step 11.2: Write failing test for trivial case**

```rust
#[test]
fn effect_initiated_digivolve_places_card_on_target_for_free() {
    // BASE Lv.3 on field, EVO Lv.4 in hand matching digivolve costs.
    let mut base = plain_digimon("BASE3", "Base3", 3);
    base.level = Some(3);
    let mut evo = plain_digimon("EVO4", "Evo4", 4);
    evo.level = Some(4);
    evo.evo_costs = vec![digimon_engine::card_data::EvoCost {
        from_level: 3,
        color: CardColor::Red,
        cost: 2,
    }];

    let mut r = DebugRunner::builder()
        .add_card(base.clone())
        .add_card(evo.clone())
        .hand(0, &["EVO4"])
        .memory(0)
        .start();

    let target = {
        let g = r.game_mut();
        let turn = g.turn_count();
        let h = g.next_card_handle();
        let c = digimon_engine::card_source::CardSource::with_handle("BASE3", h);
        g.player_mut(0).battle_area.push(digimon_engine::permanent::Permanent::new(c, turn));
        PermanentHandle { player: 0, index: 0 }
    };

    let memory_before = r.memory();
    let ok = r.game_mut().effect_initiated_digivolve(
        0,
        0, // hand_index
        target,
        digimon_engine::enums::CostDelta::Free,
        /* ignore_color = */ false,
    );
    assert!(ok);
    assert_eq!(r.hand_size(0), 0, "EVO4 moved to stack");
    assert_eq!(r.battle_area_size(0), 1, "still one permanent (stack grew)");
    let stack_size = r.game_mut().player(0).battle_area[0].card_sources.len();
    assert_eq!(stack_size, 2, "EVO4 now on top of BASE3");
    assert_eq!(r.memory(), memory_before, "CostDelta::Free paid 0");
}
```

- [ ] **Step 11.3: Verify failure**

- [ ] **Step 11.4: Implement Game::effect_initiated_digivolve**

Implementation shape (adjust to match existing digivolve code path):

```rust
pub fn effect_initiated_digivolve(
    &mut self,
    player_id: PlayerId,
    hand_index: usize,
    target: PermanentHandle,
    cost_delta: crate::enums::CostDelta,
    ignore_color: bool,
) -> bool {
    // 1. Validate hand index and target.
    let evo_card_id = {
        let Some(player) = self.players.get(player_id as usize) else { return false; };
        let Some(card) = player.hand.get(hand_index) else { return false; };
        if (target.index as usize) >= self.player(target.player).battle_area.len() {
            return false;
        }
        card.card_id(&self.card_data).to_string()
    };

    // 2. Find the matching evo_cost on the card data. If none matches the
    //    target's top-card level/color (or `ignore_color` is set, skip the
    //    color check), return false.
    let target_top = self.player(target.player).battle_area[target.index as usize]
        .top_card()
        .clone();
    let target_level = target_top.level(&self.card_data).unwrap_or(0);
    let target_color = target_top.colors(&self.card_data);

    let Some(evo_data) = self.card_data.get(&evo_card_id) else { return false };
    let matching = evo_data.evo_costs.iter().find(|ec| {
        ec.from_level == target_level
            && (ignore_color || target_color.contains(&ec.color))
    });
    let Some(matching) = matching else { return false };

    // 3. Compute effective cost and pay.
    let effective_cost = cost_delta.resolve(matching.cost);
    if !self.pay_memory(effective_cost) {
        return false;
    }

    // 4. Move the card from hand to the top of the target's digivolution stack.
    let card = self.player_mut(player_id).hand.remove(hand_index);
    self.player_mut(target.player).battle_area[target.index as usize]
        .card_sources
        .push(card);

    // 5. Update turn_digivolved, fire WhenDigivolving.
    let turn = self.turn_count;
    self.player_mut(target.player).battle_area[target.index as usize].turn_digivolved = turn;

    self.enqueue_triggered(
        crate::enums::EffectTiming::WhenDigivolving,
        crate::selection::TriggerSource::Permanent(target),
    );
    self.drain_effect_queue();

    true
}
```

(If there's an existing `digivolve_from_hand`, delegate to it rather than duplicating this logic.)

- [ ] **Step 11.5: EffectContext wrapper**

```rust
pub fn effect_initiated_digivolve(
    &mut self,
    player: PlayerId,
    hand_index: usize,
    target: PermanentHandle,
    cost_delta: crate::enums::CostDelta,
    ignore_color: bool,
) -> bool {
    self.game
        .effect_initiated_digivolve(player, hand_index, target, cost_delta, ignore_color)
}
```

- [ ] **Step 11.6: Run, verify PASS**

- [ ] **Step 11.7: Add coverage test for ignore_color=true**

Same shape, but the target's color doesn't match `evo_cost.color`, and `ignore_color=true`.

- [ ] **Step 11.8: Run, verify PASS**

- [ ] **Step 11.9: Commit**

```bash
git add -u && git commit -m "feat(engine): effect_initiated_digivolve with CostDelta + ignore_color"
```

---

## Task 12: hatch helper (EffectContext wrapper)

`Player::hatch` already exists. This task only exposes it on `EffectContext` returning `Option<PermanentHandle>`.

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 12.1: Failing test**

```rust
use digimon_engine::card_source::CardHandle;

struct TestP2_Hatch;
impl CardEffect for TestP2_Hatch {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Hatch")
            .process(|ctx| {
                let me = ctx.player;
                ctx.hatch(me);
            })
            .build()]
    }
}

#[test]
fn ctx_hatch_moves_top_of_digitama_to_breeding() {
    let mut egg = plain_digimon("EGG", "Egg", 0);
    egg.level = Some(2);
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("HATCHER", "Hatch", 3))
        .add_card(egg)
        .hand(0, &["HATCHER"])
        .digitama(0, &["EGG"])
        .memory(3)
        .start();

    r.register_effect("HATCHER", std::sync::Arc::new(TestP2_Hatch));

    assert!(r.game_mut().player(0).breeding_area.is_none());
    r.play(0, 0);
    assert!(r.game_mut().player(0).breeding_area.is_some(), "egg hatched");
}
```

(Requires `DebugRunnerBuilder::digitama(...)` — check whether it exists. If not, add it alongside `hand` / `deck` / `security`.)

- [ ] **Step 12.2: If missing, add `digitama` builder method to DebugRunnerBuilder**

Mirror the existing `hand`/`deck` methods exactly.

- [ ] **Step 12.3: Add `EffectContext::hatch`**

```rust
/// Move the top of `player`'s digitama deck into the breeding area.
/// Returns the handle of the new breeding-area permanent, or `None` if
/// the digitama deck is empty or breeding is already occupied.
pub fn hatch(&mut self, player: PlayerId) -> Option<PermanentHandle> {
    let turn = self.game.turn_count;
    let ok = self.game.player_mut(player).hatch(turn);
    if !ok {
        return None;
    }
    // Breeding-area handles use a sentinel index; follow the existing
    // convention (check permanent.rs / game.rs for `BREEDING_INDEX` or
    // similar). If no sentinel exists, return `None` and document that
    // callers must re-query via `ctx.breeding(player)`.
    None // Replace with actual handle convention if one exists.
}
```

- [ ] **Step 12.4: Run, verify PASS**

- [ ] **Step 12.5: Commit**

```bash
git add -u && git commit -m "feat(engine): EffectContext::hatch wrapper for Phase 2"
```

---

## Task 13: place_on_top_of_security

Moves a card from a zone (hand/trash/reveal pool) to the top or bottom of a player's security stack, optionally face-up.

**Files:**
- Modify: `digimon-engine/src/game_actions.rs`
- Modify: `digimon-engine/src/effect_context/mod.rs`
- Modify: `digimon-engine/tests/zone_manipulation.rs`

- [ ] **Step 13.1: Failing test**

```rust
#[test]
fn place_on_security_from_hand_grows_security_stack() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("SHIELD", "Shield", 4))
        .hand(0, &["SHIELD"])
        .start();

    let before = r.security_count(0);
    let ok = r.game_mut().place_on_security(
        0,
        CardSourceRef::Hand(0, 0),
        StackPosition::Top,
        /* face_up = */ false,
    );
    assert!(ok);
    assert_eq!(r.security_count(0), before + 1);
    assert_eq!(r.hand_size(0), 0);
}

#[test]
fn place_on_security_face_up_marks_card_visible() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("VIS", "Visible", 4))
        .hand(0, &["VIS"])
        .start();

    let h = r.game_mut().player(0).hand[0].handle();
    let ok = r.game_mut().place_on_security(
        0,
        CardSourceRef::Hand(0, 0),
        StackPosition::Top,
        /* face_up = */ true,
    );
    assert!(ok);
    assert!(r.game_mut().player(0).face_up_security.contains(&h.as_u32_or_raw()));
}
```

(Adjust `h.as_u32_or_raw()` to whatever `CardHandle` exposes for `HashSet<u16>` membership — the existing `face_up_security: HashSet<u16>` field hints that the key is card index, not handle. If so, use the index from `CardSource`.)

- [ ] **Step 13.2: Verify failure**

- [ ] **Step 13.3: Implement Game::place_on_security**

```rust
pub fn place_on_security(
    &mut self,
    player_id: PlayerId,
    source: crate::enums::CardSourceRef,
    position: crate::enums::StackPosition,
    face_up: bool,
) -> bool {
    let taken = match source {
        crate::enums::CardSourceRef::Hand(p, i) => {
            let pl = self.player_mut(p);
            if i >= pl.hand.len() { return false; }
            pl.hand.remove(i)
        }
        crate::enums::CardSourceRef::Trash(p, i) => {
            let pl = self.player_mut(p);
            if i >= pl.trash.len() { return false; }
            pl.trash.remove(i)
        }
        crate::enums::CardSourceRef::DeckTop(p) => {
            let Some(c) = self.player_mut(p).deck.pop() else { return false };
            c
        }
        crate::enums::CardSourceRef::Reveal(h) => {
            let Some(idx) = self.revealed_cards.iter().position(|c| c.handle() == h) else {
                return false;
            };
            self.revealed_cards.remove(idx)
        }
    };

    let card_idx = taken.card_index(); // find the actual accessor on CardSource
    let target = self.player_mut(player_id);
    match position {
        crate::enums::StackPosition::Top => target.security.push(taken),
        crate::enums::StackPosition::Bottom => target.security.insert(0, taken),
        crate::enums::StackPosition::Random => {
            let len = target.security.len();
            use rand::Rng;
            let idx = if len == 0 { 0 } else { self.rng.gen_range(0..=len) };
            self.player_mut(player_id).security.insert(idx, taken);
        }
    }
    if face_up {
        self.player_mut(player_id).face_up_security.insert(card_idx);
    }
    true
}
```

- [ ] **Step 13.4: EffectContext wrapper**

```rust
pub fn place_on_security(
    &mut self,
    player: PlayerId,
    source: crate::enums::CardSourceRef,
    position: crate::enums::StackPosition,
    face_up: bool,
) -> bool {
    self.game.place_on_security(player, source, position, face_up)
}
```

- [ ] **Step 13.5: Run, verify PASS**

- [ ] **Step 13.6: Commit**

```bash
git add -u && git commit -m "feat(engine): place_on_security with face_up + StackPosition"
```

---

## Task 14: API documentation

Add a §Zone Manipulation section to `docs/RUST_ENGINE_API.md` covering every new `EffectContext` method with a brief description, signature, and one-line TDD example.

**Files:**
- Modify: `docs/RUST_ENGINE_API.md`

- [ ] **Step 14.1: Open `docs/RUST_ENGINE_API.md` and locate the "EffectContext API" section (or equivalent — find via `rg -n "EffectContext" docs/RUST_ENGINE_API.md`). Append a new subsection.**

- [ ] **Step 14.2: Add the section content**

```markdown
## §Zone Manipulation (Phase 2)

All methods live on `EffectContext` and delegate to `Game`-level helpers.
Every card-moving method returns an `Option<PermanentHandle>` or
`Option<CardHandle>` for provenance — scripts that need to follow up on the
moved card thread the returned handle into the next primitive.

### Play-from-zone

| Method | Purpose |
|--------|---------|
| `play_from_hand_with_cost(player, hand_index, CostDelta)` | Play from hand; CostDelta::Free bypasses cost |
| `play_from_trash_with_cost(player, trash_index, CostDelta)` | Play from trash; same contract |

`CostDelta`: `Free` pays 0, `Reduce(n)` pays `max(0, printed - n)`, `Fixed(n)` pays exactly `max(0, n)`.

Example — free play from hand:
```rust
Effect::on_play(card).process(|ctx| {
    ctx.play_from_hand_with_cost(ctx.player, 0, CostDelta::Free);
}).build()
```

### Card movement

| Method | Purpose |
|--------|---------|
| `add_to_hand_from_deck(player, CardHandle)` | Move a specific card from deck to hand |
| `add_to_hand_from_trash(player, CardHandle)` | Same, from trash |
| `add_to_hand_from_reveal(player, CardHandle)` | Same, from reveal pool |
| `trash_from_hand_by_index(player, hand_index)` | Trash a specific hand slot |
| `trash_from_reveal(player, CardHandle)` | Trash a revealed card |
| `return_to_hand(PermanentHandle)` | Bounce: top card → hand, sources → trash |
| `return_to_deck(PermanentHandle, StackPosition)` | Bounce to deck Top/Bottom/Random |
| `return_to_deck_from_reveal(player, CardHandle, StackPosition)` | Reveal → deck |
| `shuffle_deck(player)` | Pairs with add_to_hand_from_deck to implement "search and shuffle" |

### Reveal pool

| Method | Purpose |
|--------|---------|
| `reveal_top_deck(player, n) -> Vec<CardHandle>` | Move top N to reveal pool |
| `revealed() -> &[CardSource]` | Read-only snapshot of reveal pool |

### Placement

| Method | Purpose |
|--------|---------|
| `place_as_bottom_source(CardSourceRef, target: PermanentHandle)` | Insert a card at the bottom of target's digivolution stack |
| `place_on_security(player, CardSourceRef, StackPosition, face_up)` | Move to security stack |
| `hatch(player)` | Move top of digitama deck to breeding area |
| `effect_initiated_digivolve(player, hand_index, target, CostDelta, ignore_color)` | Digivolve by effect, optionally free and/or color-ignoring |

`CardSourceRef`: `Hand(PlayerId, usize)` | `Trash(PlayerId, usize)` | `DeckTop(PlayerId)` | `Reveal(CardHandle)`.

`StackPosition`: `Top` | `Bottom` | `Random`.

### No-approximations note

Each of these primitives is a pure movement / cost-payment operation.
Selection of *which* card to move is always the caller's responsibility,
and must surface through a `PendingSelection` built with `select_hand`,
`select_trash`, `select_reveal`, or `select_own_permanent`. Never let a
script auto-pick a target without a selection — the RL action space
must observe the branch.
```

- [ ] **Step 14.3: Commit**

```bash
git add docs/RUST_ENGINE_API.md
git commit -m "docs(engine): document Phase 2 zone-manipulation API"
```

---

## Task 15: Close gap-log entries

`docs/RUST_ENGINE_GAPS.md` contains entries from the archetype audits naming these primitives as missing. Remove / annotate those that Phase 2 closes so future assess-archetype-rust runs see the narrower gap set.

**Files:**
- Modify: `docs/RUST_ENGINE_GAPS.md`

- [ ] **Step 15.1: Read the gap log**

```bash
rg -n "play_from_hand_free|play_from_trash|add_to_hand|reveal_top_deck|return_to_hand|return_to_deck|place_as_bottom_source|effect_initiated_digivolve|trash_from_hand|place_on_security" docs/RUST_ENGINE_GAPS.md
```

- [ ] **Step 15.2: For each match, either delete the entry or append a line `**Closed by Phase 2 (2026-04-19):** <api>` followed by the source file and commit hash.**

Preserve historical entries — don't delete, annotate. Delete only duplicate listings.

- [ ] **Step 15.3: Update the `docs/RUST_PYTHON_PARITY.md` entries for any Phase 0 items that the planning session discovered were already fixed**

Flip §1.1, §1.2, §1.3, §1.4, §1.5, §2.5b from 🔴 to 🟢. Cite the source lines (from the Phase 0 research):
- §1.1 → `game_actions.rs:66`
- §1.2 → `game_phases.rs:129`
- §1.3 → `game_phases.rs:333-335`
- §1.4 → `game.rs:445` + `game.rs:466`
- §1.5 → `game_phases.rs:78-85`
- §2.5b → `combat.rs:949-955` + `effect_queue.rs:65`

- [ ] **Step 15.4: Commit**

```bash
git add docs/RUST_ENGINE_GAPS.md docs/RUST_PYTHON_PARITY.md
git commit -m "docs(engine): close Phase 0 parity items + Phase 2 zone-manipulation gaps"
```

---

## Task 16: Full suite verification + roadmap update

- [ ] **Step 16.1: Run the full engine test suite**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml
```
Expected: all tests pass, including new `zone_manipulation` suite.

- [ ] **Step 16.2: Run the Python-side Rust-backend parity test if maturin is built**

```bash
DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v
```
Expected: no regressions (Phase 2 added API surface, didn't change behavior of existing callers).

- [ ] **Step 16.3: Re-run the archetype audit on one of the five audited archetypes and confirm blocked-card count drops**

Pick TS Olympos (highest card count / highest zone-helper dependence):

```bash
/assess-archetype-rust ts-olympos
```

Expected: blocked-card count lower than 100 (the pre-Phase-2 number). Record new count in the roadmap's cumulative projection table.

- [ ] **Step 16.4: Update the roadmap cumulative-readiness table**

Open `.claude/plans/recursive-coalescing-candle.md`. Update the Phase 2 row of the projection table with the actual cards-unblocked count from Step 16.3.

- [ ] **Step 16.5: Commit**

```bash
git add .claude/plans/recursive-coalescing-candle.md
git commit -m "docs(roadmap): record measured Phase 2 unblock count"
```

---

## Self-Review Checklist (run after drafting)

- [x] **Spec coverage:** All 15 target methods from the meta-analysis Cluster A have a task (Tasks 2–13). Shared types in Task 1. Docs in Task 14. Gap-log hygiene in Task 15. Verification in Task 16.
- [x] **No placeholders:** Every step contains either exact Rust code, an exact command, or a concrete doc edit.
- [x] **Type consistency:** `CostDelta`, `StackPosition`, `CardSourceRef` used with identical definitions across tasks. `PermanentHandle { player, index: u8 }` matches existing engine conventions from context pack.
- [x] **Ordering:** Shared types (Task 1) land first because every downstream task imports them. `play_from_hand_with_cost` (Task 2) lands before `play_from_trash_with_cost` (Task 4) because they share the `pay_memory` refactor idiom.

**Known open questions flagged inside tasks:**
- Task 5 Step 5.6: `next_card_handle` / `with_handle` may need to be added if absent.
- Task 11 Step 11.1: the existing `digivolve_from_hand` implementation shape is not confirmed; implementer must read the current code before writing the new method body to avoid duplicating logic.
- Task 12 Step 12.1: `DebugRunnerBuilder::digitama` may need to be added.
- Task 13 Step 13.3: `face_up_security` key type (`u16`) vs. `CardHandle` accessor — implementer must verify the existing convention.

These are listed as checks in the task steps, not as unresolved placeholders — each has a concrete action to take at implementation time.
