# Rust Engine Phase 4 — Selection-Kind Expansion

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand `SelectionKind` + `EffectContext` selection helpers so card effects can faithfully surface ordered permutations, union-zone picks, count-capped multi-selects, and opponent-as-selector flows — all routed through `PendingSelection` so the RL action space observes every branch.

**Architecture:** Add three new `SelectionKind` variants (`UnionZone`, `OrderedPermutation`, `CountCappedMultiSelect`) with matching `GamePhase` variants, new `EffectContext` helpers, and action-mask/decoder glue. `selecting_player` is already on `PendingSelection` (selection.rs:75) — opponent-as-selector is a matter of overriding that field at install time. No breaking changes to `PendingSelection`; all additions are additive.

**Tech Stack:** Rust 2021 (`digimon-engine/`), DebugRunner test harness, existing `select_*` closure/callback pattern in `effect_context/selections.rs`.

---

## Background

Phase 4 closes Cluster D from [.claude/plans/recursive-coalescing-candle.md](.claude/plans/recursive-coalescing-candle.md):

- **Ordered permutation** (~29 cards across 5 archetypes): "place these N cards on top of deck in any order"
- **Union-zone selection** (~20 cards): "choose a card from your hand OR trash"
- **Count-capped multi-select** (common primitive): "up to N" selections with a confirm/pass sentinel
- **Opponent-as-selector** (~58 cards): effect controller A, but player B chooses the target/branch

**Existing infrastructure already in place (from the selection survey):**
- `PendingSelection.selecting_player` already exists ([digimon-engine/src/selection.rs:75](digimon-engine/src/selection.rs:75))
- Mask layer already gates actions by `selecting_player` ([digimon-engine/src/action/mask.rs](digimon-engine/src/action/mask.rs:255))
- Selection resolution already rejects wrong-player actions ([digimon-engine/src/game.rs](digimon-engine/src/game.rs))
- Action space has headroom past index 2100 ([digimon-engine/src/action/space.rs](digimon-engine/src/action/space.rs))

**API design principles (from the roadmap, carry-forward):**
1. No auto-selection. Every legal branch in `valid_action_ids`.
2. Closures over flags for runtime predicates.
3. Callbacks receive scoped `EffectContext` with preserved `source_card`/`source_permanent`.
4. One concept, one primitive.
5. TDD per working rule 18 — failing test first.

**Python cross-reference (informational):**
Surveyed 2026-04-20. Relevant patterns to learn from / diverge from:
- **Union-zone** — Python's `effect_play_from_zone(player, 'hand_or_trash', ...)` (`digimon_gym/engine/game/effects.py:327-457`) populates both `SEL_HAND_START` (0-29) and `SEL_TRASH_START` (130-179) into `valid_indices` on a single `PendingSelection`; the decoder disambiguates by action-ID range. **Rust follows this pattern** — reuse existing `HAND_EFFECT`/`TRASH` ranges rather than adding a new `UNION_ZONE` range.
- **Opponent-as-selector** — Python **does not support** this. No script calls `request_selection(..., selecting_player=opponent, ...)`. Rust is ahead here: `PendingSelection.selecting_player` already exists and the mask layer already routes on it. Task 5 is net-new design with no Python precedent to copy or be constrained by.
- **Ordered permutation** — Python has no direct analog; closest is `effect_reveal_and_select_multi()` (`effects.py:156-216`), a sequential multi-pass over a reveal pool. Task 3's sequential re-install pattern is the correct Rust analog.
- **Count-capped multi** — Python has no clean primitive; BT10-081 (Baalmon) and similar scripts **auto-mill N cards without offering a selection**, violating the no-approximations policy. Rust must NOT copy this — Task 4 mandates explicit per-pick selections.
- **`on_decline` + optional selections** — Python's `PendingSelection.on_decline` callback already exists in Rust; no new work needed.

---

## File Structure

**Modified:**
- `digimon-engine/src/selection.rs` — add 3 `SelectionKind` variants; extend `PendingSelectionView` (fields additive); no other struct change
- `digimon-engine/src/enums.rs` — add 3 `GamePhase` variants: `SelectUnion`, `SelectPermutation`, `SelectBudgeted`
- `digimon-engine/src/effect_context/selections.rs` — add `select_union_zone`, `select_ordered_permutation`, `select_count_capped_multi`; add `select_*_by_player` overload(s) routing to opponent-as-selector
- `digimon-engine/src/action/space.rs` — add decoder helpers (`decode_union_zone`, `decode_permutation_step`, `decode_multi_toggle`) + ranges
- `digimon-engine/src/action/mask.rs` — mask build for three new phases
- `digimon-engine/src/game.rs` — `resolve_selection` dispatch for three new kinds
- `docs/RUST_ENGINE_API.md` — §Phase 4 Selection Kinds
- `docs/RUST_ENGINE_GAPS.md` — annotate closed Cluster D entries
- `.claude/plans/recursive-coalescing-candle.md` — mark Phase 4 ✅ LANDED

**New tests:**
- `digimon-engine/tests/selection/union_zone.rs`
- `digimon-engine/tests/selection/ordered_permutation.rs`
- `digimon-engine/tests/selection/count_capped.rs`
- `digimon-engine/tests/selection/opponent_selector.rs`
- Register modules in `digimon-engine/tests/selection/main.rs`

---

## Tasks

### Task 1: Add SelectionKind + GamePhase variants

**Files:**
- Modify: `digimon-engine/src/enums.rs` — add `GamePhase::SelectUnion`, `GamePhase::SelectPermutation`, `GamePhase::SelectBudgeted`
- Modify: `digimon-engine/src/selection.rs` — add `SelectionKind::UnionZone { zones: UnionZoneSet }`, `SelectionKind::OrderedPermutation { remaining: u8 }`, `SelectionKind::CountCappedMultiSelect { max: u8, picked: u8 }`
- Modify: `digimon-engine/src/selection.rs` — add supporting types `UnionZoneSet` (bitflags: Hand | Trash | others later), mirror new kinds in `PendingSelectionView`

- [ ] **Step 1: Write the failing test**

Create `digimon-engine/tests/selection/kinds_exist.rs`:
```rust
use digimon_engine::selection::{SelectionKind, UnionZoneSet};
use digimon_engine::enums::GamePhase;

#[test]
fn new_selection_kinds_exist() {
    let _ = SelectionKind::UnionZone { zones: UnionZoneSet::HAND | UnionZoneSet::TRASH };
    let _ = SelectionKind::OrderedPermutation { remaining: 3 };
    let _ = SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 };
    let _ = GamePhase::SelectUnion;
    let _ = GamePhase::SelectPermutation;
    let _ = GamePhase::SelectBudgeted;
}
```

Add `mod kinds_exist;` to `digimon-engine/tests/selection/main.rs`.

- [ ] **Step 2: Run test — expect compile errors for missing variants**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test selection kinds_exist`
Expected: compile failure referencing the missing variants.

- [ ] **Step 3: Implement variants + supporting types**

In `enums.rs`, append to `GamePhase` enum (before closing brace):
```rust
SelectUnion,
SelectPermutation,
SelectBudgeted,
```

In `selection.rs`, add `UnionZoneSet` as a bitflags struct near the top of the file:
```rust
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    pub struct UnionZoneSet: u8 {
        const HAND = 0b0001;
        const TRASH = 0b0010;
    }
}
```
(If `bitflags` isn't already a dep, use a plain `pub struct UnionZoneSet(pub u8)` with `HAND`/`TRASH` associated consts — match whatever pattern the crate already prefers. Check `digimon-engine/Cargo.toml` first.)

Extend `SelectionKind`:
```rust
UnionZone { zones: UnionZoneSet },
OrderedPermutation { remaining: u8 },
CountCappedMultiSelect { max: u8, picked: u8 },
```

Mirror in `PendingSelectionView` so serialization stays complete.

- [ ] **Step 4: Run test — expect PASS**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test selection kinds_exist`
Expected: PASS.

- [ ] **Step 5: Full suite green**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: all tests pass (existing + the new one).

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/enums.rs digimon-engine/src/selection.rs digimon-engine/tests/selection/kinds_exist.rs digimon-engine/tests/selection/main.rs
git commit -m "rust-engine(phase-4): add UnionZone/OrderedPermutation/CountCappedMultiSelect SelectionKind + GamePhase variants"
```

---

### Task 2: `select_union_zone` helper + dispatch

**Encoding approach (Python-parity):** The union-zone selection reuses the **existing** `HAND_EFFECT` and `TRASH` action ranges in `digimon-engine/src/action/space.rs` rather than introducing a new `UNION_ZONE_START` range. A single `PendingSelection` with `kind = UnionZone { zones }` populates `valid_action_ids` with hand indices (from the `HAND_EFFECT` range) and/or trash indices (from the `TRASH` range) per the `zones` bitset. The dispatcher in `game.rs` classifies an incoming action by which range it lies in and resolves the `CardHandle` from the matching zone. This matches Python's `effect_play_from_zone(..., 'hand_or_trash', ...)` pattern and avoids a third decoder range and extra mask plumbing.

**Files:**
- Modify: `digimon-engine/src/effect_context/selections.rs` — `select_union_zone<F, C>(player, zones, prompt, is_optional, filter, callback)`
- Modify: `digimon-engine/src/action/mask.rs` — when phase is `SelectUnion`, gate by `selecting_player` and emit 1.0 for every entry in `pending_selection.valid_action_ids` (same generic-selection pattern used by `SelectHand`/`SelectTrash`); PASS if `is_optional`
- Modify: `digimon-engine/src/game.rs` — `resolve_selection` for `UnionZone` classifies the incoming action by range: if it falls in `HAND_EFFECT`, decode as hand index; if in `TRASH`, decode as trash index; resolve to `CardHandle`; invoke callback
- Create: `digimon-engine/tests/selection/union_zone.rs`
- Register in: `digimon-engine/tests/selection/main.rs`

**No changes to `digimon-engine/src/action/space.rs`** — we do not add a new `UNION_ZONE_START` range. Existing `HAND_EFFECT` and `TRASH` ranges + existing `decode_*` helpers are sufficient.

- [ ] **Step 1: Write the failing test**

```rust
// tests/selection/union_zone.rs
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::selection::{SelectionKind, UnionZoneSet};
use digimon_engine::enums::GamePhase;

#[test]
fn select_union_zone_surfaces_hand_and_trash_actions() {
    // 1. Build a game with player 0 holding 2 cards in hand + 2 cards in trash
    // 2. Install a union-zone selection (Hand|Trash) with filter = all-true
    // 3. Assert pending_selection.kind == SelectionKind::UnionZone { zones: HAND|TRASH }
    // 4. Assert GamePhase == SelectUnion
    // 5. Assert valid_action_ids.len() == 4 (2 hand + 2 trash)
    // 6. Resolve with a trash-pointing action; assert callback received CardHandle pointing to trash card
    // ... use existing test patterns from material.rs as a template
    todo!("implement after helper lands")
}
```

Run test → expect compile failure on `select_union_zone`.

- [ ] **Step 2: Implement `select_union_zone` helper**

Pattern after `select_hand` and `select_trash` in `effect_context/selections.rs`. For each zone in the `zones` bitset: iterate the matching player zone, apply the filter, append the filter-passing `HAND_EFFECT_START + i` (hand) or `TRASH_START + i` (trash) action IDs into `valid_action_ids`. Install a single `PendingSelection { kind: UnionZone { zones }, phase: SelectUnion, valid_action_ids, ... }`.

Signature:
```rust
pub fn select_union_zone<F, C>(
    &mut self,
    of_player: PlayerId,
    zones: UnionZoneSet,
    prompt: &str,
    is_optional: bool,
    filter: F,
    callback: C,
)
where
    F: Fn(&Game, &CardSource) -> bool + Send + Sync + 'static,
    C: FnOnce(&mut EffectContext<'_>, CardHandle) + Send + Sync + 'static,
{ ... }
```

Callback receives `CardHandle` (zone-agnostic) so call-sites don't branch on source.

- [ ] **Step 3: Wire mask build in `action/mask.rs`**

Under `GamePhase::SelectUnion`, gate by `selecting_player`, set mask[action] = 1.0 for every entry in `pending_selection.valid_action_ids`. PASS stays gated by `is_optional`. This follows the exact same shape as `SelectHand` / `SelectTrash` masks — no new range logic needed.

- [ ] **Step 4: Wire `resolve_selection` dispatch in `game.rs`**

For `SelectionKind::UnionZone`, classify the incoming `action_id`:
- If `action_id` is in `[HAND_EFFECT_START, HAND_EFFECT_END)` → decode as hand index, resolve `CardHandle` via `player.hand[idx]`
- If `action_id` is in `[TRASH_START, TRASH_END)` → decode as trash index, resolve `CardHandle` via `player.trash[idx]`
- Otherwise → validation failure (should not happen; mask rejected it)

Invoke `callback(CardHandle)`. Mirror the resolution pattern already in place for `SelectionKind::Hand` and `SelectionKind::Trash`.

- [ ] **Step 5: Fill in the test body**

Replace the `todo!()` with a real scenario following the `material.rs` template (lines 57-80 there). Assert:
- `pending_selection.kind` matches `UnionZone`
- `pending_selection.selecting_player == 0`
- `valid_action_ids.len() == 4`
- `valid_action_ids` contains 2 entries in the `HAND_EFFECT` range + 2 in the `TRASH` range
- Resolving with a trash action delivers the correct `CardHandle`

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test selection union_zone`
Expected: PASS.

- [ ] **Step 6: Full suite green**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add digimon-engine/src/effect_context/selections.rs digimon-engine/src/action/mask.rs digimon-engine/src/game.rs digimon-engine/tests/selection/union_zone.rs digimon-engine/tests/selection/main.rs
git commit -m "rust-engine(phase-4): add select_union_zone helper (reuses HAND_EFFECT + TRASH ranges)"
```

---

### Task 3: `select_ordered_permutation` helper

**Semantics:** Player places N items in chosen order. Implementation is sequential: a single-step selection is installed per slot, and after the player picks slot k, the helper re-installs for slot k+1 with the picked item filtered out. After N picks the final callback fires with `Vec<CardHandle>` in chosen order.

**Encoding approach (range-reuse, consistent with Task 2):** Each permutation step uses `valid_action_ids = [i for i in 0..remaining.len()]` in the **existing** `SEL_REVEAL_START` (or `HAND_EFFECT_START`, whichever has ≥10 slots) action range — no new `PERMUTATION_START` range is added. Semantic disambiguation is handled by `PendingSelection.kind == OrderedPermutation { remaining }` + `GamePhase::SelectPermutation`; the resolver uses the kind to know that `action - RANGE_START` is an index into the *remaining* list (not into a player zone). N_max is capped at 10 (card text never exceeds this).

**Files:**
- Modify: `digimon-engine/src/effect_context/selections.rs` — `select_ordered_permutation<C>(items: Vec<CardHandle>, prompt, callback)` where `C: FnOnce(&mut EffectContext<'_>, Vec<CardHandle>) + ...`
- Modify: `digimon-engine/src/game.rs` (or `effect_queue.rs`) — dispatch only if a kind-specific arm is required; the generic selection path from Task 2 should already work since each step is just an index into `valid_action_ids`
- Create: `digimon-engine/tests/selection/ordered_permutation.rs`

**No changes to `digimon-engine/src/action/space.rs`** (no new range). **No changes to `digimon-engine/src/action/mask.rs`** (generic `is_selection_phase()` path covers `SelectPermutation` from Task 1).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn ordered_permutation_collects_picks_in_order() {
    // Install select_ordered_permutation over [card_a, card_b, card_c]
    // Phase 1: pick card_b (second position 1); remaining = [a, c]; selection reinstalls
    // Phase 2: pick card_c; remaining = [a]; selection reinstalls
    // Phase 3: pick card_a; callback fires with Vec [card_b, card_c, card_a]
    // Assert final callback's delivered Vec equals that order
    todo!()
}
```

- [ ] **Step 2: Implement `select_ordered_permutation`**

Internal state is held in the callback closure (each re-install captures the accumulator). Sketch:
```rust
pub fn select_ordered_permutation<C>(
    &mut self,
    items: Vec<CardHandle>,
    prompt: &str,
    callback: C,
) where C: FnOnce(&mut EffectContext<'_>, Vec<CardHandle>) + Send + Sync + 'static {
    fn step(ctx: &mut EffectContext<'_>, remaining: Vec<CardHandle>, accum: Vec<CardHandle>, prompt: String, final_cb: Box<dyn FnOnce(&mut EffectContext<'_>, Vec<CardHandle>) + Send + Sync>) {
        if remaining.is_empty() { final_cb(ctx, accum); return; }
        // install PendingSelection for SelectPermutation over remaining
        // on pick(action) → idx into remaining → move to accum → re-enter step()
    }
    step(self, items, Vec::new(), prompt.to_string(), Box::new(callback));
}
```

(Exact impl shape subject to existing closure/FnOnce patterns — mirror whatever `install_field_selection` does re: boxing + moving.)

- [ ] **Step 3: Decoder + mask + dispatch**

Follow the same pattern as Task 2. Decoder is simpler: `decode_permutation_step(action) -> u16` (index into remaining).

- [ ] **Step 4: Fill in test body; run, commit**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test selection ordered_permutation`
Expected: PASS.

```bash
git add digimon-engine/src/effect_context/selections.rs digimon-engine/src/action/space.rs digimon-engine/src/action/mask.rs digimon-engine/src/game.rs digimon-engine/tests/selection/ordered_permutation.rs digimon-engine/tests/selection/main.rs
git commit -m "rust-engine(phase-4): add select_ordered_permutation (sequential pick-by-pick)"
```

---

### Task 4: `select_count_capped_multi` helper

**Semantics:** Player picks *up to* N items from a given zone. Each pick is an action in the existing per-zone action range. A `Commit` sentinel (the existing `PASS` action id = 62) stops early; reaching `picked == max` auto-commits. Final callback fires with `Vec<CardHandle>` of picked items (in pick order).

**Encoding approach (range-reuse, consistent with Tasks 2 + 3):**
- Toggle actions reuse the existing zone range: hand picks use `PLAY_HAND_START + i`; trash picks use `TRASH_EFFECT_START + i` — whichever the `zone` argument selects. (Extensible to reveal/security later via the same pattern.)
- `PASS` (action 62) is the early-commit sentinel. Availability gated by `is_optional_zero || picked >= 1`.
- Reaching `picked == max` auto-commits immediately (no extra action needed) — this is NOT an auto-selection of a card, it's the terminator of a confirmed full-stack selection. Still surfaces the final pick as a normal selection action to RL; just doesn't require a separate commit after it.
- No new action range added. No new mask match arm needed — the generic `is_selection_phase()` path from Task 1 covers `SelectBudgeted`.

**Files:**
- Modify: `digimon-engine/src/effect_context/selections.rs` — `select_count_capped_multi<F, C>(player, zone, max, prompt, is_optional_zero, filter, callback)`
- Create: `digimon-engine/tests/selection/count_capped.rs`
- Register in: `digimon-engine/tests/selection/main.rs`

**No changes to `digimon-engine/src/action/space.rs`, `action/mask.rs`, `game.rs`, or `effect_queue.rs`.** All dispatch lives in the boxed step callback, same as Tasks 2 + 3.

**Signature:**
```rust
pub enum CountCappedZone { Hand, Trash }

pub fn select_count_capped_multi<F, C>(
    &mut self,
    of_player: PlayerId,
    zone: CountCappedZone,
    max: u8,
    prompt: &str,
    is_optional_zero: bool,
    filter: F,
    callback: C,
)
where
    F: Fn(&Game, &CardSource) -> bool + Send + Sync + 'static,
    C: FnOnce(&mut EffectContext<'_>, Vec<CardHandle>) + Send + Sync + 'static
```

- [ ] **Step 1: Failing tests**

Create `digimon-engine/tests/selection/count_capped.rs`. Required tests:
1. `auto_commits_at_max` — 3 hand cards, max=2, is_optional_zero=false. Pick card 0 (remaining shows picked=1), pick card 2 (remaining picked=2 == max → auto-commit). Assert callback delivered `[card_0, card_2]`.
2. `pass_commits_early_when_picked_ge_1` — 3 cards, max=3, is_optional_zero=false. Pick card 1, then PASS → callback delivered `[card_1]`.
3. `pass_rejected_when_picked_zero_and_not_optional` — 3 cards, max=2, is_optional_zero=false. Assert PASS not in `valid_action_ids` at step 0. Resolve PASS → should be rejected (mask-layer should not have allowed it; assert error return from resolve, or assert phase unchanged).
4. `optional_zero_allows_pass_at_start` — 3 cards, max=2, is_optional_zero=true. Assert PASS is in `valid_action_ids` at step 0. Resolve PASS → callback delivered empty Vec.
5. `picked_items_excluded_from_next_step` — 3 cards, max=3. Pick card 0 → next step's valid_action_ids excludes card 0's index.
6. `kind_reflects_picked_counter` — each step, `kind == CountCappedMultiSelect { max: 2, picked: N }` where N increments.

- [ ] **Step 2: Implement helper**

Pattern mirrors Task 3's trampoline (free function `install_count_capped_step`). Each step's boxed callback:
1. `debug_assert!` on action_id (either `== PASS` OR in `[ZONE_RANGE_START, ZONE_RANGE_START + zone_len)`)
2. If `action_id == PASS`: invoke final callback with `accum` (empty if picked==0 and is_optional_zero).
3. Else: `pick_idx = action_id - RANGE_START`; resolve `CardHandle` from zone; append to `accum`; track the picked indices so step re-install can exclude them.
4. If `accum.len() == max`: invoke final callback with `accum`.
5. Else: re-install via `install_count_capped_step(..., picked_indices_so_far, ...)` with updated `valid_action_ids` and `kind: CountCappedMultiSelect { max, picked: accum.len() as u8 }`. PASS included in valid_action_ids if `is_optional_zero || accum.len() >= 1`.

Cap `max` at a reasonable upper bound (e.g. `debug_assert!(max <= 10)`).

- [ ] **Step 3: Empty-items / zero-valid edge case**

If at install time no cards pass the filter:
- If `is_optional_zero == true`: invoke final callback with empty Vec immediately; no PendingSelection installed.
- If `is_optional_zero == false`: same behavior (nothing is strictly required because there are no valid items) but emit a debug log? For now, treat as empty-callback either way. Document in doc comment.

- [ ] **Step 4: Full suite + commit**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: 480 (after Task 3 fix d35d93f6) + ≥6 new tests = 486+.

```bash
git add digimon-engine/src/effect_context/selections.rs digimon-engine/tests/selection/count_capped.rs digimon-engine/tests/selection/main.rs
git commit -m "rust-engine(phase-4): add select_count_capped_multi (PASS commits early, auto-commits at max, reuses existing ranges)"
```

---

### Task 5: Opponent-as-selector helpers

**Rationale:** `selecting_player` already exists on `PendingSelection` (selection.rs:75) and the mask layer already routes on it. The only missing piece is that current `install_field_selection` / `select_*` helpers hardcode `selecting_player = self.player`. Phase 4 exposes an **opt-in player override** so effect authors can install a selection where the opponent chooses.

**API choice:** rather than duplicate every helper, add a single **builder-style** wrapper:

```rust
impl<'g> EffectContext<'g> {
    pub fn as_selecting_player(&mut self, player: PlayerId) -> EffectContextSelectorScope<'_, 'g> {
        EffectContextSelectorScope { ctx: self, selecting_player: player }
    }
}

pub struct EffectContextSelectorScope<'a, 'g> {
    ctx: &'a mut EffectContext<'g>,
    selecting_player: PlayerId,
}

impl EffectContextSelectorScope<'_, '_> {
    pub fn select_own_permanent<...>(...) { /* forward, overriding selecting_player */ }
    pub fn select_opponent_permanent<...>(...) { /* ... */ }
    pub fn select_effect_choice<...>(...) { /* ... */ }
    // + select_hand, select_trash, select_union_zone, select_count_capped_multi
}
```

Call site:
```rust
// "your opponent chooses one of your Digimon and trashes it"
ctx.as_selecting_player(opponent).select_own_permanent(
    "Opponent: choose a Digimon to trash",
    false,
    |_g, _perm| true,
    |ctx, handle| { ctx.delete_permanent(handle); },
);
```

**Files:**
- Modify: `digimon-engine/src/effect_context/selections.rs` — add `EffectContextSelectorScope` + forward methods
- Modify: `digimon-engine/src/effect_context/selections.rs` — refactor `install_field_selection` and each `select_*` to accept an optional `selecting_player` override (default = `self.player`)
- Create: `digimon-engine/tests/selection/opponent_selector.rs`

- [ ] **Step 1: Failing test**

```rust
#[test]
fn opponent_as_selector_routes_actions_to_opponent() {
    // Build a game where player 0 is installing an effect that demands
    // player 1 picks one of player 0's battle-area Digimon.
    // Assert pending_selection.selecting_player == 1
    // Assert player-0 mask for this selection phase is all-zero
    // Assert player-1 mask has the expected indices set to 1.0
    // Resolve from player 1; assert callback fires with the expected PermanentHandle
    todo!()
}
```

- [ ] **Step 2: Implement `as_selecting_player` + refactor install path**

Thread a `selecting_player: PlayerId` arg through `install_field_selection` and each `select_*`. Default remains `self.player` so existing call sites are untouched.

- [ ] **Step 3: Fill body, run, commit**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test selection opponent_selector`
Expected: PASS. Also run full suite; no existing test should regress.

```bash
git add digimon-engine/src/effect_context/selections.rs digimon-engine/tests/selection/opponent_selector.rs digimon-engine/tests/selection/main.rs
git commit -m "rust-engine(phase-4): add as_selecting_player builder for opponent-as-selector flows"
```

---

### Task 6: End-to-end behavioral test

Realistic card-style scenario tying 2+ new selection kinds together. Purpose: catch integration regressions the unit tests can miss.

**Files:**
- Create: `digimon-engine/tests/selection/behavioral_end_to_end.rs`
- Register in: `digimon-engine/tests/selection/main.rs`

- [ ] **Step 1: Write the failing test**

Scenario (no real card — synthesized):
> Effect: "Reveal top 3 cards of your deck. Place them on top of your deck in any order. Then your opponent chooses one card from your hand OR trash to trash."

This exercises `select_ordered_permutation` + `as_selecting_player` + `select_union_zone` in sequence. Assertions:
- After reveal + install permutation, `pending_selection.kind` = `OrderedPermutation { remaining: 3 }`
- Walk through 3 picks; assert deck top order matches chosen order
- After final pick, a new `PendingSelection` is installed with `selecting_player = opponent`, `kind = UnionZone { zones: HAND | TRASH }`
- Opponent picks trash card; assert that card is trashed and final game state is consistent

- [ ] **Step 2: Run — PASS**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test selection behavioral_end_to_end`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/selection/behavioral_end_to_end.rs digimon-engine/tests/selection/main.rs
git commit -m "rust-engine(phase-4): end-to-end test combining permutation + opponent-selector + union-zone"
```

---

### Task 7: Docs + roadmap update

**Files:**
- Modify: `docs/RUST_ENGINE_API.md` — add §Phase 4 Selection Kinds section documenting the three new helpers + `as_selecting_player`, with a worked example per helper
- Modify: `docs/RUST_ENGINE_GAPS.md` — annotate Cluster D entries as closed (ordered permutation, opponent-as-selector, union-zone; keep any residual sub-items that Phase 4 didn't cover)
- Modify: `.claude/plans/recursive-coalescing-candle.md` — mark Phase 4 as ✅ LANDED with commit range, bump the cumulative readiness table row

- [ ] **Step 1: Draft API doc entries**

For each of `select_union_zone`, `select_ordered_permutation`, `select_count_capped_multi`, and `as_selecting_player`:
- Signature
- One-paragraph semantics
- A TDD worked example (10-20 lines, showing the test pattern)

- [ ] **Step 2: Annotate closed gap entries**

Find each Cluster D gap in `docs/RUST_ENGINE_GAPS.md` and append:
```
**Status (2026-04-20):** Closed by Phase 4 — `select_*` helper in `digimon-engine/src/effect_context/selections.rs`, exposed as SelectionKind::<Variant>.
```

- [ ] **Step 3: Roadmap update**

In `.claude/plans/recursive-coalescing-candle.md`, flip the Phase 4 row in the cumulative table to ✅ LANDED with today's date and commit range, and update the Immediate Next Steps section.

- [ ] **Step 4: Commit**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md .claude/plans/recursive-coalescing-candle.md
git commit -m "docs(phase-4): RUST_ENGINE_API/GAPS + roadmap — Phase 4 selection kinds landed"
```

---

## Verification

After all tasks land:

1. `cargo test --manifest-path digimon-engine/Cargo.toml` — full suite green, ≥ +8 new tests beyond Phase 3's 463
2. Grep for TODO/unimplemented in `digimon-engine/src/effect_context/selections.rs` — none introduced
3. `docs/RUST_ENGINE_API.md` has a §Phase 4 entry with three helper sections
4. `docs/RUST_ENGINE_GAPS.md` shows Cluster D entries annotated closed
5. `.claude/plans/recursive-coalescing-candle.md` table row for Phase 4 is ✅ LANDED

## Non-Goals (deferred)

- **Cross-permanent source selection** across two distinct permanents (e.g. "swap sources between permanent A and permanent B"). Listed in Cluster D, but no audited card exercises this flow strictly enough to force the abstraction. Defer until a real card demands it.
- **Cross-player multi-target selection** (e.g. "choose one of your Digimon *and* one of your opponent's"). The `as_selecting_player` scope + sequential selection handles this via two back-to-back installs; no new primitive needed yet.
- **Multi-select with min > 0.** The current `select_count_capped_multi` exposes `max` only; confirm is valid once `picked >= 1` when `is_optional_zero = false`, or always when `is_optional_zero = true`. Richer min-bound semantics are YAGNI for now.
