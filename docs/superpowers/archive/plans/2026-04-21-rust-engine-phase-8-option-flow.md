# Rust Engine Phase 8 — Option Card Play Flow

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Faithfully implement the Option card type — dedicated play pipeline (pay cost → resolve → dispose), `OptionState` enum (Standard / Delayed / Linked / Training) with post-resolution routing, `linked_cards: Vec<CardSource>` sideways attachment on `Permanent`, 6 new `EffectTiming` variants for Option observers, and 4 `EffectBuilder` subtype helpers. Unblocks ~70 cards across the 5 audited archetypes (TS Olympos Counter Options, Dark Masters Option support, Rocks Plug-In suite, Medusamon Familiar plug-ins, DNA Omnimon enablers).

**Architecture:**
- New `OptionState` enum on `Permanent` (Standard default + Delayed/Linked/Training variants for Option subtypes).
- New `linked_cards: Vec<CardSource>` field on `Permanent` — sideways attachment slot for Plug-In cards (mirrors Python).
- New `PendingOption` transient state on `Game` (mirrors `PendingSecurity` / `PendingAttack`) so mid-resolution selection-unwinds can re-enter cleanly.
- New `play_option_from_hand` / `play_option_from_trash` fire-sites — dedicated Option pipeline. Decoder dispatches `HAND_EFFECT` actions by `CardKind` so Digimon stays unchanged.
- 7 `EffectTiming` variants (OnUseOption, DelayEffect, OnLink, OnTrashLinkedCard, OnUnlink, OnTrainingTrash; OptionMain pre-exists) + 4 `EffectBuilder` helpers (`.option_main()`, `.delay(trigger)`, `.link(cost, filter)`, `.training()`).
- Zero new action IDs; `ACTION_SPACE_SIZE` stays 2168.
- Phase 7 replacement framework composes — Option self-trash fires `WhenWouldBeTrashed` with cause=Cost.

**Tech Stack:** Rust 2021 (`digimon-engine/`), DebugRunner test harness, existing `EffectContext` / `Effect` / `Permanent` / `Game` patterns established in Phases 1–7.

**Spec:** [docs/superpowers/specs/2026-04-21-option-card-flow-design.md](../specs/2026-04-21-option-card-flow-design.md) — authoritative design; read before starting any task.

---

## Background

Phase 8 closes Cluster E from [`.claude/plans/recursive-coalescing-candle.md`](../../../.claude/plans/recursive-coalescing-candle.md):

- **~70 meta-pool cards** blocked across all 5 audited archetypes:
  - TS Olympos: 25+ Counter Options + Plug-Ins + Delays
  - Medusamon Petrification + Familiar: 19 Options
  - Rocks Plug-In suite: 11 cards
  - DNA Omnimon enablers: 8 cards
  - Dark Masters Option support: 7 cards

**What exists today (post-Phase 7):**
- `play_from_hand_with_cost` at [`game_actions.rs:69`](../../../digimon-engine/src/game_actions.rs) treats every CardKind the same: pays cost, removes from hand, pushes to `battle_area` as a `Permanent`, fires `OnPlay`, returns. This is **wrong for Options** — they should resolve their main effect and trash (not stay on the field).
- `EffectTiming::OptionMain` + `EffectTiming::OptionSecurity` variants exist in `enums.rs:196-197` but nothing dispatches them. Hand-authored Option scripts hijack `OnPlay` as a workaround.
- `Permanent` has no `linked_cards` field — Plug-In cards cannot attach sideways.
- Python has `_option_stays_on_field` / `_trash_option_after_resolution` helpers + `_is_delay` / `_is_training` effect flags — documented behavioral parity target.

**Python cross-reference** (see `digimon_gym/engine/game/__init__.py:483-497`, `effects.py:383-394`, `core/permanent.py:14`):
- Python distinguishes via `_is_delay` / `_is_training` flags on `ICardEffect`; any Option without these flags trashes after resolution.
- `Permanent.linked_cards: List[CardSource]` holds attached Plug-Ins.
- `Player.play_card_from_source(card)` handles the play; `execute_effects(EffectTiming.OnUseOption, ...)` fires observers; then branch on `_option_stays_on_field`.

**Design principles (carry-forward from spec §3):**
1. **No auto-selection.** Link host picks, Delay timing branches, Training slot are all `PendingSelection`s.
2. **Option is a flow, not a zone.** `OptionState` is the source of truth; zone follows.
3. **Reuse `Permanent` for Linked/Training/Delayed states** — Linked card lives INSIDE host's `linked_cards` (not as standalone Permanent); Training/Delayed ARE standalone Permanents but carry non-default `option_state`.
4. **Observer timings first-class.** `OnUseOption`, `OnTrashLinkedCard`, `OnUnlink`, `OnTrainingTrash` all get enum variants + dispatch sites.
5. **Fire-site cleanliness.** `play_option_from_hand` is a dedicated entry point; decoder forks on `CardKind`.
6. **Phase 7 replacement integration respected** — Option self-trash fires `WhenWouldBeTrashed`.
7. **Persistence survives selection-unwind** via `pending_option: Option<PendingOption>` slot on `Game`.
8. TDD per working rule 18 — failing test first.

**Cards motivating Phase 8** (representative sample):
- TS Olympos Miracle — Delay Option, fires delayed effect at end of owner's next turn.
- Rocks Battle Box — Plug-In (Link), attaches to Rocks Digimon, grants +2000 DP sideways.
- Medusamon Familiar token + Plug-In combo — Familiar tokens + Plug-Ins attached.
- DNA Omnimon cooperation trigger — Standard Option, "play an Omnimon DNA from trash".
- Dark Masters Control Option — Standard, memory denial effect.

---

## File Structure

**Modified:**
- `digimon-engine/src/enums.rs` — add 6 new `EffectTiming` variants (`OnUseOption`, `DelayEffect`, `OnLink`, `OnTrashLinkedCard`, `OnUnlink`, `OnTrainingTrash`; `OptionMain` already exists), add `DelayTrigger` enum.
- `digimon-engine/src/permanent.rs` — add `option_state: OptionState` field + `linked_cards: Vec<CardSource>` field. Extend `Default` / constructors.
- `digimon-engine/src/game.rs` — add `pending_option: Option<PendingOption>` field + initialize in `Game::new`.
- `digimon-engine/src/selection.rs` — define `PendingOption` + `OptionResolutionPhase` structs.
- `digimon-engine/src/effect.rs` — add `option_main: bool`, `delay_trigger: Option<DelayTrigger>`, `link_cost: Option<u16>`, `link_filter: Option<Box<dyn Fn...>>`, `training: bool` fields on `Effect` + 4 builder methods (`.option_main()`, `.delay(trigger)`, `.link(cost, filter)`, `.training()`).
- `digimon-engine/src/game_actions.rs` — add `play_option_from_hand` + `play_option_from_trash` + private helpers (`dispose_option`, `attach_linked_card`).
- `digimon-engine/src/action/decode.rs` (or wherever `HAND_EFFECT` routing lives) — fork on `CardKind`. Route Option cards to `play_option_from_hand`.
- `digimon-engine/src/game_phases.rs` — end-of-turn hook: scan `OptionState::Delayed` permanents whose `trash_at_end_of_turn == turn_count`, fire `DelayEffect`, trash via `delete_permanent_with_cause(Cost)`.
- `digimon-engine/src/combat.rs::commit_permanent_deletion` (from Phase 3/Phase 7) — add linked-card trash cascade before permanent removal.
- `digimon-engine/src/game_actions.rs::return_to_hand` / `return_to_deck` — add linked-card trash cascade.
- `digimon-engine/src/effect_queue.rs` — extend effect scan to include `linked_cards`' sideways-inherited effects on the host permanent.
- `digimon-engine/src/effect_context/mod.rs` — add `EffectContext::play_option(card_source)` / `trash_option(perm)` / `link_card(source, host)` helpers for effect scripts.
- `digimon-engine/src/serialization.rs` — expose `option_state` + `linked_cards` in the UI JSON view.
- `docs/RUST_ENGINE_API.md` — new §Phase 8 section.
- `docs/RUST_PYTHON_PARITY.md` — §8 entry closing Option-flow gaps.
- `.claude/plans/recursive-coalescing-candle.md` — flip Phase 8 row to ✅ Landed.

**New tests:**
- `digimon-engine/tests/option_flow/main.rs` — module harness.
- `digimon-engine/tests/option_flow/enum_and_state_shape.rs` — Task 1 shape tests.
- `digimon-engine/tests/option_flow/standard_flow.rs` — Task 2 Standard Option.
- `digimon-engine/tests/option_flow/delay_flow.rs` — Task 3 Delay.
- `digimon-engine/tests/option_flow/link_flow.rs` — Task 4 Plug-In.
- `digimon-engine/tests/option_flow/training_flow.rs` — Task 5 Training.
- `digimon-engine/tests/option_flow/replacement_integration.rs` — Task 6 Phase 7 interaction.
- `digimon-engine/tests/option_flow/behavioral_end_to_end.rs` — Task 8.

**Cargo wiring:**
- Add `[[test]] name = "option_flow" path = "tests/option_flow/main.rs"` to `digimon-engine/Cargo.toml`.

---

## Baseline

- **624 tests passing, 0 failing, 0 warnings** under `RUSTFLAGS="-D warnings"` (post–Phase 7 close commit `4abea03e`).
- `ACTION_SPACE_SIZE = 2168` — stays unchanged through Phase 8.

---

## Tasks

### Task 1: Enum + data types — no dispatch

**Files:**
- Modify: `digimon-engine/src/enums.rs` — add 6 new `EffectTiming` variants (`OptionMain` already exists; add `OnUseOption`, `DelayEffect`, `OnLink`, `OnTrashLinkedCard`, `OnUnlink`, `OnTrainingTrash`), add `DelayTrigger` enum.
- Modify: `digimon-engine/src/permanent.rs` — add `option_state` + `linked_cards` fields on `Permanent`; update `Permanent::new` to default-initialize them.
- Modify: `digimon-engine/src/selection.rs` — add `PendingOption` + `OptionResolutionPhase` data types.
- Modify: `digimon-engine/src/game.rs` — add `pending_option` field on `Game`; init in `Game::new`.
- Modify: `digimon-engine/src/effect.rs` — add `option_main`, `delay_trigger`, `link_cost`, `link_filter`, `training` fields on `Effect`; add 4 `EffectBuilder` methods.
- Create: `digimon-engine/tests/option_flow/main.rs` — module harness.
- Create: `digimon-engine/tests/option_flow/enum_and_state_shape.rs` — shape tests.
- Modify: `digimon-engine/Cargo.toml` — register `[[test]]` target.

**Key type definitions:**

In `enums.rs` — add inside the existing `EffectTiming` enum (location: after `OptionMain` / `OptionSecurity`):
```rust
// Phase 8 Option timings
/// Global observer: fires when any Option card is played by any player.
OnUseOption,

/// Fires when an Option's delayed body resolves. Most printed Delays
/// fire at end of owner's next turn; see DelayTrigger for triggers.
DelayEffect,

/// Global observer: fires AFTER a card is linked to a host Digimon.
/// Mirrors DCGO `WhenLinked` (ICardEffect.cs:992). Required by
/// Appmon-trait cards — BT21-053 (Syakomon), BT21-054, BT21-059,
/// BT21-073, AD1-005 all listen on this timing for "when this Digimon
/// gains a linked card" effects. The `OptionMain` body of the link
/// card fires BEFORE `OnLink`; the observer runs after attach.
OnLink,

/// Observer: fires when a linked card is trashed from its host.
/// Mirrors DCGO `OnLinkCardDiscarded` (ICardEffect.cs:996).
OnTrashLinkedCard,

/// Observer: fires when a linked card is cleanly unlinked.
OnUnlink,

/// Observer: fires when a Training card is trashed.
OnTrainingTrash,
```

Add the new `DelayTrigger` enum in `enums.rs`:
```rust
/// When a Delay Option's body fires relative to the play. Most printed
/// cards use `EndOfYourNextTurn`; `EndOfThisTurn` is rare but present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DelayTrigger {
    EndOfYourNextTurn,
    EndOfThisTurn,
}
```

In `permanent.rs` — add the `OptionState` enum (top-level, above `Permanent`):
```rust
/// Additional state a Permanent carries when its top card is an Option.
/// For Digimon/Tamer/DigiEgg permanents this is always `Standard`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionState {
    Standard,
    Delayed { owner: crate::enums::PlayerId, trash_at_end_of_turn: u16 },
    Linked { host: PermanentHandle },
    Training { owner: crate::enums::PlayerId },
}

impl Default for OptionState {
    fn default() -> Self {
        OptionState::Standard
    }
}
```

Extend `Permanent` struct:
```rust
pub struct Permanent {
    pub card_sources: Vec<CardSource>,
    pub is_suspended: bool,
    pub turn_played: u16,
    pub turn_digivolved: u16,
    pub is_attacking: bool,
    pub attacked_this_turn: u8,
    // Phase 8 additions:
    pub option_state: OptionState,
    pub linked_cards: Vec<CardSource>,
}
```

Every `Permanent::new` call-site must default-initialize `option_state: OptionState::Standard` and `linked_cards: Vec::new()`. Audit via `grep -rn "Permanent {" digimon-engine/src`. If constructors use a `..Default::default()` pattern, derive `Default` on `Permanent` (if feasible) or update each literal.

In `selection.rs` — add data types (near `PendingSecurity`):
```rust
/// Transient state for an Option card mid-resolution. Mirrors
/// PendingSecurity / PendingAttack. Carries the card between pay-cost
/// and dispose so effect scripts can reference it via ctx.source_card.
#[derive(Debug, Clone)]
pub struct PendingOption {
    pub owner: PlayerId,
    pub card: CardSource,
    pub resolution_phase: OptionResolutionPhase,
}

/// Where we are in the resolve-and-dispose sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionResolutionPhase {
    MainEffectDrain,
    Disposing,
    LinkSelectHost,
    Done,
}
```

In `game.rs` — add field to `Game`:
```rust
pub struct Game {
    // ... existing fields ...
    pub(crate) pending_option: Option<PendingOption>,
}
```

Initialize to `None` in every `Game` constructor.

In `effect.rs` — extend `Effect` and `EffectBuilder`:
```rust
pub struct Effect {
    // ... existing fields ...

    // Phase 8 Option flags
    pub option_main: bool,
    pub delay_trigger: Option<DelayTrigger>,
    pub link_cost: Option<u16>,
    pub link_filter: Option<Box<dyn Fn(&EffectReadContext, PermanentHandle) -> bool + Send + Sync + 'static>>,
    pub training: bool,
}
```

Default values: `false` / `None` / `None` / `None` / `false`. `Effect` is not Clone (closures) — `link_filter` follows the existing pattern for `condition`/`process`/`cost_reduction_fn`/`pay_cost_fn`/`replacement_process`.

Add 4 builder methods:
```rust
impl EffectBuilder {
    pub fn option_main(mut self) -> Self {
        self.inner.timing = EffectTiming::OptionMain;
        self.inner.option_main = true;
        self
    }

    pub fn delay(mut self, trigger: DelayTrigger) -> Self {
        self.inner.timing = EffectTiming::DelayEffect;
        self.inner.delay_trigger = Some(trigger);
        self
    }

    pub fn link<F>(mut self, cost: u16, digimon_filter: F) -> Self
    where
        F: Fn(&EffectReadContext, PermanentHandle) -> bool + Send + Sync + 'static,
    {
        self.inner.timing = EffectTiming::OptionMain;
        self.inner.link_cost = Some(cost);
        self.inner.link_filter = Some(Box::new(digimon_filter));
        self
    }

    pub fn training(mut self) -> Self {
        self.inner.timing = EffectTiming::OptionMain;
        self.inner.training = true;
        self
    }
}
```

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/option_flow/main.rs`:
```rust
mod enum_and_state_shape;
```

Create `digimon-engine/tests/option_flow/enum_and_state_shape.rs`:
```rust
use digimon_engine::enums::{DelayTrigger, EffectTiming};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionResolutionPhase, PendingOption};

#[test]
fn option_timings_exist() {
    let _ = EffectTiming::OnUseOption;
    let _ = EffectTiming::OptionMain;       // already existed — smoke
    let _ = EffectTiming::DelayEffect;
    let _ = EffectTiming::OnLink;
    let _ = EffectTiming::OnTrashLinkedCard;
    let _ = EffectTiming::OnUnlink;
    let _ = EffectTiming::OnTrainingTrash;
}

#[test]
fn delay_trigger_variants_exist() {
    let _ = DelayTrigger::EndOfYourNextTurn;
    let _ = DelayTrigger::EndOfThisTurn;
}

#[test]
fn option_state_default_is_standard() {
    assert_eq!(OptionState::default(), OptionState::Standard);
}

#[test]
fn option_state_variants_exist() {
    let h = PermanentHandle { player: 0, index: 0 };
    let _ = OptionState::Standard;
    let _ = OptionState::Delayed { owner: 0, trash_at_end_of_turn: 5 };
    let _ = OptionState::Linked { host: h };
    let _ = OptionState::Training { owner: 0 };
}

#[test]
fn option_resolution_phase_variants_exist() {
    let _ = OptionResolutionPhase::MainEffectDrain;
    let _ = OptionResolutionPhase::Disposing;
    let _ = OptionResolutionPhase::LinkSelectHost;
    let _ = OptionResolutionPhase::Done;
}

#[test]
fn permanent_default_option_state_is_standard() {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::permanent::Permanent;

    // Build a minimal CardSource — test helper; may need adjustment to match actual constructor.
    let cs = CardSource::new_test();
    let perm = Permanent::new(cs, 1);
    assert_eq!(perm.option_state, OptionState::Standard);
    assert!(perm.linked_cards.is_empty());
}

#[test]
fn effect_builder_option_main_sets_flag() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::Effect;

    let card = CardHandle(0);
    let eff = Effect::new(card, EffectTiming::None)
        .option_main()
        .build();
    assert_eq!(eff.timing, EffectTiming::OptionMain);
    assert!(eff.option_main);
}

#[test]
fn effect_builder_delay_sets_trigger() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::Effect;

    let card = CardHandle(0);
    let eff = Effect::new(card, EffectTiming::None)
        .delay(DelayTrigger::EndOfYourNextTurn)
        .build();
    assert_eq!(eff.timing, EffectTiming::DelayEffect);
    assert_eq!(eff.delay_trigger, Some(DelayTrigger::EndOfYourNextTurn));
}

#[test]
fn effect_builder_link_stores_cost_and_filter() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::Effect;

    let card = CardHandle(0);
    let eff = Effect::new(card, EffectTiming::None)
        .link(2, |_ctx, _h| true)
        .build();
    assert_eq!(eff.link_cost, Some(2));
    assert!(eff.link_filter.is_some());
}

#[test]
fn effect_builder_training_sets_flag() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::Effect;

    let card = CardHandle(0);
    let eff = Effect::new(card, EffectTiming::None)
        .training()
        .build();
    assert!(eff.training);
}

#[test]
fn game_pending_option_default_is_none() {
    use digimon_engine::debug_runner::DebugRunner;

    let r = DebugRunner::builder().start();
    assert!(r.game.pending_option.is_none());
}
```

Note: `CardSource::new_test()` may not exist — adapt to whatever minimal constructor the codebase exposes. If none exists, use `DebugRunner::builder()...add_hand(0, &["TEST-001"])` and pull the card out that way. Match the pattern used by Task 1 of Phase 7 (`enum_and_context.rs`).

Add to `digimon-engine/Cargo.toml`:
```toml
[[test]]
name = "option_flow"
path = "tests/option_flow/main.rs"
```

- [ ] **Step 2: Run — compile failures expected**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test option_flow`
Expected: FAIL — unknown `EffectTiming::OnUseOption`, etc.

- [ ] **Step 3: Implement**

Apply the type definitions above in this order:
1. `enums.rs` — add the 6 new `EffectTiming` variants (including `OnLink`) + `DelayTrigger` enum.
2. `permanent.rs` — add `OptionState` enum + fields on `Permanent` + update constructors. Audit all `Permanent {` literal constructions via `grep -rn "Permanent {" digimon-engine/src` and update each.
3. `selection.rs` — add `PendingOption` + `OptionResolutionPhase` structs.
4. `game.rs` — add `pending_option` field + initialize in constructor. Audit via `grep -rn "fn new\|fn with_rules\|fn from_scenario" digimon-engine/src/game.rs`.
5. `effect.rs` — add 5 fields on `Effect` + 4 builder methods. Update `EffectBuilder::new` / `Effect::new` to default-init the new fields.
6. Audit any exhaustive `match` statements over `EffectTiming` across the codebase and add arms for the 5 new variants (or `_` catch-all). Run `cargo build --manifest-path digimon-engine/Cargo.toml` and fix errors one by one.

- [ ] **Step 4: Run — all Task 1 tests pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test option_flow`
Expected: PASS, ~11 tests.

- [ ] **Step 5: Full suite still green**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: **624 (Phase 7 baseline) + 11 new = 635 passing, 0 failing, 0 warnings.**

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/enums.rs digimon-engine/src/permanent.rs digimon-engine/src/selection.rs digimon-engine/src/game.rs digimon-engine/src/effect.rs digimon-engine/tests/option_flow/main.rs digimon-engine/tests/option_flow/enum_and_state_shape.rs digimon-engine/Cargo.toml
git commit -m "rust-engine(phase-8): add OptionState + PendingOption + Would/Option timings + builders"
```

---

### Task 2: Standard Option play flow (no Delay/Link/Training)

**Files:**
- Modify: `digimon-engine/src/game_actions.rs` — add `play_option_from_hand` + `play_option_from_trash` + private `dispose_option_standard`.
- Modify: `digimon-engine/src/action/decode.rs` (or wherever `HAND_EFFECT` → `play_from_hand` dispatch lives) — fork on `CardKind`.
- Modify: `digimon-engine/src/action/mask.rs` — keep existing color requirement; verify Option plays still emit in `HAND_EFFECT` range.
- Create: `digimon-engine/tests/option_flow/standard_flow.rs` — 7 tests.
- Modify: `digimon-engine/tests/option_flow/main.rs` — add `mod standard_flow;`.

**Pseudocode for `play_option_from_hand`:**

```rust
pub fn play_option_from_hand(
    &mut self,
    player_id: PlayerId,
    hand_index: usize,
) -> OptionPlayResult {
    // 1. Validate.
    if self.current_phase != GamePhase::Main {
        return OptionPlayResult::Invalid;
    }
    let player = self.player(player_id);
    if hand_index >= player.hand.len() {
        return OptionPlayResult::Invalid;
    }
    let card = &player.hand[hand_index];
    if card.card_kind(&self.card_data) != CardKind::Option {
        return OptionPlayResult::Invalid;
    }

    // 2. Color requirement (same as mask).
    if !option_color_match_available(card, player, &self.card_data) {
        return OptionPlayResult::Invalid;
    }

    // 3. Compute + pay cost (Phase 5 hooks).
    let printed_cost = card.play_cost(&self.card_data);
    let total_reduction = self.scan_before_pay_cost_reduction(player_id);
    let base_cost = printed_cost as i32;
    let effective_cost = (base_cost - total_reduction).max(0) as u16;
    if !self.pay_memory(effective_cost) {
        return OptionPlayResult::Invalid;
    }

    // 4. Remove from hand, install PendingOption.
    let card = self.player_mut(player_id).hand.remove(hand_index);
    let owner = player_id;
    self.pending_option = Some(PendingOption {
        owner,
        card: card.clone(),
        resolution_phase: OptionResolutionPhase::MainEffectDrain,
    });

    // 5. Emit OnUseOption (global observer) + OptionMain (this card's effects).
    let card_id = card.card_id(&self.card_data).to_string();
    self.enqueue_triggered_for_card(
        EffectTiming::OnUseOption,
        &card_id,
        card.handle(),
    );
    self.enqueue_triggered_for_card(
        EffectTiming::OptionMain,
        &card_id,
        card.handle(),
    );
    self.drain_effect_queue();

    // 6. If a selection parked (rare for Standard — target selections etc.),
    //    return Pending. Caller drives the selection; post-resolve, we
    //    re-enter dispose via the selection callback.
    if self.pending_selection.is_some() {
        return OptionPlayResult::Pending;
    }

    // 7. Standard Option: dispose by trashing.
    self.dispose_option_standard();

    // 8. check_turn_end.
    self.check_turn_end();

    OptionPlayResult::Trashed
}

fn dispose_option_standard(&mut self) {
    let Some(pending) = self.pending_option.take() else { return; };
    // Phase 7 WhenWouldBeTrashed replacement — Task 6 wires this fully.
    // Task 2 commits the trash unconditionally; Task 6 adds the replacement window.
    self.player_mut(pending.owner).trash.push(pending.card);
}
```

Add `OptionPlayResult` enum to `selection.rs` (near `PendingOption`):
```rust
pub enum OptionPlayResult {
    Trashed,
    Delayed(PermanentHandle),
    Linked { source: PermanentHandle },  // TBD — Task 4 decides shape
    Training(PermanentHandle),
    Pending,
    Invalid,
}
```

**Decoder fork:**

In whichever file routes `HAND_EFFECT` actions (likely `action/decode.rs`), find the call to `game.play_from_hand(player, hand_index)`. Replace with:
```rust
let player = game.player(player_id);
let card_kind = player.hand[hand_index].card_kind(&game.card_data);
match card_kind {
    CardKind::Digimon | CardKind::Tamer => {
        game.play_from_hand(player_id, hand_index);
    }
    CardKind::Option => {
        match game.play_option_from_hand(player_id, hand_index) {
            OptionPlayResult::Invalid => { /* no-op; bug */ }
            _ => { /* success or pending */ }
        }
    }
    CardKind::DigiEgg | CardKind::Token => {
        // Not playable from Main hand-play path.
    }
}
```

If no such central dispatcher exists and plays go directly through `play_from_hand`, add the dispatch at the call-site and keep `play_from_hand` as the Digimon/Tamer path.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/option_flow/standard_flow.rs`:
```rust
// Tests to land — skeletons; fill in DebugRunner setup per the existing
// Phase 7 deletion_replacements.rs pattern.

#[test]
fn standard_option_trashes_after_resolve() {
    // Build a game with a test Option card that has an OptionMain effect
    // gaining 2 memory on play. Play from hand.
    // Assert: memory += 2, card in owner's trash, battle_area unchanged.
}

#[test]
fn standard_option_fires_on_use_option_globally() {
    // Play Option on P0's turn. P1 has a field card with OnUseOption
    // observer that increments a sentinel. Assert sentinel fires.
}

#[test]
fn standard_option_pays_play_cost() {
    // Option with play_cost 3; memory starts at 5. Play it.
    // Assert: memory = 5 - 3 = 2.
}

#[test]
fn standard_option_unaffordable_returns_invalid() {
    // Memory -2 (at min), Option with cost 1. Play.
    // Assert: OptionPlayResult::Invalid, card still in hand.
}

#[test]
fn standard_option_color_mismatch_is_masked_and_rejected() {
    // P0 has only Red Digimon on field. Option in hand is Yellow.
    // Assert: hand_index action bit is 0 in mask, and direct
    //   play_option_from_hand call returns Invalid.
}

#[test]
fn standard_option_in_trash_plays_via_effect() {
    // Option in P0's trash. An effect calls play_option_from_trash.
    // Assert: OptionMain fires, card in trash after resolve (goes
    //   back after resolve — trash-to-trash is a no-op for dispose).
}

#[test]
fn standard_option_with_target_selection_returns_pending() {
    // OptionMain installs PendingSelection::Target. Play it.
    // Assert: play_option_from_hand returns Pending;
    //   pending_option.resolution_phase = MainEffectDrain.
    // Resolve the selection; assert Option then trashes.
}
```

Add `mod standard_flow;` to `digimon-engine/tests/option_flow/main.rs`.

- [ ] **Step 2: Run — FAIL (unimplemented)**

- [ ] **Step 3: Implement**

1. In `selection.rs`: add `OptionPlayResult` enum.
2. In `game_actions.rs`: implement `play_option_from_hand` per the pseudocode. Implement `play_option_from_trash` analogously (source zone = trash, skip hand-validation). Extract shared logic into a private `play_option_core(source: OptionSource, ...)` if it simplifies.
3. Implement `dispose_option_standard` as a private helper.
4. In `action/decode.rs` (or the `HAND_EFFECT` consumer): fork on `CardKind`.
5. Audit `enqueue_triggered_for_card` — if no such helper exists, extend `enqueue_triggered` to accept a specific `CardHandle` + `card_id` rather than scanning zones. Mirror the Phase 1 `QueuedEffect` pattern.
6. For the selection-unwind re-entry (test 7): the selection's callback should, on resolve, re-dispatch into `dispose_option_standard` if `pending_option` is still set. This is Task 2 scope for Standard; Task 4 extends for Link selection-unwind.

- [ ] **Step 4: Run — standard_flow tests pass**

- [ ] **Step 5: Full suite green**

Expected: **635 + 7 = 642 passing, 0 failing, 0 warnings.**

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/game_actions.rs digimon-engine/src/selection.rs digimon-engine/src/action digimon-engine/tests/option_flow
git commit -m "rust-engine(phase-8): Standard Option play pipeline — pay cost, resolve, trash"
```

---

### Task 3: Delay flow

**Files:**
- Modify: `digimon-engine/src/game_actions.rs` — extend `play_option_from_hand` dispose path to branch on `delay_trigger`; push permanent to battle_area with `OptionState::Delayed` instead of trashing.
- Modify: `digimon-engine/src/game_phases.rs` — add end-of-turn hook that scans `OptionState::Delayed` permanents.
- Modify: `digimon-engine/src/action/mask.rs` — exclude `Delayed` permanents from attack-target emission (they're not attackable).
- Modify: `digimon-engine/src/combat.rs` — `can_be_attacked` / `can_attack` helpers exclude Delayed permanents.
- Create: `digimon-engine/tests/option_flow/delay_flow.rs` — 6 tests.
- Modify: `digimon-engine/tests/option_flow/main.rs` — add `mod delay_flow;`.

**Dispose-branch pseudocode:**

Extend `dispose_option_standard` → rename to `dispose_option` with subtype switch:

```rust
fn dispose_option(&mut self) {
    let Some(pending) = self.pending_option.take() else { return; };
    let card_id = pending.card.card_id(&self.card_data).to_string();

    // Inspect the card's effects to determine subtype.
    let effects = self.effects_for_card(&card_id, pending.card.handle()).unwrap_or_default();
    let subtype = classify_option_subtype(&effects);

    match subtype {
        OptionSubtype::Standard => {
            self.player_mut(pending.owner).trash.push(pending.card);
        }
        OptionSubtype::Delay(trigger) => {
            let trash_turn = match trigger {
                DelayTrigger::EndOfThisTurn => self.turn_count,
                DelayTrigger::EndOfYourNextTurn => self.turn_count + 1,
                    // Actually needs logic: "next owner's turn end".
                    // For 2-player: if owner IS the turn_player now,
                    // their next turn end is turn_count + 2 (skip opponent's).
                    // If owner is NOT the turn_player, their next turn end is turn_count + 1.
            };
            let mut perm = Permanent::new(pending.card, self.turn_count);
            perm.option_state = OptionState::Delayed {
                owner: pending.owner,
                trash_at_end_of_turn: trash_turn,
            };
            self.player_mut(pending.owner).battle_area.push(perm);
        }
        OptionSubtype::Link => { /* Task 4 */ }
        OptionSubtype::Training => { /* Task 5 */ }
    }
}

fn classify_option_subtype(effects: &[Effect]) -> OptionSubtype {
    for eff in effects {
        if let Some(trigger) = eff.delay_trigger {
            return OptionSubtype::Delay(trigger);
        }
        if eff.training { return OptionSubtype::Training; }
        if eff.link_cost.is_some() { return OptionSubtype::Link; }
    }
    OptionSubtype::Standard
}

enum OptionSubtype {
    Standard,
    Delay(DelayTrigger),
    Link,
    Training,
}
```

The `EndOfYourNextTurn` computation needs care. Rules reference: Python's Delay cards trigger at the end of the OWNER's next turn, which for 2-player games is:
- If P0 plays a Delay on P0's turn → fires at the end of P0's next turn (turn_count + 2 in a 2-player round-robin).
- If P0 plays a Delay on P1's turn via an interrupt → fires at end of P0's turn (turn_count + 1).

Simplest encoding: store `trash_at_end_of_turn: u16` as the absolute `turn_count` at which to fire. Computed at play-time using the turn_order and current turn_player.

**End-of-turn scan:**

In `game_phases.rs::end_turn` (or wherever `EndOfYourTurn` dispatches), add at the top (BEFORE firing `EndOfYourTurn` observers — delayed effects count as part of the ending turn's resolution):
```rust
// Phase 8: Delayed Option resolution.
let ending_turn = self.turn_count;
let owner = self.turn_player;  // "Your" in EndOfYourTurn

// Collect handles first (avoid double-borrow during iteration).
let to_activate: Vec<PermanentHandle> = self.player(owner).battle_area.iter()
    .enumerate()
    .filter_map(|(i, perm)| {
        if let OptionState::Delayed { owner: o, trash_at_end_of_turn: t } = perm.option_state {
            if o == owner && t == ending_turn {
                return Some(PermanentHandle { player: owner, index: i as u8 });
            }
        }
        None
    })
    .collect();

for handle in to_activate {
    self.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(handle),
    );
    self.drain_effect_queue();

    // Trash via Phase 7 replacement framework (cause = Cost).
    self.delete_permanent_with_cause(handle, ReplacementCause::Cost);
}
```

**Attack-target exclusion:**

In `action/mask.rs` attack-emit loops, skip `Delayed` permanents. In `combat::can_be_attacked` (or equivalent), return false for `Delayed` / `Training`.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/option_flow/delay_flow.rs`:
```rust
#[test]
fn delay_parks_on_field_with_delayed_state() {
    // Option with .delay(EndOfYourNextTurn). Play.
    // Assert: pending_option cleared, battle_area has the permanent
    //   with option_state = Delayed { trash_at_end_of_turn: current_turn + 1 or +2 }.
}

#[test]
fn delay_end_of_your_next_turn_fires_correctly() {
    // Play Delay on P0's turn 1. End turn. P1 turn 2 runs. End P1's turn.
    // On P0 turn 3 end: DelayEffect fires, card trashes.
    // Assert: DelayEffect sentinel flipped, card in trash.
}

#[test]
fn delay_end_of_this_turn_fires_same_turn() {
    // Option with .delay(EndOfThisTurn). Play on turn 1. End turn 1.
    // Assert: DelayEffect fires, card trashes at end of turn 1.
}

#[test]
fn delayed_permanent_is_not_attackable() {
    // P0 plays Delay on turn 1. End turn 1. P1 turn 2 has attacker.
    // Assert: attack mask does NOT emit bit for attacking the Delayed
    //   permanent. P1 can only attack Digimon / security.
}

#[test]
fn delayed_permanent_counts_against_field_slots() {
    // Fill P0 field with 13 Digimon (1 slot free). Play a Delay.
    // Assert: play succeeds, field is now full.
    // Attempt to play another Digimon → rejected (field full).
}

#[test]
fn delay_fires_observer_wwbt_at_end_of_turn_trash() {
    // Install a WhenWouldBeTrashed cancel replacement targeting the
    // Delayed permanent. Wait for end of turn.
    // Assert: replacement fires; card NOT trashed.
    // (Tests Phase 7 integration — cause should be Cost.)
}
```

Add `mod delay_flow;` to `main.rs`.

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement**

1. Add `dispose_option` subtype dispatcher replacing `dispose_option_standard`.
2. Compute `trash_at_end_of_turn` for Delay correctly based on turn_order and who played. Write a helper `fn compute_delay_trash_turn(&self, owner: PlayerId, trigger: DelayTrigger) -> u16`.
3. Wire end-of-turn scan in `game_phases::end_turn` (or equivalent).
4. Exclude Delayed from attack-target mask emission. Search `action/mask.rs` for attack loops and add a `perm.option_state == OptionState::Standard` check (Standard means "attackable as a Digimon if it's actually a Digimon" — existing logic handles Tamer exclusion already; the new check just excludes Delayed/Training Options).
5. Update `combat::can_be_attacked` / similar validators.

- [ ] **Step 4: Run — delay_flow tests pass**

- [ ] **Step 5: Full suite green**

Expected: **642 + 6 = 648 passing, 0 failing, 0 warnings.**

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/game_actions.rs digimon-engine/src/game_phases.rs digimon-engine/src/action/mask.rs digimon-engine/src/combat.rs digimon-engine/tests/option_flow
git commit -m "rust-engine(phase-8): Delay Option flow — park on field, fire DelayEffect at end of next turn"
```

---

### Task 4: Plug-In / Link flow

**Files:**
- Modify: `digimon-engine/src/game_actions.rs` — `dispose_option` Link branch; add `attach_linked_card(host)` helper.
- Modify: `digimon-engine/src/effect_context/selections.rs` — add `.select_link_host(filter)` helper that installs a `PendingSelection::OwnField` with the link_filter closure.
- Modify: `digimon-engine/src/effect_context/mod.rs` — add `EffectContext::link_card(source, host)` public helper (for cards that link-by-effect).
- Modify: `digimon-engine/src/combat.rs::commit_permanent_deletion` — iterate `linked_cards` before removing permanent; trash each + fire `OnTrashLinkedCard`.
- Modify: `digimon-engine/src/game_actions.rs::return_to_hand` / `return_to_deck` — linked-card trash cascade.
- Modify: `digimon-engine/src/effect_queue.rs` — scan `linked_cards` sideways-inherited effects at host's timing dispatch.
- Create: `digimon-engine/tests/option_flow/link_flow.rs` — 7 tests.
- Modify: `digimon-engine/tests/option_flow/main.rs` — add `mod link_flow;`.

**Link flow pseudocode:**

In `dispose_option` Link branch:
```rust
OptionSubtype::Link => {
    // Evaluate the link_filter to find eligible hosts.
    let host_candidates = self.player(pending.owner).battle_area
        .iter()
        .enumerate()
        .filter_map(|(i, perm)| {
            if !perm.is_digimon(&self.card_data) { return None; }
            let handle = PermanentHandle { player: pending.owner, index: i as u8 };
            // Evaluate the filter.
            let read_ctx = /* build EffectReadContext from pending.card */;
            if (link_filter)(&read_ctx, handle) {
                Some(handle)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if host_candidates.is_empty() {
        // No eligible host — card goes to trash per printed rules (the
        // play should have been masked out, but the mask may have a
        // stale view — defensive).
        self.player_mut(pending.owner).trash.push(pending.card);
        return;
    }

    // Install PendingSelection::OwnField with host_candidates and an
    // on-resolve callback that calls attach_linked_card.
    self.pending_option = Some(PendingOption {
        owner: pending.owner,
        card: pending.card,
        resolution_phase: OptionResolutionPhase::LinkSelectHost,
    });

    let owner = pending.owner;
    let callback: SelectionCallback = Box::new(move |game: &mut Game, action_id: u16| {
        let host_idx = /* decode action_id back to field index */;
        let host = PermanentHandle { player: owner, index: host_idx };
        game.attach_linked_card(host);
    });

    // ... install PendingSelection with valid_action_ids = encode each host_candidate ...
}
```

`attach_linked_card`:
```rust
pub(crate) fn attach_linked_card(&mut self, host: PermanentHandle) {
    let Some(pending) = self.pending_option.take() else { return; };

    // Validate host still on field.
    let host_valid = self.handle_valid(host);
    if !host_valid {
        // Host left play — trash the card.
        self.player_mut(pending.owner).trash.push(pending.card);
        return;
    }

    // Move the CardSource into host.linked_cards.
    let host_perm = &mut self.player_mut(host.player).battle_area[host.index as usize];
    host_perm.linked_cards.push(pending.card);

    // Fire OnLink observer — global (both players' battle areas).
    // Required by Appmon-trait cards (BT21-053 Syakomon, BT21-054,
    // BT21-059, BT21-073, AD1-005) that listen for "when this Digimon
    // gains a linked card". Mirrors DCGO WhenLinked (ICardEffect.cs:992).
    //
    // OptionMain already fired during the main drain BEFORE host-select;
    // OnLink fires AFTER attach so observers see the linked card present
    // in `host.linked_cards`.
    for pid in 0..self.players.len() {
        self.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::PlayerBattleArea(pid as PlayerId),
        );
    }
    self.drain_effect_queue();
}
```

**Linked-card cleanup on host leave-field:**

In `combat.rs::commit_permanent_deletion`:
```rust
fn commit_permanent_deletion(&mut self, handle: PermanentHandle) {
    // ... existing body ...

    // Phase 8: trash linked cards before removing permanent.
    let linked = std::mem::take(&mut self.player_mut(handle.player).battle_area[handle.index as usize].linked_cards);
    for linked_card in linked {
        self.player_mut(handle.player).trash.push(linked_card);
        // Fire OnTrashLinkedCard observer — global, scan both players' battle area.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnTrashLinkedCard,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();
    }

    // ... continue existing body (OnDeletion enqueue, remove from battle_area, etc.)
}
```

Mirror in `return_to_hand` / `return_to_deck` (linked cards go to trash regardless — they can't follow the host).

**Sideways inheritance:**

In `effect_queue.rs` (or wherever `effects_for_card` is consumed for a given timing), when scanning a battle-area permanent's effects for a timing, ALSO include effects from each `linked_card`. This requires distinguishing "effects that sideways-inherit" — add a flag `linked: bool` on `Effect` (next to `inherited: bool`) that marks an effect as "fires off the host's timings when attached".

For Task 4 v1: treat ALL effects on a linked card (except `option_main`) as sideways-inherited. A card that should only contribute during OptionMain play (not while linked) uses `.option_main()` which won't fire after attachment. This is a simplification; the more correct model is explicit `.linked()` builder flag mirroring `.inherited()`. Decide: if explicit is cleaner, add `.linked()` in Task 1 retrospectively, OR in Task 4 add the flag + builder method.

**Decision:** Add `.linked()` builder flag in Task 4 (not Task 1) — it's specific to the Link flow and less muddled than conflating with `.option_main()`. Update `Effect` struct + builder + documentation.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/option_flow/link_flow.rs`:
```rust
#[test]
fn link_installs_host_selection() {
    // Play a Link Option with 2 eligible hosts on own field.
    // Assert: play returns Pending, pending_selection is OwnField with
    //   both hosts as valid_action_ids.
}

#[test]
fn link_attaches_to_chosen_host() {
    // Install selection, resolve with host B.
    // Assert: B.linked_cards contains the Option card; Option card NOT
    //   in trash and NOT a standalone Permanent.
}

#[test]
fn link_no_eligible_hosts_trashes_card() {
    // Filter eliminates all hosts. Play.
    // Assert: card in trash, no selection installed.
}

#[test]
fn linked_card_sideways_inherits_effects() {
    // Link card with an inherited-style effect (e.g. +2000 DP on host).
    // After linking, assert host.effective_dp += 2000.
}

#[test]
fn host_deletion_trashes_linked_card() {
    // Link a card to a host. Delete the host.
    // Assert: linked card in owner's trash.
    //   OnTrashLinkedCard sentinel fired.
}

#[test]
fn host_return_to_hand_trashes_linked_card() {
    // Link a card. Opponent returns host to hand.
    // Assert: linked card in trash (not in hand with the host).
}

#[test]
fn linked_card_not_targetable_by_attack_or_delete() {
    // Link a card. Attempt to attack / delete the linked card directly.
    // Assert: selection masks / target validation rejects.
}

#[test]
fn on_link_observer_fires_on_both_sides_after_attach() {
    // Appmon-trait test: place two witness Digimon on the field — one
    // on P0 (the linking player) and one on P1 (opponent) — each with
    // an OnLink-timed effect bumping an Arc<Mutex<u32>> counter.
    // P0 plays a Link Option, resolves host-select to attach to their
    // own Digimon C (a third card).
    // Assert:
    //   - Both witness counters incremented by 1 (global observer).
    //   - host.linked_cards contains the Option card (observer saw
    //     the attached state, not a pre-attach snapshot).
    // Parity reference: DCGO BT21-053/054/059/073 fire their WhenLinked
    // effect regardless of which player's Digimon gains the link.
}

#[test]
fn on_link_observer_sees_option_main_already_resolved() {
    // Order-of-operations test: Link Option has an OptionMain that
    // writes "main" to a Vec<String>; a witness on the host has an
    // OnLink effect that writes "on_link". Resolve the play.
    // Assert: vec == ["main", "on_link"] — OptionMain fires BEFORE
    //   OnLink. This matches DCGO ICardEffect.cs flow: activate
    //   Link main → attach → OnLink stack.
}
```

Add `mod link_flow;` to `main.rs`.

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement**

1. Add `.linked()` builder method on `EffectBuilder`. Extend `Effect` with `linked: bool` field. Default false.
2. Implement `dispose_option` Link branch per pseudocode. Install `PendingSelection::OwnField` with host candidates.
3. Implement `attach_linked_card` helper.
4. Wire linked-card trash cascade in `commit_permanent_deletion` + `return_to_hand` + `return_to_deck`.
5. Wire sideways-inheritance in `effect_queue.rs`: when scanning a host permanent's effects for timing T, also scan each linked_card for effects with `linked == true && timing == T`.
6. Ensure linked cards aren't standalone targets — the mask shouldn't emit target action IDs for them. Since linked cards aren't in `battle_area` as standalone permanents, this should be automatic.
7. Gotcha: `CardHandle` uniqueness — linked cards retain their `card_index` (from the CardSource). Handle consumption flows (e.g. `card_kind_for_handle` scanning) need to also look inside `linked_cards`. Audit via `grep -rn "linked_cards\|battle_area.iter" digimon-engine/src`.
8. Wire `OnLink` observer emission in `attach_linked_card` per the pseudocode — fires globally across both players' battle_areas after attach, with linked_card already present. Ensures Appmon-trait cards (BT21-053 etc.) can observe.

- [ ] **Step 4: Run — link_flow tests pass**

- [ ] **Step 5: Full suite green**

Expected: **648 + 9 = 657 passing, 0 failing, 0 warnings.** (7 original tests + 2 new OnLink observer tests.)

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/game_actions.rs digimon-engine/src/effect.rs digimon-engine/src/effect_context digimon-engine/src/combat.rs digimon-engine/src/effect_queue.rs digimon-engine/src/permanent.rs digimon-engine/tests/option_flow
git commit -m "rust-engine(phase-8): Plug-In / Link flow — attach sideways, cleanup cascade, sideways inheritance"
```

---

### Task 5: Training flow

**Files:**
- Modify: `digimon-engine/src/game_actions.rs` — `dispose_option` Training branch. Push permanent with `OptionState::Training { owner }`.
- Modify: `digimon-engine/src/game_phases.rs::hatch_from_breeding` (or equivalent) — scan Training permanents and trash them after the hatch.
- Modify: `digimon-engine/src/effect_queue.rs` — breeding-permanent effect scan pulls in Training cards' inherited effects.
- Modify: `digimon-engine/src/action/mask.rs` — exclude Training from attack targets.
- Modify: `digimon-engine/src/combat.rs` — `can_be_attacked` excludes Training.
- Create: `digimon-engine/tests/option_flow/training_flow.rs` — 5 tests.
- Modify: `digimon-engine/tests/option_flow/main.rs` — add `mod training_flow;`.

**Training dispose:**
```rust
OptionSubtype::Training => {
    let mut perm = Permanent::new(pending.card, self.turn_count);
    perm.option_state = OptionState::Training { owner: pending.owner };
    self.player_mut(pending.owner).battle_area.push(perm);
}
```

Training permanents ARE in the battle_area and count against field slots (same as Delay), but are untargetable for attack/delete/digivolve like Delay.

**Hatch hook:**

Find `game_phases.rs::hatch` (or the `MOVE_FROM_BREEDING` action handler). After the egg is promoted to battle_area:
```rust
// Phase 8: trash Training permanents that belong to this owner.
let owner = /* ... */;
let training_handles: Vec<PermanentHandle> = self.player(owner).battle_area.iter()
    .enumerate()
    .filter_map(|(i, perm)| {
        if let OptionState::Training { owner: o } = perm.option_state {
            if o == owner {
                return Some(PermanentHandle { player: owner, index: i as u8 });
            }
        }
        None
    })
    .collect();

for handle in training_handles {
    self.enqueue_triggered(
        EffectTiming::OnTrainingTrash,
        TriggerSource::Permanent(handle),
    );
    self.drain_effect_queue();
    self.delete_permanent_with_cause(handle, ReplacementCause::Cost);
}
```

**Sideways-inheritance scan (breeding):**

When scanning effects on the breeding permanent for timing T (e.g. OnHatch, WhenDigivolving), additionally scan each Training permanent's effects. Only effects marked `training == true` on the Training card contribute.

Actually re-reading the spec: Training cards provide effects to the **breeding permanent** while it's in the breeding area. The inheritance flow: Training card has an `inherited: true` effect → that effect fires as if it were on the breeding Digimon's digivolution stack. The Training card itself is on the battle_area, sideways from breeding.

Simplest v1: when `effect_queue::enqueue_triggered(timing, TriggerSource::Permanent(breeding_handle))` fires, ALSO scan owner's battle_area for Training permanents and include their `inherited` effects in the emit. Scoped by `training == true` flag on the effect. Mirror the `linked` sideways pattern from Task 4.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/option_flow/training_flow.rs`:
```rust
#[test]
fn training_parks_alongside_breeding() {
    // Play a Training Option. Assert: permanent in battle_area with
    //   option_state = Training { owner }.
}

#[test]
fn training_trashes_on_breeding_promotion() {
    // Play Training. Hatch breeding egg to battle_area.
    // Assert: Training card in trash, OnTrainingTrash fired.
}

#[test]
fn training_persists_if_breeding_empty() {
    // Play Training with empty breeding area. End turn.
    // Assert: Training card still in battle_area.
}

#[test]
fn training_inherited_effect_applies_to_breeding_permanent() {
    // Training card with .inherited() .training() effect granting +1 DP.
    // Put a Digimon in breeding. Query its effective DP.
    // Assert: DP reflects the Training bonus.
}

#[test]
fn training_excluded_from_attack_targeting() {
    // Play Training. Opponent has attacker.
    // Assert: attack mask does NOT emit bit for attacking Training permanent.
}
```

Add `mod training_flow;` to `main.rs`.

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement**

1. Training dispose branch in `dispose_option`.
2. Training sideways-inheritance scan in `effect_queue.rs` (mirror Link's `linked` flag with `training` flag).
3. Hatch hook in `game_phases.rs`.
4. Attack-target exclusion in mask + combat validators (extend the Delay exclusion from Task 3).
5. If OnHatch fires during breeding promotion, include Training effects in the scan BEFORE the hatch trashes them.

- [ ] **Step 4: Run — training_flow tests pass**

- [ ] **Step 5: Full suite green**

Expected: **657 + 5 = 662 passing, 0 failing, 0 warnings.**

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/game_actions.rs digimon-engine/src/game_phases.rs digimon-engine/src/effect_queue.rs digimon-engine/src/action digimon-engine/src/combat.rs digimon-engine/tests/option_flow
git commit -m "rust-engine(phase-8): Training flow — park alongside breeding, sideways inheritance, trash on hatch"
```

---

### Task 6: Phase 7 replacement integration

**Files:**
- Modify: `digimon-engine/src/game_actions.rs::dispose_option_standard` — fire `WhenWouldBeTrashed` before committing the trash.
- Modify: Delay end-of-turn trash path (`game_phases.rs`) — already goes through `delete_permanent_with_cause(Cost)` which fires WWLBA + WWBD via Phase 7. Verify.
- Modify: Linked-card trash cascade (`combat.rs`) — document v1 constraint: WWBT does NOT fire for linked-card host-deletion cascade (too recursive; deferred).
- Create: `digimon-engine/tests/option_flow/replacement_integration.rs` — 5 tests.
- Modify: `digimon-engine/tests/option_flow/main.rs` — add `mod replacement_integration;`.

**Standard Option trash through Phase 7:**

```rust
fn dispose_option_standard_with_replacement(&mut self) {
    let Some(pending) = self.pending_option.take() else { return; };
    let card_handle = pending.card.handle();

    // Phase 7: WhenWouldBeTrashed. Subject is Card(handle, Zone::Hand),
    //   cause is Cost, original_destination is Some(Zone::Trash).
    let subject = ReplacementSubject::Card(card_handle, Zone::Hand);
    let outcome = self.try_replace(
        EffectTiming::WhenWouldBeTrashed,
        subject,
        ReplacementCause::Cost,
        Some(Zone::Trash),
    );

    if self.pending_selection.is_some() {
        // Optional replacement installed a selection; re-park pending_option
        // so the callback can finish dispose after selection resolves.
        self.pending_option = Some(PendingOption {
            owner: pending.owner,
            card: pending.card,
            resolution_phase: OptionResolutionPhase::Disposing,
        });
        return;
    }

    match outcome {
        ReplacementOutcome::None => {
            // Normal: trash.
            self.player_mut(pending.owner).trash.push(pending.card);
        }
        ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
            // Cancelled: return to hand. Per printed rules, a cancelled
            // trash returns the card to the owner's hand (this is the
            // default for "cancel a would-be-trashed event on a card
            // that wasn't a Permanent yet").
            self.player_mut(pending.owner).hand.push(pending.card);
        }
        ReplacementOutcome::Redirected(Zone::Deck) => {
            self.player_mut(pending.owner).deck.insert(0, pending.card);  // bottom
        }
        ReplacementOutcome::Redirected(Zone::Hand) => {
            self.player_mut(pending.owner).hand.push(pending.card);
        }
        _ => {
            // Unexpected — default to trash.
            debug_assert!(false, "unexpected outcome for Option trash");
            self.player_mut(pending.owner).trash.push(pending.card);
        }
    }
}
```

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/option_flow/replacement_integration.rs`:
```rust
#[test]
fn standard_option_trash_fires_when_would_be_trashed() {
    // Install a mandatory cancel replacement on a field permanent.
    // Play a Standard Option.
    // Assert: sentinel fired; Option back in owner's hand.
}

#[test]
fn standard_option_trash_cancel_returns_to_hand() {
    // Install WhenWouldBeTrashed cancel (cause=Cost filter).
    // Play Standard Option. Resolve.
    // Assert: card in hand, memory deducted (cost was paid first).
}

#[test]
fn standard_option_trash_redirect_to_deck_bottom() {
    // Mandatory redirect to Deck.
    // Assert: card at bottom of owner's deck.
}

#[test]
fn delay_end_of_turn_trash_fires_would_be_trashed() {
    // Play Delay. Install WWBT cancel. End owner's next turn.
    // Assert: cancel fires; Delayed permanent stays on field (NOT trashed).
    // Note: this is an unusual state — Delayed permanent past its trigger
    //   still on field. Document printed-rules behavior.
}

#[test]
fn linked_card_trash_on_host_deletion_does_not_fire_wwbt() {
    // Per v1 constraint: linked-card trash cascade bypasses the
    //   replacement framework (too recursive during host deletion).
    // Install WWBT on linked card. Delete host.
    // Assert: sentinel did NOT fire; linked card trashed regardless.
}
```

Add `mod replacement_integration;` to `main.rs`.

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement**

1. Wire `try_replace` call in `dispose_option_standard` per pseudocode.
2. Verify Delay end-of-turn goes through `delete_permanent_with_cause(Cost)` — should already be the case from Task 3.
3. Linked-card trash cascade: **do NOT** wire `try_replace` here (v1 constraint). Add inline comment with `TODO(phase-8-followup)`.
4. Adjust tests 4 / 5 to reflect actual v1 behavior.

- [ ] **Step 4: Run — replacement_integration tests pass**

- [ ] **Step 5: Full suite green**

Expected: **662 + 5 = 667 passing, 0 failing, 0 warnings.**

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/game_actions.rs digimon-engine/src/game_phases.rs digimon-engine/src/combat.rs digimon-engine/tests/option_flow
git commit -m "rust-engine(phase-8): Phase 7 replacement integration — Option trash fires WhenWouldBeTrashed"
```

---

### Task 7: Docs

**Files:**
- Modify: `docs/RUST_ENGINE_API.md` — add new top-level `## Phase 8 — Option Card Play Flow` section. Format matches §Phase 5, §Phase 6, §Phase 7. Cover:
  - Intro: Options are ephemeral cards — pay cost, resolve, dispose.
  - `OptionState` enum + `linked_cards` field on Permanent.
  - `PendingOption` transient state — when to consult, how selection-unwind works.
  - 7 `EffectTiming` variants (OnUseOption, OptionMain, DelayEffect, OnLink, OnTrashLinkedCard, OnUnlink, OnTrainingTrash).
  - 4 `EffectBuilder` helpers with usage examples (`.option_main()`, `.delay(trigger)`, `.link(cost, filter)`, `.training()`).
  - Worked example: a simple Standard Option script (~15 lines), a Delay script, a Link script.
  - Dispatch table: OptionPlayResult variants.
  - v1 constraints:
    - Multi-turn Delays (> 1-turn lookahead) not supported.
    - Linked-card host-deletion trash does NOT fire `WhenWouldBeTrashed`.
    - Counter-timed Options deferred to Phase 9.
    - Nested `PendingSelection::Source` inside OptionMain not supported (shared limitation with Phase 7 Partition/ArmorPurge).
  - ACTION_SPACE_SIZE unchanged note.

- Modify: `docs/RUST_PYTHON_PARITY.md` — add `## 13. Option flow (Phase 8)` section. Cover:
  - Python's `_option_stays_on_field` + `_trash_option_after_resolution` + `_is_delay` / `_is_training` flags map to Rust's `OptionState` enum + per-effect flags.
  - Rust engine now faithfully dispatches Option subtypes; Python's Option-as-Digimon-monkey-patch workarounds close.
  - v1 constraints (same list).

- Modify: `.claude/plans/recursive-coalescing-candle.md` — flip Phase 8 row in the cumulative-readiness table to ✅ Landed 2026-04-21 (re-audit pending). Update "Immediate Next Steps" to suggest Phase 9 (Combat interrupt completion) as next phase.

- Modify: `docs/superpowers/plans/2026-04-21-rust-engine-phase-8-option-flow.md` — add Status section at bottom with per-task date + commit SHA.

- [ ] **Step 1: Write §Phase 8 section**

Use the template format from `RUST_ENGINE_API.md` §Phase 7 (recently landed). Target ~200 lines. Ensure the worked examples are compilable Rust (mentally — match signatures against the actual Task 1/4 code).

- [ ] **Step 2: Write §13 parity entry**

Short (~50 lines). Mirror Phase 7 §12 style.

- [ ] **Step 3: Update roadmap**

Flip row + update next-steps paragraph.

- [ ] **Step 4: Update plan Status table**

Fill in per-task commits as a table.

- [ ] **Step 5: Verify no doc rot**

`grep -rn "TODO(phase-8" docs/` — all TODOs should be meaningful follow-ups, not stubs. Clean any stale Task-N-stub markers.

- [ ] **Step 6: Run full suite once more**

No code changes; test count unchanged at 665.

- [ ] **Step 7: Commit**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_PYTHON_PARITY.md .claude/plans/recursive-coalescing-candle.md docs/superpowers/plans/2026-04-21-rust-engine-phase-8-option-flow.md
git commit -m "docs(phase-8): RUST_ENGINE_API + PARITY + roadmap + plan status — Option flow landed"
```

---

### Task 8: Behavioral end-to-end

**Files:**
- Create: `digimon-engine/tests/option_flow/behavioral_end_to_end.rs` — 2 tests.
- Modify: `digimon-engine/tests/option_flow/main.rs` — add `mod behavioral_end_to_end;`.

**End-to-end scenarios:**

1. `ts_olympos_counter_option_then_dark_masters_delay` — a multi-turn game exercising:
   - P0 plays a Standard Option turn 1 (memory denial).
   - P1 plays a Delay Option turn 2 (end of their next turn).
   - P0 plays a Link Option turn 3 (attaches to their Digimon).
   - End of P1 turn 4: P1's Delay fires, effect resolves, Delay trashes.
   - P0 attacks into P1 (P0's linked Digimon with +2000 from Plug-In beats P1's Digimon).
   - Assert: correct state transitions across all 5 events; no soft-locks; `pending_option` correctly cleared after each play.

2. `rocks_plug_in_host_deletion_cascade` — a combat test:
   - P0 has a Rocks-style Digimon with 2 Plug-Ins linked.
   - P1 attacks and deletes the host.
   - Assert: both linked cards in P0's trash; `OnTrashLinkedCard` fired twice; `OnDeletion` fired for host.

Both tests are pure behavioral — no mocks or stubbed effects. Use hand-authored test cards (`src/cards/test/test_phase8_*.rs`) for the scenario pieces. Register in `cards/test/mod.rs`.

- [ ] **Step 1: Write failing tests**

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement test cards**

Add minimal test fixtures for: Standard Option (memory effect), Delay Option (DP effect at end of turn), Link Option (with filter + inherited DP grant). ~30 lines each.

- [ ] **Step 4: Run — behavioral tests pass**

- [ ] **Step 5: Full suite green**

Expected: **667 + 2 = 669 passing, 0 failing, 0 warnings.**

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/tests/option_flow/behavioral_end_to_end.rs digimon-engine/tests/option_flow/main.rs digimon-engine/src/cards
git commit -m "rust-engine(phase-8): behavioral end-to-end — Counter + Delay + Link across multi-turn game"
```

---

## Deferred / Out of Scope (confirmed)

The following are intentionally **not** in Phase 8:

- **Counter-timed Options** (Blast Digivolve Options played during opponent's attack). Phase 9 (Combat Interrupt Completion) handles this — it already has partial Phase 2 plumbing for `.blast_digivolve()`.
- **Multi-turn Delays.** V1 supports `EndOfThisTurn` and `EndOfYourNextTurn`. `EndOfYourNextTurnPlus2` / "at the start of each of your next 3 turns" require an extended trigger model. Document as v1 constraint.
- **Nested `PendingSelection::Source` in OptionMain** (e.g. "trash one of your digivolution sources as part of playing this Option"). Shared limitation with Phase 7 Partition/ArmorPurge; same infrastructure gap.
- **Linked-card host-deletion trash firing `WhenWouldBeTrashed`.** Too recursive during host deletion; v1 unconditionally trashes. Follow-up task if a real card requires it.
- **`OnLink` observer variant.** Rare in printed rules; not present in audits. Deferred; only `OnTrashLinkedCard` + `OnUnlink` land.
- **`CannotPlayOptionByEffect` Phase 6 flood-gate variant.** Not required by any audited card; enum not added.
- **Multi-field-slot scaling.** Delay/Training occupy 1 slot each. If printed cards ever specify "this takes 2 slots", not supported.

## Verification

Per working rule 18 + phase acceptance criteria:

1. `cargo test --manifest-path digimon-engine/Cargo.toml` — full suite green after each task (667 passing at Task 8 close).
2. Each task lands its specific behavioral DebugRunner tests before implementation.
3. `DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v` — mask size unchanged at 2168; tensor shape preserved.
4. Re-audit **Rocks** archetype (heaviest Plug-In user) after Task 4 and verify Plug-In-blocked cards drop from `.claude/plans/rust-engine-gaps-rocks.md`.
5. Re-audit **TS Olympos** archetype after Task 6 and verify Delay + Counter-Option-adjacent cards drop.
6. No new warnings introduced (`cargo build --manifest-path digimon-engine/Cargo.toml 2>&1 | grep -i warning` empty).
7. Phase 7 deletion + return + trash tests still pass after Task 4 adds `linked_cards` cleanup to `commit_permanent_deletion` / `return_to_*`.

## Open Questions (to resolve during task execution, not blocking plan acceptance)

Per spec §10:

1. **Link cost vs play cost distinction.** v1 treats the `.link(cost, ...)` cost as the sole cost (replaces play_cost). Verify vs. printed Link cards.
2. **Delay cards that last multiple turns.** Document as v1 constraint; audit for real cards.
3. **Option cards with no main effect** (pure stat-stick Plug-Ins). `.link()` doesn't require `.process()`. Confirm via tests.
4. **`OnUseOption` scope.** Global observer (both players). Confirm via DCGO reference.
5. **Training sideways-inheritance timing.** v1 scans Training effects at breeding permanent's effect-emit timings. Correct per printed rules; verify edge cases.
6. **Field-slot counting.** Delay / Training occupy a slot. Confirm.
7. **Option + Barrier interaction.** Not known to exist; if a Delay Option had Barrier, Phase 7 fire-site at `delete_permanent_with_cause` already handles it.

---

## Status

Phase 8 Option Flow — **✅ Landed 2026-04-21**. **671 tests passing**, 0 failing, 0 warnings under `-D warnings`. Baseline 624 → final 671 (+47 net new tests: 12 shape + 7 standard + 7 delay + 9 link + 5 training + 5 replacement + 2 e2e).

| Task | Commit | Landing | Quality fix |
|------|--------|---------|-------------|
| 1 Enums + types | `00fe953f` | 2026-04-21 | `8b27c6c6` |
| 2 Standard flow | `1955fec5` | 2026-04-21 | `1fdb6ce1` |
| 3 Delay flow | `30bef6f2` | 2026-04-21 | `2c2ae7a1` (critical: multi-player scan + cancel skip-set) |
| 4 Link flow | `3e93e1d0` | 2026-04-21 | `da16f4aa` |
| 5 Training flow | `ce044584` | 2026-04-21 | `4ec7e5be` |
| 6 Replacement integration | `8f38095b` | 2026-04-21 | — |
| 7 Docs | `975d004c` | 2026-04-21 | — |
| 8 Behavioral e2e | `0da9d54c` | 2026-04-21 | — |

Full suite green at end of each task was mandatory before proceeding to the next; honored.

## Deferred follow-ups

- Linked-card host-deletion cascade WhenWouldBeTrashed firing (`TODO(phase-8-followup)` in `combat.rs` §linked cascade).
- Training sideways-scope tightening once `TriggerSource::BreedingArea` exists (parity §13).
- Cancel-semantics spec note: clarify intended behavior for Card-subject trash replacements mid-resolution.
- Zone-mover helper use for Redirected(Deck)/Redirected(Hand) outcomes in `commit_option_trash_outcome` — currently direct `deck.insert(0, _)` / `hand.push(_)`.
- Counter-timed Options — Phase 9.
- Nested `PendingSelection::Source` in OptionMain — shared Phase 7 Partition/ArmorPurge limitation.
- **Post-merge code-quality items** (from final review `a5e88ee6`):
  - `OptionPlayResult::{Delayed, Linked, Training}` variants never constructed — all paths return `Trashed`/`Pending`/`Invalid`. Either wire these at subtype commit sites or remove.
  - `OptionResolutionPhase::LinkSelectHost` doc comment is stale (wrongly describes initial phase, actually reached via `MainEffectDrain → LinkSelectHost` re-install).
  - `EffectContext::delete_permanent` bypasses `delete_permanent_with_cause` — linked cards silently disappear if a card effect deletes a host via this path. Pre-Phase-8 divergence, now observable. Cross-cutting audit with Phase 7.
  - `classify_option_subtype` has no debug_assert for mutually-exclusive Delay/Link/Training flags (silent first-match-wins).
  - `compute_delay_trash_turn` 2-player hard-coding — generalize when multi-player becomes a target.
