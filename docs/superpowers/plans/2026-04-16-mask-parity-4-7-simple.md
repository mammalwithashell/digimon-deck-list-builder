# §4.7 Modifier-Gated Mask Checks (a/b/c slice) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring Rust's action mask to parity with Python on three modifier-gated checks — `CANNOT_ATTACK_TARGET`, `CANNOT_DIGIVOLVE`, `CANNOT_PLAY_FROM_HAND` — from `docs/RUST_PYTHON_PARITY.md` §4.7.

**Architecture:** All three gaps are single-purpose mask gates in [`digimon-engine/src/action/mask.rs`](../../../digimon-engine/src/action/mask.rs). Each check consults `ModifierRegistry` for an active modifier of the given type and suppresses the corresponding mask bit(s). Two existing `ModifierType` variants (`CannotDigivolve`, `CannotPlayFromHand`) are already in the enum; one new variant (`CannotAttackTarget`) is added. A single new helper `ModifierRegistry::any_with_type` supports the "any permanent has this modifier anywhere" pattern that Python expresses as a global modifier query.

**Scope limit — unconditional semantics:** Rust's `ModifierEntry` (`digimon-engine/src/modifiers.rs:13-19`) has no `condition` closure. Python's checks carry context — `{'attacker': ...}` for `CANNOT_ATTACK_TARGET`, `{'digivolving_card': ...}` for `CANNOT_DIGIVOLVE`. Rust's checks are **unconditional**: a single active modifier of that type blocks every attacker / every digivolution onto the target / every hand play. This covers the common card-text cases ("Digimon cannot attack this", "this Digimon cannot digivolve", "you cannot play cards") while over-restricting rare discriminants ("Red Digimon cannot attack this"). Documented as residual §4.7x.

**Tech Stack:** Rust 1.70+, `cargo test`, no new crates.

---

## File structure

**Modify:**
- `digimon-engine/src/enums.rs` — add `ModifierType::CannotAttackTarget` variant.
- `digimon-engine/src/modifiers.rs` — add `ModifierRegistry::any_with_type(&self, modifier: ModifierType) -> bool` helper for the "anywhere in the registry" query pattern.
- `digimon-engine/src/action/mask.rs` — three new gate checks: (a) attack target suppression in both `GamePhase::Main` and `GamePhase::EndOfTurnAction` arms; (b) digivolve-base suppression in the Main-phase digivolve loop; (c) hand-play suppression at the top of the Main-phase play-cards loop.
- `digimon-engine/tests/mask_main_parity.rs` — append tests for §4.7a (Main-phase), §4.7b, §4.7c.
- `digimon-engine/tests/mask_end_of_turn_parity.rs` — append one test for §4.7a's EndOfTurnAction path.
- `docs/RUST_PYTHON_PARITY.md` — flip §4.7 header to 🟡 partial; enumerate §4.7a/b/c as 🟢 (unconditional); add §4.7x for context-aware queries; reiterate §4.7d/e as outstanding. Tick §7 item 9.

**Create:** none.

---

## Task 1: Add `CannotAttackTarget` variant + `any_with_type` helper

**Files:**
- Modify: `digimon-engine/src/enums.rs`
- Modify: `digimon-engine/src/modifiers.rs`

No tests for this task — the additions are consumed by Tasks 2–4.

### Steps

- [ ] **Step 1.1: Add the enum variant**

In `digimon-engine/src/enums.rs`, locate the `ModifierType` enum (starts around line 158). Find the `// Attack` block (currently contains `CannotAttack`, `CannotAttackPlayer`, `CanAttackUnsuspended`, `CanAttackActivePlayer`). Append `CannotAttackTarget` to it:

```rust
    // Attack
    CannotAttack,
    CannotAttackPlayer,
    CanAttackUnsuspended,
    CanAttackActivePlayer,
    CannotAttackTarget,
```

- [ ] **Step 1.2: Add `any_with_type` to ModifierRegistry**

In `digimon-engine/src/modifiers.rs`, locate the existing `impl ModifierRegistry` block (starts at line 30). Add the following method next to `has` (around line 71):

```rust
    /// Returns true if any permanent in the registry has a modifier of
    /// the given type. Mirrors Python's "global modifier query" pattern
    /// — e.g. `_is_play_blocked_by_modifier` iterates all active
    /// modifiers of type `CannotPlayFromHand` without keying by target.
    pub fn any_with_type(&self, modifier: ModifierType) -> bool {
        self.permanent_modifiers
            .values()
            .any(|entries| entries.iter().any(|e| e.modifier == modifier))
    }
```

- [ ] **Step 1.3: Verify build**

Run: `cd digimon-engine && cargo check`
Expected: `Finished` with zero warnings.

- [ ] **Step 1.4: Commit**

```bash
git add digimon-engine/src/enums.rs digimon-engine/src/modifiers.rs
git commit -m "$(cat <<'EOF'
feat(modifiers): add CannotAttackTarget + any_with_type helper

Prerequisites for §4.7 modifier-gated mask checks. CannotAttackTarget
joins the Attack group in ModifierType. ModifierRegistry::any_with_type
supports the "global modifier query" pattern Python uses for
play-blocking effects.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: §4.7c CANNOT_PLAY_FROM_HAND mask check

**Context:** Python's `action_mask.py:58` calls `game._is_play_blocked_by_modifier(card)` at the top of the play-cards loop. Under the hood (`effects.py:303-311`), that's a global query: if any permanent has a `CANNOT_PLAY_CARD` modifier active, every hand card's play bit is suppressed. Rust's play loop doesn't consult this yet.

**Files:**
- Modify: `digimon-engine/src/action/mask.rs` (Main-phase play-cards loop, around lines 55-74)
- Modify: `digimon-engine/tests/mask_main_parity.rs` (append)

### Steps

- [ ] **Step 2.1: Append failing test**

At the end of `digimon-engine/tests/mask_main_parity.rs`, append:

```rust

// ─── §4.7c CANNOT_PLAY_FROM_HAND ───────────────────────────────────────

/// If any permanent has an active CannotPlayFromHand modifier, every
/// hand-play bit (0-29) is suppressed. Unconditional semantics — Rust
/// doesn't carry Python's context discriminants (§4.7x).
#[test]
fn mask_cannot_play_from_hand_suppresses_all_hand_bits() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon_level("BLOCKER", CardColor::Red, 3))
        .add_card(make_digimon_level("HAND-A", CardColor::Red, 3))
        .add_card(make_digimon_level("HAND-B", CardColor::Blue, 3))
        .hand(0, &["HAND-A", "HAND-B"])
        .start();

    let blocker = r.place_on_field(0, "BLOCKER", Some(0));
    r.game.set_memory(5);
    r.game.enter_main_phase();

    // Baseline: both hand cards are playable (affordable + Digimon kind).
    let mask_baseline = build_action_mask(&r.game, 0);
    assert_eq!(mask_baseline[0], 1.0, "HAND-A should be playable without the modifier");
    assert_eq!(mask_baseline[1], 1.0, "HAND-B should be playable without the modifier");

    // Grant CannotPlayFromHand to the blocker.
    r.game.modifiers.add(
        blocker,
        digimon_engine::modifiers::ModifierEntry {
            modifier: ModifierType::CannotPlayFromHand,
            value: 1,
            expiry: Expiry::EndOfTurn,
            source_player: 0,
        },
    );

    let mask_blocked = build_action_mask(&r.game, 0);
    assert_eq!(mask_blocked[0], 0.0, "HAND-A suppressed while modifier is active");
    assert_eq!(mask_blocked[1], 0.0, "HAND-B suppressed while modifier is active");
}
```

- [ ] **Step 2.2: Run to confirm it fails**

Run: `cd digimon-engine && cargo test --test mask_main_parity mask_cannot_play_from_hand_suppresses_all_hand_bits 2>&1 | tail -15`
Expected: FAIL with "HAND-A suppressed while modifier is active" (mask is still 1.0 because Rust doesn't consult the modifier).

- [ ] **Step 2.3: Implement the gate in mask.rs**

In `digimon-engine/src/action/mask.rs`, find the `GamePhase::Main` arm's play-cards loop (the block that starts with `// --- Play cards (0-29) ---`). Right before the `for i in 0..max_hand as usize {` loop, add a short-circuit:

```rust
            // --- Play cards (0-29) ---
            let max_hand = (me.hand.len() as u16).min(PLAY_HAND_END);
            // §4.7c CANNOT_PLAY_FROM_HAND — any active modifier of this
            // type suppresses every hand-play bit. Python uses a context
            // discriminant (card argument) that Rust doesn't carry yet
            // (§4.7x residual).
            let play_blocked = game
                .modifiers
                .any_with_type(ModifierType::CannotPlayFromHand);
            for i in 0..max_hand as usize {
                if play_blocked {
                    continue;
                }
                let card = &me.hand[i];
                // ... rest of the existing body unchanged
```

Keep the rest of the loop body exactly as it is today — the `continue` short-circuits before the memory check.

- [ ] **Step 2.4: Verify the test passes**

Run: `cd digimon-engine && cargo test --test mask_main_parity mask_cannot_play_from_hand_suppresses_all_hand_bits 2>&1 | tail -8`
Expected: PASS.

- [ ] **Step 2.5: Full mask_main_parity run (guard against regressions in §4.2-4.4-4.5 tests)**

Run: `cd digimon-engine && cargo test --test mask_main_parity 2>&1 | tail -20`
Expected: 13 tests pass (12 pre-existing + 1 new).

- [ ] **Step 2.6: Commit**

```bash
git add digimon-engine/src/action/mask.rs digimon-engine/tests/mask_main_parity.rs
git commit -m "$(cat <<'EOF'
feat(mask): suppress hand plays under CannotPlayFromHand modifier (§4.7c)

Any active CannotPlayFromHand modifier anywhere on the field zeros
every hand-play bit (0-29). Unconditional semantics — Python's
context discriminant (card argument) isn't carried, tracked as §4.7x.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: §4.7b CANNOT_DIGIVOLVE mask check

**Context:** Python's `action_mask.py:151-153` consults `game.modifiers.has_modifier(base_perm, ModifierType.CANNOT_DIGIVOLVE, {'digivolving_card': card})` before emitting each digivolve bit. Rust's digivolve loop only checks color/level via `can_basic_digivolve`. The Rust version will ignore the `digivolving_card` context (§4.7x) and block all digivolutions onto any base permanent that has any `CannotDigivolve` modifier.

**Files:**
- Modify: `digimon-engine/src/action/mask.rs` (Main-phase digivolve loop)
- Modify: `digimon-engine/tests/mask_main_parity.rs` (append)

### Steps

- [ ] **Step 3.1: Append failing test**

At the end of `digimon-engine/tests/mask_main_parity.rs`, append:

```rust

// ─── §4.7b CANNOT_DIGIVOLVE ────────────────────────────────────────────

/// A permanent with an active CannotDigivolve modifier must not have any
/// digivolve bit emitted, regardless of evo-cost match.
#[test]
fn mask_cannot_digivolve_suppresses_digivolve_bits_on_base() {
    use digimon_engine::action::encode_digivolve;

    // Hand card: Red Lv4 with Red Lv3 evo_cost.
    let mut evo_card = make_digimon_level("EVO-RED", CardColor::Red, 4);
    evo_card.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 0, // Red
        level: 3,
        memory_cost: 1,
    }];

    let base_card = make_digimon_level("BASE-RED", CardColor::Red, 3);

    let mut r = DebugRunner::builder()
        .add_card(evo_card)
        .add_card(base_card)
        .hand(0, &["EVO-RED"])
        .start();

    let base = r.place_on_field(0, "BASE-RED", Some(0));
    r.game.set_memory(5);
    r.game.enter_main_phase();

    // Baseline: evo bit to base[0] is emitted (evo_cost matches).
    let baseline = build_action_mask(&r.game, 0);
    let evo_bit = encode_digivolve(0 as u16, base.index as u16) as usize;
    assert_eq!(baseline[evo_bit], 1.0, "baseline digivolve bit should be set");

    // Grant CannotDigivolve to the base.
    r.game.modifiers.add(
        base,
        digimon_engine::modifiers::ModifierEntry {
            modifier: ModifierType::CannotDigivolve,
            value: 1,
            expiry: Expiry::EndOfTurn,
            source_player: 0,
        },
    );

    let blocked = build_action_mask(&r.game, 0);
    assert_eq!(
        blocked[evo_bit], 0.0,
        "CannotDigivolve on base must suppress the digivolve bit onto it",
    );
}
```

- [ ] **Step 3.2: Run to confirm it fails**

Run: `cd digimon-engine && cargo test --test mask_main_parity mask_cannot_digivolve_suppresses_digivolve_bits_on_base 2>&1 | tail -10`
Expected: FAIL with "CannotDigivolve on base must suppress the digivolve bit onto it".

- [ ] **Step 3.3: Implement the gate in mask.rs**

In `digimon-engine/src/action/mask.rs`, find the `GamePhase::Main` arm's digivolve loop (the section that begins with `// --- Digivolve (400-999) ---`). Inside the `for f in 0..max_field` inner loop, add a modifier check before `can_basic_digivolve`:

```rust
                let max_field = me.battle_area.len().min(FIELD_SLOTS);
                for f in 0..max_field {
                    let base_perm = &me.battle_area[f];
                    let base_handle = PermanentHandle { player: player_id, index: f as u8 };
                    // §4.7b CANNOT_DIGIVOLVE — suppress the bit if the base
                    // permanent carries an active CannotDigivolve modifier.
                    // Python's `{'digivolving_card': card}` discriminant is
                    // not carried in Rust (§4.7x).
                    if game
                        .modifiers
                        .has(base_handle, ModifierType::CannotDigivolve)
                    {
                        continue;
                    }
                    if can_basic_digivolve(card, base_perm, &game.card_data) {
                        mask[encode_digivolve(h as u16, f as u16) as usize] = 1.0;
                    }
                }
```

The breeding-area branch below (`if let Some(ref breeding) = me.breeding_area`) does not need the gate — `breeding_area` is not a `Permanent` with a `PermanentHandle`, and no modifier can target it. Leave that branch unchanged.

- [ ] **Step 3.4: Verify the test passes**

Run: `cd digimon-engine && cargo test --test mask_main_parity mask_cannot_digivolve_suppresses_digivolve_bits_on_base 2>&1 | tail -8`
Expected: PASS.

- [ ] **Step 3.5: Full mask_main_parity run**

Run: `cd digimon-engine && cargo test --test mask_main_parity 2>&1 | tail -20`
Expected: 14 tests pass.

- [ ] **Step 3.6: Commit**

```bash
git add digimon-engine/src/action/mask.rs digimon-engine/tests/mask_main_parity.rs
git commit -m "$(cat <<'EOF'
feat(mask): suppress digivolve bits under CannotDigivolve modifier (§4.7b)

Any active CannotDigivolve modifier on a base permanent zeros the
digivolve bits pointing to that base. Unconditional semantics — the
digivolving_card discriminant from Python is tracked as §4.7x.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: §4.7a CANNOT_ATTACK_TARGET mask check (Main + EndOfTurnAction)

**Context:** Python's `action_mask.py` consults `CANNOT_ATTACK_TARGET` before emitting any attack bit against a permanent target — Main-phase (line 129-136), Vortex block (348-351), MAY_ATTACK block (371-374), Force-Attack block (385-388). Rust needs the same gate in both its Main-phase attack loop and its `EndOfTurnAction` (Vortex) arm. Per-attacker context is dropped (§4.7x).

Security-target attacks (`SECURITY_TARGET`) are not affected — `CANNOT_ATTACK_TARGET` refers to permanent-vs-permanent attack targeting. The security bit is controlled by `CANNOT_ATTACK_PLAYER` (already wired in `can_attack`, per the §2.1 and §4.3 work).

**Files:**
- Modify: `digimon-engine/src/action/mask.rs` (Main-phase attack loop AND EndOfTurnAction arm)
- Modify: `digimon-engine/tests/mask_main_parity.rs` (append Main-phase test)
- Modify: `digimon-engine/tests/mask_end_of_turn_parity.rs` (append Vortex test)

### Steps

- [ ] **Step 4.1: Append failing Main-phase test**

At the end of `digimon-engine/tests/mask_main_parity.rs`, append:

```rust

// ─── §4.7a CANNOT_ATTACK_TARGET ────────────────────────────────────────

/// An enemy permanent with CannotAttackTarget active must not have any
/// attacker-vs-target bit emitted. Security-attack bit is orthogonal.
#[test]
fn mask_cannot_attack_target_suppresses_digimon_attack_bit() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon_level("ATK", CardColor::Red, 4))
        .add_card(make_digimon_level("DEF", CardColor::Blue, 3))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;

    r.game.set_memory(3);
    r.game.enter_main_phase();

    // Baseline: attack bit to defender is emitted (suspended).
    let baseline = build_action_mask(&r.game, tp);
    let atk_bit = encode_attack(attacker.index as u16, defender.index as u16) as usize;
    let sec_bit = encode_attack(attacker.index as u16, SECURITY_TARGET) as usize;
    assert_eq!(baseline[atk_bit], 1.0, "baseline: suspended defender is attackable");
    assert_eq!(baseline[sec_bit], 1.0, "baseline: security is attackable");

    // Grant CannotAttackTarget to defender.
    r.game.modifiers.add(
        defender,
        digimon_engine::modifiers::ModifierEntry {
            modifier: ModifierType::CannotAttackTarget,
            value: 1,
            expiry: Expiry::EndOfTurn,
            source_player: opp,
        },
    );

    let blocked = build_action_mask(&r.game, tp);
    assert_eq!(
        blocked[atk_bit], 0.0,
        "CannotAttackTarget on defender must suppress the digimon-attack bit",
    );
    assert_eq!(
        blocked[sec_bit], 1.0,
        "security attack bit is orthogonal to CannotAttackTarget",
    );
}
```

- [ ] **Step 4.2: Append failing EndOfTurnAction test**

At the end of `digimon-engine/tests/mask_end_of_turn_parity.rs`, append:

```rust

/// Vortex attack bits (EndOfTurnAction arm) must also honor
/// CannotAttackTarget on the enemy permanent.
#[test]
fn mask_vortex_respects_cannot_attack_target() {
    use digimon_engine::enums::ModifierType;

    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));

    r.game.modifiers.grant_keyword(
        attacker, Keyword::Vortex, Expiry::EndOfTurn, tp,
    );
    r.game.modifiers.add(
        defender,
        digimon_engine::modifiers::ModifierEntry {
            modifier: ModifierType::CannotAttackTarget,
            value: 1,
            expiry: Expiry::EndOfTurn,
            source_player: opp,
        },
    );
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize], 0.0,
        "Vortex must also honor CannotAttackTarget",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize], 1.0,
        "Vortex security attack is unaffected by CannotAttackTarget",
    );
}
```

- [ ] **Step 4.3: Run both tests to confirm they fail**

Run: `cd digimon-engine && cargo test --test mask_main_parity mask_cannot_attack_target_suppresses_digimon_attack_bit 2>&1 | tail -10`
Expected: FAIL ("CannotAttackTarget on defender must suppress...").

Run: `cd digimon-engine && cargo test --test mask_end_of_turn_parity mask_vortex_respects_cannot_attack_target 2>&1 | tail -10`
Expected: FAIL ("Vortex must also honor CannotAttackTarget").

- [ ] **Step 4.4: Implement the gate in mask.rs Main-phase**

In `digimon-engine/src/action/mask.rs`, find the Main-phase attack loop's Digimon-attack target inner loop (inside the `for j in 0..max_opp` body, post §4.4 Raid work). Add a `CannotAttackTarget` check at the top of the loop body, before the existing target filtering:

```rust
                for j in 0..max_opp {
                    let target = &opp.battle_area[j];
                    if !target.is_digimon(&game.card_data) {
                        continue;
                    }
                    let t_handle = PermanentHandle {
                        player: opp_id,
                        index: j as u8,
                    };
                    // §4.7a CANNOT_ATTACK_TARGET — suppress this target if
                    // it carries the modifier. Per-attacker discriminant
                    // from Python is §4.7x.
                    if game
                        .modifiers
                        .has(t_handle, ModifierType::CannotAttackTarget)
                    {
                        continue;
                    }
                    let action_bit = encode_attack(i as u16, j as u16) as usize;
                    // ... rest of the existing body unchanged (suspended /
                    // CAN_ATTACK_UNSUSPENDED / Raid logic)
                }
```

Note: one `t_handle` binding now serves both the new gate and the pre-existing Raid DP lookup that constructs `t_handle` inside the conditional branch. Remove the redundant inner `let t_handle = PermanentHandle { ... }` inside the `if let Some(max_dp) = raid_max_dp` block — reuse the one defined at the top of the iteration.

- [ ] **Step 4.5: Implement the gate in mask.rs EndOfTurnAction**

In the same file, find the `GamePhase::EndOfTurnAction` arm's inner target loop (from Task §4.6a). Add the same check:

```rust
                let max_opp = opp.battle_area.len().min(FIELD_SLOTS);
                for j in 0..max_opp {
                    let target = &opp.battle_area[j];
                    if !target.is_digimon(&game.card_data) {
                        continue;
                    }
                    let t_handle = PermanentHandle {
                        player: opp_id,
                        index: j as u8,
                    };
                    if game
                        .modifiers
                        .has(t_handle, ModifierType::CannotAttackTarget)
                    {
                        continue;
                    }
                    mask[encode_attack(i as u16, j as u16) as usize] = 1.0;
                }
```

- [ ] **Step 4.6: Verify both tests pass**

Run: `cd digimon-engine && cargo test --test mask_main_parity mask_cannot_attack_target 2>&1 | tail -5`
Expected: PASS.

Run: `cd digimon-engine && cargo test --test mask_end_of_turn_parity mask_vortex_respects_cannot_attack_target 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 4.7: Full regression**

Run: `cd digimon-engine && cargo test -p digimon-engine 2>&1 | grep "test result"`
Expected: every `test result` line shows `ok ... 0 failed`.

- [ ] **Step 4.8: Commit**

```bash
git add digimon-engine/src/action/mask.rs digimon-engine/tests/mask_main_parity.rs digimon-engine/tests/mask_end_of_turn_parity.rs
git commit -m "$(cat <<'EOF'
feat(mask): suppress attacks via CannotAttackTarget modifier (§4.7a)

Both Main-phase Digimon-attack bits and EndOfTurnAction Vortex attack
bits now consult ModifierType::CannotAttackTarget on the opponent
permanent and suppress the bit if active. Security-attack bits are
orthogonal (gated by CannotAttackPlayer via can_attack). Per-attacker
context discriminant from Python is tracked as §4.7x.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Doc update

**Files:**
- Modify: `docs/RUST_PYTHON_PARITY.md`

### Steps

- [ ] **Step 5.1: Restructure §4.7 into graded sub-items**

Find the current §4.7 block in `docs/RUST_PYTHON_PARITY.md`:

```markdown
### 4.7 🟡 Modifier-gated mask checks

Python checks these modifiers per-action; Rust does not:
- `CANNOT_ATTACK_TARGET` (per attacker-target pair)
- `CANNOT_DIGIVOLVE`
- `CANNOT_PLAY_FROM_HAND`
- `FORCE_ATTACK` (restricts mask to forced Digimon only)
- `DigiXros` cost-reduction optimistic calculation
```

Replace with:

```markdown
### 4.7 🟡 Modifier-gated mask checks — partial

Three of the five checks landed with unconditional semantics; the other two (§4.7d/e) and per-action context discriminants (§4.7x) remain future work.

### 4.7a 🟢 CannotAttackTarget — implemented

**Python** — [action_mask.py:129-136](../digimon_gym/engine/game/action_mask.py#L129): `has_modifier(target, CANNOT_ATTACK_TARGET, {'attacker': attacker})` gates each Digimon-attack bit; same check repeats in Vortex / MAY_ATTACK / FORCE_ATTACK arms.

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) Main-phase Digimon-attack inner loop + `GamePhase::EndOfTurnAction` arm call `modifiers.has(t_handle, ModifierType::CannotAttackTarget)` and skip the target. Per-attacker discriminant is dropped — see §4.7x.

**Coverage:** `mask_cannot_attack_target_suppresses_digimon_attack_bit` in [tests/mask_main_parity.rs](../digimon-engine/tests/mask_main_parity.rs); `mask_vortex_respects_cannot_attack_target` in [tests/mask_end_of_turn_parity.rs](../digimon-engine/tests/mask_end_of_turn_parity.rs).

### 4.7b 🟢 CannotDigivolve — implemented

**Python** — [action_mask.py:151-153](../digimon_gym/engine/game/action_mask.py#L151): `has_modifier(base_perm, CANNOT_DIGIVOLVE, {'digivolving_card': card})`.

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) Main-phase digivolve loop checks `modifiers.has(base_handle, ModifierType::CannotDigivolve)` before `can_basic_digivolve`. `digivolving_card` discriminant dropped (§4.7x).

**Coverage:** `mask_cannot_digivolve_suppresses_digivolve_bits_on_base` in [tests/mask_main_parity.rs](../digimon-engine/tests/mask_main_parity.rs).

### 4.7c 🟢 CannotPlayFromHand — implemented

**Python** — [action_mask.py:58](../digimon_gym/engine/game/action_mask.py#L58) → `_is_play_blocked_by_modifier(card)` (effects.py:303-311).

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) Main-phase play-cards loop short-circuits when `modifiers.any_with_type(ModifierType::CannotPlayFromHand)` is true.

**Coverage:** `mask_cannot_play_from_hand_suppresses_all_hand_bits` in [tests/mask_main_parity.rs](../digimon-engine/tests/mask_main_parity.rs).

### 4.7d 🔴 FORCE_ATTACK — outstanding

Python's Main-phase builder (`action_mask.py:227-280`) does a global mask-replacement: if any friendly Digimon has `FORCE_ATTACK`, every other legal action is zeroed and only attacks by forced Digimon remain. Requires a new `ModifierType::ForceAttack` variant plus a second mask-replacement pass after the normal build. Own plan.

### 4.7e 🔴 DigiXros cost-reduction — outstanding

Python's play-cost check (`action_mask.py:66-72`) computes `effective_cost = max(0, play_cost - max_reduction)` for cards with `digixros_cost`. Blocked on `CardData.digixros_cost` schema + `has_any_digixros_material` validator + ingest-pipeline data (same data-population shape as §4.5b). Own plan.

### 4.7x 🟡 Context-aware modifier queries — outstanding

Python's `has_modifier(target, type, context)` can refine the match via the modifier's `condition` closure — e.g. `CannotAttackTarget` that applies only to Red attackers, or `CannotDigivolve` that applies only when digivolving into a specific card. Rust's `ModifierEntry` ([modifiers.rs:13-19](../digimon-engine/src/modifiers.rs)) has no condition closure, so §4.7a and §4.7b are unconditional (any active modifier blocks regardless of the attacker/digivolving_card discriminant). Adding condition closures is an architectural change worthy of its own plan.
```

- [ ] **Step 5.2: Tick §7 item 9**

Find the existing `§7` bullet:

```markdown
9. **§4.7 — Modifier-gated mask checks** (CannotAttackTarget, CannotDigivolve, CannotPlayFromHand, FORCE_ATTACK, DigiXros optimistic cost reduction).
```

Replace with:

```markdown
9. **§4.7 — Modifier-gated mask checks** — partial. ✅ §4.7a CannotAttackTarget, §4.7b CannotDigivolve, §4.7c CannotPlayFromHand (unconditional semantics). Outstanding: §4.7d FORCE_ATTACK (own plan), §4.7e DigiXros cost-reduction (own plan; also blocked on data-population like §4.5b), §4.7x context-aware modifier queries (architectural).
```

If §7 doesn't currently have an item 9 for §4.7, add this bullet after the current last item. (Existing numbering ends at item 8 per recent §4.5/§4.6 slice doc work.)

- [ ] **Step 5.3: Commit**

```bash
git add docs/RUST_PYTHON_PARITY.md
git commit -m "$(cat <<'EOF'
docs: flip §4.7a/b/c to done; add §4.7d/e/x residuals

Three modifier-gated mask checks landed with unconditional semantics
(CannotAttackTarget, CannotDigivolve, CannotPlayFromHand). The rest
(FORCE_ATTACK, DigiXros, context-aware modifier queries) are carved
into separate residual entries.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Final verification

**Files:** none — verification only.

### Steps

- [ ] **Step 6.1: Full Rust regression**

Run: `cd digimon-engine && cargo test -p digimon-engine 2>&1 | grep "test result" | awk '{sum+=$4} END {print "Total passed:", sum}'`
Expected: `Total passed: 162` (159 pre-batch + 3 new: Main-phase §4.7c + Main-phase §4.7b + Main-phase §4.7a). Vortex §4.7a test adds one more in `mask_end_of_turn_parity`, so the count may be 163 — confirm the delta is exactly +4.

- [ ] **Step 6.2: Check no Tauri drift**

Run: `grep -E "attack_digimon|attack_player|can_attack" src-tauri/src/engine_commands.rs`
Expected: all three calls still pass the `false` vortex arg (unchanged by this batch).

- [ ] **Step 6.3: Python regression (unchanged)**

Run: `python -m pytest tests/engine -k "rush or attack or summon or option or dna" -q 2>&1 | tail -3`
Expected: same count as before this batch (84 passed / 386 deselected per the §4.5/§4.6 slice run). No Python code was touched.

---

## Self-Review checklist

**Spec coverage:**
- §4.7a → Task 4 (Main + EndOfTurnAction).
- §4.7b → Task 3.
- §4.7c → Task 2.
- §4.7d → explicitly deferred, documented in Task 5.
- §4.7e → explicitly deferred, documented in Task 5.
- §4.7x (context-aware queries) → net-new residual introduced by the plan; documented in Task 5.

**Placeholder scan:** Every code block is complete; no "handle edge cases" / "add validation" / "TODO". Test bodies assert concrete values against concrete expected outputs. Commit messages written out.

**Type consistency:** `ModifierType::CannotAttackTarget`, `ModifierType::CannotDigivolve`, `ModifierType::CannotPlayFromHand` used consistently. `ModifierRegistry::has(handle, ModifierType)` for single-target checks, `ModifierRegistry::any_with_type(ModifierType)` for the global check. `PermanentHandle { player, index: u8 }` constructed identically in every test. `encode_attack`, `encode_digivolve`, `SECURITY_TARGET` imported consistently.

**Assumptions about prior state that should still hold:**
- `make_digimon_level`, `make_digimon`, `make_digimon_dp` helpers exist in `mask_main_parity.rs` from §4.4 / §4.5 work — verify before Task 3/4 that those factories are accessible.
- `DebugRunner` exposes `game.current_phase` as `pub` for direct assignment (used in Task 4's EndOfTurnAction test).
- `ModifierRegistry::add` takes `(handle, ModifierEntry)` per §4.4 usage — matches the tests above.
