## Why

When a Digimon attacking the opponent's security gains `<Security A. +N>` mid-attack — for example, when [BT21-001 Gigimon's](data/cards.json) inherited `[Your Turn][OPT] When opponent's security stack is removed from, 1 of your Digimon may digivolve into a Reptile/Dragonkin card in hand` fires during the post-check drain and digivolves the attacker into [BT21-029 Medusamon](data/cards.json) (`<Security A. +1>`) — the Rust engine performs only the security checks it queued at attack declaration. Per DCGO ([CardController.cs:3956-3987](DCGO/Assets/Scripts/Script/CardController.cs:3956)) the loop re-reads the active permanent's `Strike` property on every iteration, so digivolving mid-attack extends the check sequence. Today's engine locks the count at declaration, so the Lamiamon-into-Medusamon flow performs 1 check when it should perform 2 — a faithfulness gap visible to RL agents and human players alike.

## What Changes

- Replace the "decrementing `checks_remaining`" model in `resolve_player_security_loop` / `drive_security_resolution` with a "current `Strike` vs `checks_performed`" model that re-reads the attacker's effective `<Security A.>` total at each iteration boundary.
- Persist `checks_performed` (cumulative) on `SecurityResolutionState` instead of `checks_remaining` (countdown), so the recompute can compare against the current attacker's Strike.
- Continue honoring existing rails: attacker-deleted-mid-check terminates the loop, deck-out declares the attacker the winner, and the loop still stops when the attacker is no longer a Digimon for rules purposes.
- Add a behavioral regression test that drives the exact Gigimon → Elizamon → Lamiamon stack flow and asserts **2** security checks after the mid-attack digivolve into Medusamon.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `security-card-effects`: add a requirement that the security-check loop re-evaluates the attacker's effective `<Security A.>` total each iteration so mid-attack changes (digivolve, modifier gain/loss, ChangeSAttack effects) take effect immediately.

## Impact

- **Affected code**: [`code/digimon-engine/src/combat.rs`](code/digimon-engine/src/combat.rs) — `resolve_player_security_loop`, `pop_and_start_security_check`, `drive_security_resolution`'s `DisposeFinalize` arm, and `SecurityResolutionState`.
- **Affected tests**: existing `tests/combat/`, `tests/cards_behavioral/`, `tests/replay_runner.rs`, plus piercing/security-attack scenarios. Anywhere a `<Security A. +N>` or `ChangeSAttack` modifier exists, the loop's iteration shape now depends on the live attacker rather than the declaration-time snapshot.
- **Replay parity**: deterministic recordings that depended on the old (under-counting) behavior will diverge on rerun; the verify step in [code/digimon-engine/src/runners/replay.rs](code/digimon-engine/src/runners/replay.rs) may flag these. Existing recordings need re-capture.
- **RL training**: agent observations and the action-mask shape are unchanged, but security-attack outcomes can now be larger — episodes touching `<Security A. +N>` effects mid-attack may have meaningfully different reward trajectories. Old checkpoints remain loadable.
- **Cross-engine parity**: brings the Rust engine in line with DCGO's `Permanent.Strike` getter ([Permanent.cs:1818-1951](DCGO/Assets/Scripts/Script/Permanent.cs:1818)). Update `docs/RUST_PYTHON_PARITY.md` if a corresponding entry exists for this divergence.
