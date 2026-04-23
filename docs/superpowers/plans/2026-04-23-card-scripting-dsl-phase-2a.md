# Card Scripting DSL — Phase 2a Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lower `CompiledClause::Triggered` plus the simplest `CompiledStep` variants (memory mutations, draw/trash, shuffle, hatch) so the first DSL-authored triggered cards fire end-to-end in `DebugRunner`. This is the first slice of spec §7.3 Phase 2 — the biggest "process: compiler" phase — split into sub-slices so each ships in a bounded increment.

**Architecture:** `DslCardEffect::effects()` grows a `CompiledClause::Triggered` dispatch arm that emits one `Effect` per triggered clause with the right timing + `condition` + `process` closure. A new `dsl_cards::bindings::Bindings` struct threads named values (`bind_as: pick` → PermanentHandle / hand index / etc.) across process steps. A new `dsl_cards::step` module lowers each `CompiledStep` variant into calls on `EffectContext`. Phase 2a implements the scalar/zone subset; Phases 2b–2e land the rest.

**Tech Stack:** Rust 2021, `digimon-engine`, `digimon-dsl` (no engine types), `EffectContext` mutators.

**Phase 2a scope (strict):**
- Triggered clause skeleton: timing mapping, condition, scope (FaceUp/Inherited), optional, once_per_turn, max_per_turn, summary.
- Steps: `GainMemory`, `LoseMemory`, `SetMemory`, `Draw`, `TrashFromTop`, `ShuffleDeck`, `Hatch`, `TrashTopSecurity`.
- Binding environment scaffold — populated by selection steps in Phase 2b; Phase 2a only writes `Bindings` for `SelfRef`/`Source` lookups used by the step lowering.
- One end-to-end fixture: a synthetic DSL-authored card `DSL-TEST-001` = "On Play: gain 1 memory" registered alongside the hand-written TEST cards, exercised through `DebugRunner`.

**Phase 2a non-goals:**
- Selection steps (Phase 2b)
- Zone moves requiring bindings (Phase 2b)
- Permanent mutations (Phase 2b)
- Play/digivolve steps (Phase 2d)
- Control flow (If/ForEach/Optional) — Phase 2d
- Formula evaluation — Phase 2e
- `raw_rust` dispatch — Phase 4

---

## File Structure

New files:
- `digimon-engine/src/dsl_cards/bindings.rs` — `Bindings` struct (empty in Phase 2a; populated in 2b)
- `digimon-engine/src/dsl_cards/lower_triggered.rs` — emit `Effect` for `CompiledTriggeredClause`
- `digimon-engine/src/dsl_cards/step/mod.rs` — `run_step(step, ctx, bindings)` dispatcher
- `digimon-engine/src/dsl_cards/step/memory.rs` — GainMemory/LoseMemory/SetMemory
- `digimon-engine/src/dsl_cards/step/draw.rs` — Draw/TrashFromTop/Hatch/ShuffleDeck/TrashTopSecurity
- `digimon-engine/src/dsl_cards/timing_map.rs` — `CompiledTiming → EffectTiming` lookup + builder constructor selector
- `digimon-engine/tests/dsl/phase2a_triggered.rs` — triggered-clause shape tests
- `digimon-engine/tests/dsl/phase2a_steps.rs` — step-by-step dispatch tests
- `digimon-engine/tests/dsl/phase2a_end_to_end.rs` — synthetic card plays through DebugRunner

Modified files:
- `digimon-engine/src/dsl_cards/mod.rs` — wire new sub-modules; add `Triggered` dispatch arm
- `digimon-engine/tests/dsl/main.rs` — `mod phase2a_*;`

---

## Task 1: Timing map + builder selector

**Files:**
- Create: `digimon-engine/src/dsl_cards/timing_map.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs`
- Test: `digimon-engine/tests/dsl/phase2a_triggered.rs`

- [ ] **Step 1: Write the failing test**

Create `digimon-engine/tests/dsl/phase2a_triggered.rs`:

```rust
use digimon_dsl::compiled::CompiledTiming;
use digimon_engine::dsl_cards::timing_map::compiled_timing_to_engine;
use digimon_engine::enums::EffectTiming;

#[test]
fn compiled_timing_mapping_covers_common_triggered_timings() {
    assert_eq!(compiled_timing_to_engine(CompiledTiming::OnPlay), Some(EffectTiming::OnPlay));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::WhenDigivolving), Some(EffectTiming::WhenDigivolving));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::OnAttack), Some(EffectTiming::OnAttack));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::EndOfYourTurn), Some(EffectTiming::EndOfYourTurn));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::StartOfYourTurn), Some(EffectTiming::StartOfYourTurn));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::OnSecurity), Some(EffectTiming::SecuritySkill));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::MainFromHand), Some(EffectTiming::MainFromHand));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::BeforePayCost), Some(EffectTiming::BeforePayCost));
}
```

Add `mod phase2a_triggered;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run the test**

```
cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2a_triggered
```

Expected: FAIL with "unresolved module `timing_map`".

- [ ] **Step 3: Implement**

Create `digimon-engine/src/dsl_cards/timing_map.rs`:

```rust
//! Map `digimon_dsl::compiled::CompiledTiming` → engine `EffectTiming`.
//! Returns None for DSL-only virtual timings (e.g. `Delayed`) that don't
//! map to a single engine timing — Phase 2 owns the handling of those.

use digimon_dsl::compiled::CompiledTiming;

use crate::enums::EffectTiming;

pub fn compiled_timing_to_engine(t: CompiledTiming) -> Option<EffectTiming> {
    Some(match t {
        CompiledTiming::OnPlay => EffectTiming::OnPlay,
        CompiledTiming::WhenDigivolving => EffectTiming::WhenDigivolving,
        CompiledTiming::WhenAttacking => EffectTiming::WhenAttacking,
        CompiledTiming::EndOfAttack => EffectTiming::EndOfAttack,
        CompiledTiming::EndOfBattle => EffectTiming::EndOfBattle,
        CompiledTiming::OnAttack => EffectTiming::OnAttack,
        CompiledTiming::OnDeletion => EffectTiming::OnDeletion,
        CompiledTiming::OnAnyDeletion => EffectTiming::OnAnyDeletion,
        CompiledTiming::OnEnterFieldAnyone => EffectTiming::OnEnterFieldAnyone,
        CompiledTiming::OnLeaveField => EffectTiming::OnLeaveField,
        CompiledTiming::OnSuspend => EffectTiming::OnSuspend,
        CompiledTiming::OnUnsuspend => EffectTiming::OnUnsuspend,
        CompiledTiming::OnHatch => EffectTiming::OnHatch,
        CompiledTiming::OnDigivolve => EffectTiming::OnDigivolve,
        CompiledTiming::OnDnaDigivolve => EffectTiming::OnDnaDigivolve,
        CompiledTiming::OnDigixros => EffectTiming::OnDigiXros,
        CompiledTiming::OnOpponentSecurityRemoved => EffectTiming::OnOpponentSecurityRemoved,
        CompiledTiming::OnDigivolutionCardTrashed => EffectTiming::OnDigivolutionCardTrashed,
        CompiledTiming::OnSecurityCheck => EffectTiming::OnSecurityCheck,
        CompiledTiming::OnLoseSecurity => EffectTiming::OnLoseSecurity,
        // SecuritySkill is the Rust engine's name for the per-card "when
        // revealed from security" hook; the DSL surface calls it `on_security`.
        CompiledTiming::OnSecurity => EffectTiming::SecuritySkill,
        CompiledTiming::StartOfYourTurn => EffectTiming::StartOfYourTurn,
        CompiledTiming::StartOfOpponentsTurn => EffectTiming::StartOfOpponentsTurn,
        CompiledTiming::StartOfYourMainPhase => EffectTiming::StartOfYourMainPhase,
        CompiledTiming::EndOfYourTurn => EffectTiming::EndOfYourTurn,
        CompiledTiming::EndOfOpponentsTurn => EffectTiming::EndOfOpponentsTurn,
        CompiledTiming::OnAttackTargetChange => EffectTiming::OnAttackTargetChange,
        CompiledTiming::MainFromHand => EffectTiming::MainFromHand,
        CompiledTiming::MainOnField => EffectTiming::MainOnField,
        CompiledTiming::MainFromTrash => EffectTiming::MainFromTrash,
        CompiledTiming::Counter => EffectTiming::CounterEffect,
        CompiledTiming::BeforePayCost => EffectTiming::BeforePayCost,
        // Listed for completeness — OnAllyPlayed doesn't have a dedicated
        // engine variant today; return None so the caller skips emission.
        CompiledTiming::OnAllyPlayed => return None,
        CompiledTiming::OnOptionPlaced => return None,
        CompiledTiming::Delayed => return None,
    })
}
```

Add `pub mod timing_map;` to `digimon-engine/src/dsl_cards/mod.rs` near the other `pub mod` lines.

**Note:** If the engine's `EffectTiming` has a variant named differently (e.g. `OnDigiXros` vs `OnDigixros`), use the real name. Open `digimon-engine/src/enums.rs:99` to confirm. Drop arms that reference non-existent engine variants.

- [ ] **Step 4: Run tests** — expect PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/timing_map.rs digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/phase2a_triggered.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2a: CompiledTiming → EffectTiming mapping"
```

---

## Task 2: Bindings scaffold

**Files:**
- Create: `digimon-engine/src/dsl_cards/bindings.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs`

- [ ] **Step 1: Write the test**

Append to `digimon-engine/tests/dsl/phase2a_triggered.rs`:

```rust
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::bindings::{BindingValue, Bindings};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn bindings_round_trip_permanent_and_card_handles() {
    let mut b = Bindings::new();
    let perm = PermanentHandle { player: 0, index: 2 };
    let card = CardHandle(42);
    b.insert("tgt", BindingValue::Permanent(perm));
    b.insert("pick", BindingValue::Card(card));

    assert_eq!(b.get_permanent("tgt"), Some(perm));
    assert_eq!(b.get_card("pick"), Some(card));
    assert_eq!(b.get_permanent("pick"), None);
    assert_eq!(b.get_card("tgt"), None);
    assert_eq!(b.get_permanent("missing"), None);
}
```

- [ ] **Step 2: Run** — expect FAIL.

- [ ] **Step 3: Implement**

Create `digimon-engine/src/dsl_cards/bindings.rs`:

```rust
//! Named-binding environment for DSL process steps. A step like
//! `select_own_permanent: { bind_as: tgt, ... }` writes a
//! `BindingValue::Permanent` under the name `"tgt"`; later steps read
//! via `bindings.get_permanent("tgt")`.
//!
//! Phase 2a: the scaffold — no selection steps write yet. Phase 2b's
//! selection lowering populates it.

use std::collections::HashMap;

use crate::card_source::CardHandle;
use crate::permanent::PermanentHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingValue {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(u16),
    TrashIndex(u16),
    Literal(i64),
}

#[derive(Debug, Default)]
pub struct Bindings {
    slots: HashMap<String, BindingValue>,
}

impl Bindings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &str, value: BindingValue) {
        self.slots.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<BindingValue> {
        self.slots.get(name).copied()
    }

    pub fn get_permanent(&self, name: &str) -> Option<PermanentHandle> {
        match self.get(name)? {
            BindingValue::Permanent(h) => Some(h),
            _ => None,
        }
    }

    pub fn get_card(&self, name: &str) -> Option<CardHandle> {
        match self.get(name)? {
            BindingValue::Card(h) => Some(h),
            _ => None,
        }
    }

    pub fn get_hand_index(&self, name: &str) -> Option<u16> {
        match self.get(name)? {
            BindingValue::HandIndex(i) => Some(i),
            _ => None,
        }
    }

    pub fn get_literal(&self, name: &str) -> Option<i64> {
        match self.get(name)? {
            BindingValue::Literal(v) => Some(v),
            _ => None,
        }
    }
}
```

Add `pub mod bindings;` to `digimon-engine/src/dsl_cards/mod.rs`.

- [ ] **Step 4: Run tests** — expect PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/bindings.rs digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/phase2a_triggered.rs
git commit -m "dsl phase 2a: Bindings scaffold for process-step values"
```

---

## Task 3: PlayerRef resolution helper

**Files:**
- Create: `digimon-engine/src/dsl_cards/step/mod.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs`
- Test: append to `phase2a_triggered.rs`

- [ ] **Step 1: Write the test**

Append:

```rust
use digimon_dsl::compiled::CompiledPlayerRef;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::step::resolve_player;
use digimon_engine::effect_context::EffectContext;

#[test]
fn resolve_player_maps_compiled_player_refs() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let card = runner.game.players[0].hand[0].handle();
    let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);

    assert_eq!(resolve_player(&ctx, CompiledPlayerRef::You), 0);
    // Opponent: ctx.player = 0, so opponent = whatever the game's
    // next_clockwise returns — typically 1 in a 2-player runner.
    let opp = ctx.opponent_id();
    assert_eq!(resolve_player(&ctx, CompiledPlayerRef::Opponent), opp);
    assert_eq!(resolve_player(&ctx, CompiledPlayerRef::Active), ctx.game.turn_player());
    assert_eq!(resolve_player(&ctx, CompiledPlayerRef::Any), 0);
}
```

- [ ] **Step 2: Run** — expect FAIL.

- [ ] **Step 3: Implement**

Create `digimon-engine/src/dsl_cards/step/mod.rs`:

```rust
//! Process-step lowering dispatch. Phase 2a: memory + draw/trash helpers.
//!
//! The top-level entry point is `run_step(step, ctx, bindings)`; per-family
//! lowering sits in sibling files.

pub mod draw;
pub mod memory;

use digimon_dsl::compiled::CompiledPlayerRef;

use crate::effect_context::EffectContext;
use crate::enums::PlayerId;

/// Resolve a `CompiledPlayerRef` to the concrete `PlayerId` used by the
/// running effect. `Any` resolves to `ctx.player` — callers that want a
/// full scan of all players should enumerate via `ctx.game.players.len()`.
pub fn resolve_player(ctx: &EffectContext<'_>, r: CompiledPlayerRef) -> PlayerId {
    match r {
        CompiledPlayerRef::You => ctx.player,
        CompiledPlayerRef::Opponent => ctx.opponent_id(),
        CompiledPlayerRef::Active => ctx.game.turn_player(),
        CompiledPlayerRef::Any => ctx.player,
    }
}
```

Create empty placeholder files for the family modules so the tree compiles:

`digimon-engine/src/dsl_cards/step/memory.rs`:
```rust
//! Memory-mutation step lowering. Implemented in Task 4.
```

`digimon-engine/src/dsl_cards/step/draw.rs`:
```rust
//! Draw / TrashFromTop / Hatch / ShuffleDeck / TrashTopSecurity step
//! lowering. Implemented in Task 5.
```

Add `pub mod step;` to `digimon-engine/src/dsl_cards/mod.rs`.

- [ ] **Step 4: Run tests** — PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/phase2a_triggered.rs
git commit -m "dsl phase 2a: step dispatcher scaffold + resolve_player helper"
```

---

## Task 4: Memory mutation steps

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/memory.rs`
- Modify: `digimon-engine/src/dsl_cards/step/mod.rs`
- Create: `digimon-engine/tests/dsl/phase2a_steps.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the test**

Create `digimon-engine/tests/dsl/phase2a_steps.rs`:

```rust
use digimon_dsl::compiled::CompiledStep;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::effect_context::EffectContext;

fn fresh_ctx(runner: &mut DebugRunner) -> EffectContext<'_> {
    let card = runner.game.players[0].hand[0].handle();
    EffectContext::new(&mut runner.game, card, None, 0)
}

fn fresh_runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build()
}

#[test]
fn gain_memory_step_mutates_game_memory() {
    let mut runner = fresh_runner();
    let before = runner.game.memory;
    {
        let mut ctx = fresh_ctx(&mut runner);
        let mut b = Bindings::new();
        run_step(&CompiledStep::GainMemory(2), &mut ctx, &mut b);
    }
    assert_eq!(runner.game.memory, before + 2);
}

#[test]
fn lose_memory_step_mutates_game_memory() {
    let mut runner = fresh_runner();
    let before = runner.game.memory;
    {
        let mut ctx = fresh_ctx(&mut runner);
        let mut b = Bindings::new();
        run_step(&CompiledStep::LoseMemory(3), &mut ctx, &mut b);
    }
    assert_eq!(runner.game.memory, before - 3);
}

#[test]
fn set_memory_step_sets_absolute_value() {
    let mut runner = fresh_runner();
    {
        let mut ctx = fresh_ctx(&mut runner);
        let mut b = Bindings::new();
        run_step(&CompiledStep::SetMemory(5), &mut ctx, &mut b);
    }
    assert_eq!(runner.game.memory, 5);
}
```

Add `mod phase2a_steps;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run** — FAIL (no `run_step` or dispatcher).

- [ ] **Step 3: Implement memory steps**

Rewrite `digimon-engine/src/dsl_cards/step/memory.rs`:

```rust
//! Memory-mutation step lowering.

use digimon_dsl::compiled::CompiledStep;

use crate::effect_context::EffectContext;

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>) -> bool {
    match step {
        CompiledStep::GainMemory(n) => {
            ctx.gain_memory(*n as i16);
            true
        }
        CompiledStep::LoseMemory(n) => {
            ctx.lose_memory(*n as i16);
            true
        }
        CompiledStep::SetMemory(n) => {
            ctx.set_memory(*n as i16);
            true
        }
        _ => false,
    }
}
```

Extend `digimon-engine/src/dsl_cards/step/mod.rs`:

```rust
use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;

pub fn run_step(step: &CompiledStep, ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    if memory::try_run(step, ctx) {
        return;
    }
    // Phase 2a: other step families land in Task 5.
    // Silently ignore unhandled steps in Phase 2a; Phase 2b/c/d add more.
}
```

- [ ] **Step 4: Run tests** — expect PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/memory.rs digimon-engine/src/dsl_cards/step/mod.rs digimon-engine/tests/dsl/phase2a_steps.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2a: lower memory-mutation steps (Gain/Lose/SetMemory)"
```

---

## Task 5: Draw/trash/shuffle/hatch steps

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/draw.rs`
- Modify: `digimon-engine/src/dsl_cards/step/mod.rs`
- Test: append to `phase2a_steps.rs`

- [ ] **Step 1: Write tests**

Append:

```rust
use digimon_dsl::compiled::CompiledPlayerRef;

#[test]
fn draw_step_pulls_cards_into_hand() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .deck(0, &["F", "F", "F"])
        .build();
    let before_hand = runner.game.players[0].hand.len();
    let before_deck = runner.game.players[0].deck.len();
    {
        let mut ctx = fresh_ctx(&mut runner);
        let mut b = Bindings::new();
        run_step(
            &CompiledStep::Draw { of: CompiledPlayerRef::You, count: 2 },
            &mut ctx,
            &mut b,
        );
    }
    assert_eq!(runner.game.players[0].hand.len(), before_hand + 2);
    assert_eq!(runner.game.players[0].deck.len(), before_deck - 2);
}

#[test]
fn trash_from_top_moves_deck_to_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .deck(0, &["F", "F"])
        .build();
    let before_deck = runner.game.players[0].deck.len();
    let before_trash = runner.game.players[0].trash.len();
    {
        let mut ctx = fresh_ctx(&mut runner);
        let mut b = Bindings::new();
        run_step(
            &CompiledStep::TrashFromTop { of: CompiledPlayerRef::You, count: 1 },
            &mut ctx,
            &mut b,
        );
    }
    assert_eq!(runner.game.players[0].deck.len(), before_deck - 1);
    assert_eq!(runner.game.players[0].trash.len(), before_trash + 1);
}

#[test]
fn shuffle_deck_step_runs_without_panic() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .deck(0, &["F", "F", "F"])
        .build();
    {
        let mut ctx = fresh_ctx(&mut runner);
        let mut b = Bindings::new();
        run_step(
            &CompiledStep::ShuffleDeck { of: CompiledPlayerRef::You },
            &mut ctx,
            &mut b,
        );
    }
    // Deck size unchanged after shuffle.
    assert_eq!(runner.game.players[0].deck.len(), 3);
}
```

**Check `DebugRunner::builder()`** for the real deck-setup helper. If it's not `.deck(player, &[ids])`, use whatever exists — look at tests in `digimon-engine/tests/` (e.g. `combat/` or `phase_flow/`) for the real API. If no deck helper exists, push cards onto `runner.game.players[0].deck` directly via the make_test_card → CardSource path after `.build()`.

- [ ] **Step 2: Run** — FAIL.

- [ ] **Step 3: Implement**

Rewrite `digimon-engine/src/dsl_cards/step/draw.rs`:

```rust
//! Draw / TrashFromTop / Hatch / ShuffleDeck / TrashTopSecurity.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::step::resolve_player;
use crate::effect_context::EffectContext;

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>) -> bool {
    match step {
        CompiledStep::Draw { of, count } => {
            let p = resolve_player(ctx, *of);
            ctx.draw(p, *count);
            true
        }
        CompiledStep::TrashFromTop { of, count } => {
            let p = resolve_player(ctx, *of);
            ctx.trash_from_top(p, *count);
            true
        }
        CompiledStep::ShuffleDeck { of } => {
            let p = resolve_player(ctx, *of);
            ctx.shuffle_deck(p);
            true
        }
        CompiledStep::Hatch { of } => {
            let p = resolve_player(ctx, *of);
            ctx.hatch(p);
            true
        }
        CompiledStep::TrashTopSecurity { of } => {
            let p = resolve_player(ctx, *of);
            ctx.trash_top_security(p);
            true
        }
        _ => false,
    }
}
```

Extend `digimon-engine/src/dsl_cards/step/mod.rs`:

```rust
pub fn run_step(step: &CompiledStep, ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    if memory::try_run(step, ctx) {
        return;
    }
    if draw::try_run(step, ctx) {
        return;
    }
    // Phase 2b/c/d handle the rest.
}
```

**Verify signatures on EffectContext:**
- `draw(&mut self, player: PlayerId, count: u8) -> u8`
- `trash_from_top(&mut self, player: PlayerId, count: u8) -> u8`
- `shuffle_deck(&mut self, player: PlayerId)`
- `hatch(&mut self, player: PlayerId) -> bool`
- `trash_top_security(&mut self, player: PlayerId) -> bool`

All from `digimon-engine/src/effect_context/mod.rs` (lines 286, 333, 779, 1001, 352). If any signature differs, adapt — do not invent new methods.

- [ ] **Step 4: Run tests** — PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/draw.rs digimon-engine/src/dsl_cards/step/mod.rs digimon-engine/tests/dsl/phase2a_steps.rs
git commit -m "dsl phase 2a: lower draw/trash/shuffle/hatch/trash_top_security steps"
```

---

## Task 6: Triggered clause lowering

**Files:**
- Create: `digimon-engine/src/dsl_cards/lower_triggered.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs`
- Test: append to `phase2a_triggered.rs`

- [ ] **Step 1: Write the test**

Append:

```rust
use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledScope, CompiledStep,
    CompiledTriggeredClause,
};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use std::sync::Arc;

fn fixture_on_play_gain_memory(n: i32) -> CompiledCard {
    CompiledCard {
        card: "F-T1".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(3),
        color: vec![],
        cost: Some(3),
        dp: Some(2000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Triggered(CompiledTriggeredClause {
            when: vec![CompiledTiming::OnPlay],
            scope: CompiledScope::FaceUp,
            active_when: None,
            condition: None,
            optional: false,
            once_per_turn: false,
            max_per_turn: None,
            process: vec![CompiledStep::GainMemory(n)],
            summary: Some("Gain N memory".into()),
            summary_key: None,
        })],
    }
}

#[test]
fn triggered_clause_emits_one_effect_per_timing() {
    let dsl = DslCardEffect::new(Arc::new(fixture_on_play_gain_memory(1)));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, digimon_engine::enums::EffectTiming::OnPlay);
    assert!(effects[0].on_play);
}

#[test]
fn triggered_clause_with_multiple_timings_emits_one_effect_each() {
    let mut c = fixture_on_play_gain_memory(1);
    if let CompiledClause::Triggered(t) = &mut c.effects[0] {
        t.when = vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving];
    }
    let dsl = DslCardEffect::new(Arc::new(c));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 2);
    let timings: Vec<_> = effects.iter().map(|e| e.timing).collect();
    assert!(timings.contains(&digimon_engine::enums::EffectTiming::OnPlay));
    assert!(timings.contains(&digimon_engine::enums::EffectTiming::WhenDigivolving));
}

#[test]
fn triggered_clause_once_per_turn_sets_max_per_turn() {
    let mut c = fixture_on_play_gain_memory(1);
    if let CompiledClause::Triggered(t) = &mut c.effects[0] {
        t.once_per_turn = true;
    }
    let dsl = DslCardEffect::new(Arc::new(c));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects[0].max_per_turn, 1);
}
```

- [ ] **Step 2: Run** — FAIL.

- [ ] **Step 3: Implement**

Create `digimon-engine/src/dsl_cards/lower_triggered.rs`:

```rust
//! Lower `CompiledClause::Triggered` — emits one `Effect` per entry in
//! `clause.when`. Each effect's timing maps through `timing_map`; the
//! condition and process closures are constructed lazily so the DSL
//! definition outlives each invocation.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledScope, CompiledTiming, CompiledTriggeredClause};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::step::run_step;
use crate::dsl_cards::timing_map::compiled_timing_to_engine;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::EffectTiming;

pub fn lower(card: CardHandle, clause: &CompiledTriggeredClause) -> Vec<Effect> {
    let mut out = Vec::new();
    for t in &clause.when {
        let Some(engine_timing) = compiled_timing_to_engine(*t) else {
            continue;
        };

        let process_steps = Arc::new(clause.process.clone());
        let active_when = clause.active_when.clone().map(Arc::new);
        let condition = clause.condition.clone().map(Arc::new);
        let optional = clause.optional;
        let once_per_turn = clause.once_per_turn;
        let max_per_turn = clause.max_per_turn;
        let summary = clause.summary.clone();

        let mut builder = new_builder(card, engine_timing);
        if matches!(clause.scope, CompiledScope::Inherited) {
            builder = builder.inherited();
        }
        if let Some(s) = summary {
            builder = builder.name(&s);
        }
        if once_per_turn {
            builder = builder.once_per_turn();
        } else if let Some(n) = max_per_turn {
            // No typed max_per_turn setter yet on EffectBuilder — fall through
            // to once_per_turn when n == 1; otherwise we can't express it in
            // Phase 2a (engine support needed). Document and skip.
            if n == 1 {
                builder = builder.once_per_turn();
            }
        }
        if optional {
            builder = builder.optional();
        }

        // Condition = active_when AND clause.condition.
        if active_when.is_some() || condition.is_some() {
            let aw = active_when.clone();
            let cc = condition.clone();
            builder = builder.condition(move |rctx| {
                if let Some(p) = &aw {
                    if !eval_predicate(p, rctx, PredicateSubject::None) {
                        return false;
                    }
                }
                if let Some(p) = &cc {
                    if !eval_predicate(p, rctx, PredicateSubject::None) {
                        return false;
                    }
                }
                true
            });
        }

        builder = builder.process(move |ctx| {
            let mut bindings = Bindings::new();
            for step in process_steps.iter() {
                run_step(step, ctx, &mut bindings);
            }
        });

        out.push(builder.build());
    }
    out
}

fn new_builder(card: CardHandle, timing: EffectTiming) -> EffectBuilder {
    match timing {
        EffectTiming::OnPlay => Effect::on_play(card),
        EffectTiming::WhenDigivolving => Effect::when_digivolving(card),
        EffectTiming::OnAttack => Effect::on_attack(card),
        EffectTiming::OnDeletion => Effect::on_deletion(card),
        EffectTiming::SecuritySkill => Effect::security(card),
        EffectTiming::BeforePayCost => Effect::before_pay_cost(card),
        EffectTiming::WhenAttacking => Effect::when_attacking(card),
        EffectTiming::EndOfAttack => Effect::end_of_attack(card),
        EffectTiming::EndOfBattle => Effect::end_of_battle(card),
        EffectTiming::StartOfYourTurn => Effect::start_of_your_turn(card),
        EffectTiming::StartOfOpponentsTurn => Effect::start_of_opponents_turn(card),
        EffectTiming::StartOfYourMainPhase => Effect::start_of_your_main_phase(card),
        EffectTiming::EndOfYourTurn => Effect::end_of_your_turn(card),
        EffectTiming::EndOfOpponentsTurn => Effect::end_of_opponents_turn(card),
        EffectTiming::OnEnterFieldAnyone => Effect::on_enter_field_anyone(card),
        EffectTiming::OnAnyDeletion => Effect::on_any_deletion(card),
        EffectTiming::OnDigivolve => Effect::on_digivolve(card),
        EffectTiming::OnSuspend => Effect::on_suspend(card),
        EffectTiming::OnUnsuspend => Effect::on_unsuspend(card),
        EffectTiming::OnAttackTargetChange => Effect::on_attack_target_change(card),
        EffectTiming::OnBlock => Effect::on_block(card),
        EffectTiming::OnAllyAttack => Effect::on_ally_attack(card),
        EffectTiming::OnOpponentAttack => Effect::on_opponent_attack(card),
        EffectTiming::OnHatch => Effect::on_hatch(card),
        EffectTiming::OnOpponentSecurityRemoved => Effect::on_opponent_security_removed(card),
        EffectTiming::OnDigivolutionCardTrashed => Effect::on_digivolution_card_trashed(card),
        EffectTiming::OnSecurityCheck => Effect::on_security_check(card),
        EffectTiming::OnLoseSecurity => Effect::on_lose_security(card),
        // Anything else: build via low-level constructor and set timing.
        other => EffectBuilder::new(card, other),
    }
}
```

Modify `digimon-engine/src/dsl_cards/mod.rs` — add `pub mod lower_triggered;` and extend the `effects()` dispatch:

```rust
CompiledClause::Triggered(clause) => {
    out.extend(lower_triggered::lower(card, clause));
}
```

Replace the existing `CompiledClause::Triggered(_) => { }` no-op arm.

- [ ] **Step 4: Run tests** — PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/lower_triggered.rs digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/phase2a_triggered.rs
git commit -m "dsl phase 2a: lower triggered clauses (one Effect per timing + process loop)"
```

---

## Task 7: End-to-end — synthetic DSL card plays through DebugRunner

**Files:**
- Create: `digimon-engine/tests/dsl/phase2a_end_to_end.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the test**

Create `digimon-engine/tests/dsl/phase2a_end_to_end.rs`:

```rust
//! End-to-end Phase 2a: compile a tiny synthetic DSL card from YAML,
//! register it into a fresh registry, place it in a DebugRunner, play it,
//! and verify the process closure ran (memory incremented).

use std::sync::Arc;

use digimon_dsl::loader::load_str;
use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;

#[test]
fn dsl_authored_on_play_gain_memory_card_increments_memory_when_played() {
    // Compile a one-line DSL card.
    let yaml = r#"
card: DSL-E2E-001
name: Gain 1
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - gain_memory: 1
"#;
    let spec = load_str(yaml).expect("valid YAML");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles");

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("DSL-E2E-001", "Gain 1"))
        .hand(0, &["DSL-E2E-001"])
        .build();

    // Register the compiled card's effects.
    let dsl_effect = Arc::new(DslCardEffect::new(Arc::new(compiled)));
    runner.game.effect_registry.insert("DSL-E2E-001", dsl_effect);

    let before = runner.game.memory;
    // Play the card from hand. Check DebugRunner's real API for the play
    // helper — likely `runner.play_from_hand(0, 0)` or similar. If the
    // helper is not present, call `runner.game.play_from_hand(0, 0)` per
    // the game_actions.rs API.
    runner.game.play_from_hand(0, 0);
    let after = runner.game.memory;
    assert_eq!(
        after,
        before + 1,
        "OnPlay gain_memory step should increment memory by 1"
    );
}
```

Add `mod phase2a_end_to_end;` to `digimon-engine/tests/dsl/main.rs`.

**If `load_str` / `digimon_dsl::compile::compile` have different names**, check `digimon-dsl/src/loader.rs` and `digimon-dsl/src/compile.rs` — use the real API.

**If `Game.effect_registry` isn't directly accessible**, use `runner.replace_registry(...)` or whatever the DebugRunner builder provides (look at `digimon-engine/src/debug_runner.rs` for `.with_registry(...)`).

**If `Game::play_from_hand(player, index)` is private** (it's in `game_actions.rs:130` area), call it via the public engine API or use `runner.step(...)` — look at runner.step for how hand-from-play is actually triggered in tests.

If this test turns out to require more DebugRunner plumbing than is convenient, simplify: just call `runner.game.effects_for_card(card_id, handle)` and invoke the returned process closure manually on an `EffectContext`. The point is to verify the closure runs; full play-pipeline integration can wait.

- [ ] **Step 2: Run the test**

```
cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2a_end_to_end -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run full engine suite**

```
cargo test --manifest-path digimon-engine/Cargo.toml
```

Expected: no regressions.

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/tests/dsl/phase2a_end_to_end.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 2a: end-to-end — synthetic DSL card plays through DebugRunner"
```

---

## Self-Review

**Spec coverage (§7.3 Phase 2 slice 1):**
- Triggered clause skeleton — Task 6
- Scalar steps (GainMemory/LoseMemory/SetMemory) — Task 4
- Zone-free steps (Draw/TrashFromTop/ShuffleDeck/Hatch/TrashTopSecurity) — Task 5
- Bindings scaffold — Task 2
- PlayerRef resolution — Task 3
- Timing map — Task 1
- End-to-end fixture — Task 7

**Explicit deferrals** (Phase 2b and later):
- Selection steps
- Binding writers (selection steps populate Bindings)
- Permanent mutations (Delete/Return/Suspend/DeDigivolve)
- Zone moves with bindings (AddToHandFromDeck/Trash/Reveal etc.)
- Play/digivolve from hand/trash
- Control flow (If/ForEach/PerSelected/Optional/ScheduleDelayed)
- Formula evaluation
- `raw_rust` dispatch

**Type consistency:** `run_step(&CompiledStep, &mut EffectContext, &mut Bindings)`, `resolve_player(&EffectContext, CompiledPlayerRef) -> PlayerId`, `compiled_timing_to_engine(CompiledTiming) -> Option<EffectTiming>`, `lower_triggered::lower(CardHandle, &CompiledTriggeredClause) -> Vec<Effect>`. Consistent across Tasks 1–7.
