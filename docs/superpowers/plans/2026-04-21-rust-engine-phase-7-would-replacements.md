# Rust Engine Phase 7 — "Would" Replacement-Timings Framework

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce a first-class replacement-effect layer — `EffectTiming::Would*` variants + `ReplacementContext` + `PendingSelection::Replacement` — that lets effects intercept impending state changes (deletion, return-to-hand/deck, trash, de-digivolve, lose-security, draw, place-in-security) before they commit, with cause attribution, optional/mandatory semantics, and layered ordering. Unblocks Barrier, Evade, Partition, Armor Purge, Fragment(N), Decode keywords and the passive "cannot be X'd" modifier family deferred from Phase 6.

**Architecture:**
- Add 9 new `EffectTiming::Would*` variants (plus 2 more `WhenWouldAttack` / `WhenWouldBeAttackTarget` reserved for Phase 9, no dispatch in this plan).
- New `ReplacementContext<'g>` struct with `cause: ReplacementCause`, `subject: ReplacementSubject`, mutating helpers `cancel / redirect_to / substitute / handled` backed by an internal `ReplacementOutcome` enum.
- New `Game::try_replace(timing, subject, cause, original_destination) -> ReplacementOutcome` dispatcher with layering (controller-first, opponent-second), optional accept/decline via `PendingSelection::Replacement`, and a recursion-depth guard (`replacement_depth: u8` on `Game`, cap = 8).
- Wire fire-sites: `delete_permanent_with_effects` (combat.rs:1223), `return_to_hand` / `return_to_deck` (game_actions.rs), `drive_security_resolution` (combat.rs:989 area), `place_on_security` (game_actions.rs:1229), `EffectContext::draw` / `de_digivolve` / effect-driven trash.
- Migrate Phase 6 deferred passives (`CannotBeReturnedToDeck`, `CannotBeReturnedToHand`, `CannotBeTrashedByEffect`, `CannotBeDeDigivolved`) to auto-install replacements. Add `cause_filter: Option<ReplacementCause>` + `replacement_condition: Option<ReplacementConditionFn>` to `ModifierEntry` / `PlayerModifierEntry`.
- Native keyword parsing at `card_data` build: Barrier / Evade / Fragment(N) / Decode / Partition / ArmorPurge emit auto-install replacements.
- Zero new action IDs — `ACTION_SPACE_SIZE` stays 2168; `PendingSelection::Replacement` reuses `EffectChoice` (accept) and `PASS` (decline) ranges.

**Tech Stack:** Rust 2021 (`digimon-engine/`), DebugRunner test harness, existing `EffectContext` / `Effect` / `Modifier` patterns established in Phases 1–6.

**Spec:** [docs/superpowers/specs/2026-04-21-would-replacement-timings-design.md](../specs/2026-04-21-would-replacement-timings-design.md) — authoritative design; read before starting any task.

---

## Background

Phase 7 closes Cluster G from [`.claude/plans/recursive-coalescing-candle.md`](../../../.claude/plans/recursive-coalescing-candle.md):

- **~60 meta-pool cards** across 5 audited archetypes need replacement effects. Barrier (TS Olympos ~9), Evade (Dark Masters/TS Olympos ~6), Partition (~3), Armor Purge (Medusamon ~3), Fragment(N) (Rocks ~4), Decode (TS Olympos ~3), plus scattered "cannot be X'd by your opponent's effects" passives.
- Additionally, Phase 6 deferred 4 passive restriction modifiers (`CannotBeReturnedToDeck`, `CannotBeReturnedToHand`, `CannotBeTrashedByEffect`, `CannotBeDeDigivolved`) that are implemented here as automatic cancel-replacements.

**What exists today:**
- All observer timings are post-hoc: `OnDeletion`, `OnLeaveField`, `OnReturn`, `OnTrash`, `OnAnyDeletion` fire *after* the event, with no ability to mutate what happens.
- `delete_permanent_with_effects` (combat.rs:1223) enqueues `OnDeletion`, drains, deletes.
- `return_to_hand` / `return_to_deck` (game_actions.rs:610/658) mutate state and fire `OnDigivolutionCardTrashed` only — no pre-event dispatch.
- `drive_security_resolution` (combat.rs:884+) trashes the revealed card at `SecurityPhase::Dispose` with no replacement window.
- `Keyword::Barrier` and `Keyword::Partition` are declared in `enums.rs` but do nothing — no Keyword variants exist for Evade, Decode, Fragment, ArmorPurge.
- `ModifierEntry` / `PlayerModifierEntry` have no `cause_filter` or `replacement_condition` fields.
- `SelectionKind` has 14 variants; `Replacement` is not one of them.
- `ACTION_SPACE_SIZE` = 2168 and must remain unchanged.

**Design-principle reminders (from spec §3):**
1. **No auto-selection.** Optional replacements (Barrier, Evade, "may" effects) emit `PendingSelection::Replacement` with both accept and decline in `valid_action_ids`.
2. **Mandatory replacements do not emit a selection.** Passive "can't be returned to deck" silently cancels.
3. **Cause attribution is first-class** — derived at fire-site, not threaded. `security_resolution.is_some()` → `SecurityCheck`; explicit `Battle` from `resolve_battle`; else `OwnEffect`/`OpponentEffect` by comparing acting player to target controller.
4. **Layering follows printed rules.** Controller-of-affected-subject's replacements first (multiple → `TriggerOrder` selection), opponent-of-subject's replacements after. Each sees the post-replacement state of the prior.
5. **Replacement windows are atomic.** Replacements run to completion (cancel / redirect / substitute / handled) before observers fire.
6. **Observers fire based on the post-replacement reality.** Evade → no `OnDeletion` but yes `OnLeaveField`+`OnReturn`. Partition substitute → `OnDeletion` fires for the source, not the original permanent.
7. **Recursion guard: `replacement_depth` cap = 8** prevents infinite loops in pathological card interactions.

**Cards motivating Phase 7** (representative):
- BT10-109 Barrier-printed Cherubimon — native `Keyword::Barrier` → `WhenWouldBeDeleted` auto-install.
- EX1-040 SaberLeomon — Evade keyword (new).
- BT9-089 Beelzemon — Partition keyword.
- BT18-066 Rapidmon Ace — Decode keyword.
- Various DNA Omnimon / Medusamon — "Cannot be returned to deck by your opponent's effects" passive.
- Various TS Olympos — "Cannot be de-digivolved" passive.

---

## File Structure

**Modified:**
- `digimon-engine/src/enums.rs` — add 9 `EffectTiming::Would*` variants + 2 reserved; add `ReplacementCause` enum; extend `Keyword` with `Evade`, `Fragment(u8)`, `Decode`, `ArmorPurge`.
- `digimon-engine/src/selection.rs` — add `SelectionKind::Replacement`; add `GamePhase::SelectReplacement` if needed (may reuse `EffectChoice` phase — decide in Task 1).
- `digimon-engine/src/effect.rs` — add `Effect::when_would_be_deleted(card)` and sibling constructors; add `replacement_process: Option<ReplacementProcessFn>` field (parallel to `process`) — see Task 1.
- `digimon-engine/src/effect_context/mod.rs` — expose `EffectContext::acting_player() -> PlayerId` (already tracked internally); add `replacement` sub-module and `ReplacementContext` type.
- `digimon-engine/src/game.rs` — add `replacement_depth: u8` field, `try_replace` dispatcher, `layer_replacements` helper.
- `digimon-engine/src/combat.rs` — wire `WhenWouldBeDeleted` + `WhenWouldLeaveBattleArea` at `delete_permanent_with_effects` entry (line 1223); wire `WhenWouldLoseSecurity` at `drive_security_resolution::BattleResolved` boundary.
- `digimon-engine/src/game_actions.rs` — wire `WhenWouldLeaveBattleArea` + per-route Would timings at `return_to_hand` (610), `return_to_deck` (658), `place_on_security` (1229).
- `digimon-engine/src/modifiers.rs` — add `cause_filter: Option<ReplacementCause>` + `replacement_condition: Option<ReplacementConditionFn>` to `ModifierEntry` + `PlayerModifierEntry`; builder defaults preserve existing behavior.
- `digimon-engine/src/card_data.rs` — extend `parse_printed_keywords` for Evade, Fragment, Decode, ArmorPurge (Barrier already parsed per Phase 3).
- `digimon-engine/src/card_registry.rs` — at registry build, emit auto-install `WhenWouldBeDeleted` effects for permanents carrying `Keyword::Barrier / Evade / Fragment(N) / Partition / ArmorPurge` and `WhenWouldBeReturnedToDeck` / `WhenWouldBeReturnedToHand` for Decode.
- `digimon-engine/src/effect_queue.rs` — no changes expected (replacement dispatch is synchronous under `try_replace`, not via the queue).
- `docs/RUST_ENGINE_API.md` — new §Phase 7 section.
- `docs/RUST_PYTHON_PARITY.md` — §7 entry.
- `.claude/plans/recursive-coalescing-candle.md` — flip Phase 7 row to ✅ Landed.

**New source files:**
- `digimon-engine/src/replacement.rs` — `ReplacementContext`, `ReplacementOutcome`, `ReplacementSubject`, `ReplacementCause`, `ReplacementConditionFn` type alias, `try_replace` implementation body (called from `Game`).

**New tests:**
- `digimon-engine/tests/replacements/main.rs` — module harness.
- `digimon-engine/tests/replacements/enum_and_context.rs` — Task 1 shape tests.
- `digimon-engine/tests/replacements/dispatcher_core.rs` — Task 2 try_replace / layering / recursion-guard tests.
- `digimon-engine/tests/replacements/deletion_replacements.rs` — Task 3 (Barrier / Evade / Partition / ArmorPurge / Fragment against delete path).
- `digimon-engine/tests/replacements/route_replacements.rs` — Task 4 (return-to-hand/deck, trash, de-digi, lose-security, draw, place-in-security).
- `digimon-engine/tests/replacements/passive_modifier_migration.rs` — Task 5 (auto-install replacements for Phase 6 deferred variants).
- `digimon-engine/tests/replacements/native_keywords.rs` — Task 6 (printed Barrier / Evade / Fragment / Decode / Partition / ArmorPurge parse + fire).
- `digimon-engine/tests/replacements/behavioral_end_to_end.rs` — Task 7 full archetype scenario (TS Olympos Cherubimon + opponent Mega Flame).

**Cargo wiring:**
- Add `[[test]] name = "replacements" path = "tests/replacements/main.rs"` to `digimon-engine/Cargo.toml`.

---

## Tasks

### Task 1: Enum + data types — no dispatch

**Files:**
- Modify: `digimon-engine/src/enums.rs` — add `EffectTiming::WhenWouldBeDeleted / WhenWouldLeaveBattleArea / WhenWouldBeReturnedToHand / WhenWouldBeReturnedToDeck / WhenWouldBeTrashed / WhenWouldBeDeDigivolved / WhenWouldLoseSecurity / WhenWouldDraw / WhenWouldPlaceInSecurity` + reserved `WhenWouldAttack / WhenWouldBeAttackTarget`. Extend `Keyword` with `Evade`, `Fragment(u8)`, `Decode`, `ArmorPurge`.
- Modify: `digimon-engine/src/selection.rs` — add `SelectionKind::Replacement`.
- Create: `digimon-engine/src/replacement.rs` — `ReplacementCause`, `ReplacementSubject`, `ReplacementOutcome`, `ReplacementContext<'g>`, `ReplacementConditionFn` type alias. No dispatcher yet.
- Modify: `digimon-engine/src/lib.rs` — `pub mod replacement;`.
- Modify: `digimon-engine/src/effect.rs` — add builder constructors (`Effect::when_would_be_deleted(card)` etc.) that return an `EffectBuilder` with timing preset; add `replacement_process: Option<ReplacementProcessFn>` field + `.replacement_process(...)` builder. (Keep `process` unchanged — replacement processes receive a different ctx type.)
- Modify: `digimon-engine/src/modifiers.rs` — add `cause_filter: Option<ReplacementCause>` + `replacement_condition: Option<ReplacementConditionFn>` to `ModifierEntry` + `PlayerModifierEntry`. Defaults: `None` / `None`. Existing call-sites need explicit `cause_filter: None, replacement_condition: None` or a builder with defaults (choose builder — less churn).
- Create: `digimon-engine/tests/replacements/main.rs` — module harness.
- Create: `digimon-engine/tests/replacements/enum_and_context.rs` — shape tests.
- Modify: `digimon-engine/Cargo.toml` — register the new `[[test]]` target.

**Key type definitions to land in `replacement.rs`:**

```rust
//! Replacement-effect framework — "Would*" timings + dispatcher.
//!
//! See docs/superpowers/specs/2026-04-21-would-replacement-timings-design.md.

use crate::card_source::CardHandle;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{PlayerId, Zone};
use crate::permanent::PermanentHandle;

/// Why a state change is happening — consumed by replacement effects that
/// filter on cause (e.g. "cannot be trashed by your opponent's effects").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReplacementCause {
    Battle,
    OwnEffect,
    OpponentEffect,
    SecurityCheck,
    Cost,
}

/// What's about to happen — a permanent leaving the field, a card being
/// trashed from hand, a player about to draw, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementSubject {
    Permanent(PermanentHandle),
    Card(CardHandle, Zone),
    Player(PlayerId),
}

/// The outcome a replacement effect sets. Mutually exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementOutcome {
    None,
    Cancelled,
    Redirected(Zone),
    Substituted(ReplacementSubject),
    CustomHandled,
}

/// Closure type for passive modifier replacement conditions. Evaluated at
/// try_replace time to decide whether the modifier applies.
pub type ReplacementConditionFn =
    Box<dyn Fn(&EffectReadContext, &ReplacementSubject) -> bool + Send + Sync + 'static>;

/// Closure type for replacement effect processes. Receives a
/// ReplacementContext so the process can mutate state AND set the outcome.
pub type ReplacementProcessFn =
    Box<dyn Fn(&mut ReplacementContext<'_>) + Send + Sync + 'static>;

/// Passed to Would* effect processes. `effect` is the underlying effect ctx;
/// `cause`, `subject`, `original_destination` are snapshot event data; the
/// process sets `outcome` via helpers to tell the dispatcher what to do.
pub struct ReplacementContext<'g> {
    pub effect: &'g mut EffectContext<'g>,
    pub cause: ReplacementCause,
    pub subject: ReplacementSubject,
    pub original_destination: Option<Zone>,
    pub(crate) outcome: ReplacementOutcome,
}

impl<'g> ReplacementContext<'g> {
    pub fn cancel(&mut self) {
        self.outcome = ReplacementOutcome::Cancelled;
    }
    pub fn redirect_to(&mut self, dest: Zone) {
        self.outcome = ReplacementOutcome::Redirected(dest);
    }
    pub fn substitute(&mut self, subject: ReplacementSubject) {
        self.outcome = ReplacementOutcome::Substituted(subject);
    }
    pub fn handled(&mut self) {
        self.outcome = ReplacementOutcome::CustomHandled;
    }
}
```

**`EffectTiming` additions (in `enums.rs`, inside the `EffectTiming` enum):**
```rust
// ── Phase 7 "Would*" replacement timings ──────────────────────────────
// Dispatched via Game::try_replace before the state change commits. See
// replacement.rs and docs/superpowers/specs/2026-04-21-would-replacement-timings-design.md.
WhenWouldBeDeleted,
WhenWouldLeaveBattleArea,
WhenWouldBeReturnedToHand,
WhenWouldBeReturnedToDeck,
WhenWouldBeTrashed,
WhenWouldBeDeDigivolved,
WhenWouldLoseSecurity,
WhenWouldDraw,
WhenWouldPlaceInSecurity,
// Reserved — Phase 9 wires dispatch. Added here so downstream match
// arms stay exhaustive and action-ID numbering doesn't shift.
WhenWouldAttack,
WhenWouldBeAttackTarget,
```

**`Keyword` additions:**
```rust
// Phase 7 — replacement-backed keywords. Printed parsing lands Task 6.
Evade,
Fragment(u8),
Decode,
ArmorPurge,
// (Barrier + Partition already exist; reuse.)
```

**`SelectionKind::Replacement` addition:**
```rust
/// Player may accept or decline an optional replacement effect. Backed by
/// EffectChoice action range (accept) + PASS (decline). `valid_action_ids`
/// holds exactly one ACCEPT entry; `is_optional = true` admits PASS.
Replacement,
```

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/replacements/main.rs`:
```rust
mod enum_and_context;
```

Create `digimon-engine/tests/replacements/enum_and_context.rs`:
```rust
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::replacement::{
    ReplacementCause, ReplacementContext, ReplacementOutcome, ReplacementSubject,
};
use digimon_engine::selection::SelectionKind;

#[test]
fn would_timings_exist() {
    let _ = EffectTiming::WhenWouldBeDeleted;
    let _ = EffectTiming::WhenWouldLeaveBattleArea;
    let _ = EffectTiming::WhenWouldBeReturnedToHand;
    let _ = EffectTiming::WhenWouldBeReturnedToDeck;
    let _ = EffectTiming::WhenWouldBeTrashed;
    let _ = EffectTiming::WhenWouldBeDeDigivolved;
    let _ = EffectTiming::WhenWouldLoseSecurity;
    let _ = EffectTiming::WhenWouldDraw;
    let _ = EffectTiming::WhenWouldPlaceInSecurity;
    let _ = EffectTiming::WhenWouldAttack;           // reserved
    let _ = EffectTiming::WhenWouldBeAttackTarget;   // reserved
}

#[test]
fn replacement_selection_kind_exists() {
    let _ = SelectionKind::Replacement;
}

#[test]
fn replacement_cause_variants_exist() {
    let _ = ReplacementCause::Battle;
    let _ = ReplacementCause::OwnEffect;
    let _ = ReplacementCause::OpponentEffect;
    let _ = ReplacementCause::SecurityCheck;
    let _ = ReplacementCause::Cost;
}

#[test]
fn replacement_outcome_defaults_none() {
    assert_eq!(ReplacementOutcome::None, ReplacementOutcome::None);
}

#[test]
fn new_keywords_exist() {
    let _ = Keyword::Evade;
    let _ = Keyword::Fragment(3);
    let _ = Keyword::Decode;
    let _ = Keyword::ArmorPurge;
}
```

Add to `digimon-engine/Cargo.toml`:
```toml
[[test]]
name = "replacements"
path = "tests/replacements/main.rs"
```

- [ ] **Step 2: Run — compile failures expected**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements`
Expected: FAIL — `enum_and_context` module fails to compile (unknown `EffectTiming::WhenWouldBeDeleted`, etc.).

- [ ] **Step 3: Implement enum + types**

In `digimon-engine/src/enums.rs`:
1. Add the 11 `EffectTiming::WhenWould*` variants inside the existing `EffectTiming` enum (copy list above into appropriate location — after `OnDigiXros` is a natural spot).
2. Extend the existing `Keyword` enum with `Evade`, `Fragment(u8)`, `Decode`, `ArmorPurge` (after `Collision`).
3. Audit `py_name()` and other exhaustive-match functions for the added variants and add `.py_name()` stubs (use identifier as name for Phase 7 variants — no Python equivalent yet).

In `digimon-engine/src/selection.rs`:
1. Add `Replacement` variant to `SelectionKind` enum (after `CountCappedMultiSelect`).
2. Audit exhaustive match arms (any `match kind` blocks) and add `Replacement => …` arms with `unimplemented!("Replacement handled via dedicated dispatch path")` for now — Task 2 replaces the stub.

Create `digimon-engine/src/replacement.rs` with the full type list above.

In `digimon-engine/src/lib.rs`: add `pub mod replacement;`.

In `digimon-engine/src/effect.rs`:
1. Add `pub replacement_process: Option<crate::replacement::ReplacementProcessFn>` field to `Effect`.
2. Initialize to `None` in `EffectBuilder::new`.
3. Add builder method:
   ```rust
   pub fn replacement_process<F>(mut self, f: F) -> Self
   where
       F: Fn(&mut crate::replacement::ReplacementContext<'_>) + Send + Sync + 'static,
   {
       self.inner.replacement_process = Some(Box::new(f));
       self
   }
   ```
4. Add constructors:
   ```rust
   pub fn when_would_be_deleted(card: CardHandle) -> EffectBuilder {
       EffectBuilder::new(card, EffectTiming::WhenWouldBeDeleted)
   }
   pub fn when_would_leave_battle_area(card: CardHandle) -> EffectBuilder {
       EffectBuilder::new(card, EffectTiming::WhenWouldLeaveBattleArea)
   }
   pub fn when_would_be_returned_to_hand(card: CardHandle) -> EffectBuilder {
       EffectBuilder::new(card, EffectTiming::WhenWouldBeReturnedToHand)
   }
   pub fn when_would_be_returned_to_deck(card: CardHandle) -> EffectBuilder {
       EffectBuilder::new(card, EffectTiming::WhenWouldBeReturnedToDeck)
   }
   pub fn when_would_be_trashed(card: CardHandle) -> EffectBuilder {
       EffectBuilder::new(card, EffectTiming::WhenWouldBeTrashed)
   }
   pub fn when_would_be_de_digivolved(card: CardHandle) -> EffectBuilder {
       EffectBuilder::new(card, EffectTiming::WhenWouldBeDeDigivolved)
   }
   pub fn when_would_lose_security(card: CardHandle) -> EffectBuilder {
       EffectBuilder::new(card, EffectTiming::WhenWouldLoseSecurity)
   }
   pub fn when_would_draw(card: CardHandle) -> EffectBuilder {
       EffectBuilder::new(card, EffectTiming::WhenWouldDraw)
   }
   pub fn when_would_place_in_security(card: CardHandle) -> EffectBuilder {
       EffectBuilder::new(card, EffectTiming::WhenWouldPlaceInSecurity)
   }
   ```

In `digimon-engine/src/modifiers.rs`:
1. Add fields to `ModifierEntry`:
   ```rust
   /// Cause filter for replacement-backed modifiers. None = cause-agnostic.
   pub cause_filter: Option<crate::replacement::ReplacementCause>,
   /// Optional runtime condition for passive replacements. None = always applies.
   pub replacement_condition: Option<crate::replacement::ReplacementConditionFn>,
   ```
2. Same two fields on `PlayerModifierEntry`.
3. `ModifierEntry` isn't `Clone` anymore because of the closure — verify callers; if cloning is required anywhere, introduce a builder and wrap the closure in `Arc`. Search: `grep "ModifierEntry {" digimon-engine/src`.
4. Remove `#[derive(Debug, Clone)]` from `ModifierEntry` and `PlayerModifierEntry`; add a manual `Debug` impl (mirror pattern in `effect.rs`).
5. Add a helper constructor to keep existing call-sites short:
   ```rust
   impl ModifierEntry {
       pub fn simple(modifier: ModifierType, value: i32, expiry: Expiry, source_player: u8) -> Self {
           Self {
               modifier, value, expiry, source_player,
               cause_filter: None, replacement_condition: None,
           }
       }
   }
   ```
   and similarly for `PlayerModifierEntry::simple(...)`.
6. Update every existing call-site that literal-constructed `ModifierEntry { ... }` or `PlayerModifierEntry { ... }` to use `::simple(...)` (search + replace: `grep -rn "ModifierEntry {" digimon-engine/src`). This keeps Phase 6 behavior unchanged.

In `digimon-engine/src/effect_context/mod.rs`:
1. Add a public accessor for the acting player used by Task 2 cause inference:
   ```rust
   pub fn acting_player(&self) -> PlayerId {
       self.player  // existing field
   }
   ```
   Check the field name — it may already be `pub`; if so this step is a no-op.

- [ ] **Step 4: Run — all Task 1 tests pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements`
Expected: PASS, 5 tests. Zero warnings.

- [ ] **Step 5: Full suite still green**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: 566 (Phase 6 + Phase 10 post-merge baseline) + 5 new = 571 passing. Zero warnings. Any modified call-sites of `ModifierEntry` should still compile.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/enums.rs digimon-engine/src/selection.rs digimon-engine/src/replacement.rs digimon-engine/src/lib.rs digimon-engine/src/effect.rs digimon-engine/src/modifiers.rs digimon-engine/src/effect_context/mod.rs digimon-engine/tests/replacements/main.rs digimon-engine/tests/replacements/enum_and_context.rs digimon-engine/Cargo.toml
git commit -m "rust-engine(phase-7): add Would* enum variants + ReplacementContext data types"
```

---

### Task 2: `try_replace` dispatcher + layering + recursion guard

**Files:**
- Modify: `digimon-engine/src/game.rs` — add `replacement_depth: u8` field (initialize to 0 in `Game::new` / `with_rules`); add `try_replace` pub(crate) method, `layer_replacements` helper, `collect_replacement_candidates` helper.
- Modify: `digimon-engine/src/replacement.rs` — implement `try_replace` body logic (function may live on `Game` or as a free function that takes `&mut Game` — pick whichever keeps borrow hygiene simpler; suggested: free function `pub(crate) fn try_replace(game: &mut Game, …)` in `replacement.rs`, called from `Game::try_replace` thin wrapper).
- Modify: `digimon-engine/src/effect_queue.rs` — add a helper `Game::effects_for_handle_at_timing(handle, timing) -> Vec<...>` if one doesn't already exist; the replacement collector needs it.
- Modify: `digimon-engine/src/selection.rs` — expand `SelectionKind::Replacement` match arms to emit a `PendingSelection` with 1 accept action ID (reusing `EffectChoice` range slot 0) + PASS as decline.
- Create: `digimon-engine/tests/replacements/dispatcher_core.rs` — unit tests using handcrafted test cards (new entries in `src/cards/test/`).

**Dispatcher signature (on `Game`):**
```rust
impl Game {
    /// Fire all applicable replacement effects for the given would-event.
    /// Returns the final ReplacementOutcome the caller should honor.
    ///
    /// Invariant: if this returns ReplacementOutcome::None, no side effects
    /// have been applied to Game state. If it returns any other variant,
    /// side effects from the chosen replacements have already committed and
    /// the caller must NOT re-apply the original event.
    pub(crate) fn try_replace(
        &mut self,
        timing: EffectTiming,
        subject: ReplacementSubject,
        cause: ReplacementCause,
        original_destination: Option<Zone>,
    ) -> ReplacementOutcome {
        replacement::try_replace_impl(self, timing, subject, cause, original_destination)
    }
}
```

**Free-function body (sketched in `replacement.rs`):**
```rust
pub(crate) fn try_replace_impl(
    game: &mut crate::game::Game,
    timing: EffectTiming,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
) -> ReplacementOutcome {
    // 1. Depth guard.
    if game.replacement_depth >= 8 {
        // Log and fall through: commit original event.
        return ReplacementOutcome::None;
    }
    game.replacement_depth += 1;
    let result = (|| -> ReplacementOutcome {
        // 2. Collect candidates (card-face effects + passive modifiers).
        let candidates = collect_candidates(game, timing, &subject, cause);
        if candidates.is_empty() {
            return ReplacementOutcome::None;
        }

        // 3. Layer (controller's first, opponent's after). If either side
        //    has >1, emit TriggerOrder selection; else run in collection order.
        let ordered = layer_candidates(game, candidates, &subject);

        // 4. Walk. Each candidate runs via run_candidate(); outcome threads
        //    through. Optional candidates emit PendingSelection::Replacement
        //    and pause the window (re-entered via resolve_selection).
        let mut outcome = ReplacementOutcome::None;
        for cand in ordered {
            outcome = run_candidate(game, cand, subject, cause, original_destination, outcome);
            if game.pending_selection.is_some() {
                // Selection installed — caller must spin. When the selection
                // resolves, the callback re-enters try_replace_impl via a
                // "resume" mechanism we install on Game.replacement_resume.
                // For v1, the callback fires the chosen-or-declined candidate
                // synchronously inside the callback, so control returns here
                // only after all replacements resolve.
                break;
            }
        }
        outcome
    })();
    game.replacement_depth -= 1;
    result
}
```

**Layering algorithm** (in `replacement.rs`):
```rust
fn layer_candidates(
    game: &mut crate::game::Game,
    candidates: Vec<Candidate>,
    subject: &ReplacementSubject,
) -> Vec<Candidate> {
    let subject_controller = subject_controller_id(game, subject);
    let (own, opp): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .partition(|c| c.source_controller == subject_controller);

    // If either side has >1 candidates, emit a TriggerOrder selection and
    // suspend until the controller picks. For v1 simplicity, run in
    // collection order when there's ambiguity only if >1 candidates from
    // the SAME controller.
    //
    // TODO(phase-7-task-2b): TriggerOrder dispatch for multi-replacement
    // stacks. For now we document the behavior as "collection order" and
    // let Task 7 behavioral test cover the single-side case.
    let mut result = own;
    result.extend(opp);
    result
}
```

(Multi-replacement TriggerOrder selection is a known limitation; call it out in the code comment and add a dedicated test for the single-side case. Multi-side cards exist but are rare — defer full treatment to a follow-up issue rather than inflating this task.)

**PendingSelection::Replacement emission** (in `replacement.rs::run_candidate`):
```rust
fn run_candidate(
    game: &mut crate::game::Game,
    cand: Candidate,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
    prior_outcome: ReplacementOutcome,
) -> ReplacementOutcome {
    if cand.is_mandatory {
        // Passive modifier — run process unconditionally.
        run_process(game, cand, subject, cause, original_destination)
    } else {
        // Optional — install PendingSelection::Replacement with accept+PASS.
        // ACCEPT_ACTION_ID reuses EffectChoice range slot 0.
        let accept_action = crate::action::space::HAND_EFFECT_START; // pick a dedicated slot
        let callback = make_accept_callback(cand, subject, cause, original_destination);
        let on_decline = make_decline_callback(prior_outcome);
        game.install_pending_selection(PendingSelection {
            kind: SelectionKind::Replacement,
            selecting_player: subject_controller_id(game, &subject),
            previous_phase: game.current_phase,
            valid_action_ids: vec![accept_action],
            is_optional: true,
            prompt: format!("May accept replacement: {}", cand.effect_name),
            effect_choices: None,
            source_card: cand.source_card,
            source_permanent: cand.source_permanent,
            callback,
            on_decline: Some(on_decline),
        });
        // Placeholder — dispatcher returns prior outcome; real outcome
        // commits inside the callback. Caller must spin on pending_selection.
        prior_outcome
    }
}
```

*Note on the synchronous-v1 constraint:* optional replacements in v1 run their callback synchronously when the selection resolves (the callback invokes the replacement process inline and applies the outcome). Nested selections inside a replacement process are supported by the existing `PendingSelection` machinery and don't require special handling here.

**Action-slot choice for Replacement accept:** reuse a stable slot from the `EffectChoice` range — define a constant:
```rust
// digimon-engine/src/action/space.rs (addition)
/// Action ID used by SelectionKind::Replacement to encode "accept this
/// replacement effect". Decline is the standard PASS (62).
pub const REPLACEMENT_ACCEPT: u16 = HAND_EFFECT_END - 1; // slot 59, last free HAND_EFFECT slot
```

Alternative: a dedicated slot like `60` (currently `HATCH`) is already taken — use slot 59 (unused in `HAND_EFFECT` range, since actual effects max out at ~50). Verify against current mask builder: search `HAND_EFFECT_START..HAND_EFFECT_END` usage.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/replacements/dispatcher_core.rs`:
```rust
use digimon_engine::enums::EffectTiming;
use digimon_engine::replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject};
// Import a test card that installs a WhenWouldBeDeleted effect cancelling.

#[test]
fn try_replace_returns_none_when_no_candidates() {
    // Build a game with no replacement effects installed.
    // Call game.try_replace(WhenWouldBeDeleted, Permanent(h), OwnEffect, None)
    // Expected: ReplacementOutcome::None.
}

#[test]
fn mandatory_cancel_replacement_applies() {
    // Use a test card with a WhenWouldBeDeleted replacement_process that
    // calls ctx.cancel() unconditionally. Place it on the field.
    // Call try_replace targeting that permanent.
    // Expected: ReplacementOutcome::Cancelled.
}

#[test]
fn redirect_replacement_applies() {
    // Test card does ctx.redirect_to(Zone::Deck).
    // Expected: ReplacementOutcome::Redirected(Zone::Deck).
}

#[test]
fn substitute_replacement_applies() {
    // Test card does ctx.substitute(Permanent(other_handle)).
    // Expected: ReplacementOutcome::Substituted(Permanent(other_handle)).
}

#[test]
fn custom_handled_replacement_applies() {
    // Test card does ctx.effect.trash_from_top(owner, 1) then ctx.handled().
    // Expected: ReplacementOutcome::CustomHandled AND owner's top card was trashed.
}

#[test]
fn optional_replacement_emits_pending_selection() {
    // Test card is marked .optional() and has a replacement_process.
    // Call try_replace; expected: pending_selection is installed with
    // SelectionKind::Replacement and both REPLACEMENT_ACCEPT + PASS in the
    // mask. try_replace returns ReplacementOutcome::None (pending).
}

#[test]
fn optional_replacement_accept_path_applies_outcome() {
    // Install optional replacement. Call try_replace. Resolve via
    // REPLACEMENT_ACCEPT. Expected: the replacement's outcome committed.
}

#[test]
fn optional_replacement_decline_path_leaves_outcome_none() {
    // Install optional replacement. Call try_replace. Resolve via PASS.
    // Expected: ReplacementOutcome::None.
}

#[test]
fn depth_guard_caps_at_8() {
    // Constructed loop: replacement A redirects to B, B redirects back to A.
    // After 8 recursive try_replace entries, returns None and commits original.
    // Assert game.replacement_depth returns to 0 after call.
}

#[test]
fn cause_filter_gates_passive_modifier() {
    // Install a ModifierEntry with cause_filter = Some(OpponentEffect).
    // Call try_replace with cause = OwnEffect.
    // Expected: modifier does NOT apply (ReplacementOutcome::None).
    // Call try_replace with cause = OpponentEffect.
    // Expected: modifier applies (Cancelled).
}
```

The test file will need a few small test-card structs in `digimon-engine/src/cards/test/` (e.g. `test_phase7_cancel.rs`, `test_phase7_redirect.rs`, …) registered in `cards/test/mod.rs`. Keep each minimal — one effect each.

- [ ] **Step 2: Run — FAIL (dispatcher unimplemented)**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements dispatcher_core`

- [ ] **Step 3: Implement dispatcher**

1. In `digimon-engine/src/game.rs`:
   - Add `pub(crate) replacement_depth: u8,` to `Game` struct.
   - Initialize to `0` in `Game::new` (or equivalent constructor) — search for `replacement_depth` to confirm it's the only touched field.
   - Add the `try_replace` thin wrapper.
2. In `digimon-engine/src/replacement.rs`:
   - Implement `try_replace_impl`, `collect_candidates`, `layer_candidates`, `run_candidate`, `run_process`, `subject_controller_id`, `make_accept_callback`, `make_decline_callback`.
   - `collect_candidates` walks:
     - The subject's own effects at the given timing (for Permanent subjects — call `game.effects_for_card_at_timing(handle, timing)` or scan `card_registry`).
     - All battle-area permanents' inherited + non-inherited effects at the timing (for cross-permanent replacements like Barrier granted via aura).
     - The target player's `player_modifiers` entries whose `modifier` is a passive-replacement variant (CannotBeReturnedToDeck, …). Map ModifierType → EffectTiming → ReplacementOutcome via a lookup table in `replacement.rs`.
     - The target permanent's `permanent_modifiers` entries likewise.
   - Each candidate carries `source_card`, `source_permanent`, `source_controller`, `is_mandatory`, `cause_filter`, `replacement_condition`, and either a `replacement_process: &ReplacementProcessFn` or a synthetic "cancel" process for passive modifiers.
   - Apply `cause_filter` filter during collection: skip any candidate whose `cause_filter` is `Some(c)` where `c != cause`.
   - Apply `replacement_condition` filter during collection if present.
3. In `digimon-engine/src/action/space.rs`: add `pub const REPLACEMENT_ACCEPT: u16 = 59;` (verify slot is free; if not, pick next free slot in HAND_EFFECT range).
4. In `digimon-engine/src/action/mask.rs`: add `SelectionKind::Replacement` handler that emits `REPLACEMENT_ACCEPT` + `PASS` into the mask.
5. In `digimon-engine/src/game.rs` (selection resolver): add `SelectionKind::Replacement` case to the selection-resolution dispatcher, running the callback on accept and `on_decline` on PASS.

- [ ] **Step 4: Run — dispatcher_core tests pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements`
Expected: 5 prior + ~10 new = 15 passing.

- [ ] **Step 5: Full suite green**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: 571 + 10 = 581 passing. Zero warnings.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/game.rs digimon-engine/src/replacement.rs digimon-engine/src/action/space.rs digimon-engine/src/action/mask.rs digimon-engine/src/selection.rs digimon-engine/src/cards/test digimon-engine/tests/replacements/dispatcher_core.rs
git commit -m "rust-engine(phase-7): try_replace dispatcher + layering + recursion guard"
```

---

### Task 3: Wire `WhenWouldBeDeleted` + `WhenWouldLeaveBattleArea` at `delete_permanent_with_effects`

**Files:**
- Modify: `digimon-engine/src/combat.rs` — `delete_permanent_with_effects` (line 1223) opens a replacement window before enqueuing `OnDeletion`.
- Modify: `digimon-engine/src/combat.rs::resolve_battle` (line 1172) — ensure cause=`Battle` threads through.
- Create: `digimon-engine/tests/replacements/deletion_replacements.rs` — Barrier / Evade / Partition / ArmorPurge / Fragment scenario tests.
- Modify: `digimon-engine/tests/replacements/main.rs` — add `mod deletion_replacements;`.

**Pseudocode for the wiring at `combat.rs:1223`:**
```rust
pub fn delete_permanent_with_effects(&mut self, handle: PermanentHandle) {
    self.delete_permanent_with_cause(handle, self.infer_deletion_cause(handle));
}

pub fn delete_permanent_with_cause(
    &mut self,
    handle: PermanentHandle,
    cause: ReplacementCause,
) {
    // Phase 7: replacement window. Fire leave-field super-timing first,
    // then the route-specific Would.
    let subject = ReplacementSubject::Permanent(handle);
    let outcome = self.try_replace(
        EffectTiming::WhenWouldLeaveBattleArea,
        subject,
        cause,
        Some(Zone::Trash),
    );
    let outcome = match outcome {
        ReplacementOutcome::None => {
            self.try_replace(
                EffectTiming::WhenWouldBeDeleted,
                subject,
                cause,
                Some(Zone::Trash),
            )
        }
        other => other,
    };
    match outcome {
        ReplacementOutcome::None => {
            // Proceed with existing path.
            self.enqueue_triggered(
                EffectTiming::OnDeletion,
                TriggerSource::Permanent(handle),
            );
            self.drain_effect_queue();
            if self.handle_valid(handle) {
                self.player_mut(handle.player).delete_permanent(handle.index as usize);
            }
            self.modifiers.clear_permanent(handle);
            self.modifiers.expire_player_on_permanent_leave(handle);
            // OnAnyDeletion unchanged — fires post-commit.
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnAnyDeletion,
                    TriggerSource::PlayerBattleArea(pid as PlayerId),
                );
            }
            self.drain_effect_queue();
        }
        ReplacementOutcome::Cancelled => {
            // Skip deletion; no observers fire.
        }
        ReplacementOutcome::Redirected(Zone::Deck) => {
            self.return_to_deck(handle, StackPosition::Bottom);
            // return_to_deck internally fires OnLeaveField + OnReturn.
        }
        ReplacementOutcome::Redirected(Zone::Hand) => {
            self.return_to_hand(handle);
        }
        ReplacementOutcome::Substituted(ReplacementSubject::Permanent(source_h)) => {
            // Partition / ArmorPurge — operate on the substituted subject
            // instead. Treat as its own deletion event (recursion-safe via
            // depth guard).
            self.delete_permanent_with_cause(source_h, cause);
        }
        ReplacementOutcome::CustomHandled => {
            // Barrier trashed a deck card etc. in-process. Skip.
        }
        _ => {
            // Unexpected redirect destination — log and fall through to delete.
        }
    }
}

fn infer_deletion_cause(&self, _handle: PermanentHandle) -> ReplacementCause {
    if self.security_resolution.is_some() {
        ReplacementCause::SecurityCheck
    } else if self.pending_attack.is_some() {
        ReplacementCause::Battle
    } else if self.effect_source_player.is_some() {
        let acting = self.effect_source_player.unwrap();
        let target_controller = _handle.player;
        if acting == target_controller {
            ReplacementCause::OwnEffect
        } else {
            ReplacementCause::OpponentEffect
        }
    } else {
        ReplacementCause::OwnEffect
    }
}
```

Note: `effect_source_player` may not be a `Game` field today — check `game.rs`. If it's tracked on `EffectContext` only, add a `pub(crate) effect_source_player: Option<PlayerId>` field on `Game`, set by `enqueue_triggered` / `run_queued_effect` during drain and cleared at drain-end.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/replacements/deletion_replacements.rs`. Each test uses a hand-authored test card with the specific replacement:

```rust
// Test card phase7_barrier: WhenWouldBeDeleted process trashes top of own deck
// then calls ctx.handled().
#[test]
fn barrier_trashes_top_of_deck_and_skips_deletion() {
    // Setup: permanent with Barrier-style replacement on field; owner has
    // 5 cards in deck, 0 in trash.
    // Trigger: DebugRunner forces delete_permanent_with_effects.
    // Assert:
    //   - permanent still on field
    //   - owner deck has 4 cards
    //   - owner trash has 1 card
    //   - OnDeletion observer did NOT fire (check via side-effect card)
}

// Test card phase7_evade: WhenWouldBeDeleted redirect_to(Zone::Deck).
#[test]
fn evade_redirects_to_bottom_of_deck() {
    // Setup: permanent with Evade-style replacement.
    // Trigger: delete.
    // Assert:
    //   - permanent removed from battle area
    //   - top card of owner's deck is the Evaded card (inserted at bottom
    //     which for a deck of size N means index 0 — verify deck semantics)
    //   - OnDeletion did NOT fire
    //   - OnLeaveField DID fire (via side-effect witness card)
    //   - OnReturn DID fire
}

#[test]
fn partition_deletes_a_source_instead() {
    // Setup: permanent with Partition-style replacement. Target permanent
    // has 2 sources stacked underneath.
    // Trigger: delete.
    // Assert:
    //   - target permanent still on field (top card unchanged)
    //   - target permanent's card_sources len decreased by 1
    //   - OnDeletion fires for the substituted source (if it has OnDeletion)
}

#[test]
fn armor_purge_trashes_a_source_instead() {
    // Similar to Partition but trashes the source rather than deleting it.
}

#[test]
fn fragment_3_trashes_three_top_cards_and_skips_deletion() {
    // Setup: permanent with Fragment(3)-style replacement.
    // Trigger: delete.
    // Assert:
    //   - permanent still on field
    //   - owner's deck lost 3 cards
    //   - owner's trash gained 3
    //   - ReplacementOutcome::CustomHandled
}

#[test]
fn deletion_cause_battle_is_inferred_during_resolve_battle() {
    // Build two opposing Digimon; attack such that defender loses.
    // Install a WhenWouldBeDeleted replacement on defender that only
    // applies when cause == Battle (cause_filter = Some(Battle)).
    // Assert: replacement fires (defender saved or redirected).
}

#[test]
fn deletion_cause_security_check_is_inferred() {
    // Set up security battle scenario where a Digimon reveals from
    // security and loses the mini-battle. Install replacement on that
    // Digimon with cause_filter = Some(SecurityCheck).
    // Assert: replacement fires.
}

#[test]
fn deletion_cause_opponent_effect_is_inferred() {
    // Player 0 plays an effect that deletes Player 1's permanent.
    // Replacement on Player 1's permanent with cause_filter = Some(OpponentEffect).
    // Assert: fires.
}

#[test]
fn deletion_cancelled_suppresses_on_any_deletion() {
    // Witness card with OnAnyDeletion on field.
    // Replacement cancels the deletion.
    // Assert: witness did NOT observe.
}

#[test]
fn deletion_redirected_to_deck_fires_on_return_not_on_deletion() {
    // Witness A with OnDeletion, witness B with OnReturn.
    // Replacement redirects to Zone::Deck.
    // Assert: A did NOT fire, B DID fire.
}
```

Add `mod deletion_replacements;` to `digimon-engine/tests/replacements/main.rs`.

- [ ] **Step 2: Run — FAIL**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test replacements deletion_replacements`

- [ ] **Step 3: Implement wiring**

1. In `digimon-engine/src/combat.rs`:
   - Split `delete_permanent_with_effects` into `delete_permanent_with_effects` (infers cause) + `delete_permanent_with_cause` (takes explicit cause). All existing callers go through the inferring form — no signature change.
   - Replace the body of `delete_permanent_with_cause` per the pseudocode above.
2. In `resolve_battle` (line 1172): replace both `self.delete_permanent_with_effects(X)` calls with `self.delete_permanent_with_cause(X, ReplacementCause::Battle)`.
3. Thread `effect_source_player` if not already present on `Game` — add field, clear on drain end, set during enqueue. Verify via grep: `grep -rn "effect_source_player" digimon-engine/src`.
4. Add test cards in `src/cards/test/` for each tested behavior:
   - `test_phase7_barrier.rs`
   - `test_phase7_evade.rs`
   - `test_phase7_partition.rs`
   - `test_phase7_armor_purge.rs`
   - `test_phase7_fragment.rs`
   - `test_phase7_battle_cause_filter.rs`
   - `test_phase7_security_cause_filter.rs`
   - `test_phase7_opponent_cause_filter.rs`
   - `test_phase7_on_any_deletion_witness.rs`
   - `test_phase7_on_return_witness.rs`
  Register in `cards/test/mod.rs`.

- [ ] **Step 4: Run — deletion_replacements tests pass**

- [ ] **Step 5: Full suite green**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: 581 + 10 = 591 passing. Zero warnings.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/combat.rs digimon-engine/src/cards digimon-engine/tests/replacements
git commit -m "rust-engine(phase-7): wire WhenWouldBeDeleted + WhenWouldLeaveBattleArea at deletion site"
```

---

### Task 4: Wire the remaining Would timings — return / trash / de-digi / lose-security / draw / place-in-security

**Files:**
- Modify: `digimon-engine/src/game_actions.rs`:
  - `return_to_hand` (line 610): open `WhenWouldLeaveBattleArea` → `WhenWouldBeReturnedToHand` window before mutating.
  - `return_to_deck` (line 658): `WhenWouldLeaveBattleArea` → `WhenWouldBeReturnedToDeck`.
  - `place_on_security` (line 1229): `WhenWouldPlaceInSecurity` at entry.
- Modify: `digimon-engine/src/combat.rs::drive_security_resolution` (line ~989): `WhenWouldLoseSecurity` fires at `SecurityPhase::BattleResolved` → `Dispose` transition (before the revealed card is trashed).
- Modify: `digimon-engine/src/effect_context/mod.rs::draw` (line 286): `WhenWouldDraw` at entry (before the `CannotDrawByEffect` check or just after — spec §14 Q3 says flood-gate takes precedence, so: check flood-gate first, then `try_replace(WhenWouldDraw, Player, cause, None)`).
- Modify: `digimon-engine/src/effect_context/mod.rs` effect-driven trash helpers (`trash_from_hand`, `trash_from_security`, effect-driven battle-area trash): add `WhenWouldBeTrashed` firing.
- Modify: `digimon-engine/src/effect_context/mod.rs::de_digivolve` (landed in Phase 10, commit `68196036`, lines ~333–398): open `WhenWouldBeDeDigivolved` window at the top of the `while popped < max` loop. Signature today is `de_digivolve(target, stop_at_level: Option<u8>, amount: Option<u8>) -> u8`. The replacement fires **once per call**, with `subject = ReplacementSubject::Permanent(target)`, `cause = infer_effect_cause(target.player)`, `original_destination = None` (de-digi doesn't move to a zone). Outcome handling:
  - `Cancelled` → return 0 without popping.
  - `CustomHandled` → return 0 (process did whatever it wanted).
  - `Substituted(Permanent(other))` → run the normal loop against `other` instead of `target`.
  - `Redirected(_)` → not meaningful for de-digi; log and fall through to default.
  - `None` → continue with the existing loop body unchanged.
  One opportunity the spec calls out in §14 Q-follow-up: some cards reduce `N` rather than fully cancel. For v1 we do not surface a mutable `amount` on `ReplacementContext` — scripts that reduce use `ctx.substitute(same_handle)` trick is wrong; the right path is to `ctx.cancel()` and have the process manually call `ctx.effect.de_digivolve(target, stop_at_level, Some(reduced_amount))`. Document this in the task commit message.
- Create: `digimon-engine/tests/replacements/route_replacements.rs`.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/replacements/route_replacements.rs`:
```rust
#[test]
fn would_be_returned_to_hand_cancel_keeps_permanent_on_field() { ... }

#[test]
fn would_be_returned_to_deck_cancel_keeps_permanent_on_field() { ... }

#[test]
fn would_be_returned_to_hand_redirect_to_trash_deletes_instead() {
    // Opponent effect tries to return to hand; replacement redirects to trash.
    // Assert permanent gone + OnDeletion fired.
}

#[test]
fn would_lose_security_cancel_keeps_card_on_stack() {
    // During a security check, cancel the loss. Card remains on security;
    // attacker's attack-security-remove counter unchanged.
}

#[test]
fn would_draw_cancel_prevents_card_movement() {
    // ctx.draw(player, 1) with a WhenWouldDraw cancel — no card moves
    // from deck to hand.
}

#[test]
fn would_draw_still_respects_flood_gate_cannot_draw_by_effect() {
    // Install CannotDrawByEffect AND a WhenWouldDraw cancel. Flood gate
    // fires first; replacement should NOT be invoked (spec §14 Q3).
    // Assert: replacement process was NOT called (use an Arc<Mutex<bool>>
    // sentinel on the test card).
}

#[test]
fn would_place_in_security_reorder_to_bottom() {
    // Effect places card face-up on top of security; replacement
    // redirects position to bottom via a new Zone+position overload, or
    // cancels + places manually. Choose design via test outcome.
    //
    // Simpler semantics for v1: redirect_to(Zone::Trash) → card is trashed
    // instead. More nuanced top/bottom adjustments deferred.
}

#[test]
fn would_be_trashed_from_battle_area_cancel() {
    // Effect calls ctx.trash_permanent (or equivalent) on a permanent
    // with WhenWouldBeTrashed cancel. Permanent survives.
}

#[test]
fn would_be_de_digivolved_cancel_prevents_pops() {
    // Target permanent with 3-card stack and a WhenWouldBeDeDigivolved
    // cancel replacement. Opponent effect calls
    //   ctx.de_digivolve(target, Some(3), Some(2))
    // Assert: return value = 0; stack unchanged.
}

#[test]
fn would_be_de_digivolved_substitute_targets_other_permanent() {
    // Permanent A has the replacement, redirects the de-digi to B.
    // Call de_digivolve(A, ...). Assert A unchanged; B pops.
}

#[test]
fn would_be_de_digivolved_custom_handled_returns_zero() {
    // Replacement runs ctx.handled() after doing bookkeeping.
    // Assert de_digivolve returns 0 and stack unchanged (process did
    // nothing; just consumed the would-event).
}
```

Add `mod route_replacements;` to `digimon-engine/tests/replacements/main.rs`.

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement wiring**

In each helper (`return_to_hand`, `return_to_deck`, `place_on_security`, `drive_security_resolution`, `EffectContext::draw`, trash helpers), insert the `try_replace` call at the function entry, BEFORE any state mutation:

Template:
```rust
pub fn return_to_hand(&mut self, handle: PermanentHandle) -> Option<CardHandle> {
    let cause = self.infer_effect_cause(handle.player);
    let subject = ReplacementSubject::Permanent(handle);
    match self.try_replace(EffectTiming::WhenWouldLeaveBattleArea, subject, cause, Some(Zone::Hand)) {
        ReplacementOutcome::Cancelled => return None,
        ReplacementOutcome::CustomHandled => return None,
        ReplacementOutcome::Redirected(z) => {
            // Route to alternative helper
            return self.route_by_zone(handle, z, cause);
        }
        ReplacementOutcome::Substituted(sub) => {
            return self.apply_substitute_return_to_hand(sub);
        }
        ReplacementOutcome::None => { /* fall through to per-route dispatch */ }
    }
    match self.try_replace(EffectTiming::WhenWouldBeReturnedToHand, subject, cause, Some(Zone::Hand)) {
        // same dispatch
    }
    // ...original return_to_hand body unchanged...
}
```

Add a helper `Game::infer_effect_cause(target_player: PlayerId) -> ReplacementCause` that applies the cause-inference rules (acting player vs target).

`drive_security_resolution` fire site: between `SecurityPhase::BattleResolved` and `SecurityPhase::OnSecurityCheckDrain`. The subject is `ReplacementSubject::Card(revealed_card, Zone::Security)`; `original_destination` is `Some(Zone::Trash)`; cause is `SecurityCheck`.

- [ ] **Step 4: Run — route_replacements tests pass**

- [ ] **Step 5: Full suite green**

Expected: 591 + 11 = 602 passing. Zero warnings. (+3 new WhenWouldBeDeDigivolved tests on top of the 8 planned route replacements, now that Phase 10's `ctx.de_digivolve` is available.)

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/game_actions.rs digimon-engine/src/combat.rs digimon-engine/src/effect_context digimon-engine/tests/replacements/route_replacements.rs digimon-engine/tests/replacements/main.rs
git commit -m "rust-engine(phase-7): wire Would* timings for return/trash/lose-security/draw/place"
```

---

### Task 5: Phase 6 passive-modifier migration — auto-install replacements

**Files:**
- Modify: `digimon-engine/src/replacement.rs` — add passive-modifier → replacement lookup:
  ```rust
  fn passive_modifier_to_would(modifier: ModifierType) -> Option<EffectTiming> {
      match modifier {
          ModifierType::CannotBeReturnedToDeck => Some(EffectTiming::WhenWouldBeReturnedToDeck),
          ModifierType::CannotBeReturnedToHand => Some(EffectTiming::WhenWouldBeReturnedToHand),
          ModifierType::CannotBeTrashedByEffect => Some(EffectTiming::WhenWouldBeTrashed),
          ModifierType::CannotBeDeDigivolved => Some(EffectTiming::WhenWouldBeDeDigivolved),
          ModifierType::CannotBeDestroyed => Some(EffectTiming::WhenWouldBeDeleted),
          ModifierType::CannotBeDestroyedByBattle => Some(EffectTiming::WhenWouldBeDeleted),
          ModifierType::CannotBeDestroyedByEffect => Some(EffectTiming::WhenWouldBeDeleted),
          _ => None,
      }
  }
  ```
  Integrate into `collect_candidates`: for each modifier entry, if the mapper returns `Some(timing)` and `timing == current_timing`, emit a synthetic mandatory candidate whose process is `ctx.cancel()`.
- Modify: `digimon-engine/src/enums.rs` — add missing passive variants if not already declared. Audit: `CannotBeReturnedToDeck`, `CannotBeReturnedToHand`, `CannotBeTrashedByEffect`, `CannotBeDeDigivolved` — check current enum. `CannotBeDestroyed*` exists since Phase 0.
- Modify: `digimon-engine/src/modifiers.rs` — ensure `cause_filter` is honored during modifier-based candidate collection; defaults to `Some(OpponentEffect)` when the printed text commonly reads "by your opponent's effects". Expose a builder helper `PlayerModifierEntry::opponent_only(…)` / `ModifierEntry::opponent_only(…)` for the common case.
- Modify: `digimon-engine/src/enums.rs::ModifierType` — keep the original `CannotBeDestroyedByBattle` path but fold its enforcement through the new replacement layer (previously handled ad hoc in combat.rs — check and remove duplicate gate if present to avoid double-firing).
- Create: `digimon-engine/tests/replacements/passive_modifier_migration.rs`.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/replacements/passive_modifier_migration.rs`:
```rust
#[test]
fn cannot_be_returned_to_deck_cancels_return() {
    // Install CannotBeReturnedToDeck on permanent A (opponent_only default).
    // Player 1 plays an effect to return A to P0's deck.
    // Assert: A still on field, opponent effect no-op.
}

#[test]
fn cannot_be_returned_to_deck_allows_own_return() {
    // Same modifier (cause_filter = OpponentEffect).
    // P0's own effect returns A to deck.
    // Assert: A returned successfully.
}

#[test]
fn cannot_be_trashed_by_effect_cancels_opponent_trash() { ... }

#[test]
fn cannot_be_de_digivolved_cancels_de_digi() { ... }

#[test]
fn cannot_be_destroyed_by_battle_cancels_battle_deletion() {
    // Retains the existing Phase 0 behavior under the new framework.
}

#[test]
fn cannot_be_destroyed_by_battle_does_not_cancel_effect_deletion() {
    // cause_filter must correctly discriminate Battle vs OwnEffect.
}

#[test]
fn player_scoped_passive_also_auto_installs_as_replacement() {
    // Install a PlayerModifierEntry with modifier = CannotBeReturnedToHand
    // targeting Player 1. Effect from Player 0 tries to bounce all of P1's
    // Digimon. Assert: none moved.
}

#[test]
fn replacement_condition_gates_passive() {
    // Install CannotBeReturnedToDeck with a replacement_condition that
    // checks battle_area size == 1. With 2 digimon on field, condition
    // returns false — replacement does NOT apply. With 1, does.
}
```

Add `mod passive_modifier_migration;` to `main.rs`.

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement**

1. In `replacement.rs::collect_candidates`:
   - After collecting card-face candidates, scan `game.modifiers.permanent_modifiers` for entries on the subject (for Permanent subjects), map via `passive_modifier_to_would`, filter by `cause_filter`, apply `replacement_condition` if present, emit synthetic candidates.
   - Similarly scan `game.modifiers.player_modifiers` for the target player.
2. Remove any ad-hoc Phase 0 deletion gates in `combat.rs` that duplicated `CannotBeDestroyedByBattle`/`CannotBeDestroyedByEffect` — search `grep -n "CannotBeDestroyed" digimon-engine/src/combat.rs` and confirm they're handled exclusively via the new replacement framework.
3. Ensure defaults match printed text: when scripts call `add_modifier` for a passive-replacement variant, if no `cause_filter` is specified, set the default per-variant:
   - `CannotBeReturnedToDeck` → `Some(OpponentEffect)` (default)
   - `CannotBeReturnedToHand` → `Some(OpponentEffect)`
   - `CannotBeTrashedByEffect` → `Some(OpponentEffect)`
   - `CannotBeDeDigivolved` → `Some(OpponentEffect)`
   - `CannotBeDestroyed` → `None` (cause-agnostic)
   - `CannotBeDestroyedByBattle` → `Some(Battle)`
   - `CannotBeDestroyedByEffect` → any-effect (Own or Opponent) — use `None` and document that per-variant.

   Encode defaults in `ModifierEntry::simple_passive_replacement(modifier, expiry, source_player) -> Self`.

- [ ] **Step 4: Run — passive_modifier_migration tests pass**

- [ ] **Step 5: Full suite green**

Expected: 602 + 8 = 610 passing. Zero warnings. Existing Phase 0 `CannotBeDestroyedByBattle` tests must still pass with the new implementation.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/replacement.rs digimon-engine/src/modifiers.rs digimon-engine/src/enums.rs digimon-engine/src/combat.rs digimon-engine/tests/replacements
git commit -m "rust-engine(phase-7): migrate passive restriction modifiers to auto-install replacements"
```

---

### Task 6: Native keyword parsing — Barrier / Evade / Fragment(N) / Decode / Partition / ArmorPurge

**Files:**
- Modify: `digimon-engine/src/card_data.rs` — extend the printed-keyword parser (added in Phase 3) with the 4 net-new keywords and the parametric `Fragment(N)`. Parse rules, matching DCGO card text:
  - `"<Barrier>"` → `Keyword::Barrier` (already parses — verify)
  - `"<Evade>"` → `Keyword::Evade`
  - `"<Fragment [N]>"` or `"<Fragment (N)>"` — confirm printed format — → `Keyword::Fragment(N)`
  - `"<Decode>"` → `Keyword::Decode`
  - `"<Partition>"` → `Keyword::Partition` (verify)
  - `"<Armor Purge>"` → `Keyword::ArmorPurge`
- Modify: `digimon-engine/src/card_registry.rs` — at registry-build time, emit auto-install `Effect`s for permanents whose `CardData::keywords` includes the new Phase 7 keywords. Each keyword maps to a `WhenWouldBeDeleted` effect with a specific `replacement_process`. Where Barrier / Evade / Fragment are "may" effects in printed rules, mark `.optional()` on the effect. Partition and ArmorPurge are also optional ("you may"). Decode on return is also optional.
- Create: `digimon-engine/tests/replacements/native_keywords.rs`.

**Mapping table** (implement as a function `keyword_to_auto_effect(keyword: Keyword, card: CardHandle) -> Option<Effect>`):

| Keyword | Timing | Process | Optional? |
|---------|--------|---------|-----------|
| `Barrier` | `WhenWouldBeDeleted` | `ctx.effect.trash_from_top(owner, 1); ctx.handled();` | Yes |
| `Evade` | `WhenWouldBeDeleted` | `ctx.redirect_to(Zone::Deck);` (bottom) | Yes |
| `Fragment(n)` | `WhenWouldBeDeleted` | `ctx.effect.trash_from_top(owner, n); ctx.handled();` | Yes |
| `Decode` | `WhenWouldBeReturnedToDeck` OR `WhenWouldBeReturnedToHand` | `ctx.redirect_to(Zone::Hand);` | Yes |
| `Partition` | `WhenWouldBeDeleted` | installs a source-select selection; substitutes chosen source | Yes |
| `ArmorPurge` | `WhenWouldBeDeleted` | similar to Partition but trashes rather than deletes | Yes |

Partition and ArmorPurge need to install a nested `PendingSelection::Source`, wait for the player to pick a source, then apply the substitute. This reuses the existing source-selection machinery; test coverage verifies the nested selection path works under the replacement window.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/replacements/native_keywords.rs`:
```rust
#[test]
fn printed_barrier_keyword_auto_installs_replacement() {
    // Build a card with effect_text containing "<Barrier>".
    // Parse to CardData::keywords — expect [Keyword::Barrier].
    // Place on field; trigger delete.
    // Assert: deck trashed 1, permanent survives.
}

#[test]
fn printed_evade_keyword_redirects_to_deck_bottom() { ... }

#[test]
fn printed_fragment_3_keyword_trashes_three() { ... }

#[test]
fn printed_decode_keyword_redirects_return_to_hand_from_deck_return() {
    // Opponent effect tries to return our Decode-keyworded Digimon to deck.
    // Card ends up in our hand instead.
}

#[test]
fn printed_partition_keyword_substitutes_a_chosen_source() {
    // Digimon with 2 sources and printed Partition.
    // Opponent effect deletes it.
    // Player gets PendingSelection::Source to pick which source.
    // Resolve with source 0.
    // Assert: target permanent top card unchanged; source 0 trashed; OnDeletion
    //  fires for that source's card.
}

#[test]
fn printed_armor_purge_keyword_trashes_source() { ... }

#[test]
fn printed_keywords_respect_optional_decline() {
    // Barrier is optional. Decline via PASS → permanent dies normally.
}
```

Add `mod native_keywords;` to `main.rs`.

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement**

1. Extend the keyword parser in `card_data.rs`.
2. Add `keyword_to_auto_effect` in `card_registry.rs` (or a new module `digimon-engine/src/cards/keyword_effects.rs`).
3. Wire it at registry-build — for each card, iterate `card_data.keywords`, call the mapper, append returned effects into the card's effect list.
4. Partition / ArmorPurge processes install nested `PendingSelection::Source` selections via the existing selection helpers. The process returns, the selection callback calls `ctx.substitute(...)` and fully exits (since `substitute` sets `outcome` and the engine commits on the next poll).

- [ ] **Step 4: Run — native_keywords tests pass**

- [ ] **Step 5: Full suite green**

Expected: 610 + 7 = 617 passing. Zero warnings.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/card_data.rs digimon-engine/src/card_registry.rs digimon-engine/src/cards digimon-engine/tests/replacements/native_keywords.rs digimon-engine/tests/replacements/main.rs
git commit -m "rust-engine(phase-7): parse printed Barrier/Evade/Fragment/Decode/Partition/ArmorPurge keywords"
```

---

### Task 7: Behavioral end-to-end + docs + roadmap

**Files:**
- Create: `digimon-engine/tests/replacements/behavioral_end_to_end.rs` — one realistic archetype scenario.
- Modify: `docs/RUST_ENGINE_API.md` — new §Phase 7 section.
- Modify: `docs/RUST_PYTHON_PARITY.md` — §7 entry closing the replacement gap.
- Modify: `.claude/plans/recursive-coalescing-candle.md` — flip Phase 7 row to ✅ Landed.
- Modify: `docs/superpowers/plans/2026-04-21-rust-engine-phase-7-would-replacements.md` — mark tasks done in a "Status" section at bottom.

**End-to-end scenario:** TS Olympos Cherubimon with printed Barrier, under attack from opposing Dark Masters MetalSeadramon. Second Barrier granted via a Tamer aura on the same turn. Two simultaneous replacements — one controller-owned — exercise layering.

- [ ] **Step 1: Write failing behavioral test**

Create `digimon-engine/tests/replacements/behavioral_end_to_end.rs`:
```rust
#[test]
fn ts_olympos_cherubimon_layered_barriers_end_to_end() {
    // Player 0: Cherubimon (printed Barrier) on field, plus a Tamer whose
    //   aura effect also grants Barrier (as a modifier) to all own Digimon.
    //   Deck has 10 cards, trash empty.
    // Player 1: attacking Digimon with effective DP > Cherubimon.
    //
    // Player 1 attacks Cherubimon.
    // Expected sequence:
    //   1. Battle resolves, DP compare, Cherubimon "would be deleted" (cause=Battle).
    //   2. Both Barriers queued. Player 0 gets TriggerOrder prompt (2 own-side
    //      replacements). Player 0 picks printed Barrier first.
    //   3. Printed Barrier: accept → trash top deck, permanent survives,
    //      ReplacementOutcome::CustomHandled set.
    //   4. Now the aura Barrier sees outcome=CustomHandled — outcome settled,
    //      second replacement does not fire a second trash.
    //   (Alternative test: decline first Barrier, accept second — verify
    //   second Barrier then fires.)
    //   5. OnDeletion does not fire.
    //   6. Attack resolves — attacker's security-check path follows.
    //
    // Assert the final game state: Cherubimon on field, P0 deck size = 9,
    // P0 trash size = 1, attacker advanced to next attack phase.
}

#[test]
fn medusamon_partition_chain_with_opponent_cannot_be_returned_to_deck() {
    // Medusamon Partition player: Digimon with printed Partition + 2 sources.
    // Opponent effect first tries to return it to deck.
    //   - Partition is a WhenWouldBeDeleted replacement, not return — doesn't fire.
    //   - "Cannot be returned to deck" passive on Medusamon Digimon cancels.
    // Then opponent effect tries to delete.
    //   - Partition fires, source selection, source trashed, Digimon survives.
    // Assert: Digimon on field, one source gone, no deck return happened.
}
```

- [ ] **Step 2: Run — FAIL (probably passes trivially if prior tasks covered all primitives — scenario check)**

- [ ] **Step 3: Implement fixtures**

Add any missing test-card fixtures. Most functionality is already covered by Task 1-6 primitives; Task 7 just composes them.

- [ ] **Step 4: Run — behavioral_end_to_end tests pass**

- [ ] **Step 5: Write docs**

In `docs/RUST_ENGINE_API.md` add a §Phase 7 section:
- Intro: "Replacement effects intercept impending state changes before they commit."
- Subsections per Would* timing with example script snippet.
- `ReplacementContext` API reference — fields + mutating helpers.
- Worked example: Barrier script (3-4 lines using the native-keyword path + 3-4 lines as a hand-authored Effect).
- Layering + optional/mandatory rules — link to spec.
- `ACTION_SPACE_SIZE` unchanged note.

In `docs/RUST_PYTHON_PARITY.md` add §7.1:
- Replacement framework is Rust-only; Python has no equivalent. All "would" behavior in Python is approximated via pre-event observers that mutate — a known faithfulness gap.
- Close the following previously-open Python parity items if they reference replacement semantics (audit the existing §6 entries).

In `.claude/plans/recursive-coalescing-candle.md`:
- Flip the Phase 7 row in the cumulative-readiness table to `✅ Landed 2026-04-21 (re-audit pending)`.
- Update "Immediate Next Steps" to suggest Phase 8 (Option card flow) as the next phase, which also requires a design spec.

In `docs/superpowers/plans/2026-04-21-rust-engine-phase-7-would-replacements.md` (this file):
- Add a "Status" section at the bottom marking each task's date and commit SHA.

- [ ] **Step 6: Run — full suite green**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: 617 + 2 = 619 passing. Zero warnings.

- [ ] **Step 7: Commit**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_PYTHON_PARITY.md .claude/plans/recursive-coalescing-candle.md docs/superpowers/plans/2026-04-21-rust-engine-phase-7-would-replacements.md digimon-engine/tests/replacements/behavioral_end_to_end.rs
git commit -m "rust-engine(phase-7): behavioral end-to-end + docs + roadmap flip"
```

---

## Deferred / Out of scope (confirmed)

The following are intentionally **not** in Phase 7:

- **Multi-replacement `TriggerOrder` full dispatch when both sides have >1 candidate.** Task 2 runs them in collection order with a `TODO`. A follow-up micro-task addresses this if a real card surfaces the ordering issue (audits don't flag any).
- **`WhenWouldAttack` / `WhenWouldBeAttackTarget` dispatch wiring.** Variants reserved in Task 1; Phase 9 (combat-interrupt completion) ships the dispatch at `begin_attack` and Raid target-switch sites.
- **Mutable `N` reduction on `WhenWouldBeDeDigivolved`.** v1 scripts that need "reduce de-digi by N" use `ctx.cancel()` + a manual follow-up call to `ctx.effect.de_digivolve(target, stop_at_level, Some(reduced_n))`. A mutable `amount` field on `ReplacementContext` could simplify this but is deferred — no audited card requires it.
- **`WhenWouldPlaceInSecurity` with ordered-position redirects.** v1 supports redirect-to-Trash semantics only. Position-swap (top↔bottom within security) is a rare enough effect that audits don't flag it.
- **Option-card replacement timings** (e.g. "counter this Option card"). Phase 8 Option flow covers the Option dispatch pipeline; replacement timings on Option plays compose there.

## Verification (per working rule 18 + acceptance criteria)

1. `cargo test --manifest-path digimon-engine/Cargo.toml` — full suite green after each task.
2. Each of Tasks 1–7 lands its specific behavioral DebugRunner test before implementation.
3. `DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v` still green (mask size unchanged at 2168 — Task 1 verifies).
4. Re-audit one archetype (TS Olympos suggested — largest Barrier/Evade surface) after Task 7 and confirm ~60-card unblock matches the roadmap projection.
5. No new warnings introduced (`cargo build --manifest-path digimon-engine/Cargo.toml 2>&1 | grep -i warning` empty).
6. Phase 0 `CannotBeDestroyedByBattle` tests still pass under new replacement-layer implementation (Task 5 verifies).

## Open questions that remain open (to resolve during task execution, not blocking plan acceptance)

Per spec §14 — these are not blockers but must be decided before the corresponding task's test is written:

1. **LeaveBattleArea + substitute interaction** (Task 3) — tentative: LeaveBattleArea fires, then Partition substitutes, LeaveBattleArea does NOT re-fire for the substitute source. Verify against a DCGO card.
2. **Barrier + empty deck** (Task 6) — tentative: accept fires, trash is a no-op, Barrier consumed. Verify against printed rules.
3. **`WhenWouldDraw` + `CannotDrawByEffect`** (Task 4) — tentative: flood gate wins, replacement does not fire. Covered explicitly in Task 4 test.
4. **`play_from_security` cause** (Task 4) — tentative: `SecurityCheck`.
5. **In-flight replacement grant** (Task 2) — tentative: forbidden; candidates collected once per `try_replace` entry.
6. **Counter-reduction as replacement?** — tentative: no, stays as a Phase 6-style gate.

---

## Status

(Fill in as tasks complete.)

- [ ] Task 1 — (date, commit SHA)
- [ ] Task 2 — (date, commit SHA)
- [ ] Task 3 — (date, commit SHA)
- [ ] Task 4 — (date, commit SHA)
- [ ] Task 5 — (date, commit SHA)
- [ ] Task 6 — (date, commit SHA)
- [ ] Task 7 — (date, commit SHA)

Full suite green at end of each task is mandatory before proceeding to the next.
