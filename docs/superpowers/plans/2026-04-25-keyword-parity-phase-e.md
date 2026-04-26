# Keyword Parity Phase E — Missing-from-Enum Backfill

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the four Phase E keyword backfills from the [keyword parity design spec](../specs/2026-04-24-dcgo-keyword-parity-design.md) §5 — `Retaliation`, `Scapegoat`, `DeDigivolve(N)` printed-form auto-install, and `DrawX(N)` printed-form auto-install — so that cards declaring only those printed keywords need zero hand-rolled `CardEffect` code.

**Architecture:** Two new `Keyword` enum variants (`Retaliation`, `Scapegoat`); one new `EffectContext` accessor (`battle_opponent_of`); four new arms in `cards/keyword_effects.rs::keyword_to_auto_effect`; two new entries in `card_data.rs::parse_printed_keywords`. Phase D's substrate (parked replacement, `pending_post_deletion_replays`, `was_deleted_by_effect`, `MainOnField` active-skill emission) is reused as-is — no new substrate. Tests land under `digimon-engine/tests/keyword_phase_e/` mirroring Phase D's per-keyword module layout.

**Tech Stack:** Rust 2021 edition; `digimon-engine` library crate; `cargo test --manifest-path digimon-engine/Cargo.toml` for the test loop; behavioral tests via `DebugRunner`.

**Reference docs to keep open:**
- Spec: [docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md](../specs/2026-04-24-dcgo-keyword-parity-design.md) §5 Phase E + §6 + §7 + §10
- Parity doc: [docs/DCGO_KEYWORD_PARITY.md](../../DCGO_KEYWORD_PARITY.md)
- Rules manual: [docs/RULES_CONTEXT.md](../../RULES_CONTEXT.md) §16 (Retaliation, Scapegoat)
- DCGO sources: `DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/Retaliation.cs`, `Scapegoat.cs`
- Phase D analogues: `digimon-engine/src/cards/keyword_effects.rs:614` (Fortitude — OnDeletion, mandatory), `:512` (Decoy — WhenWouldBeDeleted, optional, substitute), `:839` (MaterialSave — MainOnField active skill)

**Working directory:** This plan should be executed in the existing `claude/vigorous-elgamal-453703` worktree (already on `main` and clean as of session start).

---

## File Structure

**Create:**
- `digimon-engine/tests/keyword_phase_e/main.rs` — module index
- `digimon-engine/tests/keyword_phase_e/helpers.rs` — shared `plain_digimon` / `plain_option` helpers
- `digimon-engine/tests/keyword_phase_e/retaliation.rs` — E1 tests
- `digimon-engine/tests/keyword_phase_e/scapegoat.rs` — E2 tests
- `digimon-engine/tests/keyword_phase_e/de_digivolve_n.rs` — E3 tests
- `digimon-engine/tests/keyword_phase_e/draw_x_n.rs` — E4 tests

**Modify:**
- `digimon-engine/src/enums.rs:273-314` — add `Retaliation` and `Scapegoat` variants
- `digimon-engine/src/card_data.rs:297-319` — add parser entries for `"Retaliation"` and `"Scapegoat"`
- `digimon-engine/src/effect_context/mod.rs` — add `battle_opponent_of` accessor on both `EffectReadContext` and `EffectContext`
- `digimon-engine/src/cards/keyword_effects.rs` — add four match arms (Retaliation, Scapegoat, DeDigivolve, DrawX); update module docstring
- `digimon-engine/Cargo.toml` — add `[[test]] name = "keyword_phase_e"` entry (verify pattern matches the existing `keyword_phase_d` entry)
- `docs/DCGO_KEYWORD_PARITY.md` — flip rows for Retaliation, Scapegoat, DeDigivolve(N), DrawX(N) to ✅; mark Phase E gap items resolved in §"Gap ranking"

**No changes to:** `replacement.rs`, `combat.rs`, `game.rs` (substrate is reused as-is). Phase E adds NO new substrate — this is purely consumer-side wiring. If the executor finds that Retaliation needs information not exposed today, they should escalate before adding a new game-level field.

---

## Task 1: Bootstrap Phase E test crate scaffold

Mirrors the Phase D structure exactly so test patterns transfer 1:1.

**Files:**
- Create: `digimon-engine/tests/keyword_phase_e/main.rs`
- Create: `digimon-engine/tests/keyword_phase_e/helpers.rs`
- Modify: `digimon-engine/Cargo.toml` (add `[[test]]` entry)

- [ ] **Step 1.1: Inspect Phase D's `[[test]]` entry to mirror it.**

```bash
grep -A2 'name = "keyword_phase_d"' digimon-engine/Cargo.toml
```

Expected: a `[[test]]` block naming `keyword_phase_d` with a `path = "tests/keyword_phase_d/main.rs"` line. If `path` is not present, the convention may rely on cargo's default test discovery — in that case skip the Cargo.toml edit in step 1.4 and just create the directory.

- [ ] **Step 1.2: Create `tests/keyword_phase_e/main.rs`.**

```rust
mod helpers;
mod retaliation;
mod scapegoat;
mod de_digivolve_n;
mod draw_x_n;
```

Note: keep all four `mod` lines — even though we won't have all the test files written until later tasks, the unused-mod warning won't fire because the test files will exist as empty modules (created in Task 1.3 below).

- [ ] **Step 1.3: Create empty test files so the `mod` declarations resolve.**

Create four empty files:
- `digimon-engine/tests/keyword_phase_e/retaliation.rs` (empty)
- `digimon-engine/tests/keyword_phase_e/scapegoat.rs` (empty)
- `digimon-engine/tests/keyword_phase_e/de_digivolve_n.rs` (empty)
- `digimon-engine/tests/keyword_phase_e/draw_x_n.rs` (empty)

- [ ] **Step 1.4: Create `tests/keyword_phase_e/helpers.rs`.**

```rust
//! Shared fixtures for Phase E behavioral tests. Mirrors
//! `tests/keyword_phase_d/helpers.rs`'s `plain_digimon` builder; adds a
//! `plain_option` builder for E4 (DrawX) Option-card tests.

use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

pub fn plain_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

pub fn digimon_with_keywords(id: &str, level: u8, dp: u16, kws: Vec<Keyword>) -> CardData {
    let mut c = plain_digimon(id);
    c.level = Some(level);
    c.dp = Some(dp);
    c.keywords = kws;
    c
}
```

The `plain_option` builder will be added in Task 6 once we know its CardData shape.

- [ ] **Step 1.5: Add `[[test]]` entry to `digimon-engine/Cargo.toml`** (only if Phase D has an explicit entry — verified in Step 1.1).

If needed, add right after the `keyword_phase_d` block:
```toml
[[test]]
name = "keyword_phase_e"
path = "tests/keyword_phase_e/main.rs"
```

- [ ] **Step 1.6: Verify the empty harness compiles and runs.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e
```

Expected: `test result: ok. 0 passed; 0 failed; 0 ignored`. If the binary doesn't resolve, re-check Step 1.5 against Phase D's pattern.

- [ ] **Step 1.7: Commit.**

```bash
git add digimon-engine/tests/keyword_phase_e/ digimon-engine/Cargo.toml
git commit -m "test(engine): scaffold keyword_phase_e test crate

Mirror keyword_phase_d's per-keyword module layout. Empty test files
land here so subsequent Phase E tasks (E1-E4) can drop in tests one
keyword at a time."
```

---

## Task 2: Add `Retaliation` and `Scapegoat` enum variants + parser entries

Pure data plumbing. No behavior change yet — all auto-install arms still fall through `_ => Vec::new()`. Verifies the variants compile and parse before we touch behavior.

**Files:**
- Modify: `digimon-engine/src/enums.rs:313-314`
- Modify: `digimon-engine/src/card_data.rs:297-319`

- [ ] **Step 2.1: Write a failing parser test.**

Append to the `mod tests` block in `digimon-engine/src/card_data.rs` (around line 405):

```rust
#[test]
fn parse_retaliation_and_scapegoat() {
    use crate::enums::Keyword;
    let kws = parse_printed_keywords(
        "＜Retaliation＞ ＜Scapegoat＞",
        "",
        "",
    );
    assert!(kws.contains(&Keyword::Retaliation), "should parse <Retaliation>");
    assert!(kws.contains(&Keyword::Scapegoat), "should parse <Scapegoat>");
}
```

- [ ] **Step 2.2: Run test to confirm failure.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --lib parse_retaliation_and_scapegoat
```

Expected: compile error — `no variant named Retaliation found for enum Keyword`.

- [ ] **Step 2.3: Add the enum variants.**

In `digimon-engine/src/enums.rs`, after `Progress,` on line 313 and before the closing `}` on line 314:

```rust
    Progress,

    /// DCGO `Retaliation` — when this Digimon is deleted other than by an
    /// effect (Battle, SecurityCheck, Cost), delete the opposing combatant.
    /// Wire-up Phase E §E1 — auto-installed `OnDeletion` trigger that
    /// reads `ctx.battle_opponent_of(self)` to find the winner.
    /// RULES_CONTEXT 16-30 // (verify exact rule number against the manual).
    Retaliation,

    /// DCGO `Scapegoat` — when this Digimon would be deleted by anything
    /// other than its own controller's effect, optionally delete another of
    /// the controller's permanents to cancel the deletion. Wire-up Phase E
    /// §E2 — auto-installed `WhenWouldBeDeleted` substitute replacement.
    /// RULES_CONTEXT 16-31.
    Scapegoat,
}
```

- [ ] **Step 2.4: Add parser entries.**

In `digimon-engine/src/card_data.rs`, extend the non-parametric prefix table (around line 297-319). Both keywords are non-parametric, so add to the existing tuple slice (order matters per the longest-prefix rule — `Retaliation` and `Scapegoat` have no prefix collisions with existing entries, so position doesn't strictly matter):

```rust
                ("Vortex", Keyword::Vortex),
                ("Collision", Keyword::Collision),
                ("Progress", Keyword::Progress),
                ("Retaliation", Keyword::Retaliation),
                ("Scapegoat", Keyword::Scapegoat),
            ] {
```

- [ ] **Step 2.5: Run the parser test to confirm pass.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --lib parse_retaliation_and_scapegoat
```

Expected: PASS.

- [ ] **Step 2.6: Run the full lib test suite to confirm nothing else broke.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --lib
```

Expected: all green. Compiler may warn about unreachable `_ =>` arms in some pattern matches over `Keyword` — fix any non-exhaustive matches by adding the new variants. (`keyword_to_auto_effect` already has a `_ =>` catch-all so it's fine; the more-likely complainers are `combat.rs::has_keyword`-style matches and the action-mask emission. Run `cargo build --manifest-path digimon-engine/Cargo.toml` to flush all warnings before committing.)

- [ ] **Step 2.7: Commit.**

```bash
git add digimon-engine/src/enums.rs digimon-engine/src/card_data.rs
git commit -m "engine(keyword): add Retaliation + Scapegoat enum variants

Phase E §E1/E2 enum + parser plumbing. No behavior wired yet — both
variants fall through keyword_to_auto_effect's _ => Vec::new() catch-all.
RULES_CONTEXT §16-30 / §16-31."
```

---

## Task 3: Add `EffectContext::battle_opponent_of` accessor

Retaliation needs to identify the battle winner from inside an `OnDeletion` handler. `Game.pending_attack` is live at that point (`combat.rs:2138-2191` shows `delete_permanent_with_cause` is called inside `resolve_battle` while `pending_attack` is still `Some`). We expose this through a focused accessor instead of leaking the full `PendingAttack` struct.

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs` (find by grep — likely two spots, one on `EffectReadContext` near `was_deleted_by_effect`, one on `EffectContext`)

- [ ] **Step 3.1: Locate the existing `was_deleted_by_effect` definitions.**

```bash
grep -n "was_deleted_by_effect\|was_deleted_by_opponent\|deletion_cause" digimon-engine/src/effect_context/mod.rs
```

Expected: ~6 matches. The two relevant ones are the `pub fn` definitions on lines ~162 and ~336 (one per context). Note the line numbers — the next step inserts directly below `was_deleted_by_opponent` in each.

- [ ] **Step 3.2: Write a failing unit test.**

Append to the `mod tests` block at the end of `digimon-engine/src/effect_context/mod.rs`. If there's no test mod, find an existing test file under `digimon-engine/tests/effect_context/` and add it there. Use this body:

```rust
#[test]
fn battle_opponent_of_returns_attacker_when_self_is_defender() {
    use crate::debug_runner::DebugRunner;
    use crate::card_data::CardData;
    use crate::enums::{CardColor, CardKind};

    fn d(id: &str) -> CardData {
        CardData {
            card_id: id.to_string(),
            card_name: id.to_string(),
            card_kind: CardKind::Digimon,
            level: Some(4),
            dp: Some(4000),
            play_cost: 4,
            colors: vec![CardColor::Red],
            traits: Vec::new(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            keywords: Vec::new(),
            effect_class_name: id.replace('-', "_"),
            index: 0,
            norm_id: 0.0,
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(d("ATK"))
        .add_card(d("DEF"))
        .start();
    let atk = r.place_on_field(0, "ATK", None);
    let def = r.place_on_field(1, "DEF", None);
    // Set up a pending_attack manually for the test (skip the full attack
    // sequence). Use the engine's existing helper if one exists; otherwise
    // construct PendingAttack directly.
    // …
    // Once pending_attack is set: build an EffectReadContext for player 1
    // (defender) and call ctx.battle_opponent_of(def) — expect Some(atk).
}
```

> **Note for the executor.** The exact construction of `PendingAttack` for the test setup depends on whether there's a public test helper (`r.set_pending_attack(atk, def)` or similar). Check `digimon-engine/src/debug_runner.rs` for an existing helper. If none exists, the cleaner route is to drive a real attack via `r.action(...)` / `r.game.declare_attack(...)` until `pending_attack` is set, then read the accessor. Don't add a new public mutator just for the test — use the existing battle flow.

- [ ] **Step 3.3: Run test to confirm failure.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml battle_opponent_of_returns_attacker_when_self_is_defender
```

Expected: compile error — method not found.

- [ ] **Step 3.4: Implement `battle_opponent_of` on `EffectReadContext`.**

In `digimon-engine/src/effect_context/mod.rs`, directly after `was_deleted_by_opponent` on `EffectReadContext` (~line 176):

```rust
    /// Identify the opposing combatant in the currently-resolving battle.
    ///
    /// Returns `Some(opponent_handle)` when `Game.pending_attack` is live
    /// AND the supplied `self_handle` matches one side of the battle:
    ///   - `self_handle == attacker` → returns the defender
    ///   - `self_handle == effective_target.as_digimon()` → returns the attacker
    ///   - otherwise (no pending battle, or self is not a combatant) → `None`
    ///
    /// Used by Retaliation (Phase E §E1) to identify the battle winner from
    /// inside an `OnDeletion` handler — the loser is mid-deletion (calling
    /// the handler) and the winner is the other side of the pending attack.
    /// Direct player attacks (`AttackTarget::Player`) return `None` even
    /// when self is the attacker, because there is no opposing Digimon.
    pub fn battle_opponent_of(
        &self,
        self_handle: crate::card_source::PermanentHandle,
    ) -> Option<crate::card_source::PermanentHandle> {
        let pa = self.game.pending_attack.as_ref()?;
        let defender = match pa.effective_target {
            crate::combat::AttackTarget::Digimon(h) => Some(h),
            crate::combat::AttackTarget::Player(_) => None,
        }?;
        if self_handle == pa.attacker {
            Some(defender)
        } else if self_handle == defender {
            Some(pa.attacker)
        } else {
            None
        }
    }
```

> **Note.** Verify the import paths: `AttackTarget` may be re-exported via `crate::combat::AttackTarget` or `crate::enums::AttackTarget`. Use whichever the surrounding code uses. Same for `PermanentHandle` — likely `crate::card_source::PermanentHandle`.

- [ ] **Step 3.5: Mirror onto `EffectContext` (mutable variant).**

Around line ~336 of the same file, after `was_deleted_by_opponent` on the mutable `EffectContext`:

```rust
    /// See [`EffectReadContext::battle_opponent_of`].
    pub fn battle_opponent_of(
        &self,
        self_handle: crate::card_source::PermanentHandle,
    ) -> Option<crate::card_source::PermanentHandle> {
        let pa = self.game.pending_attack.as_ref()?;
        let defender = match pa.effective_target {
            crate::combat::AttackTarget::Digimon(h) => Some(h),
            crate::combat::AttackTarget::Player(_) => None,
        }?;
        if self_handle == pa.attacker {
            Some(defender)
        } else if self_handle == defender {
            Some(pa.attacker)
        } else {
            None
        }
    }
```

(Identical body — they read the same `Game` reference. If there's already a code-sharing pattern for read-vs-mutable accessors in the file, follow it.)

- [ ] **Step 3.6: Run test to confirm pass.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml battle_opponent_of_returns_attacker_when_self_is_defender
```

Expected: PASS.

- [ ] **Step 3.7: Run full lib test suite.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --lib
```

Expected: all green.

- [ ] **Step 3.8: Commit.**

```bash
git add digimon-engine/src/effect_context/mod.rs
git commit -m "engine(ctx): battle_opponent_of accessor for OnDeletion observers

Reads Game.pending_attack (live during resolve_battle's
delete_permanent_with_cause call) and returns the opposing combatant.
Phase E §E1 prerequisite — Retaliation's OnDeletion handler uses this
to identify the battle winner. Returns None for direct player attacks
and for non-combatants."
```

---

## Task 4: E1 — Retaliation auto-install (TDD)

**Mechanic.** When this Digimon is deleted by Battle (and only by Battle — `was_deleted_by_effect()` must be false), delete the opposing combatant. Mandatory (no "may" clause per RULES_CONTEXT 16-30).

**Pattern.** Mirrors Fortitude's `OnDeletion` trigger ([keyword_effects.rs:614](../../../digimon-engine/src/cards/keyword_effects.rs)) but the body uses `ctx.battle_opponent_of(self)` to identify the target and calls `ctx.delete_permanent` (with `cause = OwnEffect`) on the winner. Naturally self-scoped via the `TriggerSource::Permanent(carrier)` enqueue path.

**Files:**
- Modify: `digimon-engine/tests/keyword_phase_e/retaliation.rs` (currently empty)
- Modify: `digimon-engine/src/cards/keyword_effects.rs` (add `Keyword::Retaliation` arm before the `_ =>` fallthrough)

- [ ] **Step 4.1: Write the first failing test — happy path.**

Append to `digimon-engine/tests/keyword_phase_e/retaliation.rs`:

```rust
//! Phase E §E1 — `Keyword::Retaliation` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Retaliation]` (no
//! hand-rolled `CardEffect`) must, when self is deleted by Battle, delete
//! the opposing combatant. Mandatory; no "may" clause (RULES_CONTEXT
//! 16-30). Cause filter: `was_deleted_by_effect() == false` AND
//! `deletion_cause() == Some(ReplacementCause::Battle)`.
//!
//! Mirrors DCGO `Retaliation.cs` — fires from `OnDestroyedAnyone` with
//! `IsByBattle(hashtable)` cause filter, targeting `WinnerPermanents`.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

use super::helpers::plain_digimon;

fn retaliation_card(id: &str, dp: u16) -> CardData {
    let mut c = plain_digimon(id);
    c.dp = Some(dp);
    c.keywords = vec![Keyword::Retaliation];
    c
}

/// Stack: P0[ATTACKER 5000 DP], P1[RETAL 3000 DP, Retaliation].
/// P0 attacks RETAL → battle resolves with attacker win → RETAL goes to
/// trash → Retaliation fires → ATTACKER also deleted.
#[test]
fn retaliation_deletes_winner_when_self_loses_battle() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATK"))   // will be DP-bumped via stack
        .add_card(retaliation_card("RETAL", 3000))
        .start();

    // Attacker has 5000 DP via test setup (place a high-DP source under it
    // OR override the card's printed DP). Use the simpler route: write
    // ATK with dp=5000 in its CardData.
    // …
    // Drive the attack: r.action(…) until resolve_battle fires.
    // Expected post-state:
    //   - r.game.players[1].battle_area.is_empty() — RETAL gone (lost battle)
    //   - r.game.players[0].battle_area.is_empty() — ATK gone (Retaliation)
    //   - r.game.players[0].trash.len() == 1
    //   - r.game.players[1].trash.len() == 1
}
```

> **Note for the executor.** The exact attack-sequence driving (mask-action sequence to invoke an attack in `DebugRunner`) is well-precedented in `digimon-engine/tests/combat_scenarios/*.rs`. Find one that drives a basic Digimon-vs-Digimon attack where attacker wins, copy the action sequence, and adapt the assertions. Don't reinvent the harness.

- [ ] **Step 4.2: Run the test, confirm it fails.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e retaliation_deletes_winner_when_self_loses_battle
```

Expected: FAIL — RETAL deleted but ATK still on field (auto-install not yet wired).

- [ ] **Step 4.3: Add the `Keyword::Retaliation` arm to `keyword_to_auto_effect`.**

Insert in `digimon-engine/src/cards/keyword_effects.rs` directly before the `_ => Vec::new(),` fallthrough (~line 912):

```rust
        // Phase E §E1 — printed Retaliation: "When this Digimon is deleted
        // by Battle, delete the opposing combatant." DCGO `Retaliation.cs`.
        //
        // ## Cause filter
        //
        //   - `was_deleted_by_effect() == false` — RULES_CONTEXT 16-30
        //     specifies battle-only; effect-caused deletions don't trigger.
        //   - DCGO `Retaliation.cs` uses `IsByBattle(hashtable)` to gate;
        //     equivalent to `deletion_cause() == Some(Battle)`.
        //
        // ## Target identification
        //
        // `ctx.battle_opponent_of(self)` reads the live `Game.pending_attack`
        // (set in `combat::resolve_battle` and not cleared until after
        // `delete_permanent_with_cause` returns) and returns the opposing
        // combatant — i.e., the battle winner, since the loser is the one
        // calling this OnDeletion observer. Returns None for direct-player
        // attacks (no Digimon target) and for non-combatant deletions.
        //
        // ## Mandatory semantics
        //
        // No "may" clause. The trigger fires unconditionally when the cause
        // gate passes; this matches a non-`.optional()` `OnDeletion` process
        // (no PASS dialog).
        //
        // ## Self-scope
        //
        // The trigger is keyed on the carrier's `TriggerSource::Permanent(h)`
        // — natural self-scoping (a neighbor's deletion doesn't fire
        // Retaliation on this carrier). The body's `source_permanent` guard
        // is belt-and-suspenders.
        //
        // ## Known scope: source-card Retaliation (out of Phase E)
        //
        // If Retaliation appears as a digivolution-source effect (not on
        // the top card), the per-card override applies via hand-rolled
        // `CardEffect`. Auto-install covers the top-card carrier case.
        Keyword::Retaliation => vec![Effect::on_deletion(card)
            .name("<Retaliation>")
            .process(|ctx| {
                // Cause gate: Battle only.
                use crate::replacement::ReplacementCause;
                if !matches!(ctx.deletion_cause(), Some(ReplacementCause::Battle)) {
                    return;
                }
                let Some(me) = ctx.source_permanent else {
                    return;
                };
                let Some(winner) = ctx.battle_opponent_of(me) else {
                    return;
                };
                // Delete the winner by own-effect cause. The winner's own
                // OnDeletion / WhenWouldBeDeleted hooks will fire normally.
                ctx.delete_permanent(winner);
            })
            .build()],
```

> **Note on `ctx.delete_permanent` cause.** Verify the mutable-context API exposes a `delete_permanent(handle)` that defaults to `OwnEffect` cause. If it requires explicit cause, pass `ReplacementCause::OwnEffect` (or whatever the call signature dictates). Look at the Decoy / Save arms for the existing call shape.

- [ ] **Step 4.4: Run the test, confirm pass.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e retaliation_deletes_winner_when_self_loses_battle
```

Expected: PASS.

- [ ] **Step 4.5: Add the gate-failure tests.**

Append to `retaliation.rs`:

```rust
/// Cause gate: when self is deleted by an opponent's effect (not Battle),
/// Retaliation does NOT fire. There is no "winner" to retaliate against.
#[test]
fn retaliation_does_not_fire_on_effect_deletion() {
    // Place RETAL on field, call game.delete_permanent_with_cause(retal,
    // ReplacementCause::OpponentEffect). Assert no other permanent is
    // deleted as a side-effect.
}

/// Cause gate: when self is deleted by its own controller's effect (e.g.
/// self-sacrifice), Retaliation does NOT fire.
#[test]
fn retaliation_does_not_fire_on_own_effect_deletion() {
    // Same shape: delete with cause=OwnEffect, assert no cascade.
}

/// Mutual destruction (tied DP): both combatants die in battle. Each side's
/// Retaliation fires against the other → second-order deletion targets a
/// permanent already in trash. Verify graceful handling: no panic, no
/// double-delete.
#[test]
fn retaliation_handles_mutual_destruction() {
    // P0[ATK, Retaliation, 4000 DP], P1[DEF, Retaliation, 4000 DP].
    // Drive attack → tie → both delete → both Retaliation triggers fire
    // but the targets are already gone. Expect both trashes have 1 card,
    // no panic.
}
```

- [ ] **Step 4.6: Run all retaliation tests, confirm all pass.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e retaliation
```

Expected: 3+ tests, all green. If `retaliation_handles_mutual_destruction` panics or asserts on `handle_valid`, the body needs an extra guard:

```rust
if !ctx.game.handle_valid(winner) {
    return;
}
```

…inserted before the `delete_permanent` call. Add and re-run.

- [ ] **Step 4.7: Commit.**

```bash
git add digimon-engine/tests/keyword_phase_e/retaliation.rs digimon-engine/src/cards/keyword_effects.rs
git commit -m "engine(keyword): Retaliation auto-install (Phase E §E1)

Printed <Retaliation> mounts an OnDeletion trigger gated on
deletion_cause() == Battle that deletes the opposing combatant
identified via ctx.battle_opponent_of(self). Mandatory — no may
dialog. Mutual-destruction case guarded by handle_valid check before
the cascade delete.

Test coverage: happy-path attacker-loses, gate failure on effect
deletion (own + opponent), mutual destruction.

DCGO: Retaliation.cs IsByBattle + WinnerPermanents target
RULES_CONTEXT: 16-30"
```

---

## Task 5: E2 — Scapegoat auto-install (TDD)

**Mechanic.** When this Digimon would be deleted by anything other than its own controller's effect, the controller may delete another of their permanents to cancel this deletion. Optional (per RULES_CONTEXT 16-31 "may"). Substitute-style replacement: pick a different own permanent → that one dies, this one survives.

**Pattern.** Mirrors Decoy's `WhenWouldBeDeleted` substitute replacement ([keyword_effects.rs:512](../../../digimon-engine/src/cards/keyword_effects.rs)) but with three differences:
1. **Subject scope:** Decoy substitutes self for an *ally's* deletion (cross-permanent). Scapegoat substitutes a *different ally* for *self's* deletion (self → other).
2. **Cause filter:** Decoy has none. Scapegoat skips when `deletion_cause() == Some(OwnEffect)`.
3. **Selection:** Decoy uses sync substitute. Scapegoat needs a parked own-permanent pick (filtered: same-controller, non-self) before the substitute call.

**Files:**
- Modify: `digimon-engine/tests/keyword_phase_e/scapegoat.rs`
- Modify: `digimon-engine/src/cards/keyword_effects.rs`

- [ ] **Step 5.1: Write the first failing test — happy path.**

Append to `scapegoat.rs`:

```rust
//! Phase E §E2 — `Keyword::Scapegoat` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Scapegoat]` (no
//! hand-rolled `CardEffect`) must, when self would be deleted by anything
//! other than the controller's own effect, optionally let the controller
//! pick a different own permanent to delete instead.
//!
//! Mirrors DCGO `Scapegoat.cs` — Immediate-type, optional, picks
//! `permanentCondition`-filtered other own permanent.
//!
//! RULES_CONTEXT 16-31. Cause filter: deletion_cause() != Some(OwnEffect).

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

use super::helpers::plain_digimon;

fn scapegoat_card(id: &str) -> CardData {
    let mut c = plain_digimon(id);
    c.keywords = vec![Keyword::Scapegoat];
    c
}

/// P0 has SCAP (Scapegoat) and ALLY (plain). Opponent triggers SCAP's
/// deletion by `OpponentEffect` → outer accept dialog parks → on accept,
/// inner own-permanent pick offers ALLY → on ALLY pick, substitute fires →
/// SCAP survives, ALLY dies.
#[test]
fn scapegoat_substitutes_ally_for_self_on_opponent_effect_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);
    let _ally = r.place_on_field(0, "ALLY", None);

    // Trigger an OpponentEffect deletion. Use `delete_permanent_with_cause`
    // directly with cause=OpponentEffect to bypass the need for a real
    // opponent effect script.
    r.game.delete_permanent_with_cause(
        scap,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    // Outer accept dialog: optional ("may" substitute).
    {
        let pending = r.game.pending_selection.as_ref()
            .expect("Scapegoat outer accept dialog must be parked");
        assert!(pending.is_optional);
        assert_eq!(pending.selecting_player, 0);
    }
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Scapegoat substitute");

    // Inner pick: select an own permanent != self.
    {
        let pending = r.game.pending_selection.as_ref()
            .expect("Scapegoat inner ally pick must be parked");
        assert!(!pending.is_optional, "inner pick is mandatory once accepted");
    }
    // Pick ALLY (the only valid candidate).
    let ally_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, ally_action).expect("pick ally");

    // Post-state: SCAP survives, ALLY in trash.
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "SCAP");
    assert_eq!(r.game.players[0].trash.len(), 1);
}
```

- [ ] **Step 5.2: Run test, confirm failure.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e scapegoat_substitutes_ally
```

Expected: FAIL — SCAP deleted, no parked selection, no substitute.

- [ ] **Step 5.3: Add the `Keyword::Scapegoat` arm to `keyword_to_auto_effect`.**

Insert before the `_ => Vec::new(),` fallthrough (after the Retaliation arm from Task 4):

```rust
        // Phase E §E2 — printed Scapegoat: "When this Digimon would be
        // deleted [other than by your own effect], you may delete another
        // of your Digimon to prevent it." DCGO `Scapegoat.cs`.
        //
        // ## Cause filter
        //
        //   - `deletion_cause() != Some(OwnEffect)` — RULES_CONTEXT 16-31:
        //     a player's own effect cannot trigger their Scapegoat. Battle,
        //     SecurityCheck, OpponentEffect, and Cost all DO trigger.
        //
        // ## Selection chain
        //
        //   1. Outer optional accept dialog ("may"). PASS leaves the
        //      original deletion to proceed.
        //   2. On ACCEPT: parked own-permanent pick. Filter: same-controller,
        //      non-self. Mandatory once accepted (no "may" within the body
        //      per DCGO — once committed to substitute, you must pick).
        //   3. On pick: `rctx.substitute(ReplacementSubject::Permanent(picked))`
        //      — synchronous substitute. The dispatcher re-routes the
        //      original deletion to the picked permanent.
        //
        // ## Self-scope
        //
        // `WhenWouldBeDeleted` enumerates the carrier's effects only when
        // `subject == carrier`; the body's `subject == me_perm` guard
        // ensures Scapegoat doesn't fire on a neighbor's deletion.
        //
        // ## No-candidate handling
        //
        // If the controller has no other permanents (`ALLY` doesn't exist),
        // the inner pick has no valid actions. The substrate's
        // `select_own_permanent` handles empty-candidate sets by skipping
        // the pick and not calling the body callback — which means no
        // substitute fires, and the original deletion proceeds. This
        // matches DCGO: Scapegoat with no targets does nothing.
        Keyword::Scapegoat => vec![Effect::when_would_be_deleted(card)
            .name("<Scapegoat>")
            .optional()
            .replacement_process(|rctx| {
                use crate::replacement::{ReplacementCause, ReplacementSubject};
                // Self-scope guard.
                let me_perm = match rctx.effect.source_permanent {
                    Some(h) => h,
                    None => return,
                };
                let subject = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                if subject != me_perm {
                    return;
                }
                // Cause filter: skip OwnEffect.
                if matches!(
                    rctx.effect.game.current_deletion_cause,
                    Some(ReplacementCause::OwnEffect)
                ) {
                    return;
                }
                // Inner pick: another of own permanents.
                let owner = me_perm.player;
                rctx.select_own_permanent(
                    "select another of your permanents to delete instead",
                    /*is_optional=*/ false,
                    move |g, h| {
                        // Same-controller, non-self.
                        h.player == owner && h != me_perm
                    },
                    move |inner_rctx, picked| {
                        inner_rctx.substitute(ReplacementSubject::Permanent(picked));
                    },
                );
            })
            .build()],
```

> **Note on the inner-pick API.** `rctx.select_own_permanent` may not exist on the replacement context — check Decoy and Save for the actual API. If the parked-own-permanent pick is invoked through a different name (e.g. `rctx.park_own_permanent_pick(…)` or `select_own_permanent_in_replacement(…)`), use that. The Save arm at `keyword_effects.rs:411` is the closest precedent for a parked-pick-within-replacement.

- [ ] **Step 5.4: Run the test, confirm pass.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e scapegoat_substitutes_ally
```

Expected: PASS.

- [ ] **Step 5.5: Add gate-failure / decline / no-candidate tests.**

Append:

```rust
/// Cause gate: own-effect deletion does NOT trigger Scapegoat.
#[test]
fn scapegoat_does_not_fire_on_own_effect_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_digimon("ALLY"))
        .start();
    let scap = r.place_on_field(0, "SCAP", None);
    let _ally = r.place_on_field(0, "ALLY", None);

    r.game.delete_permanent_with_cause(
        scap,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );

    // No parked selection — Scapegoat's optional dialog should not fire.
    assert!(r.game.pending_selection.is_none());
    // SCAP is in trash; ALLY survives.
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}

/// Decline the optional dialog — original deletion proceeds.
#[test]
fn scapegoat_decline_proceeds_with_self_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_digimon("ALLY"))
        .start();
    let scap = r.place_on_field(0, "SCAP", None);
    let _ally = r.place_on_field(0, "ALLY", None);

    r.game.delete_permanent_with_cause(
        scap,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    r.game.resolve_selection(0, PASS).expect("decline Scapegoat");

    // SCAP gone, ALLY survives.
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),
        "ALLY"
    );
}

/// No other own permanents → no inner pick offered → original deletion
/// proceeds. (Whether the outer "may" dialog still parks depends on the
/// substrate; if it parks, ACCEPT should be filtered out.)
#[test]
fn scapegoat_no_other_permanents_proceeds_with_self_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .start();
    let scap = r.place_on_field(0, "SCAP", None);

    r.game.delete_permanent_with_cause(
        scap,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    // Drain any optional dialog that may have parked (decline path).
    if r.game.pending_selection.is_some() {
        r.game.resolve_selection(0, PASS).ok();
    }
    assert!(r.game.players[0].battle_area.is_empty());
    assert_eq!(r.game.players[0].trash.len(), 1);
}
```

- [ ] **Step 5.6: Run all scapegoat tests, confirm all pass.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e scapegoat
```

Expected: 4 tests, all green.

- [ ] **Step 5.7: Commit.**

```bash
git add digimon-engine/tests/keyword_phase_e/scapegoat.rs digimon-engine/src/cards/keyword_effects.rs
git commit -m "engine(keyword): Scapegoat auto-install (Phase E §E2)

Printed <Scapegoat> mounts a WhenWouldBeDeleted substitute replacement
gated on deletion_cause() != OwnEffect (RULES_CONTEXT 16-31).
Optional outer dialog; on accept, parks an own-permanent pick filtered
to same-controller non-self, then issues a sync substitute to the
picked permanent.

Test coverage: happy-path substitute, own-effect cause skip, decline
PASS, no-other-permanents.

DCGO: Scapegoat.cs Immediate-type optional substitute"
```

---

## Task 6: E3 — DeDigivolve(N) printed-form auto-install (TDD)

**Mechanic.** Existing `Keyword::DeDigivolve(N)` variant gets an active-skill auto-emit so a card with printed `＜De-Digivolve N＞` (and no hand-rolled effect) gets a usable `[Main]` skill.

**Open question (resolve before implementing):** does any current card actually print `＜De-Digivolve N＞` as a bare keyword, and if so, what timing? The spec calls it "active-skill auto-emit" without committing to a timing. Two candidates:
1. **`MainOnField`** — Digimon-on-field active skill; selects an opponent Digimon, calls `ctx.de_digivolve(target, None, Some(N))`. Free activation (no cost). Mirrors MaterialSave's shape.
2. **`OnPlay`** — emit when the card is played. More appropriate if the keyword is printed on Options.

The spec at §6 line 256 calls out `Effect::de_digivolve_active_skill(n: u8)` — the `_active_skill` suffix suggests `MainOnField`. Default to that. Flag any contrary findings to the user before implementation.

**Files:**
- Modify: `digimon-engine/tests/keyword_phase_e/de_digivolve_n.rs`
- Modify: `digimon-engine/src/cards/keyword_effects.rs`

- [ ] **Step 6.1: Verify timing assumption.**

```bash
grep -rn "De-Digivolve" data/cards.json | head -20
```

If matches show printed `＜De-Digivolve N＞` on Digimon cards (e.g., as inherited or main effects), `MainOnField` is correct. If matches are exclusively on Options (`card_kind: Option`), pause and ask the user — the auto-install body will need to dispatch on card kind. **Default assumption: `MainOnField` regardless; per-card overrides via hand-rolled effect for Options.**

- [ ] **Step 6.2: Write the failing test.**

Append to `de_digivolve_n.rs`:

```rust
//! Phase E §E3 — `Keyword::DeDigivolve(N)` printed-form auto-install tests.
//!
//! A Digimon card declaring ONLY `keywords: vec![Keyword::DeDigivolve(2)]`
//! (no hand-rolled `CardEffect`) must expose a `[Main]` active skill that
//! selects an opponent Digimon with at least 1 source under top and pops
//! up to N digivolution sources. Free activation (no cost, no suspend).
//!
//! Mirrors MaterialSave (Phase D Task 10) shape — MainOnField timing,
//! gate-checked, then a parked select-opponent-Digimon pick → calls
//! `ctx.de_digivolve(target, None, Some(N))`.
//!
//! Spec: docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md §E3

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

use super::helpers::{digimon_with_keywords, plain_digimon};

#[test]
fn de_digivolve_n_pops_up_to_n_sources_from_opponent() {
    // P0[D2(2)], P1[STACK with 3 sources].
    // Activate D2's [Main]; pick STACK; expect 2 pops.
    // Verify ctx.de_digivolve was called with target=stack, amount=Some(2)
    // by asserting on STACK.card_sources.len() before/after.
    todo!("see spec; mirror keyword_phase_d/material_save.rs::material_save_n_moves_sources");
}

#[test]
fn de_digivolve_n_gate_blocks_when_no_valid_opponent_target() {
    // Opponent has no Digimon with sources → mask doesn't expose the
    // [Main] activation slot.
    todo!();
}
```

- [ ] **Step 6.3: Run test, confirm failure.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e de_digivolve_n
```

Expected: FAIL on `todo!`.

- [ ] **Step 6.4: Implement the test bodies and the auto-install arm.**

Body for the test should:
- Build a `D2` card via `digimon_with_keywords("D2", 5, 5000, vec![Keyword::DeDigivolve(2)])`.
- Build a `STACK` opponent Digimon with 3 sources.
- Place both on field; advance to P0's main phase.
- Activate the `[Main]` skill on D2; pick STACK.
- Assert `STACK.card_sources.len() == 1` (popped 2 of 3 sources, base remains).

Auto-install arm (insert before `_ => Vec::new(),`):

```rust
        // Phase E §E3 — printed De-Digivolve N: a [Main] active skill that
        // pops up to N digivolution sources from a chosen opponent Digimon.
        // Reuses ctx.de_digivolve's existing amount=Some(N) parameter and
        // the WhenWouldBeDeDigivolved replacement window.
        //
        // Pattern mirrors MaterialSave (Phase D Task 10): MainOnField
        // declarative effect, gate at mask-build time on at-least-one-valid
        // opponent target, then a parked opp-permanent pick.
        //
        // ## Self-scope: same as MaterialSave (MainOnField iterates the
        // carrier's stack; source_permanent is the carrier).
        Keyword::DeDigivolve(n) => vec![Effect::declarative(card)
            .name(&format!("<De-Digivolve {n}>"))
            .timing(EffectTiming::MainOnField)
            .condition(move |ctx| {
                // Gate: at least one opponent Digimon has ≥1 source under
                // top (otherwise de_digivolve is a no-op).
                let owner = ctx.player;
                let opp = 1 - owner;
                ctx.battle_area(opp).iter().any(|p| {
                    p.is_digimon(&ctx.game.card_data) && p.card_sources.len() >= 2
                })
            })
            .process(move |ctx| {
                let Some(me) = ctx.source_permanent else { return; };
                let owner = me.player;
                let opp = 1 - owner;
                ctx.select_opponent_permanent(
                    "select an opponent Digimon to de-digivolve",
                    /*is_optional=*/ false,
                    move |g, h| {
                        if h.player != opp {
                            return false;
                        }
                        let p = match g.players[h.player as usize]
                            .battle_area
                            .get(h.index as usize)
                        {
                            Some(p) => p,
                            None => return false,
                        };
                        p.is_digimon(&g.card_data) && p.card_sources.len() >= 2
                    },
                    move |ctx, target| {
                        ctx.de_digivolve(target, None, Some(n));
                    },
                );
            })
            .build()],
```

> **Note.** Verify `ctx.select_opponent_permanent` exists with this signature; if not, look in `effect_context/selections.rs` for the actual filter-pick API and adapt. Verify `ctx.battle_area(player)` accessor name.

- [ ] **Step 6.5: Run, iterate until pass.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e de_digivolve_n
```

- [ ] **Step 6.6: Commit.**

```bash
git add digimon-engine/tests/keyword_phase_e/de_digivolve_n.rs digimon-engine/src/cards/keyword_effects.rs
git commit -m "engine(keyword): DeDigivolve(N) printed-form auto-install (Phase E §E3)

Printed <De-Digivolve N> on a Digimon emits a MainOnField active skill
mirroring MaterialSave's shape. Selects an opponent Digimon with >=1
source under top, calls ctx.de_digivolve(target, None, Some(N)).

Per-card overrides for Option-card timing or non-Digimon carriers via
hand-rolled CardEffect (out of scope here)."
```

---

## Task 7: E4 — DrawX(N) printed-form auto-install (TDD)

**Mechanic.** Existing `Keyword::DrawX(N)` variant gets an active-skill auto-emit so a card with printed `＜Draw N＞` (typically Options) gets a usable activation.

**Open question (resolve before implementing):** Same as E3 — Option vs Digimon carrier. The spec at §5 line 212 explicitly says "Option cards" for this one. Need to verify how `OnPlay` for Options is wired in the engine. Two candidates:
1. **`OnPlay` (Option-card play resolution)** — fires when the Option resolves; `ctx.draw(player, n)`.
2. **`MainOnField`** — only useful for Digimon carriers (rare for `<Draw N>`).

The spec calls for Option-card support, so the right primary timing is whatever fires when an Option card resolves on play. **Default assumption: `OnPlay` if it exists, else `MainOnField` as a fallback for Digimon-carrier cases.**

**Files:**
- Modify: `digimon-engine/tests/keyword_phase_e/draw_x_n.rs`
- Modify: `digimon-engine/tests/keyword_phase_e/helpers.rs` (add `plain_option`)
- Modify: `digimon-engine/src/cards/keyword_effects.rs`

- [ ] **Step 7.1: Identify the `OnPlay` timing for Options.**

```bash
grep -n "EffectTiming::On" digimon-engine/src/enums.rs
grep -rn "card_kind: CardKind::Option" digimon-engine/tests/ | head -5
grep -rn "EffectTiming::OnPlay\|EffectTiming::Option" digimon-engine/src/ | head -10
```

Identify the timing variant Options use when resolving on play. If there is no clean "fires when this Option resolves" timing, pause and ask the user — the right path may be a new timing variant (not in Phase E scope).

- [ ] **Step 7.2: Add `plain_option` to helpers.**

Append to `tests/keyword_phase_e/helpers.rs`:

```rust
pub fn plain_option(id: &str, cost: u8) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: cost,
        colors: vec![CardColor::Blue],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}
```

- [ ] **Step 7.3: Write the failing test.**

Append to `draw_x_n.rs`:

```rust
//! Phase E §E4 — `Keyword::DrawX(N)` printed-form auto-install tests.
//!
//! An Option card declaring ONLY `keywords: vec![Keyword::DrawX(2)]` (no
//! hand-rolled `CardEffect`) must, when played, draw N cards for the
//! controller. Auto-install reuses the engine's existing `ctx.draw(player, N)`
//! primitive.
//!
//! Spec: docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md §E4

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

use super::helpers::{plain_digimon, plain_option};

#[test]
fn draw_x_n_draws_n_cards_when_option_resolves() {
    // Build a Draw 2 Option card. Stack 5+ filler cards in P0's deck.
    // Play the Option from hand. Expect P0.hand.len() before-and-after
    // diff = +2 (minus the 1 Option played) = net +1 from baseline.
    todo!();
}

#[test]
fn draw_x_n_draws_zero_when_deck_empty() {
    // Pre-empty P0.deck. Play Draw 2 Option. Expect no panic, draw count = 0.
    todo!();
}
```

- [ ] **Step 7.4: Run test, confirm failure.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e draw_x_n
```

Expected: FAIL on `todo!`.

- [ ] **Step 7.5: Implement the auto-install arm.**

The exact timing depends on Step 7.1's findings. Sketch (using `EffectTiming::OnPlay` placeholder):

```rust
        // Phase E §E4 — printed Draw N: when this card resolves on play,
        // controller draws N cards. Typical carrier is an Option card.
        Keyword::DrawX(n) => vec![Effect::declarative(card)
            .name(&format!("<Draw {n}>"))
            .timing(EffectTiming::OnPlay) // VERIFY: correct variant per Step 7.1
            .process(move |ctx| {
                let player = ctx.source_permanent
                    .map(|h| h.player)
                    .unwrap_or(ctx.player);
                ctx.draw(player, n);
            })
            .build()],
```

- [ ] **Step 7.6: Run, iterate until pass.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_e draw_x_n
```

- [ ] **Step 7.7: Commit.**

```bash
git add digimon-engine/tests/keyword_phase_e/ digimon-engine/src/cards/keyword_effects.rs
git commit -m "engine(keyword): DrawX(N) printed-form auto-install (Phase E §E4)

Printed <Draw N> emits an OnPlay declarative effect that calls
ctx.draw(player, N). Targets Option-card carriers; Digimon carriers
needing different timing override via hand-rolled CardEffect.

Test coverage: happy-path draw N, deck-empty graceful zero."
```

---

## Task 8: Update parity doc + spec status

Flip the four Phase E rows to ✅ in the parity doc and mark Phase E as landed in the spec.

**Files:**
- Modify: `docs/DCGO_KEYWORD_PARITY.md` (rows for Retaliation, Scapegoat, DeDigivolve(N), DrawX(N); §"Gap ranking" item 6 + 7; missing-keyword backfill priorities table)
- Modify: `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md` (mark Phase E landed at §5)
- Modify: `digimon-engine/src/cards/keyword_effects.rs` module docstring (move Retaliation/Scapegoat/DeDigivolve/DrawX out of "Out-of-scope deferred" list)

- [ ] **Step 8.1: Update parity doc — flip ✅ rows.**

In `docs/DCGO_KEYWORD_PARITY.md`:

- **Row for `DeDigivolve(N)`** (currently 🔴): change to ✅ with note "Auto-installed in Phase E 2026-04-25; `MainOnField` active skill via `keyword_to_auto_effect`. See `keyword_effects.rs` and `tests/keyword_phase_e/de_digivolve_n.rs`."
- **Row for `DrawX(N)`** (currently 🔴): change to ✅ with note "Auto-installed in Phase E 2026-04-25; `OnPlay` declarative effect calling `ctx.draw`. See `keyword_effects.rs` and `tests/keyword_phase_e/draw_x_n.rs`."
- **Row for `Retaliation`** (currently ❌): change to ✅ with note "Auto-installed in Phase E 2026-04-25; `OnDeletion` trigger gated on `deletion_cause() == Battle`, deletes opposing combatant via new `ctx.battle_opponent_of` accessor. See `keyword_effects.rs` and `tests/keyword_phase_e/retaliation.rs`."
- **Row for `Scapegoat`** (currently ❌): change to ✅ with note "Auto-installed in Phase E 2026-04-25; `WhenWouldBeDeleted` substitute replacement gated on `deletion_cause() != OwnEffect`, parked own-permanent pick → sync substitute. See `keyword_effects.rs` and `tests/keyword_phase_e/scapegoat.rs`."

In §"Gap ranking":
- Strike-through rows for Retaliation and DeDigivolve(N) auto-install.

In §"Missing-keyword backfill priorities":
- Strike-through "Retaliation" entry (priority 1); the Scapegoat row needs the same treatment.

In §"Parametric auto-install gap":
- Mark DeDigivolve(N) and DrawX(N) as resolved Phase E.

- [ ] **Step 8.2: Update the design spec.**

In `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md`, append to the Phase E section header (line 205):

```markdown
### Phase E — Missing-from-enum backfill ✅ landed 2026-04-25 on `claude/vigorous-elgamal-453703`

Deliverables shipped:
- E1 Retaliation: enum variant + parser + auto-install + 3 behavioral tests
- E2 Scapegoat: enum variant + parser + auto-install + 4 behavioral tests
- E3 DeDigivolve(N) printed-form: auto-install (MainOnField) + 2 tests
- E4 DrawX(N) printed-form: auto-install (OnPlay) + 2 tests
- New `EffectContext::battle_opponent_of` accessor (E1 prerequisite)

No new substrate; consumed Phase B/C/D primitives as-is.
```

- [ ] **Step 8.3: Update `keyword_effects.rs` module docstring.**

Edit the "Out-of-scope deferred" comment block (around line 22-24): remove Retaliation, Scapegoat, DeDigivolve(N), DrawX(N). Add a "Phase E (landed 2026-04-25)" sub-section under "Coverage matrix":

```rust
//! ## Coverage matrix (Phase D — landed 2026-04-25, Phase E — landed 2026-04-25)
//!
//! Auto-installed: Barrier, Evade, Decode (Phase 7); Fragment(N), ArmorPurge,
//! Save, Decoy, Fortitude, Partition, MaterialSave(N) (Phase D);
//! Retaliation, Scapegoat, DeDigivolve(N), DrawX(N) (Phase E).
//!
//! Out-of-scope deferred: Execute, Iceclad, MindLink, Training (Phase F).
```

- [ ] **Step 8.4: Final verification — run the entire engine test suite.**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml
```

Expected: every test green. If anything regresses, root-cause and fix before committing.

- [ ] **Step 8.5: Commit.**

```bash
git add docs/DCGO_KEYWORD_PARITY.md docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md digimon-engine/src/cards/keyword_effects.rs
git commit -m "docs(keyword-parity): Phase E landed (Retaliation, Scapegoat, DeDigivolve, DrawX)

Flip rows in DCGO_KEYWORD_PARITY.md to ✅. Mark Phase E landed in the
design spec with deliverables list. Update keyword_effects.rs module
docstring coverage matrix.

Phase F (Execute, Iceclad, MindLink, Training) remains the only
unresolved bucket — non-alpha-archetype, deferred."
```

---

## Self-Review (executor)

Before opening a PR, run through this checklist:

1. **Spec coverage:** every Phase E sub-task (§E1/E2/E3/E4) has a code change + test commit. The new `battle_opponent_of` accessor (substrate-light addition) is committed separately so it's reviewable in isolation.
2. **No placeholders:** every test that started as `todo!()` in this plan should be filled in with concrete actions and assertions before commit. Re-grep for `todo!()` in `tests/keyword_phase_e/`.
3. **Type consistency:** `battle_opponent_of` returns `Option<PermanentHandle>` everywhere. `ctx.de_digivolve(target, None, Some(n))` matches the signature in `effect_context/mod.rs:619`. `ctx.draw(player, n)` matches `:474`.
4. **No new substrate:** Phase E adds *no* new fields to `Game`, *no* new `ReplacementCause` variants, *no* new `EffectTiming` variants. If the executor finds they need substrate, escalate to the user before adding it — Phase E is meant to be consumer-side wiring only.
5. **Cargo.toml hygiene:** verify the `[[test]]` block for `keyword_phase_e` parses; `cargo test --test keyword_phase_e` lists tests.
6. **Doc parity:** the parity doc and design spec both reflect the four ✅ landings with consistent wording.

---

## Post-Phase-E follow-ups (out of scope)

- **Phase F** (Execute, Iceclad, MindLink, Training) — non-alpha-archetype keywords. Each needs a new enum variant + a small primitive (e.g. `ctx.attach_tamer_to_digimon`, deck-card-count battle-resolution branch). Not blocked by Phase E.
- **Color-grouped Decoy / Partition / MaterialSave / Scapegoat** — per-card-text overrides for color-filtered selection. Per-card concern; not derivable from `Keyword::*` alone.
- **Source-card Retaliation / Fortitude** — the auto-installs cover the top-card carrier case only. DCGO `CardStack.Contains(card)` semantics for source-card-borne keywords are out of Phase E scope.
