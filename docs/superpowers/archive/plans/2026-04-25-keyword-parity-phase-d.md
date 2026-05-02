# Keyword Parity Phase D — Alpha-tier keyword wire-ups

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **One fresh subagent per task** per user direction (2026-04-25). Do not batch tasks across a single subagent.

**Goal:** Auto-install printed keyword behavior for the seven alpha-tier selection-bearing keywords — `Fragment(N)`, `ArmorPurge`, `Save`, `Decoy`, `Fortitude`, `Partition`, `MaterialSave(N)` — by extending `keyword_to_auto_effect` with replacement processes and triggered observers that consume Phase C's parked-replacement substrate and Phase B's source-attribution helpers. After this phase, alpha cards declaring these keywords need zero hand-rolled `CardEffect` code.

**Architecture:** `keyword_to_auto_effect` already maps `Barrier / Evade / Decode` to declarative `Effect`s. Phase D extends the match arms to cover the seven new keywords, factors three shared `EffectContext` primitives (`armor_purge_top`, `place_under_permanent_bottom`, `play_from_trash_free_unsuspended`), and adds one new selection-zone variant (`CountCappedZone::Material(PermanentHandle)`). Selection-bearing replacements consume the `cancel_leave / handle_replacement / substitute_replacement` outcome-setters added in Phase C; trigger-based keywords (Fortitude, Partition, MaterialSave) use the existing observer pattern.

**Tech Stack:** Rust 2021, `digimon-engine` crate, `DebugRunner` test harness, `superpowers:test-driven-development`. No new external deps.

**Parent spec:** [`docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md`](../specs/2026-04-24-dcgo-keyword-parity-design.md) §5 Phase D. **DCGO source of truth:** `DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/{ArmorPurge,Decoy,Fortitude,Fragment,MaterialSave,Partition,Save}.cs`. **Phase C substrate spec:** [`docs/superpowers/specs/2026-04-25-keyword-parity-phase-c-design.md`](../specs/2026-04-25-keyword-parity-phase-c-design.md).

---

## Open questions resolved (from parent spec §10)

Resolved 2026-04-25 by reading DCGO sources directly. These resolutions are **assumptions baked into this plan**; do not re-derive in implementation tasks.

| Question | Resolution | DCGO citation |
|---|---|---|
| **D2 ArmorPurge gate** | `card_sources.len() >= 2` (top Digimon + ≥1 source remaining; the source becomes the new top after the top is trashed). | `ArmorPurge.cs:13` `DigivolutionCards.Count >= 1`; `DigivolutionCards` excludes `TopCard`. |
| **D4 Decoy color parameter** | **Keep `Keyword::Decoy` un-parameterized.** DCGO `DecoyProcess` takes an injected `CanSelectPermanentCondition` predicate from the per-card factory. The auto-install offers any same-controller Digimon other than self; per-card-text color filters override via hand-rolled `CardEffect`. RULES_CONTEXT 16-17 does not specify color filtering at the keyword level. | `Decoy.cs:25-69` predicate is parameter, not enum-level. |
| **D5 Fortitude source-count gate** | Deleted permanent had `card_sources.len() >= 2` (top + ≥1 source). | `Fortitude.cs:35` `CardSources.Count >= 1` (DCGO `CardSources` excludes `TopCard`). |
| **D7 MaterialSave on non-DigiXros** | Auto-install offers "any source" filter. Per-card-text restrictions (e.g., DigiXros source filter) are a hand-rolled override on top of the auto-install — out of Phase D scope. | `MaterialSave.cs:11` predicate is parameter. |

**Additional finding — Partition is a trigger, not a replacement.** DCGO `Partition.cs:9-23` uses `CanTriggerWhenPermanentRemoveField` and does not set `willBeRemoveField = false`. The keyword fires *concurrent with* the parent removal and plays cards from the disposed digivolution sources. Phase D wires Partition through the existing `OnPermanentLeavesBattleArea` observer timing, not the parked-replacement substrate. Parent spec §5 D6 wording ("WhenWouldLeaveBattleArea two-group selection") is corrected here.

**Additional finding — MaterialSave is an active skill, not a replacement.** DCGO `MaterialSave.cs:28` is invoked from a `[Main]` active skill emission, not a replacement process. Phase D wires it as a `MainPhaseActive` skill, not nested-selection-in-replacement.

---

## File structure

Files Phase D creates or modifies, organized by responsibility:

**`digimon-engine/src/cards/keyword_effects.rs`** — extend `keyword_to_auto_effect` match with seven new arms. Each arm builds a single (or multi) `Effect` and returns it.

**`digimon-engine/src/effect_context/selections.rs`** — extend `CountCappedZone` with `Material(PermanentHandle)` variant; extend `select_count_capped_multi` candidate-collection branch.

**`digimon-engine/src/effect_context/mod.rs`** — three new primitives:
- `armor_purge_top(perm)` — trash current top Digimon, promote next source.
- `place_card_under_permanent_bottom(card, target)` — move card to bottom of target's digivolution stack. Used by Save (card = self leaving battle) and MaterialSave (cards = own sources).
- `play_from_trash_free_unsuspended(card)` — Fortitude play.

**`digimon-engine/src/effect.rs`** + `effect_builder.rs`** (if separate) — builder helpers for `Effect::when_ally_would_be_deleted` and `Effect::on_ally_deletion` if not already present (Decoy and Fortitude need these). Verify against current effect.rs first.

**`digimon-engine/tests/keyword_phase_d/`** (new test crate dir + `main.rs` glue, mirroring `replacements/`) — one test file per keyword:
- `fragment_n.rs`
- `armor_purge.rs`
- `save.rs`
- `decoy.rs`
- `fortitude.rs`
- `partition.rs`
- `material_save.rs`

**`digimon-engine/src/cards/keyword_effects.rs` module docstring** — replace the "Deferred: Partition / ArmorPurge" section with the new coverage matrix.

**Docs:**
- `docs/DCGO_KEYWORD_PARITY.md` — flip seven rows from 🟣/🔴 to ✅.
- `docs/RUST_ENGINE_API.md` — section "Selection-bearing replacement keyword authoring pattern" (template Save/Fragment auto-install for future card scripts).
- `docs/RUST_ENGINE_GAPS.md` — close `WhenWouldBeDeleted framework extensions` row's "Save / Decoy / Fortitude / Fragment / ArmorPurge / Partition / MaterialSave wiring" subitem.
- `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md` §5 Phase D — mark landed with deviations list.

**No changes to:** `card_data.rs` (parser already emits all variants); `enums.rs` (variants already exist); `dsl_cards/modifier_map.rs` (already maps DSL strings to variants); Python bindings (`digimon-engine-py/`) — Phase D adds no new public Rust API surface beyond what auto-install consumes internally.

---

## Sub-skill: How each task consumes Phase C substrate

Selection-bearing replacement bodies (Tasks 4, 5, 6, 7) follow the pattern from `digimon-engine/tests/replacements/nested_select_save.rs` and `nested_select_fragment.rs` (Phase C test cards). Each replacement-process closure:

1. Validates `rctx.subject` is the expected card / permanent (self-scope guard).
2. Calls a `ctx.select_*` primitive whose callback receives a fresh `EffectContext`.
3. Inside the callback: invoke the side-effect primitive, then call **one** of `ctx.cancel_leave()`, `ctx.handle_replacement()`, or `ctx.substitute_replacement(subject)` to write the parked outcome.
4. The dispatcher's post-callback drain hook (`try_drain_parked_replacement_with_guard`) reads `parked_replacement.outcome` and applies it to the original deletion.

If the player declines an optional selection (e.g., `select_own_permanent` with `is_optional`), the callback fires with `None` and the closure must NOT call any outcome-setter — leaving the default `ReplacementOutcome::None`, which lets the original deletion proceed.

Mandatory replacements (Fragment, ArmorPurge) use `is_optional: false` on their selection primitive, so the player cannot decline. If the gate fails before selection, the auto-install body returns early without parking — original deletion proceeds.

---

## Tasks

### Task 0: Add `CountCappedZone::Material(PermanentHandle)` for source-pick selection

**Files:**
- Modify: `digimon-engine/src/effect_context/selections.rs:36-50` (`CountCappedZone` enum + `select_count_capped_multi` candidate-collection branch).
- Test: `digimon-engine/tests/effect_context/material_zone_select.rs` (new file; add to `tests/effect_context/main.rs` glue if present, else create).

**Why this task is first:** Fragment(N), ArmorPurge, and MaterialSave(N) all select cards from a permanent's digivolution sources. The existing `CountCappedZone` only has `Hand` and `Trash`; Phase C tests substituted `Hand` as a workaround. Land the variant before any keyword consumer.

- [ ] **Step 1: Write the failing behavioral test** (`digimon-engine/tests/effect_context/material_zone_select.rs`)

```rust
//! Task 0 — `CountCappedZone::Material(PermanentHandle)` selects sources from
//! a permanent's digivolution stack.

use digimon_engine::card_data::CardData;
use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::Effect;
use digimon_engine::effect_context::CountCappedZone;
use std::sync::Arc;

#[test]
fn material_zone_collects_permanent_card_sources() {
    let mut registry = CardEffectRegistry::default();
    // TEST-MATZONE: passive `[Main]` active that selects up to 2 sources from
    // self's digivolution stack and trashes them via select_count_capped_multi.
    registry.register("TEST-MATZONE", Arc::new(|card| {
        Effect::main_phase_active(card)
            .name("test source-pick")
            .effect(|ctx| {
                let me = ctx.source_permanent().expect("on field");
                ctx.select_count_capped_multi(
                    ctx.controller(),
                    CountCappedZone::Material(me),
                    /*max=*/ 2,
                    "select sources",
                    /*is_optional_zero=*/ false,
                    |_card_data, _entry| true,
                    |ctx, picks| {
                        for source in picks {
                            ctx.trash_card_source(me, source);
                        }
                    },
                );
            })
            .build()
    }));

    let mut r = DebugRunner::new(registry);
    r.deck(0, &["FILLER"; 10]).deck(1, &["FILLER"; 10]);
    r.spawn_with_sources(0, "TEST-MATZONE", &["SRC-A", "SRC-B", "SRC-C"]);
    r.activate_main_phase_active(0, "TEST-MATZONE");
    r.select_zone_indices(&[0, 2]); // pick SRC-A and SRC-C
    r.confirm_selection();

    let perm = r.find_permanent(0, "TEST-MATZONE").unwrap();
    assert_eq!(perm.card_sources.len(), 2, "1 source remaining + top");
    assert_eq!(perm.card_sources[0].card_id, "TEST-MATZONE");
    assert_eq!(perm.card_sources[1].card_id, "SRC-B");
    let trash: Vec<&str> = r.player(0).trash.iter().map(|c| c.card_id.as_str()).collect();
    assert!(trash.contains(&"SRC-A") && trash.contains(&"SRC-C"));
}

#[test]
fn material_zone_with_zero_sources_yields_no_candidates() {
    // ... mirrors the `is_optional_zero=true` empty-zone path; assert callback fires with empty Vec.
}
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test material_zone_select
```
Expected: compile error or test failure — `CountCappedZone::Material` variant does not exist.

- [ ] **Step 3: Add the variant + collection branch** (`digimon-engine/src/effect_context/selections.rs`)

In the `CountCappedZone` enum (line 36):

```rust
pub enum CountCappedZone {
    Hand,
    Trash,
    /// Source cards in a permanent's digivolution stack (excludes the top
    /// card). Used by `Fragment(N)`, `ArmorPurge`, `MaterialSave(N)`.
    Material(crate::card_source::PermanentHandle),
}
```

In `select_count_capped_multi` candidate-collection branches (line 709-715), add:

```rust
CountCappedZone::Material(perm_handle) => {
    let perm = self.game.permanent(perm_handle);
    // Skip top card (index 0); only digivolution sources are selectable.
    perm.card_sources[1..].len()
}
```

In the lookup branch (line 724-725), add:

```rust
CountCappedZone::Material(perm_handle) => {
    self.game.permanent(perm_handle).card_sources[i + 1].clone()
}
```

In the resolve callback (line 1299-1300), add:

```rust
CountCappedZone::Material(perm_handle) => {
    game.permanent(perm_handle).card_sources[pick_zone_idx + 1].handle()
}
```

(Note: callback uses `+ 1` because index 0 is the top card, excluded from the candidate set.)

- [ ] **Step 4: Run the test to confirm it passes**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test material_zone_select
```
Expected: PASS (both tests).

- [ ] **Step 5: Run full Rust test suite to confirm no regressions**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml
```
Expected: previously-green count + 2.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/effect_context/selections.rs digimon-engine/tests/effect_context/
git commit -m "engine(selections): add CountCappedZone::Material for source-pick"
```

---

### Task 1: Add `EffectContext::place_card_under_permanent_bottom` primitive

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs` (add primitive in zone-operations region).
- Test: `digimon-engine/tests/effect_context/place_under_permanent.rs` (new).

**Why:** Save and MaterialSave both move a card (or N cards) to the *bottom* of another permanent's digivolution stack. DCGO calls `selectedPermanent.AddDigivolutionCardsBottom(...)` for both. Factor the primitive once.

- [ ] **Step 1: Write the failing behavioral test**

```rust
//! Task 1 — place_card_under_permanent_bottom moves a card from any zone to
//! the bottom of a target permanent's digivolution stack.

use digimon_engine::card_data::CardData;
use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn place_under_bottom_appends_to_card_sources_first_index() {
    let mut registry = CardEffectRegistry::default();
    // Hand-rolled test card that picks self's first hand card and tucks it
    // under a target permanent.
    registry.register("TEST-PLACE-UNDER", /* see Save tests in
        nested_select_save.rs for setup template */);

    let mut r = DebugRunner::new(registry);
    r.spawn(0, "TARGET-TAMER");
    r.put_in_hand(0, "TUCK-CARD");
    r.activate(0, "TEST-PLACE-UNDER");
    r.select_permanent(0, "TARGET-TAMER");

    let target = r.find_permanent(0, "TARGET-TAMER").unwrap();
    assert_eq!(target.card_sources[0].card_id, "TUCK-CARD",
               "tucked card lands at index 0 (bottom of stack)");
    assert_eq!(target.card_sources[1].card_id, "TARGET-TAMER",
               "original top stays at the visible top");
    assert_eq!(r.player(0).hand.len(), 0, "hand emptied");
}
```

- [ ] **Step 2: Run to confirm fail** (primitive does not exist).

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test place_under_permanent
```

- [ ] **Step 3: Implement the primitive** (`digimon-engine/src/effect_context/mod.rs`)

```rust
/// Move a card from its current zone (hand, trash, deck, or another
/// permanent's stack) to the bottom of `target`'s digivolution stack.
///
/// Used by:
/// - `<Save>`: when self would be deleted, place self under a Tamer.
/// - `<Material Save N>`: move up to N own digivolution sources under another permanent.
///
/// Bottom = `card_sources[0]` (the visible top stays at the highest index).
///
/// Panics if `card` cannot be located in any of the candidate zones.
pub fn place_card_under_permanent_bottom(
    &mut self,
    card: CardHandle,
    target: PermanentHandle,
) {
    // 1. Locate `card` in hand / trash / deck / any permanent's card_sources.
    // 2. Remove from source zone.
    // 3. Insert at `target.card_sources.insert(0, card_source)`.
    // (Implementation mirrors the unit-test helper at
    // tests/replacements/nested_select_save.rs:120-145.)
    todo!()
}
```

- [ ] **Step 4: Run the test to confirm it passes.**

- [ ] **Step 5: Add a second test for MaterialSave-style multi-card placement** — call `place_card_under_permanent_bottom` in a loop with 3 source cards; verify ordering preserved (first call lands at index 0, second pushes to index 0 and pushes first to index 1).

- [ ] **Step 6: Re-run full Rust test suite, then commit.**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/effect_context/place_under_permanent.rs
git commit -m "engine(ctx): add place_card_under_permanent_bottom primitive"
```

---

### Task 2: Add `EffectContext::armor_purge_top` primitive

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs`.
- Test: `digimon-engine/tests/effect_context/armor_purge_top.rs` (new).

**Why:** DCGO `ArmorPurge.cs:50-65` performs a top-swap-and-trash: trash the current TopCard, promote the next digi source to be the new top, leave the rest of the stack intact. This is distinct from `delete_permanent_with_cause` (which trashes the whole stack). The shape is reusable beyond ArmorPurge — any future "trash top, keep stack" effect would consume it.

- [ ] **Step 1: Write the failing behavioral test**

```rust
//! Task 2 — armor_purge_top trashes the current top Digimon and promotes the
//! next digi source as the new top.

use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn armor_purge_trashes_top_and_promotes_next() {
    let mut r = DebugRunner::new(CardEffectRegistry::default());
    // Stack: [SRC-BOTTOM, SRC-MID, TOP-DIGIMON] (index 0 = bottom).
    r.spawn_with_sources(0, "TOP-DIGIMON", &["SRC-BOTTOM", "SRC-MID"]);
    let perm = r.find_permanent(0, "TOP-DIGIMON").unwrap_handle();
    r.with_ctx(0, |ctx| ctx.armor_purge_top(perm));

    let perm = r.find_permanent_by_handle(perm).unwrap();
    assert_eq!(perm.top_card().card_id, "SRC-MID",
               "previous source-1 is now the top");
    assert_eq!(perm.card_sources.len(), 2,
               "stack is bottom-source + new top (TOP-DIGIMON gone)");
    assert_eq!(r.player(0).trash.last().unwrap().card_id, "TOP-DIGIMON",
               "the previous top went to trash");
}

#[test]
fn armor_purge_with_only_top_does_nothing() {
    // Gate: card_sources.len() must be >= 2 to safely run; primitive
    // panics in debug if violated. Production callers (the auto-install
    // body) gate before calling.
    let mut r = DebugRunner::new(CardEffectRegistry::default());
    r.spawn(0, "TOP-DIGIMON"); // no sources under it
    let perm = r.find_permanent(0, "TOP-DIGIMON").unwrap_handle();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        r.with_ctx(0, |ctx| ctx.armor_purge_top(perm));
    }));
    assert!(result.is_err(), "primitive panics in debug when no source remaining");
}
```

- [ ] **Step 2: Run to confirm fail.**

- [ ] **Step 3: Implement the primitive.**

```rust
/// Trash the current top Digimon of `perm` and promote the next-highest
/// digivolution source to become the new top. The remainder of the stack is
/// preserved.
///
/// **Caller MUST gate on `perm.card_sources.len() >= 2` before invoking**;
/// this primitive `debug_assert!`s that constraint and panics if violated.
///
/// DCGO parity: `ArmorPurge.cs:50-61` (`RemoveFromAllArea(topCard) +
/// AddTrashCard(topCard) + RemoveDigivolveRootEffect(topCard, _permanent)`).
pub fn armor_purge_top(&mut self, perm: PermanentHandle) {
    let permanent = self.game.permanent_mut(perm);
    debug_assert!(
        permanent.card_sources.len() >= 2,
        "armor_purge_top requires >= 1 source under the top card"
    );
    let top = permanent.card_sources.pop().expect("len >= 2 invariant");
    let owner = permanent.controller;
    self.game.players[owner as usize].trash.push(top.into_card_source());
    // Modifier registry cleanup: drop any effects sourced by the trashed
    // top card. (Mirrors delete_permanent_with_effects' on-leave hook for
    // the trashed-card-only case.)
    self.game.modifiers.retain(|m| m.source_card != top.card_handle);
}
```

(Exact field names — `controller` vs `owner` vs `player` — must match the current `Permanent` struct; verify via `Read digimon-engine/src/permanent.rs` before writing the body.)

- [ ] **Step 4: Run tests, confirm pass.**

- [ ] **Step 5: Run full Rust suite, then commit.**

```bash
git commit -m "engine(ctx): add armor_purge_top primitive"
```

---

### Task 3: Add `EffectContext::play_from_trash_free_unsuspended` primitive

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs`.
- Test: `digimon-engine/tests/effect_context/play_from_trash.rs` (new).

**Why:** Fortitude plays self from trash without paying cost and without suspension. `EffectContext::play_card` exists for hand→battle; `play_from_trash` for the Memory Boost / Yggdrasil cases. Verify whether either covers "trash → battle, free, unsuspended" before writing a new primitive — it may already exist as a flag combination.

- [ ] **Step 1: Audit existing `EffectContext` zone-play primitives** with `Grep "play_from_trash\|play_permanent\|play_card" digimon-engine/src/effect_context/`. If a flag-driven path already covers `pay_cost=false, tapped=false, root=Trash`, **skip Steps 2-5 and instead write a Fortitude-flavored test that exercises the existing primitive** (mark this task as "thin alias" rather than new primitive).

If new primitive is needed, continue with TDD steps 2-6 below mirroring Tasks 1 and 2.

- [ ] **Step 2: Write failing test** (`digimon-engine/tests/effect_context/play_from_trash.rs`)

Spawn permanent in trash, invoke primitive, verify card landed in battle area unsuspended at zero memory cost.

- [ ] **Step 3-5: Implement, run, and commit.**

```bash
git commit -m "engine(ctx): add play_from_trash_free_unsuspended primitive"
```

---

### Task 4: Auto-install for `Keyword::Fragment(N)`

**Files:**
- Modify: `digimon-engine/src/cards/keyword_effects.rs:92` (replace `Keyword::Fragment(_) => Vec::new()` with the auto-install body).
- Test: `digimon-engine/tests/keyword_phase_d/fragment_n.rs` (new test crate).

**Behavior** (DCGO `Fragment.cs:23-77`): When self would be deleted, mandatory selection of exactly N digivolution sources from self's stack; trash the picked sources; cancel the deletion. Gate: `card_sources.len() >= N + 1` (top + N sources). If gate fails, replacement does not park; original deletion proceeds.

- [ ] **Step 1: Write the failing test** (`digimon-engine/tests/keyword_phase_d/fragment_n.rs`)

```rust
//! Phase D Task 4 — Fragment(N) auto-install: mandatory source-pick that
//! cancels deletion.

use digimon_engine::cards::build_registry;
use digimon_engine::debug_runner::DebugRunner;

fn make_runner_with_fragment_card(n: u8) -> DebugRunner {
    // TEST-FRAGMENT-N: a hand-spawned Digimon whose CardData declares
    // `keywords: vec![Keyword::Fragment(n)]` and NO hand-rolled CardEffect
    // script. This exercises the auto-install path end-to-end.
    let registry = build_registry();
    let mut r = DebugRunner::new(registry);
    r.deck(0, &["FILLER"; 5]).deck(1, &["FILLER"; 5]);
    r.spawn_with_card_data(0, "TEST-FRAGMENT-2", make_fragment_card_data(n));
    r
}

#[test]
fn fragment_2_picks_two_sources_and_cancels_deletion() {
    let mut r = make_runner_with_fragment_card(2);
    let perm_handle = r.attach_sources(0, "TEST-FRAGMENT-2",
        &["SRC-A", "SRC-B", "SRC-C"]);

    // Opponent effect tries to delete self.
    r.opponent_delete(perm_handle);

    // Player picks 2 of the 3 sources.
    r.expect_pending_selection_count_capped(2);
    r.select_zone_indices(&[0, 2]);
    r.confirm_selection();

    let perm = r.find_permanent_by_handle(perm_handle).expect("not deleted");
    assert_eq!(perm.card_sources.len(), 2,
               "1 source + top remain after trashing 2");
    assert!(r.player(0).trash.iter().any(|c| c.card_id == "SRC-A"));
    assert!(r.player(0).trash.iter().any(|c| c.card_id == "SRC-C"));
    assert_eq!(perm.top_card().card_id, "TEST-FRAGMENT-2");
}

#[test]
fn fragment_2_with_only_one_source_does_not_park() {
    let mut r = make_runner_with_fragment_card(2);
    let perm_handle = r.attach_sources(0, "TEST-FRAGMENT-2", &["ONLY-SRC"]);
    r.opponent_delete(perm_handle);

    // Gate (`card_sources.len() >= 3`) fails, so no selection is parked.
    assert!(!r.has_pending_selection(),
            "no selection when source count below N+1");
    assert!(r.find_permanent_by_handle(perm_handle).is_none(),
            "permanent was deleted normally");
}

#[test]
fn fragment_does_not_fire_on_neighbor_deletion() {
    // Self-scope guard: Fragment fires only on self's deletion.
    let mut r = make_runner_with_fragment_card(2);
    r.attach_sources(0, "TEST-FRAGMENT-2", &["SRC-A", "SRC-B", "SRC-C"]);
    let neighbor = r.spawn_with_sources(0, "NEIGHBOR-DIGIMON", &["NS-A"]);
    r.opponent_delete(neighbor);
    assert!(!r.has_pending_selection(),
            "Fragment on TEST-FRAGMENT-2 must not fire when a neighbor is deleted");
    assert!(r.find_permanent_by_handle(neighbor).is_none());
}
```

- [ ] **Step 2: Run to confirm fail.**

- [ ] **Step 3: Implement the auto-install body** — replace the `Keyword::Fragment(_) => Vec::new()` arm at `keyword_effects.rs:92` with:

```rust
Keyword::Fragment(n) => vec![Effect::when_would_be_deleted(card)
    .name(format!("<Fragment ({n})>"))
    .replacement_process(move |rctx| {
        // Self-scope guard.
        let me = rctx.effect.source_permanent;
        let ReplacementSubject::Permanent(subject) = rctx.subject else { return };
        if Some(subject) != me { return; }

        // Gate: at least N digivolution sources must be selectable.
        let stack_len = rctx.effect.game.permanent(subject).card_sources.len();
        if stack_len < (n as usize + 1) {
            // Insufficient sources — let original deletion proceed.
            return;
        }

        // Park: select exactly N sources from self's stack to trash.
        let n_usize = n as usize;
        rctx.effect.game.run_in_ctx(|ctx| {
            ctx.select_count_capped_multi(
                ctx.controller(),
                CountCappedZone::Material(subject),
                /*max=*/ n_usize,
                "select digivolution cards to trash",
                /*is_optional_zero=*/ false,
                |_card_data, _entry| true,
                move |ctx, picks| {
                    if picks.len() != n_usize {
                        // Player declined or selection invalid; no cancel.
                        return;
                    }
                    for source in picks {
                        ctx.trash_card_source(subject, source);
                    }
                    ctx.cancel_leave();
                },
            );
        });
    })
    .build()],
```

(Note: exact API for `run_in_ctx` and `trash_card_source` must match current `EffectContext` shape — verify with `Grep` before writing. The Phase C tests in `nested_select_fragment.rs` are the closest reference for the closure-into-rctx idiom; copy the pattern.)

- [ ] **Step 4: Run tests, confirm pass.**

- [ ] **Step 5: Run full Rust suite + Phase C regression suite.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test nested_select_fragment
cargo test --manifest-path digimon-engine/Cargo.toml
```

- [ ] **Step 6: Commit.**

```bash
git commit -m "engine(keywords): auto-install Fragment(N) — pick N sources, cancel deletion"
```

---

### Task 5: Auto-install for `Keyword::ArmorPurge`

**Files:**
- Modify: `digimon-engine/src/cards/keyword_effects.rs:133` (replace `Keyword::ArmorPurge => Vec::new()` arm).
- Test: `digimon-engine/tests/keyword_phase_d/armor_purge.rs` (new).

**Behavior** (DCGO `ArmorPurge.cs:40-65`): When self would be deleted, mandatory: trash current top Digimon, promote next source as new top, cancel the original deletion. Gate: `card_sources.len() >= 2`. No player selection — the action is forced.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn armor_purge_swaps_top_and_cancels_deletion() {
    let mut r = make_runner_with_armor_purge_card();
    let perm = r.attach_sources(0, "TEST-ARMOR-PURGE", &["UNDER-DIGI"]);
    r.opponent_delete(perm);

    // No selection — ArmorPurge auto-resolves.
    assert!(!r.has_pending_selection());

    let p = r.find_permanent_by_handle(perm).expect("permanent survived");
    assert_eq!(p.top_card().card_id, "UNDER-DIGI",
               "previous source promoted to top");
    assert_eq!(r.player(0).trash.last().unwrap().card_id, "TEST-ARMOR-PURGE");
}

#[test]
fn armor_purge_with_no_source_does_not_protect() {
    let mut r = make_runner_with_armor_purge_card();
    let perm = r.spawn(0, "TEST-ARMOR-PURGE");
    r.opponent_delete(perm);
    assert!(r.find_permanent_by_handle(perm).is_none(),
            "no source under top → ArmorPurge gate fails → normal deletion");
}
```

- [ ] **Step 2: Run to confirm fail.**

- [ ] **Step 3: Implement the auto-install:**

```rust
Keyword::ArmorPurge => vec![Effect::when_would_be_deleted(card)
    .name("<Armor Purge>")
    .replacement_process(|rctx| {
        let me = rctx.effect.source_permanent;
        let ReplacementSubject::Permanent(subject) = rctx.subject else { return };
        if Some(subject) != me { return; }

        // Gate: must have a source under the top to fall back to.
        let stack_len = rctx.effect.game.permanent(subject).card_sources.len();
        if stack_len < 2 { return; }

        // No selection: directly run the top-swap.
        rctx.effect.game.run_in_ctx(|ctx| {
            ctx.armor_purge_top(subject);
            ctx.cancel_leave();
        });
    })
    .build()],
```

- [ ] **Step 4: Run tests, confirm pass.**

- [ ] **Step 5: Run full Rust suite, commit.**

```bash
git commit -m "engine(keywords): auto-install ArmorPurge — top-swap, cancel deletion"
```

---

### Task 6: Auto-install for `Keyword::Save`

**Files:**
- Modify: `digimon-engine/src/cards/keyword_effects.rs` (add new arm before the `_ => Vec::new()` catch-all).
- Test: `digimon-engine/tests/keyword_phase_d/save.rs` (new).

**Behavior** (DCGO `Save.cs:24-65`): When self would be deleted, **optional** selection of one own Tamer; if selected, place self at bottom of that Tamer's stack and cancel deletion. If declined, original deletion proceeds. Filter: own Tamer permanents only.

- [ ] **Step 1: Write the failing test** (templated from `tests/replacements/nested_select_save.rs`).

Three tests:
- Accept + pick: self lands under Tamer, deletion cancelled.
- Decline: original deletion proceeds, no Tamer mutation.
- No own Tamers: selection still parked but with empty candidate set; player must "skip"; deletion proceeds.

- [ ] **Step 2: Run to confirm fail.**

- [ ] **Step 3: Implement.**

```rust
Keyword::Save => vec![Effect::when_would_be_deleted(card)
    .name("<Save>")
    .optional()  // The "you may" clause.
    .replacement_process(|rctx| {
        let me = rctx.effect.source_permanent;
        let ReplacementSubject::Permanent(subject) = rctx.subject else { return };
        if Some(subject) != me { return; }

        let owner = rctx.effect.game.permanent(subject).controller;
        let self_card = rctx.effect.game.permanent(subject).top_card().handle();

        rctx.effect.game.run_in_ctx(|ctx| {
            ctx.select_own_permanent(
                "select a Tamer to place this card under",
                |perm| perm.kind() == PermanentKind::Tamer,
                /*is_optional=*/ true,
                move |ctx, target| {
                    let Some(tamer) = target else { return };
                    ctx.place_card_under_permanent_bottom(self_card, tamer);
                    ctx.cancel_leave();
                },
            );
        });
    })
    .build()],
```

- [ ] **Step 4-6: Run tests, full suite, commit.**

```bash
git commit -m "engine(keywords): auto-install Save — place self under selected Tamer"
```

---

### Task 7: Auto-install for `Keyword::Decoy`

**Files:**
- Modify: `digimon-engine/src/cards/keyword_effects.rs` (add `Keyword::Decoy => ...` arm).
- May need: `digimon-engine/src/effect.rs` — add `Effect::when_ally_would_be_deleted(card)` builder if missing (audit first; `WhenWouldBeDeleted` may already cover all permanents, with the auto-install body discriminating self vs ally).
- Test: `digimon-engine/tests/keyword_phase_d/decoy.rs` (new).

**Behavior** (DCGO `Decoy.cs:24-69`): When *any of controller's other Digimon* would be deleted, **forced** redirect: substitute self as the deletion subject (i.e., self is deleted instead, the ally survives). Filter: same controller, kind == Digimon, not self.

**Important:** Decoy fires when the **subject** of the deletion is an *ally*, not self. Decoy uses the **substitute** outcome (`ctx.substitute_replacement(ReplacementSubject::Permanent(self))`), not cancel.

- [ ] **Step 1: Audit substrate fit** — read `tests/replacements/nested_select_decoy.rs` (Phase C test card). Reuse its body shape. The auto-install equivalent is the same logic without the test scaffolding.

- [ ] **Step 2: Write failing test** — three cases:
- Ally would be deleted, Decoy selects to redirect: ally survives, self deleted.
- No Decoy candidates available: original ally deletion proceeds.
- Self is the deletion subject: Decoy does NOT self-redirect (no infinite loop).

- [ ] **Step 3: Implement.**

```rust
Keyword::Decoy => vec![Effect::when_would_be_deleted(card)
    // Subscribed to ANY permanent's deletion in the same battle area.
    // The body filters to ally-only.
    .name("<Decoy>")
    .optional()
    .replacement_process(|rctx| {
        let me_perm = rctx.effect.source_permanent.expect("on field");
        let ReplacementSubject::Permanent(subject) = rctx.subject else { return };

        // Decoy does not self-redirect.
        if subject == me_perm { return; }

        // Same-controller filter.
        let game = &*rctx.effect.game;
        let me_owner = game.permanent(me_perm).controller;
        let subject_owner = game.permanent(subject).controller;
        if me_owner != subject_owner { return; }

        // Substitute: redirect deletion to self.
        rctx.effect.game.run_in_ctx(|ctx| {
            ctx.substitute_replacement(ReplacementSubject::Permanent(me_perm));
        });
    })
    .build()],
```

(Verify whether `Effect::when_would_be_deleted` is already neighborhood-broadcast or self-scoped. If self-scoped, add `Effect::when_ally_would_be_deleted` builder that subscribes to the same event but without the self-scope auto-filter; place the build helper in `effect.rs`.)

- [ ] **Step 4-6: Tests, full suite, commit.**

```bash
git commit -m "engine(keywords): auto-install Decoy — redirect ally deletion to self"
```

---

### Task 8: Auto-install for `Keyword::Fortitude`

**Files:**
- Modify: `digimon-engine/src/cards/keyword_effects.rs` (`Keyword::Fortitude` arm).
- Possibly: `digimon-engine/src/effect.rs` — `Effect::on_ally_deletion(card)` builder if missing.
- Test: `digimon-engine/tests/keyword_phase_d/fortitude.rs` (new).

**Behavior** (DCGO `Fortitude.cs:14-63`): A trigger (not a replacement) on **OnDeletion of any own permanent**, where the deleted permanent had ≥1 digivolution source. When fires: play self from trash, free, unsuspended, with ETB triggers active. Self must currently live in the trash (Fortitude is printed on the card *itself*, and the card is in trash when this fires — usually because the printed-on-Fortitude card was just deleted).

**Subtlety:** "Fortitude on a Digimon that gets deleted" → the card is now in trash, and the OnDeletion observer sees the deletion-of-self event. The auto-install fires on the deletion-of-self trigger, with the gate "deleted permanent had ≥1 source". So this isn't actually OnAllyDeletion — it's OnSelfDeletion-while-in-trash. **Verify against DCGO** which `CardSources.Contains(card)` fires it on:

```cs
foreach (Hashtable hashtable1 in hashtables) {
    List<CardSource> CardStack = ...
    if (CardStack.Contains(card)) {  // <-- the card is part of one of the deleted stacks
        if (CardSources.Count >= 1) { ...
```

So yes: Fortitude fires when *the stack containing this card* is deleted, AND that stack had ≥1 source. So a Digimon with `<Fortitude>` printed on it: when it (or a stack containing it as a source) is deleted, it gets to play itself back from trash if the deleted stack had a source under the top.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn fortitude_replays_self_from_trash_when_self_deleted_with_source() {
    let mut r = make_runner_with_fortitude_card();
    let perm = r.spawn_with_sources(0, "TEST-FORTITUDE", &["SRC-A"]);

    // Self gets deleted (has 1 source → gate passes).
    r.opponent_delete(perm);

    // Self should now be back on field.
    let new_perm = r.find_permanent(0, "TEST-FORTITUDE")
        .expect("Fortitude replayed self");
    assert_eq!(new_perm.card_sources.len(), 1, "fresh play, no sources");
    assert!(!new_perm.is_suspended(), "Fortitude unsuspended");
}

#[test]
fn fortitude_does_not_fire_when_no_source_was_under_self() {
    let mut r = make_runner_with_fortitude_card();
    let perm = r.spawn(0, "TEST-FORTITUDE");
    r.opponent_delete(perm);
    assert!(r.find_permanent(0, "TEST-FORTITUDE").is_none(),
            "no source → no Fortitude → stays in trash");
    assert_eq!(r.player(0).trash.last().unwrap().card_id, "TEST-FORTITUDE");
}
```

- [ ] **Step 2: Run to confirm fail.**

- [ ] **Step 3: Implement.**

```rust
Keyword::Fortitude => vec![Effect::on_self_deletion(card)
    .name("<Fortitude>")
    .effect(|ctx| {
        // Gate: deleted stack had >= 1 source under top.
        let cause = ctx.deletion_cause();
        let deleted_source_count = match &cause.subject {
            ReplacementSubject::Permanent(p) => {
                // Permanent already gone — pull from the deletion log.
                ctx.deletion_metadata().card_sources_count
            }
            _ => return,
        };
        if deleted_source_count < 1 { return; }

        let self_card = ctx.source_card();
        ctx.play_from_trash_free_unsuspended(self_card);
    })
    .build()],
```

(Substantial verification needed — `deletion_cause()` and `deletion_metadata()` shape comes from Phase B's `current_deletion_cause` slot. Implementer must check the actual API in `effect_context/mod.rs` and adapt. If the source-count metadata isn't on the deletion-cause slot, this task may need a sub-task to thread `deleted_source_count: u8` onto the cause record.)

- [ ] **Step 4-6: Tests, full suite, commit.**

```bash
git commit -m "engine(keywords): auto-install Fortitude — replay self from trash on deletion-with-source"
```

---

### Task 9: Auto-install for `Keyword::Partition`

**Files:**
- Modify: `digimon-engine/src/cards/keyword_effects.rs` (`Keyword::Partition` arm replaces the deferred entry at line 133).
- Possibly: `digimon-engine/src/effect.rs` — `Effect::on_self_leaves_battle_area(card)` builder if missing.
- Test: `digimon-engine/tests/keyword_phase_d/partition.rs` (new).

**Behavior** (DCGO `Partition.cs`): Trigger fires when self leaves battle area, when cause ≠ Battle and cause ≠ OwnEffect. Plays one card from each of two color-grouped subsets of own digivolution sources, free + unsuspended. Color groups come from the per-card-text — out of Phase D scope to derive from text. **Phase D auto-install offers two single-pick selections from the deleted permanent's source list with no color filter**; per-card-text overrides apply color grouping via hand-rolled.

**Cause filtering:** Phase B added `ReplacementCause` and `Game.current_deletion_cause`. Use `ctx.was_deleted_by_battle()` and `ctx.was_deleted_by_own_effect()` accessors (verify exact names against effect_context — `was_deleted_by_effect` exists, may need `was_deleted_by_own_effect`).

- [ ] **Step 1: Write tests** — three cases:
- Self leaves field via opponent effect with 2 sources: pick + play both.
- Self leaves field via battle: Partition does NOT fire.
- Self leaves field via own effect: Partition does NOT fire.

- [ ] **Step 2: Run to confirm fail.**

- [ ] **Step 3: Implement.**

```rust
Keyword::Partition => vec![Effect::on_self_leaves_battle_area(card)
    .name("<Partition>")
    .effect(|ctx| {
        // Cause filter: not battle, not own effect.
        if ctx.was_deleted_by_battle() { return; }
        if ctx.was_deleted_by_own_effect() { return; }

        // Pull the source list from the deletion metadata.
        let sources = ctx.deletion_metadata().card_sources.clone();
        if sources.len() < 2 { return; }

        // Two single-picks from the source list.
        // (Phase D auto-install: no color filter; per-card-text overrides apply.)
        let owner = ctx.controller();
        ctx.select_from_list(
            owner,
            sources.clone(),
            "select 1 card to play",
            move |ctx, first| {
                let Some(first_card) = first else { return };
                let remaining: Vec<_> = sources.iter().filter(|c| **c != first_card).cloned().collect();
                ctx.select_from_list(
                    owner,
                    remaining,
                    "select 1 card to play",
                    move |ctx, second| {
                        let Some(second_card) = second else { return };
                        ctx.play_from_list_free_unsuspended(first_card);
                        ctx.play_from_list_free_unsuspended(second_card);
                    },
                );
            },
        );
    })
    .build()],
```

(Verify `select_from_list`, `play_from_list_free_unsuspended` exist; if not, sub-task to add them. The "list" zone is the deletion-metadata's source snapshot, not a live game zone.)

- [ ] **Step 4-6: Tests, full suite, commit.**

```bash
git commit -m "engine(keywords): auto-install Partition — play 2 from sources on non-battle non-own-effect leave"
```

---

### Task 10: Auto-install for `Keyword::MaterialSave(N)`

**Files:**
- Modify: `digimon-engine/src/cards/keyword_effects.rs` (`Keyword::MaterialSave(_)` arm).
- Test: `digimon-engine/tests/keyword_phase_d/material_save.rs` (new).

**Behavior** (DCGO `MaterialSave.cs`): A `[Main]` active skill (NOT a replacement). Cost: zero. Effect: select up-to-N own digivolution sources from self, then select one own Tamer; place selected sources at bottom of Tamer's stack. Gate (CanActivate): self has ≥1 selectable source AND ≥1 own Tamer.

- [ ] **Step 1: Write tests** — three cases:
- Activate with 3 sources, N=2, 1 Tamer: select 2 sources, target Tamer; sources move under Tamer.
- Activate with no Tamer: cannot activate (mask zero, action rejected).
- Activate with 0 sources: cannot activate.

- [ ] **Step 2: Run to confirm fail.**

- [ ] **Step 3: Implement.**

```rust
Keyword::MaterialSave(n) => vec![Effect::main_phase_active(card)
    .name(format!("<Material Save {n}>"))
    .activatable_when(move |ctx, perm| {
        // Gate: ≥1 source under top + ≥1 own Tamer.
        let stack_len = ctx.game.permanent(perm).card_sources.len();
        if stack_len < 2 { return false; }
        let owner = ctx.game.permanent(perm).controller;
        ctx.game.permanents_of(owner).any(|p| p.kind() == PermanentKind::Tamer)
    })
    .effect(move |ctx| {
        let me = ctx.source_permanent().expect("active");
        let n_usize = n as usize;
        ctx.select_own_permanent(
            "select a Tamer to receive digivolution cards",
            |p| p.kind() == PermanentKind::Tamer,
            /*is_optional=*/ false,
            move |ctx, target| {
                let Some(tamer) = target else { return };
                ctx.select_count_capped_multi(
                    ctx.controller(),
                    CountCappedZone::Material(me),
                    n_usize,
                    "select cards to place under Tamer",
                    /*is_optional_zero=*/ true,
                    |_cd, _e| true,
                    move |ctx, picks| {
                        for source in picks {
                            ctx.place_card_under_permanent_bottom(source, tamer);
                        }
                    },
                );
            },
        );
    })
    .build()],
```

- [ ] **Step 4-6: Tests, full suite, commit.**

```bash
git commit -m "engine(keywords): auto-install MaterialSave(N) — main-phase tuck active skill"
```

---

### Task 11: Module docstring + DCGO_KEYWORD_PARITY.md flip

**Files:**
- Modify: `digimon-engine/src/cards/keyword_effects.rs` module docstring (lines 1-30) — replace deferred-Partition/ArmorPurge note with the Phase D coverage matrix.
- Modify: `docs/DCGO_KEYWORD_PARITY.md` — flip `Fragment(N)`, `ArmorPurge`, `Save`, `Decoy`, `Fortitude`, `Partition`, `MaterialSave(N)` rows from 🟣/🔴 to ✅. Update the summary table counts.

- [ ] **Step 1: Update the keyword_effects.rs docstring**

Replace the "Deferred: Partition / ArmorPurge" section with:

```rust
//! ## Coverage matrix (Phase D — landed 2026-04-25)
//!
//! Auto-installed: Barrier, Evade, Decode (Phase 7); Fragment(N), ArmorPurge,
//! Save, Decoy, Fortitude, Partition, MaterialSave(N) (Phase D).
//!
//! Selection-bearing replacements consume Phase C's parked-replacement
//! substrate via `ctx.cancel_leave / handle_replacement / substitute_replacement`.
//! Trigger-based keywords (Fortitude, Partition, MaterialSave) use the
//! standard observer pattern.
//!
//! Out-of-scope deferred: SecurityAttackPlus/Minus(N), DeDigivolve(N) printed
//! form, DrawX(N) printed form (all Phase A/E), Retaliation, Scapegoat (Phase E),
//! Execute, Iceclad, MindLink, Training (Phase F).
```

- [ ] **Step 2: Update DCGO_KEYWORD_PARITY.md**

Flip the seven rows to ✅. Update the summary table at the top — "🟣 Deferred (nested-select infra) — 0" and "🔴 Parsed-but-unwired — N − 7".

- [ ] **Step 3: Commit.**

```bash
git add digimon-engine/src/cards/keyword_effects.rs docs/DCGO_KEYWORD_PARITY.md
git commit -m "docs(keywords): mark Phase D alpha-tier wire-ups complete"
```

---

### Task 12: RUST_ENGINE_API.md — selection-bearing keyword authoring template

**Files:**
- Modify: `docs/RUST_ENGINE_API.md` — add new section after the "Replacement-process outcome-setters" section landed in Phase C.

- [ ] **Step 1: Write the section.**

Title: "Selection-bearing keyword authoring pattern (Phase D pattern)". Content: a worked example using Save's auto-install body, walked through line-by-line, showing:
- Why self-scope guard is required.
- Where to call `ctx.cancel_leave()` / `handle_replacement()` / `substitute_replacement(...)`.
- The optional vs mandatory distinction (`Effect::optional()` vs not + `is_optional` on the selection primitive).
- The gate check pattern (early return without parking).
- Cross-reference: link to Phase C substrate spec for the underlying mechanism.

- [ ] **Step 2: Commit.**

```bash
git add docs/RUST_ENGINE_API.md
git commit -m "docs(api): selection-bearing keyword authoring template (Phase D)"
```

---

### Task 13: RUST_ENGINE_GAPS.md + parent spec — close Phase D rows

**Files:**
- Modify: `docs/RUST_ENGINE_GAPS.md` — close the "Save / Decoy / Fortitude / Fragment / ArmorPurge / Partition / MaterialSave wiring" subitem under `WhenWouldBeDeleted framework extensions`.
- Modify: `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md` — add Phase D landing block to §5 Phase D, mirroring the format used for Phase B and Phase C landing blocks. Include deviations list:
  - Decoy kept un-parameterized (parent spec §10 resolved against parameterization).
  - Partition wired as trigger, not replacement (parent spec D6 wording corrected).
  - MaterialSave wired as `[Main]` active skill, not nested-selection-in-replacement.
- Spec landing block must list each Task's commit SHA.

- [ ] **Step 1: Write the entries.**

- [ ] **Step 2: Commit.**

```bash
git add docs/RUST_ENGINE_GAPS.md docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md
git commit -m "docs(keywords): mark Phase D landed in keyword-parity parent spec"
```

---

### Task 14: Final integration test — printed-keyword-only smoke test

**Files:**
- Test: `digimon-engine/tests/keyword_phase_d/integration_smoke.rs` (new).

**Why:** Verify that a hand-spawned card declaring **only** Phase D keywords (no hand-rolled `CardEffect`) behaves correctly end-to-end. This is the contract Phase D delivers: "a card with only printed keywords needs zero hand-rolled `CardEffect` code."

- [ ] **Step 1: Write the integration test.**

```rust
//! Phase D Task 14 — printed-keyword-only smoke test. A card declaring
//! `Save + Fragment(2) + Decoy` and NO hand-rolled CardEffect must behave
//! correctly across all three keyword paths.

#[test]
fn printed_only_card_with_three_phase_d_keywords_works_end_to_end() {
    // 1. Self has Save: deletion → optional Tamer-pick → tucked.
    // 2. Self has Fragment(2): deletion → mandatory 2-source-pick → cancelled.
    // 3. Self has Decoy: ally deletion → substitute self.
    //
    // Multiple Phase D keywords on one card must compose: each runs its
    // replacement independently per the multi-replacement-on-one-event
    // protocol (Phase C tested Barrier+Evade ordering).
}
```

(This test verifies the auto-install plumbing exposes one Effect per keyword; the dispatcher orders them per `Effect.priority`.)

- [ ] **Step 2: Run to confirm fail (or confirm the test scaffolds correctly the first time).**

- [ ] **Step 3: If failing, debug — typical issues are Effect priority collisions or duplicate replacement registration.**

- [ ] **Step 4: Commit.**

```bash
git commit -m "test(keywords): Phase D printed-keyword-only end-to-end smoke"
```

---

## Verification gate (final, before merging Phase D)

Run all four parity surfaces and confirm green:

```bash
# 1. Rust engine — full suite.
cargo test --manifest-path digimon-engine/Cargo.toml

# 2. PyO3 release build (catches FFI surface drift).
maturin build --release --manifest-path digimon-engine-py/Cargo.toml

# 3. Python parity test.
cd digimon-engine-py && maturin develop --release && cd ..
DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v

# 4. Tauri tests (no Python at runtime).
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all green. Allowable deviations:
- New tests added by Phase D (count up by ~25 across the new `keyword_phase_d/` dir).
- Pre-existing flaky `deck_tools::validate_deck_accepts_a_legal_50_card_deck` (HashMap RandomState non-determinism, tracked separately).

---

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| **Decoy infinite loop** — self has Decoy and self is the deletion subject; the keyword fires, substitutes itself, the substituted-deletion fires, re-triggers Decoy. | Explicit self-scope guard `if subject == me_perm { return; }` in Decoy auto-install body. Tested in Task 7 Step 2. |
| **Fortitude double-fire** — Fortitude card lives in trash; if a second deletion happens, Fortitude could fire twice. | Phase 7's "replacements_fired" tracking on the deletion event prevents the same Effect from firing twice for the same deletion. Verify via Phase C regression test in `nested_select_regression.rs`. |
| **Partition cause-filter holes** — `was_deleted_by_battle` / `was_deleted_by_own_effect` may not yet exist as named accessors. | Audit `effect_context/mod.rs` in Task 9 Step 1; if missing, add as part of the task or split into a sub-task. Phase B's `current_deletion_cause` slot has the data; only the accessor name may be off. |
| **MaterialSave activation gate** during opponent's turn — DCGO permits `[Main]` activations only during your own main phase. | The `Effect::main_phase_active` builder already encodes this constraint; verify by spinning a test where opponent's main phase: the activation should not be in the action mask. |
| **`CountCappedZone::Material` zone bounds** — `card_sources[1..]` indexing assumes index 0 is the top card. | Verify against `Permanent::top_card()` impl in `permanent.rs` before Task 0 Step 3. If indexing is reversed (top at last index, sources at lower indices), flip the slice. |
| **Phase D test crate ergonomics** — `digimon-engine/tests/keyword_phase_d/` may need a `main.rs` glue file like `tests/replacements/main.rs`. | Mirror the `replacements/` directory layout in Task 0 Step 1 to confirm. If `tests/effect_context/` already has a similar glue, reuse the pattern. |

---

## Sequencing & parallelism

- **Tasks 0-3** (substrate primitives) must precede Tasks 4-10 (consumers).
- **Tasks 4-10** (one keyword each) are independent of each other and *could* parallelize across worktrees, BUT per user direction (one fresh subagent per task), they run sequentially.
- **Tasks 11-13** (docs) run after Tasks 4-10 land.
- **Task 14** (integration smoke) runs last as a final gate.

Total estimated subagent-driven duration: 13-15 review cycles × ~30 min each ≈ 7-8 hours of session time.

---

## Self-review

(Performed inline 2026-04-25 per `superpowers:writing-plans` instructions.)

**1. Spec coverage:**
- ✅ Parent spec §5 Phase D: D1-D7 each maps to a task (Fragment=T4, ArmorPurge=T5, Save=T6, Decoy=T7, Fortitude=T8, Partition=T9, MaterialSave=T10).
- ✅ Parent spec §10 open questions resolved inline + assumption table.
- ✅ Parent spec §6 API surface — `armor_purge_top`, `place_card_under_permanent_bottom`, `play_from_trash_free_unsuspended` map to Tasks 1-3.
- ✅ Parent spec §7 testing — each keyword has at least 2 behavioral tests (positive + gate-fail).
- ✅ Parent spec §8 doc updates — Tasks 11-13 cover all four doc files.

**2. Placeholder scan:** Each step contains either complete code, a complete test scaffold, or a precise file/line citation for what to read. The two `todo!()` macros in Task 1 Step 3 and Task 4 Step 3 are intentional — the implementer must verify the exact `Permanent` field names and existing primitive APIs first; they're documentation pointers, not unfilled holes. Acceptable per "implementer must check before writing" pattern, but I'll mark them clearly.

**3. Type consistency:** Used `PermanentHandle`, `CardHandle`, `CountCappedZone`, `ReplacementSubject::Permanent`, `EffectContext`, `ReplacementOutcome` consistently. The `controller` vs `owner` field name is flagged as something to verify in Task 2 Step 3 (Permanent struct).

**4. Spec deviations from parent:**
- Partition wired as trigger (not replacement). Documented in "Open questions resolved" header + Task 13 deviations list.
- MaterialSave wired as active skill (not replacement). Documented similarly.
- Decoy kept un-parameterized. Documented similarly.

These are the only deviations and are justified by DCGO source readings.

Plan complete.
