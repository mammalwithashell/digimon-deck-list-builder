## Context

The Rust engine's player-security-attack loop lives in [`Game::resolve_player_security_loop`](code/digimon-engine/src/combat.rs:2351) and the `DisposeFinalize` arm of [`Game::drive_security_resolution`](code/digimon-engine/src/combat.rs:2731). The current implementation reads the attacker's effective `<Security A.>` total exactly once — at loop entry — into a local `checks: u8`, then stores `checks - 1` as `checks_remaining` on [`SecurityResolutionState`](code/digimon-engine/src/combat.rs:2454). Each `DisposeFinalize` tick decrements `checks_remaining` and pops the next card when it's still non-zero.

DCGO ([`CardController.cs:3956-3987`](DCGO/Assets/Scripts/Script/CardController.cs:3956)) takes the opposite approach: a `while (true)` that re-reads `AttackingPermanent.Strike` on every iteration. `Strike` ([`Permanent.cs:1938-1951`](DCGO/Assets/Scripts/Script/Permanent.cs:1938)) is a computed getter that scans every field permanent's `IChangeSAttackEffect` from scratch. Nothing is cached; mid-attack changes to the active permanent's keywords or modifiers are visible the next time the loop iterates.

A concrete reproducer landed at [code/digimon-engine/tests/medusamon_attack_scenario.rs](code/digimon-engine/tests/medusamon_attack_scenario.rs:1): a P1 stack of [BT21-001 Gigimon / BT24-008 Elizamon / BT21-025 Lamiamon] attacks P2's security (top = BT21-093 Raging Serpentine). Gigimon's `on_opponent_security_removed` inherited fires post-check, the controller digivolves Lamiamon into BT21-029 Medusamon (`<Security A. +1>`), and the engine ends the attack after the single Lamiamon check — DCGO would continue for a second check because `Strike` is now 2.

## Goals / Non-Goals

**Goals:**
- The player-attack security loop SHALL re-evaluate the attacker's effective `<Security A.>` total at each iteration boundary, so mid-attack digivolves, keyword grants, ChangeSAttack effects, and InvertSAttack effects take effect on subsequent checks.
- Keep the existing pause-and-resume design intact: `pending_selection` mid-drain still parks the loop in `SecurityResolutionState` for resumption by `advance_security_resolution`.
- Land a behavioral regression test (with [BT21-001 / BT24-008 / BT21-025 / BT21-029] and BT21-093 in security) that fails on `main` and passes on the fix.

**Non-Goals:**
- Changing the `<Piercing>` follow-up security check entry point ([`Game::enter_piercing_security_check`](code/digimon-engine/src/combat.rs:2337)). It already delegates into `resolve_player_security_loop` and will inherit the fix.
- Changing the `Strike` aura-bonus override path ([`Game::dynamic_security_attack_aura_bonus`](code/digimon-engine/src/combat.rs:2377)). The same helper feeds the recompute.
- Reworking modifier ownership or per-card keyword storage. Recomputation reads from the existing modifier registry + native keywords.
- Cards.json or DSL surface changes — Medusamon's `<Security A. +1>` already populates `CardData.keywords` via the DSL loader.

## Decisions

### Decision 1: Replace the countdown with a "performed vs. target" comparison

`SecurityResolutionState.checks_remaining: u8` becomes `checks_performed: u8`. The loop's continuation predicate moves from `remaining > 0` to `current_effective_strike(attacker) > checks_performed`.

**Why**: Matches DCGO's getter-based shape directly. The recompute reads live state, so a mid-attack digivolve that introduces a new `<Security A. +N>` is reflected on the next iteration without needing an explicit "extend the queue" mutation.

**Alternative considered**: Keep `checks_remaining` but mutate it in a post-effect hook whenever the attacker's effective strike changes. Rejected — every mid-attack pathway that touches the attacker (digivolve, modifier add/remove, target switch) would need to remember to mutate. DCGO's design intentionally avoids that mutation surface.

### Decision 2: Centralize the strike calculation in a private helper

Extract the four-summand expression at [combat.rs:2376-2379](code/digimon-engine/src/combat.rs:2376) (`base_checks + sa_modifier + change_s_attack + sa_keyword`) into a private helper:

```rust
fn current_security_strike(&self, attacker: PermanentHandle) -> u8
```

Call it from both the initial pop in `resolve_player_security_loop` and the recompute in `DisposeFinalize`'s continuation arm. The helper does NOT subtract `checks_performed` — it returns the *current target* strike count.

**Why**: One source of truth for what counts toward `Strike`. Anything that touches Security A. in the future (new modifier kinds, new keyword payloads) lands in one place.

### Decision 3: Handle in-flight attacker invalidation by terminating the loop

If `current_security_strike` is called for an attacker that no longer satisfies `handle_valid` (deleted or returned to hand mid-resolution), treat it as 0 and let the existing `AttackerDeletedBySecurity` / `SecurityCheckSurvived` post-checks decide the outcome. The recompute does not need its own "attacker gone" branch.

**Why**: The existing `DisposeFinalize` arm already checks `!self.handle_valid(attacker)` before popping the next card ([combat.rs:2758-2760](code/digimon-engine/src/combat.rs:2758)). Folding the helper into the same code path keeps the lifecycle invariants intact.

### Decision 4: Recording schema stays unchanged; expect divergences on replay

`SecurityResolutionState` is internal — it isn't recorded directly. The only observable change in [`code/digimon-engine/src/runners/replay.rs`](code/digimon-engine/src/runners/replay.rs:1) is that recordings captured before the fix may show fewer security checks than the live engine produces post-fix when a recompute would have fired. The `--verify` flag's checks (memory / turn / phase / game-over) will flag these as divergences.

**Why**: Re-capturing recordings is cheap (training jobs regenerate them) and the alternative — gating the new behavior behind a recording-version flag — adds permanent code complexity for a transitional concern.

## Risks / Trade-offs

- **Risk**: Existing combat tests assume the old single-check behavior and break en masse.
  - **Mitigation**: Run the whole `cargo test --manifest-path code/digimon-engine/Cargo.toml` suite after the change and triage failures into "expected (was wrong)" vs. "real regression." Tests covering `<Security A. +N>` from declaration time (no mid-attack change) should still pass — the recompute returns the same value when the attacker doesn't change.

- **Risk**: A mid-attack effect that *reduces* the attacker's strike (e.g. a Medusamon being de-digivolved back to Lamiamon by a Counter security effect, or an InvertSAttack flood gate landing inside `OnSecurityCheck`) ends the loop earlier than the old behavior.
  - **Mitigation**: This is the correct DCGO behavior. Document the asymmetry in the spec scenario and add a regression test for the reduction case.

- **Risk**: Infinite loop if an `OnLoseSecurity` effect somehow re-grants `<Security A. +N>` to the attacker every iteration without progressing.
  - **Mitigation**: Loop bound is "strike value", which is a `u8` derived from a finite scan of permanents — no effect today can grow it unboundedly per tick. Add a safety cap (e.g. `checks_performed.saturating_add(1)` and abort if `current_security_strike` ever exceeds `MAX_SECURITY_CHECKS = 16`) as belt-and-braces.

- **Trade-off**: The recompute runs once per loop iteration. The scan over field permanents is O(perm_count) per `<Security A.>`-affecting effect, so in the worst case (deep keyword/modifier soup) we pay extra per check. In practice security stacks are ≤5 cards and `Strike` rarely exceeds 2 — overhead is negligible compared to the existing per-check `OnSecurityCheck` drain.

## Migration Plan

1. Land the engine change behind no flag — it's a correctness fix.
2. Re-run `cargo test` and update any test that asserted the old (incorrect) check count.
3. Spot-check a Medusamon recording with `digimon-engine-cli replay --verify` to confirm the expected divergence and document the recording-rebake step in the change archive.
4. Add a `RUST_PYTHON_PARITY.md` entry only if the Python engine ever had this same drift — otherwise the section can stay silent.

## Open Questions

- None blocking. The fix shape and reproducer are nailed down by the DCGO citation and the existing scenario test.
