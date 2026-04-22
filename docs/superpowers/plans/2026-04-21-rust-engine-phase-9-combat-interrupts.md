# Rust Engine Phase 9 — Combat Interrupt Completion

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the combat state machine. Wire `WhenWouldAttack` + `WhenWouldBeAttackTarget` replacement dispatch, broaden the Counter window to include hand-play Counter Options + field-triggered Counter abilities, add `ctx.redirect_attack` / `ctx.cancel_attack` helpers, implement Raid target-switch rider, enforce Collision MUST-block, wire Piercing post-battle security check, consume `<Reboot>` at unsuspend phase, and dispatch `OnBlock` / `OnAllyAttack` / `OnOpponentAttack` observers. Unblocks ~30 cards across the 5 audited archetypes (Dark Masters Ace Counter Lv6 sub-archetype, TS Olympos redirect+Collision, scattered observer-driven cards).

**Architecture:**
- 3 new `AttackState` values (`PostBlock`, `PostBattle`, `WouldAttackFire`-implicit) inserted into existing state machine; `advance_pending_attack` remains sole driver.
- 2 Phase 7 reserved `EffectTiming` variants (`WhenWouldAttack`, `WhenWouldBeAttackTarget`) get dispatch sites.
- 1 new `EffectBuilder` method (`.counter()`), 1 new `EffectTiming` variant (`OnBlock` via new TriggerSource variant), 2 new `EffectContext` helpers (`redirect_attack`, `cancel_attack`).
- 4 native keywords get auto-install + runtime consumers: Raid (re-target rider via `WhenWouldBeAttackTarget` declarative effect), Piercing (state-transition consumer), Collision (mandate flip), Reboot (unsuspend fan-out).
- Action-mask contract preserved: `ACTION_SPACE_SIZE = 2168` unchanged; Counter / Raid / Collision reuse existing ranges.
- Phase 7 replacement framework composes cleanly — combat replacements use the same `try_replace` dispatcher.

**Tech Stack:** Rust 2021 (`digimon-engine/`), DebugRunner test harness, existing `EffectContext` / `Effect` / `Game` / `AttackState` patterns established in Phases 1–8.

**Spec:** [docs/superpowers/specs/2026-04-21-combat-interrupt-completion-design.md](../specs/2026-04-21-combat-interrupt-completion-design.md) — authoritative design; read before any task.

---

## Background

Phase 9 closes Cluster I from [`.claude/plans/recursive-coalescing-candle.md`](../../../.claude/plans/recursive-coalescing-candle.md):

- **~30 meta-pool cards** across all 5 audited archetypes:
  - Dark Masters Ace Counter Lv6 sub-archetype: 8 cards (LM-043, EX10-010, BT16-026, EX8-026, EX10-074, BT16-046, BT21-051, BT19-064)
  - TS Olympos effect-driven redirect: 5 cards (BT18-073, ST18-14, ST15-14, EX8-050, EX8-051)
  - TS Olympos Collision: 1 (BT24-063 Locomon)
  - TS Olympos Raid target-switch: 3
  - Dark Masters `<Reboot>` consumer: 4
  - Dark Masters `OnAllyAttack` (BT15-008 Muchomon): 1
  - Scattered Piercing + OnBlock + WouldAttack-passive: ~8

**What exists today (post-Phase 8):**

- `AttackState` pipeline: Declared → AllianceOpen → CounterOpen → BlockOpen → Battle → Cleanup. Vortex short-circuits Declared→Battle.
- Alliance window wired; trait-filter deferred (every Alliance-keyword ally qualifies). Doc'd at `combat.rs:330-335`.
- Counter window wired for blast-digivolve only (`combat.rs:468-599`). `.blast_digivolve()` builder sets `counter = true` + `blast_digivolve = true`. No hand-Counter-Option path; no field-Counter-ability path.
- Block window end-to-end; fires `OnAttackTargetChange` post-declare (`combat.rs:713-720`).
- `OnAttack`, `WhenAttacking`, `EndOfAttack`, `EndOfBattle` dispatch (Phase 1).
- `CounterEffect`, `OnBlock`, `OnAllyAttack`, `OnOpponentAttack` declared as `EffectTiming` variants at `enums.rs:106/121/137/138` — **zero dispatch sites**.
- `WhenWouldAttack`, `WhenWouldBeAttackTarget` declared at `enums.rs:182-183` (Phase 7 reserved) — **zero dispatch sites**.
- Phase 3 keyword parsing: Raid, Piercing, Collision, Reboot all parsed into `CardData.keywords` but Raid is mask-only (no target-switch), Piercing has NO runtime consumer, Collision expands blocker set but doesn't mandate (`is_optional = true`), Reboot has NO runtime consumer.
- Phase 6 `CannotAttack` / `CannotAttackTarget` / `CannotBlock` / `CannotCounter` restrictions exist; Phase 9 auto-installs passive replacements via them.
- Phase 7 replacement framework: `try_replace(timing, subject, cause, original_destination) -> ReplacementOutcome` is live for deletion/return/trash. Combat-side fire-sites don't exist yet.
- Phase 8 `play_option_from_hand` exists; Counter Option routing hooks into this.

**Design principles (carry-forward from spec §3):**

1. **No auto-selection (Rule 17).** Every Counter, Raid retarget, Collision block is a `PendingSelection`.
2. **Combat is a state machine.** New interrupts are named transitions.
3. **Replacements subsume restrictions.** Phase 6 `CannotAttack` auto-installs a `WhenWouldAttack` passive cancel.
4. **Observers see post-replacement reality.**
5. **Keyword-driven helpers auto-install.** Raid → `WhenWouldBeAttackTarget` declarative rider; Piercing / Collision / Reboot consumed directly.
6. **Single script API.** `ctx.redirect_attack(new_target)` + `ctx.cancel_attack()` only.
7. **Counter window is union-zone** (Phase 4 `UnionZone` kind).
8. TDD per Working Rule 18 — failing test first.

**Cards motivating Phase 9** (representative):

- LM-043 Darkdramon — Hand-Counter-Option: "When an opponent's Digimon would attack one of your Digimon, reduce their memory gain by 2."
- EX10-010 BlackWarGreymon — Counter-triggered field ability: "[Your Turn][Counter] When your opponent declares an attack, you may redirect it to a different Digimon."
- BT24-063 Locomon — `<Collision>`. Mandatory block.
- BT15-008 Muchomon — `OnAllyAttack`: "When an ally attacks, gain 1 memory."

---

## File Structure

**New files (created this phase):**

- `digimon-engine/tests/combat/would_attack_replacements.rs` — Task 1 replacement dispatch tests.
- `digimon-engine/tests/combat/redirect_and_cancel.rs` — Task 2 script-API tests.
- `digimon-engine/tests/combat/counter_hand_play.rs` — Task 3 broadened Counter window tests.
- `digimon-engine/tests/combat/raid_retarget.rs` — Task 4 Raid rider tests.
- `digimon-engine/tests/combat/collision_mandatory.rs` — Task 5 Collision mandate tests.
- `digimon-engine/tests/combat/piercing_security.rs` — Task 6 Piercing consumer tests.
- `digimon-engine/tests/combat/reboot_unsuspend.rs` — Task 7 Reboot consumer tests.
- `digimon-engine/tests/combat/on_block_observer.rs` — Task 8 OnBlock dispatch tests.
- `digimon-engine/tests/combat/on_ally_opponent_attack.rs` — Task 8 OnAllyAttack / OnOpponentAttack dispatch tests.
- `digimon-engine/tests/combat/phase9_end_to_end.rs` — Task 10 behavioral e2e.

**Modified files:**

- `digimon-engine/src/enums.rs` — no new variants (all declared in prior phases); update doc comments on `OnBlock`, `WhenWouldAttack`, `WhenWouldBeAttackTarget`, `CounterEffect`, `OnAllyAttack`, `OnOpponentAttack` to reflect Phase 9 dispatch.
- `digimon-engine/src/selection.rs` — new `TriggerSource::OnBlock { attacker, blocker }` variant.
- `digimon-engine/src/effect.rs` — new `EffectBuilder::counter()` builder method. (The `counter: bool` field already exists, set as side effect of `.blast_digivolve()`; now settable independently.)
- `digimon-engine/src/combat.rs` — `begin_attack_impl` fires `WhenWouldAttack` + `WhenWouldBeAttackTarget` upfront; new `AttackState::PostBlock` + `AttackState::PostBattle`; `try_enter_counter` broadened to union-zone selection with hand-Option + field-ability candidates; `try_enter_block` flips `is_optional = false` on Collision + non-empty candidates; `resolve_battle` transitions to PostBattle for Piercing; `fire_on_attack` fans out to `OnAllyAttack` / `OnOpponentAttack`.
- `digimon-engine/src/effect_context/mod.rs` — new `redirect_attack(new_target)` + `cancel_attack()` helpers.
- `digimon-engine/src/game_phases.rs::begin_turn` — Reboot consumer in unsuspend step.
- `digimon-engine/src/cards/keyword_effects.rs` — Raid auto-install: emit `WhenWouldBeAttackTarget` declarative re-target rider.
- `digimon-engine/src/card_data.rs` — parse `Alliance<Trait>` filter syntax (if printed card text supports it); extend `Keyword` variants with `AllianceFiltered(Trait)` OR store on `CardData::alliance_trait_filter: Option<Trait>`.
- `digimon-engine/src/card_registry.rs` — auto-install Phase 6 passive `CannotAttack` → `WhenWouldAttack` replacement wiring at registry build.
- `digimon-engine/src/modifiers.rs` — extend `PlayerModifierEntry` / `ModifierEntry` iteration to emit auto-installed replacements for `CannotAttack*` variants.
- `docs/RUST_ENGINE_API.md` — new §Phase 9 section.
- `docs/RUST_PYTHON_PARITY.md` — new §15 combat-interrupt entry.
- `.claude/plans/recursive-coalescing-candle.md` — flip Phase 9 row.

**Cargo wiring:**

- `digimon-engine/Cargo.toml` already has `[[test]] name = "combat" path = "tests/combat/main.rs"`. New test files added as `mod {name};` in `tests/combat/main.rs`.

---

## Baseline

- **671 tests passing, 0 failing, 0 warnings** under `RUSTFLAGS="-D warnings"` (post–Phase 8 close commit `2f1502f9`).
- `ACTION_SPACE_SIZE = 2168` — stays unchanged through Phase 9.

**Target after Phase 9**: ~711 passing (+40 net new tests).

---

## Tasks

### Task 1: `WhenWouldAttack` + `WhenWouldBeAttackTarget` dispatch

**Files:**
- Modify: `digimon-engine/src/combat.rs::begin_attack_impl` — fire replacement timings before AllianceOpen transition.
- Modify: `digimon-engine/src/modifiers.rs` / `card_registry.rs` — auto-install Phase 6 `CannotAttack*` modifiers as `WhenWouldAttack` passive cancel replacements.
- Create: `digimon-engine/tests/combat/would_attack_replacements.rs` — 8 tests.
- Modify: `digimon-engine/tests/combat/main.rs` — add `mod would_attack_replacements;`.

**Key implementation:**

At the top of `begin_attack_impl` (BEFORE any state transitions, after initial validation):

```rust
pub(crate) fn begin_attack_impl(
    &mut self,
    attacker: PermanentHandle,
    declared_target: AttackTarget,
) -> AttackResult {
    // ... existing validation ...

    // Phase 9: WhenWouldAttack fires on attacker side.
    let outcome = self.try_replace(
        EffectTiming::WhenWouldAttack,
        ReplacementSubject::Permanent(attacker),
        ReplacementCause::Battle,
        None,
    );
    match outcome {
        ReplacementOutcome::Cancelled => {
            // Attack aborted before declaration effects.
            return AttackResult::Cancelled;
        }
        ReplacementOutcome::Substituted(_) => {
            // Rare: attacker-side substitution (e.g. "another Digimon attacks instead").
            // v1: debug_assert + no-op.
            debug_assert!(false, "WhenWouldAttack substitution not supported in v1");
        }
        _ => {}
    }

    if self.pending_selection.is_some() {
        // Optional-replacement selection parked; re-entry handled by selection callback.
        return AttackResult::Pending;
    }

    // Phase 9: WhenWouldBeAttackTarget fires on target side.
    let target_subject = match declared_target {
        AttackTarget::Permanent(h) => ReplacementSubject::Permanent(h),
        AttackTarget::Player(pid) => ReplacementSubject::Player(pid),
    };
    let outcome = self.try_replace(
        EffectTiming::WhenWouldBeAttackTarget,
        target_subject,
        ReplacementCause::Battle,
        None,
    );
    // Same match on outcome — Cancelled aborts, Substituted rewrites effective_target.

    if self.pending_selection.is_some() {
        return AttackResult::Pending;
    }

    // ... existing AllianceOpen transition ...
}
```

**Auto-install Phase 6 `CannotAttack` → passive replacement:**

When `card_registry.rs` or `modifiers.rs` emits a `CannotAttack` (or variants) modifier, ALSO install a matching `WhenWouldAttack` passive replacement on the same scope (player/permanent). The replacement calls `rctx.cancel()` when the cause filter matches.

Specifically: extend `default_passive_cause_filter` (Phase 7 helper) to cover `CannotAttack` → cause filter `Battle` + `OwnEffect` (blocks both player-declared attacks and Jamming-forced attacks, if applicable per printed cards — TBD at card-script time).

Tests (8):

- [ ] **Step 1: Write failing tests** — 8 tests in `digimon-engine/tests/combat/would_attack_replacements.rs`:

```rust
#[test]
fn when_would_attack_cancel_aborts_declaration() {
    // Install mandatory WhenWouldAttack cancel on attacker. Declare attack.
    // Assert: attack aborted, attacker not suspended, no memory swing.
}

#[test]
fn when_would_be_attack_target_cancel_aborts() {
    // Install mandatory cancel on target. Declare attack.
    // Assert: attack aborted, defender state unchanged.
}

#[test]
fn when_would_be_attack_target_substitute_redirects() {
    // WhenWouldBeAttackTarget rctx.substitute(other_handle). Declare.
    // Assert: pending_attack.effective_target == other_handle; OnAttackTargetChange fires.
}

#[test]
fn phase6_cannot_attack_auto_installs_replacement() {
    // Install Phase 6 CannotAttack on attacker. Declare attack via direct
    // call (bypassing mask check). Replacement auto-installed, fires cancel.
    // Assert: attack aborted.
}

#[test]
fn optional_when_would_attack_emits_selection() {
    // Install WhenWouldAttack with is_optional=true. Declare attack.
    // Assert: PendingSelection::Replacement with ACCEPT + PASS action IDs.
}

#[test]
fn multiple_replacements_layer_controller_first() {
    // Two WhenWouldAttack replacements: one owner, one opponent.
    // Owner's fires first. Assert ordering.
}

#[test]
fn when_would_attack_cancelled_fires_end_of_attack() {
    // Cancel WhenWouldAttack. Assert EndOfAttack observer sees cancelled=true.
    // EndOfBattle does NOT fire (no battle).
}

#[test]
fn when_would_be_attack_target_opponent_cancel_filter() {
    // Replacement with cause_filter=OpponentEffect only.
    // Own-turn attack declaration with Battle cause → NOT cancelled.
    // Assert attack proceeds normally.
}
```

Add `mod would_attack_replacements;` to `tests/combat/main.rs`.

- [ ] **Step 2: Run tests — FAIL** (expected).
- [ ] **Step 3: Implement** per pseudocode + auto-install wiring.
- [ ] **Step 4: Tests pass.**
- [ ] **Step 5: Full suite green.** Expected: 671 + 8 = **679 passing, 0 warnings.**
- [ ] **Step 6: Commit.**

```bash
git add digimon-engine/src/combat.rs digimon-engine/src/modifiers.rs digimon-engine/src/card_registry.rs digimon-engine/tests/combat
git commit -m "rust-engine(phase-9): WhenWouldAttack + WhenWouldBeAttackTarget dispatch + Phase 6 auto-install"
```

---

### Task 2: `ctx.redirect_attack` + `ctx.cancel_attack` script helpers

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs` — add `redirect_attack(new_target)` + `cancel_attack()` methods.
- Modify: `digimon-engine/src/combat.rs` — make `PendingAttack::effective_target` mutable through context + add `cancelled: bool` flag.
- Create: `digimon-engine/tests/combat/redirect_and_cancel.rs` — 5 tests.

**Key implementation:**

```rust
impl EffectContext<'_> {
    /// Redirect the active attack to a new target. Fires OnAttackTargetChange.
    /// Returns Err if no active attack or target is invalid.
    pub fn redirect_attack(&mut self, new_target: AttackTarget) -> Result<(), AttackError> {
        let pending = self.game.pending_attack.as_mut()
            .ok_or(AttackError::NoActiveAttack)?;
        // Validate new_target.
        self.validate_attack_target(pending.attacker, new_target)?;
        let old_target = pending.effective_target;
        pending.effective_target = new_target;
        // Fire OnAttackTargetChange with {attacker, old_target, new_target}.
        self.game.enqueue_triggered_global(EffectTiming::OnAttackTargetChange, /* context */);
        Ok(())
    }

    /// Cancel the active attack. State machine short-circuits to Cleanup.
    pub fn cancel_attack(&mut self) -> Result<(), AttackError> {
        let pending = self.game.pending_attack.as_mut()
            .ok_or(AttackError::NoActiveAttack)?;
        pending.cancelled = true;
        Ok(())
    }
}
```

Update `advance_pending_attack` to check `pending.cancelled` at every state transition and short-circuit to Cleanup.

Tests (5):

- [ ] **Step 1: Write failing tests** — 5 tests:

```rust
#[test]
fn ctx_redirect_attack_rewrites_effective_target_and_fires_observer() {
    // OnAttack observer calls ctx.redirect_attack(new_target).
    // Observer B on OnAttackTargetChange witnesses.
    // Assert: effective_target changed; witness fired.
}

#[test]
fn ctx_cancel_attack_short_circuits_to_cleanup() {
    // OnAttack observer calls ctx.cancel_attack().
    // Assert: no Battle state entered; EndOfAttack fires; EndOfBattle does not.
}

#[test]
fn ctx_redirect_attack_validates_target_legality() {
    // Try redirect to a Delayed permanent.
    // Assert: Err returned; effective_target unchanged.
}

#[test]
fn ctx_redirect_attack_invalid_without_active_attack() {
    // Call outside active attack (e.g. OnPlay observer).
    // Assert: Err returned.
}

#[test]
fn ctx_cancel_attack_memory_rollback() {
    // Attack costs memory (via attack-memory-effect card). Cancel mid-attack.
    // Assert: memory restored to pre-declaration value.
}
```

- [ ] **Step 2-6: TDD cycle.** Expected: **684 passing, 0 warnings.**

```bash
git commit -m "rust-engine(phase-9): ctx.redirect_attack + ctx.cancel_attack script API"
```

---

### Task 3: Counter window broadening (hand Options + field abilities)

**Files:**
- Modify: `digimon-engine/src/effect.rs` — add `EffectBuilder::counter()` public method.
- Modify: `digimon-engine/src/combat.rs::try_enter_counter` — broaden candidate scan; route hand Options via `play_option_from_hand` with counter overlay.
- Modify: `digimon-engine/src/effect_queue.rs` — dispatch `EffectTiming::CounterEffect` when selected card's counter body fires.
- Create: `digimon-engine/tests/combat/counter_hand_play.rs` — 6 tests.

**Key implementation:**

```rust
impl EffectBuilder {
    /// Mark this effect as eligible for Counter-window activation.
    /// Distinct from .blast_digivolve() — use for hand Counter Options or
    /// field-triggered Counter abilities.
    pub fn counter(mut self) -> Self {
        self.inner.counter = true;
        self
    }
}
```

Update `try_enter_counter`:

```rust
fn try_enter_counter(&mut self, pending: &mut PendingAttack) -> CounterResult {
    // Only for Digimon targets.
    let AttackTarget::Permanent(target) = pending.effective_target else {
        return CounterResult::NoCandidates;
    };
    let defender_pid = target.player;

    // Counter-depth guard (§5.6).
    if pending.counter_depth >= MAX_COUNTER_DEPTH {
        return CounterResult::NoCandidates;
    }

    // Scan defender's hand for Counter Options + Blast Digivolve candidates.
    let mut candidates: Vec<CounterCandidate> = Vec::new();
    for (i, card) in self.player(defender_pid).hand.iter().enumerate() {
        let card_id = /* ... */;
        let effects = self.effects_for_card(&card_id, card.handle());
        for effect in effects {
            if effect.blast_digivolve {
                // Existing: cross-join with field Digimon via can_digivolve.
                for (fi, field_perm) in self.player(defender_pid).battle_area.iter().enumerate() {
                    if self.can_digivolve(field_perm, card) {
                        candidates.push(CounterCandidate::Blast {
                            hand_index: i,
                            field_index: fi as u8,
                        });
                    }
                }
            }
            if effect.counter && card.card_kind(&self.card_data) == CardKind::Option {
                // Hand Counter Option: routes through play_option_from_hand.
                if self.can_pay_option_cost(defender_pid, card) {
                    candidates.push(CounterCandidate::HandOption { hand_index: i });
                }
            }
        }
    }

    // Scan defender's battle area for Counter abilities.
    for (fi, perm) in self.player(defender_pid).battle_area.iter().enumerate() {
        if perm.option_state != OptionState::Standard { continue; }
        let effects = self.effects_for_permanent(perm);
        for effect in effects {
            if effect.counter && effect.timing == EffectTiming::CounterEffect {
                // Field Counter ability.
                candidates.push(CounterCandidate::FieldAbility {
                    perm_index: fi as u8,
                });
            }
        }
    }

    if candidates.is_empty() {
        return CounterResult::NoCandidates;
    }

    pending.counter_depth += 1;

    // Install union-zone selection for the defender.
    self.install_counter_selection(defender_pid, candidates);
    CounterResult::Pending
}
```

Counter selection callback routes by candidate kind:

```rust
fn resolve_counter_selection(&mut self, candidate: CounterCandidate) {
    match candidate {
        CounterCandidate::Blast { hand_index, field_index } => {
            // Existing: execute_blast_digivolve.
        }
        CounterCandidate::HandOption { hand_index } => {
            // Route through Phase 8 play_option_from_hand with CounterEffect overlay.
            self.in_counter_window = true;
            self.play_option_from_hand(defender_pid, hand_index);
            self.in_counter_window = false;
        }
        CounterCandidate::FieldAbility { perm_index } => {
            // Fire the Counter ability directly.
            self.fire_counter_ability(defender_pid, perm_index);
        }
    }
}
```

The `in_counter_window` flag tells Phase 8's `play_option_core` to fire `EffectTiming::CounterEffect` BEFORE `EffectTiming::OptionMain`.

Tests (6):

- [ ] **Step 1: Write failing tests** — 6 tests:

```rust
#[test]
fn counter_window_emits_hand_option_candidates() {
    // Defender has a Counter Option in hand. Attacker declares.
    // Assert: PendingSelection::Hand with the hand index in valid_action_ids.
}

#[test]
fn counter_hand_option_resolves_through_play_option_pipeline() {
    // Resolve hand Counter Option. OptionMain + CounterEffect fire; card trashes.
    // Assert: CounterEffect fired before OptionMain; cost paid.
}

#[test]
fn counter_field_ability_fires_without_play_cost() {
    // Defender has a field Digimon with [Counter] triggered ability.
    // Assert: ability fires without hand-play cost.
}

#[test]
fn counter_blast_and_hand_option_coexist_in_selection() {
    // Defender has both a blast-digivolve source AND a Counter Option.
    // Assert: both action IDs emitted.
}

#[test]
fn counter_effect_timing_fires_only_on_selected_card() {
    // 2 cards with .counter(). Select only one. Assert only that card's
    // CounterEffect body fires.
}

#[test]
fn counter_chain_depth_guard_blocks_nested_counter() {
    // Counter Option body triggers another attack. Second attack's Counter window
    // is skipped (depth >= MAX_COUNTER_DEPTH).
    // Assert: no recursive Counter selection offered.
}
```

- [ ] **Step 2-6: TDD cycle.** Expected: **690 passing, 0 warnings.**

```bash
git commit -m "rust-engine(phase-9): Counter window broadening — hand Options + field abilities"
```

---

### Task 4: Raid target-switch rider

**Files:**
- Modify: `digimon-engine/src/combat.rs` — add `AttackState::PostBlock`; entry check for Raid + invalid target.
- Modify: `digimon-engine/src/cards/keyword_effects.rs` — Raid keyword auto-installs `WhenWouldBeAttackTarget` declarative rider.
- Create: `digimon-engine/tests/combat/raid_retarget.rs` — 4 tests.

**Key implementation:**

In `advance_pending_attack`, add state transition after `BlockOpen`:

```rust
// Post-Block state: check for Raid re-target if effective_target is invalid.
AttackState::BlockOpen if block_resolved => {
    if !self.handle_valid_or_player(pending.effective_target)
        && self.has_keyword(pending.attacker, Keyword::Raid)
    {
        let retargets = self.raid_retarget_candidates(pending);
        if !retargets.is_empty() {
            self.transition_attack_state(AttackState::PostBlock);
            self.install_raid_retarget_selection(retargets);
            return;
        }
        // No retarget available — attack fizzles.
        self.transition_attack_state(AttackState::Cleanup);
        return;
    }
    self.transition_attack_state(AttackState::Battle);
}
```

`raid_retarget_candidates` scans opponent's battle area for unsuspended Digimon (highest-DP priority per Raid rules).

Tests (4):

- [ ] **Step 1: Write failing tests** — 4 tests:

```rust
#[test]
fn raid_attacker_retargets_when_original_target_leaves() {
    // Attacker has <Raid>. Target deleted by a replacement during Block.
    // Assert: PostBlock selection offered with retarget candidates.
}

#[test]
fn raid_retarget_fizzle_when_no_candidates() {
    // Attacker has <Raid>. Target leaves, no legal retarget.
    // Assert: attack fizzles; Cleanup state reached.
}

#[test]
fn non_raid_attacker_fizzles_without_retarget() {
    // Non-Raid attacker. Target leaves mid-Block.
    // Assert: attack fizzles; no retarget selection.
}

#[test]
fn raid_retarget_fires_on_attack_target_change() {
    // Raid retargets from A to B. Assert: OnAttackTargetChange fires with
    // {attacker, old_target: A, new_target: B}.
}
```

- [ ] **Step 2-6: TDD cycle.** Expected: **694 passing, 0 warnings.**

```bash
git commit -m "rust-engine(phase-9): Raid target-switch rider + PostBlock state"
```

---

### Task 5: Collision MUST-block enforcement

**Files:**
- Modify: `digimon-engine/src/combat.rs::try_enter_block` — flip `is_optional = false` when attacker has Collision and candidate list non-empty.
- Modify: `digimon-engine/src/action/mask.rs` — drop PASS bit on non-optional selection-phase mask (should already be done — verify).
- Create: `digimon-engine/tests/combat/collision_mandatory.rs` — 3 tests.

**Key implementation:**

In `try_enter_block`:

```rust
let is_optional = if attacker_has_collision && !candidates.is_empty() {
    false
} else {
    true
};
let selection = PendingSelection {
    kind: SelectionKind::SelectTarget { own: true },
    valid_action_ids,
    is_optional,
    // ...
};
```

Verify action-mask at `mask.rs:517-542` already omits PASS when `!is_optional`. If not, extend.

Tests (3):

- [ ] **Step 1: Write failing tests** — 3 tests:

```rust
#[test]
fn collision_with_legal_blockers_mandates_selection() {
    // Attacker has <Collision>. Defender has 2 legal blockers.
    // Assert: PendingSelection with is_optional=false; mask drops PASS bit.
}

#[test]
fn collision_without_legal_blockers_falls_back_to_optional() {
    // Attacker has <Collision>. All defender Digimon are CannotBlock-gated.
    // Assert: no blocker candidates, is_optional=true fallback, attack proceeds.
}

#[test]
fn collision_combined_with_cannot_block_excludes_restricted_blockers() {
    // Attacker has Collision. Defender has 3 Digimon, 1 with CannotBlock.
    // Assert: only 2 candidates in valid_action_ids; PASS dropped.
}
```

- [ ] **Step 2-6: TDD cycle.** Expected: **697 passing, 0 warnings.**

```bash
git commit -m "rust-engine(phase-9): Collision MUST-block enforcement"
```

---

### Task 6: Piercing post-battle security check

**Files:**
- Modify: `digimon-engine/src/combat.rs` — add `AttackState::PostBattle`; entry check after Battle resolves.
- Modify: `digimon-engine/src/combat.rs::resolve_battle` — route to PostBattle when attacker survives + defender wiped + attacker has Piercing.
- Create: `digimon-engine/tests/combat/piercing_security.rs` — 4 tests.

**Key implementation:**

```rust
fn resolve_battle(&mut self, pending: &mut PendingAttack) {
    // ... existing DP compare + deletion ...

    if self.attacker_survived(pending)
        && self.defender_was_wiped(pending)
        && self.has_keyword(pending.attacker, Keyword::Piercing)
    {
        self.transition_attack_state(AttackState::PostBattle);
        // Enter security-check resolver as if direct-player attack.
        self.enter_piercing_security_check(pending);
        return;
    }

    self.transition_attack_state(AttackState::Cleanup);
}
```

`enter_piercing_security_check` reuses `drive_security_resolution` with `attacker = pending.attacker` and target = defending player.

Tests (4):

- [ ] **Step 1: Write failing tests** — 4 tests:

```rust
#[test]
fn piercing_attacker_triggers_security_check_after_wiping_defender() {
    // Attacker has <Piercing>. Attacker survives battle; defender wiped.
    // Assert: security check fires against defending player.
}

#[test]
fn piercing_honors_jamming_on_attacker() {
    // Attacker has Piercing + Jamming. Post-battle security check fires.
    // Assert: no security-skill damage to attacker.
}

#[test]
fn piercing_stacks_with_security_attack_modifier() {
    // Attacker has Piercing + <Security Attack +1>. Post-battle security check.
    // Assert: 2 security cards consumed.
}

#[test]
fn piercing_does_nothing_when_attacker_wiped() {
    // Attacker and defender mutual-KO'd. Attacker had Piercing.
    // Assert: no post-battle security check.
}
```

- [ ] **Step 2-6: TDD cycle.** Expected: **701 passing, 0 warnings.**

```bash
git commit -m "rust-engine(phase-9): Piercing post-battle security check + PostBattle state"
```

---

### Task 7: Reboot unsuspend consumer

**Files:**
- Modify: `digimon-engine/src/game_phases.rs::begin_turn` (or wherever unsuspend step is) — scan opponent's battle area for `<Reboot>` keyword.
- Create: `digimon-engine/tests/combat/reboot_unsuspend.rs` — 3 tests.

**Key implementation:**

In `begin_turn` unsuspend step:

```rust
fn unsuspend_phase(&mut self) {
    let tp = self.turn_player;

    // Existing: unsuspend all of tp's suspended Digimon.
    for perm in self.player_mut(tp).battle_area.iter_mut() {
        perm.is_suspended = false;
    }

    // Phase 9: also unsuspend opponent's Digimon with <Reboot>.
    let opponent = tp.opponent();
    for perm in self.player_mut(opponent).battle_area.iter_mut() {
        if self.has_keyword_for_perm(perm, Keyword::Reboot) {
            perm.is_suspended = false;
        }
    }
}
```

(Adjust borrow checker pattern — the example above won't compile as-is. Collect handles first, then apply.)

Tests (3):

- [ ] **Step 1: Write failing tests** — 3 tests:

```rust
#[test]
fn reboot_digimon_unsuspends_on_opponents_turn() {
    // P0 has a Reboot Digimon, suspended at end of P0's turn.
    // P1 starts turn → unsuspend phase.
    // Assert: P0's Reboot Digimon is unsuspended.
}

#[test]
fn reboot_combined_with_overclock_composes() {
    // Overclock adds its own unsuspend logic; verify Reboot + Overclock don't conflict.
    // Assert: both unsuspend semantics apply correctly.
}

#[test]
fn cannot_be_unsuspended_by_effect_gates_reboot() {
    // P0 has a Reboot Digimon also under CannotBeUnsuspendedByEffect modifier.
    // P1 unsuspend phase. Assert: Reboot Digimon stays suspended.
    // (If gate doesn't apply to Reboot — by design — reverse assertion.)
}
```

- [ ] **Step 2-6: TDD cycle.** Expected: **704 passing, 0 warnings.**

```bash
git commit -m "rust-engine(phase-9): Reboot unsuspend-phase consumer"
```

---

### Task 8: `OnBlock` + `OnAllyAttack` + `OnOpponentAttack` dispatch

**Files:**
- Modify: `digimon-engine/src/selection.rs` — add `TriggerSource::OnBlock { attacker, blocker }` variant.
- Modify: `digimon-engine/src/combat.rs::try_enter_block` — fire `OnBlock` post-declare.
- Modify: `digimon-engine/src/combat.rs::begin_attack_impl` — fire `OnAllyAttack` + `OnOpponentAttack` after `WhenAttacking`.
- Modify: `digimon-engine/src/effect_queue.rs` — handle new TriggerSource variant in scan.
- Create: `digimon-engine/tests/combat/on_block_observer.rs` — 3 tests.
- Create: `digimon-engine/tests/combat/on_ally_opponent_attack.rs` — 3 tests.

**Key implementation:**

```rust
// selection.rs:
pub enum TriggerSource {
    // ... existing ...
    OnBlock { attacker: PermanentHandle, blocker: PermanentHandle },
}
```

In `try_enter_block` after blocker declared:

```rust
self.enqueue_triggered(
    EffectTiming::OnBlock,
    TriggerSource::OnBlock { attacker, blocker },
);
self.drain_effect_queue();
```

In `begin_attack_impl` after `OnAttack` + `WhenAttacking` fires:

```rust
// Fan out OnAllyAttack to attacker-controller's battle area (excluding attacker).
for (i, perm) in self.player(attacker.player).battle_area.iter().enumerate() {
    if perm.handle() == attacker { continue; }
    // Scan for OnAllyAttack effects.
    self.enqueue_triggered_for_perm(
        EffectTiming::OnAllyAttack,
        PermanentHandle { player: attacker.player, index: i as u8 },
    );
}

// Fan out OnOpponentAttack to opposite controller.
let opp = attacker.player.opponent();
for (i, _perm) in self.player(opp).battle_area.iter().enumerate() {
    self.enqueue_triggered_for_perm(
        EffectTiming::OnOpponentAttack,
        PermanentHandle { player: opp, index: i as u8 },
    );
}
self.drain_effect_queue();
```

Tests (6):

- [ ] **Step 1: Write failing tests**:

```rust
// on_block_observer.rs
#[test]
fn on_block_fires_globally_when_blocker_declared() {
    // Witnesses on both P0 and P1 with OnBlock observer.
    // Block declared. Assert both fire with correct attacker/blocker context.
}

#[test]
fn on_block_sees_post_declare_state() {
    // OnBlock observer reads effective_target — should be the blocker.
    // Assert: observer reads the blocker, not the original target.
}

#[test]
fn on_block_fires_on_collision_forced_block() {
    // Attacker has <Collision>, defender forced to block.
    // Assert: OnBlock still fires.
}

// on_ally_opponent_attack.rs
#[test]
fn on_ally_attack_fires_on_same_controller_permanents_except_attacker() {
    // P0 has attacker + witness1 + witness2. P1 has witness3.
    // P0 attacks. Assert: witness1 + witness2 fire; witness3 does not;
    //   attacker does not fire its own OnAllyAttack.
}

#[test]
fn on_opponent_attack_fires_on_opposite_controller() {
    // P0 attacks. P1 has witness.
    // Assert: witness fires with attacker context.
}

#[test]
fn observers_see_attacker_via_ctx_helper() {
    // OnAllyAttack observer reads ctx.attacker() → attacker handle.
    // Assert: correct handle returned.
}
```

- [ ] **Step 2-6: TDD cycle.** Expected: **710 passing, 0 warnings.**

```bash
git commit -m "rust-engine(phase-9): OnBlock + OnAllyAttack + OnOpponentAttack dispatch"
```

---

### Task 9: Docs

**Files:**
- Modify: `docs/RUST_ENGINE_API.md` — new §Phase 9 section.
- Modify: `docs/RUST_PYTHON_PARITY.md` — new §15 combat-interrupt entry.
- Modify: `.claude/plans/recursive-coalescing-candle.md` — flip Phase 9 row.
- Modify: `docs/superpowers/plans/2026-04-21-rust-engine-phase-9-combat-interrupts.md` — populate Status section.

Content per spec §§4-11 (summarize, worked examples for `.counter()` + `ctx.redirect_attack` + Raid auto-install):

- [ ] **Step 1:** Write §Phase 9 section (~200 lines, mirror §Phase 8 shape).
- [ ] **Step 2:** Write §15 parity entry (~50 lines).
- [ ] **Step 3:** Flip roadmap + next-phase → Phase 10.
- [ ] **Step 4:** Populate plan Status.
- [ ] **Step 5:** Verify no stub `TODO(phase-9-stub)` markers (grep).
- [ ] **Step 6:** Full suite green (no code changes, test count unchanged at 710).
- [ ] **Step 7: Commit.**

```bash
git commit -m "docs(phase-9): RUST_ENGINE_API + PARITY + roadmap + plan status — combat interrupts landed"
```

---

### Task 10: Behavioral end-to-end

**Files:**
- Create: `digimon-engine/tests/combat/phase9_end_to_end.rs` — 1 integrated test.
- Modify: `digimon-engine/tests/combat/main.rs` — add `mod phase9_end_to_end;`.

**Scenario**: Dark Masters Ace Counter vs. TS Olympos Raid vs. Collision mandate:

1. P0 plays a Raid Digimon (attacker).
2. P0 declares attack against a P1 Digimon.
3. P1 has a Counter Option in hand — plays it in the Counter window.
4. Counter Option body redirects the attack (via `ctx.redirect_attack`).
5. New target was a Collision-keyword Digimon — force-block selection.
6. P1 declares a blocker.
7. Battle resolves.
8. Assert: Counter Option was trashed; OnAttackTargetChange fired; Collision forced block; OnBlock observer fired; no state leaks.

- [ ] **Step 1: Write failing test.**
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement scenario via DebugRunner + hand-authored test cards.**
- [ ] **Step 4: Test passes.**
- [ ] **Step 5: Full suite green.** Expected: **711 passing, 0 warnings.**
- [ ] **Step 6: Commit.**

```bash
git commit -m "rust-engine(phase-9): behavioral end-to-end — Counter + Raid + Collision multi-interrupt"
```

---

## Deferred / Out of Scope (confirmed)

The following are intentionally **not** in Phase 9:

- **Counter-chain depth > 1.** v1 caps at single Counter fire.
- **`SwitchAttacker` replacement.** Not printed.
- **Multi-target attacks.** Separate mechanic (Task 3.5 era).
- **Python Counter hand-play parity port.** Python side requires parallel work; tracked as parity §15 follow-up.
- **Alliance trait-filter parsing.** Task deferred to a small Phase 10 follow-up OR added to Task 3 if cheap (depends on existing parsing infrastructure).
- **Multi-player turn-math generalization.** Shared deferral with Phase 8 `compute_delay_trash_turn`.

---

## Verification

1. `cargo test --manifest-path digimon-engine/Cargo.toml` — full suite green after each task.
2. Per-task test files land green (Working Rule 18).
3. `RUSTFLAGS="-D warnings" cargo test --manifest-path digimon-engine/Cargo.toml` — zero warnings.
4. Final: `cargo build --manifest-path digimon-engine-py/Cargo.toml` — PyO3 bindings still compile.
5. Re-run `/assess-archetype-rust` on Dark Masters after Phase 9 → confirm 8 Ace Counter cards unblock, ~5 redirect cards unblock.

---

## Status

Phase 9 Combat Interrupts — **✅ Landed 2026-04-21**. Plan written 2026-04-21.

Baseline 671 → 710 passing after Task 8 (+39 net new tests). Task 9 is docs-only, no test count change. Task 10 behavioral e2e will bring the suite to ~711 passing.

| Task | Commit | Landing | Test delta |
|------|--------|---------|------------|
| 1 WhenWouldAttack/WouldBeAttackTarget dispatch + Phase 6 auto-install | `4a40bebb` → `c7d26c95` (quality fixes) | 2026-04-21 | +8 (→ 679) |
| 2 `ctx.redirect_attack` + `ctx.cancel_attack` | `815b14a7` | 2026-04-21 | +5 (→ 684) |
| 3 Counter window broadening (3 candidate shapes) | `f8971264` | 2026-04-21 | +6 (→ 690) |
| 4 Raid retarget + `AttackState::PostBlock` | `c5195f4b` | 2026-04-21 | +4 (→ 694) |
| 5 Collision MUST-block | `8ede9c71` | 2026-04-21 | +3 (→ 697) |
| 6 Piercing post-battle + `AttackState::PostBattle` | `5887afef` | 2026-04-21 | +4 (→ 701) |
| 7 Reboot unsuspend consumer | `0173fa68` | 2026-04-21 | +3 (→ 704) |
| 8 `OnBlock` + `OnAllyAttack` + `OnOpponentAttack` dispatch | `7a7b6fdb` | 2026-04-21 | +6 (→ 710) |
| 9 Docs (RUST_ENGINE_API + PARITY + roadmap + plan status) | this commit | 2026-04-21 | 0 (710) |
| 10 Behavioral end-to-end | upcoming | TBD | +1 (→ 711 expected) |

Zero warnings under `RUSTFLAGS="-D warnings"`. `ACTION_SPACE_SIZE` unchanged at 2168. No `TODO(phase-9-stub)` markers remain in the tree.
