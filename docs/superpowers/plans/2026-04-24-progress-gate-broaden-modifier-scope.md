# Phase E Prep — Make Progress Gate DCGO-Faithful at `add_modifier`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the Progress gate at `EffectContext::add_modifier` into literal alignment with DCGO's `CanNotAffectedClass` semantics — *every* modifier sourced by an opponent's effect against the Progress-attacker is suppressed, regardless of `ModifierType` variant or sign. Reverts the Phase B "positive DP buffs still apply" sanity carve-out.

**Architecture:** Drop the per-`ModifierType` classifier idea entirely. `Game::progress_excludes(target, source)` already encodes the exact predicate DCGO consults — `is_progress_attacker AND source != target.player`. `add_modifier` calls it unconditionally and short-circuits; `add_dp_modifier` becomes a thin pass-through to `add_modifier`. Single decision point, no allowlist, no signedness branching.

**Why option A (DCGO-literal) over option B (hostile-only):**
- DCGO's [`Progress.cs:99`](../DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/Progress.cs#L99) `SkillCondition` is `IsOpponentEffect(...)` — purely a source-controller check. No hostility classification, no sign check. Every consumer of `CanNotBeAffected` (incl. [`ChangeDP.cs:25,43`](../DCGO/Assets/Scripts/Script/CardEffectCommons/GiveEffect/GiveEffectToPermanent/ChangeDP.cs#L25)) inherits this all-or-nothing semantic — positive DP grants from an opponent gate too.
- The Phase B `opponent_effect_positive_dp_still_applies_to_progress_attacker` test was a deliberate Rust-Python divergence rationalized at write-time as "RL-friendly." That rationalization holds less weight than DCGO faithfulness for the no-approximations policy.
- Hostility classification is fragile — every new `ModifierType` variant added in future phases would need to be re-classified, and protective modifiers sourced by opponents (rare but real, e.g. global "all Digimon can't be deleted" riders) would silently leak.

**Tech Stack:** Rust 1.x, `digimon-engine` crate, integration tests under `digimon-engine/tests/combat/progress_mutation_gates.rs`.

**Branch baseline:** `claude/gracious-ptolemy-744e69` at commit `1febbe55`. Create a fresh worktree off that commit before Task 0.

**Cross-reference (informational, no longer a gating decision):** All [`GiveEffectToPermanent/*.cs`](../DCGO/Assets/Scripts/Script/CardEffectCommons/GiveEffect/GiveEffectToPermanent) helpers — `ChangeDP`, `ChangeOriginDP`, `ChangeSAttack`, `CanNotAttack`, `CanNotSuspend`, `CanNotUnsuspend`, `CanNotBlock`, `CanNotReturnToHand`, `CanNoReturnToDeck` (spelling matches the literal DCGO file `CanNoReturnToDeck.cs` — upstream typo, not a transcription error here), `CanNotBeDeletedByBattle`, `CanNotBeDeletedByEffect`, `ImmuneFromDPMinus`, `ChangeLinkMax`, `ChangePlayCost`, `StartOfMainAttack` — all consult `targetPermanent.TopCard.CanNotBeAffected(activateClass)` before installing. Option A's blanket gate covers every one of them in one place.

**Cross-engine parity caveat:** This deliberately diverges from the Python engine, which also lets opponent-sourced positive DP through. Add a row to [`docs/RUST_PYTHON_PARITY.md`](../RUST_PYTHON_PARITY.md) per Task 9.

---

## File Structure

- **Modify** `digimon-engine/src/effect_context/mod.rs` — replace Phase B negative-DP-only branches with a single `progress_excludes` short-circuit at the top of `add_modifier`; collapse `add_dp_modifier` to delegate.
- **Modify** `digimon-engine/tests/combat/progress_mutation_gates.rs` — flip the existing positive-DP sanity test, keep the existing negative-DP test, append regression tests for representative gated variants and one source-side negative control.
- **Modify** `docs/DCGO_KEYWORD_PARITY.md` — flip Progress row to 🟢, broaden § Progress fix-outline.
- **Modify** `docs/RUST_PYTHON_PARITY.md` — record the new deliberate divergence (Rust gate is broader than Python).

No new files.

---

## Task 0: Create worktree off Phase B baseline

**Files:** none (git workflow)

- [ ] **Step 1: Create the worktree**

```bash
git worktree add -b claude/progress-gate-dcgo-faithful ../progress-gate-dcgo-faithful 1febbe55
```

Expected: a new directory `../progress-gate-dcgo-faithful` checked out at commit `1febbe55` on branch `claude/progress-gate-dcgo-faithful`.

- [ ] **Step 2: Verify baseline tests pass**

From the new worktree:

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test combat -- progress_mutation_gates
```

Expected: 8 tests pass (delete, return-to-hand, return-to-deck, de_digivolve, suspend, negative-DP, positive-DP-still-applies, own-effect-still-deletes). Stop and reconcile if any fail.

---

## Task 1: Flip the Phase B positive-DP sanity test

**Files:**
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs` — the existing `opponent_effect_positive_dp_still_applies_to_progress_attacker` test (added in commit `f84b45d1`).

This is the lock-in for the DCGO-faithful direction. The test now asserts the opposite of what Phase B asserted: positive DP buffs from opponent effects DO get gated.

- [ ] **Step 1: Rewrite the test**

In `digimon-engine/tests/combat/progress_mutation_gates.rs`, replace the existing `opponent_effect_positive_dp_still_applies_to_progress_attacker` test (currently the bottom-most named test in the file at the time of `f84b45d1`) with:

```rust
#[test]
fn opponent_effect_positive_dp_does_not_apply_to_progress_attacker() {
    // DCGO-faithful: Progress.cs's SkillCondition is `IsOpponentEffect(...)` —
    // a pure source-side check. CanNotBeAffected gates regardless of sign,
    // including positive DP grants. Flipped from the Phase B precedent that
    // let positive buffs through; see plan
    // docs/superpowers/plans/2026-04-24-progress-gate-broaden-modifier-scope.md.
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_dp_modifier(progress, 1000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    let dp_sum = r.game.modifiers.sum(progress, ModifierType::ChangeDp);
    assert_eq!(
        dp_sum, 0,
        "Progress attacker must not receive opponent-effect +DP modifier; \
         got accumulated ChangeDp = {}",
        dp_sum
    );
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test combat -- opponent_effect_positive_dp_does_not_apply_to_progress_attacker
```

Expected: FAIL with `assertion `left == right` failed: ... left: 1000, right: 0`. Phase B's `add_dp_modifier` only gates when `value < 0`, so the positive value lands in the modifier registry.

(We leave it failing for now — Task 2 implements the fix.)

---

## Task 2: Replace per-variant gates with one unconditional `progress_excludes` check

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs` — both `add_dp_modifier` (lines ~913-927) and `add_modifier` (lines ~929-973) bodies.

- [ ] **Step 1: Rewrite `add_modifier`**

In `digimon-engine/src/effect_context/mod.rs`, replace the entire current `add_modifier` body (the one with the Phase B comment block "Phase B §B4: route ChangeDp through the same negative-DP Progress gate as `add_dp_modifier`...") with:

```rust
    pub fn add_modifier(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
    ) {
        // DCGO-faithful Progress gate. `progress_excludes` returns `true` iff
        // the target is the current Progress attacker, the source is the
        // opposite player, and the keyword/granted-modifier is live.
        // Equivalent to DCGO's `targetPermanent.TopCard.CanNotBeAffected(...)`
        // check that every `GiveEffectToPermanent/*.cs` helper performs.
        // Hostility-blind and sign-blind by design — see plan
        // docs/superpowers/plans/2026-04-24-progress-gate-broaden-modifier-scope.md.
        if self.game.progress_excludes(target, Some(self.player)) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(
                modifier,
                value,
                expiry,
                self.player,
            ),
        );
    }
```

- [ ] **Step 2: Rewrite `add_dp_modifier` to delegate**

In the same file, replace the `add_dp_modifier` body (currently a Phase B 6-line negative-DP gate + `modifiers.add` call) with:

```rust
    pub fn add_dp_modifier(&mut self, target: PermanentHandle, value: i32, expiry: Expiry) {
        // Single source of truth for the gate lives in `add_modifier`.
        self.add_modifier(target, ModifierType::ChangeDp, value, expiry);
    }
```

- [ ] **Step 3: Run the flipped test from Task 1 + the existing Phase B suite**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test combat -- progress_mutation_gates
```

Expected: 8/8 pass. The flipped test (Task 1) now passes because the gate fires for positive DP. Negative-DP and the other Phase B gates still pass because the same `progress_excludes` check covers them.

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "engine: gate Progress attacker against ALL opponent-sourced add_modifier calls

DCGO-faithful: CanNotAffected is hostility-blind and sign-blind. Drop the
Phase B negative-DP-only branch in favor of a single progress_excludes
short-circuit covering every ModifierType. Flip the positive-DP sanity
test to assert the new (DCGO-aligned) behavior."
```

---

## Task 3: Behavioral test for `CannotUnsuspend` (representative lockdown)

**Files:**
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs`

`CannotUnsuspend` is the highest-impact lockdown for Progress carriers — Royal Knights' Imperialdramon:PM cares specifically about not being frozen mid-attack. Pinning it down with its own test makes the regression visible if the gate ever weakens.

- [ ] **Step 1: Write the test**

Append to `digimon-engine/tests/combat/progress_mutation_gates.rs`:

```rust
#[test]
fn opponent_effect_cannot_unsuspend_does_not_freeze_progress_attacker() {
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_modifier(progress, ModifierType::CannotUnsuspend, 0, Expiry::EndOfOpponentsTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    assert!(
        !r.game.modifiers.has(progress, ModifierType::CannotUnsuspend),
        "Progress attacker must not be frozen by opponent CannotUnsuspend"
    );
}
```

- [ ] **Step 2: Run it**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test combat -- opponent_effect_cannot_unsuspend_does_not_freeze_progress_attacker
```

Expected: PASS (Task 2's blanket gate already covers it).

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "test: progress gate suppresses opponent CannotUnsuspend lockdown"
```

---

## Task 4: Behavioral tests for `CannotAttack` and `DontHaveDp`

**Files:**
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs`

Two more representative hostile variants. `CannotAttack` is the canonical attack-lockdown; `DontHaveDp` is an oblique deletion path (set DP-as-zero → loses every battle) that would bypass the Phase B `delete_permanent` gate if the modifier landed.

- [ ] **Step 1: Write the tests**

Append to `digimon-engine/tests/combat/progress_mutation_gates.rs`:

```rust
#[test]
fn opponent_effect_cannot_attack_does_not_lock_progress_attacker() {
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_modifier(progress, ModifierType::CannotAttack, 0, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    assert!(
        !r.game.modifiers.has(progress, ModifierType::CannotAttack),
        "Progress attacker must not pick up opponent-effect CannotAttack lockdown"
    );
}

#[test]
fn opponent_effect_dont_have_dp_does_not_apply_to_progress_attacker() {
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_modifier(progress, ModifierType::DontHaveDp, 0, Expiry::EndOfAttack);
    }
    r.game.set_effect_source_player_for_test(None);
    assert!(
        !r.game.modifiers.has(progress, ModifierType::DontHaveDp),
        "Progress attacker must not be DontHaveDp-clamped by opponent effect"
    );
}
```

- [ ] **Step 2: Run them**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test combat -- opponent_effect_cannot_attack opponent_effect_dont_have_dp
```

Expected: both PASS.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "test: progress gate covers CannotAttack and DontHaveDp"
```

---

## Task 5: Behavioral test for negative `ChangeBaseDp` and negative `SecurityAttackChange`

**Files:**
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs`

These are the two non-`ChangeDp` numeric-modifier variants that DCGO's `ChangeOriginDP.cs` and `ChangeSAttack.cs` install. Worth pinning because the previous (option B) plan's hostility classifier would have special-cased them; option A treats them as ordinary opponent-sourced installs.

- [ ] **Step 1: Write the tests**

Append to `digimon-engine/tests/combat/progress_mutation_gates.rs`:

```rust
#[test]
fn opponent_effect_negative_base_dp_does_not_apply_to_progress_attacker() {
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_modifier(progress, ModifierType::ChangeBaseDp, -2000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.modifiers.sum(progress, ModifierType::ChangeBaseDp),
        0,
        "Progress attacker must not receive opponent-effect ChangeBaseDp(-2000)"
    );
}

#[test]
fn opponent_effect_positive_base_dp_also_does_not_apply_to_progress_attacker() {
    // DCGO-faithful: positive base-DP grants from opponents are gated for
    // the same reason positive ChangeDp is gated — CanNotBeAffected is
    // hostility-blind.
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_modifier(progress, ModifierType::ChangeBaseDp, 1000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.modifiers.sum(progress, ModifierType::ChangeBaseDp),
        0,
        "Progress attacker must not receive opponent-effect ChangeBaseDp(+1000) either"
    );
}

#[test]
fn opponent_effect_negative_security_attack_does_not_apply_to_progress_attacker() {
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_modifier(progress, ModifierType::SecurityAttackChange, -1, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.modifiers.sum(progress, ModifierType::SecurityAttackChange),
        0,
        "Progress attacker must not receive opponent-effect SecurityAttackChange(-1)"
    );
}
```

- [ ] **Step 2: Run them**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test combat -- opponent_effect_negative_base_dp opponent_effect_positive_base_dp opponent_effect_negative_security_attack
```

Expected: 3/3 PASS.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "test: progress gate covers ChangeBaseDp (both signs) and SecurityAttackChange"
```

---

## Task 6: Behavioral test — opponent-granted protective modifier ALSO gets gated

**Files:**
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs`

This is the explicit test for "opponents wouldn't install protective modifiers, would they?" The previous plan misclassified this. Option A says: protection from an opponent's effect doesn't reach the Progress attacker either, because DCGO is source-side-only. Lock it in.

- [ ] **Step 1: Write the test**

Append to `digimon-engine/tests/combat/progress_mutation_gates.rs`:

```rust
#[test]
fn opponent_effect_protective_modifier_does_not_apply_to_progress_attacker() {
    // DCGO-faithful: even a notionally-protective modifier (e.g. global
    // "all Digimon can't be deleted by effects this turn" rider from an
    // opponent's option) does not reach the Progress attacker. The gate
    // is purely source-side per Progress.cs SkillCondition, not hostility-
    // classified. Mirrors the positive-DP test's logic.
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_modifier(
            progress,
            ModifierType::CannotBeDestroyedByEffect,
            0,
            Expiry::EndOfTurn,
        );
    }
    r.game.set_effect_source_player_for_test(None);
    assert!(
        !r.game.modifiers.has(progress, ModifierType::CannotBeDestroyedByEffect),
        "Progress gate is source-side only — opponent-granted protection doesn't pass through"
    );
}
```

- [ ] **Step 2: Run it**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test combat -- opponent_effect_protective_modifier_does_not_apply_to_progress_attacker
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "test: progress gate is source-side only (protective mods also gated)"
```

---

## Task 7: Negative-control test — own-sourced modifiers still install

**Files:**
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs`

Mirror of the Phase B `own_effect_delete_still_removes_progress_attacker`, but for the modifier path. Confirms `progress_excludes`'s own-side check (`if src == target.player { return false }`) still owns the "is this opponent-sourced?" decision after the rewrite, on both positive and hostile modifier shapes.

- [ ] **Step 1: Write the tests**

Append to `digimon-engine/tests/combat/progress_mutation_gates.rs`:

```rust
#[test]
fn own_effect_cannot_attack_still_locks_progress_attacker() {
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.add_modifier(progress, ModifierType::CannotAttack, 0, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    assert!(
        r.game.modifiers.has(progress, ModifierType::CannotAttack),
        "own-sourced CannotAttack must still install on Progress carrier"
    );
}

#[test]
fn own_effect_positive_dp_still_buffs_progress_attacker() {
    // Sanity: own buffs are not gated. progress_excludes returns false when
    // src == target.player, so own players can still buff their own
    // attacking Progress carrier mid-attack.
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.add_dp_modifier(progress, 2000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.modifiers.sum(progress, ModifierType::ChangeDp),
        2000,
        "own-sourced positive DP must still install on Progress carrier"
    );
}
```

- [ ] **Step 2: Run them**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test combat -- own_effect_cannot_attack own_effect_positive_dp
```

Expected: 2/2 PASS.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "test: own-sourced modifiers (hostile and buff) still apply to progress carrier"
```

---

## Task 8: Update `docs/DCGO_KEYWORD_PARITY.md`

**Files:**
- Modify: `docs/DCGO_KEYWORD_PARITY.md` Progress row (line 37) and § Progress fix-outline (around lines 70-79).

- [ ] **Step 1: Update the table row**

Replace the current line 37:

```
| Progress | `CanNotAffectedClass` on attacker during attack, filtered `IsOpponentEffect` + top-card-only | 🟡 (**wrong**) | Rust currently gates SecuritySkill drain instead — see §Progress below |
```

with:

```
| Progress | `CanNotAffectedClass` on attacker during attack, filtered `IsOpponentEffect` + top-card-only | 🟢 | Gated at all `ctx.*` mutation entry points; `add_modifier` short-circuits unconditionally on `progress_excludes(target, Some(self.player))` — see §Progress below |
```

- [ ] **Step 2: Append a "Phase B + Phase E prep — gate scope" subsection under § Progress**

After line 79 (the Python parity note), append:

```markdown
### Progress — gate scope (Phase B + Phase E prep)

The gate is consumed at the script-API mutation entry points in
`digimon-engine/src/effect_context/mod.rs`. As of the Phase E preparatory
broadening, the suppressed mutation set is:

- `ctx.delete_permanent`
- `ctx.return_to_hand`
- `ctx.return_to_deck`
- `ctx.de_digivolve`
- `ctx.suspend`
- `ctx.add_modifier` / `ctx.add_dp_modifier` — every `ModifierType` variant,
  every value (positive or negative), DCGO-faithful and hostility-blind.
  Mirrors DCGO's `targetPermanent.TopCard.CanNotBeAffected(activateClass)`
  check that every `GiveEffectToPermanent/*.cs` helper performs.

Out-of-scope at the gate (deliberate):

- **Player-scoped flood gates** — install on `Player`, not `Permanent`,
  and don't reach `add_modifier`. (Examples: `DrawBlock`, `MemoryBlock`,
  `CannotPlayDigimonByEffect`.)
- **Attack-target redirection** — goes through `ctx.redirect_attack`,
  not a `ModifierType`. Tracked separately for Phase E proper if a
  redirect-on-Progress-carrier interaction surfaces.
- **Rule-driven mutations** — battle damage, cost-paid trash, EOT
  expiry. `progress_excludes` returns `false` when source is `None`.

The gate's predicate is exactly DCGO's: target is the current Progress
attacker AND source is the opposite player. No hostility classification,
no sign check.
```

- [ ] **Step 3: Sanity-check the doc renders**

```bash
grep -n 'Progress' docs/DCGO_KEYWORD_PARITY.md | head -15
```

Expected: line 37 shows 🟢; the new subsection header appears below the existing § Progress.

- [ ] **Step 4: Commit**

```bash
git add docs/DCGO_KEYWORD_PARITY.md
git commit -m "docs: progress parity row → green, document DCGO-faithful gate scope"
```

---

## Task 9: Record the deliberate Python divergence in `docs/RUST_PYTHON_PARITY.md`

**Files:**
- Modify: `docs/RUST_PYTHON_PARITY.md` — add a divergence row under whichever section the file uses for keyword/effect parity.

Python's Progress implementation lets opponent-sourced positive-DP buffs and protective modifiers through (same Phase-B-style hostility-blind gap that Rust just closed). This is now a deliberate Rust-broader-than-Python divergence and needs an explicit row so the cross-engine tracker stays honest.

- [ ] **Step 1: Read the doc to find the right section**

```bash
grep -n '^##\|^###' docs/RUST_PYTHON_PARITY.md | head -30
```

Expected: a structure with section headings like "Keyword effects", "Combat", or similar. Pick the section that already covers Progress (search for `progress\|CanNotAffected` to confirm).

- [ ] **Step 2: Add the divergence row**

Append (or insert in the matching section) a row of the form the rest of the file uses. Example body if the file uses prose entries:

```markdown
### Progress gate scope (Rust broader than Python)

**Rust** (`digimon-engine/src/effect_context/mod.rs::add_modifier`):
unconditional `progress_excludes` short-circuit — every opponent-sourced
modifier against the Progress attacker is suppressed regardless of
`ModifierType` or sign. Matches DCGO's `CanNotAffected` literal.

**Python** (`digimon_gym/engine/...`): no Progress gate at the modifier
sites. Opponent-sourced positive-DP buffs and protective modifiers
land on the Progress attacker.

**Disposition:** deliberate divergence — Rust matches DCGO, Python does
not. This row retires when the Python engine is retired.
```

If the file uses a tabular row format, adapt to that shape; the substance is the same.

- [ ] **Step 3: Commit**

```bash
git add docs/RUST_PYTHON_PARITY.md
git commit -m "docs(parity): record Rust progress gate broader than Python"
```

---

## Task 10: Final regression run + PR

**Files:** none

- [ ] **Step 1: Run the relevant suites**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test combat -- progress_mutation_gates
cargo test --manifest-path digimon-engine/Cargo.toml --test engine_core
cargo test --manifest-path digimon-engine/Cargo.toml --lib -- progress
```

Expected:
- `progress_mutation_gates`: 16 tests pass — 7 Phase B carry-overs (delete, return-to-hand, return-to-deck, de_digivolve, suspend, negative-DP, own-effect-delete) + 1 flipped (now-failing-then-passing positive-DP) + 8 added (CannotUnsuspend, CannotAttack, DontHaveDp, ChangeBaseDp ±, SecurityAttackChange-, opponent-protective, own CannotAttack, own positive-DP).
- `engine_core`: pre-existing pass count unchanged.
- `lib progress` filter: `progress_excludes_only_when_attacking_and_opponent_sourced` and `opponent_sourced_mutation_only_when_effect_source_differs` still pass.

- [ ] **Step 2: Push branch and open PR**

```bash
git push -u origin claude/progress-gate-dcgo-faithful
gh pr create --base main --title "engine: make Progress gate at add_modifier DCGO-faithful (Phase E prep)" --body "$(cat <<'EOF'
## Summary
- Replaces the Phase B per-`ModifierType` Progress gate with an unconditional `progress_excludes` short-circuit at `EffectContext::add_modifier`. Every opponent-sourced modifier against the Progress attacker is now suppressed regardless of variant or sign.
- DCGO-faithful: matches `Progress.cs`'s `IsOpponentEffect` `SkillCondition` exactly. Reverts the Phase B "positive DP buffs still apply" carve-out — that test is flipped.
- Phase E prep: Retaliation / Scapegoat / Save / Decoy / Fortitude auto-installs all benefit from a complete, hostility-blind Progress gate.

## Notable behavior change
- Opponent-sourced positive DP / base-DP / security-attack buffs no longer land on the Progress attacker.
- Opponent-sourced protective modifiers (e.g. global "can't be deleted by effects" riders) also no longer land — same source-side rule.

## Cross-engine parity
- Deliberate Rust-broader-than-Python divergence; row added to `docs/RUST_PYTHON_PARITY.md`.

## Test plan
- [ ] `cargo test --manifest-path digimon-engine/Cargo.toml --test combat -- progress_mutation_gates` — 16/16 pass
- [ ] `cargo test --manifest-path digimon-engine/Cargo.toml` — full engine suite green
EOF
)"
```

Expected: PR opens, CI runs.

---

## Self-Review Checklist

**1. Spec coverage:**
- ✅ Action item 1 (enumerate variants) — handled implicitly: option A treats every variant uniformly, so no per-variant enumeration is needed for the gate. The cross-reference paragraph in the header documents the DCGO consumers for context.
- ✅ Action item 2 (cross-check DCGO) — header section + Task 8 doc updates cite `Progress.cs` and the `GiveEffectToPermanent/*.cs` family.
- ✅ Action item 3 (extend gate) — Task 2 replaces the Phase B negative-DP-only branches with a single unconditional `progress_excludes` short-circuit. `add_dp_modifier` collapses to delegate.
- ✅ Action item 4 (behavioral tests) — Tasks 1, 3-7 cover: positive DP (flipped), `CannotUnsuspend`, `CannotAttack`, `DontHaveDp`, `ChangeBaseDp` (±), `SecurityAttackChange`-, opponent-granted protective, own-sourced hostile, own-sourced buff.
- ✅ Action item 5 (update parity doc) — Task 8 (DCGO parity) + Task 9 (Python divergence).

**2. Placeholder scan:** None. All test bodies and code edits are complete.

**3. Type consistency:**
- `EffectContext::add_modifier(PermanentHandle, ModifierType, i32, Expiry)` — Task 2 step 1 matches existing signature at lines 925-931.
- `Game::progress_excludes(PermanentHandle, Option<PlayerId>) -> bool` — call site `self.game.progress_excludes(target, Some(self.player))` matches the helper at `digimon-engine/src/game.rs:931`.
- Tests use `r.game.modifiers.has(...)` and `.sum(...)` — both exist on the registry per Phase B test patterns.

---

Plan saved to `docs/superpowers/plans/2026-04-24-progress-gate-broaden-modifier-scope.md`. Two execution options:

**1. Subagent-Driven (recommended)** — fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
