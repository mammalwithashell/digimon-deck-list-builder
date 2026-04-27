# Rust Engine: User-Action DNA Digivolve Execution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve `TODO(dna-digivolve-execute)` in `Game::initiate_dna_digivolve`, extract a shared `Game::dna_digivolve_inner` used by both the user-action and engine-effect paths, and wire `OnDnaDigivolve` trigger firing in both paths.

**Architecture:** The user-action entry point ([game_actions.rs:2111](code/digimon-engine/src/game_actions.rs:2111)) currently installs a `SelectMaterial` pending selection but its callback is stubbed. We resolve this by (a) extracting the merge + payment + trigger-firing core out of `EffectContext::effect_initiated_dna_digivolve` into `Game::dna_digivolve_inner`, (b) wiring a two-stage selection chain in the user-action callback that resolves to that helper, and (c) firing `OnDnaDigivolve` (alongside the existing `WhenDigivolving` + `OnDigivolve`) inside the helper so both paths converge on identical trigger surface. Material-stack ordering is locked to the existing effect-initiated convention (`target_a.sources + target_b.sources + [from_hand]`), dropping the "CHOSEN-NOT-CANONICAL" marker.

**Tech Stack:** Rust (`code/digimon-engine/`), Cargo workspace tests, DSL example YAML + integration test runners.

**Reference precedent:**
- **Rules §8-2** ([RULES_CONTEXT.md:245-254](docs/RULES_CONTEXT.md)) — DNA digivolution is a single new card with both materials as digi-cards. Per §8-2-2-1-6 the result *can attack same turn* (no summoning sickness). Per §15-16-3, `[When Digivolving]` fires for DNA digivolve.
- **DCGO** — `JogressEffectObject` + `DNADigivolveEffects.cs` route both user-initiated and effect-initiated DNA digivolves through one `PlayCardClass(payCost: bool)` entry point. The `payCost` flag is the analogue of our `ignore_requirements`. Materials are presented as an *ordered* `JogressEvoRootsFrameIDs[2]` where `[0]` matches `JogressCondition.elements[0]` (= `requirement1`) and `[1]` matches `elements[1]` (= `requirement2`).
- **Python (sunset)** — `player.dna_digivolve` ([code/engine_py_legacy/engine/core/player.py:286](code/engine_py_legacy/engine/core/player.py)) and `_dna_select_second` ([code/engine_py_legacy/engine/game/effects.py:826](code/engine_py_legacy/engine/game/effects.py)) chain two `request_selection` calls capturing `hand_idx` and `first_field_idx` via closure.

---

## File Structure

**Create:**
- `code/digimon-engine/tests/dna_digivolve_user_action.rs` — Integration tests for the user-action path (driven via `DebugRunner` + `Game::initiate_dna_digivolve` + `Game::resolve_selection`).
- `code/digimon-engine/cards/_examples/TST_DNA_TRIGGER.yaml` — DSL card authoring an `<OnDnaDigivolve>` clause to demonstrate the new firing surface.
- `code/digimon-engine/tests/dsl/phase2g_on_dna_digivolve.rs` — DSL integration test asserting `OnDnaDigivolve` fires from both paths via the example card.

**Modify:**
- `code/digimon-engine/src/game.rs` — Add `Game::dna_digivolve_inner` co-located with other digivolve helpers (`can_digivolve` lives there at line 1037; place the new helper nearby).
- `code/digimon-engine/src/dna_digivolve.rs` — Add `get_dna_stacking_order` and `get_valid_dna_second_targets` helpers (port of `digivolve_validator.py:223-301`).
- `code/digimon-engine/src/game_actions.rs:2197-2207` — Replace stubbed callback with real first-stage callback chaining into a second-stage selection.
- `code/digimon-engine/src/effect_context/mod.rs:1895-2091` — Refactor `effect_initiated_dna_digivolve` to delegate to `Game::dna_digivolve_inner`; update the doc-comment to drop the "OnDnaDigivolve not yet wired" caveat (lines ~1934-1937) and the "no shared `Game::dna_digivolve_from_hand_inner`" caveat (lines ~1903-1907).
- `code/digimon-engine/src/effect.rs:253` — Add `Effect::on_dna_digivolve` builder mirroring `on_digivolve`.
- `code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs:~193` — Drop "CHOSEN-NOT-CANONICAL" marker; assert the now-canonical order plainly.
- `code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs:1-16` — Update file doc-comment (no longer "performs the merge inline").
- `docs/RUST_PYTHON_PARITY.md` — Add §4.5e entry tracking execution-side parity (separate from §4.5a mask).
- `docs/RUST_ENGINE_GAPS.md` — If `OnDnaDigivolve` is listed as a gap, mark it resolved.

**Out of scope (per deferred-task ticket):** sub-module decomposition of `effect_context/mod.rs`; broader refactor of the `pay_memory_unchecked` branch (lines 2007-2031). Address only if it falls out of Task 4 naturally.

---

## Open Decisions Locked Up Front

These are the ambiguous calls; locking them now prevents Task drift.

1. **Material-stack ordering (canonical):** `target_a.card_sources ++ target_b.card_sources ++ [from_hand]`. `target_a` matches `DnaCost::requirement1`, `target_b` matches `requirement2`. This matches the existing `effect_initiated_dna_digivolve` test assertion ([effect_initiated_dna_digivolve.rs:~193](code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs)) and DCGO's `orderedRoots` semantics ([DNADigivolveEffects.cs:421](DCGO/Assets/Scripts/Script/CardEffectCommons/DNADigivolveEffects.cs)). Diverges from Python's `bottom + top + [evo]` order, but Python is sunset and the rules manual does not specify intra-stack ordering between materials. The "CHOSEN-NOT-CANONICAL" marker is dropped.

2. **Trigger surface (both paths, in order):** `WhenDigivolving` on the merged permanent → `drain_effect_queue` → `OnDigivolve` on every player's battle area → `drain_effect_queue` → `OnDnaDigivolve` on the merged permanent → `drain_effect_queue`. `OnDnaDigivolve` fires *after* the broader `OnDigivolve` observer wave; this matches the principle that more-specific timings refine the broader timing rather than replacing it.

3. **Digivolution-bonus draw:** User-action path draws 1 (matching `digivolve_from_hand` line 1747 and Python `player.dna_digivolve` line 333). Effect-initiated path does **not** draw (preserving current `effect_initiated_digivolve` behavior at game_actions.rs:2416). Per-path divergence is preserved via a `grant_digivolve_bonus: bool` parameter on `dna_digivolve_inner`. The broader question of whether effect-initiated digivolves should grant the bonus is out of scope.

4. **`ignore_requirements` semantics:** Unchanged. `cost` is what's actually subtracted from memory; `ignore_requirements: true` skips the affordability floor and skips legality validation (color/level match), but still subtracts `cost`. The pay-memory-bypass branch ([effect_context/mod.rs:2015-2028](code/digimon-engine/src/effect_context/mod.rs)) stays in `EffectContext::effect_initiated_dna_digivolve` for now (kept *before* delegating to the inner helper). The user-action path always pays via `Game::pay_memory` — `ignore_requirements` is not exposed to users.

5. **Phase enforcement:** User-action entry point requires `GamePhase::Main` (already enforced at [game_actions.rs:2116](code/digimon-engine/src/game_actions.rs:2116)). The shared `dna_digivolve_inner` does **not** re-check phase — the effect-initiated path can run from any phase.

6. **Action-ID convention for material selection:** The first-stage selection populates `valid_action_ids` with raw `battle_area` indices cast to `u16` (this is the existing convention in `Game::initiate_dna_digivolve` at line 2191 — see also `SelectionKind::Material` decoder routing). The second-stage selection uses the same convention.

---

## Tasks

### Task 0: Read existing code

**Files:**
- Read: `code/digimon-engine/src/effect_context/mod.rs:1895-2091`
- Read: `code/digimon-engine/src/game_actions.rs:2107-2210`
- Read: `code/digimon-engine/src/game_actions.rs:1642-1747`
- Read: `code/digimon-engine/src/game_actions.rs:2416-2517`
- Read: `code/digimon-engine/src/dna_digivolve.rs` (whole file)
- Read: `code/digimon-engine/src/game.rs` (find `pay_memory` at line 808 and `resolve_selection` at line 598)
- Read: `code/digimon-engine/src/selection.rs` (PendingSelection + SelectionCallback types)
- Read: `code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs` (whole file)
- Read: `docs/RULES_CONTEXT.md` §8-2, §15-16-3, §16-30
- Read: `DCGO/Assets/Scripts/Script/CardEffectCommons/DNADigivolveEffects.cs:255-451` (the `DNADigivolveWithHandOrTrashCardIntoHandOrTrash` flow — note `orderedRoots` ordering)
- Read: `code/engine_py_legacy/engine/core/player.py:286-338` (`dna_digivolve`)
- Read: `code/engine_py_legacy/engine/game/effects.py:826-869` (`_initiate_dna_digivolve` + `_dna_select_second`)

- [ ] **Step 1: Read all reference files**

No code changes. The engineer should be able to answer:
- What does `effect_initiated_dna_digivolve` do today, and what's the trigger sequence at lines 2074-2088?
- What's the stub at line 2197-2207 supposed to do?
- How does `digivolve_from_hand` validate, pay memory, mutate the permanent, and fire triggers?
- How does `pay_memory` behave on failure (line 808)?
- What's `SelectionCallback`'s shape (`Box<dyn FnOnce(&mut Game, u16) + Send + Sync + 'static>`)?
- DCGO `orderedRoots` ordering and `payCost: bool` semantics.

- [ ] **Step 2: Verify baseline tests pass**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_initiated_dna_digivolve`
Expected: PASS (3 existing tests).

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`
Expected: PASS (full engine suite green pre-change).

---

### Task 1: Add `Effect::on_dna_digivolve` builder

**Files:**
- Modify: `code/digimon-engine/src/effect.rs:253`

This is a 3-line addition that mirrors the existing `Effect::on_digivolve` builder. Needed for the DSL test (Task 8) and for any hand-authored Rust card scripts that want to author `OnDnaDigivolve` clauses.

- [ ] **Step 1: Add the builder**

Locate the existing `Effect::on_digivolve` at line 253. Add directly after it:

```rust
    /// Fires when this Digimon DNA digivolves (as the merged result card).
    /// Refines `OnDigivolve`: a card with both an `OnDigivolve` and an
    /// `OnDnaDigivolve` clause sees the broader timing fire first, then the
    /// more-specific one, on the same merged permanent.
    pub fn on_dna_digivolve(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnDnaDigivolve)
    }
```

- [ ] **Step 2: Build to confirm no breakage**

Run: `cargo build --manifest-path code/digimon-engine/Cargo.toml`
Expected: build succeeds.

- [ ] **Step 3: Commit**

```bash
git add code/digimon-engine/src/effect.rs
git commit -m "engine: add Effect::on_dna_digivolve builder"
```

---

### Task 2: Port DNA stacking-order and second-target validators

**Files:**
- Modify: `code/digimon-engine/src/dna_digivolve.rs`

Port the two Python helpers we'll need in Task 5's selection-chain logic. `get_dna_stacking_order` returns the matching `DnaCost` and orders the materials so `top` matches `requirement1`. `get_valid_dna_second_targets` returns the second-material indices given a chosen first.

- [ ] **Step 1: Add tests for the new helpers**

Append to `code/digimon-engine/src/dna_digivolve.rs`:

```rust
#[cfg(test)]
mod tests_stacking {
    use super::*;
    use crate::card_data::{CardData, DnaCost, DnaRequirement};
    use crate::card_source::CardSource;
    use crate::permanent::Permanent;

    fn lvl_card(idx: usize, level: u8) -> CardData {
        let mut d = CardData::default();
        d.card_id = format!("LVL{}-{}", level, idx);
        d.level = Some(level);
        d
    }

    fn perm_at(data_index: usize) -> Permanent {
        Permanent::new(vec![CardSource::new_test(data_index)])
    }

    #[test]
    fn stacking_order_picks_correct_orientation() {
        // evo wants req1=Lv5, req2=Lv6
        let mut evo = CardData::default();
        evo.dna_costs = vec![DnaCost {
            memory_cost: 1,
            requirement1: DnaRequirement { level: 5, ..Default::default() },
            requirement2: DnaRequirement { level: 6, ..Default::default() },
        }];
        let data = vec![evo, lvl_card(0, 5), lvl_card(1, 6)];
        let p_lv5 = perm_at(1);
        let p_lv6 = perm_at(2);

        // Pass perms in (Lv6, Lv5) order — helper should report top=Lv5, bottom=Lv6.
        let order = get_dna_stacking_order(&data[0], &p_lv6, &p_lv5, &data);
        let (top_is_a, cost) = order.expect("should match");
        assert!(!top_is_a, "passed (Lv6, Lv5); top should be perm_b (Lv5)");
        assert_eq!(cost.memory_cost, 1);
    }

    #[test]
    fn second_targets_excludes_first_index() {
        let mut evo = CardData::default();
        evo.dna_costs = vec![DnaCost {
            memory_cost: 0,
            requirement1: DnaRequirement { level: 5, ..Default::default() },
            requirement2: DnaRequirement { level: 5, ..Default::default() },
        }];
        let data = vec![evo, lvl_card(0, 5)];
        let battle = vec![perm_at(1), perm_at(1), perm_at(1)];

        let valid = get_valid_dna_second_targets(&data[0], 1, &battle, &data);
        assert_eq!(valid, vec![0, 2], "first idx (1) must be excluded");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib dna_digivolve::tests_stacking`
Expected: FAIL — `cannot find function 'get_dna_stacking_order'` and `'get_valid_dna_second_targets'`.

- [ ] **Step 3: Implement `get_dna_stacking_order`**

Append to `code/digimon-engine/src/dna_digivolve.rs` (above the `tests_stacking` module):

```rust
/// Returns `Some((top_is_perm_a, &DnaCost))` for the matching cost on `evo_meta`.
/// `top_is_perm_a` is true when `perm_a` matches `requirement1` (so `perm_a`
/// is the "top half" of the bottom material stack); false when `perm_b` does.
///
/// Tries each cost in order; for each cost tries `(perm_a, perm_b)` mapped to
/// `(req1, req2)` first, then `(req2, req1)`. Returns `None` if no orientation
/// of any cost is satisfied.
///
/// Port of Python's `digivolve_validator.py::get_dna_stacking_order`.
pub fn get_dna_stacking_order<'a>(
    evo_meta: &'a CardData,
    perm_a: &Permanent,
    perm_b: &Permanent,
    data: &[CardData],
) -> Option<(bool, &'a DnaCost)> {
    for cost in &evo_meta.dna_costs {
        if perm_matches_req(perm_a, &cost.requirement1, data)
            && perm_matches_req(perm_b, &cost.requirement2, data)
        {
            return Some((true, cost));
        }
        if perm_matches_req(perm_a, &cost.requirement2, data)
            && perm_matches_req(perm_b, &cost.requirement1, data)
        {
            return Some((false, cost));
        }
    }
    None
}

/// Returns battle-area indices that can be the second material when the
/// first material is `first_idx`. The first index itself is excluded.
///
/// Port of Python's `digivolve_validator.py::get_valid_dna_second_targets`.
pub fn get_valid_dna_second_targets(
    evo_meta: &CardData,
    first_idx: usize,
    battle_area: &[Permanent],
    data: &[CardData],
) -> Vec<u16> {
    if first_idx >= battle_area.len() {
        return Vec::new();
    }
    let first_perm = &battle_area[first_idx];
    let mut out = Vec::new();
    for j in 0..battle_area.len() {
        if j == first_idx {
            continue;
        }
        if can_dna_digivolve(evo_meta, first_perm, &battle_area[j], data) {
            out.push(j as u16);
        }
    }
    out
}
```

The need for `DnaCost` to be public (the function returns a borrow of one) means `pub struct DnaCost` and `pub struct DnaRequirement` in `card_data.rs` — verify those are already `pub` in Task 0; if not, this task includes a 1-line visibility bump.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib dna_digivolve::tests_stacking`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/dna_digivolve.rs code/digimon-engine/src/card_data.rs
git commit -m "engine: add get_dna_stacking_order + get_valid_dna_second_targets"
```

---

### Task 3: Add failing test asserting `OnDnaDigivolve` fires from effect-initiated path

**Files:**
- Modify: `code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs`

Add a test that authors a card with an `OnDnaDigivolve`-timed effect and asserts the effect runs after the merge. Will fail because the helper does not yet enqueue `OnDnaDigivolve`.

- [ ] **Step 1: Add the failing test**

Append to `code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs`:

```rust
#[test]
fn effect_initiated_dna_digivolve_fires_on_dna_digivolve_trigger() {
    use digimon_engine::effect::{Effect, EffectBuilder};
    use digimon_engine::card_registry::register_effect;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    // OnDnaDigivolve handler: increments a counter when fired on the merged perm.
    let fire_count = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&fire_count);

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-B", "DnaSourceB"))
        .add_card(make_test_card_with_effect(
            "TST-DNA-RESULT",
            "DnaResult",
            Box::new(move |handle| {
                let c = Arc::clone(&counter);
                Effect::on_dna_digivolve(handle)
                    .with_handler(move |_ctx| {
                        c.fetch_add(1, Ordering::SeqCst);
                    })
                    .build()
            }),
        ))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();

    let handle_a = runner.place_on_field(0, "TST-A", None);
    let handle_b = runner.place_on_field(0, "TST-B", None);
    let hand_card_handle = runner.game.players[0].hand[0].handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card_handle, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, hand_card_handle, 0, true)
    };
    assert!(result.is_some());

    assert_eq!(
        fire_count.load(Ordering::SeqCst),
        1,
        "OnDnaDigivolve must fire exactly once on the merged permanent"
    );
}
```

> **Note:** If `make_test_card_with_effect` and `Effect::with_handler` aren't already in `DebugRunner`, fall back to the existing pattern used elsewhere in `tests/effect_context/` — search for tests that author triggered effects on test cards (e.g., `tests/timing_dispatch.rs`). If no helper exists, register the effect directly via `register_effect` post-`DebugRunner::builder().build()`. Don't invent new helpers in this task — use whatever pattern the existing OnDigivolve trigger tests use. If the existing tests use `card_registry` directly, do the same.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_initiated_dna_digivolve effect_initiated_dna_digivolve_fires_on_dna_digivolve_trigger`
Expected: FAIL with `OnDnaDigivolve must fire exactly once` — counter reads 0 because the timing isn't enqueued yet.

- [ ] **Step 3: Commit (failing test, intentionally)**

```bash
git add code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs
git commit -m "engine: failing test for OnDnaDigivolve trigger from effect path"
```

---

### Task 4: Extract `Game::dna_digivolve_inner` and wire `OnDnaDigivolve`

**Files:**
- Modify: `code/digimon-engine/src/game.rs` (add new method)
- Modify: `code/digimon-engine/src/effect_context/mod.rs:1895-2091` (delegate)

Move the merge + payment + trigger surface out of `EffectContext::effect_initiated_dna_digivolve` into a `pub(crate) Game::dna_digivolve_inner`. Both call sites (Task 5's user-action callback and the existing effect-initiated `EffectContext` method) call into it. While moving, add `OnDnaDigivolve` to the trigger sequence.

- [ ] **Step 1: Add `Game::dna_digivolve_inner`**

In `code/digimon-engine/src/game.rs`, near `Game::can_digivolve` (line 1037), add:

```rust
    /// Shared core for DNA digivolve. Performs material consumption, hand-card
    /// consumption, stack merging, memory payment (if `cost > 0` and not under
    /// `ignore_requirements`), and trigger firing.
    ///
    /// **Material-stack ordering (canonical):** `target_a.card_sources` are
    /// concatenated, then `target_b.card_sources`, then `from_hand` is pushed
    /// on top. `target_a` should correspond to `DnaCost::requirement1` and
    /// `target_b` to `requirement2`; callers that select materials by user
    /// input must pre-orient via `get_dna_stacking_order`.
    ///
    /// **Trigger surface (in order):**
    /// 1. `WhenDigivolving` on the merged permanent → drain
    /// 2. `OnDigivolve` on every player's battle area → drain
    /// 3. `OnDnaDigivolve` on the merged permanent → drain
    ///
    /// **Index-shift:** if `target_a.player == target_b.player` and
    /// `target_b.index < target_a.index`, the merged permanent ends up at
    /// `target_a.index - 1` (because removing `target_b` first shifts the
    /// remaining slots). Callers should use the returned handle, not
    /// `target_a` directly.
    ///
    /// **Returns** `Some(merged_handle)` on success, `None` on:
    /// - identical targets (`target_a == target_b`)
    /// - either target's index out of range on its player's battle area
    /// - hand index out of range on `hand_owner`
    /// - `cost > 0` and `!ignore_requirements` and `Game::pay_memory` fails
    ///
    /// The pay-memory-bypass branch (`ignore_requirements && cost > 0`) is
    /// *not* present here — callers that need to bypass the affordability
    /// floor must subtract from `self.memory` before calling. See
    /// `EffectContext::effect_initiated_dna_digivolve` for the wrapper that
    /// implements that branch.
    ///
    /// `grant_digivolve_bonus`: if true, `hand_owner` draws 1 card after the
    /// merge but before triggers fire. The user-action path passes `true`
    /// (matching `digivolve_from_hand`); the effect-initiated path passes
    /// `false`.
    pub(crate) fn dna_digivolve_inner(
        &mut self,
        target_a: PermanentHandle,
        target_b: PermanentHandle,
        hand_owner: PlayerId,
        hand_index: usize,
        cost: u16,
        grant_digivolve_bonus: bool,
    ) -> Option<PermanentHandle> {
        use crate::enums::EffectTiming;
        use crate::selection::TriggerSource;

        if target_a == target_b {
            return None;
        }
        if (target_a.index as usize) >= self.player(target_a.player).battle_area.len() {
            return None;
        }
        if (target_b.index as usize) >= self.player(target_b.player).battle_area.len() {
            return None;
        }
        if hand_index >= self.player(hand_owner).hand.len() {
            return None;
        }

        if cost > 0 && !self.pay_memory(cost) {
            return None;
        }

        let target_a_index_after = if target_a.player == target_b.player
            && (target_b.index as usize) < (target_a.index as usize)
        {
            (target_a.index as usize) - 1
        } else {
            target_a.index as usize
        };

        let perm_b = self
            .player_mut(target_b.player)
            .battle_area
            .remove(target_b.index as usize);
        let new_top = self.player_mut(hand_owner).hand.remove(hand_index);

        let turn = self.turn_count;
        {
            let perm_a =
                &mut self.player_mut(target_a.player).battle_area[target_a_index_after];
            perm_a.card_sources.extend(perm_b.card_sources.into_iter());
            perm_a.card_sources.push(new_top);
            perm_a.turn_digivolved = turn;
            // Per Rules §8-2-2-1-6, DNA digivolve grants "can attack same
            // turn" — the merged permanent has no summoning sickness. The
            // existing `Permanent::digivolve` semantics already treat
            // `turn_digivolved == turn_count` as eligible to attack; no
            // extra flag needed.
        }

        let merged_handle = PermanentHandle {
            player: target_a.player,
            index: target_a_index_after as u8,
        };

        if grant_digivolve_bonus {
            self.player_mut(hand_owner).draw();
        }

        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(merged_handle),
        );
        self.drain_effect_queue();

        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnDigivolve,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();

        self.enqueue_triggered(
            EffectTiming::OnDnaDigivolve,
            TriggerSource::Permanent(merged_handle),
        );
        self.drain_effect_queue();

        Some(merged_handle)
    }
```

- [ ] **Step 2: Refactor `EffectContext::effect_initiated_dna_digivolve` to delegate**

In `code/digimon-engine/src/effect_context/mod.rs`, replace the body of `effect_initiated_dna_digivolve` (lines 1938 onward — the existing 200-ish-line implementation) with delegation. The early validation and `ignore_requirements`-bypass branch stay; the merge + triggers move out.

```rust
    pub fn effect_initiated_dna_digivolve(
        &mut self,
        target_a: PermanentHandle,
        target_b: PermanentHandle,
        from_hand: CardHandle,
        cost: i32,
        ignore_requirements: bool,
    ) -> Option<PermanentHandle> {
        if target_a == target_b {
            return None;
        }
        if (target_a.index as usize)
            >= self.game.player(target_a.player).battle_area.len()
        {
            return None;
        }
        if (target_b.index as usize)
            >= self.game.player(target_b.player).battle_area.len()
        {
            return None;
        }

        // Locate the from_hand card across all players' hands.
        let mut hand_owner: Option<PlayerId> = None;
        let mut hand_index: Option<usize> = None;
        for pid in 0..self.game.players.len() {
            if let Some(idx) = self.game.players[pid]
                .hand
                .iter()
                .position(|c| c.handle() == from_hand)
            {
                hand_owner = Some(pid as PlayerId);
                hand_index = Some(idx);
                break;
            }
        }
        let (hand_owner, hand_index) = (hand_owner?, hand_index?);

        let effective_cost: u16 = cost.max(0) as u16;

        // Memory: under ignore_requirements bypass the floor; otherwise let
        // dna_digivolve_inner pay normally.
        if ignore_requirements && effective_cost > 0 {
            let new_memory = self.game.memory - effective_cost as i16;
            let delta = new_memory - self.game.memory;
            self.game.memory = new_memory;
            let seq = self.game.next_event_seq();
            let player = self.game.turn_player();
            self.game
                .events
                .push(crate::events::GameEvent::MemoryChange {
                    seq,
                    player,
                    delta,
                    total: self.game.memory,
                });
            // Pass cost=0 to the inner so it doesn't double-pay.
            self.game.dna_digivolve_inner(
                target_a,
                target_b,
                hand_owner,
                hand_index,
                0,
                false,
            )
        } else {
            self.game.dna_digivolve_inner(
                target_a,
                target_b,
                hand_owner,
                hand_index,
                effective_cost,
                false,
            )
        }
    }
```

- [ ] **Step 3: Update the doc-comment on `effect_initiated_dna_digivolve`**

Replace lines ~1895-1937 (the doc-comment above the `pub fn`). The new doc:

```rust
    /// Merge two existing battle-area permanents into a single permanent
    /// topped with a card from hand. Effect-initiated DNA digivolve.
    ///
    /// Delegates to `Game::dna_digivolve_inner` for the merge + triggers.
    /// This wrapper handles the IR's two-knob shape (`cost: i32` separate
    /// from `ignore_requirements: bool`) and the pay-memory-bypass branch
    /// that fires when `ignore_requirements` is set and the printed cost
    /// would otherwise dip below the memory floor.
    ///
    /// ## Stacking order
    ///
    /// `target_a.card_sources ++ target_b.card_sources ++ [from_hand]`.
    /// `target_a` corresponds to `DnaCost::requirement1`. See
    /// `Game::dna_digivolve_inner` for the canonical contract.
    ///
    /// ## Triggers
    ///
    /// `WhenDigivolving` → `OnDigivolve` (global) → `OnDnaDigivolve`,
    /// each followed by a queue drain. See
    /// `Game::dna_digivolve_inner` for the firing sequence.
    ///
    /// ## Semantics of `ignore_requirements`
    ///
    /// `ignore_requirements: true` skips the affordability floor — i.e. the
    /// merge runs even when subtracting `cost` from memory would dip below
    /// `rules.memory_range.0`. The `cost` argument is still subtracted —
    /// `ignore_requirements` is not the same as "free". For
    /// `cost: 0, ignore_requirements: true`, no memory mutation occurs.
    ///
    /// ## Defensive validation
    ///
    /// Returns `None` if:
    /// - `target_a == target_b`
    /// - either target's index is out of range on its player's battle area
    /// - `from_hand` is not present in any player's hand
    /// - `cost > 0` and `!ignore_requirements` and the controller cannot
    ///   pay the memory cost (early-out before any state mutation)
```

The old caveats ("OnDnaDigivolve not yet wired", "no shared `Game::dna_digivolve_from_hand_inner`", "performs the merge inline") are gone — both have been resolved.

- [ ] **Step 4: Run all DNA digivolve tests**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_initiated_dna_digivolve`
Expected: PASS (4 tests including `effect_initiated_dna_digivolve_fires_on_dna_digivolve_trigger` from Task 3).

- [ ] **Step 5: Run full engine suite**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`
Expected: PASS — no regressions from the refactor.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/game.rs code/digimon-engine/src/effect_context/mod.rs
git commit -m "engine: extract Game::dna_digivolve_inner + wire OnDnaDigivolve

Both DNA digivolve paths now share a single merge/trigger core. Trigger
surface unifies as WhenDigivolving -> OnDigivolve -> OnDnaDigivolve.
EffectContext wrapper retains the ignore_requirements bypass branch."
```

---

### Task 5: Resolve `TODO(dna-digivolve-execute)` — wire two-stage selection

**Files:**
- Modify: `code/digimon-engine/src/game_actions.rs:2197-2207`

Replace the stubbed callback with a real two-stage selection chain. First-stage callback: decode first material idx, build valid second-material list via `get_valid_dna_second_targets`, install second-stage `PendingSelection`. Second-stage callback: decode second material idx, look up `DnaCost` via `get_dna_stacking_order`, compute effective cost (printed - `BeforePayCost` reductions, matching `digivolve_from_hand` lines 1709-1721), and call `dna_digivolve_inner`. The merged-permanent-handle is dropped; phase auto-restores via `previous_phase` on the *last* selection.

- [ ] **Step 1: Add failing tests for the user-action path**

Create `code/digimon-engine/tests/dna_digivolve_user_action.rs`:

```rust
//! Integration tests for `Game::initiate_dna_digivolve` resolving through
//! both selection stages into `Game::dna_digivolve_inner`. Companion to
//! `tests/effect_context/effect_initiated_dna_digivolve.rs` which covers
//! the engine-effect path.

use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;

fn dna_card(card_id: &str, name: &str, req1_lv: u8, req2_lv: u8, mem: u16) -> CardData {
    let mut d = make_test_card(card_id, name);
    d.dna_costs = vec![DnaCost {
        memory_cost: mem,
        requirement1: DnaRequirement { level: req1_lv, ..Default::default() },
        requirement2: DnaRequirement { level: req2_lv, ..Default::default() },
    }];
    d.is_digimon = true;
    d
}

fn lv_card(card_id: &str, name: &str, level: u8) -> CardData {
    let mut d = make_test_card(card_id, name);
    d.level = Some(level);
    d.is_digimon = true;
    d
}

#[test]
fn user_action_dna_digivolve_two_stage_resolution_merges_permanents() {
    let mut runner = DebugRunner::builder()
        .add_card(lv_card("TST-LV5", "FiveDigi", 5))
        .add_card(lv_card("TST-LV6", "SixDigi", 6))
        .add_card(dna_card("TST-DNA", "DnaDigi", 5, 6, 0))
        .hand(0, &["TST-DNA"])
        .memory(5)
        .start();

    let handle_lv5 = runner.place_on_field(0, "TST-LV5", None);
    let handle_lv6 = runner.place_on_field(0, "TST-LV6", None);

    // Initiate: phase flips to SelectMaterial, first-stage selection installed.
    let ok = runner.game.initiate_dna_digivolve(0, 0);
    assert!(ok, "initiate must accept valid hand index");
    assert_eq!(runner.game.current_phase, GamePhase::SelectMaterial);
    assert!(runner.game.pending_selection.is_some());

    // Resolve first stage: pick handle_lv5 (idx 0).
    let resolved = runner.game.resolve_selection(0, handle_lv5.index as u16);
    assert!(resolved, "first-stage resolution must succeed");
    // Second-stage selection now installed (still in SelectMaterial phase).
    assert_eq!(runner.game.current_phase, GamePhase::SelectMaterial);
    assert!(runner.game.pending_selection.is_some());

    // Resolve second stage: pick handle_lv6 (idx 1, which after handle_lv5
    // was NOT yet removed is still 1).
    let resolved2 = runner.game.resolve_selection(0, handle_lv6.index as u16);
    assert!(resolved2);

    // Phase restored to Main.
    assert_eq!(runner.game.current_phase, GamePhase::Main);
    assert!(runner.game.pending_selection.is_none());

    // One merged permanent with 3 stacked sources.
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    let merged = &runner.game.players[0].battle_area[0];
    assert_eq!(merged.card_sources.len(), 3);
    // Hand consumed.
    assert_eq!(runner.game.players[0].hand.len(), 0);
}

#[test]
fn user_action_dna_digivolve_pays_memory_cost() {
    let mut runner = DebugRunner::builder()
        .add_card(lv_card("TST-LV5", "FiveDigi", 5))
        .add_card(lv_card("TST-LV6", "SixDigi", 6))
        .add_card(dna_card("TST-DNA-3", "DnaCost3", 5, 6, 3))
        .hand(0, &["TST-DNA-3"])
        .memory(5)
        .start();

    runner.place_on_field(0, "TST-LV5", None);
    runner.place_on_field(0, "TST-LV6", None);

    runner.game.initiate_dna_digivolve(0, 0);
    runner.game.resolve_selection(0, 0);
    runner.game.resolve_selection(0, 1);

    // memory: 5 - 3 = 2
    assert_eq!(runner.game.memory, 2);
}

#[test]
fn user_action_dna_digivolve_grants_draw_bonus() {
    let mut runner = DebugRunner::builder()
        .add_card(lv_card("TST-LV5", "FiveDigi", 5))
        .add_card(lv_card("TST-LV6", "SixDigi", 6))
        .add_card(dna_card("TST-DNA", "DnaDigi", 5, 6, 0))
        .add_card(make_test_card("TST-DECK", "DeckCard"))
        .hand(0, &["TST-DNA"])
        .deck(0, &["TST-DECK"])
        .memory(5)
        .start();

    runner.place_on_field(0, "TST-LV5", None);
    runner.place_on_field(0, "TST-LV6", None);

    let pre_hand_size = runner.game.players[0].hand.len();
    runner.game.initiate_dna_digivolve(0, 0);
    runner.game.resolve_selection(0, 0);
    runner.game.resolve_selection(0, 1);

    // Hand: -1 (DNA card consumed), +1 (digivolution bonus draw) = same size.
    // But deck shrank by 1.
    assert_eq!(runner.game.players[0].hand.len(), pre_hand_size);
    assert_eq!(runner.game.players[0].deck.len(), 0);
}

#[test]
fn user_action_dna_digivolve_rejects_when_phase_is_not_main() {
    let mut runner = DebugRunner::builder()
        .add_card(dna_card("TST-DNA", "DnaDigi", 5, 6, 0))
        .hand(0, &["TST-DNA"])
        .start();
    runner.game.current_phase = GamePhase::Battle; // force non-Main

    let ok = runner.game.initiate_dna_digivolve(0, 0);
    assert!(!ok, "non-Main phase must reject");
    assert!(runner.game.pending_selection.is_none());
}
```

> If `dna_costs` requires a `pub` deserialize path that doesn't yet exist (i.e., `DnaCost`/`DnaRequirement` were only constructed via JSON), Task 2 should have already exposed enough surface — but if not, expose constructors as needed. Match whatever pattern the existing `mask_main_parity.rs` tests use (they construct `dna_costs` by hand).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action`
Expected: FAIL — first-stage callback is stubbed (`let _ = (game, action_id);`), so `resolve_selection(0, 0)` returns and the second-stage selection is never installed. Tests fail at the `assert!(runner.game.pending_selection.is_some())` post-first-resolve.

- [ ] **Step 3: Replace the stubbed callback in `Game::initiate_dna_digivolve`**

In `code/digimon-engine/src/game_actions.rs:2197-2207`, replace the callback body:

```rust
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                // Stage 1 resolution: action_id is the chosen first-material
                // battle_area index for `selecting_player`.
                let first_idx = action_id as usize;
                let first_player = selecting_player;
                let evo_hand_index = hand_index;
                let evo_card = source_card;

                // Validate the first index against the controller's current
                // battle_area (it could have shifted since selection was
                // installed if a triggered effect removed a permanent during
                // the install drain — defensive).
                if first_idx >= game.player(first_player).battle_area.len() {
                    return;
                }

                // Build valid second-material list for the chosen first.
                let evo_meta = &game.card_data[
                    game.player(first_player).hand[evo_hand_index].data_index
                ];
                let second_targets = crate::dna_digivolve::get_valid_dna_second_targets(
                    evo_meta,
                    first_idx,
                    &game.player(first_player).battle_area,
                    &game.card_data,
                );
                if second_targets.is_empty() {
                    return;
                }

                // Install stage-2 selection. previous_phase was Main when
                // stage-1 was installed; we preserve it through the chain so
                // the final resolution returns to Main.
                let stage1_previous_phase = previous_phase;

                game.pending_selection = Some(crate::selection::PendingSelection {
                    kind: crate::selection::SelectionKind::Material,
                    selecting_player: first_player,
                    previous_phase: stage1_previous_phase,
                    valid_action_ids: second_targets,
                    is_optional: false,
                    prompt: "Select second DNA material".to_string(),
                    effect_choices: None,
                    source_card: evo_card,
                    source_permanent: None,
                    callback: Box::new(move |game: &mut Game, action_id: u16| {
                        // Stage 2 resolution: action_id is the chosen
                        // second-material battle_area index.
                        let second_idx = action_id as usize;
                        if second_idx >= game.player(first_player).battle_area.len() {
                            return;
                        }
                        if first_idx == second_idx {
                            return;
                        }

                        // Re-resolve the matching DnaCost for the chosen pair.
                        // Use the snapshot AFTER any pending state mutations,
                        // since stage-1 install drain could have changed
                        // things (rare).
                        let evo_meta = &game.card_data[
                            game.player(first_player).hand[evo_hand_index].data_index
                        ];
                        let battle = &game.player(first_player).battle_area;
                        let perm_first = &battle[first_idx];
                        let perm_second = &battle[second_idx];

                        let stacking = crate::dna_digivolve::get_dna_stacking_order(
                            evo_meta, perm_first, perm_second, &game.card_data,
                        );
                        let Some((first_is_top, dna_cost)) = stacking else {
                            return;
                        };

                        // Orient: target_a maps to req1 (the "top" of the
                        // bottom-half stack). If first_is_top, the chosen
                        // first IS req1; else swap.
                        let (target_a, target_b) = if first_is_top {
                            (
                                crate::card_source::PermanentHandle {
                                    player: first_player,
                                    index: first_idx as u8,
                                },
                                crate::card_source::PermanentHandle {
                                    player: first_player,
                                    index: second_idx as u8,
                                },
                            )
                        } else {
                            (
                                crate::card_source::PermanentHandle {
                                    player: first_player,
                                    index: second_idx as u8,
                                },
                                crate::card_source::PermanentHandle {
                                    player: first_player,
                                    index: first_idx as u8,
                                },
                            )
                        };

                        // Cost: printed - BeforePayCost reductions (matches
                        // digivolve_from_hand:1709-1721).
                        let printed_cost = dna_cost.memory_cost;
                        let total_reduction =
                            game.scan_before_pay_cost_reduction(first_player);
                        let effective_cost =
                            (printed_cost as i32 - total_reduction).max(0) as u16;

                        // Delegate to the shared core. `grant_digivolve_bonus
                        // = true` matches `digivolve_from_hand` and Python's
                        // `player.dna_digivolve` draw on success.
                        let _ = game.dna_digivolve_inner(
                            target_a,
                            target_b,
                            first_player,
                            evo_hand_index,
                            effective_cost,
                            true,
                        );

                        // Phase auto-restores to stage1_previous_phase via
                        // resolve_selection's post-callback logic; no manual
                        // restore needed here.
                    }),
                    on_decline: None,
                });
                // current_phase stays SelectMaterial; pending_selection is
                // freshly installed for stage 2.
            }),
```

> **Note on closure captures:** `selecting_player`, `previous_phase`, `hand_index`, `source_card` are already captured by the outer closure. The inner closure additionally captures `first_player`, `first_idx`, `evo_hand_index`, `evo_card`, and `stage1_previous_phase` from the stage-1 closure scope. All values are `Copy` or `Clone` (`PlayerId` = `u8`, `usize`, `CardHandle` is `Copy`). `Box<dyn FnOnce(&mut Game, u16) + Send + Sync + 'static>` is satisfied because the captured values are `Send + Sync + 'static`.

- [ ] **Step 4: Run the user-action tests**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action`
Expected: PASS (4 tests).

- [ ] **Step 5: Run the full engine suite**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`
Expected: PASS — no regressions.

- [ ] **Step 6: Verify the TODO is gone**

Run: `Grep` for `TODO\(dna-digivolve-execute\)` across `code/digimon-engine/`.
Expected: No matches.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/game_actions.rs code/digimon-engine/tests/dna_digivolve_user_action.rs
git commit -m "engine: resolve TODO(dna-digivolve-execute) - wire two-stage selection

User-action DNA digivolve flow now drives both selection stages and
delegates to Game::dna_digivolve_inner. Material-stack ordering and
trigger surface match the engine-effect path."
```

---

### Task 6: Drop "CHOSEN-NOT-CANONICAL" marker

**Files:**
- Modify: `code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs`

The order is now canonical (locked in Task 4 and matched by Task 5). Replace the marker comment with a positive statement of the contract.

- [ ] **Step 1: Update the marker comment**

In `code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs`, find the "CHOSEN-NOT-CANONICAL" block (around line 193 in the test asserting stack order). Replace:

```rust
    // CHOSEN-NOT-CANONICAL contract: order is target_a's stack first,
    // target_b's second, then hand top. When the user-action
    // initiate_dna_digivolve lands (TODO(dna-digivolve-execute) at
    // game_actions.rs:2198), update both this test and that path together.
```

with:

```rust
    // Canonical stack order (locked in Game::dna_digivolve_inner): target_a's
    // stack first, target_b's second, then hand top. Matches the user-action
    // path (Game::initiate_dna_digivolve -> two-stage selection).
```

- [ ] **Step 2: Update the file's top doc-comment**

Replace lines 1-16 (the file-header comment) with:

```rust
//! Tests for `EffectContext::effect_initiated_dna_digivolve`, which delegates
//! to the shared `Game::dna_digivolve_inner` (see also
//! `tests/dna_digivolve_user_action.rs` for the user-action path).
//!
//! Card-text precedent: BT5-085 Omnimon-style "DNA digivolve from-effect".
//!
//! Stacking order (canonical, shared with the user-action path):
//!   target_a.card_sources ++ target_b.card_sources ++ [from_hand]
//! `target_a` corresponds to `DnaCost::requirement1`.
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_initiated_dna_digivolve`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs
git commit -m "engine: drop CHOSEN-NOT-CANONICAL marker; both DNA paths share order"
```

---

### Task 7: Add DSL example card and integration test for `<OnDnaDigivolve>` clause

**Files:**
- Create: `code/digimon-engine/cards/_examples/TST_DNA_TRIGGER.yaml`
- Create: `code/digimon-engine/tests/dsl/phase2g_on_dna_digivolve.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs` (register new test module)

A DSL author who writes `<OnDnaDigivolve>` should now see their effect fire. This task proves it via end-to-end.

- [ ] **Step 1: Create the example card**

Match the structure of an existing example, e.g. `code/digimon-engine/cards/_examples/BT20-083.yaml`. Inspect that file before writing this one — replicate the YAML schema verbatim. Schematic outline:

```yaml
card_id: TST-DNA-TRIGGER
name: TestDnaTrigger
type: Digimon
level: 7
dna_costs:
  - memory_cost: 0
    requirement1: { level: 5 }
    requirement2: { level: 6 }
effects:
  - timing: OnDnaDigivolve
    steps:
      - kind: Draw
        target: controller
        count: 1
```

> If the YAML field names differ from the above (e.g. `Type` vs `type`, snake_case vs PascalCase, presence of `is_digimon: true`), follow `BT20-083.yaml` exactly. The intent is: a level-7 digi card that DNA-digivolves from a Lv5+Lv6 pair, no memory cost, with a single `OnDnaDigivolve` clause that draws 1 card for the controller.

- [ ] **Step 2: Create the integration test**

Create `code/digimon-engine/tests/dsl/phase2g_on_dna_digivolve.rs`:

```rust
//! Phase 2g — `<OnDnaDigivolve>` clause fires from both DNA digivolve paths.
//!
//! Card text precedent: any "[When this Digimon DNA-digivolves]" effect.
//! Authored via the DSL example card `TST_DNA_TRIGGER`.

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::GamePhase;

const DNA_TRIGGER_YAML: &str = include_str!(
    "../../cards/_examples/TST_DNA_TRIGGER.yaml"
);

fn lv_card(card_id: &str, name: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut d = make_test_card(card_id, name);
    d.level = Some(level);
    d.is_digimon = true;
    d
}

#[test]
fn dsl_on_dna_digivolve_fires_from_user_action_path() {
    let mut runner = DebugRunner::builder()
        .add_card(lv_card("TST-LV5", "FiveDigi", 5))
        .add_card(lv_card("TST-LV6", "SixDigi", 6))
        .add_dsl_card(DNA_TRIGGER_YAML)
        .hand(0, &["TST-DNA-TRIGGER"])
        .deck(0, &["TST-DECK-A"])
        .memory(5)
        .start();

    runner.place_on_field(0, "TST-LV5", None);
    runner.place_on_field(0, "TST-LV6", None);

    let pre_hand = runner.game.players[0].hand.len();
    runner.game.initiate_dna_digivolve(0, 0);
    runner.game.resolve_selection(0, 0);
    runner.game.resolve_selection(0, 1);

    // Hand: -1 (DNA card consumed), +1 (digivolution bonus draw),
    // +1 (OnDnaDigivolve draw clause). Net +1 from pre.
    assert_eq!(
        runner.game.players[0].hand.len(),
        pre_hand + 1,
        "OnDnaDigivolve clause must draw 1 in addition to digivolution bonus"
    );
}

#[test]
fn dsl_on_dna_digivolve_fires_from_effect_path() {
    let mut runner = DebugRunner::builder()
        .add_card(lv_card("TST-LV5", "FiveDigi", 5))
        .add_card(lv_card("TST-LV6", "SixDigi", 6))
        .add_dsl_card(DNA_TRIGGER_YAML)
        .hand(0, &["TST-DNA-TRIGGER"])
        .deck(0, &["TST-DECK-A"])
        .memory(5)
        .start();

    let handle_a = runner.place_on_field(0, "TST-LV5", None);
    let handle_b = runner.place_on_field(0, "TST-LV6", None);
    let hand_card = runner.game.players[0].hand[0].handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, hand_card, 0, true)
    };
    assert!(result.is_some());

    // Effect-initiated path does NOT grant digivolution bonus, but the
    // OnDnaDigivolve clause still fires.
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "OnDnaDigivolve clause draws 1 (no digivolution bonus from effect path)"
    );
}
```

> If `DebugRunner::add_dsl_card` doesn't exist, fall back to whatever loader the existing `tests/dsl/phase2f1_*.rs` files use. The pattern there is the canonical reference — match it. The actual card-name string `"TST-DNA-TRIGGER"` must match the `card_id` field in the YAML.

- [ ] **Step 3: Register the test module**

In `code/digimon-engine/tests/dsl/main.rs`, add the new module entry:

```rust
mod phase2g_on_dna_digivolve;
```

Match the surrounding pattern (the file already lists `mod phase2f1_*`, etc.).

- [ ] **Step 4: Run the DSL tests**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2g_on_dna_digivolve`
Expected: PASS (2 tests).

- [ ] **Step 5: Run the full engine suite**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/phase2g_on_dna_digivolve.rs code/digimon-engine/cards/_examples/TST_DNA_TRIGGER.yaml
git commit -m "engine: DSL <OnDnaDigivolve> clause fires from both DNA digivolve paths"
```

---

### Task 8: Update tracker docs

**Files:**
- Modify: `docs/RUST_PYTHON_PARITY.md`
- Modify: `docs/RUST_ENGINE_GAPS.md` (only if `OnDnaDigivolve` is currently listed as a gap)

- [ ] **Step 1: Add §4.5e entry to `RUST_PYTHON_PARITY.md`**

Find the §4.5a "DNA digivolve mask — implemented" entry (around line 372). Add a new sub-section after the existing 4.5a/4.5b/4.5c entries:

```markdown
### 4.5e 🟢 DNA digivolve execution — implemented

**User-action path** — `Game::initiate_dna_digivolve` ([game_actions.rs:2111](../code/digimon-engine/src/game_actions.rs#L2111)) installs a two-stage `SelectionKind::Material` chain. Stage 1 picks the first material, stage 2 picks the second. Stage 2 resolution computes the matching `DnaCost` via `get_dna_stacking_order`, applies `BeforePayCost` reductions, and calls `Game::dna_digivolve_inner`.

**Engine-effect path** — `EffectContext::effect_initiated_dna_digivolve` ([effect_context/mod.rs](../code/digimon-engine/src/effect_context/mod.rs)) delegates to the same `Game::dna_digivolve_inner`. The wrapper handles the IR's `cost: i32` + `ignore_requirements: bool` shape and the pay-memory-bypass branch.

**Shared core** — `Game::dna_digivolve_inner` performs material consumption, hand-card consumption, stack merging, optional memory payment, optional digivolution-bonus draw, and trigger firing.

**Trigger surface** (both paths): `WhenDigivolving` (merged perm) → drain → `OnDigivolve` (global) → drain → `OnDnaDigivolve` (merged perm) → drain.

**Stack ordering** (canonical, both paths): `target_a.card_sources ++ target_b.card_sources ++ [from_hand]`. `target_a` corresponds to `DnaCost::requirement1`. Diverges from Python's `bottom + top + [evo]` order; Python is sunset and the printed rules don't specify intra-stack ordering between materials.

**Coverage:**
- `tests/effect_context/effect_initiated_dna_digivolve.rs` — engine-effect path (4 tests including `OnDnaDigivolve` firing)
- `tests/dna_digivolve_user_action.rs` — user-action path (4 tests covering two-stage flow, memory cost, draw bonus, phase rejection)
- `tests/dsl/phase2g_on_dna_digivolve.rs` — DSL `<OnDnaDigivolve>` clause from both paths
```

- [ ] **Step 2: Update `RUST_ENGINE_GAPS.md` if applicable**

Run: `Grep` for `OnDnaDigivolve` in `docs/RUST_ENGINE_GAPS.md`.

If matches exist, mark the gap resolved (add a "✅ resolved" note linking to this plan or to the §4.5e entry above). If no matches, this step is a no-op.

- [ ] **Step 3: Commit**

```bash
git add docs/RUST_PYTHON_PARITY.md docs/RUST_ENGINE_GAPS.md
git commit -m "docs: track DNA digivolve execution parity (§4.5e) + OnDnaDigivolve resolution"
```

---

### Task 9: Final sanity sweep

**Files:** None modified — verification only.

- [ ] **Step 1: Verify no `TODO(dna-digivolve-execute)` references remain**

Run: `Grep` for `TODO\(dna-digivolve-execute\)` across the entire repo.
Expected: No matches.

- [ ] **Step 2: Verify no `CHOSEN-NOT-CANONICAL` references remain**

Run: `Grep` for `CHOSEN-NOT-CANONICAL` across the entire repo.
Expected: No matches.

- [ ] **Step 3: Run the full engine test suite**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`
Expected: PASS — no regressions, all new tests green.

- [ ] **Step 4: Run the Python-side parity tests via the PyO3 backend**

If the PyO3 bindings expose DNA digivolve through `RustHeadlessGame` (verify by `Grep` for `dna_digivolve` in `code/digimon-engine-py/src/lib.rs`), run:

`DIGIMON_BACKEND=rust python -m pytest code/tests/engine/test_rust_backend_parity.py -v`
Expected: PASS or unchanged-fail (no new failures introduced by this work).

If the PyO3 bindings do not currently surface DNA digivolve, this step is a no-op — surface them in a follow-up plan.

- [ ] **Step 5: Build PyO3 bindings to ensure the engine-side refactor didn't break the binding crate**

Run: `cd code/digimon-engine-py && maturin develop`
Expected: build succeeds.

---

## Self-Review Checklist

After all tasks are committed, verify:

1. **Spec coverage:**
   - ✅ `Game::dna_digivolve_inner` extracted (Task 4)
   - ✅ `OnDnaDigivolve` fires in both paths (Task 4 wires it; Task 5 user-action path uses the inner; Task 7 demonstrates via DSL)
   - ✅ Material-stack ordering canonized (Task 4 in inner; Task 5 orients via `get_dna_stacking_order`; Task 6 drops marker)
   - ✅ Doc-comment caveats removed (Task 4 step 3)
   - ✅ Behavioral tests for both paths (Tasks 3, 5, 7)
   - ✅ Tracker updates (Task 8)

2. **No placeholders:** Every code block shows real code. Every command shows expected output. No "TBD" or "similar to Task N" without repetition.

3. **Type consistency:**
   - `dna_digivolve_inner` signature is consistent across Tasks 4, 5, 7
   - `get_dna_stacking_order` returns `(bool, &DnaCost)` consistently across Task 2 and Task 5
   - `target_a` always corresponds to `requirement1` across all tasks and the doc-comments

4. **Out-of-scope items honored:** No sub-module decomposition of `effect_context/mod.rs`. The `pay_memory_unchecked` branch is preserved as-is in Task 4 step 2.

---

## Reference Index

- **Rules:** [RULES_CONTEXT.md §8-2](../docs/RULES_CONTEXT.md), §15-16-3, §16-30
- **DCGO:** [DNADigivolveEffects.cs](../DCGO/Assets/Scripts/Script/CardEffectCommons/DNADigivolveEffects.cs) (`orderedRoots` ordering at line 421), [JogressEffectObject.cs](../DCGO/Assets/Scripts/Script/JogressEffectObject.cs) (UI animation)
- **Python (sunset):** [player.py:286-338](../code/engine_py_legacy/engine/core/player.py) (`dna_digivolve`), [effects.py:826-869](../code/engine_py_legacy/engine/game/effects.py) (`_dna_select_second`), [digivolve_validator.py:207-301](../code/engine_py_legacy/engine/validation/digivolve_validator.py) (validators we ported)
- **Rust siblings:** [game_actions.rs:1642 `digivolve_from_hand`](../code/digimon-engine/src/game_actions.rs), [game_actions.rs:2416 `effect_initiated_digivolve`](../code/digimon-engine/src/game_actions.rs), [effect_context/mod.rs:1549 `EffectContext::effect_initiated_digivolve`](../code/digimon-engine/src/effect_context/mod.rs)
- **Existing tests:** [tests/effect_context/effect_initiated_dna_digivolve.rs](../code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs), [tests/mask_main_parity.rs](../code/digimon-engine/tests/mask_main_parity.rs) (DNA mask tests)
- **Earlier mask plan:** [2026-04-16-mask-parity-4-5-4-6-slice.md](2026-04-16-mask-parity-4-5-4-6-slice.md)
- **DSL phase that landed `effect_initiated_dna_digivolve`:** [2026-04-25-card-scripting-dsl-phase-2f.md](2026-04-25-card-scripting-dsl-phase-2f.md)
