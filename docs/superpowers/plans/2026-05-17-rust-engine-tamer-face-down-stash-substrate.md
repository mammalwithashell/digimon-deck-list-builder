# Tamer Face-Down Stash Substrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the engine + DSL substrate the ST-23 BEATBREAK and ST-24 DATA SQUAD starter decks need for their defining mechanic — placing face-down cards under any of your Tamers, and paying triggered-effect costs by trashing the bottom face-down source from under a chosen Tamer.

**Architecture:** Generalize three existing engine helpers (`place_card_under_permanent_bottom`, `place_as_bottom_source`, `place_as_bottom_source_observed`) with a `face_down: bool` axis so any caller — not just `<Training>` — can write face-down sources onto a permanent. Add a new `trash_bottom_face_down_source` helper + a one-step DSL verb that bundles "pick one of your Tamers that has at least one face-down digivolution source → trash its bottom face-down source → fire `OnDigivolutionCardTrashed`". Add four new `PredicateSpec` leaves so DSL authors can filter sources and permanents on face-down state, stack position, host-permanent kind, and face-down-source presence.

**Tech Stack:** Rust engine in `code/digimon-engine`, DSL crate in `code/digimon-dsl`, Rust integration tests under `code/digimon-engine/tests/`, existing `EffectContext` + `Game` + `Permanent` + `CardSource` + `PredicateSpec` + `eval_predicate_with_bindings` substrate.

---

## Scope note

This plan covers **only Phase A** from the source spec [`.claude/plans/rust-engine-gaps-st-23-beatbreak-st-24-data-squad.md`](../../../.claude/plans/rust-engine-gaps-st-23-beatbreak-st-24-data-squad.md). Phases B–F are independent subsystems and should each get their own plan after this one lands:

- **Phase B** — `event_host_permanent_is_source` DSL predicate. Depends on Task A5 (Tamer-host `OnDigivolutionCardTrashed` coverage).
- **Phase C** — Option-lifecycle exit (`move_self_option_under_permanent`) + unified `play_or_use_from_hand_free` + filtered hand-or-trash origin-preserving free-play. Depends on the existing "Option card play flow residual" gap.
- **Phase D** — `BeforePayCost` Parked-outcome handling + cost-reduction trigger with target-card trait predicate.
- **Phase E** — `select_player_by_metric` + player-scope mass `CannotUnsuspend`.
- **Phase F** — Shared-OPT heterogeneous-timing predicate gating + "Also treated as" name-rule alias + inherited use-requirement gating.

After Phase A lands, run `/assess-archetype-rust` again to refresh the ST-23/ST-24 audit-index counts and confirm card-by-card unblocking.

---

## Current baseline

- `code/digimon-engine/src/card_source.rs:37` — `CardSource::face_down: bool` field exists. Only `<Training>` (`code/digimon-engine/src/effect_context/mod.rs:3134-3162` `training_place_deck_top_under_self_face_down`) writes `face_down = true` today. `<Mind Link>` reads it (`MindLink.cs:25` parity) and the observation tensor zeros out `data_index` for face-down sources.
- `code/digimon-engine/src/effect_context/mod.rs:2870` — `place_card_under_permanent_bottom(card, target)` exists for `<Save>` / `<Material Save N>`. Always inserts face-up. Locates `card` in any zone, pushes under `target`.
- `code/digimon-engine/src/effect_context/mod.rs:2817-2824` — `place_as_bottom_source(source: CardSourceRef, target: PermanentHandle) -> bool` delegates to `Game::place_as_bottom_source_observed`. Always inserts face-up.
- `code/digimon-engine/src/game_actions.rs:3438-3496` — `Game::place_as_bottom_source_observed` walks `take_card_source_ref` to remove from origin, calls `push_under(card)` on the target permanent. No face-down knob.
- `code/digimon-engine/src/permanent.rs` — `Permanent::push_under(card: CardSource)` inserts at `card_sources[0]`. Caller has full control of the inserted `CardSource` instance.
- `code/digimon-dsl/src/step.rs:1036-1039` — `PlaceAsBottomSourceArgs { source: BindingRef, target: BindingRef }`. No `face_down` field.
- `code/digimon-dsl/src/compiled.rs:761-764` — `CompiledStep::PlaceAsBottomSource { source, target }`. No `face_down` field.
- `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs:348-362` — `CompiledStep::PlaceAsBottomSource` execution: resolves source + target, calls `ctx.place_as_bottom_source(source_ref, target_handle)`.
- `code/digimon-engine/src/enums.rs` — `CardSourceRef::DeckTop(PlayerId)` already exists and is plumbed through `take_card_source_ref` (`game_actions.rs:3336`).
- `code/digimon-dsl/src/predicate.rs:22-230` — `PredicateSpec` is a flat struct of `Option<_>` leaves. Existing leaves include `event_permanent_is_source`, `host_permanent_trait_has`, `trashed_source_trait_has`, `is_suspended`, `kind`, `trait_has`, `name_contains`. No `is_face_down`, no `is_bottom_source`, no `host_kind_is`, no `has_face_down_source`.
- `code/digimon-engine/src/dsl_cards/predicate.rs:47` — `eval_predicate_with_bindings(pred, rctx, subject, bindings)`. `subject` is `PredicateSubject::{Permanent, BreedingPermanent, Card, RevealedCard, None}`.
- `code/digimon-engine/src/effect_queue.rs` — `OnDigivolutionCardTrashed` dispatch. The `SourceTrashedFromStack` trigger context already populates `event_host_permanent` for Digimon-host stacks. Tamer-host stack coverage needs verification (Task A5).

## File structure

### Engine files (Rust)

- **Modify** `code/digimon-engine/src/effect_context/mod.rs`
  - `place_card_under_permanent_bottom(card, target)` → `place_card_under_permanent_bottom(card, target, face_down)`. All existing callers (`<Save>`, `<Material Save N>`) pass `face_down: false`.
  - `place_as_bottom_source(source, target)` → `place_as_bottom_source(source, target, face_down)`. All existing callers pass `face_down: false`.
  - New helper `place_deck_top_under_permanent(target, face_down) -> Option<CardHandle>` — pops controller's deck top, inserts as face-down/face-up bottom source of target.
  - New helper `trash_bottom_face_down_source(target) -> bool` — pops `target.card_sources[0]` only if `face_down == true`, routes to owner trash, fires `OnDigivolutionCardTrashed`.

- **Modify** `code/digimon-engine/src/game_actions.rs`
  - `Game::place_as_bottom_source(source, target)` → `place_as_bottom_source(source, target, face_down)`. Internal-only signature change.
  - `Game::place_as_bottom_source_observed(source, target, observer_player)` → `place_as_bottom_source_observed(source, target, observer_player, face_down)`. After `push_under`, set `card_sources[0].face_down = face_down`.

- **No changes** to `code/digimon-engine/src/card_source.rs` (field already exists), `code/digimon-engine/src/permanent.rs` (`push_under` is generic over `CardSource`), `code/digimon-engine/src/effect_queue.rs` (`OnDigivolutionCardTrashed` dispatch is already host-aware — Task A5 only adds a coverage test).

### DSL files (Rust)

- **Modify** `code/digimon-dsl/src/step.rs`
  - `PlaceAsBottomSourceArgs` adds `pub face_down: Option<bool>` (default `Some(false)` at compile time).
  - New `TrashBottomFaceDownSourceUnderTamerArgs { of: PlayerRef }` and `StepSpec::TrashBottomFaceDownSourceUnderTamer(...)` enum variant.

- **Modify** `code/digimon-dsl/src/compiled.rs`
  - `CompiledStep::PlaceAsBottomSource` adds `face_down: bool`.
  - New `CompiledStep::TrashBottomFaceDownSourceUnderTamer { of: CompiledPlayerRef }`.

- **Modify** `code/digimon-dsl/src/compile.rs`
  - `PlaceAsBottomSource` lowering threads `face_down: args.face_down.unwrap_or(false)`.
  - New `TrashBottomFaceDownSourceUnderTamer` arm.

- **Modify** `code/digimon-dsl/src/predicate.rs`
  - Add four `Option<bool>`/`Option<CardKind>` leaves to `PredicateSpec`:
    - `is_face_down: Option<bool>` (source-subject only)
    - `is_bottom_source: Option<bool>` (source-subject only; sugar for `card_sources` index 0)
    - `host_kind_is: Option<CardKind>` (source-subject only; reads host permanent's top card kind)
    - `has_face_down_source: Option<bool>` (permanent-subject only)

- **Modify** `code/digimon-engine/src/dsl_cards/predicate.rs`
  - Add eval arms for the four new leaves.
  - The three source-subject leaves require expanding `PredicateSubject` with a new `Source(SourceSelectionRef)` variant (the current `Card(CardHandle)` doesn't carry stack-position or host-permanent info). `SourceSelectionRef` (`code/digimon-engine/src/selection.rs:64-69`) already has `permanent`, `field_index`, `source_index`, `card`.

- **Modify** `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`
  - `CompiledStep::PlaceAsBottomSource` execution threads the new `face_down` argument into `ctx.place_as_bottom_source(...)`.

- **Modify** `code/digimon-engine/src/dsl_cards/step/` (new file) or extend `code/digimon-engine/src/dsl_cards/step/selections.rs`
  - Implementation for the new `TrashBottomFaceDownSourceUnderTamer` DSL step — installs `select_own_permanent { kind: tamer, has_face_down_source: true }` then `trash_bottom_face_down_source` on the pick.

### Test files

- **New** `code/digimon-engine/tests/effect_context/place_under_permanent_face_down.rs` — A1 face-down placement (engine-level).
- **New** `code/digimon-engine/tests/effect_context/place_deck_top_under_permanent.rs` — A2 deck-top placement under chosen permanent.
- **New** `code/digimon-engine/tests/effect_context/trash_bottom_face_down_source.rs` — A4 trash-bottom-face-down + `OnDigivolutionCardTrashed` dispatch.
- **New** `code/digimon-engine/tests/dsl/place_as_bottom_source_face_down.rs` — A1/A2 DSL lowering coverage.
- **New** `code/digimon-engine/tests/dsl/predicate_face_down_stack_position.rs` — A3 four new predicate leaves.
- **New** `code/digimon-engine/tests/dsl/trash_bottom_face_down_source_under_tamer.rs` — A4 DSL verb end-to-end.
- **New** `code/digimon-engine/tests/timing_dispatch/tamer_host_source_trashed.rs` — A5 Tamer-host `OnDigivolutionCardTrashed` dispatch.
- **Update existing** `code/digimon-engine/tests/effect_context/place_under_permanent.rs` — update `<Save>` and `<Material Save N>` callers to pass `face_down: false`.

### Documentation

- **Update** `docs/RUST_ENGINE_API.md` §3 "Field mutations" — document new `face_down: bool` parameter on `place_card_under_permanent_bottom` / `place_as_bottom_source`; new `place_deck_top_under_permanent` and `trash_bottom_face_down_source` helpers.
- **Update** `docs/RUST_ENGINE_API.md` §5 `PredicateSpec` — add the four new leaves.
- **Update** `docs/RUST_ENGINE_GAPS.md` — annotate the "BEATBREAK / DATA SQUAD Tamer face-down stash substrate" entry with sub-phase landings as PRs merge.

---

## Task A1.1: Add `face_down` axis to `Game::place_as_bottom_source_observed`

**Files:**
- Modify: `code/digimon-engine/src/game_actions.rs:3430-3496`
- Modify: `code/digimon-engine/src/effect_context/mod.rs:2817-2824` (caller signature stays for now — passes `face_down: false`)
- Test: `code/digimon-engine/tests/effect_context/place_under_permanent_face_down.rs` (new)

- [ ] **Step 1: Write the failing test**

Create `code/digimon-engine/tests/effect_context/place_under_permanent_face_down.rs`:

```rust
//! Task A1: `Game::place_as_bottom_source_observed` honors the new
//! `face_down: bool` axis. Existing `<Save>` / `<Material Save N>` callers
//! pass `false`; future Tamer-stash callers pass `true`.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, PlayerId};
use digimon_engine::permanent::PermanentHandle;

mod common;
use common::{make_digimon, make_tamer};

#[test]
fn place_deck_top_under_tamer_face_down_sets_face_down_flag() {
    let mut runner = DebugRunner::new();
    runner.register_card(make_tamer("TAMER-1"));
    runner.register_card(make_digimon("CARD-1"));

    let tamer_handle: PermanentHandle = runner.spawn_permanent(PlayerId::P1, "TAMER-1");
    runner.push_deck_top(PlayerId::P1, "CARD-1");

    // CALL UNDER TEST: place deck top of P1 under TAMER-1 face-down.
    let ok = runner
        .game_mut()
        .place_as_bottom_source_observed(
            CardSourceRef::DeckTop(PlayerId::P1),
            tamer_handle,
            PlayerId::P1,
            true, // <- new face_down argument
        );
    assert!(ok, "placement must succeed");

    let tamer = &runner.game().player(PlayerId::P1).battle_area[tamer_handle.index as usize];
    assert_eq!(tamer.card_sources.len(), 1);
    assert!(
        tamer.card_sources[0].face_down,
        "placed source must be face-down when face_down=true"
    );
}

#[test]
fn place_hand_under_digimon_face_up_preserves_face_up_flag() {
    let mut runner = DebugRunner::new();
    runner.register_card(make_digimon("DIGI-1"));
    runner.register_card(make_digimon("CARD-1"));

    let target: PermanentHandle = runner.spawn_permanent(PlayerId::P1, "DIGI-1");
    runner.push_hand(PlayerId::P1, "CARD-1");

    let ok = runner.game_mut().place_as_bottom_source_observed(
        CardSourceRef::Hand(PlayerId::P1, 0),
        target,
        PlayerId::P1,
        false, // face_up
    );
    assert!(ok);
    let perm = &runner.game().player(PlayerId::P1).battle_area[target.index as usize];
    assert!(!perm.card_sources[0].face_down, "face_up placement preserves face_down=false");
}
```

You'll need a `common` module with `make_digimon` / `make_tamer` helpers — check `code/digimon-engine/tests/effect_context/place_under_permanent.rs` for the existing factory pattern and copy it into a sibling `common.rs` if not already shared.

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- place_under_permanent_face_down
```

Expected: compile error "this function takes 3 arguments but 4 arguments were supplied".

- [ ] **Step 3: Update `Game::place_as_bottom_source_observed` signature and body**

In `code/digimon-engine/src/game_actions.rs:3438`:

```rust
pub(crate) fn place_as_bottom_source_observed(
    &mut self,
    source: crate::enums::CardSourceRef,
    target: PermanentHandle,
    observer_player: PlayerId,
    face_down: bool,
) -> bool {
    // ... existing Security-branch handling (lines 3444-3474) is unchanged
    // because Security-source placement is always face-up in DCGO. Pass
    // `face_down` through to the fire_effect_security_removal callsite only
    // if a future card needs face-down security-source placement.

    let Some(taken) = self.take_card_source_ref(source) else {
        return false;
    };

    if target.index == crate::action::space::BREEDING_TARGET as u8 {
        let Some(breeding) = self.player_mut(target.player).breeding_area.as_mut() else {
            let _ = self.restore_card_source_ref(source, taken);
            return false;
        };
        let mut card = taken.card;
        card.face_down = face_down;
        breeding.push_under(card);
        return true;
    }

    let target_player = self.player_mut(target.player);
    if (target.index as usize) >= target_player.battle_area.len() {
        let _ = self.restore_card_source_ref(source, taken);
        return false;
    }
    let mut card = taken.card;
    card.face_down = face_down;
    target_player.battle_area[target.index as usize].push_under(card);
    true
}
```

Also update the public wrapper at line 3430:

```rust
pub fn place_as_bottom_source(
    &mut self,
    source: crate::enums::CardSourceRef,
    target: PermanentHandle,
    face_down: bool,
) -> bool {
    self.place_as_bottom_source_observed(source, target, target.player, face_down)
}
```

- [ ] **Step 4: Update all internal callers of `Game::place_as_bottom_source*` to pass `face_down: false`**

Grep for callers:

```bash
grep -rn "place_as_bottom_source\b\|place_as_bottom_source_observed" code/digimon-engine/src/
```

Update each callsite to pass `false` as the new argument. Existing callers include:
- `code/digimon-engine/src/effect_context/mod.rs:2823` (the `EffectContext` wrapper — Task A1.2 handles)
- Any other callsites from the grep output.

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- place_under_permanent_face_down
```

Expected: PASS (both test cases).

- [ ] **Step 6: Run full engine test suite to confirm no regressions**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: all existing tests pass. Existing `<Save>` and `<Material Save N>` tests at `code/digimon-engine/tests/effect_context/place_under_permanent.rs` still pass because they implicitly pass `face_down: false` via the unchanged `EffectContext::place_card_under_permanent_bottom` wrapper (Task A1.2 changes that).

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/game_actions.rs \
        code/digimon-engine/src/effect_context/mod.rs \
        code/digimon-engine/tests/effect_context/place_under_permanent_face_down.rs \
        code/digimon-engine/tests/effect_context/common.rs
git commit -m "engine: add face_down axis to place_as_bottom_source_observed"
```

---

## Task A1.2: Add `face_down` axis to `EffectContext::place_as_bottom_source` and `place_card_under_permanent_bottom`

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs:2817-2889`

- [ ] **Step 1: Extend the failing test from Task A1.1**

Add to `code/digimon-engine/tests/effect_context/place_under_permanent_face_down.rs`:

```rust
use digimon_engine::effect_context::EffectContext;

#[test]
fn effect_context_place_card_under_permanent_bottom_face_down_sets_flag() {
    let mut runner = DebugRunner::new();
    runner.register_card(make_digimon("DIGI-1"));
    runner.register_card(make_digimon("CARD-1"));

    let target = runner.spawn_permanent(PlayerId::P1, "DIGI-1");
    let card_handle = runner.push_hand(PlayerId::P1, "CARD-1");

    let mut ctx = runner.effect_context(PlayerId::P1);
    ctx.place_card_under_permanent_bottom(card_handle, target, true);

    let perm = &runner.game().player(PlayerId::P1).battle_area[target.index as usize];
    assert_eq!(perm.card_sources.len(), 1);
    assert!(perm.card_sources[0].face_down);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_context_place_card_under_permanent_bottom_face_down
```

Expected: compile error "this function takes 2 arguments but 3 arguments were supplied".

- [ ] **Step 3: Update `EffectContext::place_card_under_permanent_bottom` signature**

In `code/digimon-engine/src/effect_context/mod.rs:2870`:

```rust
pub fn place_card_under_permanent_bottom(
    &mut self,
    card: CardHandle,
    target: PermanentHandle,
    face_down: bool,
) {
    let taken = self
        .game
        .remove_card_from_any_zone(card)
        .unwrap_or_else(|| {
            panic!(
                "place_card_under_permanent_bottom: card {:?} not found in any zone",
                card
            )
        });

    let target_player = self.game.player_mut(target.player);
    if (target.index as usize) >= target_player.battle_area.len() {
        // Safe-fail: target permanent no longer exists; route to its
        // controller's trash rather than dropping the card on the floor.
        target_player.trash.push(taken);
        return;
    }
    let mut taken = taken;
    taken.face_down = face_down;
    target_player.battle_area[target.index as usize].push_under(taken);
}
```

And update `EffectContext::place_as_bottom_source` at line 2817:

```rust
pub fn place_as_bottom_source(
    &mut self,
    source: crate::enums::CardSourceRef,
    target: PermanentHandle,
    face_down: bool,
) -> bool {
    self.game
        .place_as_bottom_source_observed(source, target, self.player, face_down)
}
```

- [ ] **Step 4: Update all callers of `place_card_under_permanent_bottom` / `place_as_bottom_source` on `EffectContext`**

Grep for callers:

```bash
grep -rn "\.place_card_under_permanent_bottom(\|\.place_as_bottom_source(" \
  code/digimon-engine/src/ \
  code/digimon-engine/tests/
```

Pass `face_down: false` at every existing callsite. The Training-keyword helper at `code/digimon-engine/src/effect_context/mod.rs:3134` already writes face-down via direct `card_sources.insert(0, card)` — it does NOT route through these helpers, so no change there.

- [ ] **Step 5: Run the test plus the full effect_context test file**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context
```

Expected: all tests pass, including the new face-down case and the existing Save/MaterialSave cases (those now pass `face_down: false` explicitly).

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/effect_context/mod.rs \
        code/digimon-engine/tests/effect_context/place_under_permanent_face_down.rs
git commit -m "engine: add face_down axis to EffectContext placement helpers"
```

---

## Task A1.3: Add `face_down` axis to DSL `place_as_bottom_source` step

**Files:**
- Modify: `code/digimon-dsl/src/step.rs:1036-1039`
- Modify: `code/digimon-dsl/src/compiled.rs:761-764`
- Modify: `code/digimon-dsl/src/compile.rs:1551`
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs:348-362`
- Test: `code/digimon-engine/tests/dsl/place_as_bottom_source_face_down.rs` (new)

- [ ] **Step 1: Write the failing test**

Create `code/digimon-engine/tests/dsl/place_as_bottom_source_face_down.rs`:

```rust
//! Task A1.3: DSL `place_as_bottom_source` step honors the new `face_down` flag.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::PlayerId;

mod common;
use common::dsl_smoke;

#[test]
fn place_as_bottom_source_face_down_true_sets_flag() {
    // Author a one-clause [On Play] card that places its own hand-binding under
    // the source Tamer face-down. Fixture details are in common::dsl_smoke.
    let yaml = r#"
card: TEST-FD-1
name: TestFaceDown
kind: tamer
cost: 4
color: [green]

effects:
  - when: on_play
    process:
      - place_as_bottom_source:
          source: { deck_top: you }
          target: source
          face_down: true
"#;

    let mut runner = dsl_smoke::load_card(yaml);
    // Stub a deck-top card so the placement has something to consume.
    runner.register_card(common::make_digimon("CARD-1"));
    runner.push_deck_top(PlayerId::P1, "CARD-1");

    let tamer_handle = runner.play_from_hand_as_tamer(PlayerId::P1, "TEST-FD-1");
    runner.resolve_pending();

    let tamer = &runner.game().player(PlayerId::P1).battle_area[tamer_handle.index as usize];
    assert_eq!(tamer.card_sources.len(), 1);
    assert!(
        tamer.card_sources[0].face_down,
        "DSL face_down: true must set face-down flag"
    );
}

#[test]
fn place_as_bottom_source_face_down_omitted_defaults_to_false() {
    let yaml = r#"
card: TEST-FU-1
name: TestFaceUp
kind: tamer
cost: 4
color: [green]

effects:
  - when: on_play
    process:
      - place_as_bottom_source:
          source: { deck_top: you }
          target: source
"#;
    let mut runner = dsl_smoke::load_card(yaml);
    runner.register_card(common::make_digimon("CARD-1"));
    runner.push_deck_top(PlayerId::P1, "CARD-1");

    let tamer_handle = runner.play_from_hand_as_tamer(PlayerId::P1, "TEST-FU-1");
    runner.resolve_pending();

    let tamer = &runner.game().player(PlayerId::P1).battle_area[tamer_handle.index as usize];
    assert!(!tamer.card_sources[0].face_down, "face_down omitted defaults to false");
}
```

(Note: Task A2.1 wires `BindingRef::DeckTop` through DSL — this test assumes that wiring exists. If A2.1 isn't done first, swap `source: { deck_top: you }` for `source: { hand: { of: you, index: 0 } }` and push the card to hand instead of deck. Re-check `code/digimon-dsl/src/binding.rs` for the canonical `BindingRef::DeckTop` YAML key.)

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- place_as_bottom_source_face_down
```

Expected: serde error "unknown field `face_down`" because `PlaceAsBottomSourceArgs` doesn't declare it.

- [ ] **Step 3: Add `face_down` field to DSL types**

In `code/digimon-dsl/src/step.rs:1036`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceAsBottomSourceArgs {
    pub source: BindingRef,
    pub target: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_down: Option<bool>,
}
```

In `code/digimon-dsl/src/compiled.rs:761`:

```rust
PlaceAsBottomSource {
    source: CompiledBindingRef,
    target: CompiledBindingRef,
    face_down: bool,
},
```

In `code/digimon-dsl/src/compile.rs:1551`:

```rust
S::PlaceAsBottomSource(a) => CompiledStep::PlaceAsBottomSource {
    source: compile_binding_ref(&a.source, ctx)?,
    target: compile_binding_ref(&a.target, ctx)?,
    face_down: a.face_down.unwrap_or(false),
},
```

In `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs:348`:

```rust
CompiledStep::PlaceAsBottomSource { source, target, face_down } => {
    let Some(source_ref) = resolve_card_source_ref(source, ctx, bindings) else {
        return RunOutcome::Synchronous;
    };
    let Some(target_handle) = resolve_permanent_handle(target, ctx, bindings) else {
        return RunOutcome::Synchronous;
    };
    let _ = ctx.place_as_bottom_source(source_ref, target_handle, *face_down);
    RunOutcome::Synchronous
}
```

- [ ] **Step 4: Run the failing test**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- place_as_bottom_source_face_down
```

Expected: PASS (both cases).

- [ ] **Step 5: Run the full DSL test suite**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-dsl/Cargo.toml
```

Expected: all existing tests pass (Task A1 callsites already pass `face_down: false` implicitly via `unwrap_or(false)`).

- [ ] **Step 6: Commit**

```bash
git add code/digimon-dsl/src/step.rs \
        code/digimon-dsl/src/compiled.rs \
        code/digimon-dsl/src/compile.rs \
        code/digimon-engine/src/dsl_cards/step/play_digivolve.rs \
        code/digimon-engine/tests/dsl/place_as_bottom_source_face_down.rs
git commit -m "dsl: add face_down flag to place_as_bottom_source step"
```

---

## Task A2.1: Add `EffectContext::place_deck_top_under_permanent` convenience helper

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs` (insert after existing `training_place_deck_top_under_self_face_down`, ~line 3162)
- Test: `code/digimon-engine/tests/effect_context/place_deck_top_under_permanent.rs` (new)

- [ ] **Step 1: Write the failing test**

Create `code/digimon-engine/tests/effect_context/place_deck_top_under_permanent.rs`:

```rust
//! Task A2.1: `EffectContext::place_deck_top_under_permanent` — generalize
//! Training's `push_deck_top_under_self_face_down` to a chosen target.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::PlayerId;

mod common;
use common::{make_digimon, make_tamer};

#[test]
fn place_deck_top_under_chosen_tamer_face_down() {
    let mut runner = DebugRunner::new();
    runner.register_card(make_tamer("TAMER-1"));
    runner.register_card(make_digimon("CARD-1"));

    let tamer = runner.spawn_permanent(PlayerId::P1, "TAMER-1");
    runner.push_deck_top(PlayerId::P1, "CARD-1");

    let mut ctx = runner.effect_context(PlayerId::P1);
    let placed = ctx.place_deck_top_under_permanent(tamer, true);
    assert!(placed.is_some(), "deck top must be consumed");

    let tamer_perm = &runner.game().player(PlayerId::P1).battle_area[tamer.index as usize];
    assert_eq!(tamer_perm.card_sources.len(), 1);
    assert!(tamer_perm.card_sources[0].face_down);
}

#[test]
fn place_deck_top_under_permanent_empty_deck_returns_none() {
    let mut runner = DebugRunner::new();
    runner.register_card(make_tamer("TAMER-1"));
    let tamer = runner.spawn_permanent(PlayerId::P1, "TAMER-1");
    // Deck empty.
    let mut ctx = runner.effect_context(PlayerId::P1);
    let placed = ctx.place_deck_top_under_permanent(tamer, true);
    assert!(placed.is_none(), "empty deck must return None");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- place_deck_top_under_permanent
```

Expected: compile error "no method named `place_deck_top_under_permanent`".

- [ ] **Step 3: Add the helper to `EffectContext`**

Insert after the `training_place_deck_top_under_self_face_down` block at `code/digimon-engine/src/effect_context/mod.rs:3162`:

```rust
/// Place the top card of `target.player`'s deck as the bottom digivolution
/// source of `target`. Generalizes `training_place_deck_top_under_self_face_down`
/// to an arbitrary target permanent (Tamer or Digimon, in either player's
/// battle area or breeding area).
///
/// Returns `Some(card_handle)` on success or `None` if the controller's
/// deck is empty. Empty-deck behavior is silent no-op (mirrors `Player::draw`
/// no-op-on-empty convention).
///
/// Used by: ST-23 BEATBREAK / ST-24 DATA SQUAD Tamer-stash placement cards
/// (e.g. ST23-13 Tomoro Tenma & Kyo Sawashiro, ST24-09 Sunflowmon, ST24-13
/// Marcus Damon & Thomas H. Norstein). The `face_down: true` variant is the
/// load-bearing case; `face_down: false` is provided for future cards that
/// want face-up deck-top placement under arbitrary targets.
pub fn place_deck_top_under_permanent(
    &mut self,
    target: PermanentHandle,
    face_down: bool,
) -> Option<CardHandle> {
    let card_handle = self.game.player(target.player).deck.last()?.handle();
    let ok = self.game.place_as_bottom_source_observed(
        crate::enums::CardSourceRef::DeckTop(target.player),
        target,
        self.player,
        face_down,
    );
    if ok {
        Some(card_handle)
    } else {
        None
    }
}
```

- [ ] **Step 4: Run the failing tests**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- place_deck_top_under_permanent
```

Expected: PASS (both cases).

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/effect_context/mod.rs \
        code/digimon-engine/tests/effect_context/place_deck_top_under_permanent.rs
git commit -m "engine: add EffectContext::place_deck_top_under_permanent helper"
```

---

## Task A2.2: Verify `BindingRef::DeckTop` is reachable from DSL `place_as_bottom_source` step

**Files:**
- Modify (if needed): `code/digimon-dsl/src/binding.rs` (BindingRef enum)
- Modify (if needed): `code/digimon-engine/src/dsl_cards/bindings.rs` (resolve_card_source_ref)
- Test: extend `code/digimon-engine/tests/dsl/place_as_bottom_source_face_down.rs`

- [ ] **Step 1: Grep for existing DeckTop binding support**

```bash
grep -rn "deck_top\|DeckTop\|BindingRef" code/digimon-dsl/src/binding.rs code/digimon-engine/src/dsl_cards/bindings.rs 2>&1
```

If `BindingRef::DeckTop` already exists and resolves through `resolve_card_source_ref` → `CardSourceRef::DeckTop`, this task is a coverage-test add only. If not, this task adds the DSL binding form.

- [ ] **Step 2: Write the failing test (or skip if A1.3 already covered it)**

Extend `code/digimon-engine/tests/dsl/place_as_bottom_source_face_down.rs` with:

```rust
#[test]
fn deck_top_binding_resolves_for_place_as_bottom_source() {
    let yaml = r#"
card: TEST-DT-1
name: TestDeckTop
kind: tamer
cost: 4
color: [green]

effects:
  - when: on_play
    process:
      - place_as_bottom_source:
          source: { deck_top: you }
          target: source
          face_down: true
"#;
    let mut runner = dsl_smoke::load_card(yaml);
    runner.register_card(common::make_digimon("CARD-X"));
    runner.push_deck_top(PlayerId::P1, "CARD-X");

    let tamer_handle = runner.play_from_hand_as_tamer(PlayerId::P1, "TEST-DT-1");
    runner.resolve_pending();

    let tamer = &runner.game().player(PlayerId::P1).battle_area[tamer_handle.index as usize];
    assert_eq!(tamer.card_sources.len(), 1, "deck top must have been consumed");
    assert!(tamer.card_sources[0].face_down);
    assert_eq!(
        runner.game().player(PlayerId::P1).deck.len(),
        0,
        "deck must be empty after the deck-top was placed"
    );
}
```

- [ ] **Step 3: Run the test**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- deck_top_binding_resolves_for_place_as_bottom_source
```

If PASS: skip to Step 5. If FAIL with a serde error or `resolve_card_source_ref returning None`: continue to Step 4.

- [ ] **Step 4: If failing — wire `BindingRef::DeckTop`**

Add variant to `BindingRef` in `code/digimon-dsl/src/binding.rs`:

```rust
pub enum BindingRef {
    // existing variants ...
    DeckTop { of: PlayerRef },
}
```

Add YAML deserialization for `{ deck_top: you }` shorthand (follow the existing pattern for `Hand { of, index }`).

In `code/digimon-engine/src/dsl_cards/bindings.rs` `resolve_card_source_ref`, add the `BindingRef::DeckTop` arm:

```rust
CompiledBindingRef::DeckTop(player_ref) => {
    let p = resolve_player_ref(player_ref, ctx)?;
    Some(CardSourceRef::DeckTop(p))
}
```

(Adjust enum-variant names to match the actual codebase.)

- [ ] **Step 5: Run all DSL tests**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-dsl/Cargo.toml
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-dsl/src/binding.rs \
        code/digimon-engine/src/dsl_cards/bindings.rs \
        code/digimon-engine/tests/dsl/place_as_bottom_source_face_down.rs
git commit -m "dsl: wire BindingRef::DeckTop through place_as_bottom_source"
```

---

## Task A3.1: Add `is_face_down` predicate leaf

**Files:**
- Modify: `code/digimon-dsl/src/predicate.rs:22-230` (`PredicateSpec` struct)
- Modify: `code/digimon-dsl/src/compiled.rs` (`CompiledPredicate` struct — mirror leaf)
- Modify: `code/digimon-dsl/src/compile.rs` (predicate lowering — pass through)
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs:21-30` (`PredicateSubject` enum — add `Source(SourceSelectionRef)` variant)
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs:47` (`eval_predicate_with_bindings` — handle new leaf and subject)
- Test: `code/digimon-engine/tests/dsl/predicate_face_down_stack_position.rs` (new)

- [ ] **Step 1: Write the failing test**

Create `code/digimon-engine/tests/dsl/predicate_face_down_stack_position.rs`:

```rust
//! Task A3: Source-subject predicates for face-down / bottom-source / host-kind.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::PlayerId;

mod common;
use common::{make_digimon, make_tamer};

#[test]
fn is_face_down_filter_excludes_face_up_sources() {
    // Setup: a Tamer with a face-down source AND a face-up source.
    // Then author a YAML that calls select_own_sources with `is_face_down: true`
    // and assert only the face-down source is offered as a candidate.
    // ...
    // See `code/digimon-engine/tests/source_multi/` for the existing select_own_sources
    // test pattern — copy it and add the new filter.
}
```

Use the existing `select_own_sources` test fixture pattern (`code/digimon-engine/tests/source_multi/`) as the template — the new test adds `filter: { is_face_down: true }` to the YAML and asserts the action mask exposes only the face-down source's selection slot.

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- is_face_down_filter
```

Expected: serde error "unknown field `is_face_down`".

- [ ] **Step 3: Add `is_face_down` to `PredicateSpec`**

In `code/digimon-dsl/src/predicate.rs:106` (after the existing source-relative leaves block):

```rust
// Leaf — source-subject (Phase A3 — Tamer face-down stash)
#[serde(skip_serializing_if = "Option::is_none")]
pub is_face_down: Option<bool>,
```

In `code/digimon-dsl/src/compiled.rs` (`CompiledPredicate`):

```rust
pub is_face_down: Option<bool>,
```

In `code/digimon-dsl/src/compile.rs` (the `PredicateSpec → CompiledPredicate` lowering):

```rust
is_face_down: spec.is_face_down,
```

- [ ] **Step 4: Add `PredicateSubject::Source` variant and wire eval**

In `code/digimon-engine/src/dsl_cards/predicate.rs:21`:

```rust
#[derive(Debug, Clone, Copy)]
pub enum PredicateSubject {
    Permanent(PermanentHandle),
    BreedingPermanent(PlayerId),
    Card(CardHandle),
    RevealedCard(CardHandle),
    Source(crate::selection::SourceSelectionRef),
    None,
}
```

In `eval_predicate_with_bindings` (after the existing source-relative-fields block — search for `source_is_tamer`):

```rust
if let Some(want) = pred.is_face_down {
    let actual = match subject {
        PredicateSubject::Source(sref) => {
            // Look up the host permanent and read card_sources[sref.source_index].face_down
            let perm = match rctx.game.player(sref.permanent.player)
                .battle_area.get(sref.permanent.index as usize) {
                Some(p) => p,
                None => return false,
            };
            match perm.card_sources.get(sref.source_index) {
                Some(cs) => cs.face_down,
                None => return false,
            }
        }
        _ => return false, // is_face_down only applies to Source subjects
    };
    if actual != want {
        return false;
    }
}
```

- [ ] **Step 5: Wire `select_own_sources` filter to pass `PredicateSubject::Source`**

Locate `select_own_sources` filter-eval site (likely `code/digimon-engine/src/dsl_cards/step/selections.rs` near `select_own_sources` step impl). Replace the existing `PredicateSubject::Card(card_handle)` subject with `PredicateSubject::Source(source_ref)` so the new leaf can be evaluated. Existing card-subject leaves (`trait_has`, `kind`, etc.) must still work — add a fallback in `eval_predicate_with_bindings` that, for `Source` subjects, also evaluates card-subject leaves by looking up the source's `CardData` via `sref.card`.

This is the most delicate change in the plan. Pattern:

```rust
// In eval_predicate_with_bindings, before the existing subject-specific
// blocks, if subject is Source, evaluate card-subject leaves against
// sref.card:
let card_subject_for_source = if let PredicateSubject::Source(sref) = subject {
    Some(PredicateSubject::Card(sref.card))
} else {
    None
};
// ... and where card-subject leaves are evaluated, fall back to
// card_subject_for_source if the primary subject didn't match.
```

Document this dispatch behavior with a comment so future predicate-leaf authors understand the Source-subject card-leaf inheritance.

- [ ] **Step 6: Run the failing test**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- is_face_down_filter
```

Expected: PASS.

- [ ] **Step 7: Run full DSL test suite to confirm no regressions**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-dsl/Cargo.toml
```

Expected: all existing `select_own_sources` tests still pass — Source-subject card-leaf inheritance preserves the prior behavior for filters like `trait_has`, `kind`, etc.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-dsl/src/predicate.rs \
        code/digimon-dsl/src/compiled.rs \
        code/digimon-dsl/src/compile.rs \
        code/digimon-engine/src/dsl_cards/predicate.rs \
        code/digimon-engine/src/dsl_cards/step/selections.rs \
        code/digimon-engine/tests/dsl/predicate_face_down_stack_position.rs
git commit -m "dsl: add is_face_down source predicate + Source subject variant"
```

---

## Task A3.2: Add `is_bottom_source` predicate leaf

**Files:**
- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Test: extend `code/digimon-engine/tests/dsl/predicate_face_down_stack_position.rs`

- [ ] **Step 1: Add the test case**

```rust
#[test]
fn is_bottom_source_filter_excludes_non_zero_source_indices() {
    // Tamer with 3 sources (face-down at index 0, face-up at indices 1, 2).
    // YAML: select_own_sources with filter { is_bottom_source: true }.
    // Assert: only the index-0 source is offered.
}
```

- [ ] **Step 2: Run to verify fail**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- is_bottom_source_filter
```

Expected: serde error "unknown field `is_bottom_source`".

- [ ] **Step 3: Add `is_bottom_source` to `PredicateSpec` + `CompiledPredicate` + eval**

Mirror Task A3.1's structure. The eval logic:

```rust
if let Some(want) = pred.is_bottom_source {
    let actual = match subject {
        PredicateSubject::Source(sref) => sref.source_index == 0,
        _ => return false,
    };
    if actual != want {
        return false;
    }
}
```

- [ ] **Step 4: Run test to verify pass**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- is_bottom_source_filter
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-dsl/src/predicate.rs \
        code/digimon-dsl/src/compiled.rs \
        code/digimon-dsl/src/compile.rs \
        code/digimon-engine/src/dsl_cards/predicate.rs \
        code/digimon-engine/tests/dsl/predicate_face_down_stack_position.rs
git commit -m "dsl: add is_bottom_source source predicate"
```

---

## Task A3.3: Add `host_kind_is` predicate leaf

**Files:**
- Same surface as A3.1/A3.2.

- [ ] **Step 1: Add the test case**

```rust
#[test]
fn host_kind_is_tamer_excludes_digimon_host_sources() {
    // Player has one Tamer with 1 source AND one Digimon with 1 source.
    // YAML: select_own_sources with filter { host_kind_is: tamer }.
    // Assert: only the Tamer-hosted source appears.
}
```

- [ ] **Step 2: Run to verify fail**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- host_kind_is_tamer
```

Expected: serde error.

- [ ] **Step 3: Add `host_kind_is: Option<CardKind>` to `PredicateSpec` + eval**

```rust
if let Some(want_kind) = pred.host_kind_is {
    let actual_kind = match subject {
        PredicateSubject::Source(sref) => {
            let perm = match rctx.game.player(sref.permanent.player)
                .battle_area.get(sref.permanent.index as usize) {
                Some(p) => p,
                None => return false,
            };
            let top = perm.card_sources.last();
            match top {
                Some(cs) => {
                    let data = &rctx.game.card_data[cs.data_index];
                    data.card_kind
                }
                None => return false,
            }
        }
        _ => return false,
    };
    let want_engine_kind: crate::enums::CardKind = compile_card_kind(want_kind);
    if actual_kind != want_engine_kind {
        return false;
    }
}
```

(Match the `CompiledCardKind` → `CardKind` lowering pattern used elsewhere in the file.)

- [ ] **Step 4: Run test to verify pass**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- host_kind_is_tamer
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-dsl/src/predicate.rs \
        code/digimon-dsl/src/compiled.rs \
        code/digimon-dsl/src/compile.rs \
        code/digimon-engine/src/dsl_cards/predicate.rs \
        code/digimon-engine/tests/dsl/predicate_face_down_stack_position.rs
git commit -m "dsl: add host_kind_is source predicate"
```

---

## Task A3.4: Add `has_face_down_source` permanent predicate leaf

**Files:**
- Same surface as A3.1/A3.2/A3.3, but the new leaf is a Permanent-subject predicate.

- [ ] **Step 1: Add the test case**

```rust
#[test]
fn has_face_down_source_filter_excludes_empty_stack_tamers() {
    // Player has two Tamers — one with a face-down source, one with no sources.
    // YAML: select_own_permanent with filter { kind: tamer, has_face_down_source: true }.
    // Assert: only the stacked Tamer is offered.
}

#[test]
fn has_face_down_source_filter_excludes_face_up_only_stacks() {
    // Player has one Tamer with only face-up sources.
    // YAML same as above. Assert: no Tamer is offered.
}
```

- [ ] **Step 2: Run to verify fail**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- has_face_down_source
```

Expected: serde error.

- [ ] **Step 3: Add `has_face_down_source: Option<bool>` to `PredicateSpec` + eval**

```rust
if let Some(want) = pred.has_face_down_source {
    let actual = match subject {
        PredicateSubject::Permanent(handle) => {
            let perm = match rctx.game.player(handle.player)
                .battle_area.get(handle.index as usize) {
                Some(p) => p,
                None => return false,
            };
            perm.card_sources.iter().any(|cs| cs.face_down)
        }
        PredicateSubject::BreedingPermanent(player) => {
            match &rctx.game.player(player).breeding_area {
                Some(b) => b.card_sources.iter().any(|cs| cs.face_down),
                None => return false,
            }
        }
        _ => return false,
    };
    if actual != want {
        return false;
    }
}
```

- [ ] **Step 4: Run test to verify pass**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- has_face_down_source
```

Expected: PASS (both cases).

- [ ] **Step 5: Commit**

```bash
git add code/digimon-dsl/src/predicate.rs \
        code/digimon-dsl/src/compiled.rs \
        code/digimon-dsl/src/compile.rs \
        code/digimon-engine/src/dsl_cards/predicate.rs \
        code/digimon-engine/tests/dsl/predicate_face_down_stack_position.rs
git commit -m "dsl: add has_face_down_source permanent predicate"
```

---

## Task A4.1: Add `EffectContext::trash_bottom_face_down_source` engine helper

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs` (insert near `trash_card_source` / `trash_top_source`)
- Test: `code/digimon-engine/tests/effect_context/trash_bottom_face_down_source.rs` (new)

- [ ] **Step 1: Write the failing test**

Create `code/digimon-engine/tests/effect_context/trash_bottom_face_down_source.rs`:

```rust
//! Task A4.1: Engine helper `trash_bottom_face_down_source(target)` —
//! pops `target.card_sources[0]` only if face-down, routes to owner trash,
//! fires `OnDigivolutionCardTrashed` with `event_host_permanent = target`.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming, PlayerId};

mod common;
use common::{make_digimon, make_tamer};

#[test]
fn trash_bottom_face_down_source_pops_and_routes_to_owner_trash() {
    let mut runner = DebugRunner::new();
    runner.register_card(make_tamer("TAMER-1"));
    runner.register_card(make_digimon("STASH-1"));

    let tamer = runner.spawn_permanent(PlayerId::P1, "TAMER-1");
    runner.stash_face_down_under_permanent(tamer, "STASH-1");

    let trash_count_before = runner.game().player(PlayerId::P1).trash.len();

    let mut ctx = runner.effect_context(PlayerId::P1);
    let trashed = ctx.trash_bottom_face_down_source(tamer);
    assert!(trashed, "must succeed when a face-down bottom source exists");

    let trash_count_after = runner.game().player(PlayerId::P1).trash.len();
    assert_eq!(trash_count_after, trash_count_before + 1);

    let tamer_perm = &runner.game().player(PlayerId::P1).battle_area[tamer.index as usize];
    assert_eq!(tamer_perm.card_sources.len(), 0, "Tamer stack must now be empty");
}

#[test]
fn trash_bottom_face_down_source_no_face_down_returns_false() {
    let mut runner = DebugRunner::new();
    runner.register_card(make_tamer("TAMER-1"));
    runner.register_card(make_digimon("STASH-1"));

    let tamer = runner.spawn_permanent(PlayerId::P1, "TAMER-1");
    runner.stash_face_up_under_permanent(tamer, "STASH-1");

    let mut ctx = runner.effect_context(PlayerId::P1);
    let trashed = ctx.trash_bottom_face_down_source(tamer);
    assert!(!trashed, "face-up bottom source must NOT be trashed");

    let tamer_perm = &runner.game().player(PlayerId::P1).battle_area[tamer.index as usize];
    assert_eq!(tamer_perm.card_sources.len(), 1, "face-up source must remain");
}

#[test]
fn trash_bottom_face_down_source_fires_on_digivolution_card_trashed_with_tamer_host() {
    // Register an observer on OnDigivolutionCardTrashed; assert
    // event_host_permanent == tamer when the trash fires.
    // ... (use existing OnDigivolutionCardTrashed test pattern from
    // code/digimon-engine/tests/timing_dispatch/on_digivolution_card_trashed.rs)
}
```

You'll need `runner.stash_face_down_under_permanent` and `runner.stash_face_up_under_permanent` test helpers — add them to `code/digimon-engine/tests/effect_context/common.rs` if not already present. They should just push a `CardSource` with `face_down: true|false` onto `perm.card_sources[0]`.

- [ ] **Step 2: Run to verify fail**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- trash_bottom_face_down_source
```

Expected: compile error "no method named `trash_bottom_face_down_source`".

- [ ] **Step 3: Add the helper**

Insert after the existing `trash_card_source` / `trash_top_source` helpers in `code/digimon-engine/src/effect_context/mod.rs`:

```rust
/// Trash the bottom-most face-down digivolution source from `target` and
/// fire `OnDigivolutionCardTrashed` per player. Returns `true` iff a
/// face-down source was found at index 0 and trashed; returns `false`
/// (no mutation) otherwise.
///
/// This is the cost-form trash primitive for ST-23 BEATBREAK and ST-24
/// DATA SQUAD cards whose printed text reads "by trashing the bottom
/// face-down card from under any of your Tamers, ..." — the upstream
/// permanent selection (`select_own_permanent { kind: tamer,
/// has_face_down_source: true }`) gates eligibility; this helper applies
/// the trash to the player's chosen target.
///
/// The trashed source carries its owner (`CardSource.owner`); the routing
/// follows that owner's trash, matching the standard
/// `OnDigivolutionCardTrashed` semantics established by
/// `Game::return_to_hand` (RUST_ENGINE_GAPS.md Rocks refresh 2026-04-29).
///
/// Used by: ST23-01 Kekkomon, ST23-03 Cougarmon, ST23-04 Murasamemon,
/// ST23-08 Monarchlizamon, ST23-11 Wolvermon, ST23-12 Chiropmon,
/// ST24-01 Koromon, ST24-06 RizeGreymon, ST24-10 Lilamon, ST24-11 Rosemon,
/// ST24-12 Falcomon.
pub fn trash_bottom_face_down_source(&mut self, target: PermanentHandle) -> bool {
    let target_player_id = target.player;
    let target_idx = target.index as usize;

    // Inspect-without-mutate to confirm the bottom source is face-down.
    let perm = match self.game.player(target_player_id).battle_area.get(target_idx) {
        Some(p) => p,
        None => return false,
    };
    let bottom = match perm.card_sources.first() {
        Some(cs) if cs.face_down => cs.clone(),
        _ => return false,
    };

    // Remove from stack.
    let perm_mut = &mut self.game.player_mut(target_player_id).battle_area[target_idx];
    let trashed = perm_mut.card_sources.remove(0);

    // Capture the host's identity for the trigger context.
    let host_top_card = self.game.player(target_player_id).battle_area[target_idx]
        .top_card_handle()
        .or_else(|| {
            // Edge: target had only the one face-down bottom source; after
            // its removal the stack is empty. Use the Tamer's top card —
            // which is the Tamer itself for printed-Tamer permanents.
            Some(bottom.handle())
        });

    // Route to the source owner's trash (NOT the host owner's — DCGO
    // `IsCardOwnerSelf` parity; ownership is stable across stash).
    self.game.player_mut(trashed.owner).trash.push(trashed);

    // Fire OnDigivolutionCardTrashed with event_host_permanent = target.
    self.game.fire_digivolution_card_trashed(
        bottom.handle(),                              // event_card / event_source_card
        target,                                       // event_host_permanent
        host_top_card,                                // event_host_card
        crate::trigger_context::EventCause::Effect,   // cause
    );

    true
}
```

(Match the existing `fire_digivolution_card_trashed` signature — grep `code/digimon-engine/src/game_actions.rs` for the exact parameter list and adapt the call shape.)

- [ ] **Step 4: Run test to verify pass**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- trash_bottom_face_down_source
```

Expected: PASS (all three cases).

- [ ] **Step 5: Run full effect_context test suite**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context
```

Expected: no regressions.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/effect_context/mod.rs \
        code/digimon-engine/tests/effect_context/trash_bottom_face_down_source.rs \
        code/digimon-engine/tests/effect_context/common.rs
git commit -m "engine: add EffectContext::trash_bottom_face_down_source helper"
```

---

## Task A4.2: Add DSL verb `trash_bottom_face_down_source_under_tamer`

**Files:**
- Modify: `code/digimon-dsl/src/step.rs` (new `StepSpec` variant + args struct)
- Modify: `code/digimon-dsl/src/compiled.rs` (new `CompiledStep` variant)
- Modify: `code/digimon-dsl/src/compile.rs` (lowering)
- Modify: `code/digimon-engine/src/dsl_cards/step/selections.rs` (execution — install select + dispatch trash)
- Test: `code/digimon-engine/tests/dsl/trash_bottom_face_down_source_under_tamer.rs` (new)

- [ ] **Step 1: Write the failing test**

Create `code/digimon-engine/tests/dsl/trash_bottom_face_down_source_under_tamer.rs`:

```rust
//! Task A4.2: DSL verb `trash_bottom_face_down_source_under_tamer: { of: you }`
//! installs `select_own_permanent { kind: tamer, has_face_down_source: true }`
//! then trashes the bottom face-down source from the picked Tamer.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::PlayerId;

mod common;
use common::{dsl_smoke, make_digimon, make_tamer};

#[test]
fn trash_bottom_face_down_source_under_tamer_picks_eligible_tamer() {
    let yaml = r#"
card: TEST-COST-1
name: TestTrashCost
kind: digimon
level: 4
color: [green]
cost: 4
dp: 4000

effects:
  - when: on_play
    process:
      - trash_bottom_face_down_source_under_tamer: { of: you }
"#;
    let mut runner = dsl_smoke::load_card(yaml);
    runner.register_card(make_tamer("TAMER-1"));
    runner.register_card(make_digimon("STASH-1"));

    // Place a Tamer with a face-down source. No other Tamers exist.
    let tamer = runner.spawn_permanent(PlayerId::P1, "TAMER-1");
    runner.stash_face_down_under_permanent(tamer, "STASH-1");

    runner.play_from_hand_as_digimon(PlayerId::P1, "TEST-COST-1");
    runner.resolve_pending(); // single-Tamer case must auto-resolve the select

    // Tamer's stack is now empty.
    let t = &runner.game().player(PlayerId::P1).battle_area[tamer.index as usize];
    assert_eq!(t.card_sources.len(), 0);
    // The stashed card landed in P1's trash.
    let trash = &runner.game().player(PlayerId::P1).trash;
    assert_eq!(trash.len(), 1);
    assert_eq!(trash[0].card_id(&runner.game().card_data), "STASH-1");
}

#[test]
fn trash_bottom_face_down_source_under_tamer_skips_when_no_eligible_tamer() {
    let yaml = r#"
card: TEST-COST-2
name: TestTrashCostSkip
kind: digimon
level: 4
color: [green]
cost: 4
dp: 4000

effects:
  - when: on_play
    optional: true
    process:
      - trash_bottom_face_down_source_under_tamer: { of: you }
      - gain_memory: 1
"#;
    let mut runner = dsl_smoke::load_card(yaml);
    runner.register_card(make_tamer("TAMER-1"));

    // Tamer present but EMPTY stack — cost is unpayable.
    runner.spawn_permanent(PlayerId::P1, "TAMER-1");

    let memory_before = runner.game().memory;
    runner.play_from_hand_as_digimon(PlayerId::P1, "TEST-COST-2");
    runner.resolve_pending();

    // No eligible Tamer → optional clause declines, gain_memory does NOT fire.
    assert_eq!(runner.game().memory, memory_before);
}
```

- [ ] **Step 2: Run to verify fail**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- trash_bottom_face_down_source_under_tamer
```

Expected: serde error "unknown variant `trash_bottom_face_down_source_under_tamer`".

- [ ] **Step 3: Add DSL types**

In `code/digimon-dsl/src/step.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashBottomFaceDownSourceUnderTamerArgs {
    pub of: PlayerRef,
}
```

And add the enum variant + visit-map arm following the existing patterns. Search the file for `StepSpec::TrashTopSource` and mirror its registration in the manual `Deserialize`.

In `code/digimon-dsl/src/compiled.rs`:

```rust
TrashBottomFaceDownSourceUnderTamer {
    of: CompiledPlayerRef,
},
```

In `code/digimon-dsl/src/compile.rs`:

```rust
S::TrashBottomFaceDownSourceUnderTamer(a) => CompiledStep::TrashBottomFaceDownSourceUnderTamer {
    of: compile_player_ref(&a.of, ctx)?,
},
```

- [ ] **Step 4: Add execution arm**

In `code/digimon-engine/src/dsl_cards/step/selections.rs` (the file currently containing `install_select_own_permanent` / `install_select_own_sources`), add the dispatch. Pattern:

```rust
CompiledStep::TrashBottomFaceDownSourceUnderTamer { of } => {
    let player = resolve_player_ref(of, ctx);
    // Install select_own_permanent { kind: tamer, has_face_down_source: true }
    // with a callback that calls ctx.trash_bottom_face_down_source(picked).
    let pred = CompiledPredicate {
        kind: Some(CompiledCardKind::Tamer),
        has_face_down_source: Some(true),
        ..Default::default()
    };
    install_select_own_permanent(
        ctx,
        bindings,
        player,
        pred,
        "Choose a Tamer (has face-down source)",
        |ctx, picked| {
            ctx.trash_bottom_face_down_source(picked);
        },
    )
}
```

(Match the precise install-helper signature in the surrounding file — `install_select_own_permanent`'s actual API may differ; treat this as pseudocode and inline the trash call into whatever continuation shape the file uses. The auto-resolve-single-candidate behavior is already built into `install_select_own_permanent`; declining when no Tamer is eligible relies on the outer `optional: true` clause.)

- [ ] **Step 5: Run test to verify pass**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- trash_bottom_face_down_source_under_tamer
```

Expected: PASS (both cases).

- [ ] **Step 6: Run full DSL suite**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-dsl/Cargo.toml
```

Expected: no regressions.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-dsl/src/step.rs \
        code/digimon-dsl/src/compiled.rs \
        code/digimon-dsl/src/compile.rs \
        code/digimon-engine/src/dsl_cards/step/selections.rs \
        code/digimon-engine/tests/dsl/trash_bottom_face_down_source_under_tamer.rs
git commit -m "dsl: add trash_bottom_face_down_source_under_tamer verb"
```

---

## Task A5.1: Tamer-host `OnDigivolutionCardTrashed` dispatch coverage test

**Files:**
- Test: `code/digimon-engine/tests/timing_dispatch/tamer_host_source_trashed.rs` (new)
- Modify (if test fails): `code/digimon-engine/src/effect_queue.rs` (`SourceTrashedFromStack` arm — extend fan-out to include Tamer-host stacks)

This task verifies the substrate built in A1–A4 fires `OnDigivolutionCardTrashed` correctly when the host permanent is a Tamer (not a Digimon). The existing Rocks refresh closed Digimon-host coverage; Tamer-host is the new case.

- [ ] **Step 1: Write the test**

Create `code/digimon-engine/tests/timing_dispatch/tamer_host_source_trashed.rs`:

```rust
//! Task A5: Tamer-host `OnDigivolutionCardTrashed` dispatch — when an effect
//! trashes a source from under a Tamer (as opposed to under a Digimon), the
//! observer fires with `event_host_permanent` set to the Tamer and
//! `event_card` set to the trashed source.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::PlayerId;
use digimon_engine::trigger_context::TriggerContextEventCard;

mod common;
use common::{make_digimon, make_tamer};

#[test]
fn tamer_host_source_trash_fires_with_correct_event_context() {
    let mut runner = DebugRunner::new();
    runner.register_card(make_tamer("TAMER-1"));
    runner.register_card(make_digimon("STASH-1"));

    let tamer = runner.spawn_permanent(PlayerId::P1, "TAMER-1");
    runner.stash_face_down_under_permanent(tamer, "STASH-1");

    // Set up a one-shot observer on OnDigivolutionCardTrashed. Use the
    // existing test pattern from
    // code/digimon-engine/tests/timing_dispatch/on_digivolution_card_trashed.rs.
    let mut observer_log: Vec<(String, u8)> = Vec::new();
    runner.install_observer_on_digivolution_card_trashed(|ctx| {
        let host_perm = ctx.event_host_permanent().expect("host must be populated");
        let host_card = ctx.event_host_card().map(|h| h.0).unwrap_or(u16::MAX as u16);
        observer_log.push((format!("{:?}", host_perm), host_card as u8));
    });

    let mut ctx = runner.effect_context(PlayerId::P1);
    let trashed = ctx.trash_bottom_face_down_source(tamer);
    assert!(trashed);
    runner.drain_effect_queue();

    assert_eq!(observer_log.len(), 1, "observer must fire exactly once");
    // Assert event_host_permanent == tamer (formatted check).
    assert!(
        observer_log[0].0.contains(&format!("{}", tamer.index)),
        "event_host_permanent must reference the Tamer's permanent index"
    );
}

#[test]
fn tamer_host_source_trash_does_not_alias_shifted_permanent() {
    // Edge case from Rocks refresh: removed-stack handles must not alias
    // shifted battle-area permanents. Tamer-host parity coverage.
    // Setup: trash a Tamer's face-down source, then play another Tamer/Digimon.
    // The freshly-played permanent must NOT be reported as the host by any
    // subsequent observer query.
    // ... (mirror code/digimon-engine/tests/timing_dispatch/
    // source_trash_host_context_does_not_alias_shifted_permanent)
}
```

(The exact observer-installation API differs; consult `code/digimon-engine/tests/timing_dispatch/on_digivolution_card_trashed.rs` for the canonical pattern and copy it.)

- [ ] **Step 2: Run the test**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- tamer_host_source_trash
```

If PASS: skip to Step 4. Tamer-host dispatch already works (likely outcome — the existing `fire_digivolution_card_trashed` is permanent-kind-agnostic).

If FAIL: continue to Step 3.

- [ ] **Step 3: If failing — extend the dispatch fan-out**

Locate the `SourceTrashedFromStack` arm in `code/digimon-engine/src/effect_queue.rs::dispatch`. If the fan-out filters on `permanent.kind == Digimon` (it shouldn't, but verify), remove the filter so Tamer-host stacks also receive the observer.

- [ ] **Step 4: Commit**

```bash
git add code/digimon-engine/tests/timing_dispatch/tamer_host_source_trashed.rs
git commit -m "engine: cover Tamer-host OnDigivolutionCardTrashed dispatch"
```

---

## Task A5.2: Documentation pass — update `RUST_ENGINE_API.md` and `RUST_ENGINE_GAPS.md`

**Files:**
- Modify: `docs/RUST_ENGINE_API.md` §3 (Field mutations + zone manipulation + selection helpers)
- Modify: `docs/RUST_ENGINE_API.md` §5 (`PredicateSpec`)
- Modify: `docs/RUST_ENGINE_GAPS.md` (annotate the Tamer face-down stash substrate entry with sub-phase landings)

- [ ] **Step 1: Update `RUST_ENGINE_API.md` §3 — Field mutations**

Add documentation for the new `face_down: bool` parameter on `place_card_under_permanent_bottom` and `place_as_bottom_source`. Add new entries for `place_deck_top_under_permanent(target, face_down)` and `trash_bottom_face_down_source(target)`.

Sample doc block (mirror surrounding style):

```markdown
#### `ctx.place_deck_top_under_permanent(target: PermanentHandle, face_down: bool) -> Option<CardHandle>`

Place the top card of `target.player`'s deck as the bottom digivolution
source of `target`. Returns `Some(card_handle)` on success or `None` if
the controller's deck is empty (silent no-op on empty deck).

Used by ST-23 BEATBREAK / ST-24 DATA SQUAD Tamer-stash placement cards.

#### `ctx.trash_bottom_face_down_source(target: PermanentHandle) -> bool`

Trash the bottom-most face-down digivolution source from `target` and
fire `OnDigivolutionCardTrashed`. Returns `true` iff a face-down source
was found at index 0 and trashed. The trashed source routes to the
source's owner trash (not the host's), matching DCGO `IsCardOwnerSelf`
parity.

Used by ST-23 BEATBREAK / ST-24 DATA SQUAD Tamer-stash cost-form cards.
```

- [ ] **Step 2: Update `RUST_ENGINE_API.md` §5 — `PredicateSpec`**

Add the four new leaves under the existing source-relative / permanent-only sections:

```markdown
**Source-subject predicates (Phase A3 — Tamer face-down stash):**
- `is_face_down: Option<bool>` — matches `CardSource.face_down`.
- `is_bottom_source: Option<bool>` — matches `source_index == 0`.
- `host_kind_is: Option<CardKind>` — matches the host permanent's top
  card kind.

**Permanent-only:**
- `has_face_down_source: Option<bool>` — matches whether `card_sources`
  contains at least one face-down source.
```

- [ ] **Step 3: Annotate `RUST_ENGINE_GAPS.md`**

In the "BEATBREAK / DATA SQUAD Tamer face-down stash substrate" entry, add a status footer:

```markdown
- **Status — Phase A landings (2026-05-17 → ...):**
  - **A1 (face_down axis on placement helpers):** landed PR #...
  - **A2 (place_deck_top_under_permanent + DeckTop DSL binding):** landed PR #...
  - **A3 (is_face_down / is_bottom_source / host_kind_is / has_face_down_source predicates):** landed PR #...
  - **A4 (trash_bottom_face_down_source + DSL verb):** landed PR #...
  - **A5 (Tamer-host OnDigivolutionCardTrashed coverage):** landed PR #...
```

(Update the PR numbers as each task lands.)

- [ ] **Step 4: Commit**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md
git commit -m "docs: document Phase A Tamer face-down stash substrate"
```

---

## Acceptance check

After all tasks land:

- [ ] **Full engine test suite passes:**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

- [ ] **Full DSL test suite passes:**

```bash
cargo test --manifest-path code/digimon-dsl/Cargo.toml
```

- [ ] **Worked-card spot check:** Write a one-off behavioral test for ST23-12 Chiropmon (the cleanest worked example — only the Tamer-stash cost-form, no other gaps). Author the YAML using the new substrate (`trash_bottom_face_down_source_under_tamer` cost + `return_from_trash` body) and verify it resolves end-to-end. This test is not yet part of regression but should pass after Phase A lands. Card text:

> [On Play] By trashing the bottom face-down card from under any of your Tamers, you may return 1 Digimon card with the [Glowing Dawn] trait from your trash to the hand.

- [ ] **Update `docs/RUST_ENGINE_GAPS.md` ST-23/ST-24 audit-index counts** to reflect the unblocked cards (run `/assess-archetype-rust` again to refresh, or hand-update the audit-index row).

---

## Out of scope for Phase A

- **Phases B–F** as described in the source spec. Each gets its own plan after A lands.
- **Authoring ST-23 / ST-24 card YAML files.** That belongs to a separate batch via `/batch-implement-cards-rust-dsl` once Phases A–F are all done.
- **DCGO behavioral verification beyond the substrate.** ST-23/ST-24 DCGO scripts at `DCGO/Assets/Scripts/CardEffect/ST23/` and `DCGO/Assets/Scripts/CardEffect/ST24/` should be consulted during card authoring, not during substrate land.
- **Python / RL parity.** None of these substrate changes have a Python counterpart; `docs/RUST_PYTHON_PARITY.md` requires no new entry. The observation tensor's existing face-down handling (zero-out `data_index` for `CardSource.face_down == true`) already covers Tamer-host face-down sources via the same code path as Digimon-host face-down sources.
